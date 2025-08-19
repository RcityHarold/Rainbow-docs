use std::sync::Arc;
use std::collections::HashMap;
use crate::services::{
    chunking::{ChunkingConfig, ChunkingStrategy, EnhancedDocumentChunk, ContentType},
    structure_parser::{DocumentStructureParser, DocumentStructure, Section, DocumentElement, ElementType},
    quality_assessor::{ChunkQualityAssessor, QualityAssessment},
    embedding::EmbeddingService,
};
use crate::error::{AppError, Result};

/// 智能分块器
pub struct IntelligentChunker {
    /// 分块配置
    config: ChunkingConfig,
    /// 结构解析器
    structure_parser: Arc<DocumentStructureParser>,
    /// 质量评估器
    quality_assessor: Arc<ChunkQualityAssessor>,
}

/// 分块结果
#[derive(Debug, Clone)]
pub struct ChunkingResult {
    /// 生成的文档块
    pub chunks: Vec<EnhancedDocumentChunk>,
    /// 文档结构信息
    pub structure: DocumentStructure,
    /// 质量评估结果
    pub quality_assessments: Vec<QualityAssessment>,
    /// 分块统计信息
    pub statistics: ChunkingStatistics,
}

/// 分块统计信息
#[derive(Debug, Clone)]
pub struct ChunkingStatistics {
    /// 总块数
    pub total_chunks: usize,
    /// 平均块大小
    pub average_chunk_size: f32,
    /// 最小块大小
    pub min_chunk_size: usize,
    /// 最大块大小
    pub max_chunk_size: usize,
    /// 平均质量分数
    pub average_quality_score: f32,
    /// 处理时间（毫秒）
    pub processing_time_ms: u64,
    /// 使用的策略
    pub strategy_used: ChunkingStrategy,
}

impl IntelligentChunker {
    /// 创建新的智能分块器
    pub fn new(config: ChunkingConfig) -> Result<Self> {
        // 验证配置
        config.validate()
            .map_err(|e| AppError::bad_request(format!("Invalid chunking config: {}", e)))?;
        
        let structure_parser = Arc::new(DocumentStructureParser::new_default());
        let quality_assessor = Arc::new(ChunkQualityAssessor::from_chunking_config(&config));
        
        Ok(Self {
            config,
            structure_parser,
            quality_assessor,
        })
    }
    
    /// 对文档进行智能分块
    pub async fn chunk_document(
        &self,
        document_id: &str,
        title: &str,
        content: &str,
    ) -> Result<ChunkingResult> {
        let start_time = std::time::Instant::now();
        
        // 1. 解析文档结构
        let structure = self.structure_parser.parse(content);
        
        // 2. 根据策略进行分块
        let chunks = match self.config.strategy {
            ChunkingStrategy::Simple => {
                self.simple_chunking(document_id, title, content)?
            }
            ChunkingStrategy::Structural => {
                self.structural_chunking(document_id, title, content, &structure)?
            }
            ChunkingStrategy::Semantic => {
                self.semantic_chunking(document_id, title, content, &structure)?
            }
            ChunkingStrategy::Hybrid => {
                self.hybrid_chunking(document_id, title, content, &structure)?
            }
            ChunkingStrategy::Adaptive => {
                self.adaptive_chunking(document_id, title, content, &structure)?
            }
        };
        
        // 3. 质量评估
        let quality_assessments = self.quality_assessor.assess_chunks(&chunks);
        
        // 4. 计算统计信息
        let statistics = self.calculate_statistics(&chunks, &quality_assessments, start_time.elapsed());
        
        Ok(ChunkingResult {
            chunks,
            structure,
            quality_assessments,
            statistics,
        })
    }
    
