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
use chrono::{Utc, Duration};
use uuid::Uuid;

pub struct SpaceMemberService {
    db: Arc<Database>,
}

impl SpaceMemberService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
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
        let expires_at = Utc::now() + Duration::days(request.expires_in_days.unwrap_or(7) as i64);

        // 创建邀请记录
        let invitation = SpaceInvitation {
            id: None,
            space_id: space_id.to_string(),
            email: request.email.clone(),
            user_id: request.user_id.clone(),
            invite_token: invite_token.clone(),
            role: request.role.clone(),
            permissions: request.role.default_permissions(),
            invited_by: inviter.id.clone(),
            message: request.message,
            max_uses: 1,
            used_count: 0,
            expires_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 保存到数据库
        let created: Vec<SpaceInvitationDb> = self.db.client
            .create("space_invitation")
            .content(&invitation)
            .await
            .map_err(|e| AppError::Database(e))?;

        let created_invitation = created.into_iter().next()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create invitation")))?;

        info!("User {} invited {} to space {}", inviter.id, 
              request.email.as_deref().unwrap_or(request.user_id.as_deref().unwrap_or("unknown")), space_id);

        Ok(created_invitation.into())
    }

    /// 接受邀请
    pub async fn accept_invitation(&self, user_id: &str, request: AcceptInvitationRequest) -> Result<SpaceMember> {
        // 查找邀请
        let invitation_query = "SELECT * FROM space_invitation WHERE invite_token = $token AND expires_at > $now";
        let invitations: Vec<SpaceInvitationDb> = self.db.client
            .query(invitation_query)
            .bind(("token", &request.invite_token))
            .bind(("now", Utc::now()))
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

        // 创建成员记录
        let member = SpaceMember {
            id: None,
            space_id: invitation.space_id.id.to_string(),
            user_id: user_id.to_string(),
            role: invitation.role.clone(),
            permissions: invitation.permissions.clone(),
            invited_by: invitation.invited_by.clone(),
            invited_at: invitation.created_at,
            accepted_at: Some(Utc::now()),
            status: MemberStatus::Accepted,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 保存成员记录
        let created_members: Vec<SpaceMemberDb> = self.db.client
            .create("space_member")
            .content(&member)
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

        info!("User {} accepted invitation to space {}", user_id, member.space_id);

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
}