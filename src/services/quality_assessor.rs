use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::services::chunking::{EnhancedDocumentChunk, ContentType, ChunkingConfig};

/// 分块质量评估器
pub struct ChunkQualityAssessor {
    /// 质量阈值配置
    thresholds: QualityThresholds,
    /// 评估指标计算器
    calculator: QualityMetricsCalculator,
}

/// 质量阈值配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityThresholds {
    /// 最小语义连贯性分数
    pub min_coherence: f32,
    /// 最小信息密度分数
    pub min_density: f32,
    /// 最小上下文完整性分数
    pub min_completeness: f32,
    /// 最小结构完整性分数
    pub min_integrity: f32,
    /// 最小重叠适当性分数
    pub min_overlap_appropriateness: f32,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            min_coherence: 0.6,
            min_density: 0.5,
            min_completeness: 0.7,
            min_integrity: 0.8,
            min_overlap_appropriateness: 0.6,
        }
    }
}

/// 质量评估指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// 语义连贯性 (0.0 - 1.0)
    pub semantic_coherence: f32,
    /// 信息密度 (0.0 - 1.0)
    pub information_density: f32,
    /// 上下文完整性 (0.0 - 1.0)
    pub context_completeness: f32,
    /// 结构完整性 (0.0 - 1.0)
    pub structural_integrity: f32,
    /// 重叠适当性 (0.0 - 1.0)
    pub overlap_appropriateness: f32,
    /// 长度适当性 (0.0 - 1.0)
    pub length_appropriateness: f32,
}

/// 质量评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessment {
    /// 质量指标
    pub metrics: QualityMetrics,
    /// 综合质量分数
    pub overall_score: f32,
    /// 是否通过质量检查
    pub passed: bool,
    /// 改进建议
    pub recommendations: Vec<QualityRecommendation>,
    /// 详细分析
    pub detailed_analysis: HashMap<String, String>,
}

/// 质量改进建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRecommendation {
    /// 建议类型
    pub recommendation_type: RecommendationType,
    /// 建议描述
    pub description: String,
    /// 严重程度 (1-5, 5最严重)
    pub severity: u8,
    /// 预期改进效果
    pub expected_improvement: f32,
}

/// 建议类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    /// 调整分块大小
    AdjustSize,
    /// 改进边界检测
    ImproveBoundary,
    /// 增加上下文
    AddContext,
    /// 减少重叠
    ReduceOverlap,
    /// 增加重叠
    IncreaseOverlap,
    /// 保持结构完整性
    PreserveStructure,
    /// 提高信息密度
    IncreaseInformationDensity,
}

/// 质量指标计算器
pub struct QualityMetricsCalculator {
    /// 计算配置
    config: CalculatorConfig,
}

/// 计算器配置
#[derive(Debug, Clone)]
pub struct CalculatorConfig {
    /// 理想块大小范围
    pub ideal_size_range: (usize, usize),
    /// 信息密度权重
    pub density_weights: DensityWeights,
    /// 结构完整性权重
    pub structure_weights: StructureWeights,
}

/// 信息密度权重配置
#[derive(Debug, Clone)]
pub struct DensityWeights {
    /// 关键词密度权重
    pub keyword_density: f32,
    /// 句子完整性权重
    pub sentence_completeness: f32,
    /// 概念覆盖权重
    pub concept_coverage: f32,
}

/// 结构完整性权重配置
#[derive(Debug, Clone)]
pub struct StructureWeights {
    /// 段落完整性权重
    pub paragraph_completeness: f32,
    /// 列表完整性权重
    pub list_completeness: f32,
    /// 代码块完整性权重
    pub code_block_completeness: f32,
    /// 表格完整性权重
    pub table_completeness: f32,
}

