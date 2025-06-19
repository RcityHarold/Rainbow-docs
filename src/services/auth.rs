use crate::config::Config;
use crate::error::{AppError, Result};
use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    headers::{authorization::Bearer, Authorization},
    http::{request::Parts, StatusCode},
    RequestPartsExt, TypedHeader,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use tracing::{info, warn, error, debug};

#[derive(Clone)]
pub struct AuthService {
    config: Config,
    http_client: Client,
    user_cache: Arc<RwLock<HashMap<String, CachedUser>>>,
    permission_cache: Arc<RwLock<HashMap<String, CachedPermission>>>,
}

#[derive(Debug, Clone)]
struct CachedUser {
    user: User,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CachedPermission {
    has_permission: bool,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // 用户ID
    pub email: String,      // 用户邮箱
    pub exp: usize,         // 过期时间
    pub iat: usize,         // 签发时间
    pub roles: Vec<String>, // 用户角色
    pub permissions: Vec<String>, // 用户权限
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub profile: Option<UserProfile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RainbowAuthUserResponse {
    pub id: String,
    pub email: String,
    pub email_verified: bool,
    pub created_at: String,
    // 其他字段根据Rainbow-Auth实际返回调整
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RainbowAuthPermissionResponse {
    pub success: bool,
    pub data: PermissionData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionData {
    pub has_permission: bool,
    pub user_id: String,
    pub permission: String,
}

impl AuthService {
    pub fn new(config: Config) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            user_cache: Arc::new(RwLock::new(HashMap::new())),
            permission_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn verify_jwt(&self, token: &str) -> Result<Claims> {
        let decoding_key = DecodingKey::from_secret(self.config.auth.jwt_secret.as_ref());
        let validation = Validation::new(Algorithm::HS256);

        match decode::<Claims>(token, &decoding_key, &validation) {
            Ok(token_data) => {
                info!("JWT token verified for user: {}", token_data.claims.sub);
                Ok(token_data.claims)
            }
            Err(e) => {
                warn!("JWT verification failed: {}", e);
                Err(AppError::Authentication("Invalid token".to_string()))
            }
        }
    }

    pub async fn get_user_from_rainbow_auth(&self, user_id: &str, token: &str) -> Result<User> {
        if !self.config.auth.integration_mode {
            return Err(AppError::Authentication("Rainbow-Auth integration not enabled".to_string()));
        }

        // 检查缓存
        if let Some(cached_user) = self.get_cached_user(user_id).await {
            debug!("Using cached user data for user: {}", user_id);
            return Ok(cached_user);
        }

        let rainbow_auth_url = self.config.auth.rainbow_auth_url
            .as_ref()
            .ok_or_else(|| AppError::Authentication("Rainbow-Auth URL not configured".to_string()))?;

        let url = format!("{}/api/auth/me", rainbow_auth_url);
        
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch user from Rainbow-Auth: {}", e);
                AppError::Authentication("Failed to verify user with Rainbow-Auth".to_string())
            })?;

        if !response.status().is_success() {
            warn!("Rainbow-Auth returned error status: {}", response.status());
            return Err(AppError::Authentication("Invalid credentials".to_string()));
        }

        let user_data: RainbowAuthUserResponse = response.json().await
            .map_err(|e| {
                error!("Failed to parse Rainbow-Auth response: {}", e);
                AppError::Authentication("Invalid response from Rainbow-Auth".to_string())
            })?;

        // 获取用户角色和权限
        let (roles, permissions) = self.get_user_permissions(&user_data.id, token).await?;

        let user = User {
            id: user_data.id.clone(),
            email: user_data.email,
            roles,
            permissions,
            profile: None, // 可以后续扩展获取用户档案
        };

        // 缓存用户数据
        self.cache_user(&user_data.id, user.clone()).await;

        Ok(user)
    }

    async fn get_cached_user(&self, user_id: &str) -> Option<User> {
        let cache = self.user_cache.read().await;
        if let Some(cached) = cache.get(user_id) {
            if cached.expires_at > Utc::now() {
                return Some(cached.user.clone());
            }
        }
        None
    }

    async fn cache_user(&self, user_id: &str, user: User) {
        let mut cache = self.user_cache.write().await;
        cache.insert(user_id.to_string(), CachedUser {
            user,
            expires_at: Utc::now() + Duration::minutes(15), // 缓存15分钟
        });
    }

    pub async fn get_user_permissions(&self, user_id: &str, token: &str) -> Result<(Vec<String>, Vec<String>)> {
        if !self.config.auth.integration_mode {
            // 独立模式：返回默认权限
            return Ok((vec!["user".to_string()], vec!["docs.read".to_string()]));
        }

        let rainbow_auth_url = self.config.auth.rainbow_auth_url
            .as_ref()
            .ok_or_else(|| AppError::Authentication("Rainbow-Auth URL not configured".to_string()))?;

        // 获取用户角色
        let roles_url = format!("{}/api/rbac/users/{}/roles", rainbow_auth_url, user_id);
        let roles_response = self.http_client
            .get(&roles_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|_| AppError::Authentication("Failed to fetch user roles".to_string()))?;

        let roles = if roles_response.status().is_success() {
            // 解析角色响应
            vec!["user".to_string()] // 简化处理，实际应解析完整响应
        } else {
            vec!["user".to_string()]
        };

        // 获取用户权限
        let permissions_url = format!("{}/api/rbac/users/{}/permissions", rainbow_auth_url, user_id);
        let permissions_response = self.http_client
            .get(&permissions_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|_| AppError::Authentication("Failed to fetch user permissions".to_string()))?;

        let permissions = if permissions_response.status().is_success() {
            // 解析权限响应
            vec!["docs.read".to_string()] // 简化处理，实际应解析完整响应
        } else {
            vec!["docs.read".to_string()]
        };

        Ok((roles, permissions))
    }

    pub async fn check_permission(&self, user_id: &str, permission: &str, token: &str) -> Result<bool> {
        if !self.config.auth.integration_mode {
            // 独立模式：简单权限检查
            return Ok(permission == "docs.read" || permission == "docs.write");
        }

        // 检查权限缓存
        let cache_key = format!("{}:{}", user_id, permission);
        if let Some(cached_permission) = self.get_cached_permission(&cache_key).await {
            debug!("Using cached permission for {}: {}", cache_key, cached_permission);
            return Ok(cached_permission);
        }

        let rainbow_auth_url = self.config.auth.rainbow_auth_url
            .as_ref()
            .ok_or_else(|| AppError::Authentication("Rainbow-Auth URL not configured".to_string()))?;

        let url = format!("{}/api/rbac/check/permission/{}", rainbow_auth_url, permission);
        
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to check permission with Rainbow-Auth: {}", e);
                AppError::Authorization("Failed to verify permission".to_string())
            })?;

        if !response.status().is_success() {
            // 缓存失败结果，避免重复请求
            self.cache_permission(&cache_key, false).await;
            return Ok(false);
        }

        let permission_data: RainbowAuthPermissionResponse = response.json().await
            .map_err(|e| {
                error!("Failed to parse permission response: {}", e);
                AppError::Authorization("Invalid permission response".to_string())
            })?;

        let has_permission = permission_data.data.has_permission;
        
        // 缓存权限结果
        self.cache_permission(&cache_key, has_permission).await;

        Ok(has_permission)
    }

