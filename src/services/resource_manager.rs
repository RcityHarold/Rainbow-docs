//! 资源管理器
//! 
//! 管理系统资源，包括内存、CPU、并发限制等

use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};
use crate::error::ApiError;

/// 资源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// 最大内存使用量（字节）
    pub max_memory_bytes: usize,
    /// 最大CPU使用率（0.0-1.0）
    pub max_cpu_usage: f64,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 内存压力阈值
    pub memory_pressure_threshold: f64,
    /// CPU压力阈值
    pub cpu_pressure_threshold: f64,
    /// 监控间隔
    pub monitoring_interval: Duration,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_cpu_usage: 0.8, // 80%
            max_concurrent_tasks: 50,
            memory_pressure_threshold: 0.8, // 80%
            cpu_pressure_threshold: 0.8, // 80%
            monitoring_interval: Duration::from_secs(5),
        }
    }
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// 当前内存使用量
    pub memory_used_bytes: usize,
    /// 内存使用率
    pub memory_usage_percent: f64,
    /// CPU使用率
    pub cpu_usage_percent: f64,
    /// 活跃任务数
    pub active_tasks: usize,
    /// 等待任务数
    pub waiting_tasks: usize,
    /// 采样时间（Unix时间戳，秒）
    pub timestamp: u64,
}

/// 资源限制器
pub struct ResourceLimiter {
    /// 内存限制器
    memory_limiter: Arc<MemoryLimiter>,
    /// CPU限制器
    cpu_limiter: Arc<CpuLimiter>,
    /// 任务限制器
    task_limiter: Arc<Semaphore>,
    /// 配置
    config: ResourceConfig,
}

impl ResourceLimiter {
    pub fn new(config: ResourceConfig) -> Self {
        Self {
            memory_limiter: Arc::new(MemoryLimiter::new(config.max_memory_bytes)),
            cpu_limiter: Arc::new(CpuLimiter::new(config.max_cpu_usage)),
            task_limiter: Arc::new(Semaphore::new(config.max_concurrent_tasks)),
            config,
        }
    }

    /// 请求资源
    pub async fn acquire_resources(&self, estimated_memory: usize) -> Result<ResourceGuard, ApiError> {
        // 检查内存限制
        self.memory_limiter.check_available(estimated_memory).await?;
        
        // 检查CPU限制
        self.cpu_limiter.check_available().await?;
        
        // 获取任务许可
        let permit = self.task_limiter
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to acquire task permit".to_string()))?;
        
        // 分配内存
        self.memory_limiter.allocate(estimated_memory).await?;
        
        Ok(ResourceGuard {
            memory_allocated: estimated_memory,
            memory_limiter: self.memory_limiter.clone(),
            _permit: permit,
        })
    }

    /// 获取当前资源使用情况
    pub async fn get_usage(&self) -> ResourceUsage {
        ResourceUsage {
            memory_used_bytes: self.memory_limiter.get_used().await,
            memory_usage_percent: self.memory_limiter.get_usage_percent().await,
            cpu_usage_percent: self.cpu_limiter.get_usage().await,
            active_tasks: self.config.max_concurrent_tasks - self.task_limiter.available_permits(),
            waiting_tasks: 0, // 需要实际队列实现
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// 检查是否处于资源压力状态
    pub async fn is_under_pressure(&self) -> bool {
        let usage = self.get_usage().await;
        usage.memory_usage_percent > self.config.memory_pressure_threshold ||
        usage.cpu_usage_percent > self.config.cpu_pressure_threshold
    }
}

/// 资源守卫
pub struct ResourceGuard {
    memory_allocated: usize,
    memory_limiter: Arc<MemoryLimiter>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        // 释放内存
        let memory_limiter = self.memory_limiter.clone();
        let memory = self.memory_allocated;
        tokio::spawn(async move {
            memory_limiter.release(memory).await;
        });
    }
}

/// 内存限制器
struct MemoryLimiter {
    max_memory: usize,
    used_memory: Arc<RwLock<usize>>,
}

impl MemoryLimiter {
    fn new(max_memory: usize) -> Self {
        Self {
            max_memory,
            used_memory: Arc::new(RwLock::new(0)),
        }
    }

    async fn check_available(&self, required: usize) -> Result<(), ApiError> {
        let used = *self.used_memory.read().await;
        if used + required > self.max_memory {
            Err(ApiError::InternalServerError(
                format!("Insufficient memory: required={}, available={}", 
                    required, self.max_memory - used)
            ))
        } else {
            Ok(())
        }
    }

