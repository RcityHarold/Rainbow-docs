//! 多级缓存系统
//! 
//! 提供高性能的文档分块缓存，支持多级缓存策略

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use crate::error::ApiError;
use crate::services::chunking::EnhancedDocumentChunk;
use crate::services::intelligent_chunker::ChunkingResult;

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// L1 缓存大小（内存缓存）
    pub l1_capacity: usize,
    /// L2 缓存大小（Redis缓存）
    pub l2_capacity: usize,
    /// 缓存过期时间
    pub ttl: Duration,
    /// 是否启用L2缓存
    pub enable_l2: bool,
    /// 预热策略
    pub warmup_strategy: WarmupStrategy,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_capacity: 1000,
            l2_capacity: 10000,
            ttl: Duration::from_secs(3600), // 1小时
            enable_l2: true,
            warmup_strategy: WarmupStrategy::Lazy,
        }
    }
}

/// 缓存预热策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarmupStrategy {
    /// 延迟加载
    Lazy,
    /// 启动时预热
    Eager,
    /// 定时预热
    Scheduled { interval: Duration },
}

/// 缓存键
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    pub document_id: String,
    pub version: u64,
    pub strategy: String,
}

/// 缓存项
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub data: ChunkingResult,
    pub created_at: Instant,
    pub access_count: u64,
    pub last_accessed: Instant,
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub total_requests: u64,
    pub evictions: u64,
    pub avg_response_time: f64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        let total_hits = self.l1_hits + self.l2_hits;
        total_hits as f64 / self.total_requests as f64
    }

    pub fn l1_hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.l1_hits as f64 / self.total_requests as f64
    }
}

/// 简单的LRU缓存实现
struct SimpleLruCache {
    capacity: usize,
    data: HashMap<CacheKey, CacheEntry>,
    access_order: VecDeque<CacheKey>,
}

impl SimpleLruCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: HashMap::new(),
            access_order: VecDeque::new(),
        }
    }

    fn get(&mut self, key: &CacheKey) -> Option<&CacheEntry> {
        if let Some(entry) = self.data.get(key) {
            // 更新访问顺序
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                let key = self.access_order.remove(pos).unwrap();
                self.access_order.push_back(key);
            }
            Some(entry)
        } else {
            None
        }
    }

    fn get_mut(&mut self, key: &CacheKey) -> Option<&mut CacheEntry> {
        if self.data.contains_key(key) {
            // 更新访问顺序
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                let key = self.access_order.remove(pos).unwrap();
                self.access_order.push_back(key);
            }
            self.data.get_mut(key)
        } else {
            None
        }
    }

    fn put(&mut self, key: CacheKey, value: CacheEntry) -> Option<CacheEntry> {
        let evicted = if self.data.len() >= self.capacity && !self.data.contains_key(&key) {
            // 需要逐出最老的项
            if let Some(old_key) = self.access_order.pop_front() {
                self.data.remove(&old_key)
            } else {
                None
            }
        } else {
            None
        };

        // 如果key已存在，先移除旧的访问记录
        if self.data.contains_key(&key) {
            if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
                self.access_order.remove(pos);
            }
        }

        self.data.insert(key.clone(), value);
        self.access_order.push_back(key);

        evicted
    }

    fn pop(&mut self, key: &CacheKey) -> Option<CacheEntry> {
        if let Some(pos) = self.access_order.iter().position(|k| k == key) {
            self.access_order.remove(pos);
        }
        self.data.remove(key)
    }

    fn clear(&mut self) {
        self.data.clear();
        self.access_order.clear();
    }

    fn iter(&self) -> impl Iterator<Item = (&CacheKey, &CacheEntry)> {
        self.data.iter()
    }
}

/// 多级缓存系统
pub struct ChunkingCache {
    /// L1缓存（内存）
    l1_cache: Arc<RwLock<SimpleLruCache>>,
    /// L2缓存（Redis）- 这里简化为内存实现，实际可替换为Redis
    l2_cache: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,
    /// 缓存配置
    config: CacheConfig,
    /// 统计信息
    stats: Arc<RwLock<CacheStats>>,
}

