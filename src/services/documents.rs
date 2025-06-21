use std::sync::Arc;
use surrealdb::{sql::Thing, Surreal, engine::remote::ws::Client};
use validator::Validate;

use crate::{
    error::ApiError,
    models::document::{Document, CreateDocumentRequest, UpdateDocumentRequest},
    models::search::SearchIndex,
    models::version::{CreateVersionRequest, VersionChangeType},
    services::{auth::AuthService, search::SearchService, versions::VersionService},
    utils::markdown::MarkdownProcessor,
};

#[derive(Clone)]
pub struct DocumentService {
    db: Arc<Surreal<Client>>,
    auth_service: Arc<AuthService>,
    markdown_processor: Arc<MarkdownProcessor>,
    search_service: Option<Arc<SearchService>>,
    version_service: Option<Arc<VersionService>>,
}

impl DocumentService {
    pub fn new(
        db: Arc<Surreal<Client>>,
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
        let processed = self.markdown_processor.process(&request.content).await?;

        let space_thing = Thing::from(("space", space_id));
        let parent_id_thing = if let Some(parent_id) = &request.parent_id {
            Some(Thing::from(("document", parent_id.as_str())))
        } else {
            None
        };

        let mut document = Document::new(
            space_thing,
            request.title.clone(),
            request.slug.clone(),
            request.content.clone(),
            author_id.to_string(),
        );

        if let Some(parent_id) = parent_id_thing {
            document = document.with_parent(parent_id);
        }

        if let Some(description) = request.description {
            document = document.with_description(description);
        }

        document.excerpt = Some(processed.excerpt);
        document.word_count = processed.word_count;
        document.reading_time = processed.reading_time;

        let created: Vec<Document> = self.db
            .create("document")
            .content(document.clone())
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let created_document = created
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::InternalServerError("Failed to create document".to_string()))?;

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
        let document: Option<Document> = self.db
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

        if let Some(description) = request.description {
            document.description = Some(description);
        }

        if let Some(is_public) = request.is_public {
            document.is_public = is_public;
        }

        document.updated_by = Some(editor_id.to_string());
        document.updated_at = surrealdb::sql::Datetime::default();

        let updated: Option<Document> = self.db
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

        let _: Option<Document> = self.db
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

        let documents: Vec<Document> = self.db
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

        let children: Vec<Document> = self.db
            .query(query)
            .bind(("parent_id", Thing::from(("document", parent_id))))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(children)
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
            document.parent_id = Some(Thing::from(("document", parent_id.as_str())));
        } else {
            document.parent_id = None;
        }

        if let Some(order_index) = new_order_index {
            document.order_index = order_index;
        }

        document.updated_by = Some(mover_id.to_string());
        document.updated_at = surrealdb::sql::Datetime::default();

        let updated: Option<Document> = self.db
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
            original.content.clone(),
            duplicator_id.to_string(),
        );

        new_document.description = original.description.clone();
        new_document.excerpt = original.excerpt.clone();
        new_document.word_count = original.word_count;
        new_document.reading_time = original.reading_time;
        new_document.is_public = original.is_public;

        let created: Vec<Document> = self.db
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

    async fn document_slug_exists(&self, space_id: &str, slug: &str) -> Result<bool, ApiError> {
        let query = "
            SELECT count() FROM document 
            WHERE space_id = $space_id 
            AND slug = $slug 
            AND is_deleted = false
            GROUP ALL
        ";

        let result: Vec<surrealdb::sql::Value> = self.db
            .query(query)
            .bind(("space_id", Thing::from(("space", space_id))))
            .bind(("slug", slug))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let count = result
            .first()
            .and_then(|v| v.as_int())
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

        let result: Vec<surrealdb::sql::Value> = self.db
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