    /// 简单分块（保持与现有实现兼容）
    fn simple_chunking(
        &self,
        document_id: &str,
        title: &str,
        content: &str,
    ) -> Result<Vec<EnhancedDocumentChunk>> {
        // 合并标题和内容
        let full_text = format!("{}\n\n{}", title, content);
        
        // 使用现有的简单分块逻辑
        let chunks = self.chunk_text_simple(&full_text, document_id);
        
        Ok(chunks)
    }
    
    /// 结构化分块
    fn structural_chunking(
        &self,
        document_id: &str,
        title: &str,
        content: &str,
        structure: &DocumentStructure,
    ) -> Result<Vec<EnhancedDocumentChunk>> {
        let mut chunks = Vec::new();
        let full_text = format!("{}\n\n{}", title, content);
        
        // 如果没有章节结构，退回到简单分块
        if structure.outline.is_empty() {
            return self.simple_chunking(document_id, title, content);
        }
        
        // 基于章节进行分块
        for section in &structure.outline {
            let section_chunks = self.chunk_section(
                document_id,
                &full_text,
                section,
                &structure.elements,
                &chunks.len(),
            )?;
            chunks.extend(section_chunks);
        }
        
        // 处理不属于任何章节的内容
        let orphan_chunks = self.chunk_orphan_content(
            document_id,
            &full_text,
            &structure,
            &chunks.len(),
        )?;
        chunks.extend(orphan_chunks);
        
        // 更新块的总数和链接关系
        self.finalize_chunks(&mut chunks);
        
        Ok(chunks)
    }
    
    /// 语义分块
    fn semantic_chunking(
        &self,
        document_id: &str,
        title: &str,
        content: &str,
        structure: &DocumentStructure,
    ) -> Result<Vec<EnhancedDocumentChunk>> {
        // 当前实现：基于段落和语义边界
        let mut chunks = Vec::new();
        let full_text = format!("{}\n\n{}", title, content);
        
        // 识别语义单元（段落、代码块、表格等）
        let semantic_units = self.identify_semantic_units(&full_text, structure);
        
        // 将语义单元组合成块
        let mut current_chunk_content = String::new();
        let mut current_chunk_start = 0;
        let mut chunk_index = 0;
        
        for unit in semantic_units {
            // 检查是否需要开始新块
            if self.should_start_new_chunk(&current_chunk_content, &unit.content) {
                if !current_chunk_content.is_empty() {
                    let chunk = self.create_semantic_chunk(
                        document_id,
                        current_chunk_content.clone(),
                        chunk_index,
                        current_chunk_start,
                    );
                    chunks.push(chunk);
                    chunk_index += 1;
                    
                    // 处理重叠
                    if self.config.overlap_size > 0 {
                        current_chunk_content = self.extract_overlap(&current_chunk_content);
                    } else {
                        current_chunk_content.clear();
                    }
                }
                current_chunk_start = unit.start_position;
            }
            
            // 添加单元到当前块
            if !current_chunk_content.is_empty() {
                current_chunk_content.push_str("\n\n");
            }
            current_chunk_content.push_str(&unit.content);
        }
        
        // 处理最后一个块
        if !current_chunk_content.is_empty() {
            let chunk = self.create_semantic_chunk(
                document_id,
                current_chunk_content,
                chunk_index,
                current_chunk_start,
            );
            chunks.push(chunk);
        }
        
        self.finalize_chunks(&mut chunks);
        Ok(chunks)
    }
    
