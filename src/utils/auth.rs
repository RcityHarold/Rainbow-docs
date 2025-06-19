use crate::error::{AppError, Result};
use crate::services::auth::User;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashSet;

/// 权限检查中间件
pub async fn require_permission(
    permission: &str,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> + Clone {
    let permission = permission.to_string();
    move |req: Request, next: Next| {
        let permission = permission.clone();
        Box::pin(async move {
            // 从请求中提取用户信息
            let user = req.extensions().get::<User>()
                .ok_or_else(|| AppError::Authentication("User not authenticated".to_string()))?;

            // 检查权限
            if !user.permissions.contains(&permission) {
                return Err(AppError::Authorization(format!("Permission {} required", permission)));
            }

            Ok(next.run(req).await)
        })
    }
}

/// 角色检查中间件
pub async fn require_role(
    role: &str,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> + Clone {
    let role = role.to_string();
    move |req: Request, next: Next| {
        let role = role.clone();
        Box::pin(async move {
            // 从请求中提取用户信息
            let user = req.extensions().get::<User>()
                .ok_or_else(|| AppError::Authentication("User not authenticated".to_string()))?;

            // 检查角色
            if !user.roles.contains(&role) {
                return Err(AppError::Authorization(format!("Role {} required", role)));
            }

            Ok(next.run(req).await)
        })
    }
}

/// 管理员权限检查
pub fn require_admin(user: &User) -> Result<()> {
    if user.roles.contains(&"admin".to_string()) || 
       user.permissions.contains(&"docs.admin".to_string()) {
        Ok(())
    } else {
        Err(AppError::Authorization("Admin permission required".to_string()))
    }
}

/// 检查用户是否有文档读取权限
pub fn can_read_document(user: &User) -> bool {
    user.permissions.contains(&"docs.read".to_string()) ||
    user.permissions.contains(&"docs.write".to_string()) ||
    user.permissions.contains(&"docs.admin".to_string())
}

/// 检查用户是否有文档写入权限
pub fn can_write_document(user: &User) -> bool {
    user.permissions.contains(&"docs.write".to_string()) ||
    user.permissions.contains(&"docs.admin".to_string())
}

/// 检查用户是否有文档管理权限
pub fn can_admin_document(user: &User) -> bool {
    user.permissions.contains(&"docs.admin".to_string())
}

/// 检查用户是否是空间所有者或有管理权限
pub fn can_manage_space(user: &User, space_owner_id: &str) -> bool {
    user.id == space_owner_id || can_admin_document(user)
}

/// 文档权限类型
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentPermission {
    Read,
    Write,
    Admin,
}

impl DocumentPermission {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
        }
    }

    /// 检查权限是否包含另一个权限
    pub fn includes(&self, other: &DocumentPermission) -> bool {
        match (self, other) {
            (DocumentPermission::Admin, _) => true,
            (DocumentPermission::Write, DocumentPermission::Read) => true,
            (DocumentPermission::Write, DocumentPermission::Write) => true,
            (DocumentPermission::Read, DocumentPermission::Read) => true,
            _ => false,
        }
    }
}

/// 权限验证宏
#[macro_export]
macro_rules! require_permission {
    ($user:expr, $permission:expr) => {
        if !$user.permissions.contains(&$permission.to_string()) {
            return Err(AppError::Authorization(format!("Permission {} required", $permission)));
        }
    };
}

#[macro_export]
macro_rules! require_role {
    ($user:expr, $role:expr) => {
        if !$user.roles.contains(&$role.to_string()) {
            return Err(AppError::Authorization(format!("Role {} required", $role)));
        }
    };
}

#[macro_export]
macro_rules! require_admin {
    ($user:expr) => {
        if !($user.roles.contains(&"admin".to_string()) || 
             $user.permissions.contains(&"docs.admin".to_string())) {
            return Err(AppError::Authorization("Admin permission required".to_string()));
        }
    };
}

/// JWT Token 工具函数
pub mod jwt {
    use crate::config::Config;
    use crate::error::{AppError, Result};
    use crate::services::auth::Claims;
    use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};
    use chrono::{Utc, Duration};

    /// 生成 JWT Token（独立模式使用）
    pub fn generate_token(config: &Config, user_id: &str, email: &str, roles: Vec<String>, permissions: Vec<String>) -> Result<String> {
        let now = Utc::now();
        let exp = (now + Duration::seconds(config.auth.jwt_expiration as i64)).timestamp() as usize;
        let iat = now.timestamp() as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            exp,
            iat,
            roles,
            permissions,
        };

        let encoding_key = EncodingKey::from_secret(config.auth.jwt_secret.as_ref());
        let header = Header::new(Algorithm::HS256);

        encode(&header, &claims, &encoding_key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to generate token: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_permission_includes() {
        assert!(DocumentPermission::Admin.includes(&DocumentPermission::Read));
        assert!(DocumentPermission::Admin.includes(&DocumentPermission::Write));
        assert!(DocumentPermission::Admin.includes(&DocumentPermission::Admin));
        
        assert!(DocumentPermission::Write.includes(&DocumentPermission::Read));
        assert!(DocumentPermission::Write.includes(&DocumentPermission::Write));
        assert!(!DocumentPermission::Write.includes(&DocumentPermission::Admin));
        
        assert!(DocumentPermission::Read.includes(&DocumentPermission::Read));
        assert!(!DocumentPermission::Read.includes(&DocumentPermission::Write));
        assert!(!DocumentPermission::Read.includes(&DocumentPermission::Admin));
    }

    #[test]
    fn test_permission_functions() {
        let admin_user = User {
            id: "admin_1".to_string(),
            email: "admin@example.com".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["docs.admin".to_string()],
            profile: None,
        };

        let writer_user = User {
            id: "writer_1".to_string(),
            email: "writer@example.com".to_string(),
            roles: vec!["writer".to_string()],
            permissions: vec!["docs.read".to_string(), "docs.write".to_string()],
            profile: None,
        };

        let reader_user = User {
            id: "reader_1".to_string(),
            email: "reader@example.com".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["docs.read".to_string()],
            profile: None,
        };

        // Test admin permissions
        assert!(can_read_document(&admin_user));
        assert!(can_write_document(&admin_user));
        assert!(can_admin_document(&admin_user));

        // Test writer permissions
        assert!(can_read_document(&writer_user));
        assert!(can_write_document(&writer_user));
        assert!(!can_admin_document(&writer_user));

        // Test reader permissions
        assert!(can_read_document(&reader_user));
        assert!(!can_write_document(&reader_user));
        assert!(!can_admin_document(&reader_user));
    }
}