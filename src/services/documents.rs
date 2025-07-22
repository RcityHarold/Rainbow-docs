use std::sync::Arc;
use surrealdb::{sql::Thing, Surreal, engine::remote::ws::Client};
use validator::Validate;
use chrono::Utc;

use crate::{
    error::ApiError,
    models::document::{Document, CreateDocumentRequest, UpdateDocumentRequest, DocumentTreeNode, DocumentMetadata},
    models::version::{CreateVersionRequest, VersionChangeType},
    services::{auth::AuthService, search::SearchService, versions::VersionService, database::Database},
    utils::markdown::MarkdownProcessor,
};

#[derive(Clone)]
pub struct DocumentService {
    db: Arc<Database>,
    auth_service: Arc<AuthService>,
    markdown_processor: Arc<MarkdownProcessor>,
    search_service: Option<Arc<SearchService>>,
    version_service: Option<Arc<VersionService>>,
}

impl DocumentService {
    pub fn new(
        db: Arc<Database>,
        auth_service: Arc<AuthService>,
        markdown_processor: Arc<MarkdownProcessor>,
    ) -> Self {
        Self {
            db,
            auth_service,
            markdown_processor,
            search_service: None,
            version_service: None,
        }
    }

    pub fn with_search_service(mut self, search_service: Arc<SearchService>) -> Self {
        self.search_service = Some(search_service);
        self
    }

    pub fn with_version_service(mut self, version_service: Arc<VersionService>) -> Self {
        self.version_service = Some(version_service);
        self
    }