    /// 混合分块策略
    fn hybrid_chunking(
        &self,
        document_id: &str,
        title: &str,
        content: &str,
        structure: &DocumentStructure,
    ) -> Result<Vec<EnhancedDocumentChunk>> {
        // 先进行结构化分块
        let structural_chunks = self.structural_chunking(document_id, title, content, structure)?;
        
        // 对过大的块进行语义分割
        let mut refined_chunks = Vec::new();
        
        for chunk in structural_chunks {
            if chunk.text_length > self.config.max_chunk_size {
                // 对大块进行语义分割
                let sub_chunks = self.split_large_chunk(document_id, &chunk)?;
                refined_chunks.extend(sub_chunks);
            } else if chunk.text_length < self.config.min_chunk_size && !refined_chunks.is_empty() {
                // 合并过小的块
                if let Some(last_chunk) = refined_chunks.last_mut() {
                    if last_chunk.text_length + chunk.text_length <= self.config.target_chunk_size {
                        last_chunk.content.push_str("\n\n");
                        last_chunk.content.push_str(&chunk.content);
                        last_chunk.text_length = last_chunk.content.len();
                        continue;
                    }
                }
                refined_chunks.push(chunk);
            } else {
                refined_chunks.push(chunk);
            }
        }
        
        self.finalize_chunks(&mut refined_chunks);
        Ok(refined_chunks)
    }
    
    /// 自适应分块
    fn adaptive_chunking(
        &self,
        document_id: &str,
        title: &str,
        content: &str,
        structure: &DocumentStructure,
    ) -> Result<Vec<EnhancedDocumentChunk>> {
        // 分析文档特征
        let doc_features = self.analyze_document_features(content, structure);
        
        // 根据文档特征选择最佳策略
        let best_strategy = self.select_best_strategy(&doc_features);
        
        // 使用选定的策略进行分块
        match best_strategy {
            ChunkingStrategy::Simple => self.simple_chunking(document_id, title, content),
            ChunkingStrategy::Structural => self.structural_chunking(document_id, title, content, structure),
            ChunkingStrategy::Semantic => self.semantic_chunking(document_id, title, content, structure),
            ChunkingStrategy::Hybrid => self.hybrid_chunking(document_id, title, content, structure),
            _ => self.simple_chunking(document_id, title, content), // 默认使用简单分块
        }
    }
    
    /// 简单文本分块（兼容现有实现）
    fn chunk_text_simple(&self, text: &str, document_id: &str) -> Vec<EnhancedDocumentChunk> {
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;
        
        while start < text.len() {
            let end = (start + self.config.target_chunk_size).min(text.len());
            
            // 尝试在句子边界分割
            let chunk_end = if end < text.len() {
                text[start..end]
                    .rfind(&['.', '。', '!', '！', '?', '？'][..])
                    .or_else(|| text[start..end].rfind('\n'))
                    .or_else(|| text[start..end].rfind(' '))
                    .map(|pos| start + pos + 1)
                    .unwrap_or(end)
            } else {
                end
            };
            
            if chunk_end > start {
                let chunk_content = text[start..chunk_end].trim().to_string();
                if !chunk_content.is_empty() {
                    let chunk = EnhancedDocumentChunk::new(
                        document_id.to_string(),
                        chunk_content,
                        chunk_index,
                        0, // 将在finalize中更新
                    );
                    chunks.push(chunk);
                    chunk_index += 1;
                }
            }
            
            start = chunk_end;
        }
        
        chunks
    }
    
    /// 分块章节内容
    fn chunk_section(
        &self,
        document_id: &str,
        full_text: &str,
        section: &Section,
        elements: &[DocumentElement],
        base_index: &usize,
    ) -> Result<Vec<EnhancedDocumentChunk>> {
        let mut chunks = Vec::new();
        let section_content = &full_text[section.start_position..section.end_position];
        
        // 如果章节内容小于最大块大小，作为一个块
        if section_content.len() <= self.config.max_chunk_size {
            let mut chunk = EnhancedDocumentChunk::new(
                document_id.to_string(),
                section_content.to_string(),
                base_index + chunks.len(),
                0,
            );
            
            // 设置结构信息
            chunk = chunk.with_structure(
                section.path.clone(),
                Some(section.level),
            );
            
            // 设置内容类型
            chunk.content_type = self.detect_content_type(section_content, &section.element_indices, elements);
            
            chunks.push(chunk);
        } else {
            // 章节过大，需要进一步分割
            let section_chunks = self.split_large_section(
                document_id,
                section_content,
                section,
                elements,
                base_index,
            )?;
            chunks.extend(section_chunks);
        }
        
        Ok(chunks)
    }
    
