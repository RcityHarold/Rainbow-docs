-- Rainbow-Docs Database Schema
-- This file should be executed before starting the application

-- 文档空间表 (类似GitBook的Space)
DEFINE TABLE space SCHEMAFULL;
DEFINE FIELD id ON space TYPE record(space);
DEFINE FIELD name ON space TYPE string ASSERT $value != NONE AND string::len($value) > 0;
DEFINE FIELD slug ON space TYPE string ASSERT $value != NONE AND string::len($value) > 0;
DEFINE FIELD description ON space TYPE string;
DEFINE FIELD avatar_url ON space TYPE string;
DEFINE FIELD is_public ON space TYPE bool DEFAULT false;
DEFINE FIELD owner_id ON space TYPE string ASSERT $value != NONE; -- Rainbow-Auth用户ID
DEFINE FIELD settings ON space TYPE object DEFAULT {};
DEFINE FIELD created_at ON space TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON space TYPE datetime DEFAULT time::now();

-- 空间唯一性约束
DEFINE INDEX space_slug_idx ON space COLUMNS slug UNIQUE;
DEFINE INDEX space_owner_idx ON space COLUMNS owner_id;

-- 文档表
DEFINE TABLE document SCHEMAFULL;
DEFINE FIELD id ON document TYPE record(document);
DEFINE FIELD space_id ON document TYPE record(space) ASSERT $value != NONE;
DEFINE FIELD title ON document TYPE string ASSERT $value != NONE AND string::len($value) > 0;
DEFINE FIELD slug ON document TYPE string ASSERT $value != NONE AND string::len($value) > 0;
DEFINE FIELD content ON document TYPE string DEFAULT "";
DEFINE FIELD excerpt ON document TYPE string;
DEFINE FIELD is_published ON document TYPE bool DEFAULT false;
DEFINE FIELD parent_id ON document TYPE option<record(document)>;
DEFINE FIELD sort_order ON document TYPE number DEFAULT 0;
DEFINE FIELD author_id ON document TYPE string ASSERT $value != NONE;
DEFINE FIELD last_editor_id ON document TYPE string;
DEFINE FIELD view_count ON document TYPE number DEFAULT 0;
DEFINE FIELD metadata ON document TYPE object DEFAULT {};
DEFINE FIELD created_at ON document TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON document TYPE datetime DEFAULT time::now();

-- 文档索引
DEFINE INDEX document_space_slug_idx ON document COLUMNS space_id, slug UNIQUE;
DEFINE INDEX document_parent_idx ON document COLUMNS parent_id;
DEFINE INDEX document_author_idx ON document COLUMNS author_id;
DEFINE INDEX document_published_idx ON document COLUMNS is_published;

-- 文档版本表
DEFINE TABLE document_version SCHEMAFULL;
DEFINE FIELD id ON document_version TYPE record(document_version);
DEFINE FIELD document_id ON document_version TYPE record(document) ASSERT $value != NONE;
DEFINE FIELD version_number ON document_version TYPE number ASSERT $value != NONE AND $value > 0;
DEFINE FIELD title ON document_version TYPE string ASSERT $value != NONE;
DEFINE FIELD content ON document_version TYPE string DEFAULT "";
DEFINE FIELD change_message ON document_version TYPE string;
DEFINE FIELD author_id ON document_version TYPE string ASSERT $value != NONE;
DEFINE FIELD created_at ON document_version TYPE datetime DEFAULT time::now();

-- 版本索引
DEFINE INDEX version_document_idx ON document_version COLUMNS document_id;
DEFINE INDEX version_number_idx ON document_version COLUMNS document_id, version_number UNIQUE;

-- 文档权限表 (扩展Rainbow-Auth的RBAC)
DEFINE TABLE document_permission SCHEMAFULL;
DEFINE FIELD id ON document_permission TYPE record(document_permission);
DEFINE FIELD space_id ON document_permission TYPE record(space) ASSERT $value != NONE;
DEFINE FIELD document_id ON document_permission TYPE option<record(document)>; -- null表示整个空间权限
DEFINE FIELD user_id ON document_permission TYPE option<string>;
DEFINE FIELD role_name ON document_permission TYPE option<string>; -- Rainbow-Auth角色
DEFINE FIELD permission_type ON document_permission TYPE string ASSERT $value INSIDE ["read", "write", "admin"];
DEFINE FIELD granted_by ON document_permission TYPE string ASSERT $value != NONE;
DEFINE FIELD created_at ON document_permission TYPE datetime DEFAULT time::now();

-- 权限索引
DEFINE INDEX permission_space_idx ON document_permission COLUMNS space_id;
DEFINE INDEX permission_user_idx ON document_permission COLUMNS user_id;
DEFINE INDEX permission_role_idx ON document_permission COLUMNS role_name;