    async fn get_cached_permission(&self, cache_key: &str) -> Option<bool> {
        let cache = self.permission_cache.read().await;
        if let Some(cached) = cache.get(cache_key) {
            if cached.expires_at > Utc::now() {
                return Some(cached.has_permission);
            }
        }
        None
    }

    async fn cache_permission(&self, cache_key: &str, has_permission: bool) {
        let mut cache = self.permission_cache.write().await;
        cache.insert(cache_key.to_string(), CachedPermission {
            has_permission,
            expires_at: Utc::now() + Duration::minutes(10), // 权限缓存10分钟
        });
    }

    // 批量权限检查
    pub async fn check_multiple_permissions(&self, user_id: &str, permissions: &[&str], token: &str) -> Result<HashMap<String, bool>> {
        let mut results = HashMap::new();
        
        for permission in permissions {
            let has_permission = self.check_permission(user_id, permission, token).await?;
            results.insert(permission.to_string(), has_permission);
        }
        
        Ok(results)
    }

    // 清理过期缓存
    pub async fn cleanup_cache(&self) {
        let now = Utc::now();
        
        // 清理用户缓存
        {
            let mut user_cache = self.user_cache.write().await;
            user_cache.retain(|_, cached| cached.expires_at > now);
        }
        
        // 清理权限缓存  
        {
            let mut permission_cache = self.permission_cache.write().await;
            permission_cache.retain(|_, cached| cached.expires_at > now);
        }
        
        debug!("Cache cleanup completed");
    }
}

// Axum extractor for authentication
#[async_trait]
impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        // Extract the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::Authentication("Missing authorization header".to_string()))?;

        // Get auth service from app state
        let Extension(auth_service): Extension<Arc<AuthService>> = parts
            .extract::<Extension<Arc<AuthService>>>()
            .await
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Auth service not found")))?;

        // Verify JWT token
        let claims = auth_service.verify_jwt(bearer.token())?;

        // Get user details from Rainbow-Auth if integration is enabled
        if auth_service.config.auth.integration_mode {
            auth_service.get_user_from_rainbow_auth(&claims.sub, bearer.token()).await
        } else {
            // Standalone mode: create user from JWT claims
            Ok(User {
                id: claims.sub,
                email: claims.email,
                roles: claims.roles,
                permissions: claims.permissions,
                profile: None,
            })
        }
    }
}

// Optional authentication extractor
pub struct OptionalUser(pub Option<User>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        match User::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalUser(Some(user))),
            Err(_) => Ok(OptionalUser(None)),
        }
    }
}