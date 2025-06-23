use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    error::ApiError,
    models::file::{FileQuery, UploadFileRequest},
    services::{file_upload::FileUploadService, auth::AuthService},
    utils::auth::extract_user_from_header,
};

pub fn router() -> Router<Arc<crate::AppState>> {
    Router::new()
        .route("/", get(list_files).post(upload_file))
        .route("/:file_id", get(get_file_info).delete(delete_file))
        .route("/:file_id/download", get(download_file))
        .route("/:file_id/thumbnail", get(get_thumbnail))
}

async fn upload_file(
    State(service): State<Arc<FileUploadService>>,
    State(auth_service): State<Arc<AuthService>>,
    headers: axum::http::HeaderMap,
    multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_from_header(&headers, &auth_service).await?;
    
    // 从 multipart 中提取请求参数
    let request = UploadFileRequest {
        space_id: None, // TODO: 从 multipart 中提取
        document_id: None, // TODO: 从 multipart 中提取
        description: None,
    };

    let file_response = service.upload_file(&user_id, multipart, request).await?;
    
    info!("File uploaded by user {}", user_id);
    Ok((StatusCode::CREATED, Json(file_response)))
}

async fn list_files(
    State(service): State<Arc<FileUploadService>>,
    State(auth_service): State<Arc<AuthService>>,
    headers: axum::http::HeaderMap,
    Query(query): Query<FileQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_from_header(&headers, &auth_service).await?;
    
    let files = service.list_files(&user_id, query).await?;
    Ok(Json(files))
}

async fn get_file_info(
    State(service): State<Arc<FileUploadService>>,
    State(auth_service): State<Arc<AuthService>>,
    headers: axum::http::HeaderMap,
    Path(file_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let _user_id = extract_user_from_header(&headers, &auth_service).await?;
    
    let file = service.get_file(&file_id).await?;
    let file_response = file.into();
    Ok(Json(file_response))
}

async fn download_file(
    State(service): State<Arc<FileUploadService>>,
    State(auth_service): State<Arc<AuthService>>,
    headers: axum::http::HeaderMap,
    Path(file_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let _user_id = extract_user_from_header(&headers, &auth_service).await?;
    
    let (content, mime_type, original_name) = service.get_file_content(&file_id).await?;
    
    let headers = [
        (header::CONTENT_TYPE, mime_type),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", original_name),
        ),
    ];

    Ok((headers, content))
}

async fn get_thumbnail(
    State(service): State<Arc<FileUploadService>>,
    State(auth_service): State<Arc<AuthService>>,
    headers: axum::http::HeaderMap,
    Path(file_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let _user_id = extract_user_from_header(&headers, &auth_service).await?;
    
    let thumbnail_content = service.get_thumbnail(&file_id).await?;
    
    let headers = [
        (header::CONTENT_TYPE, "image/jpeg".to_string()),
        (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
    ];

    Ok((headers, thumbnail_content))
}

async fn delete_file(
    State(service): State<Arc<FileUploadService>>,
    State(auth_service): State<Arc<AuthService>>,
    headers: axum::http::HeaderMap,
    Path(file_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = extract_user_from_header(&headers, &auth_service).await?;
    
    service.delete_file(&user_id, &file_id).await?;
    
    info!("File {} deleted by user {}", file_id, user_id);
    Ok((
        StatusCode::OK,
        Json(json!({ "message": "File deleted successfully" })),
    ))
}