    /// 分割大章节
    fn split_large_section(
        &self,
        document_id: &str,
        content: &str,
        section: &Section,
        elements: &[DocumentElement],
        base_index: &usize,
    ) -> Result<Vec<EnhancedDocumentChunk>> {
        let mut chunks = Vec::new();
        
        // 收集该章节的元素
        let section_elements: Vec<&DocumentElement> = section.element_indices
            .iter()
            .filter_map(|&idx| elements.get(idx))
            .collect();
        
        // 基于元素边界进行分割
        let mut current_chunk = String::new();
        let mut chunk_start = 0;
        
        for element in section_elements {
            let element_content = &element.content;
            
            // 检查是否需要开始新块
            if current_chunk.len() + element_content.len() > self.config.target_chunk_size 
               && !current_chunk.is_empty() {
                // 创建块
                let mut chunk = EnhancedDocumentChunk::new(
                    document_id.to_string(),
                    current_chunk.clone(),
                    base_index + chunks.len(),
                    0,
                );
                
                chunk = chunk.with_structure(section.path.clone(), Some(section.level));
                chunks.push(chunk);
                
                // 处理重叠
                current_chunk = if self.config.overlap_size > 0 {
                    self.extract_overlap(&current_chunk)
                } else {
                    String::new()
                };
            }
            
            // 添加元素到当前块
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(element_content);
        }
        
        // 处理剩余内容
        if !current_chunk.is_empty() {
            let mut chunk = EnhancedDocumentChunk::new(
                document_id.to_string(),
                current_chunk,
                base_index + chunks.len(),
                0,
            );
            chunk = chunk.with_structure(section.path.clone(), Some(section.level));
            chunks.push(chunk);
        }
        
        Ok(chunks)
    }
    
    /// 处理不属于任何章节的内容
    fn chunk_orphan_content(
        &self,
        document_id: &str,
        full_text: &str,
        structure: &DocumentStructure,
        base_index: &usize,
    ) -> Result<Vec<EnhancedDocumentChunk>> {
        let mut chunks = Vec::new();
        let mut covered_ranges = Vec::new();
        
        // 收集所有章节覆盖的范围
        for section in &structure.outline {
            covered_ranges.push((section.start_position, section.end_position));
        }
        covered_ranges.sort_by_key(|r| r.0);
        
        // 找到未覆盖的范围
        let mut uncovered_start = 0;
        for (start, end) in covered_ranges {
            if uncovered_start < start {
                let orphan_content = &full_text[uncovered_start..start];
                if !orphan_content.trim().is_empty() {
                    let orphan_chunks = self.chunk_text_simple(orphan_content, document_id);
                    for (i, mut chunk) in orphan_chunks.into_iter().enumerate() {
                        chunk.chunk_index = base_index + chunks.len();
                        chunks.push(chunk);
                    }
                }
            }
            uncovered_start = end.max(uncovered_start);
        }
        
        // 处理最后的未覆盖内容
        if uncovered_start < full_text.len() {
            let orphan_content = &full_text[uncovered_start..];
            if !orphan_content.trim().is_empty() {
                let orphan_chunks = self.chunk_text_simple(orphan_content, document_id);
                for (i, mut chunk) in orphan_chunks.into_iter().enumerate() {
                    chunk.chunk_index = base_index + chunks.len();
                    chunks.push(chunk);
                }
            }
        }
        
        Ok(chunks)
    }
    