-- 评论表
DEFINE TABLE comment SCHEMAFULL;
DEFINE FIELD id ON comment TYPE record(comment);
DEFINE FIELD document_id ON comment TYPE record(document) ASSERT $value != NONE;
DEFINE FIELD parent_id ON comment TYPE option<record(comment)>; -- 回复支持
DEFINE FIELD author_id ON comment TYPE string ASSERT $value != NONE;
DEFINE FIELD content ON comment TYPE string ASSERT $value != NONE AND string::len($value) > 0;
DEFINE FIELD is_resolved ON comment TYPE bool DEFAULT false;
DEFINE FIELD metadata ON comment TYPE object DEFAULT {};
DEFINE FIELD created_at ON comment TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON comment TYPE datetime DEFAULT time::now();

-- 评论索引
DEFINE INDEX comment_document_idx ON comment COLUMNS document_id;
DEFINE INDEX comment_author_idx ON comment COLUMNS author_id;
DEFINE INDEX comment_parent_idx ON comment COLUMNS parent_id;

-- 文档标签表
DEFINE TABLE tag SCHEMAFULL;
DEFINE FIELD id ON tag TYPE record(tag);
DEFINE FIELD name ON tag TYPE string ASSERT $value != NONE AND string::len($value) > 0;
DEFINE FIELD color ON tag TYPE string DEFAULT "#3b82f6";
DEFINE FIELD space_id ON tag TYPE record(space) ASSERT $value != NONE;
DEFINE FIELD created_at ON tag TYPE datetime DEFAULT time::now();

-- 标签索引
DEFINE INDEX tag_space_name_idx ON tag COLUMNS space_id, name UNIQUE;

-- 文档标签关联表
DEFINE TABLE document_tag SCHEMAFULL;
DEFINE FIELD id ON document_tag TYPE record(document_tag);
DEFINE FIELD document_id ON document_tag TYPE record(document) ASSERT $value != NONE;
DEFINE FIELD tag_id ON document_tag TYPE record(tag) ASSERT $value != NONE;
DEFINE FIELD created_at ON document_tag TYPE datetime DEFAULT time::now();

-- 文档标签关联索引
DEFINE INDEX document_tag_unique_idx ON document_tag COLUMNS document_id, tag_id UNIQUE;

-- 搜索索引表 (全文搜索)
DEFINE TABLE search_index SCHEMAFULL;
DEFINE FIELD id ON search_index TYPE record(search_index);
DEFINE FIELD document_id ON search_index TYPE record(document) ASSERT $value != NONE;
DEFINE FIELD space_id ON search_index TYPE record(space) ASSERT $value != NONE;
DEFINE FIELD title ON search_index TYPE string;
DEFINE FIELD content ON search_index TYPE string; -- 纯文本内容
DEFINE FIELD keywords ON search_index TYPE array<string> DEFAULT [];
DEFINE FIELD updated_at ON search_index TYPE datetime DEFAULT time::now();

-- 搜索索引
DEFINE INDEX search_document_idx ON search_index COLUMNS document_id UNIQUE;
DEFINE INDEX search_space_idx ON search_index COLUMNS space_id;

-- 用户收藏表
DEFINE TABLE user_favorite SCHEMAFULL;
DEFINE FIELD id ON user_favorite TYPE record(user_favorite);
DEFINE FIELD user_id ON user_favorite TYPE string ASSERT $value != NONE;
DEFINE FIELD document_id ON user_favorite TYPE record(document) ASSERT $value != NONE;
DEFINE FIELD created_at ON user_favorite TYPE datetime DEFAULT time::now();

-- 收藏索引
DEFINE INDEX favorite_user_document_idx ON user_favorite COLUMNS user_id, document_id UNIQUE;

-- 活动日志表
DEFINE TABLE activity_log SCHEMAFULL;
DEFINE FIELD id ON activity_log TYPE record(activity_log);
DEFINE FIELD user_id ON activity_log TYPE string ASSERT $value != NONE;
DEFINE FIELD action ON activity_log TYPE string ASSERT $value != NONE;
DEFINE FIELD resource_type ON activity_log TYPE string ASSERT $value INSIDE ["space", "document", "comment"];
DEFINE FIELD resource_id ON activity_log TYPE string ASSERT $value != NONE;
DEFINE FIELD metadata ON activity_log TYPE object DEFAULT {};
DEFINE FIELD created_at ON activity_log TYPE datetime DEFAULT time::now();

-- 活动日志索引
DEFINE INDEX activity_user_idx ON activity_log COLUMNS user_id;
DEFINE INDEX activity_resource_idx ON activity_log COLUMNS resource_type, resource_id;
DEFINE INDEX activity_created_idx ON activity_log COLUMNS created_at;