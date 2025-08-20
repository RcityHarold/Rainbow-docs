//! 性能基准测试模块

pub mod chunking_benchmark;
pub mod cache_benchmark;
pub mod concurrent_benchmark;

use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

/// 基准测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub test_name: String,
    pub iterations: usize,
    pub total_time: Duration,
    pub average_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
    pub throughput: f64,
    pub metadata: serde_json::Value,
}

impl BenchmarkResult {
    pub fn new(test_name: String, iterations: usize) -> Self {
        Self {
            test_name,
            iterations,
            total_time: Duration::from_secs(0),
            average_time: Duration::from_secs(0),
            min_time: Duration::from_secs(u64::MAX),
            max_time: Duration::from_secs(0),
            throughput: 0.0,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn update(&mut self, elapsed: Duration) {
        self.total_time += elapsed;
        if elapsed < self.min_time {
            self.min_time = elapsed;
        }
        if elapsed > self.max_time {
            self.max_time = elapsed;
        }
    }

    pub fn finalize(&mut self) {
        if self.iterations > 0 {
            self.average_time = self.total_time / self.iterations as u32;
        }
    }
}

/// 基准测试运行器
pub struct BenchmarkRunner {
    results: Vec<BenchmarkResult>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub async fn run_benchmark<F, Fut>(
        &mut self,
        name: &str,
        iterations: usize,
        setup: F,
    ) -> BenchmarkResult
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Duration>,
    {
        let mut result = BenchmarkResult::new(name.to_string(), iterations);

        for _ in 0..iterations {
            let elapsed = setup().await;
            result.update(elapsed);
        }

        result.finalize();
        self.results.push(result.clone());
        result
    }

    pub fn get_results(&self) -> &[BenchmarkResult] {
        &self.results
    }

    pub fn generate_report(&self) -> String {
        let mut report = String::from("=== Performance Benchmark Report ===\n\n");

        for result in &self.results {
            report.push_str(&format!("Test: {}\n", result.test_name));
            report.push_str(&format!("  Iterations: {}\n", result.iterations));
            report.push_str(&format!("  Total Time: {:?}\n", result.total_time));
            report.push_str(&format!("  Average Time: {:?}\n", result.average_time));
            report.push_str(&format!("  Min Time: {:?}\n", result.min_time));
            report.push_str(&format!("  Max Time: {:?}\n", result.max_time));
            report.push_str(&format!("  Throughput: {:.2} ops/sec\n", result.throughput));
            report.push_str("\n");
        }

        report
    }
}