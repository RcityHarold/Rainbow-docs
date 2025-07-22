-- 空间成员表扩展
-- 添加到现有的docs_schema.sql中

-- 空间成员表
DEFINE TABLE space_member SCHEMAFULL;
DEFINE FIELD id ON space_member TYPE record(space_member);
DEFINE FIELD space_id ON space_member TYPE record(space) ASSERT $value != NONE;
DEFINE FIELD user_id ON space_member TYPE string ASSERT $value != NONE; -- Rainbow-Auth用户ID
DEFINE FIELD role ON space_member TYPE string DEFAULT "member" ASSERT $value INSIDE ["owner", "admin", "editor", "viewer", "member"];
DEFINE FIELD permissions ON space_member TYPE array<string> DEFAULT ["docs.read"];
DEFINE FIELD invited_by ON space_member TYPE string ASSERT $value != NONE;
DEFINE FIELD invited_at ON space_member TYPE datetime DEFAULT time::now();
DEFINE FIELD accepted_at ON space_member TYPE option<datetime>;
DEFINE FIELD status ON space_member TYPE string DEFAULT "pending" ASSERT $value INSIDE ["pending", "accepted", "rejected", "removed"];
DEFINE FIELD expires_at ON space_member TYPE option<datetime>; -- 邀请过期时间
DEFINE FIELD created_at ON space_member TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON space_member TYPE datetime DEFAULT time::now();

-- 空间成员索引
DEFINE INDEX space_member_unique_idx ON space_member COLUMNS space_id, user_id UNIQUE;
DEFINE INDEX space_member_space_idx ON space_member COLUMNS space_id;
DEFINE INDEX space_member_user_idx ON space_member COLUMNS user_id;
DEFINE INDEX space_member_status_idx ON space_member COLUMNS status;
DEFINE INDEX space_member_role_idx ON space_member COLUMNS role;

-- 空间邀请表（用于邀请链接等）
DEFINE TABLE space_invitation SCHEMAFULL;
DEFINE FIELD id ON space_invitation TYPE record(space_invitation);
DEFINE FIELD space_id ON space_invitation TYPE record(space) ASSERT $value != NONE;
DEFINE FIELD email ON space_invitation TYPE option<string>; -- 被邀请人邮箱
DEFINE FIELD user_id ON space_invitation TYPE option<string>; -- 被邀请人用户ID（如果已注册）
DEFINE FIELD invite_token ON space_invitation TYPE string ASSERT $value != NONE; -- 唯一邀请令牌
DEFINE FIELD role ON space_invitation TYPE string DEFAULT "member" ASSERT $value INSIDE ["admin", "editor", "viewer", "member"];
DEFINE FIELD permissions ON space_invitation TYPE array<string> DEFAULT ["docs.read"];
DEFINE FIELD invited_by ON space_invitation TYPE string ASSERT $value != NONE;
DEFINE FIELD message ON space_invitation TYPE option<string>; -- 邀请消息
DEFINE FIELD max_uses ON space_invitation TYPE number DEFAULT 1; -- 最大使用次数
DEFINE FIELD used_count ON space_invitation TYPE number DEFAULT 0; -- 已使用次数
DEFINE FIELD expires_at ON space_invitation TYPE datetime ASSERT $value != NONE;
DEFINE FIELD created_at ON space_invitation TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON space_invitation TYPE datetime DEFAULT time::now();

-- 邀请索引
DEFINE INDEX space_invitation_token_idx ON space_invitation COLUMNS invite_token UNIQUE;
DEFINE INDEX space_invitation_space_idx ON space_invitation COLUMNS space_id;
DEFINE INDEX space_invitation_inviter_idx ON space_invitation COLUMNS invited_by;
DEFINE INDEX space_invitation_email_idx ON space_invitation COLUMNS email;