impl Default for CalculatorConfig {
    fn default() -> Self {
        Self {
            ideal_size_range: (800, 1200),
            density_weights: DensityWeights {
                keyword_density: 0.3,
                sentence_completeness: 0.4,
                concept_coverage: 0.3,
            },
            structure_weights: StructureWeights {
                paragraph_completeness: 0.4,
                list_completeness: 0.2,
                code_block_completeness: 0.3,
                table_completeness: 0.1,
            },
        }
    }
}

impl ChunkQualityAssessor {
    /// 创建新的质量评估器
    pub fn new(thresholds: QualityThresholds, calculator_config: CalculatorConfig) -> Self {
        Self {
            thresholds,
            calculator: QualityMetricsCalculator::new(calculator_config),
        }
    }
    
    /// 使用默认配置创建评估器
    pub fn new_default() -> Self {
        Self {
            thresholds: QualityThresholds::default(),
            calculator: QualityMetricsCalculator::new_default(),
        }
    }
    
    /// 从分块配置创建评估器
    pub fn from_chunking_config(config: &ChunkingConfig) -> Self {
        let thresholds = QualityThresholds {
            min_coherence: config.quality.min_coherence_score,
            min_density: config.quality.min_information_density,
            min_completeness: config.quality.min_context_completeness,
            min_integrity: config.quality.min_structural_integrity,
            min_overlap_appropriateness: 0.6,
        };
        
        let calculator_config = CalculatorConfig {
            ideal_size_range: (config.target_chunk_size - 200, config.target_chunk_size + 200),
            ..CalculatorConfig::default()
        };
        
        Self::new(thresholds, calculator_config)
    }
    
    /// 评估单个文档块的质量
    pub fn assess_chunk(&self, chunk: &EnhancedDocumentChunk) -> QualityAssessment {
        let metrics = self.calculator.calculate_metrics(chunk);
        let overall_score = self.calculate_overall_score(&metrics);
        let passed = self.meets_thresholds(&metrics);
        let recommendations = self.generate_recommendations(chunk, &metrics);
        let detailed_analysis = self.generate_detailed_analysis(chunk, &metrics);
        
        QualityAssessment {
            metrics,
            overall_score,
            passed,
            recommendations,
            detailed_analysis,
        }
    }
    
    /// 批量评估多个文档块
    pub fn assess_chunks(&self, chunks: &[EnhancedDocumentChunk]) -> Vec<QualityAssessment> {
        chunks.iter().map(|chunk| self.assess_chunk(chunk)).collect()
    }
    
    /// 评估文档块组合的质量（考虑块之间的关系）
    pub fn assess_chunk_sequence(&self, chunks: &[EnhancedDocumentChunk]) -> QualityAssessment {
        // 计算序列级别的指标
        let sequence_metrics = self.calculator.calculate_sequence_metrics(chunks);
        let overall_score = self.calculate_overall_score(&sequence_metrics);
        let passed = self.meets_thresholds(&sequence_metrics);
        let recommendations = self.generate_sequence_recommendations(chunks, &sequence_metrics);
        let detailed_analysis = self.generate_sequence_analysis(chunks, &sequence_metrics);
        
        QualityAssessment {
            metrics: sequence_metrics,
            overall_score,
            passed,
            recommendations,
            detailed_analysis,
        }
    }
    
    /// 计算综合质量分数
    fn calculate_overall_score(&self, metrics: &QualityMetrics) -> f32 {
        // 使用加权平均计算综合分数
        let weights = [0.25, 0.20, 0.25, 0.15, 0.10, 0.05]; // 各指标权重
        let scores = [
            metrics.semantic_coherence,
            metrics.information_density,
            metrics.context_completeness,
            metrics.structural_integrity,
            metrics.overlap_appropriateness,
            metrics.length_appropriateness,
        ];
        
        weights.iter()
            .zip(scores.iter())
            .map(|(weight, score)| weight * score)
            .sum()
    }
    
