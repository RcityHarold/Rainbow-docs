use std::sync::Arc;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub text: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: usize,
    pub token_count: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub api_key: String,
    pub api_base_url: String,
    pub model: String,
    pub dimension: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "openai".to_string()),
            api_key: std::env::var("EMBEDDING_API_KEY").unwrap_or_default(),
            api_base_url: std::env::var("EMBEDDING_API_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            model: std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "text-embedding-3-small".to_string()),
            dimension: std::env::var("EMBEDDING_DIMENSION")
                .unwrap_or_else(|_| "1536".to_string())
                .parse()
                .unwrap_or(1536),
        }
    }
}

pub struct EmbeddingService {
    client: Client,
    config: EmbeddingConfig,
}

impl EmbeddingService {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub fn from_env() -> Self {
        Self::new(EmbeddingConfig::default())
    }

    /// 生成文本向量
    pub async fn generate_embedding(&self, text: &str) -> Result<EmbeddingResponse> {
        if text.trim().is_empty() {
            return Err(AppError::BadRequest("Text cannot be empty".to_string()));
        }

        match self.config.provider.as_str() {
            "openai" => self.generate_openai_embedding(text).await,
            "azure" => self.generate_azure_embedding(text).await,
            "local" => self.generate_local_embedding(text).await,
            _ => Err(AppError::BadRequest(format!("Unsupported embedding provider: {}", self.config.provider))),
        }
    }

    /// OpenAI 向量生成
    async fn generate_openai_embedding(&self, text: &str) -> Result<EmbeddingResponse> {
        #[derive(Serialize)]
        struct OpenAIRequest {
            input: String,
            model: String,
            dimensions: Option<usize>,
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            data: Vec<OpenAIEmbedding>,
            usage: OpenAIUsage,
        }

        #[derive(Deserialize)]
        struct OpenAIEmbedding {
            embedding: Vec<f32>,
        }

        #[derive(Deserialize)]
        struct OpenAIUsage {
            total_tokens: usize,
        }

        let request = OpenAIRequest {
            input: text.to_string(),
            model: self.config.model.clone(),
            dimensions: if self.config.model.contains("text-embedding-3") {
                Some(self.config.dimension)
            } else {
                None
            },
        };

        let response = self.client
            .post(&format!("{}/embeddings", self.config.api_base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("OpenAI API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalServiceError(format!("OpenAI API error: {}", error_text)));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Failed to parse OpenAI response: {}", e)))?;

        if openai_response.data.is_empty() {
            return Err(AppError::ExternalServiceError("No embeddings returned from OpenAI".to_string()));
        }

        Ok(EmbeddingResponse {
            embedding: openai_response.data[0].embedding.clone(),
            model: self.config.model.clone(),
            dimension: openai_response.data[0].embedding.len(),
            token_count: Some(openai_response.usage.total_tokens),
        })
    }

    /// Azure OpenAI 向量生成
    async fn generate_azure_embedding(&self, text: &str) -> Result<EmbeddingResponse> {
        // Azure OpenAI 的 API 格式与 OpenAI 类似，但 URL 结构不同
        #[derive(Serialize)]
        struct AzureRequest {
            input: String,
        }

        #[derive(Deserialize)]
        struct AzureResponse {
            data: Vec<AzureEmbedding>,
            usage: AzureUsage,
        }

        #[derive(Deserialize)]
        struct AzureEmbedding {
            embedding: Vec<f32>,
        }

        #[derive(Deserialize)]
        struct AzureUsage {
            total_tokens: usize,
        }

        let request = AzureRequest {
            input: text.to_string(),
        };

        let response = self.client
            .post(&format!("{}/openai/deployments/{}/embeddings", 
                self.config.api_base_url, 
                self.config.model
            ))
            .header("api-key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .query(&[("api-version", "2024-02-01")])
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Azure API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalServiceError(format!("Azure API error: {}", error_text)));
        }

        let azure_response: AzureResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Failed to parse Azure response: {}", e)))?;

        if azure_response.data.is_empty() {
            return Err(AppError::ExternalServiceError("No embeddings returned from Azure".to_string()));
        }

        Ok(EmbeddingResponse {
            embedding: azure_response.data[0].embedding.clone(),
            model: self.config.model.clone(),
            dimension: azure_response.data[0].embedding.len(),
            token_count: Some(azure_response.usage.total_tokens),
        })
    }

    /// 本地向量生成（如果有本地模型服务）
    async fn generate_local_embedding(&self, text: &str) -> Result<EmbeddingResponse> {
        #[derive(Serialize)]
        struct LocalRequest {
            text: String,
            model: String,
        }

        #[derive(Deserialize)]
        struct LocalResponse {
            embedding: Vec<f32>,
            model: String,
        }

        let request = LocalRequest {
            text: text.to_string(),
            model: self.config.model.clone(),
        };

        let response = self.client
            .post(&format!("{}/embeddings", self.config.api_base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Local API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalServiceError(format!("Local API error: {}", error_text)));
        }

        let local_response: LocalResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Failed to parse local response: {}", e)))?;

        Ok(EmbeddingResponse {
            embedding: local_response.embedding.clone(),
            model: local_response.model,
            dimension: local_response.embedding.len(),
            token_count: None,
        })
    }

    /// 批量生成向量
    pub async fn generate_embeddings_batch(&self, texts: &[&str]) -> Result<Vec<EmbeddingResponse>> {
        let mut results = Vec::new();
        
        // 为了避免API限制，分批处理
        const BATCH_SIZE: usize = 50;
        
        for chunk in texts.chunks(BATCH_SIZE) {
            for text in chunk {
                let embedding = self.generate_embedding(text).await?;
                results.push(embedding);
                
                // 小的延迟以避免过快的API调用
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }
        
        Ok(results)
    }

    /// 检查服务是否可用
    pub async fn health_check(&self) -> Result<bool> {
        // 尝试生成一个简单的测试向量
        match self.generate_embedding("test").await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 预处理文本内容
    pub fn preprocess_text(&self, content: &str) -> String {
        // 移除Markdown语法
        let content = regex::Regex::new(r"```[^`]*```").unwrap().replace_all(content, " ");
        let content = regex::Regex::new(r"`[^`]*`").unwrap().replace_all(&content, " ");
        let content = regex::Regex::new(r"!\[.*?\]\(.*?\)").unwrap().replace_all(&content, " ");
        let content = regex::Regex::new(r"\[.*?\]\(.*?\)").unwrap().replace_all(&content, " ");
        let content = regex::Regex::new(r"#{1,6}\s+").unwrap().replace_all(&content, " ");
        let content = regex::Regex::new(r"\*{1,2}([^*]+)\*{1,2}").unwrap().replace_all(&content, "$1");
        let content = regex::Regex::new(r"_{1,2}([^_]+)_{1,2}").unwrap().replace_all(&content, "$1");
        
        // 清理多余的空白字符
        let content = regex::Regex::new(r"\s+").unwrap().replace_all(&content, " ");
        
        content.trim().to_string()
    }

}