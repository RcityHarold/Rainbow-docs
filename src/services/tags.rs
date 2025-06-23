use std::sync::Arc;
use surrealdb::sql::Thing;
use validator::Validate;

use crate::{
    error::ApiError,
    models::tag::{Tag, DocumentTag, CreateTagRequest, UpdateTagRequest, TagDocumentRequest},
    services::{auth::AuthService, database::Database},
};

#[derive(Clone)]
pub struct TagService {
    db: Arc<Database>,
    auth_service: Arc<AuthService>,
}

impl TagService {
    pub fn new(db: Arc<Database>, auth_service: Arc<AuthService>) -> Self {
        Self { db, auth_service }
    }

    pub async fn create_tag(
        &self,
        creator_id: &str,
        request: CreateTagRequest,
    ) -> Result<Tag, ApiError> {
        request.validate()?;

        // 检查标签名在空间内是否唯一
        if self.tag_exists_in_space(&request.space_id, &request.name).await? {
            return Err(ApiError::Conflict("Tag name already exists in this space".to_string()));
        }

        let space_thing = if let Some(space_id) = &request.space_id {
            Some(Thing::from(("space", space_id.as_str())))
        } else {
            None
        };

        let tag = Tag::new(request.name, request.color, creator_id.to_string())
            .with_description(request.description.unwrap_or_default())
            .with_space(space_thing);

        let created: Vec<Tag> = self.db
            .create("tag")
            .content(tag)
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        created
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::InternalServerError("Failed to create tag".to_string()))
    }

    pub async fn get_tag(&self, tag_id: &str) -> Result<Tag, ApiError> {
        let tag: Option<Tag> = self.db
            .select(("tag", tag_id))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        tag.ok_or_else(|| ApiError::NotFound("Tag not found".to_string()))
    }

    pub async fn update_tag(
        &self,
        tag_id: &str,
        updater_id: &str,
        request: UpdateTagRequest,
    ) -> Result<Tag, ApiError> {
        request.validate()?;

        let mut tag = self.get_tag(tag_id).await?;

        if let Some(name) = request.name {
            // 检查新名称是否与其他标签冲突
            if name != tag.name && self.tag_exists_in_space(&tag.space_id.as_ref().map(|s| s.to_string()), &name).await? {
                return Err(ApiError::Conflict("Tag name already exists in this space".to_string()));
            }
            tag.name = name;
            tag.slug = Tag::generate_slug(&tag.name);
        }

        if let Some(description) = request.description {
            tag.description = Some(description);
        }

        if let Some(color) = request.color {
            tag.color = color;
        }

        tag.updated_at = surrealdb::sql::Datetime::default();

        let updated: Option<Tag> = self.db
            .update(("tag", tag_id))
            .content(tag)
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        updated.ok_or_else(|| ApiError::InternalServerError("Failed to update tag".to_string()))
    }

    pub async fn delete_tag(&self, tag_id: &str) -> Result<(), ApiError> {
        // 首先删除所有与此标签关联的文档标签关系
        let query = "DELETE document_tag WHERE tag_id = $tag_id";
        let _: Vec<surrealdb::sql::Value> = self.db
            .query(query)
            .bind(("tag_id", Thing::from(("tag", tag_id))))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 删除标签
        let _: Option<Tag> = self.db
            .delete(("tag", tag_id))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn get_tags_by_space(
        &self,
        space_id: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<Vec<Tag>, ApiError> {
        let offset = (page - 1) * per_page;
        
        let query = if let Some(space_id) = space_id {
            "SELECT * FROM tag WHERE space_id = $space_id ORDER BY usage_count DESC, name ASC LIMIT $limit START $offset"
        } else {
            "SELECT * FROM tag WHERE space_id IS NULL ORDER BY usage_count DESC, name ASC LIMIT $limit START $offset"
        };

        let mut db_query = self.db.query(query);
        
        if let Some(space_id) = space_id {
            db_query = db_query.bind(("space_id", Thing::from(("space", space_id))));
        }

        let tags: Vec<Tag> = db_query
            .bind(("limit", per_page))
            .bind(("offset", offset))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(tags)
    }

    pub async fn get_popular_tags(&self, space_id: Option<&str>, limit: i64) -> Result<Vec<Tag>, ApiError> {
        let query = if let Some(space_id) = space_id {
            "SELECT * FROM tag WHERE space_id = $space_id AND usage_count > 0 ORDER BY usage_count DESC LIMIT $limit"
        } else {
            "SELECT * FROM tag WHERE space_id IS NULL AND usage_count > 0 ORDER BY usage_count DESC LIMIT $limit"
        };

        let mut db_query = self.db.query(query);
        
        if let Some(space_id) = space_id {
            db_query = db_query.bind(("space_id", Thing::from(("space", space_id))));
        }

        let tags: Vec<Tag> = db_query
            .bind(("limit", limit))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(tags)
    }

    pub async fn search_tags(&self, space_id: Option<&str>, query: &str, limit: i64) -> Result<Vec<Tag>, ApiError> {
        let search_query = if let Some(space_id) = space_id {
            "SELECT * FROM tag WHERE space_id = $space_id AND (name CONTAINSTEXT $query OR description CONTAINSTEXT $query) ORDER BY usage_count DESC LIMIT $limit"
        } else {
            "SELECT * FROM tag WHERE space_id IS NULL AND (name CONTAINSTEXT $query OR description CONTAINSTEXT $query) ORDER BY usage_count DESC LIMIT $limit"
        };

        let mut db_query = self.db.query(search_query);
        
        if let Some(space_id) = space_id {
            db_query = db_query.bind(("space_id", Thing::from(("space", space_id))));
        }

        let tags: Vec<Tag> = db_query
            .bind(("query", query))
            .bind(("limit", limit))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(tags)
    }

    // 文档标签关联管理
    pub async fn tag_document(
        &self,
        tagger_id: &str,
        request: TagDocumentRequest,
    ) -> Result<Vec<DocumentTag>, ApiError> {
        let document_thing = Thing::from(("document", request.document_id.as_str()));
        let mut created_tags = Vec::new();

        for tag_id in request.tag_ids {
            // 检查关联是否已存在
            if !self.document_tag_exists(&request.document_id, &tag_id).await? {
                let tag_thing = Thing::from(("tag", tag_id.as_str()));
                
                let document_tag = DocumentTag::new(
                    document_thing.clone(),
                    tag_thing.clone(),
                    tagger_id.to_string(),
                );

                let created: Vec<DocumentTag> = self.db
                    .create("document_tag")
                    .content(document_tag)
                    .await
                    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

                if let Some(created_tag) = created.into_iter().next() {
                    created_tags.push(created_tag);
                    
                    // 增加标签使用计数
                    self.increment_tag_usage(&tag_id).await?;
                }
            }
        }

        Ok(created_tags)
    }

    pub async fn untag_document(
        &self,
        document_id: &str,
        tag_id: &str,
    ) -> Result<(), ApiError> {
        let query = "DELETE document_tag WHERE document_id = $document_id AND tag_id = $tag_id";
        
        let _: Vec<surrealdb::sql::Value> = self.db
            .query(query)
            .bind(("document_id", Thing::from(("document", document_id))))
            .bind(("tag_id", Thing::from(("tag", tag_id))))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 减少标签使用计数
        self.decrement_tag_usage(tag_id).await?;

        Ok(())
    }

    pub async fn get_document_tags(&self, document_id: &str) -> Result<Vec<Tag>, ApiError> {
        let query = "
            SELECT tag.* FROM tag, document_tag 
            WHERE document_tag.document_id = $document_id 
            AND document_tag.tag_id = tag.id
            ORDER BY tag.name ASC
        ";

        let tags: Vec<Tag> = self.db
            .query(query)
            .bind(("document_id", Thing::from(("document", document_id))))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(tags)
    }

    pub async fn get_documents_by_tag(
        &self,
        tag_id: &str,
        page: i64,
        per_page: i64,
    ) -> Result<Vec<String>, ApiError> {
        let offset = (page - 1) * per_page;
        
        let query = "
            SELECT document_tag.document_id FROM document_tag 
            WHERE document_tag.tag_id = $tag_id
            ORDER BY document_tag.tagged_at DESC
            LIMIT $limit START $offset
        ";

        let results: Vec<surrealdb::sql::Value> = self.db
            .query(query)
            .bind(("tag_id", Thing::from(("tag", tag_id))))
            .bind(("limit", per_page))
            .bind(("offset", offset))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let document_ids: Vec<String> = results
            .into_iter()
            .filter_map(|v| v.as_string())
            .collect();

        Ok(document_ids)
    }

    pub async fn get_tag_statistics(&self, space_id: Option<&str>) -> Result<TagStatistics, ApiError> {
        let total_query = if let Some(space_id) = space_id {
            "SELECT count() as total FROM tag WHERE space_id = $space_id GROUP ALL"
        } else {
            "SELECT count() as total FROM tag WHERE space_id IS NULL GROUP ALL"
        };

        let mut db_query = self.db.query(total_query);
        if let Some(space_id) = space_id {
            db_query = db_query.bind(("space_id", Thing::from(("space", space_id))));
        }

        let total_result: Vec<surrealdb::sql::Value> = db_query
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let total_tags = total_result
            .first()
            .and_then(|v| v.as_int())
            .unwrap_or(0);

        let used_query = if let Some(space_id) = space_id {
            "SELECT count() as used FROM tag WHERE space_id = $space_id AND usage_count > 0 GROUP ALL"
        } else {
            "SELECT count() as used FROM tag WHERE space_id IS NULL AND usage_count > 0 GROUP ALL"
        };

        let mut db_query = self.db.query(used_query);
        if let Some(space_id) = space_id {
            db_query = db_query.bind(("space_id", Thing::from(("space", space_id))));
        }

        let used_result: Vec<surrealdb::sql::Value> = db_query
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let used_tags = used_result
            .first()
            .and_then(|v| v.as_int())
            .unwrap_or(0);

        Ok(TagStatistics {
            total_tags,
            used_tags,
            unused_tags: total_tags - used_tags,
            most_used_tags: self.get_popular_tags(space_id, 5).await?,
        })
    }

    // 私有辅助方法
    async fn tag_exists_in_space(&self, space_id: &Option<String>, name: &str) -> Result<bool, ApiError> {
        let query = if let Some(space_id) = space_id {
            "SELECT count() FROM tag WHERE space_id = $space_id AND name = $name GROUP ALL"
        } else {
            "SELECT count() FROM tag WHERE space_id IS NULL AND name = $name GROUP ALL"
        };

        let mut db_query = self.db.query(query);
        if let Some(space_id) = space_id {
            db_query = db_query.bind(("space_id", Thing::from(("space", space_id.as_str()))));
        }

        let result: Vec<surrealdb::sql::Value> = db_query
            .bind(("name", name))
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

    async fn document_tag_exists(&self, document_id: &str, tag_id: &str) -> Result<bool, ApiError> {
        let query = "SELECT count() FROM document_tag WHERE document_id = $document_id AND tag_id = $tag_id GROUP ALL";

        let result: Vec<surrealdb::sql::Value> = self.db
            .query(query)
            .bind(("document_id", Thing::from(("document", document_id))))
            .bind(("tag_id", Thing::from(("tag", tag_id))))
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

    async fn increment_tag_usage(&self, tag_id: &str) -> Result<(), ApiError> {
        let query = "UPDATE tag SET usage_count += 1 WHERE id = $tag_id";

        let _: Vec<surrealdb::sql::Value> = self.db
            .query(query)
            .bind(("tag_id", Thing::from(("tag", tag_id))))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn decrement_tag_usage(&self, tag_id: &str) -> Result<(), ApiError> {
        let query = "UPDATE tag SET usage_count = math::max(usage_count - 1, 0) WHERE id = $tag_id";

        let _: Vec<surrealdb::sql::Value> = self.db
            .query(query)
            .bind(("tag_id", Thing::from(("tag", tag_id))))
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct TagStatistics {
    pub total_tags: i64,
    pub used_tags: i64,
    pub unused_tags: i64,
    pub most_used_tags: Vec<Tag>,
}