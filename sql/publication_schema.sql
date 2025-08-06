-- Rainbow-Docs 发布功能数据库表结构
-- 用于实现 GitBook 风格的文档集发布功能

-- 空间发布记录表
-- 存储每次发布的快照信息
DEFINE TABLE space_publication SCHEMAFULL;

DEFINE FIELD id ON TABLE space_publication TYPE record(space_publication);
DEFINE FIELD space_id ON TABLE space_publication TYPE string ASSERT $value != NONE;
DEFINE FIELD slug ON TABLE space_publication TYPE string ASSERT $value != NONE;
DEFINE FIELD version ON TABLE space_publication TYPE number DEFAULT 1;
DEFINE FIELD title ON TABLE space_publication TYPE string ASSERT $value != NONE;
DEFINE FIELD description ON TABLE space_publication TYPE option<string>;
DEFINE FIELD cover_image ON TABLE space_publication TYPE option<string>;
DEFINE FIELD theme ON TABLE space_publication TYPE string DEFAULT 'default';

-- 发布设置
DEFINE FIELD include_private_docs ON TABLE space_publication TYPE bool DEFAULT false;
DEFINE FIELD enable_search ON TABLE space_publication TYPE bool DEFAULT true;
DEFINE FIELD enable_comments ON TABLE space_publication TYPE bool DEFAULT false;
DEFINE FIELD custom_css ON TABLE space_publication TYPE option<string>;
DEFINE FIELD custom_js ON TABLE space_publication TYPE option<string>;

-- SEO 设置
DEFINE FIELD seo_title ON TABLE space_publication TYPE option<string>;
DEFINE FIELD seo_description ON TABLE space_publication TYPE option<string>;
DEFINE FIELD seo_keywords ON TABLE space_publication TYPE array<string> DEFAULT [];

-- 状态和时间戳
DEFINE FIELD is_active ON TABLE space_publication TYPE bool DEFAULT true;
DEFINE FIELD is_deleted ON TABLE space_publication TYPE bool DEFAULT false;
DEFINE FIELD published_by ON TABLE space_publication TYPE string ASSERT $value != NONE;
DEFINE FIELD published_at ON TABLE space_publication TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE space_publication TYPE datetime DEFAULT time::now();
DEFINE FIELD deleted_at ON TABLE space_publication TYPE option<datetime>;

-- 索引
DEFINE INDEX idx_space_publication_slug ON TABLE space_publication COLUMNS slug UNIQUE;
DEFINE INDEX idx_space_publication_space_id ON TABLE space_publication COLUMNS space_id;
DEFINE INDEX idx_space_publication_active ON TABLE space_publication COLUMNS is_active;

-- 发布的文档快照表
-- 存储发布时文档的完整快照
DEFINE TABLE publication_document SCHEMAFULL;

DEFINE FIELD id ON TABLE publication_document TYPE record(publication_document);
DEFINE FIELD publication_id ON TABLE publication_document TYPE string ASSERT $value != NONE;
DEFINE FIELD original_doc_id ON TABLE publication_document TYPE string ASSERT $value != NONE;

-- 文档内容快照
DEFINE FIELD title ON TABLE publication_document TYPE string ASSERT $value != NONE;
DEFINE FIELD slug ON TABLE publication_document TYPE string ASSERT $value != NONE;
DEFINE FIELD content ON TABLE publication_document TYPE string ASSERT $value != NONE;
DEFINE FIELD excerpt ON TABLE publication_document TYPE option<string>;

-- 文档结构
DEFINE FIELD parent_id ON TABLE publication_document TYPE option<string>;
DEFINE FIELD order_index ON TABLE publication_document TYPE number DEFAULT 0;

-- 文档元数据
DEFINE FIELD word_count ON TABLE publication_document TYPE number DEFAULT 0;
DEFINE FIELD reading_time ON TABLE publication_document TYPE number DEFAULT 0;

-- 时间戳
DEFINE FIELD created_at ON TABLE publication_document TYPE datetime DEFAULT time::now();