    async fn allocate(&self, amount: usize) -> Result<(), ApiError> {
        let mut used = self.used_memory.write().await;
        if *used + amount > self.max_memory {
            return Err(ApiError::InternalServerError("Memory allocation failed".to_string()));
        }
        *used += amount;
        Ok(())
    }

    async fn release(&self, amount: usize) {
        let mut used = self.used_memory.write().await;
        *used = used.saturating_sub(amount);
    }

    async fn get_used(&self) -> usize {
        *self.used_memory.read().await
    }

    async fn get_usage_percent(&self) -> f64 {
        let used = self.get_used().await;
        (used as f64 / self.max_memory as f64) * 100.0
    }
}

/// CPU限制器
struct CpuLimiter {
    max_cpu_usage: f64,
    current_usage: Arc<RwLock<f64>>,
}

impl CpuLimiter {
    fn new(max_cpu_usage: f64) -> Self {
        Self {
            max_cpu_usage,
            current_usage: Arc::new(RwLock::new(0.0)),
        }
    }

    async fn check_available(&self) -> Result<(), ApiError> {
        let usage = *self.current_usage.read().await;
        if usage > self.max_cpu_usage {
            Err(ApiError::InternalServerError(
                format!("CPU usage too high: current={:.1}%, max={:.1}%", 
                    usage * 100.0, self.max_cpu_usage * 100.0)
            ))
        } else {
            Ok(())
        }
    }

    async fn get_usage(&self) -> f64 {
        *self.current_usage.read().await * 100.0
    }

    // 实际实现需要系统调用获取真实CPU使用率
    async fn update_usage(&self, usage: f64) {
        let mut current = self.current_usage.write().await;
        *current = usage;
    }
}

/// 自适应资源管理器
pub struct AdaptiveResourceManager {
    limiter: Arc<ResourceLimiter>,
    history: Arc<RwLock<Vec<ResourceUsage>>>,
    config: ResourceConfig,
}

impl AdaptiveResourceManager {
    pub fn new(config: ResourceConfig) -> Self {
        let limiter = Arc::new(ResourceLimiter::new(config.clone()));
        
        Self {
            limiter,
            history: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// 启动监控
    pub async fn start_monitoring(&self) {
        let limiter = self.limiter.clone();
        let history = self.history.clone();
        let interval = self.config.monitoring_interval;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                // 获取当前使用情况
                let usage = limiter.get_usage().await;
                
                // 记录历史
                let mut hist = history.write().await;
                hist.push(usage.clone());
                
                // 保留最近1000条记录
                if hist.len() > 1000 {
                    hist.drain(0..100);
                }
                
                // 检查资源压力
                if limiter.is_under_pressure().await {
                    tracing::warn!("System under resource pressure: memory={:.1}%, cpu={:.1}%",
                        usage.memory_usage_percent, usage.cpu_usage_percent);
                }
            }
        });
    }

    /// 获取资源使用趋势
    pub async fn get_usage_trend(&self) -> ResourceTrend {
        let history = self.history.read().await;
        
        if history.is_empty() {
            return ResourceTrend::default();
        }
        
        let recent_count = history.len().min(20);
        let recent = &history[history.len() - recent_count..];
        
        let avg_memory = recent.iter()
            .map(|u| u.memory_usage_percent)
            .sum::<f64>() / recent_count as f64;
        
        let avg_cpu = recent.iter()
            .map(|u| u.cpu_usage_percent)
            .sum::<f64>() / recent_count as f64;
        
        let memory_trend = calculate_trend(
            recent.iter().map(|u| u.memory_usage_percent).collect()
        );
        
        let cpu_trend = calculate_trend(
            recent.iter().map(|u| u.cpu_usage_percent).collect()
        );
        
        ResourceTrend {
            avg_memory_usage: avg_memory,
            avg_cpu_usage: avg_cpu,
            memory_trend,
            cpu_trend,
        }
    }

    /// 自适应调整并发数
    pub async fn adapt_concurrency(&self) -> usize {
        let trend = self.get_usage_trend().await;
        let current_max = self.config.max_concurrent_tasks;
        
        if trend.avg_memory_usage > 80.0 || trend.avg_cpu_usage > 80.0 {
            // 资源紧张，减少并发
            (current_max as f64 * 0.8) as usize
        } else if trend.avg_memory_usage < 50.0 && trend.avg_cpu_usage < 50.0 {
            // 资源充足，增加并发
            (current_max as f64 * 1.2) as usize
        } else {
            // 保持当前并发数
            current_max
        }
    }
}

