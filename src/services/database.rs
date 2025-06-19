use crate::config::Config;
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use surrealdb::engine::remote::http::{Client, Http};
use surrealdb::{Surreal, opt::auth::Root};
use tracing::{info, warn, error};

#[derive(Clone)]
pub struct Database {
    pub client: Surreal<Client>,
    pub config: Config,
}

impl Database {
    pub async fn new(config: &Config) -> Result<Self> {
        let client = Surreal::new::<Http>(&config.database.url)
            .await
            .map_err(|e| {
                error!("Failed to connect to database at {}: {}", config.database.url, e);
                AppError::Database(e)
            })?;

        // 设置认证
        client
            .signin(Root {
                username: &config.database.user,
                password: &config.database.pass,
            })
            .await
            .map_err(|e| {
                error!("Database authentication failed: {}", e);
                AppError::Database(e)
            })?;

        // 使用命名空间和数据库
        client
            .use_ns(&config.database.namespace)
            .use_db(&config.database.database)
            .await
            .map_err(|e| {
                error!("Failed to select namespace/database: {}", e);
                AppError::Database(e)
            })?;

        info!("Successfully connected to SurrealDB at {}", config.database.url);

        Ok(Database {
            client,
            config: config.clone(),
        })
    }

    pub async fn verify_connection(&self) -> Result<()> {
        // 尝试执行一个简单的查询来验证连接
        let _: Vec<serde_json::Value> = self
            .client
            .query("SELECT 1 as test")
            .await
            .map_err(|e| {
                error!("Database connection verification failed: {}", e);
                AppError::Database(e)
            })?
            .take(0)
            .map_err(|e| {
                error!("Failed to parse connection test result: {}", e);
                AppError::Database(e)
            })?;

        info!("Database connection verified successfully");
        Ok(())
    }

    pub async fn health_check(&self) -> Result<DatabaseHealth> {
        let start = std::time::Instant::now();
        
        match self.verify_connection().await {
            Ok(_) => {
                let response_time = start.elapsed();
                Ok(DatabaseHealth {
                    connected: true,
                    response_time_ms: response_time.as_millis() as u64,
                    error: None,
                })
            }
            Err(e) => {
                warn!("Database health check failed: {}", e);
                Ok(DatabaseHealth {
                    connected: false,
                    response_time_ms: 0,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub response_time_ms: u64,
    pub error: Option<String>,
}