/// Rainbow-Docs 智能分块系统使用示例
/// 
/// 这个示例展示了如何使用新的分块系统，包括：
/// - 配置分块策略
/// - 解析文档结构
/// - 生成智能分块
/// - 评估分块质量
/// - 运行测试套件

use std::collections::HashMap;
use rainbow_docs::services::{
    chunking::{ChunkingConfig, ChunkingStrategy, EnhancedDocumentChunk, ContentType},
    structure_parser::{DocumentStructureParser, ParserConfig},
    quality_assessor::ChunkQualityAssessor,
    chunking_tests::ChunkingTestSuite,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Rainbow-Docs 智能分块系统示例 ===\n");
    
    // 示例1: 基本配置和使用
    basic_chunking_example().await?;
    
    // 示例2: 结构化分块
    structural_chunking_example().await?;
    
    // 示例3: 质量评估
    quality_assessment_example().await?;
    
    // 示例4: 运行测试套件
    test_suite_example().await?;
    
    Ok(())
}

/// 基本分块示例
async fn basic_chunking_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. 基本分块示例");
    println!("================");
    
    // 创建分块配置
    let config = ChunkingConfig {
        strategy: ChunkingStrategy::Simple,
        target_chunk_size: 500,
        max_chunk_size: 800,
        min_chunk_size: 200,
        overlap_size: 50,
        ..ChunkingConfig::default()
    };
    
    println!("配置: {:?}", config.strategy);
    println!("目标块大小: {} 字符", config.target_chunk_size);
    
    // 示例文档
    let document = r#"
# 人工智能简介

人工智能（Artificial Intelligence，AI）是计算机科学的一个分支，旨在创建能够执行通常需要人类智能的任务的系统。

## 历史发展

人工智能的概念可以追溯到古代神话，但作为一门学科，它始于20世纪50年代。1956年，约翰·麦卡锡首次提出了"人工智能"这个术语。

## 主要应用领域

### 机器学习
机器学习是人工智能的一个重要分支，通过算法使计算机能够从数据中学习和改进。

### 自然语言处理
自然语言处理（NLP）专注于使计算机能够理解、解释和生成人类语言。

### 计算机视觉
计算机视觉使机器能够解释和理解视觉信息，如图像和视频。
"#;
    
    println!("文档长度: {} 字符", document.len());
    println!("文档预览: {}...\n", &document[0..100.min(document.len())]);
    
    // 简单分块（模拟实现）
    let chunks = create_simple_chunks(document, &config);
    
    println!("生成了 {} 个分块:", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        println!("块 {}: {} 字符", i + 1, chunk.text_length);
        println!("内容: {}...\n", chunk.content.chars().take(100).collect::<String>());
    }
    
    Ok(())
}

/// 结构化分块示例
async fn structural_chunking_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. 结构化分块示例");
    println!("==================");
    
    let markdown_content = r#"
# API 文档

本文档介绍了我们的 REST API 的使用方法。

## 认证

所有API请求都需要认证。使用以下格式的请求头：

```http
Authorization: Bearer YOUR_TOKEN
```

## 端点

### 用户管理

#### 获取用户信息
```http
GET /api/users/{id}
```

**响应示例：**
```json
{
  "id": 123,
  "name": "张三",
  "email": "zhang@example.com"
}
```

#### 创建用户
```http
POST /api/users
```

**请求体：**
| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| name | string | 是 | 用户名 |
| email | string | 是 | 邮箱地址 |

### 文档管理

#### 文档列表
获取用户的文档列表：

- 支持分页
- 支持按标题搜索
- 支持按创建时间排序

```http
GET /api/documents?page=1&limit=10&search=关键词
```
"#;
    
    // 解析文档结构
    let parser = DocumentStructureParser::new_default();
    let structure = parser.parse(markdown_content);
    
    println!("文档结构分析:");
    println!("- 章节数量: {}", structure.outline.len());
    println!("- 元素数量: {}", structure.elements.len());
    println!("- 代码块数量: {}", structure.metadata.code_block_count);
    println!("- 表格数量: {}", structure.metadata.table_count);
    println!("- 预估阅读时间: {:.1} 分钟", structure.metadata.estimated_reading_time);
    
    println!("\n章节大纲:");
    for section in &structure.outline {
        let indent = "  ".repeat((section.level - 1) as usize);
        println!("{}- {} (级别 {}, {} 字符)", indent, section.title, section.level, section.content_length);
    }
    
    println!("\n文档元素:");
    for (i, element) in structure.elements.iter().take(5).enumerate() {
        println!("{}. {:?}: {} 字符", i + 1, element.element_type, element.content.len());
    }
    
    Ok(())
}

