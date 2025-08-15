use crate::config::Config;
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use soulcore::prelude::*;
use surrealdb::{Response};
use tracing::{info, error};
use surrealdb::sql::Thing;

/// 客户端包装器，提供完全兼容的 SurrealDB API
#[derive(Clone)]
pub struct ClientWrapper {
    storage: Arc<soulcore::engines::storage::StorageEngine>,
}

impl ClientWrapper {
    /// 执行原始SQL查询
    pub fn query(&self, sql: impl Into<String>) -> QueryBuilder {
        QueryBuilder::new(self.storage.clone(), sql.into())
    }
    
    /// Select - 兼容原有API，直接返回Future
    pub async fn select<T>(&self, resource: impl Into<ResourceId>) -> std::result::Result<T, surrealdb::Error>
    where
        T: for<'de> serde::Deserialize<'de> + Serialize + std::fmt::Debug,
    {
        let resource_id = resource.into();
        match resource_id {
            ResourceId::Record(table, id) => {
                // 单条记录查询，返回 Option<T>
                let result: Option<T> = self.storage.select(&format!("{}:{}", table, id))
                    .await
                    .map_err(|e| surrealdb::Error::Api(surrealdb::error::Api::Query(e.to_string())))?
                    .into_iter()
                    .next();
                
                // 如果T是Option<U>，直接返回；否则尝试转换
                serde_json::to_value(result)
                    .and_then(serde_json::from_value)
                    .map_err(|e| surrealdb::Error::Api(surrealdb::error::Api::Query(e.to_string())))
            }
            ResourceId::Table(table) => {
                // 表查询，返回 Vec<T>
                let result: Vec<serde_json::Value> = self.storage.select(&table)
                    .await
                    .map_err(|e| surrealdb::Error::Api(surrealdb::error::Api::Query(e.to_string())))?;
                
                serde_json::to_value(result)
                    .and_then(serde_json::from_value)
                    .map_err(|e| surrealdb::Error::Api(surrealdb::error::Api::Query(e.to_string())))
            }
        }
    }
    
    /// Create - 返回兼容的builder
    pub fn create(&self, resource: impl Into<String>) -> CreateWrapper {
        CreateWrapper::new(self.storage.clone(), resource.into())
    }
    
    /// Update - 返回兼容的builder  
    pub fn update(&self, resource: impl Into<ResourceId>) -> UpdateWrapper {
        UpdateWrapper::new(self.storage.clone(), resource.into())
    }
    
    /// Delete - 直接执行
    pub async fn delete<T>(&self, resource: impl Into<ResourceId>) -> std::result::Result<Option<T>, surrealdb::Error>
    where
        T: for<'de> serde::Deserialize<'de> + std::fmt::Debug,
    {
        let resource_id = resource.into();
        let thing = match resource_id {
            ResourceId::Record(table, id) => {
                Thing::from((table.as_str(), surrealdb::sql::Id::String(id)))
            }
            _ => {
                return Err(surrealdb::Error::Api(surrealdb::error::Api::Query(
                    "Delete requires a specific record ID".to_string()
                )));
            }
        };
        
        self.storage.delete(thing)
            .await
            .map_err(|e| surrealdb::Error::Api(surrealdb::error::Api::Query(e.to_string())))
    }
}

/// 资源标识符
pub enum ResourceId {
    Table(String),
    Record(String, String),
}

impl From<&str> for ResourceId {
    fn from(s: &str) -> Self {
        ResourceId::Table(s.to_string())
    }
}

impl From<String> for ResourceId {
    fn from(s: String) -> Self {
        ResourceId::Table(s)
    }
}

impl From<(&str, &str)> for ResourceId {
    fn from((table, id): (&str, &str)) -> Self {
        ResourceId::Record(table.to_string(), id.to_string())
    }
}

impl From<(String, String)> for ResourceId {
    fn from((table, id): (String, String)) -> Self {
        ResourceId::Record(table, id)
    }
}

impl From<(&str, String)> for ResourceId {
    fn from((table, id): (&str, String)) -> Self {
        ResourceId::Record(table.to_string(), id)
    }
}

impl From<(&str, surrealdb::sql::Id)> for ResourceId {
    fn from((table, id): (&str, surrealdb::sql::Id)) -> Self {
        ResourceId::Record(table.to_string(), id.to_string())
    }
}

/// Query构建器 - 兼容原有的链式调用
pub struct QueryBuilder {
    storage: Arc<soulcore::engines::storage::StorageEngine>,
    sql: String,
    bindings: Vec<(String, serde_json::Value)>,
}

