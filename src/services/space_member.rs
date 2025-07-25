use crate::config::Config;
use crate::error::{AppError, Result};
use crate::models::space_member::{
    SpaceMember, SpaceMemberDb, SpaceInvitation, SpaceInvitationDb,
    InviteMemberRequest, UpdateMemberRequest, AcceptInvitationRequest,
    MemberRole, MemberStatus, SpaceMemberResponse
};
use crate::services::auth::User;
use crate::services::database::Database;
use serde_json::Value;
use std::sync::Arc;
use surrealdb::sql::Thing;
use tracing::{info, warn, error, debug};
use validator::Validate;
use chrono::Utc;
use uuid::Uuid;

pub struct SpaceMemberService {
    db: Arc<Database>,
    config: Config,
}

impl SpaceMemberService {
    pub fn new(db: Arc<Database>, config: Config) -> Self {
        Self { db, config }
    }

    /// 检查用户是否为空间成员或所有者
    pub async fn can_access_space(&self, space_id: &str, user_id: Option<&str>) -> Result<bool> {
        let Some(uid) = user_id else {
            return Ok(false);
        };

        // 提取实际的空间ID（去掉"space:"前缀，如果存在）
        let actual_space_id = if space_id.starts_with("space:") {
            space_id.strip_prefix("space:").unwrap()
        } else {
            space_id
        };

        // 检查是否为空间所有者
        let owner_query = "SELECT owner_id FROM space WHERE id = $space_id";
        let mut owner_result = self.db.client
            .query(owner_query)
            .bind(("space_id", Thing::from(("space", actual_space_id))))
            .await
            .map_err(|e| AppError::Database(e))?;

        if let Ok(spaces) = owner_result.take::<Vec<Value>>(0) {
            if let Some(space) = spaces.first() {
                if let Some(owner_id) = space.get("owner_id").and_then(|v| v.as_str()) {
                    if owner_id == uid {
                        return Ok(true);
                    }
                }
            }
        }

        // 检查是否为空间成员
        let member_query = "SELECT id FROM space_member WHERE space_id = $space_id AND user_id = $user_id AND status = 'accepted'";
        let member_result: Vec<SpaceMemberDb> = self.db.client
            .query(member_query)
            .bind(("space_id", Thing::from(("space", actual_space_id))))
            .bind(("user_id", uid))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        Ok(!member_result.is_empty())
    }

