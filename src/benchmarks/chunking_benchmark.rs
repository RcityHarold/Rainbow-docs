//! 分块性能基准测试

use std::time::{Duration, Instant};
use crate::services::chunking::{ChunkingConfig, ChunkingStrategy};
use crate::services::intelligent_chunker::IntelligentChunker;
use super::BenchmarkResult;

/// 分块基准测试套件
pub struct ChunkingBenchmark {
    test_documents: Vec<TestDocument>,
}

#[derive(Clone)]
struct TestDocument {
    id: String,
    title: String,
    content: String,
    size_category: SizeCategory,
}

#[derive(Clone, Debug)]
enum SizeCategory {
    Small,   // < 1KB
    Medium,  // 1KB - 10KB
    Large,   // 10KB - 100KB
    XLarge,  // > 100KB
}

impl ChunkingBenchmark {
    pub fn new() -> Self {
        Self {
            test_documents: Self::generate_test_documents(),
        }
    }

    fn generate_test_documents() -> Vec<TestDocument> {
        let mut docs = Vec::new();

        // 小文档
        docs.push(TestDocument {
            id: "small_1".to_string(),
            title: "Small Document".to_string(),
            content: "This is a small test document.\n\n".repeat(10),
            size_category: SizeCategory::Small,
        });

        // 中等文档
        let medium_content = r#"
# Medium Document

## Introduction
This is a medium-sized document for testing chunking performance.

## Content Section
Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.

### Subsection 1
More content here...

### Subsection 2
Additional content...

## Code Example
```rust
fn main() {
    println!("Hello, world!");
}
```

## Conclusion
This concludes our medium document.
"#.repeat(5);

        docs.push(TestDocument {
            id: "medium_1".to_string(),
            title: "Medium Document".to_string(),
            content: medium_content,
            size_category: SizeCategory::Medium,
        });

        // 大文档
        let large_content = Self::generate_large_document();
        docs.push(TestDocument {
            id: "large_1".to_string(),
            title: "Large Document".to_string(),
            content: large_content,
            size_category: SizeCategory::Large,
        });

        // 超大文档
        let xlarge_content = Self::generate_xlarge_document();
        docs.push(TestDocument {
            id: "xlarge_1".to_string(),
            title: "Extra Large Document".to_string(),
            content: xlarge_content,
            size_category: SizeCategory::XLarge,
        });

        docs
    }

    fn generate_large_document() -> String {
        let mut content = String::new();
        
        for i in 1..=20 {
            content.push_str(&format!("\n# Chapter {}\n\n", i));
            
            for j in 1..=5 {
                content.push_str(&format!("## Section {}.{}\n\n", i, j));
                content.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ");
                content.push_str("Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ");
                content.push_str("Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.\n\n");
                
                if j % 2 == 0 {
                    content.push_str("```python\n");
                    content.push_str("def example_function():\n");
                    content.push_str("    return 'Hello, World!'\n");
                    content.push_str("```\n\n");
                }
            }
        }
        
        content
    }

    fn generate_xlarge_document() -> String {
        Self::generate_large_document().repeat(5)
    }

    /// 运行单策略基准测试
    pub async fn benchmark_strategy(&self, strategy: ChunkingStrategy) -> BenchmarkResult {
        let config = ChunkingConfig {
            strategy,
            ..Default::default()
        };

        let chunker = IntelligentChunker::new(config).unwrap();
        let mut result = BenchmarkResult::new(
            format!("Chunking Strategy: {:?}", strategy),
            self.test_documents.len()
        );

        for doc in &self.test_documents {
            let start = Instant::now();
            
            let _ = chunker.chunk_document(&doc.id, &doc.title, &doc.content).await;
            
            let elapsed = start.elapsed();
            result.update(elapsed);
        }

        result.finalize();
        
        // 计算吞吐量
        let total_bytes: usize = self.test_documents.iter()
            .map(|d| d.content.len())
            .sum();
        let total_seconds = result.total_time.as_secs_f64();
        result.throughput = (total_bytes as f64 / 1024.0 / 1024.0) / total_seconds; // MB/s

        result
    }

