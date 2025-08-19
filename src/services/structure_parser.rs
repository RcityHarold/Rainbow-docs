use std::collections::HashMap;
use pulldown_cmark::{Parser, Event, Tag, CowStr, CodeBlockKind};
use serde::{Deserialize, Serialize};

/// 文档结构解析器
pub struct DocumentStructureParser {
    /// 解析配置
    config: ParserConfig,
}

/// 解析配置
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// 是否提取标题
    pub extract_headers: bool,
    /// 是否提取代码块
    pub extract_code_blocks: bool,
    /// 是否提取表格
    pub extract_tables: bool,
    /// 是否提取列表
    pub extract_lists: bool,
    /// 是否提取链接
    pub extract_links: bool,
    /// 最大解析深度
    pub max_depth: usize,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            extract_headers: true,
            extract_code_blocks: true,
            extract_tables: true,
            extract_lists: true,
            extract_links: false,
            max_depth: 10,
        }
    }
}

/// 文档结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentStructure {
    /// 文档元数据
    pub metadata: DocumentMetadata,
    /// 章节大纲
    pub outline: Vec<Section>,
    /// 文档元素
    pub elements: Vec<DocumentElement>,
    /// 原始内容长度
    pub content_length: usize,
}

/// 文档元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentMetadata {
    /// 标题数量（按级别）
    pub header_counts: HashMap<u8, usize>,
    /// 代码块数量
    pub code_block_count: usize,
    /// 表格数量
    pub table_count: usize,
    /// 列表数量
    pub list_count: usize,
    /// 段落数量
    pub paragraph_count: usize,
    /// 预估阅读时间（分钟）
    pub estimated_reading_time: f32,
}

/// 章节结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// 唯一标识
    pub id: String,
    /// 标题文本
    pub title: String,
    /// 标题级别 (1-6)
    pub level: u8,
    /// 章节路径
    pub path: Vec<String>,
    /// 在文档中的起始位置
    pub start_position: usize,
    /// 在文档中的结束位置
    pub end_position: usize,
    /// 子章节
    pub children: Vec<Section>,
    /// 包含的元素索引
    pub element_indices: Vec<usize>,
    /// 章节内容长度
    pub content_length: usize,
}

/// 文档元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentElement {
    /// 元素类型
    pub element_type: ElementType,
    /// 元素内容
    pub content: String,
    /// 在文档中的位置
    pub position: (usize, usize),
    /// 所属章节ID
    pub section_id: Option<String>,
    /// 附加属性
    pub attributes: HashMap<String, String>,
}

/// 元素类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ElementType {
    /// 标题
    Header { level: u8 },
    /// 段落
    Paragraph,
    /// 代码块
    CodeBlock { language: Option<String> },
    /// 内联代码
    InlineCode,
    /// 表格
    Table,
    /// 列表
    List { ordered: bool },
    /// 列表项
    ListItem,
    /// 引用
    BlockQuote,
    /// 链接
    Link { url: String },
    /// 图片
    Image { url: String, alt: String },
    /// 分割线
    Rule,
    /// 普通文本
    Text,
}

impl DocumentStructureParser {
    /// 创建新的解析器
    pub fn new(config: ParserConfig) -> Self {
        Self { config }
    }
    
