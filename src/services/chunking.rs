use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 分块策略枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkingStrategy {
    /// 简单分块（当前实现）
    Simple,
    /// 基于结构的分块
    Structural,
    /// 基于语义的分块
    Semantic,
    /// 混合策略
    Hybrid,
    /// 自适应分块
    Adaptive,
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        ChunkingStrategy::Simple
    }
}

/// 分块配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// 分块策略
    pub strategy: ChunkingStrategy,
    /// 目标块大小（字符数）
    pub target_chunk_size: usize,
    /// 最大块大小（字符数）
    pub max_chunk_size: usize,
    /// 最小块大小（字符数）
    pub min_chunk_size: usize,
    /// 重叠大小（字符数）
    pub overlap_size: usize,
    /// 结构化分块配置
    pub structural: StructuralConfig,
    /// 语义分块配置
    pub semantic: SemanticConfig,
    /// 质量控制配置
    pub quality: QualityConfig,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            strategy: ChunkingStrategy::default(),
            target_chunk_size: 1000,
            max_chunk_size: 1500,
            min_chunk_size: 100,
            overlap_size: 50,
            structural: StructuralConfig::default(),
            semantic: SemanticConfig::default(),
            quality: QualityConfig::default(),
        }
    }
}

/// 结构化分块配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralConfig {
    /// 是否尊重标题层级
    pub respect_headers: bool,
    /// 是否保持代码块完整
    pub preserve_code_blocks: bool,
    /// 是否保持表格完整
    pub preserve_tables: bool,
    /// 是否保持列表完整
    pub preserve_lists: bool,
    /// 最小章节大小
    pub min_section_size: usize,
    /// 标题权重
    pub header_weights: HashMap<u8, f32>,
}

impl Default for StructuralConfig {
    fn default() -> Self {
        let mut header_weights = HashMap::new();
        header_weights.insert(1, 1.0); // H1
        header_weights.insert(2, 0.8); // H2
        header_weights.insert(3, 0.6); // H3
        header_weights.insert(4, 0.4); // H4
        header_weights.insert(5, 0.3); // H5
        header_weights.insert(6, 0.2); // H6
        
        Self {
            respect_headers: true,
            preserve_code_blocks: true,
            preserve_tables: true,
            preserve_lists: true,
            min_section_size: 200,
            header_weights,
        }
    }
}

/// 语义分块配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConfig {
    /// 是否启用概念提取
    pub use_concept_extraction: bool,
    /// 是否启用依赖分析
    pub use_dependency_analysis: bool,
    /// 是否启用重要性权重
    pub importance_weighting: bool,
    /// 语义相似度阈值
    pub similarity_threshold: f32,
    /// 概念重叠阈值
    pub concept_overlap_threshold: f32,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            use_concept_extraction: false, // 默认关闭，需要额外的 NLP 库
            use_dependency_analysis: false,
            importance_weighting: true,
            similarity_threshold: 0.7,
            concept_overlap_threshold: 0.3,
        }
    }
}

/// 质量控制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfig {
    /// 是否启用质量评估
    pub enabled: bool,
    /// 最小语义连贯性分数
    pub min_coherence_score: f32,
    /// 最小信息密度分数
    pub min_information_density: f32,
    /// 最小上下文完整性分数
    pub min_context_completeness: f32,
    /// 最小结构完整性分数
    pub min_structural_integrity: f32,
    /// 是否自动调整低质量块
    pub auto_adjust_low_quality: bool,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_coherence_score: 0.6,
            min_information_density: 0.5,
            min_context_completeness: 0.7,
            min_structural_integrity: 0.8,
            auto_adjust_low_quality: false,
        }
    }
}

impl ChunkingConfig {
    /// 从环境变量创建配置
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        // 读取基础配置
        if let Ok(strategy_str) = std::env::var("CHUNKING_STRATEGY") {
            config.strategy = match strategy_str.to_lowercase().as_str() {
                "simple" => ChunkingStrategy::Simple,
                "structural" => ChunkingStrategy::Structural,
                "semantic" => ChunkingStrategy::Semantic,
                "hybrid" => ChunkingStrategy::Hybrid,
                "adaptive" => ChunkingStrategy::Adaptive,
                _ => ChunkingStrategy::Simple,
            };
        }
        
