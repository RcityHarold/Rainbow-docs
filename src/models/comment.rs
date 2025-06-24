use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Option<String>,
    pub document_id: String,
    pub parent_id: Option<String>, // 回复评论的ID
    pub author_id: String,
    pub content: String,
    pub is_resolved: bool,
    pub metadata: CommentMetadata,
    pub liked_by: Vec<String>, // 点赞用户列表
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentMetadata {
    pub mentions: Vec<String>, // 提及的用户ID
    pub attachments: Vec<String>, // 附件URL
    pub custom_fields: HashMap<String, serde_json::Value>,
}

impl Default for CommentMetadata {
    fn default() -> Self {
        Self {
            mentions: Vec::new(),
            attachments: Vec::new(),
            custom_fields: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateCommentRequest {
    #[validate(length(min = 1, max = 2000, message = "Content must be between 1 and 2000 characters"))]
    pub content: String,
    
    pub parent_id: Option<String>, // 回复的评论ID
    pub metadata: Option<CommentMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateCommentRequest {
    #[validate(length(min = 1, max = 2000, message = "Content must be between 1 and 2000 characters"))]
    pub content: Option<String>,
    
    pub is_resolved: Option<bool>,
    pub metadata: Option<CommentMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentResponse {
    pub id: String,
    pub document_id: String,
    pub parent_id: Option<String>,
    pub author_id: String,
    pub author_info: Option<CommentAuthor>, // 作者信息
    pub content: String,
    pub is_resolved: bool,
    pub metadata: CommentMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub replies: Option<Vec<CommentResponse>>, // 回复列表
    pub can_edit: bool, // 当前用户是否可以编辑
    pub can_delete: bool, // 当前用户是否可以删除
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentAuthor {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentListResponse {
    pub comments: Vec<CommentResponse>,
    pub total: u32,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub parent_id: Option<String>, // 只获取特定父评论的回复
    pub author_id: Option<String>,
    pub is_resolved: Option<bool>,
    pub sort: Option<String>, // "created_at", "updated_at"
    pub order: Option<String>, // "asc", "desc"
}

impl Default for CommentQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(20),
            parent_id: None,
            author_id: None,
            is_resolved: None,
            sort: Some("created_at".to_string()),
            order: Some("asc".to_string()),
        }
    }
}

impl Comment {
    pub fn new(document_id: String, author_id: String, content: String) -> Self {
        Self {
            id: None,
            document_id,
            parent_id: None,
            author_id,
            content,
            is_resolved: false,
            metadata: CommentMetadata::default(),
            liked_by: Vec::new(),
            is_deleted: false,
            deleted_at: None,
            deleted_by: None,
            created_at: None,
            updated_at: None,
        }
    }

    pub fn is_author(&self, user_id: &str) -> bool {
        self.author_id == user_id
    }

    pub fn is_reply(&self) -> bool {
        self.parent_id.is_some()
    }

    pub fn can_edit(&self, user_id: &str, is_moderator: bool) -> bool {
        self.is_author(user_id) || is_moderator
    }

    pub fn can_delete(&self, user_id: &str, is_moderator: bool) -> bool {
        self.is_author(user_id) || is_moderator
    }

    pub fn with_parent(mut self, parent_id: String) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn update_content(&mut self, content: String, editor_id: String) {
        self.content = content;
        self.updated_at = Some(Utc::now());
        // Note: editor_id could be used for audit trail if needed
    }

    pub fn soft_delete(&mut self, deleter_id: String) {
        self.is_deleted = true;
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(deleter_id);
    }

    pub fn like(&mut self, user_id: String) {
        if !self.liked_by.contains(&user_id) {
            self.liked_by.push(user_id);
        }
    }

    pub fn unlike(&mut self, user_id: String) {
        self.liked_by.retain(|id| id != &user_id);
    }

    pub fn is_liked_by(&self, user_id: &str) -> bool {
        self.liked_by.contains(&user_id.to_string())
    }

    pub fn like_count(&self) -> usize {
        self.liked_by.len()
    }
}

impl From<Comment> for CommentResponse {
    fn from(comment: Comment) -> Self {
        Self {
            id: comment.id.unwrap_or_default(),
            document_id: comment.document_id,
            parent_id: comment.parent_id,
            author_id: comment.author_id,
            author_info: None, // 需要在服务层填充
            content: comment.content,
            is_resolved: comment.is_resolved,
            metadata: comment.metadata,
            created_at: comment.created_at.unwrap_or_else(Utc::now),
            updated_at: comment.updated_at.unwrap_or_else(Utc::now),
            replies: None, // 需要在服务层填充
            can_edit: false, // 需要在服务层计算
            can_delete: false, // 需要在服务层计算
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_creation() {
        let comment = Comment::new(
            "doc123".to_string(),
            "user456".to_string(),
            "This is a comment".to_string(),
        );

        assert_eq!(comment.document_id, "doc123");
        assert_eq!(comment.author_id, "user456");
        assert_eq!(comment.content, "This is a comment");
        assert!(!comment.is_resolved);
        assert!(!comment.is_reply());
    }

    #[test]
    fn test_comment_permissions() {
        let comment = Comment::new(
            "doc123".to_string(),
            "user456".to_string(),
            "Test comment".to_string(),
        );

        // 作者可以编辑和删除
        assert!(comment.can_edit("user456", false));
        assert!(comment.can_delete("user456", false));

        // 其他用户不能编辑或删除
        assert!(!comment.can_edit("user789", false));
        assert!(!comment.can_delete("user789", false));

        // 管理员可以编辑和删除
        assert!(comment.can_edit("user789", true));
        assert!(comment.can_delete("user789", true));
    }

    #[test]
    fn test_create_comment_validation() {
        let valid_request = CreateCommentRequest {
            content: "This is a valid comment".to_string(),
            parent_id: None,
            metadata: None,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateCommentRequest {
            content: "".to_string(), // 无效：空内容
            parent_id: None,
            metadata: None,
        };
        assert!(invalid_request.validate().is_err());

        let too_long_request = CreateCommentRequest {
            content: "x".repeat(2001), // 无效：超过2000字符
            parent_id: None,
            metadata: None,
        };
        assert!(too_long_request.validate().is_err());
    }
}