    /// 识别语义单元
    fn identify_semantic_units(&self, text: &str, structure: &DocumentStructure) -> Vec<SemanticUnit> {
        let mut units = Vec::new();
        
        // 基于文档元素创建语义单元
        for element in &structure.elements {
            units.push(SemanticUnit {
                content: element.content.clone(),
                unit_type: match &element.element_type {
                    ElementType::CodeBlock { .. } => SemanticUnitType::Code,
                    ElementType::Table => SemanticUnitType::Table,
                    ElementType::List { .. } => SemanticUnitType::List,
                    ElementType::BlockQuote => SemanticUnitType::Quote,
                    _ => SemanticUnitType::Paragraph,
                },
                start_position: element.position.0,
                end_position: element.position.1,
                importance: 1.0, // 可以基于更复杂的逻辑计算
            });
        }
        
        // 如果没有识别到元素，按段落分割
        if units.is_empty() {
            let paragraphs: Vec<&str> = text.split("\n\n").collect();
            let mut position = 0;
            
            for paragraph in paragraphs {
                if !paragraph.trim().is_empty() {
                    units.push(SemanticUnit {
                        content: paragraph.to_string(),
                        unit_type: SemanticUnitType::Paragraph,
                        start_position: position,
                        end_position: position + paragraph.len(),
                        importance: 1.0,
                    });
                }
                position += paragraph.len() + 2; // +2 for \n\n
            }
        }
        
        units
    }
    
    /// 检查是否应该开始新块
    fn should_start_new_chunk(&self, current_content: &str, new_content: &str) -> bool {
        // 如果当前块为空，不需要新块
        if current_content.is_empty() {
            return false;
        }
        
        // 如果添加新内容会超过目标大小，需要新块
        if current_content.len() + new_content.len() + 2 > self.config.target_chunk_size {
            return true;
        }
        
        // 如果当前块已经接近最大大小，需要新块
        if current_content.len() > self.config.max_chunk_size - 200 {
            return true;
        }
        
        false
    }
    
    /// 创建语义块
    fn create_semantic_chunk(
        &self,
        document_id: &str,
        content: String,
        index: usize,
        start_position: usize,
    ) -> EnhancedDocumentChunk {
        let mut chunk = EnhancedDocumentChunk::new(
            document_id.to_string(),
            content,
            index,
            0,
        );
        
        // 设置源范围
        chunk.source_range = Some((start_position, start_position + chunk.text_length));
        
        // 检测内容类型
        chunk.content_type = self.detect_content_type_from_text(&chunk.content);
        
        chunk
    }
    
    /// 提取重叠内容
    fn extract_overlap(&self, content: &str) -> String {
        if content.len() <= self.config.overlap_size {
            return content.to_string();
        }
        
        let start = content.len().saturating_sub(self.config.overlap_size);
        
        // 尝试在词边界开始
        let overlap_start = content[..start]
            .rfind(' ')
            .map(|pos| pos + 1)
            .unwrap_or(start);
        
        content[overlap_start..].to_string()
    }
    
    /// 分割大块
    fn split_large_chunk(
        &self,
        document_id: &str,
        large_chunk: &EnhancedDocumentChunk,
    ) -> Result<Vec<EnhancedDocumentChunk>> {
        let mut sub_chunks = Vec::new();
        let content = &large_chunk.content;
        
        // 使用语义分割
        let units = self.identify_semantic_units(content, &DocumentStructure::default());
        
        let mut current_content = String::new();
        let mut chunk_index = large_chunk.chunk_index;
        
        for unit in units {
            if self.should_start_new_chunk(&current_content, &unit.content) {
                if !current_content.is_empty() {
                    let mut sub_chunk = EnhancedDocumentChunk::new(
                        document_id.to_string(),
                        current_content.clone(),
                        chunk_index,
                        0,
                    );
                    
                    // 继承父块的结构信息
                    sub_chunk.section_path = large_chunk.section_path.clone();
                    sub_chunk.section_level = large_chunk.section_level;
                    
                    sub_chunks.push(sub_chunk);
                    chunk_index += 1;
                    current_content.clear();
                }
            }
            
            if !current_content.is_empty() {
                current_content.push_str("\n\n");
            }
            current_content.push_str(&unit.content);
        }
        
        // 处理剩余内容
        if !current_content.is_empty() {
            let mut sub_chunk = EnhancedDocumentChunk::new(
                document_id.to_string(),
                current_content,
                chunk_index,
                0,
            );
            sub_chunk.section_path = large_chunk.section_path.clone();
            sub_chunk.section_level = large_chunk.section_level;
            sub_chunks.push(sub_chunk);
        }
        
        Ok(sub_chunks)
    }
    
