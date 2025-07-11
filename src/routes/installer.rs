#[cfg(feature = "installer")]
use axum::{
    extract::Query,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use crate::utils::installer::{InstallationChecker, wizard::{InstallationWizard, InstallConfig}};

#[derive(Debug, Deserialize)]
pub struct InstallQuery {
    step: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub status: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[cfg(feature = "installer")]
pub fn installer_routes() -> Router {
    Router::new()
        .route("/status", get(check_install_status))
        .route("/steps", get(get_install_steps))
        .route("/install", post(perform_install))
}

#[cfg(not(feature = "installer"))]
pub fn installer_routes() -> Router {
    Router::new()
}

#[cfg(feature = "installer")]
async fn check_install_status() -> Result<Json<InstallResponse>, StatusCode> {
    match InstallationChecker::check_installation_status() {
        Ok(status) => Ok(Json(InstallResponse {
            status: "success".to_string(),
            message: "Installation status retrieved".to_string(),
            data: Some(serde_json::to_value(status).unwrap()),
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "installer")]
async fn get_install_steps(Query(query): Query<InstallQuery>) -> Json<InstallResponse> {
    let steps = InstallationWizard::get_steps();
    
    let data = if let Some(step_num) = query.step {
        // 返回特定步骤
        steps.into_iter()
            .find(|s| s.step == step_num)
            .map(|s| serde_json::to_value(s).unwrap())
    } else {
        // 返回所有步骤
        Some(serde_json::to_value(steps).unwrap())
    };
    
    Json(InstallResponse {
        status: "success".to_string(),
        message: "Installation steps retrieved".to_string(),
        data,
    })
}

#[cfg(feature = "installer")]
async fn perform_install(Json(config): Json<InstallConfig>) -> Result<Json<InstallResponse>, StatusCode> {
    // 检查是否已安装
    match InstallationChecker::check_installation_status() {
        Ok(status) if status.is_installed => {
            return Ok(Json(InstallResponse {
                status: "error".to_string(),
                message: "System is already installed".to_string(),
                data: None,
            }));
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        _ => {}
    }
    
    // 执行安装
    match InstallationWizard::perform_installation(config).await {
        Ok(_) => Ok(Json(InstallResponse {
            status: "success".to_string(),
            message: "Installation completed successfully".to_string(),
            data: None,
        })),
        Err(e) => Ok(Json(InstallResponse {
            status: "error".to_string(),
            message: format!("Installation failed: {}", e),
            data: None,
        })),
    }
}