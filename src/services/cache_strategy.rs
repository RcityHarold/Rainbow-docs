//! 缓存策略优化
//! 
//! 提供智能的缓存策略，包括预测性缓存、自适应TTL等

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::{HashMap, VecDeque};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use crate::services::cache::{CacheKey, ChunkingCache};

/// 访问模式分析器
pub struct AccessPatternAnalyzer {
    /// 访问历史记录
    access_history: Arc<RwLock<VecDeque<AccessRecord>>>,
    /// 文档访问频率
    document_frequency: Arc<RwLock<HashMap<String, AccessFrequency>>>,
    /// 配置
    config: AnalyzerConfig,
}

#[derive(Debug, Clone)]
struct AccessRecord {
    document_id: String,
    timestamp: Instant,
    strategy: String,
}

#[derive(Debug, Clone)]
struct AccessFrequency {
    count: u64,
    last_access: Instant,
    access_intervals: VecDeque<Duration>,
}

#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// 历史记录保留时间
    pub history_retention: Duration,
    /// 最大历史记录数
    pub max_history_size: usize,
    /// 访问间隔统计窗口大小
    pub interval_window_size: usize,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            history_retention: Duration::from_secs(3600 * 24), // 24小时
            max_history_size: 10000,
            interval_window_size: 10,
        }
    }
}

