use std::path::Path;
use std::fs;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationStatus {
    pub is_installed: bool,
    pub config_exists: bool,
    pub database_initialized: bool,
    pub admin_created: bool,
    pub install_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for InstallationStatus {
    fn default() -> Self {
        Self {
            is_installed: false,
            config_exists: false,
            database_initialized: false,
            admin_created: false,
            install_time: None,
        }
    }
}

pub struct InstallationChecker;

impl InstallationChecker {
    const INSTALL_MARKER_FILE: &'static str = ".rainbow_docs_installed";
    const CONFIG_FILE: &'static str = "config/production.toml";
    
    /// 检查系统是否已安装
    pub fn check_installation_status() -> Result<InstallationStatus> {
        let mut status = InstallationStatus::default();
        
        // 检查安装标记文件
        if Path::new(Self::INSTALL_MARKER_FILE).exists() {
            if let Ok(content) = fs::read_to_string(Self::INSTALL_MARKER_FILE) {
                if let Ok(marker_status) = serde_json::from_str::<InstallationStatus>(&content) {
                    status = marker_status;
                }
            }
        }
        
        // 检查配置文件
        status.config_exists = Path::new(Self::CONFIG_FILE).exists();
        
        // 更新整体安装状态
        status.is_installed = status.config_exists && status.database_initialized && status.admin_created;
        
        Ok(status)
    }
    
    /// 标记系统为已安装
    pub fn mark_as_installed(status: &InstallationStatus) -> Result<()> {
        let content = serde_json::to_string_pretty(status)?;
        fs::write(Self::INSTALL_MARKER_FILE, content)?;
        Ok(())
    }
    
    /// 检查是否需要显示安装界面
    pub fn should_show_installer() -> Result<bool> {
        #[cfg(feature = "installer")]
        {
            let status = Self::check_installation_status()?;
            Ok(!status.is_installed)
        }
        
        #[cfg(not(feature = "installer"))]
        {
            Ok(false)
        }
    }
}

#[cfg(feature = "installer")]
pub mod wizard {
    use super::*;
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Deserialize, Serialize)]
    pub struct InstallConfig {
        pub database_url: String,
        pub admin_username: String,
        pub admin_email: String,
        pub admin_password: String,
        pub site_name: String,
        pub site_description: Option<String>,
        pub jwt_secret: String,
    }
    
    #[derive(Debug, Serialize)]
    pub struct InstallStep {
        pub step: u8,
        pub title: String,
        pub description: String,
        pub completed: bool,
    }
    
    pub struct InstallationWizard;
    
    impl InstallationWizard {
        pub fn get_steps() -> Vec<InstallStep> {
            vec![
                InstallStep {
                    step: 1,
                    title: "环境检查".to_string(),
                    description: "检查系统环境和依赖".to_string(),
                    completed: false,
                },
                InstallStep {
                    step: 2,
                    title: "数据库配置".to_string(),
                    description: "配置SurrealDB连接".to_string(),
                    completed: false,
                },
                InstallStep {
                    step: 3,
                    title: "管理员账户".to_string(),
                    description: "创建系统管理员账户".to_string(),
                    completed: false,
                },
                InstallStep {
                    step: 4,
                    title: "站点配置".to_string(),
                    description: "配置站点基本信息".to_string(),
                    completed: false,
                },
                InstallStep {
                    step: 5,
                    title: "完成安装".to_string(),
                    description: "保存配置并初始化系统".to_string(),
                    completed: false,
                },
            ]
        }
        
        pub async fn perform_installation(config: InstallConfig) -> Result<()> {
            // 这里实现实际的安装逻辑
            // 1. 验证数据库连接
            // 2. 初始化数据库结构
            // 3. 创建管理员账户
            // 4. 生成配置文件
            // 5. 标记为已安装
            
            // 暂时返回成功，具体实现在下一步
            Ok(())
        }
    }
}