    /// 检查是否满足质量阈值
    fn meets_thresholds(&self, metrics: &QualityMetrics) -> bool {
        metrics.semantic_coherence >= self.thresholds.min_coherence &&
        metrics.information_density >= self.thresholds.min_density &&
        metrics.context_completeness >= self.thresholds.min_completeness &&
        metrics.structural_integrity >= self.thresholds.min_integrity &&
        metrics.overlap_appropriateness >= self.thresholds.min_overlap_appropriateness
    }
    
    /// 生成改进建议
    fn generate_recommendations(
        &self, 
        chunk: &EnhancedDocumentChunk, 
        metrics: &QualityMetrics
    ) -> Vec<QualityRecommendation> {
        let mut recommendations = Vec::new();
        
        // 检查各项指标并生成相应建议
        if metrics.semantic_coherence < self.thresholds.min_coherence {
            recommendations.push(QualityRecommendation {
                recommendation_type: RecommendationType::ImproveBoundary,
                description: "分块边界可能割裂了语义单元，建议调整分块策略".to_string(),
                severity: 4,
                expected_improvement: 0.2,
            });
        }
        
        if metrics.information_density < self.thresholds.min_density {
            recommendations.push(QualityRecommendation {
                recommendation_type: RecommendationType::IncreaseInformationDensity,
                description: "块中信息密度偏低，可能包含过多空白或重复内容".to_string(),
                severity: 3,
                expected_improvement: 0.15,
            });
        }
        
        if metrics.context_completeness < self.thresholds.min_completeness {
            recommendations.push(QualityRecommendation {
                recommendation_type: RecommendationType::AddContext,
                description: "缺乏足够的上下文信息，建议增加重叠或调整分块大小".to_string(),
                severity: 4,
                expected_improvement: 0.25,
            });
        }
        
        if metrics.structural_integrity < self.thresholds.min_integrity {
            recommendations.push(QualityRecommendation {
                recommendation_type: RecommendationType::PreserveStructure,
                description: "结构完整性不足，可能割裂了重要的结构元素".to_string(),
                severity: 5,
                expected_improvement: 0.3,
            });
        }
        
        // 基于块大小的建议
        if chunk.text_length < 400 {
            recommendations.push(QualityRecommendation {
                recommendation_type: RecommendationType::AdjustSize,
                description: "块大小偏小，可能导致上下文不足".to_string(),
                severity: 3,
                expected_improvement: 0.1,
            });
        } else if chunk.text_length > 2000 {
            recommendations.push(QualityRecommendation {
                recommendation_type: RecommendationType::AdjustSize,
                description: "块大小偏大，可能影响检索精度".to_string(),
                severity: 2,
                expected_improvement: 0.1,
            });
        }
        
        recommendations
    }
    
    /// 生成详细分析
    fn generate_detailed_analysis(
        &self, 
        chunk: &EnhancedDocumentChunk, 
        metrics: &QualityMetrics
    ) -> HashMap<String, String> {
        let mut analysis = HashMap::new();
        
        analysis.insert(
            "size_analysis".to_string(),
            format!(
                "块大小: {}字符, 理想范围: {}-{}字符",
                chunk.text_length,
                self.calculator.config.ideal_size_range.0,
                self.calculator.config.ideal_size_range.1
            ),
        );
        
        analysis.insert(
            "content_type_analysis".to_string(),
            format!("内容类型: {:?}", chunk.content_type),
        );
        
        if let Some(section_level) = chunk.section_level {
            analysis.insert(
                "structure_analysis".to_string(),
                format!("章节级别: {}, 路径: {:?}", section_level, chunk.section_path),
            );
        }
        
        analysis.insert(
            "metrics_summary".to_string(),
            format!(
                "语义连贯性: {:.2}, 信息密度: {:.2}, 上下文完整性: {:.2}, 结构完整性: {:.2}",
                metrics.semantic_coherence,
                metrics.information_density,
                metrics.context_completeness,
                metrics.structural_integrity
            ),
        );
        
        analysis
    }
    