        if let Ok(size_str) = std::env::var("CHUNKING_TARGET_SIZE") {
            if let Ok(size) = size_str.parse::<usize>() {
                config.target_chunk_size = size;
            }
        }
        
        if let Ok(size_str) = std::env::var("CHUNKING_MAX_SIZE") {
            if let Ok(size) = size_str.parse::<usize>() {
                config.max_chunk_size = size;
            }
        }
        
        if let Ok(size_str) = std::env::var("CHUNKING_MIN_SIZE") {
            if let Ok(size) = size_str.parse::<usize>() {
                config.min_chunk_size = size;
            }
        }
        
        if let Ok(size_str) = std::env::var("CHUNKING_OVERLAP_SIZE") {
            if let Ok(size) = size_str.parse::<usize>() {
                config.overlap_size = size;
            }
        }
        
        // 读取质量控制配置
        if let Ok(enabled_str) = std::env::var("CHUNK_QUALITY_ENABLED") {
            config.quality.enabled = enabled_str.parse().unwrap_or(true);
        }
        
        if let Ok(score_str) = std::env::var("MIN_QUALITY_SCORE") {
            if let Ok(score) = score_str.parse::<f32>() {
                config.quality.min_coherence_score = score;
                config.quality.min_information_density = score;
                config.quality.min_context_completeness = score;
                config.quality.min_structural_integrity = score;
            }
        }
        
        config
    }
    
    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.min_chunk_size >= self.target_chunk_size {
            return Err("min_chunk_size must be less than target_chunk_size".to_string());
        }
        
        if self.target_chunk_size >= self.max_chunk_size {
            return Err("target_chunk_size must be less than max_chunk_size".to_string());
        }
        
        if self.overlap_size >= self.min_chunk_size {
            return Err("overlap_size must be less than min_chunk_size".to_string());
        }
        
        if self.quality.enabled {
            let scores = [
                self.quality.min_coherence_score,
                self.quality.min_information_density,
                self.quality.min_context_completeness,
                self.quality.min_structural_integrity,
            ];
            
            for score in scores.iter() {
                if *score < 0.0 || *score > 1.0 {
                    return Err("Quality scores must be between 0.0 and 1.0".to_string());
                }
            }
        }
        
        Ok(())
    }
    
    /// 获取当前策略是否需要结构分析
    pub fn needs_structural_analysis(&self) -> bool {
        matches!(
            self.strategy,
            ChunkingStrategy::Structural | ChunkingStrategy::Hybrid | ChunkingStrategy::Adaptive
        )
    }
    
    /// 获取当前策略是否需要语义分析
    pub fn needs_semantic_analysis(&self) -> bool {
        matches!(
            self.strategy,
            ChunkingStrategy::Semantic | ChunkingStrategy::Hybrid | ChunkingStrategy::Adaptive
        )
    }
}

/// 内容类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    /// 普通文本
    Text,
    /// 代码块
    Code { language: Option<String> },
    /// 表格
    Table,
    /// 列表
    List,
    /// 引用
    Quote,
    /// 图片
    Image,
    /// 链接
    Link,
    /// 混合内容
    Mixed,
}

impl Default for ContentType {
    fn default() -> Self {
        ContentType::Text
    }
}