impl ChunkingCache {
    /// 创建新的缓存实例
    pub fn new(config: CacheConfig) -> Self {
        let l1_cache = SimpleLruCache::new(config.l1_capacity);
        
        Self {
            l1_cache: Arc::new(RwLock::new(l1_cache)),
            l2_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats {
                l1_hits: 0,
                l1_misses: 0,
                l2_hits: 0,
                l2_misses: 0,
                total_requests: 0,
                evictions: 0,
                avg_response_time: 0.0,
            })),
        }
    }

    /// 获取缓存项
    pub async fn get(&self, key: &CacheKey) -> Option<ChunkingResult> {
        let start = Instant::now();
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;

        // 先查L1缓存
        {
            let mut l1_cache = self.l1_cache.write().await;
            if let Some(entry) = l1_cache.get_mut(key) {
                // 检查是否过期
                if entry.created_at.elapsed() < self.config.ttl {
                    entry.access_count += 1;
                    entry.last_accessed = Instant::now();
                    stats.l1_hits += 1;
                    
                    let elapsed = start.elapsed().as_micros() as f64;
                    self.update_avg_response_time(&mut stats, elapsed).await;
                    
                    return Some(entry.data.clone());
                } else {
                    // 过期，移除
                    l1_cache.pop(key);
                }
            }
        }

        stats.l1_misses += 1;

        // L1未命中，查L2缓存
        if self.config.enable_l2 {
            let mut l2_cache = self.l2_cache.write().await;
            if let Some(entry) = l2_cache.get_mut(key) {
                // 检查是否过期
                if entry.created_at.elapsed() < self.config.ttl {
                    entry.access_count += 1;
                    entry.last_accessed = Instant::now();
                    stats.l2_hits += 1;
                    
                    // 提升到L1缓存
                    let mut l1_cache = self.l1_cache.write().await;
                    l1_cache.put(key.clone(), entry.clone());
                    
                    let elapsed = start.elapsed().as_micros() as f64;
                    self.update_avg_response_time(&mut stats, elapsed).await;
                    
                    return Some(entry.data.clone());
                } else {
                    // 过期，移除
                    l2_cache.remove(key);
                }
            }
            stats.l2_misses += 1;
        }

        let elapsed = start.elapsed().as_micros() as f64;
        self.update_avg_response_time(&mut stats, elapsed).await;
        
        None
    }

    /// 设置缓存项
    pub async fn set(&self, key: CacheKey, value: ChunkingResult) -> Result<(), ApiError> {
        let entry = CacheEntry {
            data: value,
            created_at: Instant::now(),
            access_count: 1,
            last_accessed: Instant::now(),
        };

        // 先写入L1缓存
        {
            let mut l1_cache = self.l1_cache.write().await;
            if let Some(_) = l1_cache.put(key.clone(), entry.clone()) {
                // 有项被逐出
                let mut stats = self.stats.write().await;
                stats.evictions += 1;
            }
        }

        // 同时写入L2缓存
        if self.config.enable_l2 {
            let mut l2_cache = self.l2_cache.write().await;
            
            // 检查L2缓存大小限制
            if l2_cache.len() >= self.config.l2_capacity {
                // 简单的LRU逐出策略
                if let Some(oldest_key) = self.find_oldest_entry(&l2_cache).await {
                    l2_cache.remove(&oldest_key);
                    let mut stats = self.stats.write().await;
                    stats.evictions += 1;
                }
            }
            
            l2_cache.insert(key, entry);
        }

        Ok(())
    }

    /// 删除缓存项
    pub async fn invalidate(&self, key: &CacheKey) -> Result<(), ApiError> {
        // 从L1缓存删除
        {
            let mut l1_cache = self.l1_cache.write().await;
            l1_cache.pop(key);
        }

        // 从L2缓存删除
        if self.config.enable_l2 {
            let mut l2_cache = self.l2_cache.write().await;
            l2_cache.remove(key);
        }

        Ok(())
    }

    /// 批量删除文档相关的所有缓存
    pub async fn invalidate_document(&self, document_id: &str) -> Result<(), ApiError> {
        // 收集要删除的键
        let mut keys_to_remove = Vec::new();

        // 从L1缓存收集
        {
            let l1_cache = self.l1_cache.read().await;
            for (key, _) in l1_cache.iter() {
                if key.document_id == document_id {
                    keys_to_remove.push(key.clone());
                }
            }
        }

        // 从L2缓存收集
        if self.config.enable_l2 {
            let l2_cache = self.l2_cache.read().await;
            for key in l2_cache.keys() {
                if key.document_id == document_id {
                    keys_to_remove.push(key.clone());
                }
            }
        }

        // 执行删除
        for key in keys_to_remove {
            self.invalidate(&key).await?;
        }

        Ok(())
    }

    /// 清空所有缓存
    pub async fn clear(&self) -> Result<(), ApiError> {
        {
            let mut l1_cache = self.l1_cache.write().await;
            l1_cache.clear();
        }

        if self.config.enable_l2 {
            let mut l2_cache = self.l2_cache.write().await;
            l2_cache.clear();
        }

        // 重置统计信息
        let mut stats = self.stats.write().await;
        *stats = CacheStats {
            l1_hits: 0,
            l1_misses: 0,
            l2_hits: 0,
            l2_misses: 0,
            total_requests: 0,
            evictions: 0,
            avg_response_time: 0.0,
        };

        Ok(())
    }

    /// 获取缓存统计信息
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// 预热缓存
    pub async fn warmup(&self, documents: Vec<(String, u64)>) -> Result<(), ApiError> {
        match &self.config.warmup_strategy {
            WarmupStrategy::Lazy => {
                // 延迟加载，不做任何操作
                Ok(())
            }
            WarmupStrategy::Eager => {
                // 立即加载热点文档
                // 这里需要调用实际的分块服务来生成数据
                // 简化实现，实际需要注入分块服务
                tracing::info!("Cache warmup started for {} documents", documents.len());
                Ok(())
            }
            WarmupStrategy::Scheduled { interval } => {
                // 定时预热，这里只是标记，实际预热由外部调度器触发
                tracing::info!("Scheduled cache warmup configured with interval: {:?}", interval);
                Ok(())
            }
        }
    }

    /// 查找最老的缓存项
    async fn find_oldest_entry(&self, cache: &HashMap<CacheKey, CacheEntry>) -> Option<CacheKey> {
        cache.iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone())
    }

    /// 更新平均响应时间
    async fn update_avg_response_time(&self, stats: &mut CacheStats, elapsed: f64) {
        let total = stats.total_requests as f64;
        stats.avg_response_time = 
            (stats.avg_response_time * (total - 1.0) + elapsed) / total;
    }
}

