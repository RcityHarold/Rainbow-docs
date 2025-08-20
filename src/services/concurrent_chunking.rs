//! 并发分块处理
//! 
//! 提供高性能的并发文档分块处理能力

use std::sync::Arc;
use tokio::sync::{Semaphore, RwLock};
use tokio::task::JoinSet;
use serde::{Serialize, Deserialize};
use crate::error::ApiError;
use crate::services::chunking::{ChunkingConfig, EnhancedDocumentChunk};
use crate::services::intelligent_chunker::{IntelligentChunker, ChunkingResult};
use crate::services::structure_parser::DocumentStructure;
use std::time::Instant;

/// 并发配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 每个任务的最大文档大小（字节）
    pub max_document_size: usize,
    /// 任务队列大小
    pub queue_size: usize,
    /// 工作线程数
    pub worker_threads: usize,
    /// 是否启用批处理
    pub enable_batching: bool,
    /// 批处理大小
    pub batch_size: usize,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 10,
            max_document_size: 10 * 1024 * 1024, // 10MB
            queue_size: 100,
            worker_threads: 4,
            enable_batching: true,
            batch_size: 5,
        }
    }
}

/// 分块任务
#[derive(Debug, Clone)]
pub struct ChunkingTask {
    pub document_id: String,
    pub title: String,
    pub content: String,
    pub config: ChunkingConfig,
    pub priority: TaskPriority,
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// 任务结果
#[derive(Debug)]
pub struct TaskResult {
    pub document_id: String,
    pub result: Result<ChunkingResult, ApiError>,
    pub processing_time: std::time::Duration,
}

/// 并发分块处理器
pub struct ConcurrentChunker {
    /// 并发配置
    config: ConcurrencyConfig,
    /// 信号量控制并发
    semaphore: Arc<Semaphore>,
    /// 智能分块器池
    chunker_pool: Arc<RwLock<Vec<Arc<IntelligentChunker>>>>,
    /// 任务统计
    stats: Arc<RwLock<ProcessingStats>>,
}

/// 处理统计
#[derive(Debug, Default, Clone)]
pub struct ProcessingStats {
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub total_processing_time: std::time::Duration,
    pub total_bytes_processed: u64,
}

impl ProcessingStats {
    pub fn average_processing_time(&self) -> std::time::Duration {
        if self.completed_tasks == 0 {
            return std::time::Duration::from_secs(0);
        }
        self.total_processing_time / self.completed_tasks as u32
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            return 0.0;
        }
        self.completed_tasks as f64 / self.total_tasks as f64
    }
}

