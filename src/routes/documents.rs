use crate::{AppState, error::{AppError, Result}};
use crate::models::document::{CreateDocumentRequest, UpdateDocumentRequest, DocumentQuery};
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

pub fn router() -> Router<Arc<crate::AppState>> {
    Router::new()
        .route("/:space_slug", get(list_documents).post(create_document))
        .route("/:space_slug/:doc_slug", get(get_document).put(update_document).delete(delete_document))
        .route("/:space_slug/tree", get(get_document_tree))
        .route("/:space_slug/:doc_slug/children", get(get_document_children))
        .route("/:space_slug/:doc_slug/breadcrumbs", get(get_document_breadcrumbs))
}

/// 获取文档列表
/// GET /api/docs/:space_slug
async fn list_documents(
    State(app_state): State<Arc<AppState>>,
    Path(space_slug): Path<String>,
    Query(query): Query<DocumentQuery>,
    OptionalUser(user): OptionalUser,
) -> Result<Json<Value>> {
    let result = app_state.document_service.list_documents(&space_slug, query, user.as_ref()).await?;

    Ok(Json(json!({
        "success": true,
        "data": result,
        "message": "Documents retrieved successfully"
    })))
}

/// 创建新文档
/// POST /api/docs/:space_slug
async fn create_document(
    State(app_state): State<Arc<AppState>>,
    Path(space_slug): Path<String>,
    user: User,
    Json(request): Json<CreateDocumentRequest>,
) -> Result<Json<Value>> {
    // 检查用户是否有文档写入权限
    if !user.permissions.contains(&"docs.write".to_string()) && !user.permissions.contains(&"docs.admin".to_string()) {
        return Err(AppError::Authorization("Permission denied: docs.write required".to_string()));
    }
    let result = app_state.document_service.create_document(&space_slug, &user.id, request).await?;

    info!("User {} created document: {} in space: {}", user.id, result.slug, space_slug);

    Ok(Json(json!({
        "success": true,
        "data": result,
        "message": "Document created successfully"
    })))
}

/// 获取文档详情
/// GET /api/docs/:space_slug/:doc_slug
async fn get_document(
    State(app_state): State<Arc<AppState>>,
    Path((space_slug, doc_slug)): Path<(String, String)>,
    OptionalUser(user): OptionalUser,
) -> Result<Json<Value>> {
    let space_service = &app_state.space_service;
    let document_service = &app_state.document_service;
    let auth_service = &app_state.auth_service;
    
    // 根据slug获取space
    let space = space_service.get_space_by_slug(&space_slug, user.as_ref()).await?;
    
    // 检查读取权限
    if let Some(user) = &user {
        auth_service
            .check_permission(&user.id, "docs.read", Some(&space.id))
            .await?;
    }
    
    // 根据slug获取document
    let document = document_service.get_document_by_slug(&space.id, &doc_slug).await?;

    Ok(Json(json!({
        "success": true,
        "data": document,
        "message": "Document retrieved successfully"
    })))
}

/// 更新文档
/// PUT /api/docs/:space_slug/:doc_slug
async fn update_document(
    State(app_state): State<Arc<AppState>>,
    Path((space_slug, doc_slug)): Path<(String, String)>,
    user: User,
    Json(request): Json<UpdateDocumentRequest>,
) -> Result<Json<Value>> {
    let space_service = &app_state.space_service;
    let document_service = &app_state.document_service;
    let auth_service = &app_state.auth_service;
    
    // 根据slug获取space
    let space = space_service.get_space_by_slug(&space_slug, Some(&user)).await?;
    
    // 检查写入权限
    auth_service
        .check_permission(&user.id, "docs.write", Some(&space.id))
        .await?;
    
    // 根据slug获取document
    let document = document_service.get_document_by_slug(&space.id, &doc_slug).await?;
    
    // 更新文档
    let document_id = document.id.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Document ID is missing"))
    })?;
    let updated_document = document_service.update_document(document_id, &user.id, request).await?;

    info!("User {} updated document: {} in space: {}", user.id, doc_slug, space_slug);

    Ok(Json(json!({
        "success": true,
        "data": updated_document,
        "message": "Document updated successfully"
    })))
}

/// 删除文档
/// DELETE /api/docs/:space_slug/:doc_slug
async fn delete_document(
    State(app_state): State<Arc<AppState>>,
    Path((space_slug, doc_slug)): Path<(String, String)>,
    user: User,
) -> Result<Json<Value>> {
    let space_service = &app_state.space_service;
    let document_service = &app_state.document_service;
    let auth_service = &app_state.auth_service;
    
    // 根据slug获取space
    let space = space_service.get_space_by_slug(&space_slug, Some(&user)).await?;
    
    // 检查删除权限
    auth_service
        .check_permission(&user.id, "docs.delete", Some(&space.id))
        .await?;
    
    // 根据slug获取document
    let document = document_service.get_document_by_slug(&space.id, &doc_slug).await?;
    
    // 删除文档
    let document_id = document.id.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Document ID is missing"))
    })?;
    document_service.delete_document(document_id, &user.id).await?;

    info!("User {} deleted document: {} in space: {}", user.id, doc_slug, space_slug);

    Ok(Json(json!({
        "success": true,
        "data": null,
        "message": "Document deleted successfully"
    })))
}