-- 索引
DEFINE INDEX idx_publication_document_publication_id ON TABLE publication_document COLUMNS publication_id;
DEFINE INDEX idx_publication_document_original_doc_id ON TABLE publication_document COLUMNS original_doc_id;
DEFINE INDEX idx_publication_document_parent_id ON TABLE publication_document COLUMNS parent_id;
DEFINE INDEX idx_publication_document_slug ON TABLE publication_document COLUMNS publication_id, slug UNIQUE;

-- 发布访问统计表
DEFINE TABLE publication_analytics SCHEMAFULL;

DEFINE FIELD id ON TABLE publication_analytics TYPE record(publication_analytics);
DEFINE FIELD publication_id ON TABLE publication_analytics TYPE string ASSERT $value != NONE;

-- 访问统计
DEFINE FIELD total_views ON TABLE publication_analytics TYPE number DEFAULT 0;
DEFINE FIELD unique_visitors ON TABLE publication_analytics TYPE number DEFAULT 0;

-- 按时间段统计
DEFINE FIELD views_today ON TABLE publication_analytics TYPE number DEFAULT 0;
DEFINE FIELD views_week ON TABLE publication_analytics TYPE number DEFAULT 0;
DEFINE FIELD views_month ON TABLE publication_analytics TYPE number DEFAULT 0;

-- 最热门文档
DEFINE FIELD popular_documents ON TABLE publication_analytics TYPE array<object> DEFAULT [];

-- 更新时间
DEFINE FIELD updated_at ON TABLE publication_analytics TYPE datetime DEFAULT time::now();

-- 索引
DEFINE INDEX idx_publication_analytics_publication_id ON TABLE publication_analytics COLUMNS publication_id UNIQUE;

-- 自定义域名表
DEFINE TABLE publication_domain SCHEMAFULL;

DEFINE FIELD id ON TABLE publication_domain TYPE record(publication_domain);
DEFINE FIELD publication_id ON TABLE publication_domain TYPE string ASSERT $value != NONE;
DEFINE FIELD domain ON TABLE publication_domain TYPE string ASSERT $value != NONE;

-- SSL 证书信息
DEFINE FIELD ssl_status ON TABLE publication_domain TYPE string DEFAULT 'pending';
DEFINE FIELD ssl_issued_at ON TABLE publication_domain TYPE option<datetime>;
DEFINE FIELD ssl_expires_at ON TABLE publication_domain TYPE option<datetime>;

-- 状态
DEFINE FIELD is_verified ON TABLE publication_domain TYPE bool DEFAULT false;
DEFINE FIELD is_active ON TABLE publication_domain TYPE bool DEFAULT false;

-- 时间戳
DEFINE FIELD created_at ON TABLE publication_domain TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE publication_domain TYPE datetime DEFAULT time::now();

-- 索引
DEFINE INDEX idx_publication_domain_publication_id ON TABLE publication_domain COLUMNS publication_id;
DEFINE INDEX idx_publication_domain_domain ON TABLE publication_domain COLUMNS domain UNIQUE;

-- 创建发布历史记录表（用于版本管理）
DEFINE TABLE publication_history SCHEMAFULL;

DEFINE FIELD id ON TABLE publication_history TYPE record(publication_history);
DEFINE FIELD publication_id ON TABLE publication_history TYPE string ASSERT $value != NONE;
DEFINE FIELD version ON TABLE publication_history TYPE number ASSERT $value != NONE;

-- 变更信息
DEFINE FIELD change_summary ON TABLE publication_history TYPE option<string>;
DEFINE FIELD changed_documents ON TABLE publication_history TYPE array<object> DEFAULT [];

-- 操作信息
DEFINE FIELD published_by ON TABLE publication_history TYPE string ASSERT $value != NONE;
DEFINE FIELD published_at ON TABLE publication_history TYPE datetime DEFAULT time::now();

-- 索引
DEFINE INDEX idx_publication_history_publication_id ON TABLE publication_history COLUMNS publication_id;
DEFINE INDEX idx_publication_history_version ON TABLE publication_history COLUMNS publication_id, version UNIQUE;