    /// 运行所有策略的基准测试
    pub async fn benchmark_all_strategies(&self) -> Vec<BenchmarkResult> {
        let strategies = vec![
            ChunkingStrategy::Simple,
            ChunkingStrategy::Structural,
            ChunkingStrategy::Semantic,
            ChunkingStrategy::Hybrid,
            ChunkingStrategy::Adaptive,
        ];

        let mut results = Vec::new();

        for strategy in strategies {
            let result = self.benchmark_strategy(strategy).await;
            results.push(result);
        }

        results
    }

    /// 基准测试不同文档大小
    pub async fn benchmark_by_size(&self) -> Vec<BenchmarkResult> {
        let config = ChunkingConfig::default();
        let chunker = IntelligentChunker::new(config).unwrap();
        let mut results = Vec::new();

        // 按大小分组
        let size_groups = vec![
            (SizeCategory::Small, "Small Documents"),
            (SizeCategory::Medium, "Medium Documents"),
            (SizeCategory::Large, "Large Documents"),
            (SizeCategory::XLarge, "Extra Large Documents"),
        ];

        for (size_category, name) in size_groups {
            let docs: Vec<_> = self.test_documents.iter()
                .filter(|d| matches!(&d.size_category, cat if std::mem::discriminant(cat) == std::mem::discriminant(&size_category)))
                .collect();

            if docs.is_empty() {
                continue;
            }

            let mut result = BenchmarkResult::new(name.to_string(), docs.len());

            for doc in docs {
                let start = Instant::now();
                let _ = chunker.chunk_document(&doc.id, &doc.title, &doc.content).await;
                let elapsed = start.elapsed();
                result.update(elapsed);
            }

            result.finalize();
            results.push(result);
        }

        results
    }

    /// 基准测试并发性能
    pub async fn benchmark_concurrency(&self) -> BenchmarkResult {
        use tokio::task::JoinSet;
        
        let config = ChunkingConfig::default();
        let mut result = BenchmarkResult::new(
            "Concurrent Chunking".to_string(),
            self.test_documents.len()
        );

        let start = Instant::now();
        let mut join_set = JoinSet::new();

        for doc in &self.test_documents {
            let config_clone = config.clone();
            let doc_clone = doc.clone();
            
            join_set.spawn(async move {
                let chunker = IntelligentChunker::new(config_clone).unwrap();
                chunker.chunk_document(&doc_clone.id, &doc_clone.title, &doc_clone.content).await
            });
        }

        while let Some(_) = join_set.join_next().await {
            // 收集结果
        }

        result.total_time = start.elapsed();
        result.finalize();

        result
    }
}

/// 运行完整的分块基准测试套件
pub async fn run_chunking_benchmarks() -> String {
    let benchmark = ChunkingBenchmark::new();
    let mut report = String::from("=== Chunking Performance Benchmarks ===\n\n");

    // 测试不同策略
    report.push_str("## Strategy Performance\n");
    let strategy_results = benchmark.benchmark_all_strategies().await;
    for result in strategy_results {
        report.push_str(&format!("{}: {:.2} MB/s\n", 
            result.test_name, result.throughput));
    }

    // 测试不同文档大小
    report.push_str("\n## Document Size Performance\n");
    let size_results = benchmark.benchmark_by_size().await;
    for result in size_results {
        report.push_str(&format!("{}: avg {:?}\n", 
            result.test_name, result.average_time));
    }

    // 测试并发性能
    report.push_str("\n## Concurrency Performance\n");
    let concurrent_result = benchmark.benchmark_concurrency().await;
    report.push_str(&format!("Concurrent processing: {:?} total\n", 
        concurrent_result.total_time));

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chunking_benchmark() {
        let benchmark = ChunkingBenchmark::new();
        let result = benchmark.benchmark_strategy(ChunkingStrategy::Simple).await;
        
        assert!(result.iterations > 0);
        assert!(result.total_time > Duration::from_secs(0));
        assert!(result.throughput > 0.0);
    }
}