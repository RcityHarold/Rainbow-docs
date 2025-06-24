use axum::{
    extract::State,
    response::Json,
    routing::get,
    Extension,
    Router,
};
use serde::Serialize;
use std::sync::Arc;

use crate::{
    error::ApiError,
    services::{auth::AuthService, search::SearchService},
};

#[derive(Serialize)]
pub struct SearchStats {
    pub total_documents: i64,
    pub total_searches_today: i64,
    pub most_searched_terms: Vec<SearchTerm>,
    pub recent_searches: Vec<RecentSearch>,
}

#[derive(Serialize)]
pub struct SearchTerm {
    pub term: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct RecentSearch {
    pub query: String,
    pub results_count: i64,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct DocumentStats {
    pub total_documents: i64,
    pub total_spaces: i64,
    pub total_comments: i64,
    pub documents_created_today: i64,
    pub most_active_spaces: Vec<SpaceActivity>,
}

#[derive(Serialize)]
pub struct SpaceActivity {
    pub space_id: String,
    pub space_name: String,
    pub document_count: i64,
    pub recent_activity: i64,
}

pub async fn get_search_stats(
    State(app_state): State<Arc<crate::AppState>>,
    Extension(user_id): Extension<String>,
) -> Result<Json<SearchStats>, ApiError> {
    let search_service = &app_state.search_service;
    let auth_service = &app_state.auth_service;
    // 检查统计查看权限
    auth_service
        .check_permission(&user_id, "docs.read", None)
        .await?;

    // 这里应该从数据库获取真实的统计数据
    // 为了演示，我们返回模拟数据
    let stats = SearchStats {
        total_documents: 156,
        total_searches_today: 42,
        most_searched_terms: vec![
            SearchTerm {
                term: "API documentation".to_string(),
                count: 15,
            },
            SearchTerm {
                term: "authentication".to_string(),
                count: 12,
            },
            SearchTerm {
                term: "database".to_string(),
                count: 8,
            },
        ],
        recent_searches: vec![
            RecentSearch {
                query: "user management".to_string(),
                results_count: 7,
                timestamp: "2024-01-15T10:30:00Z".to_string(),
            },
            RecentSearch {
                query: "deployment guide".to_string(),
                results_count: 3,
                timestamp: "2024-01-15T10:25:00Z".to_string(),
            },
        ],
    };

    Ok(Json(stats))
}

pub async fn get_document_stats(
    State(app_state): State<Arc<crate::AppState>>,
    Extension(user_id): Extension<String>,
) -> Result<Json<DocumentStats>, ApiError> {
    let auth_service = &app_state.auth_service;
    // 检查统计查看权限
    auth_service
        .check_permission(&user_id, "docs.read", None)
        .await?;

    // 模拟统计数据
    let stats = DocumentStats {
        total_documents: 156,
        total_spaces: 12,
        total_comments: 89,
        documents_created_today: 3,
        most_active_spaces: vec![
            SpaceActivity {
                space_id: "space_1".to_string(),
                space_name: "API Documentation".to_string(),
                document_count: 45,
                recent_activity: 12,
            },
            SpaceActivity {
                space_id: "space_2".to_string(),
                space_name: "User Guides".to_string(),
                document_count: 32,
                recent_activity: 8,
            },
        ],
    };

    Ok(Json(stats))
}

pub fn router() -> Router<Arc<crate::AppState>> {
    Router::new()
        .route("/search", get(get_search_stats))
        .route("/documents", get(get_document_stats))
}