    pub async fn list_documents(
        &self,
        space_id: &str,
        query: crate::models::document::DocumentQuery,
        _user: Option<&crate::services::auth::User>,
    ) -> Result<serde_json::Value, ApiError> {
        use crate::models::document::{DocumentQuery, DocumentListItem, DocumentListResponse};
        
        // 提取实际的空间ID（去掉"space:"前缀，如果存在）
        let actual_space_id = if space_id.starts_with("space:") {
            space_id.strip_prefix("space:").unwrap()
        } else {
            space_id
        };

        // 注意：权限检查已经在路由层完成，这里不再重复检查

        // 构建查询条件
        let page = query.page.unwrap_or(1);
        let limit = query.limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        let mut where_conditions = vec![
            "space_id = $space_id".to_string(),
            "is_deleted = false".to_string()
        ];

        let mut bindings = vec![
            ("space_id", serde_json::Value::String(format!("space:{}", actual_space_id))),
            ("limit", serde_json::Value::Number(limit.into())),
            ("offset", serde_json::Value::Number(offset.into()))
        ];

        // 添加搜索条件
        if let Some(search) = &query.search {
            where_conditions.push("(title CONTAINS $search OR content CONTAINS $search)".to_string());
            bindings.push(("search", serde_json::Value::String(search.clone())));
        }

        // 添加父文档过滤
        if let Some(parent_id) = &query.parent_id {
            where_conditions.push("parent_id = $parent_id".to_string());
            bindings.push(("parent_id", serde_json::Value::String(format!("document:{}", parent_id))));
        }

        // 添加发布状态过滤
        if let Some(is_public) = query.is_public {
            where_conditions.push("is_public = $is_public".to_string());
            bindings.push(("is_public", serde_json::Value::Bool(is_public)));
        }

        let where_clause = where_conditions.join(" AND ");

        // 查询文档列表 - tags字段暂时使用空数组，children_count暂时设为0
        let documents_query = format!(
            "SELECT id, title, slug, excerpt, is_public, parent_id, order_index, author_id, 
                    view_count, created_at, updated_at, [] as tags, 0 as children_count
             FROM document 
             WHERE {} 
             ORDER BY order_index ASC, created_at DESC 
             LIMIT $limit START $offset",
            where_clause
        );

        // Debug logging - remove in production
        // tracing::debug!("Document query: {}", documents_query);
        // tracing::debug!("Query bindings: {:?}", bindings);

        let mut documents_result = self.db.client.query(&documents_query);
        for (key, value) in &bindings {
            documents_result = documents_result.bind((key, value));
        }

        let documents: Vec<DocumentListItem> = documents_result
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // tracing::debug!("Found {} documents", documents.len());

        // 查询总数
        let count_query = format!(
            "SELECT count() FROM document WHERE {} GROUP ALL",
            where_clause
        );

        let mut count_result = self.db.client.query(&count_query);
        for (key, value) in &bindings {
            if *key != "limit" && *key != "offset" {
                count_result = count_result.bind((key, value));
            }
        }

        let mut count_response = count_result
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            
        let count_results: Vec<surrealdb::sql::Value> = count_response
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 将 surrealdb::sql::Value 转换为 serde_json::Value
        let total = if let Some(first_result) = count_results.first() {
            let json_value: serde_json::Value = first_result.clone().try_into()
                .map_err(|_| ApiError::InternalServerError("Failed to convert count result".to_string()))?;
            json_value
                .get("count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32
        } else {
            0
        };

        let total_pages = (total + limit - 1) / limit;

        let response = DocumentListResponse {
            documents,
            total,
            page,
            limit,
            total_pages,
        };

        Ok(serde_json::to_value(response)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
    }

    pub async fn create_document(
        &self,
        space_id: &str,
        author_id: &str,
        request: CreateDocumentRequest,
    ) -> Result<Document, ApiError> {
        request.validate()?;

        // 检查slug在空间内是否唯一
        if self.document_slug_exists(space_id, &request.slug).await? {
            return Err(ApiError::Conflict("Document slug already exists in this space".to_string()));
        }

        // 验证父文档存在性
        if let Some(parent_id) = &request.parent_id {
            self.verify_parent_document(space_id, parent_id).await?;
        }

        // 处理Markdown内容
        let content = request.content.as_deref().unwrap_or("");
        let processed = self.markdown_processor.process(content).await?;

        // 使用 SurrealQL 创建记录 - 不设置 metadata，让它使用默认值
        let query = if request.parent_id.is_some() {
            r#"
                CREATE document SET
                    space_id = type::thing('space', $space_id),
                    title = $title,
                    slug = $slug,
                    author_id = $author_id,
                    content = $content,
                    excerpt = $excerpt,
                    word_count = $word_count,
                    reading_time = $reading_time,
                    is_public = $is_public,
                    parent_id = type::thing('document', $parent_id),
                    order_index = $order_index
            "#
        } else {
            r#"
                CREATE document SET
                    space_id = type::thing('space', $space_id),
                    title = $title,
                    slug = $slug,
                    author_id = $author_id,
                    content = $content,
                    excerpt = $excerpt,
                    word_count = $word_count,
                    reading_time = $reading_time,
                    is_public = $is_public,
                    parent_id = NONE,
                    order_index = $order_index
            "#
        };
        
        // 提取space_id的实际ID部分（去掉"space:"前缀）
        let actual_space_id = if space_id.starts_with("space:") {
            space_id.strip_prefix("space:").unwrap()
        } else {
            space_id
        };

        let mut query_builder = self.db.client.query(query);
        query_builder = query_builder
            .bind(("space_id", actual_space_id))
            .bind(("title", request.title.clone()))
            .bind(("slug", request.slug.clone()))
            .bind(("author_id", author_id.to_string()))
            .bind(("content", content.to_string()))
            .bind(("excerpt", processed.excerpt.clone()))
            .bind(("word_count", processed.word_count))
            .bind(("reading_time", processed.reading_time))
            .bind(("is_public", request.is_public.unwrap_or(false)))
            .bind(("order_index", request.order_index.unwrap_or(0)));
            
        if let Some(parent_id) = &request.parent_id {
            query_builder = query_builder.bind(("parent_id", parent_id));
        }
        
        let mut result = query_builder
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            
        let created: Vec<crate::models::document::DocumentDb> = result
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let created_document_db = created
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::InternalServerError("Failed to create document".to_string()))?;
        
        // 转换为普通的 Document
        let created_document: Document = created_document_db.into();

        // 更新搜索索引
        if let Some(search_service) = &self.search_service {
            let _ = search_service.update_document_index(
                &created_document.id.as_ref().unwrap().to_string(),
                space_id,
                &created_document.title,
                &created_document.content,
                &created_document.excerpt.clone().unwrap_or_default(),
                Vec::new(), // 标签将在后续更新
                author_id,
                created_document.is_public,
            ).await;
        }

        // 创建初始版本
        if let Some(version_service) = &self.version_service {
            let version_request = CreateVersionRequest {
                title: created_document.title.clone(),
                content: created_document.content.clone(),
                summary: Some("Initial version".to_string()),
                change_type: VersionChangeType::Created,
            };
            
            let _ = version_service.create_version(
                &created_document.id.as_ref().unwrap().to_string(),
                author_id,
                version_request,
            ).await;
        }

        Ok(created_document)
    }

    pub async fn get_document(&self, document_id: &str) -> Result<Document, ApiError> {
        let document: Option<Document> = self.db.client
            .select(("document", document_id))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        document.ok_or_else(|| ApiError::NotFound("Document not found".to_string()))
    }

    pub async fn update_document(
        &self,
        document_id: &str,
        editor_id: &str,
        request: UpdateDocumentRequest,
    ) -> Result<Document, ApiError> {
        request.validate()?;

        let mut document = self.get_document(document_id).await?;

        if let Some(title) = request.title {
            document.title = title;
        }

        if let Some(content) = request.content {
            let processed = self.markdown_processor.process(&content).await?;
            document.content = content;
            document.excerpt = Some(processed.excerpt);
            document.word_count = processed.word_count;
            document.reading_time = processed.reading_time;
        }

        if let Some(excerpt) = request.excerpt {
            document.excerpt = Some(excerpt);
        }

        if let Some(is_public) = request.is_public {
            document.is_public = is_public;
        }

        document.updated_by = Some(editor_id.to_string());
        document.updated_at = Some(chrono::Utc::now());

        let updated: Option<Document> = self.db.client
            .update(("document", document_id))
            .content(document.clone())
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let updated_document = updated
            .ok_or_else(|| ApiError::InternalServerError("Failed to update document".to_string()))?;

        // 更新搜索索引
        if let Some(search_service) = &self.search_service {
            let _ = search_service.update_document_index(
                document_id,
                &updated_document.space_id.to_string(),
                &updated_document.title,
                &updated_document.content,
                &updated_document.excerpt.clone().unwrap_or_default(),
                Vec::new(), // 标签将在后续更新
                &updated_document.author_id,
                updated_document.is_public,
            ).await;
        }

        // 创建新版本
        if let Some(version_service) = &self.version_service {
            let version_request = CreateVersionRequest {
                title: updated_document.title.clone(),
                content: updated_document.content.clone(),
                summary: Some("Document updated".to_string()),
                change_type: VersionChangeType::Updated,
            };
            
            let _ = version_service.create_version(
                document_id,
                editor_id,
                version_request,
            ).await;
        }

        Ok(updated_document)
    }

    pub async fn delete_document(&self, document_id: &str, deleter_id: &str) -> Result<(), ApiError> {
        let mut document = self.get_document(document_id).await?;
        
        document.soft_delete(deleter_id.to_string());

        let _: Option<Document> = self.db.client
            .update(("document", document_id))
            .content(document)
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 从搜索索引中删除
        if let Some(search_service) = &self.search_service {
            let _ = search_service.delete_index(document_id).await;
        }

        Ok(())
    }

    pub async fn get_space_documents(
        &self,
        space_id: &str,
        page: i64,
        per_page: i64,
    ) -> Result<Vec<Document>, ApiError> {
        let offset = (page - 1) * per_page;
        
        let query = "
            SELECT * FROM document 
            WHERE space_id = $space_id 
            AND is_deleted = false
            ORDER BY created_at DESC
            LIMIT $limit START $offset
        ";

        let documents: Vec<Document> = self.db.client
            .query(query)
            .bind(("space_id", Thing::from(("space", space_id))))
            .bind(("limit", per_page))
            .bind(("offset", offset))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(documents)
    }

    pub async fn get_document_children(
        &self,
        parent_id: &str,
    ) -> Result<Vec<Document>, ApiError> {
        let query = "
            SELECT * FROM document 
            WHERE parent_id = $parent_id 
            AND is_deleted = false
            ORDER BY order_index ASC, created_at ASC
        ";

        let children_db: Vec<crate::models::document::DocumentDb> = self.db.client
            .query(query)
            .bind(("parent_id", Thing::from(("document", parent_id))))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let children: Vec<Document> = children_db.into_iter()
            .map(|db| db.into())
            .collect();

        Ok(children)
    }

    pub async fn get_document_children_by_id(
        &self,
        parent_id: &str,
    ) -> Result<Vec<Document>, ApiError> {
        // 分离前缀和实际ID
        let actual_id = if parent_id.starts_with("document:") {
            parent_id.strip_prefix("document:").unwrap()
        } else {
            parent_id
        };
        
        self.get_document_children(actual_id).await
    }

    pub async fn get_document_tree(&self, space_id: &str) -> Result<Vec<DocumentTreeNode>, ApiError> {
        tracing::debug!("Getting document tree for space_id: {}", space_id);
        
        // 提取实际的空间ID（去掉"space:"前缀，如果存在）
        let actual_space_id = if space_id.starts_with("space:") {
            space_id.strip_prefix("space:").unwrap()
        } else {
            space_id
        };
        
        // 获取空间内所有文档
        let query = "
            SELECT * FROM document 
            WHERE space_id = $space_id 
            AND is_deleted = false
            ORDER BY order_index ASC, created_at ASC
        ";

        let space_thing = Thing::from(("space", actual_space_id));
        tracing::debug!("Querying with space_thing: {:?}", space_thing);
        
        let all_documents_db: Vec<crate::models::document::DocumentDb> = self.db.client
            .query(query)
            .bind(("space_id", space_thing))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            
        tracing::debug!("Found {} documents in database", all_documents_db.len());
            
        // 转换为 Document
        let all_documents: Vec<Document> = all_documents_db.into_iter()
            .map(|db| db.into())
            .collect();
            
        tracing::debug!("Converted to {} Document objects", all_documents.len());

        // 构建文档映射
        let mut doc_map = std::collections::HashMap::new();
        let mut root_docs = Vec::new();

        for doc in all_documents {
            if let Some(doc_id) = &doc.id {
                let node = DocumentTreeNode {
                    id: doc_id.clone(),
                    title: doc.title.clone(),
                    slug: doc.slug.clone(),
                    is_public: doc.is_public,
                    order_index: doc.order_index,
                    children: Vec::new(),
                };
                
                if doc.parent_id.is_none() {
                    root_docs.push(doc_id.clone());
                }
                
                doc_map.insert(doc_id.clone(), (node, doc.parent_id.clone()));
            }
        }

        // 构建树结构
        let mut tree_map = std::collections::HashMap::new();
        for (doc_id, (node, parent_id)) in doc_map {
            if parent_id.is_none() {
                tree_map.insert(doc_id, node);
            } else if let Some(parent_id_str) = parent_id {
                if let Some(parent_id_thing) = parent_id_str.to_string().split(':').nth(1) {
                    tree_map.entry(parent_id_thing.to_string())
                        .or_insert_with(|| DocumentTreeNode {
                            id: parent_id_thing.to_string(),
                            title: "Unknown".to_string(),
                            slug: "unknown".to_string(),
                            is_public: false,
                            order_index: 0,
                            children: Vec::new(),
                        })
                        .children.push(node);
                }
            }
        }

        // 返回根节点
        let mut result = Vec::new();
        for root_id in root_docs {
            if let Some(node) = tree_map.remove(&root_id) {
                result.push(node);
            }
        }

        tracing::debug!("Returning {} root documents in tree", result.len());
        Ok(result)
    }

    pub async fn move_document(
        &self,
        document_id: &str,
        new_parent_id: Option<String>,
        new_order_index: Option<i32>,
        mover_id: &str,
    ) -> Result<Document, ApiError> {
        let mut document = self.get_document(document_id).await?;

        if let Some(parent_id) = new_parent_id {
            self.verify_parent_document(&document.space_id.to_string(), &parent_id).await?;
            document.parent_id = Some(parent_id);
        } else {
            document.parent_id = None;
        }

        if let Some(order_index) = new_order_index {
            document.order_index = order_index;
        }

        document.updated_by = Some(mover_id.to_string());
        document.updated_at = Some(chrono::Utc::now());

        let updated: Option<Document> = self.db.client
            .update(("document", document_id))
            .content(document.clone())
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        updated.ok_or_else(|| ApiError::InternalServerError("Failed to move document".to_string()))
    }

    pub async fn get_document_breadcrumbs(&self, document_id: &str) -> Result<Vec<Document>, ApiError> {
        let mut breadcrumbs = Vec::new();
        let mut current_id = Some(document_id.to_string());

        while let Some(id) = current_id {
            let document = self.get_document(&id).await?;
            current_id = document.parent_id.as_ref().map(|p| p.to_string());
            breadcrumbs.push(document);
        }

        breadcrumbs.reverse();
        Ok(breadcrumbs)
    }

    pub async fn get_document_breadcrumbs_by_id(&self, document_id: &str) -> Result<Vec<Document>, ApiError> {
        // 分离前缀和实际ID
        let actual_id = if document_id.starts_with("document:") {
            document_id.strip_prefix("document:").unwrap()
        } else {
            document_id
        };
        
        let mut breadcrumbs = Vec::new();
        let mut current_id = Some(actual_id.to_string());

        while let Some(id) = current_id {
            let document = self.get_document_by_id(&format!("document:{}", id)).await?;
            // 从parent_id中提取实际ID
            current_id = document.parent_id.as_ref().map(|p| {
                if p.starts_with("document:") {
                    p.strip_prefix("document:").unwrap().to_string()
                } else {
                    p.clone()
                }
            });
            breadcrumbs.push(document);
        }

        breadcrumbs.reverse();
        Ok(breadcrumbs)
    }

    pub async fn duplicate_document(
        &self,
        document_id: &str,
        new_title: Option<String>,
        new_slug: Option<String>,
        duplicator_id: &str,
    ) -> Result<Document, ApiError> {
        let original = self.get_document(document_id).await?;
        
        let title = new_title.unwrap_or_else(|| format!("{} (Copy)", original.title));
        let slug = new_slug.unwrap_or_else(|| format!("{}-copy", original.slug));

        // 检查新slug是否唯一
        if self.document_slug_exists(&original.space_id.to_string(), &slug).await? {
            return Err(ApiError::Conflict("New slug already exists".to_string()));
        }

        let mut new_document = Document::new(
            original.space_id.clone(),
            title,
            slug,
            duplicator_id.to_string(),
        );
        new_document.content = original.content.clone();

        new_document.excerpt = original.excerpt.clone();
        new_document.word_count = original.word_count;
        new_document.reading_time = original.reading_time;
        new_document.is_public = original.is_public;

        let created: Vec<Document> = self.db.client
            .create("document")
            .content(new_document)
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let created_document = created
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::InternalServerError("Failed to duplicate document".to_string()))?;

        // 更新搜索索引
        if let Some(search_service) = &self.search_service {
            let _ = search_service.update_document_index(
                &created_document.id.as_ref().unwrap().to_string(),
                &created_document.space_id.to_string(),
                &created_document.title,
                &created_document.content,
                &created_document.excerpt.clone().unwrap_or_default(),
                Vec::new(),
                duplicator_id,
                created_document.is_public,
            ).await;
        }

        Ok(created_document)
    }