    /// 检测内容类型
    fn detect_content_type(
        &self,
        content: &str,
        element_indices: &[usize],
        elements: &[DocumentElement],
    ) -> ContentType {
        // 基于元素类型统计
        let mut type_counts = HashMap::new();
        
        for &idx in element_indices {
            if let Some(element) = elements.get(idx) {
                let content_type = match &element.element_type {
                    ElementType::CodeBlock { language } => ContentType::Code { 
                        language: language.clone() 
                    },
                    ElementType::Table => ContentType::Table,
                    ElementType::List { .. } => ContentType::List,
                    ElementType::BlockQuote => ContentType::Quote,
                    _ => ContentType::Text,
                };
                
                *type_counts.entry(format!("{:?}", content_type)).or_insert(0) += 1;
            }
        }
        
        // 如果大部分是某种类型，返回该类型
        if let Some((dominant_type, _)) = type_counts.iter().max_by_key(|(_, count)| *count) {
            match dominant_type.as_str() {
                t if t.starts_with("Code") => ContentType::Code { language: None },
                "Table" => ContentType::Table,
                "List" => ContentType::List,
                "Quote" => ContentType::Quote,
                _ => ContentType::Text,
            }
        } else {
            // 否则基于内容检测
            self.detect_content_type_from_text(content)
        }
    }
    
    /// 从文本检测内容类型
    fn detect_content_type_from_text(&self, content: &str) -> ContentType {
        if content.contains("```") {
            ContentType::Code { language: None }
        } else if content.contains('|') && content.contains("---") {
            ContentType::Table
        } else if content.lines().any(|line| line.trim_start().starts_with(&['-', '*', '+', '1'][..])) {
            ContentType::List
        } else if content.lines().all(|line| line.trim_start().starts_with('>')) {
            ContentType::Quote
        } else {
            ContentType::Text
        }
    }
    
    /// 分析文档特征
    fn analyze_document_features(&self, content: &str, structure: &DocumentStructure) -> DocumentFeatures {
        DocumentFeatures {
            has_clear_structure: !structure.outline.is_empty(),
            section_count: structure.outline.len(),
            code_block_ratio: structure.metadata.code_block_count as f32 / structure.elements.len().max(1) as f32,
            table_count: structure.metadata.table_count,
            average_paragraph_length: if structure.metadata.paragraph_count > 0 {
                content.len() as f32 / structure.metadata.paragraph_count as f32
            } else {
                0.0
            },
            document_length: content.len(),
        }
    }
    
    /// 选择最佳策略
    fn select_best_strategy(&self, features: &DocumentFeatures) -> ChunkingStrategy {
        // 基于文档特征选择策略
        if features.has_clear_structure && features.section_count > 3 {
            ChunkingStrategy::Structural
        } else if features.code_block_ratio > 0.3 || features.table_count > 2 {
            ChunkingStrategy::Hybrid
        } else if features.average_paragraph_length > 500.0 {
            ChunkingStrategy::Semantic
        } else {
            ChunkingStrategy::Simple
        }
    }
    
