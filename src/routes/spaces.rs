use crate::{AppState, error::{AppError, Result}};
use crate::models::space::{CreateSpaceRequest, UpdateSpaceRequest, SpaceListQuery};
use crate::services::auth::{User, OptionalUser};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{info, warn};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_spaces).post(create_space))
        .route("/create", post(handle_legacy_create)) // Legacy frontend support
        .route("/create/stats", get(handle_legacy_create_stats)) // Legacy frontend support
        .route("/:slug", get(get_space).put(update_space).delete(delete_space))
        .route("/:slug/stats", get(get_space_stats))
}

/// 获取空间列表
/// GET /api/spaces
async fn list_spaces(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<SpaceListQuery>,
    OptionalUser(user): OptionalUser,
) -> Result<Json<Value>> {
    let result = app_state.space_service.list_spaces(query, user.as_ref()).await?;

    Ok(Json(json!({
        "success": true,
        "data": result,
        "message": "Spaces retrieved successfully"
    })))
}

/// 创建新空间
/// POST /api/spaces
async fn create_space(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(request): Json<CreateSpaceRequest>,
) -> Result<Json<Value>> {
    // 检查用户是否有创建空间的权限
    if !user.permissions.contains(&"spaces.write".to_string()) && !user.permissions.contains(&"docs.admin".to_string()) {
        return Err(AppError::Authorization("Permission denied: spaces.write required".to_string()));
    }
    let result = app_state.space_service.create_space(request, &user).await?;

    info!("User {} created space: {}", user.id, result.slug);

    Ok(Json(json!({
        "success": true,
        "data": result,
        "message": "Space created successfully"
    })))
}

/// 获取空间详情
/// GET /api/spaces/:slug
async fn get_space(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    OptionalUser(user): OptionalUser,
) -> Result<Json<Value>> {
    let result = app_state.space_service.get_space_by_slug(&slug, user.as_ref()).await?;

    Ok(Json(json!({
        "success": true,
        "data": result,
        "message": "Space retrieved successfully"
    })))
}

/// 更新空间信息
/// PUT /api/spaces/:slug
async fn update_space(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    user: User,
    Json(request): Json<UpdateSpaceRequest>,
) -> Result<Json<Value>> {
    let result = app_state.space_service.update_space(&slug, request, &user).await?;

    info!("User {} updated space: {}", user.id, slug);

    Ok(Json(json!({
        "success": true,
        "data": result,
        "message": "Space updated successfully"
    })))
}

/// 删除空间
/// DELETE /api/spaces/:slug
async fn delete_space(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    user: User,
) -> Result<Json<Value>> {
    app_state.space_service.delete_space(&slug, &user).await?;

    info!("User {} deleted space: {}", user.id, slug);

    Ok(Json(json!({
        "success": true,
        "data": null,
        "message": "Space deleted successfully"
    })))
}

/// 获取空间统计信息
/// GET /api/spaces/:slug/stats
async fn get_space_stats(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    OptionalUser(user): OptionalUser,
) -> Result<Json<Value>> {
    // 首先检查用户是否有访问空间的权限
    let space = app_state.space_service.get_space_by_slug(&slug, user.as_ref()).await?;
    
    // 统计信息已经包含在空间响应中
    let stats = space.stats.unwrap_or_default();

    Ok(Json(json!({
        "success": true,
        "data": stats,
        "message": "Space statistics retrieved successfully"
    })))
}

/// Legacy handler for frontend calls to /create (should use POST /)
async fn handle_legacy_create(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(request): Json<CreateSpaceRequest>,
) -> Result<Json<Value>> {
    // Forward to the correct create_space handler
    create_space(State(app_state), user, Json(request)).await
}

/// Legacy handler for frontend calls to /create/stats
async fn handle_legacy_create_stats(
    State(_app_state): State<Arc<AppState>>,
    OptionalUser(_user): OptionalUser,
) -> Result<Json<Value>> {
    Err(AppError::BadRequest(
        "Invalid endpoint. Please use '/api/docs/spaces/{slug}/stats' instead.".to_string()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::space::CreateSpaceRequest;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    // 注意：这些测试需要实际的数据库连接
    // 在实际项目中，应该使用测试数据库或模拟

    async fn create_test_server() -> TestServer {
        let app = Router::new()
            .nest("/api/spaces", router());
        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_create_space_validation() {
        let request = CreateSpaceRequest {
            name: "".to_string(), // 无效：空名称
            slug: "test-space".to_string(),
            description: None,
            avatar_url: None,
            is_public: None,
            settings: None,
        };

        // 验证应该失败
        assert!(request.validate().is_err());
    }

    #[tokio::test]
    async fn test_slug_format() {
        let valid_slugs = vec!["test", "test-space", "my-docs-123"];
        let invalid_slugs = vec!["Test", "test_space", "test space", "test@space"];

        for slug in valid_slugs {
            let request = CreateSpaceRequest {
                name: "Test Space".to_string(),
                slug: slug.to_string(),
                description: None,
                avatar_url: None,
                is_public: None,
                settings: None,
            };
            assert!(request.validate().is_ok(), "Should be valid: {}", slug);
        }

        for slug in invalid_slugs {
            let request = CreateSpaceRequest {
                name: "Test Space".to_string(),
                slug: slug.to_string(),
                description: None,
                avatar_url: None,
                is_public: None,
                settings: None,
            };
            assert!(request.validate().is_err(), "Should be invalid: {}", slug);
        }
    }
}