    /// 生成序列级别的改进建议
    fn generate_sequence_recommendations(
        &self, 
        _chunks: &[EnhancedDocumentChunk], 
        metrics: &QualityMetrics
    ) -> Vec<QualityRecommendation> {
        let mut recommendations = Vec::new();
        
        if metrics.overlap_appropriateness < self.thresholds.min_overlap_appropriateness {
            recommendations.push(QualityRecommendation {
                recommendation_type: RecommendationType::AdjustSize,
                description: "块之间的重叠不够恰当，需要调整重叠策略".to_string(),
                severity: 3,
                expected_improvement: 0.2,
            });
        }
        
        recommendations
    }
    
    /// 生成序列级别的详细分析
    fn generate_sequence_analysis(
        &self, 
        chunks: &[EnhancedDocumentChunk], 
        _metrics: &QualityMetrics
    ) -> HashMap<String, String> {
        let mut analysis = HashMap::new();
        
        analysis.insert(
            "sequence_summary".to_string(),
            format!("分析了{}个文档块的序列质量", chunks.len()),
        );
        
        let total_length: usize = chunks.iter().map(|c| c.text_length).sum();
        analysis.insert(
            "total_size".to_string(),
            format!("总长度: {}字符", total_length),
        );
        
        analysis
    }
}

impl QualityMetricsCalculator {
    /// 创建新的指标计算器
    pub fn new(config: CalculatorConfig) -> Self {
        Self { config }
    }
    
    /// 使用默认配置创建计算器
    pub fn new_default() -> Self {
        Self {
            config: CalculatorConfig::default(),
        }
    }
    
    /// 计算单个块的质量指标
    pub fn calculate_metrics(&self, chunk: &EnhancedDocumentChunk) -> QualityMetrics {
        QualityMetrics {
            semantic_coherence: self.calculate_semantic_coherence(chunk),
            information_density: self.calculate_information_density(chunk),
            context_completeness: self.calculate_context_completeness(chunk),
            structural_integrity: self.calculate_structural_integrity(chunk),
            overlap_appropriateness: 0.8, // 单个块无重叠，设置默认值
            length_appropriateness: self.calculate_length_appropriateness(chunk),
        }
    }
    
    /// 计算块序列的质量指标
    pub fn calculate_sequence_metrics(&self, chunks: &[EnhancedDocumentChunk]) -> QualityMetrics {
        if chunks.is_empty() {
            return QualityMetrics {
                semantic_coherence: 0.0,
                information_density: 0.0,
                context_completeness: 0.0,
                structural_integrity: 0.0,
                overlap_appropriateness: 0.0,
                length_appropriateness: 0.0,
            };
        }
        
        // 计算平均值
        let individual_metrics: Vec<_> = chunks.iter()
            .map(|chunk| self.calculate_metrics(chunk))
            .collect();
        
        let count = individual_metrics.len() as f32;
        
        QualityMetrics {
            semantic_coherence: individual_metrics.iter().map(|m| m.semantic_coherence).sum::<f32>() / count,
            information_density: individual_metrics.iter().map(|m| m.information_density).sum::<f32>() / count,
            context_completeness: individual_metrics.iter().map(|m| m.context_completeness).sum::<f32>() / count,
            structural_integrity: individual_metrics.iter().map(|m| m.structural_integrity).sum::<f32>() / count,
            overlap_appropriateness: self.calculate_overlap_appropriateness(chunks),
            length_appropriateness: individual_metrics.iter().map(|m| m.length_appropriateness).sum::<f32>() / count,
        }
    }
    