impl ConcurrentChunker {
    /// 创建新的并发分块处理器
    pub async fn new(config: ConcurrencyConfig, chunking_config: ChunkingConfig) -> Result<Self, ApiError> {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_tasks));
        
        // 创建分块器池
        let mut chunkers = Vec::new();
        for _ in 0..config.worker_threads {
            let chunker = IntelligentChunker::new(chunking_config.clone())
                .map_err(|e| ApiError::InternalServerError(format!("Failed to create chunker: {}", e)))?;
            chunkers.push(Arc::new(chunker));
        }
        
        Ok(Self {
            config,
            semaphore,
            chunker_pool: Arc::new(RwLock::new(chunkers)),
            stats: Arc::new(RwLock::new(ProcessingStats::default())),
        })
    }

    /// 处理单个文档
    pub async fn process_document(&self, task: ChunkingTask) -> TaskResult {
        let start = Instant::now();
        let document_id = task.document_id.clone();
        
        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.total_tasks += 1;
            stats.total_bytes_processed += task.content.len() as u64;
        }
        
        // 获取信号量许可
        let _permit = self.semaphore.acquire().await.unwrap();
        
        // 从池中获取一个分块器
        let chunker = {
            let pool = self.chunker_pool.read().await;
            let idx = (simple_hash(&task.document_id) as usize) % pool.len();
            pool[idx].clone()
        };
        
        // 执行分块
        let result = chunker.chunk_document(
            &task.document_id,
            &task.title,
            &task.content
        ).await;
        
        // 更新统计
        {
            let mut stats = self.stats.write().await;
            match &result {
                Ok(_) => stats.completed_tasks += 1,
                Err(_) => stats.failed_tasks += 1,
            }
            stats.total_processing_time += start.elapsed();
        }
        
        TaskResult {
            document_id,
            result: result.map_err(|e| ApiError::InternalServerError(e.to_string())),
            processing_time: start.elapsed(),
        }
    }

    /// 批量处理文档
    pub async fn process_batch(&self, tasks: Vec<ChunkingTask>) -> Vec<TaskResult> {
        let mut results = Vec::new();
        
        // 按优先级排序
        let mut sorted_tasks = tasks;
        sorted_tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // 创建任务集
        let mut join_set = JoinSet::new();
        
        for task in sorted_tasks {
            let self_clone = Arc::new(self.clone());
            join_set.spawn(async move {
                self_clone.process_document(task).await
            });
        }
        
        // 收集结果
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(task_result) => results.push(task_result),
                Err(e) => {
                    tracing::error!("Task failed: {}", e);
                }
            }
        }
        
        results
    }

    /// 异步处理任务队列
    pub async fn process_queue_async(&self, tasks: Vec<ChunkingTask>) -> Vec<TaskResult> {
        let mut join_set = JoinSet::new();
        
        // 分批处理以避免资源过度使用
        let chunk_size = self.config.max_concurrent_tasks;
        let mut results = Vec::new();
        
        for chunk in tasks.chunks(chunk_size) {
            for task in chunk {
                let self_clone = Arc::new(self.clone());
                let task_clone = task.clone();
                
                join_set.spawn(async move {
                    self_clone.process_document(task_clone).await
                });
            }
            
            // 等待当前批次完成
            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok(task_result) => results.push(task_result),
                    Err(e) => {
                        tracing::error!("Task failed: {}", e);
                    }
                }
            }
        }
        
        results
    }

    /// 获取处理统计
    pub async fn get_stats(&self) -> ProcessingStats {
        self.stats.read().await.clone()
    }

    /// 重置统计信息
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = ProcessingStats::default();
    }
}

// 为了支持 Clone
impl Clone for ConcurrentChunker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            semaphore: self.semaphore.clone(),
            chunker_pool: self.chunker_pool.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// 批处理优化器
pub struct BatchOptimizer {
    config: ConcurrencyConfig,
}

impl BatchOptimizer {
    pub fn new(config: ConcurrencyConfig) -> Self {
        Self { config }
    }

    /// 优化批次分组
    pub fn optimize_batches(&self, tasks: Vec<ChunkingTask>) -> Vec<Vec<ChunkingTask>> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_size = 0;
        
        for task in tasks {
            let task_size = task.content.len();
            
            // 检查是否需要新批次
            if !current_batch.is_empty() && 
               (current_batch.len() >= self.config.batch_size || 
                current_size + task_size > self.config.max_document_size) {
                batches.push(current_batch);
                current_batch = Vec::new();
                current_size = 0;
            }
            
            current_size += task_size;
            current_batch.push(task);
        }
        
        if !current_batch.is_empty() {
            batches.push(current_batch);
        }
        
        batches
    }

    /// 根据文档特征分组
    pub fn group_by_characteristics(&self, tasks: Vec<ChunkingTask>) -> HashMap<String, Vec<ChunkingTask>> {
        let mut groups = HashMap::new();
        
        for task in tasks {
            // 简单的特征分类
            let characteristic = if task.content.len() < 1000 {
                "small"
            } else if task.content.len() < 10000 {
                "medium"
            } else {
                "large"
            };
            
            groups.entry(characteristic.to_string())
                .or_insert_with(Vec::new)
                .push(task);
        }
        
        groups
    }
}

/// 并发管理器
pub struct ConcurrencyManager {
    chunker: Arc<ConcurrentChunker>,
    optimizer: Arc<BatchOptimizer>,
}

impl ConcurrencyManager {
    pub async fn new(
        concurrency_config: ConcurrencyConfig,
        chunking_config: ChunkingConfig,
    ) -> Result<Self, ApiError> {
        let chunker = Arc::new(ConcurrentChunker::new(concurrency_config.clone(), chunking_config).await?);
        let optimizer = Arc::new(BatchOptimizer::new(concurrency_config));
        
        Ok(Self { chunker, optimizer })
    }