    /// 完成块的处理
    fn finalize_chunks(&self, chunks: &mut Vec<EnhancedDocumentChunk>) {
        let total_chunks = chunks.len();
        
        for (i, chunk) in chunks.iter_mut().enumerate() {
            // 更新总数
            chunk.total_chunks = total_chunks;
            
            // 设置链接关系
            if i > 0 {
                chunk.previous_chunk_id = Some(format!("{}_chunk_{:04}", chunk.document_id, i - 1));
            }
            if i < total_chunks - 1 {
                chunk.next_chunk_id = Some(format!("{}_chunk_{:04}", chunk.document_id, i + 1));
            }
            
            // 更新索引
            chunk.chunk_index = i;
            chunk.id = format!("{}_chunk_{:04}", chunk.document_id, i);
            
            // 添加处理标记
            chunk.processing_flags.push(format!("strategy:{:?}", self.config.strategy));
            chunk.extraction_method = format!("{:?}", self.config.strategy).to_lowercase();
            
            // 更新时间戳
            chunk.updated_at = chrono::Utc::now();
        }
    }
    
    /// 计算统计信息
    fn calculate_statistics(
        &self,
        chunks: &[EnhancedDocumentChunk],
        quality_assessments: &[QualityAssessment],
        processing_time: std::time::Duration,
    ) -> ChunkingStatistics {
        let total_chunks = chunks.len();
        
        let sizes: Vec<usize> = chunks.iter().map(|c| c.text_length).collect();
        let average_chunk_size = if total_chunks > 0 {
            sizes.iter().sum::<usize>() as f32 / total_chunks as f32
        } else {
            0.0
        };
        
        let min_chunk_size = sizes.iter().min().copied().unwrap_or(0);
        let max_chunk_size = sizes.iter().max().copied().unwrap_or(0);
        
        let average_quality_score = if !quality_assessments.is_empty() {
            quality_assessments.iter().map(|a| a.overall_score).sum::<f32>() / quality_assessments.len() as f32
        } else {
            0.0
        };
        
        ChunkingStatistics {
            total_chunks,
            average_chunk_size,
            min_chunk_size,
            max_chunk_size,
            average_quality_score,
            processing_time_ms: processing_time.as_millis() as u64,
            strategy_used: self.config.strategy.clone(),
        }
    }
}

/// 语义单元
#[derive(Debug, Clone)]
struct SemanticUnit {
    content: String,
    unit_type: SemanticUnitType,
    start_position: usize,
    end_position: usize,
    importance: f32,
}

/// 语义单元类型
#[derive(Debug, Clone)]
enum SemanticUnitType {
    Paragraph,
    Code,
    Table,
    List,
    Quote,
}

/// 文档特征
#[derive(Debug, Clone)]
struct DocumentFeatures {
    has_clear_structure: bool,
    section_count: usize,
    code_block_ratio: f32,
    table_count: usize,
    average_paragraph_length: f32,
    document_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intelligent_chunker_creation() {
        let config = ChunkingConfig::default();
        let chunker = IntelligentChunker::new(config).unwrap();
        assert!(chunker.config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_simple_chunking() {
        let config = ChunkingConfig {
            strategy: ChunkingStrategy::Simple,
            target_chunk_size: 100,
            ..ChunkingConfig::default()
        };
        
        let chunker = IntelligentChunker::new(config).unwrap();
        let content = "This is a test document. ".repeat(20);
        
        let result = chunker.chunk_document("doc1", "Test Title", &content).await.unwrap();
        
        assert!(result.chunks.len() > 1);
        assert_eq!(result.statistics.strategy_used, ChunkingStrategy::Simple);
    }

    #[tokio::test]
    async fn test_structural_chunking() {
        let config = ChunkingConfig {
            strategy: ChunkingStrategy::Structural,
            ..ChunkingConfig::default()
        };
        
        let chunker = IntelligentChunker::new(config).unwrap();
        let content = r#"
# Chapter 1

This is chapter 1 content.

## Section 1.1

This is section 1.1 content.

# Chapter 2

This is chapter 2 content.
"#;
        
        let result = chunker.chunk_document("doc1", "Test Doc", content).await.unwrap();
        
        assert!(result.chunks.len() >= 2);
        assert!(result.structure.outline.len() >= 3);
    }
}