/// 获取文档树结构
/// GET /api/docs/:space_slug/tree
async fn get_document_tree(
    State(app_state): State<Arc<AppState>>,
    Path(space_slug): Path<String>,
    OptionalUser(user): OptionalUser,
) -> Result<Json<Value>> {
    let space_service = &app_state.space_service;
    let document_service = &app_state.document_service;
    let auth_service = &app_state.auth_service;
    
    // 根据slug获取space
    let space = space_service.get_space_by_slug(&space_slug, user.as_ref()).await?;
    
    // 检查读取权限
    if let Some(user) = &user {
        auth_service
            .check_permission(&user.id, "docs.read", Some(&space.id))
            .await?;
    }
    
    // 获取文档树结构
    let tree = document_service.get_document_tree(&space.id).await?;

    Ok(Json(json!({
        "success": true,
        "data": tree,
        "message": "Document tree retrieved successfully"
    })))
}

/// 获取文档子级
/// GET /api/docs/:space_slug/:doc_slug/children
async fn get_document_children(
    State(app_state): State<Arc<AppState>>,
    Path((space_slug, doc_slug)): Path<(String, String)>,
    OptionalUser(user): OptionalUser,
) -> Result<Json<Value>> {
    let space_service = &app_state.space_service;
    let document_service = &app_state.document_service;
    let auth_service = &app_state.auth_service;
    
    // 根据slug获取space
    let space = space_service.get_space_by_slug(&space_slug, user.as_ref()).await?;
    
    // 检查读取权限
    if let Some(user) = &user {
        auth_service
            .check_permission(&user.id, "docs.read", Some(&space.id))
            .await?;
    }
    
    // 根据slug获取document
    let document = document_service.get_document_by_slug(&space.id, &doc_slug).await?;
    
    // 获取文档子级
    let document_id = document.id.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Document ID is missing"))
    })?;
    let children = document_service.get_document_children(document_id).await?;

    Ok(Json(json!({
        "success": true,
        "data": children,
        "message": "Document children retrieved successfully"
    })))
}

/// 获取文档面包屑导航
/// GET /api/docs/:space_slug/:doc_slug/breadcrumbs
async fn get_document_breadcrumbs(
    State(app_state): State<Arc<AppState>>,
    Path((space_slug, doc_slug)): Path<(String, String)>,
    OptionalUser(user): OptionalUser,
) -> Result<Json<Value>> {
    let space_service = &app_state.space_service;
    let document_service = &app_state.document_service;
    let auth_service = &app_state.auth_service;
    
    // 根据slug获取space
    let space = space_service.get_space_by_slug(&space_slug, user.as_ref()).await?;
    
    // 检查读取权限
    if let Some(user) = &user {
        auth_service
            .check_permission(&user.id, "docs.read", Some(&space.id))
            .await?;
    }
    
    // 根据slug获取document
    let document = document_service.get_document_by_slug(&space.id, &doc_slug).await?;
    
    // 获取文档面包屑
    let document_id = document.id.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Document ID is missing"))
    })?;
    let breadcrumbs = document_service.get_document_breadcrumbs(document_id).await?;

    Ok(Json(json!({
        "success": true,
        "data": breadcrumbs,
        "message": "Document breadcrumbs retrieved successfully"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::document::CreateDocumentRequest;
    use axum_test::TestServer;

    async fn create_test_server() -> TestServer {
        let app = Router::new()
            .nest("/api/docs", router());
        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_create_document_validation() {
        let request = CreateDocumentRequest {
            title: "".to_string(), // 无效：空标题
            slug: "test-doc".to_string(),
            content: None,
            excerpt: None,
            is_published: None,
            parent_id: None,
            sort_order: None,
            metadata: None,
        };

        assert!(request.validate().is_err());
    }

    #[tokio::test]
    async fn test_document_slug_validation() {
        let valid_request = CreateDocumentRequest {
            title: "Test Document".to_string(),
            slug: "test-document".to_string(),
            content: Some("# Test Content".to_string()),
            excerpt: None,
            is_published: Some(true),
            parent_id: None,
            sort_order: Some(1),
            metadata: None,
        };

        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateDocumentRequest {
            title: "Test Document".to_string(),
            slug: "Test Document".to_string(), // 无效：包含空格和大写
            content: None,
            excerpt: None,
            is_published: None,
            parent_id: None,
            sort_order: None,
            metadata: None,
        };

        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_title_length_validation() {
        let long_title = "x".repeat(201); // 超过200字符限制
        
        let request = CreateDocumentRequest {
            title: long_title,
            slug: "test-doc".to_string(),
            content: None,
            excerpt: None,
            is_published: None,
            parent_id: None,
            sort_order: None,
            metadata: None,
        };

        assert!(request.validate().is_err());
    }
}