    /// 智能处理任务队列
    pub async fn process_queue(&self, tasks: Vec<ChunkingTask>) -> Vec<TaskResult> {
        if self.chunker.config.enable_batching {
            // 使用批处理优化
            let batches = self.optimizer.optimize_batches(tasks);
            let mut all_results = Vec::new();
            
            for batch in batches {
                let results = self.chunker.process_batch(batch).await;
                all_results.extend(results);
            }
            
            all_results
        } else {
            // 直接并发处理
            self.chunker.process_batch(tasks).await
        }
    }

    /// 获取性能报告
    pub async fn get_performance_report(&self) -> PerformanceReport {
        let stats = self.chunker.get_stats().await;
        
        PerformanceReport {
            total_tasks: stats.total_tasks,
            completed_tasks: stats.completed_tasks,
            failed_tasks: stats.failed_tasks,
            success_rate: stats.success_rate(),
            average_processing_time: stats.average_processing_time(),
            total_bytes_processed: stats.total_bytes_processed,
            throughput_mbps: calculate_throughput(&stats),
        }
    }
}

/// 性能报告
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub success_rate: f64,
    pub average_processing_time: std::time::Duration,
    pub total_bytes_processed: u64,
    pub throughput_mbps: f64,
}

fn calculate_throughput(stats: &ProcessingStats) -> f64 {
    if stats.total_processing_time.as_secs_f64() == 0.0 {
        return 0.0;
    }
    
    let bytes_per_second = stats.total_bytes_processed as f64 / stats.total_processing_time.as_secs_f64();
    bytes_per_second / (1024.0 * 1024.0) // 转换为 MB/s
}

// 简单的哈希函数用于分布式选择
fn simple_hash(s: &str) -> u32 {
    let mut hash = 0u32;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    hash
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_processing() {
        let config = ConcurrencyConfig {
            max_concurrent_tasks: 5,
            ..Default::default()
        };
        
        let chunking_config = ChunkingConfig::default();
        let chunker = ConcurrentChunker::new(config, chunking_config).await.unwrap();
        
        // 创建测试任务
        let tasks = vec![
            ChunkingTask {
                document_id: "doc1".to_string(),
                title: "Test Doc 1".to_string(),
                content: "This is test content 1".to_string(),
                config: ChunkingConfig::default(),
                priority: TaskPriority::Normal,
            },
            ChunkingTask {
                document_id: "doc2".to_string(),
                title: "Test Doc 2".to_string(),
                content: "This is test content 2".to_string(),
                config: ChunkingConfig::default(),
                priority: TaskPriority::High,
            },
        ];
        
        let results = chunker.process_batch(tasks).await;
        assert_eq!(results.len(), 2);
        
        let stats = chunker.get_stats().await;
        assert_eq!(stats.total_tasks, 2);
    }

    #[tokio::test]
    async fn test_batch_optimizer() {
        let config = ConcurrencyConfig {
            batch_size: 2,
            max_document_size: 100,
            ..Default::default()
        };
        
        let optimizer = BatchOptimizer::new(config);
        
        let tasks = vec![
            ChunkingTask {
                document_id: "doc1".to_string(),
                title: "Small Doc".to_string(),
                content: "x".repeat(30),
                config: ChunkingConfig::default(),
                priority: TaskPriority::Normal,
            },
            ChunkingTask {
                document_id: "doc2".to_string(),
                title: "Medium Doc".to_string(),
                content: "x".repeat(50),
                config: ChunkingConfig::default(),
                priority: TaskPriority::Normal,
            },
            ChunkingTask {
                document_id: "doc3".to_string(),
                title: "Large Doc".to_string(),
                content: "x".repeat(80),
                config: ChunkingConfig::default(),
                priority: TaskPriority::Normal,
            },
        ];
        
        let batches = optimizer.optimize_batches(tasks);
        assert!(batches.len() >= 2); // 应该分成至少2批
    }
}