impl QueryBuilder {
    fn new(storage: Arc<soulcore::engines::storage::StorageEngine>, sql: String) -> Self {
        Self {
            storage,
            sql,
            bindings: Vec::new(),
        }
    }
    
    // 灵活的bind方法，可以接受多种类型
    pub fn bind<B>(mut self, binding: B) -> Self 
    where
        B: IntoBinding,
    {
        binding.add_to(&mut self.bindings);
        self
    }
}

// Trait用于各种类型转换为绑定
pub trait IntoBinding {
    fn add_to(self, bindings: &mut Vec<(String, serde_json::Value)>);
}

// 实现元组绑定
impl<K, V> IntoBinding for (K, V)
where
    K: Into<String>,
    V: Serialize,
{
    fn add_to(self, bindings: &mut Vec<(String, serde_json::Value)>) {
        let json_value = serde_json::to_value(self.1).unwrap_or(serde_json::Value::Null);
        bindings.push((self.0.into(), json_value));
    }
}

// 实现HashMap绑定
impl IntoBinding for std::collections::HashMap<String, serde_json::Value> {
    fn add_to(self, bindings: &mut Vec<(String, serde_json::Value)>) {
        for (key, value) in self {
            bindings.push((key, value));
        }
    }
}

// 实现引用的HashMap绑定
impl IntoBinding for &std::collections::HashMap<String, serde_json::Value> {
    fn add_to(self, bindings: &mut Vec<(String, serde_json::Value)>) {
        for (key, value) in self {
            bindings.push((key.clone(), value.clone()));
        }
    }
}


impl std::future::IntoFuture for QueryBuilder {
    type Output = std::result::Result<Response, surrealdb::Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;
    
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let mut params = serde_json::Map::new();
            for (key, value) in self.bindings {
                params.insert(key, value);
            }
            
            self.storage.query_with_params(&self.sql, params)
                .await
                .map_err(|e| surrealdb::Error::Api(surrealdb::error::Api::Query(e.to_string())))
        })
    }
}

/// Create包装器 - 类型擦除版本
pub struct CreateWrapper {
    storage: Arc<soulcore::engines::storage::StorageEngine>,
    table: String,
}

impl CreateWrapper {
    fn new(storage: Arc<soulcore::engines::storage::StorageEngine>, table: String) -> Self {
        Self { storage, table }
    }
    
    /// content方法，能够处理引用和值
    pub fn content<T>(self, content: T) -> TypedCreateFuture<T>
    where
        T: Serialize + for<'de> serde::Deserialize<'de> + Clone + std::fmt::Debug + Send + 'static,
    {
        TypedCreateFuture {
            storage: self.storage,
            table: self.table,
            content,
        }
    }
}

/// Trait来处理content的各种输入类型
pub trait ContentProvider<T> {
    fn provide(self) -> T;
}

// 实现值类型
impl<T> ContentProvider<T> for T {
    fn provide(self) -> T {
        self
    }
}

// 实现引用类型（需要Clone）
impl<T: Clone> ContentProvider<T> for &T {
    fn provide(self) -> T {
        self.clone()
    }
}


/// 有类型的Create Future
pub struct TypedCreateFuture<T> {
    storage: Arc<soulcore::engines::storage::StorageEngine>,
    table: String,
    content: T,
}

impl<T> std::future::IntoFuture for TypedCreateFuture<T>
where
    T: Serialize + for<'de> serde::Deserialize<'de> + Clone + std::fmt::Debug + Send + 'static,
{
    type Output = std::result::Result<Vec<T>, surrealdb::Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;
    
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.storage.create(&self.table, self.content)
                .await
                .map_err(|e| surrealdb::Error::Api(surrealdb::error::Api::Query(e.to_string())))
        })
    }
}

/// Update包装器
pub struct UpdateWrapper {
    storage: Arc<soulcore::engines::storage::StorageEngine>,
    resource: ResourceId,
}

impl UpdateWrapper {
    fn new(storage: Arc<soulcore::engines::storage::StorageEngine>, resource: ResourceId) -> Self {
        Self { storage, resource }
    }
    
    /// content方法，能够处理引用和值
    pub fn content<T>(self, content: T) -> TypedUpdateFuture<T>
    where
        T: Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
    {
        TypedUpdateFuture {
            storage: self.storage,
            resource: self.resource,
            content,
        }
    }
}

/// 有类型的Update Future
pub struct TypedUpdateFuture<T> {
    storage: Arc<soulcore::engines::storage::StorageEngine>,
    resource: ResourceId,
    content: T,
}

