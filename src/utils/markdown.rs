use crate::error::{AppError, Result};
use pulldown_cmark::{Parser, Options, html};
use syntect::html::{ClassedHTMLGenerator, ClassStyle};
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Theme};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedContent {
    pub html: String,
    pub word_count: u32,
    pub reading_time: u32,
    pub excerpt: String,
    pub toc: Vec<TocEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TocEntry {
    pub level: u8,
    pub title: String,
    pub id: String,
}

pub struct MarkdownProcessor {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    default_theme: Theme,
}

impl MarkdownProcessor {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let default_theme = theme_set.themes["base16-ocean.dark"].clone();

        Self {
            syntax_set,
            theme_set,
            default_theme,
        }
    }

    /// 完整处理Markdown内容
    pub async fn process(&self, markdown: &str) -> Result<ProcessedContent> {
        let html = self.render(markdown)?;
        let word_count = self.count_words(markdown);
        let reading_time = self.estimate_reading_time(markdown);
        let excerpt = self.extract_excerpt(markdown, 200);
        let toc = self.extract_toc(markdown)?;

        Ok(ProcessedContent {
            html,
            word_count,
            reading_time,
            excerpt,
            toc,
        })
    }

    /// 统计字数
    pub fn count_words(&self, markdown: &str) -> u32 {
        let plain_text = self.strip_markdown(markdown);
        plain_text.split_whitespace().count() as u32
    }

    /// 提取目录
    pub fn extract_toc(&self, markdown: &str) -> Result<Vec<TocEntry>> {
        let mut toc = Vec::new();
        let parser = Parser::new(markdown);
        let mut current_heading = None;
        let mut heading_counter = 0;

        for event in parser {
            match event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading(level, _, _)) => {
                    current_heading = Some((level as u8, String::new()));
                }
                pulldown_cmark::Event::Text(text) if current_heading.is_some() => {
                    if let Some((level, ref mut title)) = current_heading {
                        title.push_str(&text);
                    }
                }
                pulldown_cmark::Event::End(pulldown_cmark::Tag::Heading(_, _, _)) => {
                    if let Some((level, title)) = current_heading.take() {
                        heading_counter += 1;
                        let id = format!("heading-{}", heading_counter);
                        toc.push(TocEntry { level, title, id });
                    }
                }
                _ => {}
            }
        }

        Ok(toc)
    }

    /// 将Markdown渲染为HTML
    pub fn render(&self, markdown: &str) -> Result<String> {
        // 配置Markdown解析选项
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);

        // 解析Markdown
        let parser = Parser::new_ext(markdown, options);

        // 处理代码块语法高亮
        let parser = parser.map(|event| {
            match &event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(kind)) => {
                    match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                            pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(
                                pulldown_cmark::CodeBlockKind::Fenced(lang.clone())
                            ))
                        }
                        _ => event,
                    }
                }
                pulldown_cmark::Event::Text(_) => {
                    // 这里可以添加自定义文本处理逻辑
                    event
                }
                _ => event,
            }
        });

        // 渲染为HTML
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        // 后处理：添加代码高亮
        let processed_html = self.process_code_blocks(&html_output)?;

        Ok(processed_html)
    }

    /// 生成目录(TOC)
    pub fn generate_toc(&self, markdown: &str) -> Result<Vec<TocItem>> {
        let mut toc = Vec::new();
        let mut options = Options::empty();
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

        let parser = Parser::new_ext(markdown, options);
        let mut heading_id_counter = HashMap::new();

        for event in parser {
            if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading(level, fragment_id, _)) = event {
                // 下一个事件应该是标题文本
                let title = "".to_string(); // 这里需要更复杂的逻辑来提取标题文本
                
                let id = if let Some(id) = fragment_id {
                    id.to_string()
                } else {
                    // 生成ID
                    self.generate_heading_id(&title, &mut heading_id_counter)
                };

                toc.push(TocItem {
                    level: level as u32,
                    title,
                    id,
                });
            }
        }

        Ok(toc)
    }

    /// 提取摘要
    pub fn extract_excerpt(&self, markdown: &str, max_length: usize) -> String {
        // 移除Markdown标记
        let plain_text = self.strip_markdown(markdown);
        
        if plain_text.len() <= max_length {
            plain_text
        } else {
            // 使用字符边界安全截断
            let mut end = max_length;
            
            // 确保不会在字符中间截断
            while !plain_text.is_char_boundary(end) && end > 0 {
                end -= 1;
            }
            
            if end == 0 {
                return String::new();
            }
            
            let truncated = &plain_text[..end];
            
            // 尝试在单词或中文字符边界截断
            let chars: Vec<char> = truncated.chars().collect();
            if let Some(pos) = chars.iter().rposition(|&c| c.is_whitespace() || c == '。' || c == '，' || c == '！' || c == '？') {
                let safe_truncated: String = chars[..=pos].iter().collect();
                format!("{}...", safe_truncated.trim())
            } else {
                format!("{}...", truncated)
            }
        }
    }

    /// 估算阅读时间
    pub fn estimate_reading_time(&self, markdown: &str) -> u32 {
        let plain_text = self.strip_markdown(markdown);
        let word_count = plain_text.split_whitespace().count();
        // 假设阅读速度为每分钟200词
        std::cmp::max(1, (word_count / 200) as u32)
    }

    /// 验证Markdown语法
    pub fn validate(&self, markdown: &str) -> Result<Vec<ValidationError>> {
        let mut errors = Vec::new();
        let parser = Parser::new(markdown);
        
        for (pos, event) in parser.enumerate() {
            match event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(kind)) => {
                    if let pulldown_cmark::CodeBlockKind::Fenced(lang) = kind {
                        // 验证语言标识符
                        if !lang.is_empty() && !self.is_valid_language(&lang) {
                            errors.push(ValidationError {
                                position: pos,
                                message: format!("Unknown language: {}", lang),
                                error_type: ValidationErrorType::UnknownLanguage,
                            });
                        }
                    }
                }
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link(_, url, _)) => {
                    // 验证链接
                    if url.is_empty() {
                        errors.push(ValidationError {
                            position: pos,
                            message: "Empty link URL".to_string(),
                            error_type: ValidationErrorType::InvalidLink,
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(errors)
    }

    // 私有方法

    fn process_code_blocks(&self, html: &str) -> Result<String> {
        // 简化版代码高亮处理
        // 实际实现需要解析HTML并替换<code>块
        Ok(html.to_string())
    }

    fn strip_markdown(&self, markdown: &str) -> String {
        let parser = Parser::new(markdown);
        let mut text = String::new();

        for event in parser {
            match event {
                pulldown_cmark::Event::Text(t) => text.push_str(&t),
                pulldown_cmark::Event::SoftBreak | pulldown_cmark::Event::HardBreak => {
                    text.push(' ');
                }
                _ => {}
            }
        }

        // 清理多余的空白
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn generate_heading_id(&self, title: &str, counter: &mut HashMap<String, u32>) -> String {
        // 生成URL友好的ID
        let base_id = title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();

        // 处理重复ID
        let count = counter.entry(base_id.clone()).or_insert(0);
        *count += 1;

        if *count == 1 {
            base_id
        } else {
            format!("{}-{}", base_id, count)
        }
    }

    fn is_valid_language(&self, lang: &str) -> bool {
        // 检查是否是有效的编程语言标识符
        self.syntax_set.find_syntax_by_token(lang).is_some()
    }
}

#[derive(Debug, Clone)]
pub struct TocItem {
    pub level: u32,
    pub title: String,
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub position: usize,
    pub message: String,
    pub error_type: ValidationErrorType,
}

#[derive(Debug, Clone)]
pub enum ValidationErrorType {
    UnknownLanguage,
    InvalidLink,
    SyntaxError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_rendering() {
        let processor = MarkdownProcessor::new();
        let markdown = "# Hello World\n\nThis is **bold** text.";
        let html = processor.render(markdown).unwrap();
        
        assert!(html.contains("<h1>"));
        assert!(html.contains("<strong>"));
    }

    #[test]
    fn test_excerpt_extraction() {
        let processor = MarkdownProcessor::new();
        let markdown = "# Title\n\nThis is a long paragraph that should be truncated at some point to create a proper excerpt.";
        let excerpt = processor.extract_excerpt(markdown, 50);
        
        assert!(excerpt.len() <= 53); // 50 + "..."
        assert!(excerpt.ends_with("..."));
    }

    #[test]
    fn test_reading_time_estimation() {
        let processor = MarkdownProcessor::new();
        let markdown = &"word ".repeat(200); // 200 words
        let time = processor.estimate_reading_time(markdown);
        
        assert_eq!(time, 1); // Should be 1 minute
    }

    #[test]
    fn test_strip_markdown() {
        let processor = MarkdownProcessor::new();
        let markdown = "# Title\n\nThis is **bold** and *italic* text with [link](url).";
        let plain = processor.strip_markdown(markdown);
        
        assert_eq!(plain, "Title This is bold and italic text with link.");
    }
}