    /// 计算语义连贯性
    fn calculate_semantic_coherence(&self, chunk: &EnhancedDocumentChunk) -> f32 {
        let mut score: f32 = 0.5; // 基础分数
        
        // 检查句子完整性
        let content = &chunk.content;
        let sentences: Vec<&str> = content.split(&['.', '!', '?', '。', '！', '？'][..]).collect();
        
        if !sentences.is_empty() {
            let complete_sentences = sentences.iter()
                .filter(|s| s.trim().len() > 10) // 认为长度大于10的为完整句子
                .count();
            
            let sentence_completeness = complete_sentences as f32 / sentences.len() as f32;
            score += sentence_completeness * 0.3;
        }
        
        // 检查是否在段落或结构边界处分割
        if chunk.content.ends_with(&['.', '\n', '。'][..]) {
            score += 0.2;
        }
        
        score.min(1.0)
    }
    
    /// 计算信息密度
    fn calculate_information_density(&self, chunk: &EnhancedDocumentChunk) -> f32 {
        let content = &chunk.content;
        let content_length = content.len() as f32;
        
        if content_length == 0.0 {
            return 0.0;
        }
        
        // 计算非空白字符比例
        let non_whitespace = content.chars().filter(|c| !c.is_whitespace()).count() as f32;
        let whitespace_ratio = 1.0 - (content_length - non_whitespace) / content_length;
        
        // 计算句子密度
        let sentences = content.split(&['.', '!', '?', '。', '！', '？'][..]).count();
        let sentence_density = (sentences as f32 / content_length * 1000.0).min(1.0);
        
        // 检查重复内容
        let words: Vec<&str> = content.split_whitespace().collect();
        let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
        let uniqueness_ratio = if words.is_empty() { 
            0.0 
        } else { 
            unique_words.len() as f32 / words.len() as f32 
        };
        
        // 综合计算
        (whitespace_ratio * 0.3 + sentence_density * 0.4 + uniqueness_ratio * 0.3).min(1.0)
    }
    
    /// 计算上下文完整性
    fn calculate_context_completeness(&self, chunk: &EnhancedDocumentChunk) -> f32 {
        let mut score: f32 = 0.5; // 基础分数
        
        // 检查是否有章节信息
        if !chunk.section_path.is_empty() {
            score += 0.2;
        }
        
        // 检查是否有前后块链接
        if chunk.previous_chunk_id.is_some() || chunk.next_chunk_id.is_some() {
            score += 0.2;
        }
        
        // 检查内容完整性（是否以完整句子结束）
        let content = chunk.content.trim();
        if content.ends_with(&['.', '!', '?', '。', '！', '？'][..]) {
            score += 0.1;
        }
        
        score.min(1.0)
    }
    
    /// 计算结构完整性
    fn calculate_structural_integrity(&self, chunk: &EnhancedDocumentChunk) -> f32 {
        let mut score: f32 = 0.8; // 默认高分数
        
        match &chunk.content_type {
            ContentType::Code { .. } => {
                // 代码块应该保持完整
                if chunk.content.contains("```") && !chunk.content.starts_with("```") {
                    score -= 0.5; // 代码块被截断
                }
            }
            ContentType::Table => {
                // 表格应该保持完整
                if !chunk.content.contains('|') {
                    score -= 0.4; // 表格结构损坏
                }
            }
            ContentType::List => {
                // 列表应该保持完整
                let list_markers = chunk.content.matches(&['-', '*', '+'][..]).count();
                let lines = chunk.content.lines().count();
                if lines > 0 && (list_markers as f32 / lines as f32) < 0.3 {
                    score -= 0.3; // 列表结构不完整
                }
            }
            _ => {}
        }
        
        score.max(0.0)
    }
    
    /// 计算长度适当性
    fn calculate_length_appropriateness(&self, chunk: &EnhancedDocumentChunk) -> f32 {
        let length = chunk.text_length;
        let (min_ideal, max_ideal) = self.config.ideal_size_range;
        
        if length >= min_ideal && length <= max_ideal {
            1.0
        } else if length < min_ideal {
            // 过短
            (length as f32 / min_ideal as f32).max(0.1)
        } else {
            // 过长
            (max_ideal as f32 / length as f32).max(0.1)
        }
    }
    