impl<T> std::future::IntoFuture for TypedUpdateFuture<T>
where
    T: Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + 'static,
{
    type Output = std::result::Result<Option<T>, surrealdb::Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;
    
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let thing = match self.resource {
                ResourceId::Record(table, id) => {
                    Thing::from((table.as_str(), surrealdb::sql::Id::String(id)))
                }
                _ => {
                    return Err(surrealdb::Error::Api(surrealdb::error::Api::Query(
                        "Update requires a specific record ID".to_string()
                    )));
                }
            };
            
            self.storage.update(thing, self.content)
                .await
                .map_err(|e| surrealdb::Error::Api(surrealdb::error::Api::Query(e.to_string())))
        })
    }
}

#[derive(Clone)]
pub struct Database {
    pub client: ClientWrapper,
    pub config: Config,
    // 核心：soulcore存储引擎
    storage: Arc<soulcore::engines::storage::StorageEngine>,
}

impl Database {
    pub async fn new(config: &Config) -> Result<Self> {
        // 构建soulcore配置
        let soulcore_config = soulcore::config::StorageConfig {
            connection_mode: soulcore::config::ConnectionMode::Http,
            url: config.database.url.clone(),
            username: config.database.user.clone(),
            password: config.database.pass.clone(),
            namespace: config.database.namespace.clone(),
            database: config.database.database.clone(),
            pool_size: config.database.max_connections as usize,
            connection_timeout: config.database.connection_timeout,
            query_timeout: 60,
            max_retries: 3,
            retry_delay_ms: 1000,
        };
        
        // 创建soulcore存储引擎
        let storage = Arc::new(
            soulcore::engines::storage::StorageEngine::new(soulcore_config)
                .await
                .map_err(|e| {
                    error!("Failed to create soulcore storage engine: {}", e);
                    AppError::Internal(anyhow::anyhow!("Database initialization failed: {}", e))
                })?
        );
        
        // 验证连接
        let test_client = storage.get_connection().await
            .map_err(|e| {
                error!("Failed to get database connection: {}", e);
                AppError::Internal(anyhow::anyhow!("Database connection failed: {}", e))
            })?;
        
        test_client.verify_connection().await
            .map_err(|e| {
                error!("Failed to verify database connection: {}", e);
                AppError::Internal(anyhow::anyhow!("Database connection verification failed: {}", e))
            })?;
        
        info!("Successfully connected to SurrealDB via soulcore at {}", config.database.url);
        
        // 创建客户端包装器
        let client = ClientWrapper {
            storage: storage.clone(),
        };
        
        Ok(Database {
            client,
            config: config.clone(),
            storage,
        })
    }

    pub async fn verify_connection(&self) -> Result<()> {
        // 使用soulcore验证连接
        let client = self.storage.get_connection().await
            .map_err(|e| {
                error!("Failed to get connection for verification: {}", e);
                AppError::Internal(anyhow::anyhow!("Connection verification failed: {}", e))
            })?;
        
        client.verify_connection().await
            .map_err(|e| {
                error!("Database connection verification failed: {}", e);
                AppError::Internal(anyhow::anyhow!("Connection verification failed: {}", e))
            })?;
        
        info!("Database connection verified successfully");
        Ok(())
    }

    pub async fn health_check(&self) -> Result<DatabaseHealth> {
        let start = std::time::Instant::now();
        
        match self.verify_connection().await {
            Ok(_) => {
                let response_time = start.elapsed();
                let _pool_stats = self.storage.get_pool_stats();
                
                Ok(DatabaseHealth {
                    connected: true,
                    response_time_ms: response_time.as_millis() as u64,
                    error: None,
                })
            }
            Err(e) => {
                error!("Database health check failed: {}", e);
                Ok(DatabaseHealth {
                    connected: false,
                    response_time_ms: 0,
                    error: Some(e.to_string()),
                })
            }
        }
    }
    
    // 获取soulcore存储引擎（供需要高级功能的地方使用）
    pub fn storage(&self) -> &Arc<soulcore::engines::storage::StorageEngine> {
        &self.storage
    }
    
    // 使用soulcore查询构建器
    pub fn query_builder(&self) -> soulcore::surrealdb::QueryBuilder {
        self.storage.query_builder()
    }
    
    // 获取连接池统计
    pub fn get_pool_stats(&self) -> soulcore::surrealdb::connection_pool::PoolStats {
        self.storage.get_pool_stats()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub response_time_ms: u64,
    pub error: Option<String>,
}