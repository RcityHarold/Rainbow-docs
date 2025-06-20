use std::sync::Arc;
use axum::{
    routing::Router,
    Extension,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::info;
use tokio::time::{interval, Duration};

mod routes;
mod models;
mod services;
mod config;
mod error;
mod utils;

use crate::{
    config::Config,
    services::{
        database::Database,
        auth::AuthService,
        spaces::SpaceService,
        documents::DocumentService,
        comments::CommentService,
        search::SearchService,
    },
    utils::markdown::MarkdownProcessor,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Config,
    pub auth_service: Arc<AuthService>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("rainbow_docs=debug,tower_http=debug"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Rainbow-Docs service...");

    // 加载配置
    dotenv::dotenv().ok();
    let config = Config::from_env()?;

    // 初始化数据库连接
    let db = Database::new(&config).await?;
    db.verify_connection().await?;
    
    info!("Database connection established. Please ensure database schema is initialized with docs_schema.sql");

    // 创建共享的数据库实例
    let shared_db = Arc::new(db.clone());

    // 创建认证服务
    let auth_service = Arc::new(AuthService::new(config.clone()));

    // 创建业务服务
    let markdown_processor = Arc::new(MarkdownProcessor::new());
    let space_service = Arc::new(SpaceService::new(shared_db.clone(), auth_service.clone()));
    let search_service = Arc::new(SearchService::new(shared_db.clone(), auth_service.clone()));
    let document_service = Arc::new(DocumentService::new(
        shared_db.clone(),
        auth_service.clone(),
        markdown_processor.clone(),
    ).with_search_service(search_service.clone()));
    let comment_service = Arc::new(CommentService::new(shared_db.clone(), auth_service.clone()));

    // 启动缓存清理任务
    let cleanup_auth = auth_service.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(1800)); // 每30分钟清理一次
        loop {
            interval.tick().await;
            cleanup_auth.cleanup_cache().await;
        }
    });

    // 创建 app state
    let app_state = AppState {
        db,
        config: config.clone(),
        auth_service: auth_service.clone(),
    };

    // 创建路由
    let app = Router::new()
        .nest("/api/spaces", routes::spaces::router())
        .nest("/api/docs", routes::documents::router())
        .nest("/api/comments", routes::comments::router())
        .nest("/api/search", routes::search::router())
        .nest("/api/stats", routes::stats::router())
        .with_state(space_service.clone())
        .with_state(document_service.clone())
        .with_state(comment_service.clone())
        .with_state(search_service.clone())
        .with_state(auth_service.clone())
        .layer(Extension(shared_db))
        .layer(Extension(Arc::new(app_state)))
        .layer(Extension(config.clone()))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // 启动服务器
    let addr = "0.0.0.0:3000";
    info!("Rainbow-Docs server listening on {}", addr);
    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}