/// 缓存管理器
pub struct CacheManager {
    caches: HashMap<String, Arc<ChunkingCache>>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            caches: HashMap::new(),
        }
    }

    /// 获取或创建缓存实例
    pub fn get_or_create(&mut self, name: &str, config: CacheConfig) -> Arc<ChunkingCache> {
        self.caches
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(ChunkingCache::new(config)))
            .clone()
    }

    /// 获取所有缓存的统计信息
    pub async fn get_all_stats(&self) -> HashMap<String, CacheStats> {
        let mut all_stats = HashMap::new();
        
        for (name, cache) in &self.caches {
            all_stats.insert(name.clone(), cache.get_stats().await);
        }
        
        all_stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let config = CacheConfig::default();
        let cache = ChunkingCache::new(config);

        let key = CacheKey {
            document_id: "doc1".to_string(),
            version: 1,
            strategy: "adaptive".to_string(),
        };

        // 测试缓存未命中
        assert!(cache.get(&key).await.is_none());

        // 测试缓存设置和获取
        let result = ChunkingResult {
            chunks: vec![],
            structure: Default::default(),
            quality_assessments: vec![],
            statistics: Default::default(),
        };

        cache.set(key.clone(), result.clone()).await.unwrap();
        assert!(cache.get(&key).await.is_some());

        // 测试统计信息
        let stats = cache.get_stats().await;
        assert_eq!(stats.l1_hits, 1);
        assert_eq!(stats.l1_misses, 1);
        assert_eq!(stats.total_requests, 2);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let config = CacheConfig::default();
        let cache = ChunkingCache::new(config);

        let key = CacheKey {
            document_id: "doc1".to_string(),
            version: 1,
            strategy: "adaptive".to_string(),
        };

        let result = ChunkingResult {
            chunks: vec![],
            structure: Default::default(),
            quality_assessments: vec![],
            statistics: Default::default(),
        };

        // 设置缓存
        cache.set(key.clone(), result).await.unwrap();
        assert!(cache.get(&key).await.is_some());

        // 删除缓存
        cache.invalidate(&key).await.unwrap();
        assert!(cache.get(&key).await.is_none());
    }
}