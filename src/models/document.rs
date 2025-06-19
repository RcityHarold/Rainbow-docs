use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Option<String>,
    pub space_id: String,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub is_published: bool,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    pub author_id: String,
    pub last_editor_id: Option<String>,
    pub view_count: u32,
    pub metadata: DocumentMetadata,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub tags: Vec<String>,
    pub custom_fields: HashMap<String, serde_json::Value>,
    pub seo: SeoMetadata,
    pub reading_time: Option<u32>, // in minutes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub og_image: Option<String>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            custom_fields: HashMap::new(),
            seo: SeoMetadata::default(),
            reading_time: None,
        }
    }
}

impl Default for SeoMetadata {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            keywords: Vec::new(),
            og_image: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateDocumentRequest {
    #[validate(length(min = 1, max = 200, message = "Title must be between 1 and 200 characters"))]
    pub title: String,
    
    #[validate(length(min = 1, max = 100, message = "Slug must be between 1 and 100 characters"))]
    #[validate(regex(path = "SLUG_REGEX", message = "Slug can only contain lowercase letters, numbers, and hyphens"))]
    pub slug: String,
    
    pub content: Option<String>,
    pub excerpt: Option<String>,
    pub is_published: Option<bool>,
    pub parent_id: Option<String>,
    pub sort_order: Option<i32>,
    pub metadata: Option<DocumentMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateDocumentRequest {
    #[validate(length(min = 1, max = 200, message = "Title must be between 1 and 200 characters"))]
    pub title: Option<String>,
    
    pub content: Option<String>,
    pub excerpt: Option<String>,
    pub is_published: Option<bool>,
    pub parent_id: Option<String>,
    pub sort_order: Option<i32>,
    pub metadata: Option<DocumentMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub space_id: String,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub is_published: bool,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    pub author_id: String,
    pub last_editor_id: Option<String>,
    pub view_count: u32,
    pub metadata: DocumentMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub rendered_content: Option<String>, // HTML rendered from markdown
    pub children: Option<Vec<DocumentResponse>>,
    pub breadcrumbs: Option<Vec<BreadcrumbItem>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentListItem {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub is_published: bool,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    pub author_id: String,
    pub view_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub children_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentListItem>,
    pub total: u32,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BreadcrumbItem {
    pub id: String,
    pub title: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub search: Option<String>,
    pub parent_id: Option<String>,
    pub is_published: Option<bool>,
    pub author_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub sort: Option<String>, // "title", "created_at", "updated_at", "sort_order"
    pub order: Option<String>, // "asc", "desc"
}

impl Default for DocumentQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(20),
            search: None,
            parent_id: None,
            is_published: None,
            author_id: None,
            tags: None,
            sort: Some("sort_order".to_string()),
            order: Some("asc".to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentTreeNode {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub is_published: bool,
    pub sort_order: i32,
    pub children: Vec<DocumentTreeNode>,
}

// 正则表达式验证
lazy_static::lazy_static! {
    static ref SLUG_REGEX: regex::Regex = regex::Regex::new(r"^[a-z0-9-]+$").unwrap();
}

impl Document {
    pub fn new(
        space_id: String,
        title: String,
        slug: String,
        author_id: String,
    ) -> Self {
        Self {
            id: None,
            space_id,
            title,
            slug,
            content: String::new(),
            excerpt: None,
            is_published: false,
            parent_id: None,
            sort_order: 0,
            author_id,
            last_editor_id: None,
            view_count: 0,
            metadata: DocumentMetadata::default(),
            created_at: None,
            updated_at: None,
        }
    }

    pub fn is_author(&self, user_id: &str) -> bool {
        self.author_id == user_id
    }

    pub fn can_read(&self, user_id: Option<&str>, is_space_public: bool) -> bool {
        if !self.is_published {
            // 未发布的文档只有作者可以查看
            return match user_id {
                Some(uid) => self.is_author(uid),
                None => false,
            };
        }

        // 已发布的文档根据空间的公开性决定
        if is_space_public {
            return true;
        }

        // 私有空间需要用户认证
        user_id.is_some()
    }

    pub fn generate_excerpt(&self) -> String {
        if let Some(ref excerpt) = self.excerpt {
            return excerpt.clone();
        }

        // 从内容生成摘要
        let content = self.content.trim();
        if content.is_empty() {
            return String::new();
        }

        // 移除 Markdown 标记并截取前150个字符
        let plain_text = self.strip_markdown(&content);
        if plain_text.len() <= 150 {
            plain_text
        } else {
            format!("{}...", &plain_text[..147])
        }
    }

    fn strip_markdown(&self, content: &str) -> String {
        // 简单的 Markdown 清理，实际可以使用更复杂的解析器
        content
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join(" ")
            .replace("**", "")
            .replace("*", "")
            .replace("`", "")
            .replace("  ", " ")
            .trim()
            .to_string()
    }

    pub fn estimate_reading_time(&self) -> u32 {
        let word_count = self.content.split_whitespace().count();
        // 假设平均阅读速度为每分钟200词
        std::cmp::max(1, (word_count / 200) as u32)
    }
}

impl From<Document> for DocumentResponse {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id.unwrap_or_default(),
            space_id: doc.space_id,
            title: doc.title,
            slug: doc.slug,
            content: doc.content,
            excerpt: doc.excerpt,
            is_published: doc.is_published,
            parent_id: doc.parent_id,
            sort_order: doc.sort_order,
            author_id: doc.author_id,
            last_editor_id: doc.last_editor_id,
            view_count: doc.view_count,
            metadata: doc.metadata,
            created_at: doc.created_at.unwrap_or_else(Utc::now),
            updated_at: doc.updated_at.unwrap_or_else(Utc::now),
            rendered_content: None,
            children: None,
            breadcrumbs: None,
        }
    }
}

impl From<Document> for DocumentListItem {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id.unwrap_or_default(),
            title: doc.title,
            slug: doc.slug,
            excerpt: doc.excerpt,
            is_published: doc.is_published,
            parent_id: doc.parent_id,
            sort_order: doc.sort_order,
            author_id: doc.author_id,
            view_count: doc.view_count,
            created_at: doc.created_at.unwrap_or_else(Utc::now),
            updated_at: doc.updated_at.unwrap_or_else(Utc::now),
            tags: doc.metadata.tags,
            children_count: 0, // 需要在查询时填充
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new(
            "space_123".to_string(),
            "Test Document".to_string(),
            "test-document".to_string(),
            "user_123".to_string(),
        );

        assert_eq!(doc.title, "Test Document");
        assert_eq!(doc.slug, "test-document");
        assert_eq!(doc.space_id, "space_123");
        assert_eq!(doc.author_id, "user_123");
        assert!(!doc.is_published);
    }

    #[test]
    fn test_excerpt_generation() {
        let mut doc = Document::new(
            "space_123".to_string(),
            "Test Document".to_string(),
            "test-document".to_string(),
            "user_123".to_string(),
        );

        // Test with explicit excerpt
        doc.excerpt = Some("Custom excerpt".to_string());
        assert_eq!(doc.generate_excerpt(), "Custom excerpt");

        // Test with content
        doc.excerpt = None;
        doc.content = "This is a long content that should be truncated when generating an excerpt. ".repeat(10);
        let excerpt = doc.generate_excerpt();
        assert!(excerpt.len() <= 150);
        assert!(excerpt.ends_with("..."));
    }

    #[test]
    fn test_reading_time_estimation() {
        let mut doc = Document::new(
            "space_123".to_string(),
            "Test Document".to_string(),
            "test-document".to_string(),
            "user_123".to_string(),
        );

        doc.content = "word ".repeat(400); // 400 words
        assert_eq!(doc.estimate_reading_time(), 2); // 400/200 = 2 minutes

        doc.content = "word ".repeat(100); // 100 words
        assert_eq!(doc.estimate_reading_time(), 1); // minimum 1 minute
    }

    #[test]
    fn test_document_access_control() {
        let mut doc = Document::new(
            "space_123".to_string(),
            "Test Document".to_string(),
            "test-document".to_string(),
            "user_123".to_string(),
        );

        // Unpublished document - only author can read
        assert!(doc.can_read(Some("user_123"), true));
        assert!(!doc.can_read(Some("user_456"), true));
        assert!(!doc.can_read(None, true));

        // Published document in public space - anyone can read
        doc.is_published = true;
        assert!(doc.can_read(Some("user_123"), true));
        assert!(doc.can_read(Some("user_456"), true));
        assert!(doc.can_read(None, true));

        // Published document in private space - authenticated users only
        assert!(doc.can_read(Some("user_123"), false));
        assert!(doc.can_read(Some("user_456"), false));
        assert!(!doc.can_read(None, false));
    }
}