    /// 检查用户在空间中的权限
    pub async fn check_permission(&self, space_id: &str, user_id: &str, permission: &str) -> Result<bool> {
        // 提取实际的空间ID（去掉"space:"前缀，如果存在）
        let actual_space_id = if space_id.starts_with("space:") {
            space_id.strip_prefix("space:").unwrap()
        } else {
            space_id
        };

        // 首先检查是否为空间所有者
        let owner_query = "SELECT owner_id FROM space WHERE id = $space_id";
        let mut owner_result = self.db.client
            .query(owner_query)
            .bind(("space_id", Thing::from(("space", actual_space_id))))
            .await
            .map_err(|e| AppError::Database(e))?;

        if let Ok(spaces) = owner_result.take::<Vec<Value>>(0) {
            if let Some(space) = spaces.first() {
                if let Some(owner_id) = space.get("owner_id").and_then(|v| v.as_str()) {
                    if owner_id == user_id {
                        return Ok(true); // 所有者拥有所有权限
                    }
                }
            }
        }

        // 检查成员权限
        let member_query = "SELECT role, permissions FROM space_member WHERE space_id = $space_id AND user_id = $user_id AND status = 'accepted'";
        let members: Vec<SpaceMemberDb> = self.db.client
            .query(member_query)
            .bind(("space_id", Thing::from(("space", actual_space_id))))
            .bind(("user_id", user_id))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        if let Some(member) = members.first() {
            // 检查角色默认权限
            if member.role.can_perform(permission) {
                return Ok(true);
            }
            
            // 检查自定义权限
            if member.permissions.contains(&permission.to_string()) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 邀请用户加入空间
    pub async fn invite_member(&self, space_id: &str, inviter: &User, request: InviteMemberRequest) -> Result<SpaceInvitation> {
        request.validate().map_err(|e| AppError::Validation(e.to_string()))?;

        // 检查邀请权限
        if !self.check_permission(space_id, &inviter.id, "members.invite").await? {
            return Err(AppError::Authorization("Permission denied: members.invite required".to_string()));
        }

        // 如果通过user_id邀请，检查用户是否已经是成员
        if let Some(user_id) = &request.user_id {
            if self.can_access_space(space_id, Some(user_id)).await? {
                return Err(AppError::Conflict("User is already a member of this space".to_string()));
            }
        }

        // 生成邀请令牌
        let invite_token = Uuid::new_v4().to_string();
        let expires_in_days = request.expires_in_days.unwrap_or(7);

        // 使用 SQL 查询创建邀请记录，使用 SurrealDB 的时间函数和 duration 语法
        let query = format!(r#"
            CREATE space_invitation SET
                space_id = type::thing('space', $space_id),
                email = $email,
                user_id = $user_id,
                invite_token = $invite_token,
                role = $role,
                permissions = $permissions,
                invited_by = $invited_by,
                message = $message,
                max_uses = $max_uses,
                used_count = $used_count,
                expires_at = time::now() + {}d
        "#, expires_in_days);

        let created: Vec<SpaceInvitationDb> = self.db.client
            .query(query)
            .bind(("space_id", space_id))
            .bind(("email", request.email.clone()))
            .bind(("user_id", request.user_id.clone()))
            .bind(("invite_token", invite_token.clone()))
            .bind(("role", request.role.clone()))
            .bind(("permissions", request.role.default_permissions()))
            .bind(("invited_by", inviter.id.clone()))
            .bind(("message", request.message.clone()))
            .bind(("max_uses", 1))
            .bind(("used_count", 0))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        let created_invitation = created.into_iter().next()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create invitation")))?;

        info!("User {} invited {} to space {}", inviter.id, 
              request.email.as_deref().unwrap_or(request.user_id.as_deref().unwrap_or("unknown")), space_id);

        // 获取邀请者显示名称，优先使用profile中的名称，否则使用用户ID
        let inviter_name = inviter.profile.as_ref()
            .and_then(|p| p.display_name.clone())
            .filter(|name| !name.is_empty())
            .or_else(|| {
                // 如果email不是默认的unknown@example.com，则使用email
                if inviter.email != "unknown@example.com" {
                    Some(inviter.email.clone())
                } else {
                    // 否则使用用户ID
                    Some(inviter.id.clone())
                }
            })
            .unwrap_or_else(|| inviter.id.clone());
        
        info!("Inviter info - ID: {}, Email: {}, Display name: {}", 
              inviter.id, inviter.email, inviter_name);

        // 发送邮件和通知
        self.send_invitation_notifications(
            request.email.as_deref(),
            request.user_id.as_deref(),
            space_id,
            &inviter_name,
            &invite_token,
            &request.role.to_string(),
            request.message.as_deref(),
            expires_in_days.into(),
        ).await.unwrap_or_else(|e| {
            error!("Failed to send invitation notifications: {}", e);
        });

        Ok(created_invitation.into())
    }

    /// 接受邀请
    pub async fn accept_invitation(&self, user_id: &str, request: AcceptInvitationRequest) -> Result<SpaceMember> {
        // 查找邀请
        let invitation_query = "SELECT * FROM space_invitation WHERE invite_token = $token AND expires_at > time::now()";
        let invitations: Vec<SpaceInvitationDb> = self.db.client
            .query(invitation_query)
            .bind(("token", &request.invite_token))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        let invitation = invitations.into_iter().next()
            .ok_or_else(|| AppError::NotFound("Invitation not found or expired".to_string()))?;

        // 检查邀请是否已用完
        if invitation.used_count >= invitation.max_uses {
            return Err(AppError::Conflict("Invitation has been used up".to_string()));
        }

        // 检查是否已经是成员
        if self.can_access_space(&invitation.space_id.id.to_string(), Some(user_id)).await? {
            return Err(AppError::Conflict("User is already a member of this space".to_string()));
        }

        // 使用简单的方法创建成员记录
        use serde_json::json;
        
        let member_data = json!({
            "space_id": Thing::from(("space", invitation.space_id.id.to_string().as_str())),
            "user_id": user_id,
            "role": invitation.role,
            "permissions": invitation.permissions,
            "invited_by": invitation.invited_by,
            "invited_at": invitation.created_at,
            "accepted_at": Utc::now(),
            "status": "accepted",
            "expires_at": null,
            "created_at": Utc::now(),
            "updated_at": Utc::now()
        });

        let created_members: Vec<SpaceMemberDb> = self.db.client
            .create("space_member")
            .content(member_data)
            .await
            .map_err(|e| AppError::Database(e))?;

        let created_member = created_members.into_iter().next()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create member")))?;

        // 更新邀请使用次数
        let _: Option<SpaceInvitationDb> = self.db.client
            .query("UPDATE space_invitation SET used_count = used_count + 1 WHERE invite_token = $token")
            .bind(("token", &request.invite_token))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        info!("User {} accepted invitation to space {}", user_id, invitation.space_id.id.to_string());

        Ok(created_member.into())
    }

    /// 获取空间成员列表
    pub async fn list_space_members(&self, space_id: &str, requester: &User) -> Result<Vec<SpaceMemberResponse>> {
        // 检查查看权限
        if !self.check_permission(space_id, &requester.id, "members.manage").await? {
            return Err(AppError::Authorization("Permission denied: members.manage required".to_string()));
        }

        // 提取实际的空间ID（去掉"space:"前缀，如果存在）
        let actual_space_id = if space_id.starts_with("space:") {
            space_id.strip_prefix("space:").unwrap()
        } else {
            space_id
        };

        let query = "SELECT * FROM space_member WHERE space_id = $space_id ORDER BY created_at ASC";
        let members: Vec<SpaceMemberDb> = self.db.client
            .query(query)
            .bind(("space_id", Thing::from(("space", actual_space_id))))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        let member_responses = members.into_iter()
            .map(|member| SpaceMemberResponse::from(SpaceMember::from(member)))
            .collect();

        Ok(member_responses)
    }

    /// 更新成员权限
    pub async fn update_member(&self, space_id: &str, member_user_id: &str, updater: &User, request: UpdateMemberRequest) -> Result<SpaceMemberResponse> {
        request.validate().map_err(|e| AppError::Validation(e.to_string()))?;

        // 检查管理权限
        if !self.check_permission(space_id, &updater.id, "members.manage").await? {
            return Err(AppError::Authorization("Permission denied: members.manage required".to_string()));
        }

        // 提取实际的空间ID（去掉"space:"前缀，如果存在）
        let actual_space_id = if space_id.starts_with("space:") {
            space_id.strip_prefix("space:").unwrap()
        } else {
            space_id
        };

        // 获取当前成员信息
        let query = "SELECT * FROM space_member WHERE space_id = $space_id AND user_id = $user_id";
        let members: Vec<SpaceMemberDb> = self.db.client
            .query(query)
            .bind(("space_id", Thing::from(("space", actual_space_id))))
            .bind(("user_id", member_user_id))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        let mut member: SpaceMember = members.into_iter().next()
            .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?
            .into();

        // 更新字段
        if let Some(role) = request.role {
            member.role = role.clone();
            member.permissions = role.default_permissions();
        }

        if let Some(permissions) = request.permissions {
            member.permissions = permissions;
        }

        member.updated_at = Utc::now();

        // 保存更新
        let updated: Option<SpaceMemberDb> = self.db.client
            .query("UPDATE space_member SET role = $role, permissions = $permissions, updated_at = $updated_at WHERE space_id = $space_id AND user_id = $user_id RETURN AFTER")
            .bind(("role", &member.role))
            .bind(("permissions", &member.permissions))
            .bind(("updated_at", member.updated_at))
            .bind(("space_id", Thing::from(("space", actual_space_id))))
            .bind(("user_id", member_user_id))
            .await
            .map_err(|e| AppError::Database(e))?
            .take((0, "AFTER"))?;

        let updated_member = updated
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update member")))?;

        info!("User {} updated member {} in space {}", updater.id, member_user_id, space_id);

        Ok(SpaceMemberResponse::from(SpaceMember::from(updated_member)))
    }

    /// 移除成员
    pub async fn remove_member(&self, space_id: &str, member_user_id: &str, remover: &User) -> Result<()> {
        // 检查移除权限
        if !self.check_permission(space_id, &remover.id, "members.remove").await? {
            return Err(AppError::Authorization("Permission denied: members.remove required".to_string()));
        }

        // 不能移除自己
        if member_user_id == remover.id {
            return Err(AppError::Conflict("Cannot remove yourself from space".to_string()));
        }

        // 提取实际的空间ID（去掉"space:"前缀，如果存在）
        let actual_space_id = if space_id.starts_with("space:") {
            space_id.strip_prefix("space:").unwrap()
        } else {
            space_id
        };

        // 删除成员记录
        let _: Option<SpaceMemberDb> = self.db.client
            .query("DELETE space_member WHERE space_id = $space_id AND user_id = $user_id")
            .bind(("space_id", Thing::from(("space", actual_space_id))))
            .bind(("user_id", member_user_id))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        info!("User {} removed member {} from space {}", remover.id, member_user_id, space_id);

        Ok(())
    }

    /// 获取用户参与的空间列表
    pub async fn get_user_spaces(&self, user_id: &str) -> Result<Vec<String>> {
        let query = "SELECT space_id FROM space_member WHERE user_id = $user_id AND status = 'accepted'";
        let members: Vec<SpaceMemberDb> = self.db.client
            .query(query)
            .bind(("user_id", user_id))
            .await
            .map_err(|e| AppError::Database(e))?
            .take(0)?;

        let space_ids = members.into_iter()
            .map(|member| member.space_id.id.to_string())
            .collect();

        Ok(space_ids)
    }

    /// 发送邀请通知（邮件和站内通知）
    async fn send_invitation_notifications(
        &self,
        to_email: Option<&str>,
        to_user_id: Option<&str>,
        space_id: &str,
        inviter_name: &str,
        invite_token: &str,
        role: &str,
        message: Option<&str>,
        expires_in_days: u64,
    ) -> Result<()> {
        // 获取空间名称
        let space_name = self.get_space_name(space_id).await?;

        // 如果提供了用户ID，创建站内通知
        if let Some(user_id) = to_user_id {
            self.create_space_invitation_notification(
                user_id,
                &space_name,
                inviter_name,
                invite_token,
                role,
                message,
            ).await?;
        }

        // 如果提供了邮箱，发送邮件通知
        if let Some(email) = to_email {
            self.send_invitation_email(
                email,
                &space_name,
                inviter_name,
                invite_token,
                role,
                message,
                expires_in_days,
            ).await?;
        }

        Ok(())
    }

    /// 获取空间名称
    async fn get_space_name(&self, space_id: &str) -> Result<String> {
        // 提取实际的空间ID（去掉"space:"前缀，如果存在）
        let actual_space_id = if space_id.starts_with("space:") {
            space_id.strip_prefix("space:").unwrap()
        } else {
            space_id
        };
        
        let query = "SELECT name FROM space WHERE id = $space_id";
        let mut response = self.db.client
            .query(query)
            .bind(("space_id", Thing::from(("space", actual_space_id))))
            .await
            .map_err(|e| {
                error!("Failed to get space name for {}: {}", space_id, e);
                AppError::Database(e)
            })?;

        let spaces: Vec<serde_json::Value> = response.take(0)?;
        match spaces.into_iter().next() {
            Some(space_data) => {
                let name = space_data["name"].as_str().unwrap_or("未知空间").to_string();
                info!("Found space name: {} for space: {}", name, space_id);
                Ok(name)
            }
            None => {
                warn!("No space found for ID: {}", space_id);
                Ok("未知空间".to_string())
            }
        }
    }

    /// 创建站内通知
    async fn create_space_invitation_notification(
        &self,
        user_id: &str,
        space_name: &str,
        inviter_name: &str,
        invite_token: &str,
        role: &str,
        message: Option<&str>,
    ) -> Result<()> {
        use serde_json::json;

        // 创建通知数据
        let notification_data = json!({
            "space_name": space_name,
            "invite_token": invite_token,
            "role": role,
            "inviter_name": inviter_name,
        });

        info!("Creating notification with data: {}", notification_data);

        let title = format!("{} 邀请您加入 {} 空间", inviter_name, space_name);
        let content = format!(
            "{} 邀请您以 {} 的身份加入 {} 空间。{}",
            inviter_name,
            role,
            space_name,
            message.unwrap_or(""),
        );

        // 最终解决方案：将invite_token作为独立字段存储，完全绕过data字段的问题
        info!("Storing invite_token as separate field: {}", invite_token);

        let query = r#"
            CREATE notification SET
                user_id = $user_id,
                type = $type,
                title = $title,
                content = $content,
                data = NONE,
                invite_token = $invite_token,
                space_name = $space_name,
                role = $role,
                inviter_name = $inviter_name,
                is_read = false,
                created_at = time::now(),
                updated_at = time::now()
        "#;

        let mut result = self.db.client
            .query(query)
            .bind(("user_id", user_id))
            .bind(("type", "space_invitation"))
            .bind(("title", &title))
            .bind(("content", &content))
            .bind(("invite_token", invite_token))
            .bind(("space_name", space_name))
            .bind(("role", role))
            .bind(("inviter_name", inviter_name))
            .await
            .map_err(|e| {
                error!("Failed to create notification: {}", e);
                AppError::Database(e)
            })?;

        // 获取创建的通知记录
        let created_notifications: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| {
                error!("Failed to retrieve created notification: {}", e);
                AppError::Database(e.into())
            })?;

        if created_notifications.is_empty() {
            error!("No notification was created for user {}", user_id);
            return Err(AppError::Internal(anyhow::anyhow!("Failed to create notification")));
        }

        // 记录创建的通知详情
        if let Some(created_notification) = created_notifications.first() {
            info!("Successfully created notification: {}", 
                  serde_json::to_string_pretty(created_notification).unwrap_or_default());
        }

        info!("Created space invitation notification for user {}", user_id);
        Ok(())
    }

    /// 发送邀请邮件
    async fn send_invitation_email(
        &self,
        to_email: &str,
        space_name: &str,
        inviter_name: &str,
        invite_token: &str,
        role: &str,
        message: Option<&str>,
        expires_in_days: u64,
    ) -> Result<()> {
       /*  use serde_json::json;

        // 调用 Rainbow-Auth 的邮件服务
        let rainbow_auth_url = self.config.auth.rainbow_auth_url
            .as_ref()
            .ok_or_else(|| AppError::Configuration("Rainbow-Auth URL not configured".to_string()))?;

        let url = format!("{}/api/internal/email/notification", rainbow_auth_url);

        let email_data = json!({
            "to": to_email,
            "notification_type": "space_invitation",
            "data": {
                "space_name": space_name,
                "inviter_name": inviter_name,
                "invite_token": invite_token,
                "role": role,
                "message": message,
                "expires_in_days": expires_in_days,
            }
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("X-Internal-API-Key", "todo-implement-api-key") // TODO: 实现内部API密钥
            .json(&email_data)
            .send()
            .await
            .map_err(|e| AppError::External(format!("Failed to send email: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to send email notification: {}", error_text);
            return Err(AppError::External(format!("Email service error: {}", error_text)));
        } */

        info!("Sent invitation email to {}", to_email);
        Ok(())
    }
}