    /// 计算重叠适当性
    fn calculate_overlap_appropriateness(&self, chunks: &[EnhancedDocumentChunk]) -> f32 {
        if chunks.len() < 2 {
            return 1.0;
        }
        
        let mut total_overlap_score = 0.0;
        let mut comparisons = 0;
        
        for i in 0..chunks.len() - 1 {
            let current = &chunks[i];
            let next = &chunks[i + 1];
            
            // 简单的重叠检测（基于内容相似性）
            let overlap_score = self.calculate_content_overlap(current, next);
            total_overlap_score += overlap_score;
            comparisons += 1;
        }
        
        if comparisons > 0 {
            total_overlap_score / comparisons as f32
        } else {
            1.0
        }
    }
    
    /// 计算两个块之间的内容重叠
    fn calculate_content_overlap(&self, chunk1: &EnhancedDocumentChunk, chunk2: &EnhancedDocumentChunk) -> f32 {
        // 简化的重叠计算：检查最后和开始的部分是否相似
        let chunk1_end = if chunk1.content.len() > 100 {
            &chunk1.content[chunk1.content.len() - 100..]
        } else {
            &chunk1.content
        };
        
        let chunk2_start = if chunk2.content.len() > 100 {
            &chunk2.content[..100]
        } else {
            &chunk2.content
        };
        
        // 简单的词汇重叠检测
        let words1: std::collections::HashSet<&str> = chunk1_end.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = chunk2_start.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union > 0 {
            intersection as f32 / union as f32
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::chunking::ContentType;

    #[test]
    fn test_quality_assessment_basic() {
        let assessor = ChunkQualityAssessor::new_default();
        let chunk = EnhancedDocumentChunk::new(
            "doc1".to_string(),
            "This is a well-structured paragraph. It contains complete sentences and meaningful content. The information density is appropriate for a quality chunk.".to_string(),
            0,
            1,
        );
        
        let assessment = assessor.assess_chunk(&chunk);
        assert!(assessment.overall_score > 0.5);
        assert!(assessment.metrics.information_density > 0.0);
    }

    #[test]
    fn test_quality_thresholds() {
        let thresholds = QualityThresholds {
            min_coherence: 0.8,
            min_density: 0.8,
            min_completeness: 0.8,
            min_integrity: 0.8,
            min_overlap_appropriateness: 0.8,
        };
        
        let assessor = ChunkQualityAssessor::new(thresholds, CalculatorConfig::default());
        let poor_chunk = EnhancedDocumentChunk::new(
            "doc1".to_string(),
            "Short.".to_string(),
            0,
            1,
        );
        
        let assessment = assessor.assess_chunk(&poor_chunk);
        assert!(!assessment.passed);
        assert!(!assessment.recommendations.is_empty());
    }

    #[test]
    fn test_structural_integrity_code() {
        let calculator = QualityMetricsCalculator::new_default();
        let mut code_chunk = EnhancedDocumentChunk::new(
            "doc1".to_string(),
            "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```".to_string(),
            0,
            1,
        );
        code_chunk.content_type = ContentType::Code { language: Some("rust".to_string()) };
        
        let metrics = calculator.calculate_metrics(&code_chunk);
        assert!(metrics.structural_integrity > 0.5);
        
        // 测试被截断的代码块
        let mut broken_code_chunk = EnhancedDocumentChunk::new(
            "doc1".to_string(),
            "fn main() {\n    println!(\"Hello\");\n}\n```".to_string(), // 缺少开始的```
            0,
            1,
        );
        broken_code_chunk.content_type = ContentType::Code { language: Some("rust".to_string()) };
        
        let broken_metrics = calculator.calculate_metrics(&broken_code_chunk);
        assert!(broken_metrics.structural_integrity < metrics.structural_integrity);
    }
}