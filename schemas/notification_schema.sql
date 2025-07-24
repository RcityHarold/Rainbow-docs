-- 通知表
DEFINE TABLE notification SCHEMAFULL;
DEFINE FIELD id ON notification TYPE record(notification);
DEFINE FIELD user_id ON notification TYPE string ASSERT $value != NONE;
DEFINE FIELD type ON notification TYPE string ASSERT $value INSIDE ["space_invitation", "document_shared", "comment_mention", "document_update", "system"];
DEFINE FIELD title ON notification TYPE string ASSERT $value != NONE;
DEFINE FIELD content ON notification TYPE string ASSERT $value != NONE;
DEFINE FIELD data ON notification TYPE option<object>; -- 额外的数据，如邀请令牌、文档ID等
DEFINE FIELD is_read ON notification TYPE bool DEFAULT false;
DEFINE FIELD read_at ON notification TYPE option<datetime>;
DEFINE FIELD created_at ON notification TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON notification TYPE datetime DEFAULT time::now();

-- 索引
DEFINE INDEX notification_user_idx ON notification COLUMNS user_id;
DEFINE INDEX notification_user_unread_idx ON notification COLUMNS user_id, is_read;
DEFINE INDEX notification_created_idx ON notification COLUMNS created_at;