impl AccessPatternAnalyzer {
    pub fn new(config: AnalyzerConfig) -> Self {
        Self {
            access_history: Arc::new(RwLock::new(VecDeque::new())),
            document_frequency: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// 记录访问
    pub async fn record_access(&self, document_id: &str, strategy: &str) {
        let record = AccessRecord {
            document_id: document_id.to_string(),
            timestamp: Instant::now(),
            strategy: strategy.to_string(),
        };

        // 更新访问历史
        {
            let mut history = self.access_history.write().await;
            history.push_back(record);
            
            // 清理过期记录
            while history.len() > self.config.max_history_size {
                history.pop_front();
            }
        }

        // 更新频率统计
        {
            let mut frequency = self.document_frequency.write().await;
            let entry = frequency.entry(document_id.to_string())
                .or_insert(AccessFrequency {
                    count: 0,
                    last_access: Instant::now(),
                    access_intervals: VecDeque::new(),
                });

            // 计算访问间隔
            let interval = entry.last_access.elapsed();
            if entry.count > 0 {
                entry.access_intervals.push_back(interval);
                if entry.access_intervals.len() > self.config.interval_window_size {
                    entry.access_intervals.pop_front();
                }
            }

            entry.count += 1;
            entry.last_access = Instant::now();
        }
    }

    /// 预测下次访问时间
    pub async fn predict_next_access(&self, document_id: &str) -> Option<Duration> {
        let frequency = self.document_frequency.read().await;
        
        if let Some(freq) = frequency.get(document_id) {
            if freq.access_intervals.is_empty() {
                return None;
            }

            // 计算平均访问间隔
            let total: Duration = freq.access_intervals.iter().sum();
            let avg_interval = total / freq.access_intervals.len() as u32;
            
            Some(avg_interval)
        } else {
            None
        }
    }

    /// 获取热点文档
    pub async fn get_hot_documents(&self, top_n: usize) -> Vec<(String, u64)> {
        let frequency = self.document_frequency.read().await;
        let mut docs: Vec<_> = frequency
            .iter()
            .map(|(id, freq)| (id.clone(), freq.count))
            .collect();
        
        docs.sort_by(|a, b| b.1.cmp(&a.1));
        docs.truncate(top_n);
        docs
    }
}

/// 自适应TTL策略
pub struct AdaptiveTTLStrategy {
    /// 基础TTL
    base_ttl: Duration,
    /// 最小TTL
    min_ttl: Duration,
    /// 最大TTL
    max_ttl: Duration,
    /// 访问模式分析器
    analyzer: Arc<AccessPatternAnalyzer>,
}

impl AdaptiveTTLStrategy {
    pub fn new(base_ttl: Duration, analyzer: Arc<AccessPatternAnalyzer>) -> Self {
        Self {
            base_ttl,
            min_ttl: Duration::from_secs(60), // 1分钟
            max_ttl: Duration::from_secs(3600 * 24), // 24小时
            analyzer,
        }
    }

    /// 计算自适应TTL
    pub async fn calculate_ttl(&self, document_id: &str) -> Duration {
        // 基于预测的下次访问时间计算TTL
        if let Some(predicted_interval) = self.analyzer.predict_next_access(document_id).await {
            // TTL = 预测间隔 * 1.2 (留20%余量)
            let ttl = predicted_interval.mul_f32(1.2);
            
            // 限制在最小和最大值之间
            if ttl < self.min_ttl {
                self.min_ttl
            } else if ttl > self.max_ttl {
                self.max_ttl
            } else {
                ttl
            }
        } else {
            // 没有历史数据，使用基础TTL
            self.base_ttl
        }
    }
}

/// 预测性缓存策略
pub struct PredictiveCacheStrategy {
    /// 访问模式分析器
    analyzer: Arc<AccessPatternAnalyzer>,
    /// 相关文档映射
    related_documents: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl PredictiveCacheStrategy {
    pub fn new(analyzer: Arc<AccessPatternAnalyzer>) -> Self {
        Self {
            analyzer,
            related_documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 学习文档关联关系
    pub async fn learn_relationships(&self, access_sequence: Vec<String>) {
        if access_sequence.len() < 2 {
            return;
        }

        let mut related = self.related_documents.write().await;
        
        // 滑动窗口分析相邻访问
        for window in access_sequence.windows(2) {
            let doc1 = &window[0];
            let doc2 = &window[1];
            
            // 记录关联关系
            related.entry(doc1.clone())
                .or_insert_with(Vec::new)
                .push(doc2.clone());
        }
    }

    /// 预测相关文档
    pub async fn predict_related(&self, document_id: &str) -> Vec<String> {
        let related = self.related_documents.read().await;
        
        if let Some(docs) = related.get(document_id) {
            // 统计出现频率最高的相关文档
            let mut frequency: HashMap<String, usize> = HashMap::new();
            for doc in docs {
                *frequency.entry(doc.clone()).or_insert(0) += 1;
            }
            
            // 按频率排序
            let mut sorted: Vec<_> = frequency.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            
            // 返回前5个最相关的文档
            sorted.into_iter()
                .take(5)
                .map(|(doc, _)| doc)
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// 智能缓存管理器
pub struct SmartCacheManager {
    /// 缓存实例
    cache: Arc<ChunkingCache>,
    /// 访问模式分析器
    analyzer: Arc<AccessPatternAnalyzer>,
    /// TTL策略
    ttl_strategy: Arc<AdaptiveTTLStrategy>,
    /// 预测策略
    predictive_strategy: Arc<PredictiveCacheStrategy>,
}

impl SmartCacheManager {
    pub fn new(cache: Arc<ChunkingCache>, base_ttl: Duration) -> Self {
        let analyzer = Arc::new(AccessPatternAnalyzer::new(Default::default()));
        let ttl_strategy = Arc::new(AdaptiveTTLStrategy::new(base_ttl, analyzer.clone()));
        let predictive_strategy = Arc::new(PredictiveCacheStrategy::new(analyzer.clone()));

        Self {
            cache,
            analyzer,
            ttl_strategy,
            predictive_strategy,
        }
    }

    /// 智能获取（带预测性预热）
    pub async fn smart_get(&self, key: &CacheKey) -> Option<crate::services::intelligent_chunker::ChunkingResult> {
        // 记录访问
        self.analyzer.record_access(&key.document_id, &key.strategy).await;

        // 获取缓存
        let result = self.cache.get(key).await;

        // 如果命中，触发预测性预热
        if result.is_some() {
            tokio::spawn({
                let predictive = self.predictive_strategy.clone();
                let cache = self.cache.clone();
                let document_id = key.document_id.clone();
                
                async move {
                    // 预测相关文档
                    let related = predictive.predict_related(&document_id).await;
                    
                    // 预热相关文档（这里简化处理，实际需要调用分块服务）
                    for related_doc in related {
                        tracing::debug!("Predictive cache warmup for document: {}", related_doc);
                    }
                }
            });
        }

        result
    }

    /// 定期优化缓存
    pub async fn optimize(&self) {
        // 获取热点文档
        let hot_docs = self.analyzer.get_hot_documents(100).await;
        
        // 调整热点文档的TTL
        for (doc_id, _count) in hot_docs {
            let ttl = self.ttl_strategy.calculate_ttl(&doc_id).await;
            tracing::debug!("Adjusted TTL for document {}: {:?}", doc_id, ttl);
        }

        // 清理冷数据
        let stats = self.cache.get_stats().await;
        if stats.hit_rate() < 0.3 {
            // 命中率太低，可能需要调整缓存策略
            tracing::warn!("Low cache hit rate: {:.2}%", stats.hit_rate() * 100.0);
        }
    }
}

/// 缓存预热器
pub struct CacheWarmer {
    cache: Arc<ChunkingCache>,
    analyzer: Arc<AccessPatternAnalyzer>,
}

impl CacheWarmer {
    pub fn new(cache: Arc<ChunkingCache>, analyzer: Arc<AccessPatternAnalyzer>) -> Self {
        Self { cache, analyzer }
    }

    /// 执行缓存预热
    pub async fn warmup(&self) -> Result<(), crate::error::ApiError> {
        // 获取热点文档
        let hot_docs = self.analyzer.get_hot_documents(50).await;
        
        tracing::info!("Starting cache warmup for {} hot documents", hot_docs.len());
        
        // 并发预热
        let tasks: Vec<_> = hot_docs
            .into_iter()
            .map(|(doc_id, _)| {
                let cache = self.cache.clone();
                tokio::spawn(async move {
                    // 这里需要调用实际的分块服务来生成数据
                    tracing::debug!("Warming up document: {}", doc_id);
                })
            })
            .collect();

        // 等待所有预热任务完成
        for task in tasks {
            let _ = task.await;
        }

        tracing::info!("Cache warmup completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_access_pattern_analyzer() {
        let analyzer = AccessPatternAnalyzer::new(Default::default());
        
        // 记录访问
        analyzer.record_access("doc1", "adaptive").await;
        analyzer.record_access("doc2", "structural").await;
        analyzer.record_access("doc1", "adaptive").await;
        
        // 获取热点文档
        let hot_docs = analyzer.get_hot_documents(10).await;
        assert!(!hot_docs.is_empty());
        assert_eq!(hot_docs[0].0, "doc1");
        assert_eq!(hot_docs[0].1, 2);
    }

    #[tokio::test]
    async fn test_adaptive_ttl() {
        let analyzer = Arc::new(AccessPatternAnalyzer::new(Default::default()));
        let ttl_strategy = AdaptiveTTLStrategy::new(
            Duration::from_secs(3600),
            analyzer.clone()
        );
        
        // 第一次访问，应该返回基础TTL
        let ttl = ttl_strategy.calculate_ttl("doc1").await;
        assert_eq!(ttl, Duration::from_secs(3600));
        
        // 记录多次访问
        for _ in 0..3 {
            analyzer.record_access("doc1", "adaptive").await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // TTL应该根据访问模式调整
        let adaptive_ttl = ttl_strategy.calculate_ttl("doc1").await;
        assert!(adaptive_ttl >= ttl_strategy.min_ttl);
        assert!(adaptive_ttl <= ttl_strategy.max_ttl);
    }
}