/// 质量评估示例
async fn quality_assessment_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3. 质量评估示例");
    println!("================");
    
    // 创建测试块
    let good_chunk = EnhancedDocumentChunk::new(
        "doc1".to_string(),
        "这是一个结构良好的段落。它包含完整的句子和有意义的内容。信息密度适中，适合作为一个高质量的文档块。".to_string(),
        0,
        3,
    );
    
    let poor_chunk = EnhancedDocumentChunk::new(
        "doc1".to_string(),
        "短。".to_string(),
        1,
        3,
    );
    
    let code_chunk = EnhancedDocumentChunk::new(
        "doc1".to_string(),
        "```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```".to_string(),
        2,
        3,
    ).with_structure(
        vec!["API文档".to_string(), "示例代码".to_string()],
        Some(3),
    );
    
    // 创建质量评估器
    let config = ChunkingConfig::default();
    let assessor = ChunkQualityAssessor::from_chunking_config(&config);
    
    let chunks = vec![good_chunk, poor_chunk, code_chunk];
    
    println!("评估 {} 个文档块的质量:\n", chunks.len());
    
    for (i, chunk) in chunks.iter().enumerate() {
        let assessment = assessor.assess_chunk(chunk);
        
        println!("块 {} 评估结果:", i + 1);
        println!("  - 综合得分: {:.2}", assessment.overall_score);
        println!("  - 是否通过: {}", if assessment.passed { "是" } else { "否" });
        println!("  - 语义连贯性: {:.2}", assessment.metrics.semantic_coherence);
        println!("  - 信息密度: {:.2}", assessment.metrics.information_density);
        println!("  - 上下文完整性: {:.2}", assessment.metrics.context_completeness);
        println!("  - 结构完整性: {:.2}", assessment.metrics.structural_integrity);
        
        if !assessment.recommendations.is_empty() {
            println!("  - 改进建议:");
            for rec in &assessment.recommendations {
                println!("    * {}", rec.description);
            }
        }
        println!();
    }
    
    Ok(())
}

/// 测试套件示例
async fn test_suite_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. 测试套件示例");
    println!("================");
    
    // 创建测试套件
    let test_suite = ChunkingTestSuite::create_default_suite();
    
    println!("创建了包含 {} 个测试用例的测试套件", test_suite.test_cases.len());
    
    // 显示测试用例信息
    println!("\n测试用例列表:");
    for (i, test_case) in test_suite.test_cases.iter().enumerate() {
        println!("{}. {} - {}", i + 1, test_case.name, test_case.description);
        println!("   策略: {:?}", test_case.chunking_config.strategy);
        println!("   输入长度: {} 字符", test_case.input_document.len());
        println!("   预期块数: {}-{}", 
                test_case.expected_results.expected_chunk_count_range.0,
                test_case.expected_results.expected_chunk_count_range.1);
        println!();
    }
    
    // 运行测试（注意：这可能需要一些时间）
    println!("运行测试套件...");
    let results = test_suite.run_all_tests().await;
    
    println!("\n测试结果:");
    println!("- 总体通过: {}", if results.passed { "是" } else { "否" });
    println!("- 总体得分: {:.2}", results.overall_score);
    println!("- 执行时间: {} ms", results.execution_time_ms);
    
    if !results.errors.is_empty() {
        println!("- 错误信息:");
        for error in &results.errors {
            println!("  * {}", error);
        }
    }
    
    println!("\n详细结果:");
    for result in &results.detailed_results {
        let status = if result.passed { "✓" } else { "✗" };
        println!("{} {} - 得分: {:.2}, 块数: {}, 质量: {:.2}", 
                status, result.test_case_name, result.score, 
                result.chunk_count, result.average_quality_score);
    }
    
    Ok(())
}

/// 简单分块实现（示例用）
fn create_simple_chunks(content: &str, config: &ChunkingConfig) -> Vec<EnhancedDocumentChunk> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut chunk_index = 0;
    
    while start < content.len() {
        let end = (start + config.target_chunk_size).min(content.len());
        
        // 尝试在合适的位置分割
        let actual_end = if end < content.len() {
            // 寻找最近的句号、换行或空格
            content[start..end]
                .rfind('。')
                .or_else(|| content[start..end].rfind('.'))
                .or_else(|| content[start..end].rfind('\n'))
                .or_else(|| content[start..end].rfind(' '))
                .map(|pos| start + pos + 1)
                .unwrap_or(end)
        } else {
            end
        };
        
        let chunk_content = content[start..actual_end].trim().to_string();
        
        if !chunk_content.is_empty() {
            let mut chunk = EnhancedDocumentChunk::new(
                "example_doc".to_string(),
                chunk_content,
                chunk_index,
                0, // 将在最后更新
            );
            
            // 设置基本的内容类型检测
            if chunk.content.contains("```") {
                chunk.content_type = ContentType::Code { language: None };
            } else if chunk.content.contains('|') && chunk.content.contains("---") {
                chunk.content_type = ContentType::Table;
            } else if chunk.content.trim_start().starts_with(&['-', '*', '+'][..]) {
                chunk.content_type = ContentType::List;
            }
            
            chunks.push(chunk);
            chunk_index += 1;
        }
        
        start = actual_end;
    }
    
    // 更新总数和链接关系
    let total_chunks = chunks.len();
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.total_chunks = total_chunks;
        
        if i > 0 {
            chunk.previous_chunk_id = Some(format!("example_doc_chunk_{:04}", i - 1));
        }
        if i < total_chunks - 1 {
            chunk.next_chunk_id = Some(format!("example_doc_chunk_{:04}", i + 1));
        }
    }
    
    chunks
}

// 注意：这个示例文件需要在项目根目录下创建 examples 目录
// 运行方式: cargo run --example chunking_example