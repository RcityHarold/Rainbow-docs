//! 缓存性能基准测试

use std::time::{Duration, Instant};
use std::sync::Arc;
use crate::services::cache::{ChunkingCache, CacheConfig, CacheKey};
use crate::services::intelligent_chunker::ChunkingResult;
use crate::services::chunking::ChunkingStatistics;
use super::BenchmarkResult;

/// 缓存基准测试
pub struct CacheBenchmark {
    cache: Arc<ChunkingCache>,
    test_keys: Vec<CacheKey>,
    test_data: Vec<ChunkingResult>,
}

impl CacheBenchmark {
    pub fn new() -> Self {
        let config = CacheConfig::default();
        let cache = Arc::new(ChunkingCache::new(config));
        
        let (test_keys, test_data) = Self::generate_test_data();
        
        Self {
            cache,
            test_keys,
            test_data,
        }
    }

    fn generate_test_data() -> (Vec<CacheKey>, Vec<ChunkingResult>) {
        let mut keys = Vec::new();
        let mut data = Vec::new();

        for i in 0..100 {
            keys.push(CacheKey {
                document_id: format!("doc_{}", i),
                version: 1,
                strategy: "adaptive".to_string(),
            });

            data.push(ChunkingResult {
                chunks: vec![],
                structure: Default::default(),
                quality_assessments: vec![],
                statistics: ChunkingStatistics::default(),
            });
        }

        (keys, data)
    }

    /// 基准测试缓存写入性能
    pub async fn benchmark_write(&self) -> BenchmarkResult {
        let mut result = BenchmarkResult::new(
            "Cache Write Performance".to_string(),
            self.test_keys.len()
        );

        // 清空缓存
        self.cache.clear().await.unwrap();

        for (key, data) in self.test_keys.iter().zip(&self.test_data) {
            let start = Instant::now();
            self.cache.set(key.clone(), data.clone()).await.unwrap();
            let elapsed = start.elapsed();
            result.update(elapsed);
        }

        result.finalize();
        result.throughput = self.test_keys.len() as f64 / result.total_time.as_secs_f64();
        result
    }

    /// 基准测试缓存读取性能（命中）
    pub async fn benchmark_read_hit(&self) -> BenchmarkResult {
        // 预热缓存
        for (key, data) in self.test_keys.iter().zip(&self.test_data) {
            self.cache.set(key.clone(), data.clone()).await.unwrap();
        }

        let mut result = BenchmarkResult::new(
            "Cache Read Performance (Hit)".to_string(),
            self.test_keys.len()
        );

        for key in &self.test_keys {
            let start = Instant::now();
            let _ = self.cache.get(key).await;
            let elapsed = start.elapsed();
            result.update(elapsed);
        }

        result.finalize();
        result.throughput = self.test_keys.len() as f64 / result.total_time.as_secs_f64();
        result
    }

    /// 基准测试缓存读取性能（未命中）
    pub async fn benchmark_read_miss(&self) -> BenchmarkResult {
        // 清空缓存
        self.cache.clear().await.unwrap();

        let mut result = BenchmarkResult::new(
            "Cache Read Performance (Miss)".to_string(),
            self.test_keys.len()
        );

        for key in &self.test_keys {
            let start = Instant::now();
            let _ = self.cache.get(key).await;
            let elapsed = start.elapsed();
            result.update(elapsed);
        }

        result.finalize();
        result.throughput = self.test_keys.len() as f64 / result.total_time.as_secs_f64();
        result
    }

    /// 基准测试混合读写性能
    pub async fn benchmark_mixed_workload(&self) -> BenchmarkResult {
        let mut result = BenchmarkResult::new(
            "Cache Mixed Workload (70% read, 30% write)".to_string(),
            1000
        );

        // 预热一半的数据
        for i in 0..50 {
            self.cache.set(
                self.test_keys[i].clone(),
                self.test_data[i].clone()
            ).await.unwrap();
        }
        
        // 使用简单的伪随机数生成
        let mut seed = 42u64;
        
        for i in 0..1000 {
            // 简单的线性同余生成器
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let is_read = (seed % 100) < 70; // 70% 读操作
            let index = (seed as usize) % self.test_keys.len();
            
            let start = Instant::now();
            
            if is_read {
                let _ = self.cache.get(&self.test_keys[index]).await;
            } else {
                self.cache.set(
                    self.test_keys[index].clone(),
                    self.test_data[index].clone()
                ).await.unwrap();
            }
            
            let elapsed = start.elapsed();
            result.update(elapsed);
        }

        result.finalize();
        result.throughput = 1000.0 / result.total_time.as_secs_f64();
        result
    }

