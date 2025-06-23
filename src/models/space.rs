use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: Option<String>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub is_public: bool,
    pub owner_id: String,
    pub settings: SpaceSettings,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceSettings {
    pub theme: String,
    pub allow_comments: bool,
    pub allow_search: bool,
    pub custom_domain: Option<String>,
    pub analytics_id: Option<String>,
    pub custom_css: Option<String>,
    pub navigation: NavigationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationSettings {
    pub show_breadcrumbs: bool,
    pub show_navigation: bool,
    pub show_edit_links: bool,
    pub custom_links: Vec<CustomLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomLink {
    pub title: String,
    pub url: String,
    pub icon: Option<String>,
    pub order: i32,
}

impl Default for SpaceSettings {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            allow_comments: true,
            allow_search: true,
            custom_domain: None,
            analytics_id: None,
            custom_css: None,
            navigation: NavigationSettings::default(),
        }
    }
}

impl Default for NavigationSettings {
    fn default() -> Self {
        Self {
            show_breadcrumbs: true,
            show_navigation: true,
            show_edit_links: true,
            custom_links: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateSpaceRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be between 1 and 100 characters"))]
    pub name: String,
    
    #[validate(length(min = 1, max = 50, message = "Slug must be between 1 and 50 characters"))]
    #[validate(regex(path = "crate::models::space::SLUG_REGEX", message = "Slug can only contain lowercase letters, numbers, and hyphens"))]
    pub slug: String,
    
    #[validate(length(max = 500, message = "Description cannot exceed 500 characters"))]
    pub description: Option<String>,
    
    pub avatar_url: Option<String>,
    pub is_public: Option<bool>,
    pub settings: Option<SpaceSettings>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateSpaceRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be between 1 and 100 characters"))]
    pub name: Option<String>,
    
    #[validate(length(max = 500, message = "Description cannot exceed 500 characters"))]
    pub description: Option<String>,
    
    pub avatar_url: Option<String>,
    pub is_public: Option<bool>,
    pub settings: Option<SpaceSettings>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpaceResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub is_public: bool,
    pub owner_id: String,
    pub settings: SpaceSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub stats: Option<SpaceStats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpaceStats {
    pub document_count: u32,
    pub public_document_count: u32,
    pub comment_count: u32,
    pub view_count: u32,
    pub last_activity: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpaceListResponse {
    pub spaces: Vec<SpaceResponse>,
    pub total: u32,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpaceListQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub search: Option<String>,
    pub owner_id: Option<String>,
    pub is_public: Option<bool>,
    pub sort: Option<String>, // "name", "created_at", "updated_at"
    pub order: Option<String>, // "asc", "desc"
}

impl Default for SpaceListQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(20),
            search: None,
            owner_id: None,
            is_public: None,
            sort: Some("updated_at".to_string()),
            order: Some("desc".to_string()),
        }
    }
}

// 正则表达式验证
lazy_static::lazy_static! {
    static ref SLUG_REGEX: regex::Regex = regex::Regex::new(r"^[a-z0-9-]+$").unwrap();
}

impl Space {
    pub fn new(name: String, slug: String, owner_id: String) -> Self {
        Self {
            id: None,
            name,
            slug,
            description: None,
            avatar_url: None,
            is_public: false,
            owner_id,
            settings: SpaceSettings::default(),
            created_at: None,
            updated_at: None,
        }
    }

    pub fn is_owner(&self, user_id: &str) -> bool {
        self.owner_id == user_id
    }

    pub fn can_access(&self, user_id: Option<&str>) -> bool {
        if self.is_public {
            return true;
        }

        match user_id {
            Some(uid) => self.is_owner(uid),
            None => false,
        }
    }
}

impl From<Space> for SpaceResponse {
    fn from(space: Space) -> Self {
        Self {
            id: space.id.unwrap_or_default(),
            name: space.name,
            slug: space.slug,
            description: space.description,
            avatar_url: space.avatar_url,
            is_public: space.is_public,
            owner_id: space.owner_id,
            settings: space.settings,
            created_at: space.created_at.unwrap_or_else(Utc::now),
            updated_at: space.updated_at.unwrap_or_else(Utc::now),
            stats: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_space_creation() {
        let space = Space::new(
            "Test Space".to_string(),
            "test-space".to_string(),
            "user_123".to_string(),
        );

        assert_eq!(space.name, "Test Space");
        assert_eq!(space.slug, "test-space");
        assert_eq!(space.owner_id, "user_123");
        assert!(!space.is_public);
    }

    #[test]
    fn test_space_access_control() {
        let mut space = Space::new(
            "Test Space".to_string(),
            "test-space".to_string(),
            "user_123".to_string(),
        );

        // Private space - only owner can access
        assert!(space.can_access(Some("user_123")));
        assert!(!space.can_access(Some("user_456")));
        assert!(!space.can_access(None));

        // Public space - anyone can access
        space.is_public = true;
        assert!(space.can_access(Some("user_123")));
        assert!(space.can_access(Some("user_456")));
        assert!(space.can_access(None));
    }

    #[test]
    fn test_slug_validation() {
        let valid_slugs = vec!["test", "test-123", "my-awesome-space"];
        let invalid_slugs = vec!["Test", "test_123", "test space", "test@123"];

        for slug in valid_slugs {
            assert!(SLUG_REGEX.is_match(slug), "Should be valid: {}", slug);
        }

        for slug in invalid_slugs {
            assert!(!SLUG_REGEX.is_match(slug), "Should be invalid: {}", slug);
        }
    }
}