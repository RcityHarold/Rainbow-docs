use std::sync::Arc;
use axum::{
    routing::Router,
    Extension,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{info, warn};
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
        space_member::SpaceMemberService,
        documents::DocumentService,
        comments::CommentService,
        publication::PublicationService,
        search::SearchService,
        versions::VersionService,
        tags::TagService,
        file_upload::FileUploadService,
    },
    utils::markdown::MarkdownProcessor,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub config: Config,
    pub auth_service: Arc<AuthService>,
    pub space_service: Arc<SpaceService>,
    pub space_member_service: Arc<SpaceMemberService>,
    pub file_upload_service: Arc<FileUploadService>,
    pub tag_service: Arc<TagService>,
    pub document_service: Arc<DocumentService>,
    pub comment_service: Arc<CommentService>,
    pub publication_service: Arc<PublicationService>,
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

    // 加载配置
    dotenv::dotenv().ok();
    let config = Config::from_env()?;

    // 检查是否需要跳过数据库连接（安装模式且未安装）
    #[cfg(feature = "installer")]
    {
        use crate::utils::installer::InstallationChecker;
        if let Ok(should_install) = InstallationChecker::should_show_installer() {
            if should_install {
                info!("System not installed, starting in installer-only mode");
                return start_installer_only_mode(config).await;
            }
        }
    }

    // 初始化数据库连接（已安装或非安装模式）
    // 如果数据库连接失败，尝试自动启动数据库
    let db = match Database::new(&config).await {
        Ok(db) => {
            match db.verify_connection().await {
                Ok(_) => {
                    info!("Database connection established successfully");
                    db
                }
                Err(e) => {
                    warn!("Database connection failed: {}", e);
                    info!("Attempting to auto-start database...");
                    
                    // 尝试自动启动数据库
                    if let Err(start_err) = auto_start_database(&config).await {
                        return Err(anyhow::anyhow!("Failed to auto-start database: {}. Original error: {}", start_err, e));
                    }
                    
                    // 重新尝试连接
                    let db = Database::new(&config).await?;
                    db.verify_connection().await?;
                    info!("Database auto-started and connected successfully");
                    db
                }
            }
        }
        Err(e) => {
            warn!("Failed to create database connection: {}", e);
            info!("Attempting to auto-start database...");
            
            // 尝试自动启动数据库
            if let Err(start_err) = auto_start_database(&config).await {
                return Err(anyhow::anyhow!("Failed to auto-start database: {}. Original error: {}", start_err, e));
            }
            
            // 重新尝试连接
            let db = Database::new(&config).await?;
            db.verify_connection().await?;
            info!("Database auto-started and connected successfully");
            db
        }
    };
    
    info!("Database connection established. Please ensure database schema is initialized with docs_schema.sql");

    // 创建共享的数据库实例
    let shared_db = Arc::new(db.clone());

    // 创建认证服务
    let auth_service = Arc::new(AuthService::new(config.clone()));

    // 创建业务服务
    let space_service = Arc::new(SpaceService::new(shared_db.clone()));
    let space_member_service = Arc::new(SpaceMemberService::new(shared_db.clone(), config.clone()));
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
    let publication_service = Arc::new(PublicationService::new(shared_db.clone()));

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
        db: shared_db.clone(),
        config: config.clone(),
        auth_service: auth_service.clone(),
        space_service: space_service.clone(),
        space_member_service: space_member_service.clone(),
        file_upload_service: file_upload_service.clone(),
        tag_service: tag_service.clone(),
        document_service: document_service.clone(),
        comment_service: comment_service.clone(),
        publication_service: publication_service.clone(),
        search_service: search_service.clone(),
        version_service: version_service.clone(),
    };

    // 创建路由
    let mut app = Router::new()
        .nest("/api/docs/spaces", routes::spaces::router())
        .nest("/api/docs/spaces", routes::space_members::router())
        .nest("/api/docs/files", routes::files::router())
        .nest("/api/docs/tags", routes::tags::router())
        .nest("/api/docs/documents", routes::documents::router())
        .nest("/api/docs/comments", routes::comments::router())
        .nest("/api/docs/notifications", routes::notifications::router())
        .nest("/api/docs/publications", routes::publication::router())
        .nest("/api/docs/search", routes::search::router())
        .nest("/api/docs/stats", routes::stats::router())
        .nest("/api/docs/versions", routes::versions::router())
        .with_state(Arc::new(app_state));

    // 如果是安装模式，额外添加安装路由
    #[cfg(feature = "installer")]
    {
        app = app.nest("/api/install", routes::installer::installer_routes());
    }

    let app = app
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

// 自动启动数据库的函数
async fn auto_start_database(config: &Config) -> anyhow::Result<()> {
    use std::process::Command;
    use std::fs;
    use std::path::Path;
    
    info!("Auto-starting SurrealDB database service...");
    
    // 创建数据目录（如果不存在）
    let data_dir = "./data";
    if !Path::new(data_dir).exists() {
        fs::create_dir_all(data_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create data directory: {}", e))?;
    }
    
    // 构建数据库文件路径
    let db_file = format!("{}/rainbow.db", data_dir);
    
    // 从配置中读取数据库认证信息
    let database_user = config.database.user.clone();
    let database_pass = config.database.pass.clone();
    let database_url = config.database.url.clone();
    
    // 构建启动命令
    let mut cmd = Command::new("surreal");
    cmd.arg("start")
       .arg("--auth")
       .arg("--user").arg(&database_user)
       .arg("--pass").arg(&database_pass)
       .arg("--bind").arg(&database_url)
       .arg(format!("file://{}", db_file));
    
    // 在后台启动数据库
    info!("Executing: surreal start --auth --user {} --pass *** --bind {} file://{}", 
           database_user, database_url, db_file);
    
    let child = cmd.spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start SurrealDB: {}. Please make sure SurrealDB is installed.", e))?;
    
    // 保存进程ID
    let pid = child.id();
    fs::write(".surreal_pid", pid.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to save database PID: {}", e))?;
    
    info!("SurrealDB process started (PID: {})", pid);
    
    // 等待数据库启动
    info!("Waiting for database service to be ready...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    
    info!("Database service should be ready now");
    Ok(())
}

#[cfg(feature = "installer")]
async fn start_installer_only_mode(config: Config) -> anyhow::Result<()> {
    use crate::routes::installer::installer_routes;
    
    info!("Starting installer-only mode (no database required)");
    
    // 创建仅包含安装路由的应用
    let app = Router::new()
        .nest("/api/install", installer_routes())
        .layer(Extension(config))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );
    
    // 启动服务器
    let addr = "0.0.0.0:3000";
    info!("Rainbow-Docs installer-only mode listening on {}", addr);
    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}