    /// 基准测试并发访问性能
    pub async fn benchmark_concurrent_access(&self) -> BenchmarkResult {
        use tokio::task::JoinSet;
        
        let mut result = BenchmarkResult::new(
            "Cache Concurrent Access".to_string(),
            100
        );

        // 预热缓存
        for i in 0..50 {
            self.cache.set(
                self.test_keys[i].clone(),
                self.test_data[i].clone()
            ).await.unwrap();
        }

        let start = Instant::now();
        let mut join_set = JoinSet::new();

        // 创建100个并发任务
        for i in 0..100 {
            let cache = self.cache.clone();
            let key = self.test_keys[i % self.test_keys.len()].clone();
            let data = self.test_data[i % self.test_data.len()].clone();
            
            join_set.spawn(async move {
                if i % 3 == 0 {
                    // 写操作
                    cache.set(key, data).await
                } else {
                    // 读操作
                    cache.get(&key).await.map(|_| ())
                        .ok_or(crate::error::ApiError::NotFound("Not in cache".to_string()))
                }
            });
        }

        let mut success_count = 0;
        while let Some(result) = join_set.join_next().await {
            if result.is_ok() {
                success_count += 1;
            }
        }

        result.total_time = start.elapsed();
        result.finalize();
        result.throughput = 100.0 / result.total_time.as_secs_f64();
        result.metadata = serde_json::json!({
            "success_count": success_count,
            "total_tasks": 100
        });

        result
    }

    /// 基准测试缓存失效性能
    pub async fn benchmark_invalidation(&self) -> BenchmarkResult {
        let mut result = BenchmarkResult::new(
            "Cache Invalidation Performance".to_string(),
            self.test_keys.len()
        );

        // 预热缓存
        for (key, data) in self.test_keys.iter().zip(&self.test_data) {
            self.cache.set(key.clone(), data.clone()).await.unwrap();
        }

        // 测试单个失效
        for key in &self.test_keys {
            let start = Instant::now();
            self.cache.invalidate(key).await.unwrap();
            let elapsed = start.elapsed();
            result.update(elapsed);
        }

        result.finalize();
        result.throughput = self.test_keys.len() as f64 / result.total_time.as_secs_f64();
        result
    }
}

/// 运行完整的缓存基准测试套件
pub async fn run_cache_benchmarks() -> String {
    let benchmark = CacheBenchmark::new();
    let mut report = String::from("=== Cache Performance Benchmarks ===\n\n");

    // 写入性能
    let write_result = benchmark.benchmark_write().await;
    report.push_str(&format!("Write: {:.2} ops/sec, avg {:?}\n", 
        write_result.throughput, write_result.average_time));

    // 读取性能（命中）
    let read_hit_result = benchmark.benchmark_read_hit().await;
    report.push_str(&format!("Read (hit): {:.2} ops/sec, avg {:?}\n", 
        read_hit_result.throughput, read_hit_result.average_time));

    // 读取性能（未命中）
    let read_miss_result = benchmark.benchmark_read_miss().await;
    report.push_str(&format!("Read (miss): {:.2} ops/sec, avg {:?}\n", 
        read_miss_result.throughput, read_miss_result.average_time));

    // 混合负载
    let mixed_result = benchmark.benchmark_mixed_workload().await;
    report.push_str(&format!("Mixed workload: {:.2} ops/sec\n", 
        mixed_result.throughput));

    // 并发访问
    let concurrent_result = benchmark.benchmark_concurrent_access().await;
    report.push_str(&format!("Concurrent access: {:.2} ops/sec\n", 
        concurrent_result.throughput));

    // 失效性能
    let invalidation_result = benchmark.benchmark_invalidation().await;
    report.push_str(&format!("Invalidation: {:.2} ops/sec\n", 
        invalidation_result.throughput));

    // 获取缓存统计
    let stats = benchmark.cache.get_stats().await;
    report.push_str(&format!("\n## Cache Statistics\n"));
    report.push_str(&format!("Hit rate: {:.2}%\n", stats.hit_rate() * 100.0));
    report.push_str(&format!("L1 hit rate: {:.2}%\n", stats.l1_hit_rate() * 100.0));

    report
}

// 使用内置的伪随机数生成，避免外部依赖

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_benchmark() {
        let benchmark = CacheBenchmark::new();
        let result = benchmark.benchmark_write().await;
        
        assert!(result.iterations > 0);
        assert!(result.throughput > 0.0);
    }
}