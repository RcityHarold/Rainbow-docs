use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Option<Thing>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub color: String,
    pub space_id: Option<Thing>,
    pub created_by: String,
    pub created_at: Datetime,
    pub usage_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTag {
    pub id: Option<Thing>,
    pub document_id: Thing,
    pub tag_id: Thing,
    pub tagged_by: String,
    pub tagged_at: Datetime,
}

#[derive(Debug, Validate, Deserialize)]
pub struct CreateTagRequest {
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    #[validate(length(max = 200))]
    pub description: Option<String>,
    #[validate(regex = "^#[0-9A-Fa-f]{6}$")]
    pub color: String,
    pub space_id: Option<String>,
}

#[derive(Debug, Validate, Deserialize)]
pub struct UpdateTagRequest {
    #[validate(length(min = 1, max = 50))]
    pub name: Option<String>,
    #[validate(length(max = 200))]
    pub description: Option<String>,
    #[validate(regex = "^#[0-9A-Fa-f]{6}$")]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TagDocumentRequest {
    pub document_id: String,
    pub tag_ids: Vec<String>,
}

impl Tag {
    pub fn new(name: String, color: String, created_by: String) -> Self {
        let slug = Self::generate_slug(&name);
        Self {
            id: None,
            name,
            slug,
            description: None,
            color,
            space_id: None,
            created_by,
            created_at: Datetime::default(),
            usage_count: 0,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_space(mut self, space_id: Thing) -> Self {
        self.space_id = Some(space_id);
        self
    }

    pub fn generate_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("-")
    }

    pub fn increment_usage(&mut self) {
        self.usage_count += 1;
    }

    pub fn decrement_usage(&mut self) {
        if self.usage_count > 0 {
            self.usage_count -= 1;
        }
    }
}

impl DocumentTag {
    pub fn new(document_id: Thing, tag_id: Thing, tagged_by: String) -> Self {
        Self {
            id: None,
            document_id,
            tag_id,
            tagged_by,
            tagged_at: Datetime::default(),
        }
    }
}