    /// 使用默认配置创建解析器
    pub fn new_default() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }
    
    /// 解析Markdown文档结构
    pub fn parse(&self, markdown: &str) -> DocumentStructure {
        let parser = Parser::new(markdown);
        let mut structure = DocumentStructure {
            metadata: DocumentMetadata {
                header_counts: HashMap::new(),
                code_block_count: 0,
                table_count: 0,
                list_count: 0,
                paragraph_count: 0,
                estimated_reading_time: 0.0,
            },
            outline: Vec::new(),
            elements: Vec::new(),
            content_length: markdown.len(),
        };
        
        let mut parser_state = ParserState::new();
        let mut current_position = 0;
        
        for event in parser {
            match event {
                Event::Start(tag) => {
                    self.handle_start_tag(&mut structure, &mut parser_state, tag, current_position);
                }
                Event::End(tag) => {
                    self.handle_end_tag(&mut structure, &mut parser_state, tag, current_position);
                }
                Event::Text(text) => {
                    let text_len = text.len();
                    self.handle_text(&mut structure, &mut parser_state, text, current_position);
                    current_position += text_len;
                }
                Event::Code(code) => {
                    if self.config.extract_code_blocks {
                        let element = DocumentElement {
                            element_type: ElementType::InlineCode,
                            content: code.to_string(),
                            position: (current_position, current_position + code.len()),
                            section_id: parser_state.current_section_id.clone(),
                            attributes: HashMap::new(),
                        };
                        structure.elements.push(element);
                    }
                    current_position += code.len();
                }
                Event::SoftBreak | Event::HardBreak => {
                    current_position += 1;
                }
                Event::Rule => {
                    let element = DocumentElement {
                        element_type: ElementType::Rule,
                        content: "---".to_string(),
                        position: (current_position, current_position + 3),
                        section_id: parser_state.current_section_id.clone(),
                        attributes: HashMap::new(),
                    };
                    structure.elements.push(element);
                    current_position += 3;
                }
                _ => {}
            }
        }
        
        // 完善章节的结束位置
        self.finalize_sections(&mut structure);
        
        // 计算元数据
        self.calculate_metadata(&mut structure);
        
        structure
    }
    
    fn handle_start_tag(
        &self,
        structure: &mut DocumentStructure,
        state: &mut ParserState,
        tag: Tag,
        position: usize,
    ) {
        match tag {
            Tag::Heading(level, _, _) => {
                if self.config.extract_headers && level as usize <= self.config.max_depth {
                    let header_level = level as u8;
                    state.current_header_level = Some(header_level);
                    state.current_element_start = position;
                    
                    // 更新计数
                    *structure.metadata.header_counts.entry(header_level).or_insert(0) += 1;
                }
            }
            Tag::Paragraph => {
                state.in_paragraph = true;
                state.current_element_start = position;
                state.current_text_buffer.clear();
            }
            Tag::CodeBlock(info) => {
                if self.config.extract_code_blocks {
                    state.in_code_block = true;
                    state.current_element_start = position;
                    state.code_language = match info {
                        CodeBlockKind::Fenced(lang) => {
                            lang.split_whitespace().next().map(|s| s.to_string())
                        },
                        _ => None,
                    };
                    state.current_text_buffer.clear();
                    structure.metadata.code_block_count += 1;
                }
            }
            Tag::List(_) => {
                if self.config.extract_lists {
                    state.in_list = true;
                    state.list_depth += 1;
                    state.current_element_start = position;
                    structure.metadata.list_count += 1;
                }
            }
            Tag::Table(_) => {
                if self.config.extract_tables {
                    state.in_table = true;
                    state.current_element_start = position;
                    state.current_text_buffer.clear();
                    structure.metadata.table_count += 1;
                }
            }
            Tag::BlockQuote => {
                state.in_blockquote = true;
                state.current_element_start = position;
                state.current_text_buffer.clear();
            }
            Tag::Link(_, dest_url, _) => {
                if self.config.extract_links {
                    state.current_link_url = Some(dest_url.to_string());
                    state.current_element_start = position;
                    state.current_text_buffer.clear();
                }
            }
            Tag::Image(_, dest_url, _) => {
                state.current_image_url = Some(dest_url.to_string());
                state.current_element_start = position;
                state.current_text_buffer.clear();
            }
            _ => {}
        }
    }
    
    fn handle_end_tag(
        &self,
        structure: &mut DocumentStructure,
        state: &mut ParserState,
        tag: Tag,
        position: usize,
    ) {
        match tag {
            Tag::Heading(level, _, _) => {
                if self.config.extract_headers && 
                   state.current_header_level == Some(level as u8) {
                    
                    let title = state.current_text_buffer.trim().to_string();
                    if !title.is_empty() {
                        let section = self.create_section(
                            title,
                            level as u8,
                            state.current_element_start,
                            position,
                            &structure.outline,
                        );
                        
                        state.current_section_id = Some(section.id.clone());
                        structure.outline.push(section);
                    }
                    
                    state.current_header_level = None;
                    state.current_text_buffer.clear();
                }
            }
            Tag::Paragraph => {
                if state.in_paragraph {
                    let content = state.current_text_buffer.trim().to_string();
                    if !content.is_empty() {
                        let element = DocumentElement {
                            element_type: ElementType::Paragraph,
                            content,
                            position: (state.current_element_start, position),
                            section_id: state.current_section_id.clone(),
                            attributes: HashMap::new(),
                        };
                        structure.elements.push(element);
                        structure.metadata.paragraph_count += 1;
                    }
                    
                    state.in_paragraph = false;
                    state.current_text_buffer.clear();
                }
            }
            Tag::CodeBlock(_) => {
                if state.in_code_block {
                    let element = DocumentElement {
                        element_type: ElementType::CodeBlock {
                            language: state.code_language.take(),
                        },
                        content: state.current_text_buffer.clone(),
                        position: (state.current_element_start, position),
                        section_id: state.current_section_id.clone(),
                        attributes: HashMap::new(),
                    };
                    structure.elements.push(element);
                    
                    state.in_code_block = false;
                    state.current_text_buffer.clear();
                }
            }
            Tag::List(_) => {
                if state.in_list {
                    state.list_depth = state.list_depth.saturating_sub(1);
                    if state.list_depth == 0 {
                        state.in_list = false;
                    }
                }
            }
            Tag::Table(_) => {
                if state.in_table {
                    let element = DocumentElement {
                        element_type: ElementType::Table,
                        content: state.current_text_buffer.clone(),
                        position: (state.current_element_start, position),
                        section_id: state.current_section_id.clone(),
                        attributes: HashMap::new(),
                    };
                    structure.elements.push(element);
                    
                    state.in_table = false;
                    state.current_text_buffer.clear();
                }
            }
            Tag::BlockQuote => {
                if state.in_blockquote {
                    let element = DocumentElement {
                        element_type: ElementType::BlockQuote,
                        content: state.current_text_buffer.clone(),
                        position: (state.current_element_start, position),
                        section_id: state.current_section_id.clone(),
                        attributes: HashMap::new(),
                    };
                    structure.elements.push(element);
                    
                    state.in_blockquote = false;
                    state.current_text_buffer.clear();
                }
            }
            Tag::Link(_, _, _) => {
                if let Some(url) = state.current_link_url.take() {
                    let mut attributes = HashMap::new();
                    attributes.insert("url".to_string(), url.clone());
                    
                    let element = DocumentElement {
                        element_type: ElementType::Link { url },
                        content: state.current_text_buffer.clone(),
                        position: (state.current_element_start, position),
                        section_id: state.current_section_id.clone(),
                        attributes,
                    };
                    structure.elements.push(element);
                    state.current_text_buffer.clear();
                }
            }
            Tag::Image(_, _, _) => {
                if let Some(url) = state.current_image_url.take() {
                    let alt = state.current_text_buffer.clone();
                    let mut attributes = HashMap::new();
                    attributes.insert("url".to_string(), url.clone());
                    attributes.insert("alt".to_string(), alt.clone());
                    
                    let element = DocumentElement {
                        element_type: ElementType::Image { url, alt },
                        content: state.current_text_buffer.clone(),
                        position: (state.current_element_start, position),
                        section_id: state.current_section_id.clone(),
                        attributes,
                    };
                    structure.elements.push(element);
                    state.current_text_buffer.clear();
                }
            }
            _ => {}
        }
    }
    
    fn handle_text(
        &self,
        _structure: &mut DocumentStructure,
        state: &mut ParserState,
        text: CowStr,
        _position: usize,
    ) {
        state.current_text_buffer.push_str(&text);
    }
    
    fn create_section(
        &self,
        title: String,
        level: u8,
        start_position: usize,
        end_position: usize,
        existing_sections: &[Section],
    ) -> Section {
        // 构建章节路径
        let mut path = Vec::new();
        
        // 找到父章节
        for section in existing_sections.iter().rev() {
            if section.level < level {
                path = section.path.clone();
                path.push(section.title.clone());
                break;
            }
        }
        
        // 生成唯一ID
        let id = format!(
            "section_{}_{}", 
            level,
            title.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "_")
        );
        
        Section {
            id,
            title: title.clone(),
            level,
            path,
            start_position,
            end_position,
            children: Vec::new(),
            element_indices: Vec::new(),
            content_length: end_position - start_position,
        }
    }
    
    fn finalize_sections(&self, structure: &mut DocumentStructure) {
        // 先计算所有章节的结束位置
        let section_count = structure.outline.len();
        for i in 0..section_count {
            let current_start = structure.outline[i].start_position;
            let mut end_position = structure.content_length;
            
            // 查找下一个同级或更高级章节
            for j in (i + 1)..section_count {
                if structure.outline[j].level <= structure.outline[i].level {
                    end_position = structure.outline[j].start_position;
                    break;
                }
            }
            
            structure.outline[i].end_position = end_position;
            structure.outline[i].content_length = end_position - current_start;
        }
        
        // 然后更新每个章节包含的元素
        for i in 0..section_count {
            let start_pos = structure.outline[i].start_position;
            let end_pos = structure.outline[i].end_position;
            
            structure.outline[i].element_indices = structure
                .elements
                .iter()
                .enumerate()
                .filter(|(_, element)| {
                    element.position.0 >= start_pos && element.position.1 <= end_pos
                })
                .map(|(index, _)| index)
                .collect();
        }
    }
    
    fn calculate_metadata(&self, structure: &mut DocumentStructure) {
        // 估算阅读时间 (假设每分钟200个中文字符或800个英文单词)
        let word_count = structure.content_length as f32;
        structure.metadata.estimated_reading_time = (word_count / 400.0).max(1.0);
    }
}

