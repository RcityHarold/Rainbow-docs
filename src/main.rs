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
        versions::VersionService,
        tags::TagService,
        file_upload::FileUploadService,
    },
    utils::markdown::MarkdownProcessor,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Config,
    pub auth_service: Arc<AuthService>,
    pub space_service: Arc<SpaceService>,
    pub file_upload_service: Arc<FileUploadService>,
    pub tag_service: Arc<TagService>,
    pub document_service: Arc<DocumentService>,
    pub comment_service: Arc<CommentService>,
    pub search_service: Arc<SearchService>,
    pub version_service: Arc<VersionService>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("rainbow_docs=debug,tower_http=debug"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Rainbow-Docs service...");

    // 检查是否需要显示安装界面
    #[cfg(feature = "installer")]
    {
        use crate::utils::installer::InstallationChecker;
        if let Ok(should_install) = InstallationChecker::should_show_installer() {
            if should_install {
                info!("System not installed, enabling installer routes");
                return start_installer_mode().await;
            }
        }
    }

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
    let space_service = Arc::new(SpaceService::new(shared_db.clone()));
    let file_upload_service = Arc::new(FileUploadService::new(shared_db.clone(), auth_service.clone()));
    let tag_service = Arc::new(TagService::new(shared_db.clone(), auth_service.clone()));
    
    let markdown_processor = Arc::new(MarkdownProcessor::new());
    let search_service = Arc::new(SearchService::new(shared_db.clone(), auth_service.clone()));
    let version_service = Arc::new(VersionService::new(shared_db.clone(), auth_service.clone()));
    let document_service = Arc::new(DocumentService::new(
        shared_db.clone(),
        auth_service.clone(),
        markdown_processor.clone(),
    ).with_search_service(search_service.clone()).with_version_service(version_service.clone()));
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
        space_service: space_service.clone(),
        file_upload_service: file_upload_service.clone(),
        tag_service: tag_service.clone(),
        document_service: document_service.clone(),
        comment_service: comment_service.clone(),
        search_service: search_service.clone(),
        version_service: version_service.clone(),
    };

    // 创建路由
    let app = Router::new()
        .nest("/api/docs/spaces", routes::spaces::router())
        .nest("/api/docs/files", routes::files::router())
        .nest("/api/docs/tags", routes::tags::router())
        .nest("/api/docs/documents", routes::documents::router())
        .nest("/api/docs/comments", routes::comments::router())
        .nest("/api/docs/search", routes::search::router())
        .nest("/api/docs/stats", routes::stats::router())
        .nest("/api/docs/versions", routes::versions::router())
        .nest("/api/install", routes::installer::installer_routes().with_state(()))
        .with_state(Arc::new(app_state))
        .layer(Extension(shared_db))
        .layer(Extension(config.clone()))
        .layer(Extension(auth_service.clone()))
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

#[cfg(feature = "installer")]
async fn start_installer_mode() -> anyhow::Result<()> {
    use crate::routes::installer::installer_routes;
    
    info!("Starting in installer mode...");
    
    // 创建简化的安装路由
    let app = Router::new()
        .nest("/api/install", installer_routes())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );
    
    // 启动服务器
    let addr = "0.0.0.0:3000";
    info!("Rainbow-Docs installer listening on {}", addr);
    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}