use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    SpaceInvitation,
    DocumentShared,
    CommentMention,
    DocumentUpdate,
    System,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationDb {
    pub id: Option<Thing>,
    pub user_id: String,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub title: String,
    pub content: String,
    pub data: Option<serde_json::Value>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Option<String>,
    pub user_id: String,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub title: String,
    pub content: String,
    pub data: Option<serde_json::Value>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateNotificationRequest {
    pub user_id: String,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub title: String,
    pub content: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateNotificationRequest {
    pub is_read: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationListQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub unread_only: Option<bool>,
}

impl From<NotificationDb> for Notification {
    fn from(db: NotificationDb) -> Self {
        Self {
            id: db.id.map(|thing| thing.id.to_string()),
            user_id: db.user_id,
            notification_type: db.notification_type,
            title: db.title,
            content: db.content,
            data: db.data,
            is_read: db.is_read,
            read_at: db.read_at,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}