/// 解析器状态
struct ParserState {
    current_header_level: Option<u8>,
    current_section_id: Option<String>,
    current_element_start: usize,
    current_text_buffer: String,
    in_paragraph: bool,
    in_code_block: bool,
    in_list: bool,
    in_table: bool,
    in_blockquote: bool,
    list_depth: usize,
    code_language: Option<String>,
    current_link_url: Option<String>,
    current_image_url: Option<String>,
}

impl ParserState {
    fn new() -> Self {
        Self {
            current_header_level: None,
            current_section_id: None,
            current_element_start: 0,
            current_text_buffer: String::new(),
            in_paragraph: false,
            in_code_block: false,
            in_list: false,
            in_table: false,
            in_blockquote: false,
            list_depth: 0,
            code_language: None,
            current_link_url: None,
            current_image_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headers() {
        let parser = DocumentStructureParser::new_default();
        let markdown = r#"
# Chapter 1
Content of chapter 1

## Section 1.1
Content of section 1.1

### Subsection 1.1.1
Content of subsection

# Chapter 2
Content of chapter 2
"#;
        
        let structure = parser.parse(markdown);
        assert_eq!(structure.outline.len(), 4);
        assert_eq!(structure.outline[0].level, 1);
        assert_eq!(structure.outline[0].title, "Chapter 1");
        assert_eq!(structure.outline[1].level, 2);
        assert_eq!(structure.outline[1].title, "Section 1.1");
    }

    #[test]
    fn test_parse_code_blocks() {
        let parser = DocumentStructureParser::new_default();
        let markdown = r#"
Here's some Rust code:

```rust
fn main() {
    println!("Hello, world!");
}
```

And some inline `code`.
"#;
        
        let structure = parser.parse(markdown);
        let code_blocks: Vec<_> = structure.elements
            .iter()
            .filter(|e| matches!(e.element_type, ElementType::CodeBlock { .. }))
            .collect();
        
        assert_eq!(code_blocks.len(), 1);
        if let ElementType::CodeBlock { language } = &code_blocks[0].element_type {
            assert_eq!(language, &Some("rust".to_string()));
        }
    }

    #[test]
    fn test_metadata_calculation() {
        let parser = DocumentStructureParser::new_default();
        let markdown = r#"
# Title

## Section
Some paragraph content.

```rust
code block
```

- List item 1
- List item 2

| Column 1 | Column 2 |
|----------|----------|
| Data 1   | Data 2   |
"#;
        
        let structure = parser.parse(markdown);
        assert_eq!(structure.metadata.header_counts.get(&1), Some(&1));
        assert_eq!(structure.metadata.header_counts.get(&2), Some(&1));
        assert_eq!(structure.metadata.code_block_count, 1);
        assert_eq!(structure.metadata.list_count, 1);
        assert_eq!(structure.metadata.table_count, 1);
        assert!(structure.metadata.estimated_reading_time > 0.0);
    }
}