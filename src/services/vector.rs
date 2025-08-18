use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::services::database::Database;
use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorData {
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: usize,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchRequest {
    pub query_vector: Vec<f32>,
    pub space_id: Option<String>,
    pub limit: usize,
    pub threshold: f32,
    pub include_content: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub document_id: String,
    pub title: String,
    pub content: Option<String>,
    pub similarity: f32,
    pub space_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGetRequest {
    pub document_ids: Vec<String>,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVectorRequest {
    pub vectors: Vec<BatchVectorData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVectorData {
    pub document_id: String,
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorResponse {
    pub success: bool,
    pub vector_id: String,
    pub document_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResponse {
    pub results: Vec<VectorSearchResult>,
    pub total: usize,
    pub query_dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVectorsResponse {
    pub document_id: String,
    pub vectors: Vec<VectorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorInfo {
    pub vector_id: String,
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: usize,
    pub created_at: String,
}

pub struct VectorService {
    db: Arc<Database>,
}

impl VectorService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 存储文档向量
    pub async fn store_vector(
        &self,
        document_id: &str,
        vector_data: VectorData,
    ) -> Result<VectorResponse> {
        // 验证向量维度
        if vector_data.embedding.len() != vector_data.dimension {
            return Err(AppError::BadRequest("Vector dimension mismatch".to_string()));
        }

        // 获取文档信息以验证文档存在并获取space_id
        let doc_query = format!(
            "SELECT id, space_id FROM document WHERE id = document:{} LIMIT 1",
            document_id
        );
        
        let mut doc_result = self.db.client
            .query(doc_query)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch document: {}", e)))?;
        
        let docs: Vec<serde_json::Value> = doc_result.take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse document result: {}", e)))?;
        
        if docs.is_empty() {
            return Err(AppError::NotFound("Document not found".to_string()));
        }
        
        let space_id = docs[0]["space_id"].as_str()
            .ok_or_else(|| AppError::DatabaseError("Invalid space_id".to_string()))?
            .replace("space:", "");

        // 将向量转换为JSON格式
        let embedding_json = serde_json::to_string(&vector_data.embedding)
            .map_err(|e| AppError::DatabaseError(format!("Failed to serialize embedding: {}", e)))?;

        // 存储到数据库
        let metadata_json = vector_data.metadata
            .map(|m| m.to_string())
            .unwrap_or_else(|| "{}".to_string());

        let query = format!(
            r#"
            CREATE document_vector SET
                document_id = document:{},
                space_id = space:{},
                embedding = {},
                embedding_model = '{}',
                dimension = {},
                metadata = {},
                updated_at = time::now()
            "#,
            document_id,
            space_id,
            embedding_json,
            vector_data.model,
            vector_data.dimension,
            metadata_json
        );

        let mut result = self.db.client
            .query(query)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to store vector: {}", e)))?;

        let created: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse create result: {}", e)))?;

        let vector_id = created.get(0)
            .and_then(|v| v["id"].as_str())
            .ok_or_else(|| AppError::DatabaseError("Failed to get vector ID".to_string()))?;

        Ok(VectorResponse {
            success: true,
            vector_id: vector_id.to_string(),
            document_id: format!("document:{}", document_id),
        })
    }

    /// 向量相似度搜索
    pub async fn search_similar(
        &self,
        request: VectorSearchRequest,
    ) -> Result<VectorSearchResponse> {
        // 构建查询向量字符串
        let query_vector_str = serde_json::to_string(&request.query_vector)
            .map_err(|e| AppError::DatabaseError(format!("Failed to serialize query vector: {}", e)))?;
        
        // 构建查询
        let query = if let Some(space_id) = request.space_id {
            format!(
                r#"
                SELECT 
                    document_id,
                    vector::similarity::cosine(embedding, {}) as similarity,
                    document_id.title as title,
                    {} as content,
                    document_id.space_id as space_id
                FROM document_vector
                WHERE 
                    space_id = space:{}
                    AND vector::similarity::cosine(embedding, {}) >= {}
                ORDER BY similarity DESC
                LIMIT {}
                "#,
                query_vector_str,
                if request.include_content { "document_id.content" } else { "NONE" },
                space_id,
                query_vector_str,
                request.threshold,
                request.limit
            )
        } else {
            format!(
                r#"
                SELECT 
                    document_id,
                    vector::similarity::cosine(embedding, {}) as similarity,
                    document_id.title as title,
                    {} as content,
                    document_id.space_id as space_id
                FROM document_vector
                WHERE 
                    vector::similarity::cosine(embedding, {}) >= {}
                ORDER BY similarity DESC
                LIMIT {}
                "#,
                query_vector_str,
                if request.include_content { "document_id.content" } else { "NONE" },
                query_vector_str,
                request.threshold,
                request.limit
            )
        };

        let mut results = self.db.client
            .query(query)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to search vectors: {}", e)))?;

        let rows: Vec<serde_json::Value> = results.take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse search results: {}", e)))?;

        let search_results: Vec<VectorSearchResult> = rows
            .iter()
            .filter_map(|row| {
                Some(VectorSearchResult {
                    document_id: row["document_id"].as_str()?.to_string(),
                    title: row["title"].as_str()?.to_string(),
                    content: if request.include_content {
                        row["content"].as_str().map(|s| s.to_string())
                    } else {
                        None
                    },
                    similarity: row["similarity"].as_f64()? as f32,
                    space_id: row["space_id"].as_str()?.to_string(),
                })
            })
            .collect();

        Ok(VectorSearchResponse {
            total: search_results.len(),
            results: search_results,
            query_dimension: request.query_vector.len(),
        })
    }

    /// 获取文档的所有向量
    pub async fn get_document_vectors(
        &self,
        document_id: &str,
    ) -> Result<DocumentVectorsResponse> {
        let query = format!(
            r#"
            SELECT * FROM document_vector
            WHERE document_id = document:{}
            ORDER BY created_at DESC
            "#,
            document_id
        );

        let mut vectors = self.db.client
            .query(query)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get vectors: {}", e)))?;

        let rows: Vec<serde_json::Value> = vectors.take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse vector results: {}", e)))?;

        let vector_infos: Vec<VectorInfo> = rows
            .iter()
            .filter_map(|row| {
                Some(VectorInfo {
                    vector_id: row["id"].as_str()?.to_string(),
                    embedding: serde_json::from_value(row["embedding"].clone()).ok()?,
                    model: row["embedding_model"].as_str()?.to_string(),
                    dimension: row["dimension"].as_u64()? as usize,
                    created_at: row["created_at"].as_str()?.to_string(),
                })
            })
            .collect();

        Ok(DocumentVectorsResponse {
            document_id: format!("document:{}", document_id),
            vectors: vector_infos,
        })
    }

    /// 删除文档向量
    pub async fn delete_vector(
        &self,
        vector_id: &str,
    ) -> Result<bool> {
        let query = format!("DELETE {}", vector_id);
        
        self.db.client
            .query(query)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete vector: {}", e)))?;

        Ok(true)
    }

    /// 批量存储向量
    pub async fn store_vectors_batch(
        &self,
        vectors: Vec<BatchVectorData>,
    ) -> Result<Vec<String>> {
        let mut vector_ids = Vec::new();
        
        for vector_data in vectors {
            let result = self.store_vector(
                &vector_data.document_id,
                VectorData {
                    embedding: vector_data.embedding,
                    model: vector_data.model,
                    dimension: vector_data.dimension,
                    metadata: None,
                },
            ).await?;
            
            vector_ids.push(result.vector_id);
        }

        Ok(vector_ids)
    }

    /// 批量获取文档内容（供向量生成）
    pub async fn batch_get_documents(
        &self,
        document_ids: Vec<String>,
        fields: Vec<String>,
    ) -> Result<Vec<serde_json::Value>> {
        let ids_str = document_ids
            .iter()
            .map(|id| format!("document:{}", id))
            .collect::<Vec<_>>()
            .join(", ");
        
        let fields_str = fields.join(", ");
        
        let query = format!(
            "SELECT id, {} FROM document WHERE id IN [{}]",
            fields_str,
            ids_str
        );
        
        let mut results = self.db.client
            .query(query)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get documents: {}", e)))?;
        
        let docs: Vec<serde_json::Value> = results.take(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse documents: {}", e)))?;
        
        Ok(docs)
    }
}