/// 增强的文档块结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedDocumentChunk {
    // 基础信息
    pub id: String,
    pub document_id: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    
    // 内容信息
    pub content: String,
    pub content_type: ContentType,
    pub text_length: usize,
    pub token_count: Option<usize>,
    
    // 结构信息
    pub section_path: Vec<String>,
    pub section_level: Option<u8>,
    pub position_in_section: Option<usize>,
    pub source_range: Option<(usize, usize)>,
    
    // 语义信息（可选，用于高级分块策略）
    pub concepts: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub importance_score: Option<f32>,
    
    // 上下文信息
    pub previous_chunk_id: Option<String>,
    pub next_chunk_id: Option<String>,
    pub parent_section_id: Option<String>,
    
    // 质量评估
    pub quality_score: Option<f32>,
    pub quality_metrics: Option<HashMap<String, f32>>,
    
    // 向量信息
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: Option<String>,
    pub embedding_dimension: Option<usize>,
    
    // 元数据
    pub extraction_method: String,
    pub processing_flags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl EnhancedDocumentChunk {
    /// 创建新的文档块
    pub fn new(
        document_id: String,
        content: String,
        chunk_index: usize,
        total_chunks: usize,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: format!("{}_chunk_{:04}", document_id, chunk_index),
            document_id,
            chunk_index,
            total_chunks,
            content: content.clone(),
            content_type: ContentType::default(),
            text_length: content.len(),
            token_count: None,
            section_path: Vec::new(),
            section_level: None,
            position_in_section: None,
            source_range: None,
            concepts: None,
            keywords: None,
            importance_score: None,
            previous_chunk_id: None,
            next_chunk_id: None,
            parent_section_id: None,
            quality_score: None,
            quality_metrics: None,
            embedding: None,
            embedding_model: None,
            embedding_dimension: None,
            extraction_method: "simple".to_string(),
            processing_flags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// 设置结构信息
    pub fn with_structure(mut self, section_path: Vec<String>, level: Option<u8>) -> Self {
        self.section_path = section_path;
        self.section_level = level;
        self.extraction_method = "structural".to_string();
        self
    }
    
    /// 设置语义信息
    pub fn with_semantics(
        mut self, 
        concepts: Option<Vec<String>>, 
        keywords: Option<Vec<String>>,
        importance: Option<f32>
    ) -> Self {
        self.concepts = concepts;
        self.keywords = keywords;
        self.importance_score = importance;
        if self.extraction_method == "simple" {
            self.extraction_method = "semantic".to_string();
        }
        self
    }
    
    /// 设置质量评估信息
    pub fn with_quality(mut self, score: f32, metrics: Option<HashMap<String, f32>>) -> Self {
        self.quality_score = Some(score);
        self.quality_metrics = metrics;
        self
    }
    
    /// 设置向量信息
    pub fn with_embedding(
        mut self, 
        embedding: Vec<f32>, 
        model: String, 
        dimension: usize
    ) -> Self {
        self.embedding = Some(embedding);
        self.embedding_model = Some(model);
        self.embedding_dimension = Some(dimension);
        self
    }
    
    /// 转换为旧的向量数据格式（兼容现有系统）
    pub fn to_vector_data(&self) -> Option<crate::services::vector::VectorData> {
        if let Some(embedding) = &self.embedding {
            Some(crate::services::vector::VectorData {
                embedding: embedding.clone(),
                model: self.embedding_model.clone().unwrap_or_default(),
                dimension: self.embedding_dimension.unwrap_or(embedding.len()),
                metadata: Some(serde_json::json!({
                    "chunk_index": self.chunk_index,
                    "total_chunks": self.total_chunks,
                    "text_length": self.text_length,
                    "content_type": self.content_type,
                    "section_path": self.section_path,
                    "quality_score": self.quality_score,
                    "extraction_method": self.extraction_method,
                    "created_at": self.created_at.to_rfc3339()
                })),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_config_default() {
        let config = ChunkingConfig::default();
        assert_eq!(config.target_chunk_size, 1000);
        assert_eq!(config.max_chunk_size, 1500);
        assert_eq!(config.min_chunk_size, 100);
        assert!(config.quality.enabled);
    }

    #[test]
    fn test_config_validation() {
        let mut config = ChunkingConfig::default();
        assert!(config.validate().is_ok());
        
        // 测试无效配置
        config.min_chunk_size = 1200; // 大于 target_chunk_size
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_enhanced_document_chunk_creation() {
        let chunk = EnhancedDocumentChunk::new(
            "doc123".to_string(),
            "This is test content.".to_string(),
            0,
            1,
        );
        
        assert_eq!(chunk.document_id, "doc123");
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.text_length, 21);
        assert_eq!(chunk.id, "doc123_chunk_0000");
    }
}