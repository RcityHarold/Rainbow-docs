use crate::config::Config;
use crate::error::{AppError, Result};
use crate::models::document::{
    Document, DocumentResponse, DocumentListResponse, DocumentListItem, DocumentQuery,
    CreateDocumentRequest, UpdateDocumentRequest, BreadcrumbItem, DocumentTreeNode
};
use crate::services::auth::User;
use crate::services::database::Database;
use crate::utils::markdown::MarkdownProcessor;
use serde_json::Value;
use std::sync::Arc;
use surrealdb::sql::Thing;
use tracing::{info, warn, error, debug};
use validator::Validate;

pub struct DocumentService {
    db: Arc<Database>,
    markdown_processor: MarkdownProcessor,
}

impl DocumentService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { 
            db,
            markdown_processor: MarkdownProcessor::new(),
        }
    }

    /// 创建新文档
    pub async fn create_document(&self, space_slug: &str, request: CreateDocumentRequest, user: &User) -> Result<DocumentResponse> {
        // 验证输入
        request.validate().map_err(|e| AppError::Validation(e.to_string()))?;

        // 获取空间信息并检查权限
        let space = self.get_space_by_slug(space_slug).await?;
        self.check_write_permission(&space, user).await?;

        // 检查文档slug在空间内是否唯一
        if self.document_slug_exists(&space.id, &request.slug).await? {
            return Err(AppError::Conflict("Document slug already exists in this space".to_string()));
        }

        // 验证父文档存在性
        if let Some(parent_id) = &request.parent_id {
            self.verify_parent_document(&space.id, parent_id).await?;
        }

        // 创建文档对象
        let mut document = Document::new(
            format!("space:{}", space.id),
            request.title,
            request.slug.clone(),
            user.id.clone(),
        );

        if let Some(content) = request.content {
            document.content = content;
        }

        if let Some(excerpt) = request.excerpt {
            document.excerpt = Some(excerpt);
        } else {
            // 自动生成摘要
            document.excerpt = Some(document.generate_excerpt());
        }

        if let Some(is_published) = request.is_published {
            document.is_published = is_published;
        }

        if let Some(parent_id) = request.parent_id {
            document.parent_id = Some(parent_id);
        }

        if let Some(sort_order) = request.sort_order {
            document.sort_order = sort_order;
        }

        if let Some(metadata) = request.metadata {
            document.metadata = metadata;
        }

        // 计算阅读时间
        document.metadata.reading_time = Some(document.estimate_reading_time());

        // 保存到数据库
        let created_document: Option<Document> = self.db.client
            .create("document")
            .content(&document)
            .await
            .map_err(|e| {
                error!("Failed to create document: {}", e);
                AppError::Database(e)
            })?
            .into_iter()
            .next();

        let created_document = created_document.ok_or_else(|| {
            error!("Failed to get created document from database");
            AppError::Internal(anyhow::anyhow!("Failed to create document"))
        })?;

        // 更新搜索索引
        self.update_search_index(&created_document).await?;

        info!("Created new document: {} in space: {} by user: {}", request.slug, space_slug, user.id);

        // 记录活动日志
        self.log_activity(&user.id, "document_created", "document", &created_document.id.as_ref().unwrap_or(&String::new())).await?;

        // 生成响应
        let mut response = DocumentResponse::from(created_document);
        response.rendered_content = Some(self.markdown_processor.render(&response.content)?);
        response.breadcrumbs = Some(self.generate_breadcrumbs(&response.id, &space.id).await?);

        Ok(response)
    }

    /// 获取文档列表
    pub async fn list_documents(&self, space_slug: &str, query: DocumentQuery, user: Option<&User>) -> Result<DocumentListResponse> {
        // 获取空间信息并检查权限
        let space = self.get_space_by_slug(space_slug).await?;
        self.check_read_permission(&space, user).await?;

        let page = query.page.unwrap_or(1);
        let limit = query.limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        // 构建查询条件
        let mut where_conditions = vec![format!("space_id = space:{}", space.id)];
        let mut params = std::collections::HashMap::new();

        // 权限过滤：只显示已发布的文档，除非是作者或有管理权限
        if let Some(user) = user {
            if !self.can_manage_space(&space, user) {
                where_conditions.push("(is_published = true OR author_id = $user_id)".to_string());
                params.insert("user_id".to_string(), user.id.clone().into());
            }
        } else {
            where_conditions.push("is_published = true".to_string());
        }

        // 其他过滤条件
        if let Some(search) = &query.search {
            where_conditions.push("(string::lowercase(title) CONTAINS string::lowercase($search) OR string::lowercase(content) CONTAINS string::lowercase($search))".to_string());
            params.insert("search".to_string(), search.clone().into());
        }

        if let Some(parent_id) = &query.parent_id {
            where_conditions.push("parent_id = $parent_id".to_string());
            params.insert("parent_id".to_string(), parent_id.clone().into());
        }

        if let Some(is_published) = query.is_published {
            where_conditions.push("is_published = $is_published".to_string());
            params.insert("is_published".to_string(), is_published.into());
        }

        if let Some(author_id) = &query.author_id {
            where_conditions.push("author_id = $author_id".to_string());
            params.insert("author_id".to_string(), author_id.clone().into());
        }

        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                where_conditions.push("metadata.tags ANYINSIDE $tags".to_string());
                params.insert("tags".to_string(), tags.clone().into());
            }
        }

        let where_clause = format!("WHERE {}", where_conditions.join(" AND "));

        // 排序
        let sort_field = query.sort.unwrap_or_else(|| "sort_order".to_string());
        let sort_order = query.order.unwrap_or_else(|| "asc".to_string());
        let order_clause = format!("ORDER BY {} {}", sort_field, sort_order);

        // 查询总数
        let count_query = format!("SELECT count() FROM document {}", where_clause);
        let total: Option<u32> = self.db.client
            .query(&count_query)
            .bind(params.clone())
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "count"))?;

        let total = total.unwrap_or(0);
        let total_pages = (total + limit - 1) / limit;

        // 查询数据
        let data_query = format!(
            "SELECT *, (SELECT count() FROM document WHERE parent_id = $parent.id) AS children_count FROM document {} {} LIMIT {} START {}",
            where_clause, order_clause, limit, offset
        );

        let documents: Vec<Document> = self.db.client
            .query(&data_query)
            .bind(params)
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        // 转换为响应格式
        let document_items: Vec<DocumentListItem> = documents.into_iter()
            .map(|doc| DocumentListItem::from(doc))
            .collect();

        debug!("Listed {} documents in space: {} for user: {:?}", document_items.len(), space_slug, user.map(|u| &u.id));

        Ok(DocumentListResponse {
            documents: document_items,
            total,
            page,
            limit,
            total_pages,
        })
    }

    /// 根据slug获取文档详情
    pub async fn get_document_by_slug(&self, space_slug: &str, doc_slug: &str, user: Option<&User>) -> Result<DocumentResponse> {
        // 获取空间信息
        let space = self.get_space_by_slug(space_slug).await?;

        // 查询文档
        let document: Option<Document> = self.db.client
            .query("SELECT * FROM document WHERE space_id = $space_id AND slug = $slug")
            .bind(("space_id", format!("space:{}", space.id)))
            .bind(("slug", doc_slug))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        let document = document.ok_or_else(|| AppError::NotFound("Document not found".to_string()))?;

        // 检查读取权限
        if !document.can_read(user.map(|u| u.id.as_str()), space.is_public) {
            return Err(AppError::Authorization("Access denied to this document".to_string()));
        }

        // 增加浏览量（异步，不影响响应）
        let doc_id = document.id.clone();
        let db_clone = self.db.clone();
        tokio::spawn(async move {
            if let Some(id) = doc_id {
                let _ = db_clone.client
                    .query("UPDATE document SET view_count += 1 WHERE id = $id")
                    .bind(("id", format!("document:{}", id)))
                    .await;
            }
        });

        // 生成响应
        let mut response = DocumentResponse::from(document);
        response.rendered_content = Some(self.markdown_processor.render(&response.content)?);
        response.breadcrumbs = Some(self.generate_breadcrumbs(&response.id, &space.id).await?);
        
        // 获取子文档
        response.children = Some(self.get_child_documents(&response.id).await?);

        debug!("Retrieved document: {} in space: {} for user: {:?}", doc_slug, space_slug, user.map(|u| &u.id));

        Ok(response)
    }

    /// 更新文档
    pub async fn update_document(&self, space_slug: &str, doc_slug: &str, request: UpdateDocumentRequest, user: &User) -> Result<DocumentResponse> {
        // 验证输入
        request.validate().map_err(|e| AppError::Validation(e.to_string()))?;

        // 获取空间和文档
        let space = self.get_space_by_slug(space_slug).await?;
        let existing_doc = self.get_document_by_slug(space_slug, doc_slug, Some(user)).await?;

        // 检查写入权限
        self.check_write_permission(&space, user).await?;

        // 检查是否是作者或有管理权限
        if existing_doc.author_id != user.id && !self.can_manage_space(&space, user) {
            return Err(AppError::Authorization("Only document author or space owner can update document".to_string()));
        }

        // 构建更新数据
        let mut update_data = std::collections::HashMap::new();
        
        if let Some(title) = request.title {
            update_data.insert("title", Value::String(title));
        }
        
        if let Some(content) = request.content {
            // 更新内容时重新生成摘要和阅读时间
            let excerpt = if request.excerpt.is_some() {
                request.excerpt.unwrap()
            } else {
                // 从新内容生成摘要（简化版）
                if content.len() <= 150 {
                    content.clone()
                } else {
                    format!("{}...", &content[..147])
                }
            };
            
            let reading_time = content.split_whitespace().count() / 200;
            
            update_data.insert("content", Value::String(content));
            update_data.insert("excerpt", Value::String(excerpt));
            update_data.insert("metadata.reading_time", Value::Number(std::cmp::max(1, reading_time).into()));
        }
        
        if let Some(is_published) = request.is_published {
            update_data.insert("is_published", Value::Bool(is_published));
        }
        
        if let Some(parent_id) = request.parent_id {
            update_data.insert("parent_id", Value::String(parent_id));
        }
        
        if let Some(sort_order) = request.sort_order {
            update_data.insert("sort_order", Value::Number(sort_order.into()));
        }
        
        if let Some(metadata) = request.metadata {
            update_data.insert("metadata", serde_json::to_value(metadata)?);
        }

        update_data.insert("last_editor_id", Value::String(user.id.clone()));
        update_data.insert("updated_at", Value::String(chrono::Utc::now().to_rfc3339()));

        // 执行更新
        let updated_document: Option<Document> = self.db.client
            .query("UPDATE document SET $data WHERE space_id = $space_id AND slug = $slug RETURN AFTER")
            .bind(("data", update_data))
            .bind(("space_id", format!("space:{}", space.id)))
            .bind(("slug", doc_slug))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "AFTER"))?;

        let updated_document = updated_document.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Failed to update document"))
        })?;

        // 更新搜索索引
        self.update_search_index(&updated_document).await?;

        info!("Updated document: {} in space: {} by user: {}", doc_slug, space_slug, user.id);

        // 记录活动日志
        self.log_activity(&user.id, "document_updated", "document", &updated_document.id.as_ref().unwrap_or(&String::new())).await?;

        // 生成响应
        let mut response = DocumentResponse::from(updated_document);
        response.rendered_content = Some(self.markdown_processor.render(&response.content)?);
        response.breadcrumbs = Some(self.generate_breadcrumbs(&response.id, &space.id).await?);

        Ok(response)
    }

    /// 删除文档
    pub async fn delete_document(&self, space_slug: &str, doc_slug: &str, user: &User) -> Result<()> {
        // 获取空间和文档
        let space = self.get_space_by_slug(space_slug).await?;
        let existing_doc = self.get_document_by_slug(space_slug, doc_slug, Some(user)).await?;

        // 检查权限
        if existing_doc.author_id != user.id && !self.can_manage_space(&space, user) {
            return Err(AppError::Authorization("Only document author or space owner can delete document".to_string()));
        }

        // 检查是否有子文档
        let child_count: Option<u32> = self.db.client
            .query("SELECT count() FROM document WHERE parent_id = $doc_id")
            .bind(("doc_id", format!("document:{}", existing_doc.id)))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "count"))?;

        if let Some(count) = child_count {
            if count > 0 {
                return Err(AppError::Conflict("Cannot delete document with child documents".to_string()));
            }
        }

        // 删除相关数据
        let doc_id = format!("document:{}", existing_doc.id);

        // 删除评论
        let _: Vec<Value> = self.db.client
            .query("DELETE comment WHERE document_id = $doc_id")
            .bind(("doc_id", &doc_id))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        // 删除搜索索引
        let _: Vec<Value> = self.db.client
            .query("DELETE search_index WHERE document_id = $doc_id")
            .bind(("doc_id", &doc_id))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        // 删除文档
        let _: Option<Document> = self.db.client
            .query("DELETE document WHERE space_id = $space_id AND slug = $slug")
            .bind(("space_id", format!("space:{}", space.id)))
            .bind(("slug", doc_slug))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        info!("Deleted document: {} in space: {} by user: {}", doc_slug, space_slug, user.id);

        // 记录活动日志
        self.log_activity(&user.id, "document_deleted", "document", &existing_doc.id).await?;

        Ok(())
    }

    /// 获取文档树结构
    pub async fn get_document_tree(&self, space_slug: &str, user: Option<&User>) -> Result<Vec<DocumentTreeNode>> {
        let space = self.get_space_by_slug(space_slug).await?;
        self.check_read_permission(&space, user).await?;

        // 递归构建文档树
        self.build_document_tree(&space.id, None, user).await
    }

    // 私有辅助方法

    async fn get_space_by_slug(&self, slug: &str) -> Result<crate::models::space::Space> {
        let space: Option<crate::models::space::Space> = self.db.client
            .query("SELECT * FROM space WHERE slug = $slug")
            .bind(("slug", slug))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        space.ok_or_else(|| AppError::NotFound("Space not found".to_string()))
    }

    async fn check_read_permission(&self, space: &crate::models::space::Space, user: Option<&User>) -> Result<()> {
        if !space.can_access(user.map(|u| u.id.as_str())) {
            return Err(AppError::Authorization("Access denied to this space".to_string()));
        }
        Ok(())
    }

    async fn check_write_permission(&self, space: &crate::models::space::Space, user: &User) -> Result<()> {
        // 检查用户是否有文档写入权限
        if !user.permissions.contains(&"docs.write".to_string()) && !user.permissions.contains(&"docs.admin".to_string()) {
            return Err(AppError::Authorization("Write permission required".to_string()));
        }

        // 私有空间只有所有者可以写入
        if !space.is_public && space.owner_id != user.id {
            return Err(AppError::Authorization("Only space owner can write to private space".to_string()));
        }

        Ok(())
    }

    fn can_manage_space(&self, space: &crate::models::space::Space, user: &User) -> bool {
        space.owner_id == user.id || user.permissions.contains(&"docs.admin".to_string())
    }

    async fn document_slug_exists(&self, space_id: &str, slug: &str) -> Result<bool> {
        let existing: Option<Document> = self.db.client
            .query("SELECT id FROM document WHERE space_id = $space_id AND slug = $slug LIMIT 1")
            .bind(("space_id", format!("space:{}", space_id)))
            .bind(("slug", slug))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        Ok(existing.is_some())
    }

    async fn verify_parent_document(&self, space_id: &str, parent_id: &str) -> Result<()> {
        let parent: Option<Document> = self.db.client
            .query("SELECT id FROM document WHERE space_id = $space_id AND id = $parent_id LIMIT 1")
            .bind(("space_id", format!("space:{}", space_id)))
            .bind(("parent_id", parent_id))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        if parent.is_none() {
            return Err(AppError::NotFound("Parent document not found".to_string()));
        }

        Ok(())
    }

    async fn update_search_index(&self, document: &Document) -> Result<()> {
        let doc_id = document.id.as_ref().unwrap_or(&String::new());
        let space_id = document.space_id.strip_prefix("space:").unwrap_or(&document.space_id);

        // 提取关键词（简化版）
        let keywords: Vec<String> = document.content
            .split_whitespace()
            .filter(|word| word.len() > 3)
            .take(20)
            .map(|s| s.to_lowercase())
            .collect();

        let search_data = serde_json::json!({
            "document_id": format!("document:{}", doc_id),
            "space_id": format!("space:{}", space_id),
            "title": document.title,
            "content": document.content,
            "keywords": keywords,
            "updated_at": chrono::Utc::now()
        });

        // 更新或插入搜索索引
        let _: Option<Value> = self.db.client
            .query("UPDATE search_index SET $data WHERE document_id = $doc_id OR CREATE search_index CONTENT $data")
            .bind(("data", search_data))
            .bind(("doc_id", format!("document:{}", doc_id)))
            .await
            .map_err(|e| {
                warn!("Failed to update search index: {}", e);
                e
            })
            .ok()
            .and_then(|mut result| result.pop());

        Ok(())
    }

    async fn generate_breadcrumbs(&self, doc_id: &str, space_id: &str) -> Result<Vec<BreadcrumbItem>> {
        let mut breadcrumbs = Vec::new();
        let mut current_id = Some(doc_id.to_string());

        while let Some(id) = current_id {
            let doc: Option<Document> = self.db.client
                .query("SELECT id, title, slug, parent_id FROM document WHERE id = $id")
                .bind(("id", format!("document:{}", id)))
                .await
                .map_err(|e| AppError::Database(e))?
                .take(0)?;

            if let Some(doc) = doc {
                breadcrumbs.push(BreadcrumbItem {
                    id: doc.id.unwrap_or_default(),
                    title: doc.title,
                    slug: doc.slug,
                });
                current_id = doc.parent_id;
            } else {
                break;
            }
        }

        breadcrumbs.reverse();
        Ok(breadcrumbs)
    }

    async fn get_child_documents(&self, parent_id: &str) -> Result<Vec<DocumentResponse>> {
        let children: Vec<Document> = self.db.client
            .query("SELECT * FROM document WHERE parent_id = $parent_id ORDER BY sort_order ASC")
            .bind(("parent_id", format!("document:{}", parent_id)))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        let mut child_responses = Vec::new();
        for child in children {
            let mut response = DocumentResponse::from(child);
            response.rendered_content = Some(self.markdown_processor.render(&response.content)?);
            child_responses.push(response);
        }

        Ok(child_responses)
    }

    async fn build_document_tree(&self, space_id: &str, parent_id: Option<&str>, user: Option<&User>) -> Result<Vec<DocumentTreeNode>> {
        let parent_condition = if let Some(parent_id) = parent_id {
            format!("parent_id = document:{}", parent_id)
        } else {
            "parent_id = NONE".to_string()
        };

        let documents: Vec<Document> = self.db.client
            .query(&format!("SELECT id, title, slug, is_published, sort_order FROM document WHERE space_id = space:{} AND {} ORDER BY sort_order ASC", space_id, parent_condition))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        let mut nodes = Vec::new();
        for doc in documents {
            // 权限过滤
            if !doc.is_published && user.map(|u| &u.id) != Some(&doc.author_id) {
                continue;
            }

            let doc_id = doc.id.as_ref().unwrap();
            let children = self.build_document_tree(space_id, Some(doc_id), user).await?;

            nodes.push(DocumentTreeNode {
                id: doc_id.clone(),
                title: doc.title,
                slug: doc.slug,
                is_published: doc.is_published,
                sort_order: doc.sort_order,
                children,
            });
        }

        Ok(nodes)
    }

    async fn log_activity(&self, user_id: &str, action: &str, resource_type: &str, resource_id: &str) -> Result<()> {
        let activity = serde_json::json!({
            "user_id": user_id,
            "action": action,
            "resource_type": resource_type,
            "resource_id": resource_id,
            "metadata": {},
            "created_at": chrono::Utc::now()
        });

        let _: Option<Value> = self.db.client
            .create("activity_log")
            .content(activity)
            .await
            .map_err(|e| {
                warn!("Failed to log activity: {}", e);
                e
            })
            .ok()
            .and_then(|mut result| result.pop());

        Ok(())
    }
}