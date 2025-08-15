use crate::config::Config;
use crate::error::{AppError, Result};
use crate::models::space::{
    Space, SpaceResponse, SpaceListResponse, SpaceListQuery, SpaceStats,
    CreateSpaceRequest, UpdateSpaceRequest
};
use crate::services::auth::User;
use crate::services::database::Database;
use serde_json::Value;
use std::sync::Arc;
use surrealdb::sql::Thing;
use tracing::{info, warn, error, debug};
use validator::Validate;

pub struct SpaceService {
    db: Arc<Database>,
}

impl SpaceService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 创建新的文档空间
    pub async fn create_space(&self, request: CreateSpaceRequest, user: &User) -> Result<SpaceResponse> {
        // 验证输入
        request.validate().map_err(|e| AppError::Validation(e.to_string()))?;

        // 检查slug是否已存在（全局唯一）
        if self.slug_exists(&request.slug).await? {
            return Err(AppError::Conflict("Space slug already exists globally. Please choose a different slug.".to_string()));
        }

        // 创建空间对象
        let mut space = Space::new(
            request.name,
            request.slug.clone(),
            user.id.clone(),
        );

        if let Some(description) = request.description {
            space.description = Some(description);
        }

        if let Some(avatar_url) = request.avatar_url {
            space.avatar_url = Some(avatar_url);
        }

        if let Some(is_public) = request.is_public {
            space.is_public = is_public;
        }

        if let Some(settings) = request.settings {
            space.settings = settings;
        }

        // 保存到数据库
        let created_spaces: Vec<Space> = self.db.client
            .create("space")
            .content(space)
            .await
            .map_err(|e| {
                error!("Failed to create space: {}", e);
                AppError::Database(e)
            })?;

        let created_space = created_spaces.into_iter().next();

        let created_space = created_space.ok_or_else(|| {
            error!("Failed to get created space from database");
            AppError::Internal(anyhow::anyhow!("Failed to create space"))
        })?;

        info!("Created new space: {} by user: {}", request.slug, user.id);

        // 记录活动日志
        self.log_activity(&user.id, "space_created", "space", &created_space.id.as_ref().unwrap_or(&String::new())).await?;

        Ok(SpaceResponse::from(created_space))
    }

    /// 获取空间列表
    pub async fn list_spaces(&self, query: SpaceListQuery, user: Option<&User>) -> Result<SpaceListResponse> {
        let page = query.page.unwrap_or(1);
        let limit = query.limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        // 构建查询条件
        let mut where_conditions = Vec::new();
        let mut params: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();

        // 权限过滤：只显示用户拥有的空间或用户加入的空间
        // 公开空间应该通过直接链接访问，而不是在列表中显示
        if let Some(user) = user {
            info!("Listing spaces for user: {}", user.id);
            
            // 由于SurrealDB子查询语法问题，分两步获取：
            // 1. 先获取用户拥有的空间
            // 2. 再获取用户加入的空间，然后合并
            
            // 这里先查询用户拥有的空间
            where_conditions.push("owner_id = $user_id");
            params.insert("user_id".to_string(), user.id.clone().into());
        } else {
            // 未登录用户看不到任何空间列表
            where_conditions.push("1 = 0");
        }

        // 基础过滤条件
        where_conditions.push("is_deleted = false");

        // 搜索过滤
        if let Some(search) = &query.search {
            where_conditions.push("(string::lowercase(name) CONTAINS string::lowercase($search) OR string::lowercase(description) CONTAINS string::lowercase($search))");
            params.insert("search".to_string(), search.clone().into());
        }

        // 所有者过滤
        if let Some(owner_id) = &query.owner_id {
            where_conditions.push("owner_id = $owner_id");
            params.insert("owner_id".to_string(), owner_id.clone().into());
        }

        // 公开性过滤
        if let Some(is_public) = query.is_public {
            where_conditions.push("is_public = $is_public");
            params.insert("is_public".to_string(), is_public.into());
        }

        let where_clause = if where_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_conditions.join(" AND "))
        };

        // 排序
        let sort_field = query.sort.unwrap_or_else(|| "updated_at".to_string());
        let sort_order = query.order.unwrap_or_else(|| "desc".to_string());
        let order_clause = format!("ORDER BY {} {}", sort_field, sort_order);

        // 查询总数
        let count_query = format!("SELECT count() AS total FROM space {} GROUP ALL", where_clause);
        let count_result: Vec<serde_json::Value> = self.db.client
            .query(&count_query)
            .bind(params.clone())
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;
            
        let total = count_result.first()
            .and_then(|v| v.get("total"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let total_pages = (total + limit - 1) / limit;

        // 查询数据
        let data_query = format!(
            "SELECT * FROM space {} {} LIMIT {} START {}",
            where_clause, order_clause, limit, offset
        );

        info!("Executing space list query: {}", data_query);
        info!("Query params: {:?}", params);

        // 首先获取数据库格式的数据
        let spaces_db: Vec<crate::models::space::SpaceDb> = self.db.client
            .query(&data_query)
            .bind(params)
            .await
            .map_err(|e| {
                error!("Failed to execute space list query: {}", e);
                AppError::Database(e)
            })?
            .take(0)?;
        
        // 转换为 Space 类型
        let mut spaces: Vec<Space> = spaces_db.into_iter()
            .map(|db| db.into())
            .collect();

        // 如果是登录用户，还需要添加用户作为成员的空间
        if let Some(user) = user {
            let member_spaces = self.get_user_member_spaces(&user.id).await?;
            info!("Found {} member spaces for user {}", member_spaces.len(), user.id);
            
            // 合并空间，避免重复
            let existing_ids: std::collections::HashSet<String> = spaces.iter()
                .filter_map(|s| s.id.clone())
                .collect();
            
            for member_space in member_spaces {
                if let Some(space_id) = &member_space.id {
                    if !existing_ids.contains(space_id) {
                        spaces.push(member_space);
                    }
                }
            }
        }

        // 转换为响应格式
        let mut space_responses = Vec::new();
        for space in spaces {
            let mut response = SpaceResponse::from(space);
            // 获取空间统计信息
            if let Ok(stats) = self.get_space_stats(&response.id).await {
                response.stats = Some(stats);
            }
            space_responses.push(response);
        }

        debug!("Listed {} spaces for user: {:?}", space_responses.len(), user.map(|u| &u.id));

        Ok(SpaceListResponse {
            spaces: space_responses,
            total,
            page,
            limit,
            total_pages,
        })
    }

    /// 根据slug获取空间详情
    pub async fn get_space_by_slug(&self, slug: &str, user: Option<&User>) -> Result<SpaceResponse> {
        let space_db: Option<crate::models::space::SpaceDb> = self.db.client
            .query("SELECT * FROM space WHERE slug = $slug AND is_deleted = false")
            .bind(("slug", slug))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        let space_db = space_db.ok_or_else(|| AppError::NotFound("Space not found".to_string()))?;
        let space: Space = space_db.into();

        // 检查基础访问权限（公开性和所有者）
        if !space.can_access(user.map(|u| u.id.as_str())) {
            // 注意：这里应该集成SpaceMemberService的权限检查
            // 但为了避免循环依赖，建议在调用方进行额外的成员权限检查
            return Err(AppError::Authorization("Access denied to this space".to_string()));
        }

        let mut response = SpaceResponse::from(space);
        
        // 获取统计信息
        if let Ok(stats) = self.get_space_stats(&response.id).await {
            response.stats = Some(stats);
        }

        debug!("Retrieved space: {} for user: {:?}", slug, user.map(|u| &u.id));

        Ok(response)
    }

    /// 根据ID获取空间详情
    pub async fn get_space_by_id(&self, id: &str, user: Option<&User>) -> Result<SpaceResponse> {
        let query_id = format!("space:{}", id);
        info!("get_space_by_id: searching for id = {}", query_id);
        
        let space_db: Option<crate::models::space::SpaceDb> = self.db.client
            .query("SELECT * FROM $id WHERE is_deleted = false")
            .bind(("id", Thing::from(("space", id))))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        info!("get_space_by_id: query result = {:?}", space_db.is_some());
        let space_db = space_db.ok_or_else(|| AppError::NotFound("Space not found".to_string()))?;
        let space: Space = space_db.into();

        // 检查基础访问权限（公开性和所有者）
        if !space.can_access(user.map(|u| u.id.as_str())) {
            // 注意：这里应该集成SpaceMemberService的权限检查
            // 但为了避免循环依赖，建议在调用方进行额外的成员权限检查
            return Err(AppError::Authorization("Access denied to this space".to_string()));
        }

        let mut response = SpaceResponse::from(space);
        
        // 获取统计信息
        if let Ok(stats) = self.get_space_stats(&response.id).await {
            response.stats = Some(stats);
        }

        debug!("Retrieved space by ID: {} for user: {:?}", id, user.map(|u| &u.id));

        Ok(response)
    }

    /// 更新空间信息
    pub async fn update_space(&self, slug: &str, request: UpdateSpaceRequest, user: &User) -> Result<SpaceResponse> {
        // 验证输入
        request.validate().map_err(|e| AppError::Validation(e.to_string()))?;

        // 获取现有空间
        let existing_space = self.get_space_by_slug(slug, Some(user)).await?;

        // 检查权限：只有所有者可以更新
        if existing_space.owner_id != user.id {
            return Err(AppError::Authorization("Only space owner can update space".to_string()));
        }

        // 构建更新数据
        let mut update_data = std::collections::HashMap::new();
        
        if let Some(name) = request.name {
            update_data.insert("name", Value::String(name));
        }
        
        if let Some(description) = request.description {
            update_data.insert("description", Value::String(description));
        }
        
        if let Some(avatar_url) = request.avatar_url {
            update_data.insert("avatar_url", Value::String(avatar_url));
        }
        
        if let Some(is_public) = request.is_public {
            update_data.insert("is_public", Value::Bool(is_public));
        }
        
        if let Some(settings) = request.settings {
            update_data.insert("settings", serde_json::to_value(settings)?);
        }

        update_data.insert("updated_at", Value::String(chrono::Utc::now().to_rfc3339()));

        // 执行更新
        let updated_space_db: Option<crate::models::space::SpaceDb> = self.db.client
            .query("UPDATE space SET $data WHERE slug = $slug RETURN AFTER")
            .bind(("data", update_data))
            .bind(("slug", slug))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "AFTER"))?;

        let updated_space_db = updated_space_db.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Failed to update space"))
        })?;
        let updated_space: Space = updated_space_db.into();

        info!("Updated space: {} by user: {}", slug, user.id);

        // 记录活动日志
        self.log_activity(&user.id, "space_updated", "space", &updated_space.id.as_ref().unwrap_or(&String::new())).await?;

        Ok(SpaceResponse::from(updated_space))
    }

    /// 删除空间
    pub async fn delete_space(&self, slug: &str, user: &User) -> Result<()> {
        // 获取现有空间
        let existing_space = self.get_space_by_slug(slug, Some(user)).await?;

        // 检查权限：只有所有者可以删除
        if existing_space.owner_id != user.id {
            return Err(AppError::Authorization("Only space owner can delete space".to_string()));
        }

        // 检查空间是否有文档
        let doc_count: Option<u32> = self.db.client
            .query("SELECT count() FROM document WHERE space_id = $space_id")
            .bind(("space_id", format!("space:{}", existing_space.id)))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "count"))?;

        if let Some(count) = doc_count {
            if count > 0 {
                return Err(AppError::Conflict("Cannot delete space with existing documents".to_string()));
            }
        }

        // 删除空间
        let _: Option<crate::models::space::SpaceDb> = self.db.client
            .query("DELETE space WHERE slug = $slug")
            .bind(("slug", slug))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        info!("Deleted space: {} by user: {}", slug, user.id);

        // 记录活动日志
        self.log_activity(&user.id, "space_deleted", "space", &existing_space.id).await?;

        Ok(())
    }

    /// 检查slug是否已存在（全局检查）
    async fn slug_exists(&self, slug: &str) -> Result<bool> {
        let existing: Option<crate::models::space::SpaceDb> = self.db.client
            .query("SELECT id FROM space WHERE slug = $slug AND is_deleted = false LIMIT 1")
            .bind(("slug", slug))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        Ok(existing.is_some())
    }

    /// 获取空间统计信息
    async fn get_space_stats(&self, space_id: &str) -> Result<SpaceStats> {
        // 查询文档数量
        let doc_count: Option<u32> = self.db.client
            .query("SELECT count() FROM document WHERE space_id = $space_id")
            .bind(("space_id", format!("space:{}", space_id)))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "count"))?;

        // 查询公开文档数量
        let public_doc_count: Option<u32> = self.db.client
            .query("SELECT count() FROM document WHERE space_id = $space_id AND is_public = true")
            .bind(("space_id", format!("space:{}", space_id)))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "count"))?;

        // 查询评论数量
        let comment_count: Option<u32> = self.db.client
            .query("SELECT count() FROM comment WHERE document_id IN (SELECT id FROM document WHERE space_id = $space_id)")
            .bind(("space_id", format!("space:{}", space_id)))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "count"))?;

        // 查询总浏览量
        let view_count: Option<u32> = self.db.client
            .query("SELECT math::sum(view_count) AS total_views FROM document WHERE space_id = $space_id")
            .bind(("space_id", format!("space:{}", space_id)))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "total_views"))?;

        // 查询最后活动时间
        let last_activity: Option<String> = self.db.client
            .query("SELECT updated_at FROM document WHERE space_id = $space_id ORDER BY updated_at DESC LIMIT 1")
            .bind(("space_id", format!("space:{}", space_id)))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "updated_at"))?;

        let last_activity = last_activity
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        Ok(SpaceStats {
            document_count: doc_count.unwrap_or(0),
            public_document_count: public_doc_count.unwrap_or(0),
            comment_count: comment_count.unwrap_or(0),
            view_count: view_count.unwrap_or(0),
            last_activity,
        })
    }

    /// 获取用户作为成员的空间列表
    async fn get_user_member_spaces(&self, user_id: &str) -> Result<Vec<Space>> {
        info!("Getting member spaces for user: {}", user_id);
        
        // 清理user_id格式，去掉user:前缀以匹配数据库存储格式
        let clean_user_id = if user_id.starts_with("user:") {
            user_id.strip_prefix("user:").unwrap_or(user_id)
        } else {
            user_id
        };
        info!("Querying member spaces with cleaned user_id: {} (original: {})", clean_user_id, user_id);
        
        // 查询用户是成员的space_id列表
        let member_query = "SELECT space_id FROM space_member WHERE user_id = $user_id AND status = 'accepted'";
        let member_results: Vec<serde_json::Value> = self.db.client
            .query(member_query)
            .bind(("user_id", clean_user_id))
            .await
            .map_err(|e| {
                error!("Failed to query space members: {}", e);
                AppError::Database(e)
            })?
            .take(0)?;
            
        // 如果没有找到结果，尝试查看所有space_member记录进行调试
        if member_results.is_empty() {
            info!("No member spaces found for cleaned user_id: {} (original: {}), checking all space_member records for debugging", clean_user_id, user_id);
            let all_members: Vec<serde_json::Value> = self.db.client
                .query("SELECT user_id, space_id, status FROM space_member LIMIT 5")
                .await
                .map_err(|e| AppError::Database(e))?
                .take(0)?;
            
            for member in &all_members {
                info!("Found space_member record: {:?}", member);
            }
        }
        
        info!("Found {} space member records for user {}", member_results.len(), user_id);
        
        // 提取space_id列表
        let mut space_ids = Vec::new();
        for result in member_results {
            if let Some(space_id_value) = result.get("space_id") {
                info!("Processing space_id value: {:?}", space_id_value);
                
                // 处理SurrealDB Thing对象格式
                if let Some(space_id_obj) = space_id_value.as_object() {
                    if let Some(id_value) = space_id_obj.get("id") {
                        if let Some(id_obj) = id_value.as_object() {
                            if let Some(actual_id) = id_obj.get("String") {
                                if let Some(space_id_str) = actual_id.as_str() {
                                    space_ids.push(space_id_str.to_string());
                                    info!("Found member space_id: {}", space_id_str);
                                }
                            }
                        }
                    }
                }
                // 也尝试直接字符串格式（向后兼容）
                else if let Some(space_id_str) = space_id_value.as_str() {
                    let clean_id = if space_id_str.starts_with("space:") {
                        space_id_str.strip_prefix("space:").unwrap_or(space_id_str)
                    } else {
                        space_id_str
                    };
                    space_ids.push(clean_id.to_string());
                    info!("Found member space_id (string format): {}", clean_id);
                }
            }
        }
        
        if space_ids.is_empty() {
            info!("No member spaces found for user {}", user_id);
            return Ok(Vec::new());
        }
        
        // 查询对应的space记录
        let mut spaces = Vec::new();
        for space_id in space_ids {
            let space_query = "SELECT * FROM space WHERE id = $space_id AND is_deleted = false";
            let space_results: Vec<crate::models::space::SpaceDb> = self.db.client
                .query(space_query)
                .bind(("space_id", Thing::from(("space", space_id.as_str()))))
                .await
                .map_err(|e| {
                    error!("Failed to query space: {}", e);
                    AppError::Database(e)
                })?
                .take(0)?;
                
            for space_db in space_results {
                spaces.push(space_db.into());
            }
        }
        
        info!("Retrieved {} actual spaces for user {}", spaces.len(), user_id);
        Ok(spaces)
    }

    /// 记录活动日志
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::space::CreateSpaceRequest;

    // 注意：实际测试需要数据库连接，这里只是示例结构
    #[tokio::test]
    async fn test_create_space_validation() {
        let request = CreateSpaceRequest {
            name: "".to_string(), // 无效：空名称
            slug: "test-space".to_string(),
            description: None,
            avatar_url: None,
            is_public: None,
            settings: None,
        };

        assert!(request.validate().is_err());
    }

    #[tokio::test] 
    async fn test_slug_validation() {
        let valid_request = CreateSpaceRequest {
            name: "Test Space".to_string(),
            slug: "test-space".to_string(),
            description: None,
            avatar_url: None,
            is_public: None,
            settings: None,
        };

        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateSpaceRequest {
            name: "Test Space".to_string(),
            slug: "Test Space".to_string(), // 无效：包含空格和大写
            description: None,
            avatar_url: None,
            is_public: None,
            settings: None,
        };

        assert!(invalid_request.validate().is_err());
    }
}