/// 资源使用趋势
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ResourceTrend {
    pub avg_memory_usage: f64,
    pub avg_cpu_usage: f64,
    pub memory_trend: Trend,
    pub cpu_trend: Trend,
}

/// 趋势
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum Trend {
    #[default]
    Stable,
    Increasing,
    Decreasing,
}

fn calculate_trend(values: Vec<f64>) -> Trend {
    if values.len() < 2 {
        return Trend::Stable;
    }
    
    let first_half_avg = values[..values.len()/2].iter().sum::<f64>() / (values.len()/2) as f64;
    let second_half_avg = values[values.len()/2..].iter().sum::<f64>() / (values.len() - values.len()/2) as f64;
    
    let diff = second_half_avg - first_half_avg;
    
    if diff > 5.0 {
        Trend::Increasing
    } else if diff < -5.0 {
        Trend::Decreasing
    } else {
        Trend::Stable
    }
}

/// 批处理资源分配器
pub struct BatchResourceAllocator {
    resource_manager: Arc<AdaptiveResourceManager>,
}

impl BatchResourceAllocator {
    pub fn new(resource_manager: Arc<AdaptiveResourceManager>) -> Self {
        Self { resource_manager }
    }

    /// 为批处理任务分配资源
    pub async fn allocate_for_batch(
        &self,
        tasks: &[crate::services::concurrent_chunking::ChunkingTask],
    ) -> Result<Vec<ResourceAllocation>, ApiError> {
        let mut allocations = Vec::new();
        
        // 计算总内存需求
        let total_memory_needed: usize = tasks.iter()
            .map(|t| estimate_memory_for_task(t))
            .sum();
        
        // 获取可用资源
        let usage = self.resource_manager.limiter.get_usage().await;
        let available_memory = (1.0 - usage.memory_usage_percent / 100.0) * 
            self.resource_manager.config.max_memory_bytes as f64;
        
        // 如果内存不足，进行比例分配
        let scale_factor = if total_memory_needed as f64 > available_memory {
            available_memory / total_memory_needed as f64
        } else {
            1.0
        };
        
        // 分配资源
        for task in tasks {
            let base_memory = estimate_memory_for_task(task);
            let allocated_memory = (base_memory as f64 * scale_factor) as usize;
            
            allocations.push(ResourceAllocation {
                task_id: task.document_id.clone(),
                memory_bytes: allocated_memory,
                cpu_shares: 1.0 / tasks.len() as f64,
                priority: task.priority as u8,
            });
        }
        
        Ok(allocations)
    }
}

/// 资源分配
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub task_id: String,
    pub memory_bytes: usize,
    pub cpu_shares: f64,
    pub priority: u8,
}

fn estimate_memory_for_task(task: &crate::services::concurrent_chunking::ChunkingTask) -> usize {
    // 基础内存 + 内容大小 * 膨胀系数
    let base_memory = 1024 * 1024; // 1MB
    let content_memory = task.content.len() * 10; // 假设处理时需要10倍内存
    base_memory + content_memory
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_limiter() {
        let config = ResourceConfig {
            max_memory_bytes: 1024 * 1024 * 100, // 100MB
            ..Default::default()
        };
        
        let limiter = ResourceLimiter::new(config);
        
        // 测试资源获取
        let guard = limiter.acquire_resources(1024 * 1024 * 10).await.unwrap(); // 10MB
        
        let usage = limiter.get_usage().await;
        assert!(usage.memory_used_bytes > 0);
        assert_eq!(usage.active_tasks, 1);
        
        // 释放资源
        drop(guard);
        
        // 等待异步释放
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let usage = limiter.get_usage().await;
        assert_eq!(usage.memory_used_bytes, 0);
    }

    #[test]
    fn test_trend_calculation() {
        let increasing = vec![10.0, 20.0, 30.0, 40.0];
        assert!(matches!(calculate_trend(increasing), Trend::Increasing));
        
        let decreasing = vec![40.0, 30.0, 20.0, 10.0];
        assert!(matches!(calculate_trend(decreasing), Trend::Decreasing));
        
        let stable = vec![20.0, 22.0, 21.0, 20.0];
        assert!(matches!(calculate_trend(stable), Trend::Stable));
    }
}