    pub async fn get_document_by_slug(&self, space_id: &str, slug: &str) -> Result<Document, ApiError> {
        let query = "
            SELECT * FROM document 
            WHERE space_id = $space_id 
            AND slug = $slug 
            AND is_deleted = false
        ";

        let mut result = self.db.client
            .query(query)
            .bind(("space_id", Thing::from(("space", space_id))))
            .bind(("slug", slug))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let documents: Vec<Document> = result
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        documents.into_iter()
            .next()
            .ok_or_else(|| ApiError::NotFound("Document not found".to_string()))
    }

    pub async fn get_document_by_id(&self, id: &str) -> Result<Document, ApiError> {
        // 添加调试日志
        tracing::info!("Searching for document with ID: '{}'", id);
        
        // 分离前缀和实际ID
        let actual_id = if id.starts_with("document:") {
            id.strip_prefix("document:").unwrap()
        } else {
            id
        };
        
        tracing::info!("Using actual_id for Thing: '{}'", actual_id);

        let query = "
            SELECT * FROM document 
            WHERE id = $id 
            AND is_deleted = false
        ";

        let mut result = self.db.client
            .query(query)
            .bind(("id", Thing::from(("document", actual_id))))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let documents_db: Vec<crate::models::document::DocumentDb> = result
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        tracing::info!("Found {} documents from database", documents_db.len());

        let document_db = documents_db.into_iter()
            .next()
            .ok_or_else(|| ApiError::NotFound("Document not found".to_string()))?;
        
        // 转换为普通的 Document
        let document: Document = document_db.into();
        Ok(document)
    }

    async fn document_slug_exists(&self, space_id: &str, slug: &str) -> Result<bool, ApiError> {
        let query = "
            SELECT count() FROM document 
            WHERE space_id = $space_id 
            AND slug = $slug 
            AND is_deleted = false
            GROUP ALL
        ";

        let result: Vec<surrealdb::sql::Value> = self.db.client
            .query(query)
            .bind(("space_id", Thing::from(("space", space_id))))
            .bind(("slug", slug))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let count = result
            .first()
            .and_then(|v| v.to_string().parse::<i64>().ok())
            .unwrap_or(0);

        Ok(count > 0)
    }

    async fn verify_parent_document(&self, space_id: &str, parent_id: &str) -> Result<(), ApiError> {
        let query = "
            SELECT id FROM document 
            WHERE id = $parent_id 
            AND space_id = $space_id 
            AND is_deleted = false
        ";

        let result: Vec<surrealdb::sql::Value> = self.db.client
            .query(query)
            .bind(("parent_id", Thing::from(("document", parent_id))))
            .bind(("space_id", Thing::from(("space", space_id))))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        if result.is_empty() {
            return Err(ApiError::NotFound("Parent document not found".to_string()));
        }

        Ok(())
    }
}