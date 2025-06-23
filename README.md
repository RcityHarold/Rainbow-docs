# Rainbow-Docs

一个基于 Rust 构建的现代化文档系统，类似 GitBook，支持独立使用或与 Rainbow-Auth 认证系统集成。

## 功能特点

### 📚 文档管理
- **文档空间**: 类似 GitBook 的 Space 概念，支持多个独立的文档项目
- **层级结构**: 支持章节嵌套，灵活的文档组织方式
- **Markdown 编辑**: 完整的 Markdown 支持，富文本编辑体验
- **实时预览**: 编辑时实时预览文档效果
- **版本控制**: 完整的文档版本管理和历史记录

### 🔐 权限系统
- **集成模式**: 与 Rainbow-Auth 完全集成，使用企业级 RBAC 权限控制
- **独立模式**: 可独立运行，内置基础用户管理
- **细粒度权限**: 支持空间级别和文档级别的权限控制
- **角色管理**: 支持多种角色：所有者、编辑者、阅读者

### 🔍 搜索与发现
- **全文搜索**: 快速搜索文档内容，支持高亮显示
- **高级搜索**: 按空间、标签、作者等条件筛选
- **搜索建议**: 智能搜索自动补全
- **搜索索引**: 自动维护的全文搜索索引

### 🏷️ 标签系统
- **标签管理**: 创建、编辑、删除标签
- **文档标记**: 为文档添加多个标签
- **标签统计**: 使用频率和热门标签分析
- **标签搜索**: 按标签快速查找文档
- **颜色配置**: 自定义标签颜色和样式

### 📁 文件管理
- **多格式支持**: 图片、文档、文本、代码、压缩包等
- **图片处理**: 自动生成缩略图，支持多种图片格式
- **文件上传**: 支持拖拽上传和批量上传
- **安全验证**: 文件类型检查和大小限制
- **关联管理**: 文件可关联到文档和空间
- **权限控制**: 基于用户和空间的访问控制

### 🤝 协作功能
- **评论系统**: 文档评论和讨论
- **通知系统**: 实时更新通知
- **活动日志**: 完整的操作历史记录

### 📤 导出功能
- **多种格式**: 支持 PDF、HTML、电子书等格式导出
- **主题定制**: 可定制的导出样式和品牌化

## 技术栈

### 后端技术
- **Web框架**: Axum (与 Rainbow-Auth 相同)
- **数据库**: SurrealDB (与 Rainbow-Auth 相同)
- **认证**: JWT + OAuth 2.0 / OIDC
- **文档处理**: pulldown-cmark, comrak
- **异步运行时**: Tokio

## 快速开始

### 环境要求
- Rust 1.87.0 或更高版本
- SurrealDB
- Rainbow-Auth (集成模式)
- 足够的磁盘空间用于文件存储

### 安装步骤

1. **克隆项目**
```bash
git clone https://github.com/yourusername/rainbow-docs.git
cd rainbow-docs
```

2. **配置环境变量**
```bash
cp .env.example .env
# 编辑 .env 文件配置数据库和认证信息
```

3. **初始化数据库**
```bash
# 连接到 SurrealDB
surreal sql --conn http://localhost:8000 --user root --pass root --ns docs --db main

# 导入数据库架构
surreal import --conn http://localhost:8000 --user root --pass root --ns docs --db main schemas/docs_schema.sql
```

4. **构建和运行**
```bash
cargo build --release
cargo run
```

5. **验证安装**
```bash
curl http://localhost:3000/health
```

## 配置说明

### 集成模式 (推荐)
与 Rainbow-Auth 集成，享受企业级认证和权限管理：

```env
RAINBOW_AUTH_URL=http://localhost:8080
RAINBOW_AUTH_INTEGRATION=true
JWT_SECRET=your-jwt-secret

# 文件上传配置
UPLOAD_DIR=./uploads
MAX_FILE_SIZE=10485760
```

### 独立模式
独立运行，使用内置用户系统：

```env
RAINBOW_AUTH_INTEGRATION=false
JWT_SECRET=your-jwt-secret

# 文件上传配置
UPLOAD_DIR=./uploads
MAX_FILE_SIZE=10485760
```

### 文件上传配置
- `UPLOAD_DIR`: 文件上传目录，默认为 `./uploads`
- `MAX_FILE_SIZE`: 最大文件大小（字节），默认为 10MB (10485760)

## API 文档

### 认证
所有API需要在请求头中包含有效的JWT token：
```
Authorization: Bearer <your-jwt-token>
```

### 文档空间管理

#### 获取空间列表
```http
GET /api/spaces
```

**查询参数:**
- `page` (可选): 页码，默认为1
- `per_page` (可选): 每页数量，默认为20
- `search` (可选): 搜索关键词

**响应示例:**
```json
{
  "spaces": [
    {
      "id": "space:123",
      "name": "API Documentation",
      "slug": "api-docs",
      "description": "API接口文档",
      "is_public": true,
      "created_at": "2024-01-01T00:00:00Z",
      "created_by": "user123"
    }
  ],
  "total_count": 5,
  "page": 1,
  "per_page": 20
}
```

#### 创建空间
```http
POST /api/spaces
Content-Type: application/json
```

**请求体:**
```json
{
  "name": "新文档空间",
  "slug": "new-space",
  "description": "空间描述",
  "is_public": true
}
```

**响应示例:**
```json
{
  "id": "space:456",
  "name": "新文档空间",
  "slug": "new-space",
  "description": "空间描述",
  "is_public": true,
  "created_at": "2024-01-15T10:30:00Z",
  "created_by": "user123"
}
```

#### 获取空间详情
```http
GET /api/spaces/{space_id}
```

#### 更新空间
```http
PUT /api/spaces/{space_id}
Content-Type: application/json
```

**请求体:**
```json
{
  "name": "更新的空间名称",
  "description": "更新的描述",
  "is_public": false
}
```

#### 删除空间
```http
DELETE /api/spaces/{space_id}
```

#### 获取空间统计
```http
GET /api/spaces/{space_id}/stats
```

### 文档管理

#### 获取文档列表
```http
GET /api/docs
```

**查询参数:**
- `space_id` (可选): 空间ID
- `page` (可选): 页码，默认为1
- `per_page` (可选): 每页数量，默认为20
- `parent_id` (可选): 父文档ID

#### 创建文档
```http
POST /api/docs
Content-Type: application/json
```

**请求体:**
```json
{
  "space_id": "space:123",
  "title": "新文档标题",
  "slug": "new-document",
  "content": "# 文档内容\n\n这是文档内容...",
  "description": "文档描述",
  "parent_id": "doc:parent",
  "is_public": true
}
```

#### 获取文档详情
```http
GET /api/docs/{document_id}
```

#### 更新文档
```http
PUT /api/docs/{document_id}
Content-Type: application/json
```

**请求体:**
```json
{
  "title": "更新的标题",
  "content": "更新的内容",
  "description": "更新的描述",
  "is_public": false
}
```

#### 删除文档
```http
DELETE /api/docs/{document_id}
```

#### 移动文档
```http
PUT /api/docs/{document_id}/move
Content-Type: application/json
```

**请求体:**
```json
{
  "new_parent_id": "doc:new_parent",
  "new_order_index": 5
}
```

#### 复制文档
```http
POST /api/docs/{document_id}/duplicate
Content-Type: application/json
```

**请求体:**
```json
{
  "title": "复制的文档标题",
  "slug": "duplicated-document"
}
```

#### 获取文档面包屑
```http
GET /api/docs/{document_id}/breadcrumbs
```

#### 获取文档子页面
```http
GET /api/docs/{document_id}/children
```

### 版本控制

#### 获取文档版本列表
```http
GET /api/versions/{document_id}/versions
```

**查询参数:**
- `page` (可选): 页码，默认为1
- `per_page` (可选): 每页数量，默认为20
- `author_id` (可选): 按作者筛选

#### 创建新版本
```http
POST /api/versions/{document_id}/versions
Content-Type: application/json
```

**请求体:**
```json
{
  "title": "文档标题",
  "content": "文档内容",
  "summary": "本次更改的描述",
  "change_type": "Updated"
}
```

**change_type 可选值:** `Created`, `Updated`, `Restored`, `Merged`

#### 获取当前版本
```http
GET /api/versions/{document_id}/versions/current
```

#### 获取特定版本
```http
GET /api/versions/{document_id}/versions/{version_id}
```

#### 恢复版本
```http
POST /api/versions/{document_id}/versions/{version_id}/restore
Content-Type: application/json
```

**请求体:**
```json
{
  "summary": "恢复到版本 3"
}
```

#### 比较版本
```http
GET /api/versions/{document_id}/versions/compare?from_version={version_id_1}&to_version={version_id_2}
```

#### 获取版本历史摘要
```http
GET /api/versions/{document_id}/versions/summary
```

#### 删除版本
```http
DELETE /api/versions/{document_id}/versions/{version_id}
```

### 搜索功能

#### 全文搜索
```http
GET /api/search
```

**查询参数:**
- `q`: 搜索关键词 (必需)
- `space_id` (可选): 限制在特定空间内搜索
- `tags` (可选): 按标签筛选，逗号分隔
- `author_id` (可选): 按作者筛选
- `page` (可选): 页码，默认为1
- `per_page` (可选): 每页数量，默认为20
- `sort` (可选): 排序方式 (`relevance`, `created_at`, `updated_at`, `title`)

**响应示例:**
```json
{
  "results": [
    {
      "document_id": "doc:123",
      "space_id": "space:456",
      "title": "API文档",
      "excerpt": "...包含搜索关键词的摘要...",
      "tags": ["api", "documentation"],
      "author_id": "user123",
      "last_updated": "2024-01-15T10:30:00Z",
      "score": 95.5,
      "highlights": [
        {
          "field": "title",
          "text": "API文档",
          "start": 0,
          "end": 5
        }
      ]
    }
  ],
  "total_count": 42,
  "page": 1,
  "per_page": 20,
  "total_pages": 3,
  "query": "API",
  "took": 15
}
```

#### 搜索建议
```http
GET /api/search/suggest?q={prefix}&limit=10
```

#### 重建搜索索引
```http
POST /api/search/reindex
```

#### 空间内搜索
```http
GET /api/search/spaces/{space_id}?q={query}
```

#### 按标签搜索
```http
GET /api/search/tags?tags={tag1,tag2}
```

### 标签管理

#### 获取标签列表
```http
GET /api/tags
```

**查询参数:**
- `space_id` (可选): 空间ID，为空则获取全局标签
- `page` (可选): 页码，默认为1
- `per_page` (可选): 每页数量，默认为20
- `search` (可选): 搜索关键词

**响应示例:**
```json
{
  "tags": [
    {
      "id": "tag:123",
      "name": "API",
      "slug": "api",
      "description": "API相关文档",
      "color": "#10b981",
      "space_id": "space:456",
      "usage_count": 15,
      "created_by": "user123",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ],
  "total_count": 25,
  "page": 1,
  "per_page": 20,
  "total_pages": 2
}
```

#### 创建标签
```http
POST /api/tags
Content-Type: application/json
```

**请求体:**
```json
{
  "name": "新标签",
  "description": "标签描述",
  "color": "#3b82f6",
  "space_id": "space:123"
}
```

#### 获取标签详情
```http
GET /api/tags/{tag_id}
```

#### 更新标签
```http
PUT /api/tags/{tag_id}
Content-Type: application/json
```

**请求体:**
```json
{
  "name": "更新的标签名",
  "description": "更新的描述",
  "color": "#ef4444"
}
```

#### 删除标签
```http
DELETE /api/tags/{tag_id}
```

#### 获取热门标签
```http
GET /api/tags/popular?space_id={space_id}&limit=10
```

#### 标签搜索建议
```http
GET /api/tags/suggest?search={query}&space_id={space_id}
```

#### 获取标签统计
```http
GET /api/tags/statistics?space_id={space_id}
```

**响应示例:**
```json
{
  "total_tags": 45,
  "used_tags": 32,
  "unused_tags": 13,
  "most_used_tags": [
    {
      "id": "tag:123",
      "name": "API",
      "usage_count": 25
    }
  ]
}
```

#### 为文档添加标签
```http
POST /api/tags/documents/tag
Content-Type: application/json
```

**请求体:**
```json
{
  "document_id": "doc:123",
  "tag_ids": ["tag:456", "tag:789"]
}
```

#### 获取文档标签
```http
GET /api/tags/documents/{document_id}
```

**响应示例:**
```json
{
  "document_id": "doc:123",
  "tags": [
    {
      "id": "tag:456",
      "name": "Tutorial",
      "color": "#3b82f6"
    }
  ]
}
```

#### 移除文档标签
```http
DELETE /api/tags/documents/{document_id}/tags/{tag_id}
```

#### 获取标签关联的文档
```http
GET /api/tags/{tag_id}/documents?page=1&per_page=20
```

**响应示例:**
```json
{
  "tag_id": "tag:123",
  "document_ids": ["doc:456", "doc:789"],
  "total_count": 15,
  "page": 1,
  "per_page": 20
}
```

### 📁 文件上传和管理

#### 上传文件
```http
POST /api/files
Content-Type: multipart/form-data
```

**表单数据:**
- `file` (必需): 要上传的文件
- `space_id` (可选): 关联的空间ID
- `document_id` (可选): 关联的文档ID
- `description` (可选): 文件描述

**支持的文件类型:**
- **图片**: JPEG, PNG, GIF, WebP, SVG
- **文档**: PDF, Word, Excel, PowerPoint
- **文本**: TXT, Markdown, CSV
- **代码**: JSON, XML, HTML, CSS, JavaScript
- **压缩包**: ZIP, TAR, GZIP

**响应示例:**
```json
{
  "id": "file_upload:123",
  "filename": "uuid-filename.jpg",
  "original_name": "my-image.jpg",
  "file_size": 1048576,
  "file_type": "image",
  "mime_type": "image/jpeg",
  "url": "/api/files/file_upload:123/download",
  "thumbnail_url": "/api/files/file_upload:123/thumbnail",
  "space_id": "space:456",
  "document_id": null,
  "uploaded_by": "user123",
  "created_at": "2024-01-15T10:30:00Z"
}
```

#### 获取文件列表
```http
GET /api/files
```

**查询参数:**
- `space_id` (可选): 空间ID筛选
- `document_id` (可选): 文档ID筛选
- `file_type` (可选): 文件类型筛选 (`image`, `document`, `text`, `archive`, 等)
- `page` (可选): 页码，默认为1
- `per_page` (可选): 每页数量，默认为20，最大100

**响应示例:**
```json
{
  "files": [
    {
      "id": "file_upload:123",
      "filename": "uuid-filename.jpg",
      "original_name": "my-image.jpg",
      "file_size": 1048576,
      "file_type": "image",
      "mime_type": "image/jpeg",
      "url": "/api/files/file_upload:123/download",
      "thumbnail_url": "/api/files/file_upload:123/thumbnail",
      "space_id": "space:456",
      "document_id": null,
      "uploaded_by": "user123",
      "created_at": "2024-01-15T10:30:00Z"
    }
  ],
  "total_count": 25,
  "page": 1,
  "per_page": 20,
  "total_pages": 2
}
```

#### 获取文件信息
```http
GET /api/files/{file_id}
```

#### 下载文件
```http
GET /api/files/{file_id}/download
```

**响应:**
- 返回文件的二进制内容
- 设置正确的 `Content-Type` 和 `Content-Disposition` 头

#### 获取图片缩略图
```http
GET /api/files/{file_id}/thumbnail
```

**响应:**
- 仅适用于图片文件
- 返回 300x300 像素的 JPEG 缩略图
- 设置缓存头 `Cache-Control: public, max-age=86400`

#### 删除文件
```http
DELETE /api/files/{file_id}
```

**响应示例:**
```json
{
  "message": "File deleted successfully"
}
```

**注意事项:**
- 文件删除采用软删除机制，不会立即删除物理文件
- 只有文件上传者或具有相应权限的用户可以删除文件
- 删除后文件仍保留在数据库中，但标记为已删除状态

### 评论系统

#### 获取文档评论列表
```http
GET /api/comments/document/{document_id}
```

**查询参数:**
- `page` (可选): 页码，默认为1
- `per_page` (可选): 每页数量，默认为20
- `sort` (可选): 排序方式

#### 创建评论
```http
POST /api/comments/document/{document_id}
Content-Type: application/json
```

**请求体:**
```json
{
  "content": "这是一条评论",
  "parent_id": "comment:parent"
}
```

#### 获取评论详情
```http
GET /api/comments/{comment_id}
```

#### 更新评论
```http
PUT /api/comments/{comment_id}
Content-Type: application/json
```

**请求体:**
```json
{
  "content": "更新的评论内容"
}
```

#### 删除评论
```http
DELETE /api/comments/{comment_id}
```

#### 获取评论回复
```http
GET /api/comments/{comment_id}/replies
```

#### 点赞/取消点赞评论
```http
POST /api/comments/{comment_id}/like
```

### 统计信息

#### 获取搜索统计
```http
GET /api/stats/search
```

**响应示例:**
```json
{
  "total_documents": 156,
  "total_searches_today": 42,
  "most_searched_terms": [
    {
      "term": "API documentation",
      "count": 15
    }
  ],
  "recent_searches": [
    {
      "query": "user management",
      "results_count": 7,
      "timestamp": "2024-01-15T10:30:00Z"
    }
  ]
}
```

#### 获取文档统计
```http
GET /api/stats/documents
```

**响应示例:**
```json
{
  "total_documents": 156,
  "total_spaces": 12,
  "total_comments": 89,
  "documents_created_today": 3,
  "most_active_spaces": [
    {
      "space_id": "space_1",
      "space_name": "API Documentation",
      "document_count": 45,
      "recent_activity": 12
    }
  ]
}
```

## 错误处理

API使用标准HTTP状态码，错误响应格式：

```json
{
  "error": "错误类型",
  "message": "详细错误信息",
  "details": "额外的错误详情"
}
```

常见状态码：
- `200` - 成功
- `201` - 创建成功
- `204` - 删除成功
- `400` - 请求参数错误
- `401` - 未认证
- `403` - 权限不足
- `404` - 资源不存在
- `409` - 资源冲突
- `500` - 服务器内部错误

## 数据库架构

系统使用 SurrealDB 作为数据库，提供完整的关系型数据库功能和灵活的架构设计。

### 核心业务表

#### 文档空间表 (space)
- `id` - 空间唯一标识
- `name` - 空间名称
- `slug` - URL友好的标识符
- `description` - 空间描述
- `is_public` - 是否公开
- `owner_id` - 所有者用户ID
- `member_count` - 成员数量
- `document_count` - 文档数量
- `settings` - 空间配置
- `theme_config` - 主题配置

#### 文档表 (document)
- `id` - 文档唯一标识
- `space_id` - 所属空间
- `title` - 文档标题
- `slug` - URL友好标识符
- `content` - Markdown内容
- `excerpt` - 文档摘要
- `parent_id` - 父文档ID（支持层级结构）
- `order_index` - 排序索引
- `status` - 文档状态（draft/published/archived）
- `word_count` - 字数统计
- `reading_time` - 预计阅读时间
- `view_count` - 访问次数

#### 版本控制表 (document_version)
- `id` - 版本唯一标识
- `document_id` - 关联文档
- `version_number` - 版本号
- `title` - 版本标题
- `content` - 版本内容
- `summary` - 变更摘要
- `change_type` - 变更类型
- `is_current` - 是否当前版本
- `author_id` - 作者ID

### 标签系统

#### 标签表 (tag)
- `id` - 标签唯一标识
- `name` - 标签名称
- `slug` - URL友好标识符
- `description` - 标签描述
- `color` - 标签颜色（十六进制）
- `space_id` - 所属空间（NULL为全局标签）
- `usage_count` - 使用次数
- `created_by` - 创建者

#### 文档标签关联表 (document_tag)
- `id` - 关联唯一标识
- `document_id` - 文档ID
- `tag_id` - 标签ID
- `tagged_by` - 标记者
- `tagged_at` - 标记时间

### 协作系统

#### 评论表 (comment)
- `id` - 评论唯一标识
- `document_id` - 关联文档
- `parent_id` - 父评论ID（支持嵌套回复）
- `author_id` - 评论作者
- `content` - 评论内容
- `like_count` - 点赞数
- `liked_by` - 点赞用户列表
- `is_resolved` - 是否已解决

#### 权限表 (document_permission)
- `id` - 权限唯一标识
- `resource_type` - 资源类型
- `resource_id` - 资源ID
- `user_id` - 用户ID
- `role_id` - 角色ID
- `permissions` - 权限列表
- `expires_at` - 过期时间

### 搜索和索引

#### 搜索索引表 (search_index)
- `id` - 索引唯一标识
- `document_id` - 文档ID
- `space_id` - 空间ID
- `title` - 标题
- `content` - 纯文本内容
- `tags` - 标签列表
- `is_public` - 是否公开

### 用户交互

#### 收藏表 (user_favorite)
- `id` - 收藏唯一标识
- `user_id` - 用户ID
- `resource_type` - 资源类型
- `resource_id` - 资源ID

#### 访问记录表 (document_view)
- `id` - 记录唯一标识
- `document_id` - 文档ID
- `user_id` - 用户ID
- `duration` - 阅读时长
- `ip_address` - IP地址

### 系统表

#### 活动日志表 (activity_log)
- `id` - 日志唯一标识
- `user_id` - 用户ID
- `action` - 操作类型
- `resource_type` - 资源类型
- `resource_id` - 资源ID
- `details` - 操作详情

#### 文件上传表 (file_upload)
- `id` - 文件唯一标识
- `filename` - 系统生成的文件名（UUID）
- `original_name` - 用户原始文件名
- `file_path` - 文件存储路径
- `file_size` - 文件大小（字节）
- `file_type` - 文件类型分类（image/document/text等）
- `mime_type` - MIME类型
- `uploaded_by` - 上传者用户ID
- `space_id` - 关联空间（可选）
- `document_id` - 关联文档（可选）
- `is_deleted` - 是否已删除
- `deleted_at` - 删除时间
- `deleted_by` - 删除者

#### 通知表 (notification)
- `id` - 通知唯一标识
- `recipient_id` - 接收者ID
- `sender_id` - 发送者ID
- `type` - 通知类型
- `title` - 通知标题
- `content` - 通知内容
- `is_read` - 是否已读

### 索引优化

系统为所有表创建了适当的索引以提升查询性能：
- 主键索引
- 外键关联索引
- 搜索字段索引
- 状态字段索引
- 时间字段索引

### 初始数据

系统会自动创建以下默认标签：
- API（绿色）- API相关文档
- Tutorial（蓝色）- 教程文档
- Guide（紫色）- 指南文档  
- Reference（橙色）- 参考文档
- FAQ（红色）- 常见问题

## 开发指南

### 项目结构
```
src/
├── main.rs              # 应用入口
├── config.rs            # 配置管理
├── error.rs             # 错误处理
├── models/              # 数据模型
│   ├── space.rs         # 空间模型
│   ├── document.rs      # 文档模型
│   ├── version.rs       # 版本模型
│   ├── comment.rs       # 评论模型
│   ├── permission.rs    # 权限模型
│   ├── tag.rs          # 标签模型
│   ├── file.rs         # 文件模型
│   └── search.rs       # 搜索模型
├── routes/              # 路由处理
│   ├── spaces.rs       # 空间路由
│   ├── documents.rs    # 文档路由
│   ├── versions.rs     # 版本路由
│   ├── comments.rs     # 评论路由
│   ├── search.rs       # 搜索路由
│   ├── tags.rs         # 标签路由
│   ├── files.rs        # 文件路由
│   └── stats.rs        # 统计路由
├── services/            # 业务逻辑
│   ├── auth.rs         # 认证服务
│   ├── spaces.rs       # 空间服务
│   ├── documents.rs    # 文档服务
│   ├── versions.rs     # 版本服务
│   ├── comments.rs     # 评论服务
│   ├── search.rs       # 搜索服务
│   ├── tags.rs         # 标签服务
│   └── file_upload.rs  # 文件上传服务
└── utils/               # 工具函数
    └── markdown.rs     # Markdown处理
```

### 添加新功能
1. 在 `models/` 中定义数据模型
2. 在 `services/` 中实现业务逻辑
3. 在 `routes/` 中添加 API 端点
4. 更新数据库 schema

## 部署指南

### Docker 部署
```bash
# 构建镜像
docker build -t rainbow-docs .

# 运行容器
docker run -d \
  --name rainbow-docs \
  -p 3000:3000 \
  -e DATABASE_URL=http://surrealdb:8000 \
  -e JWT_SECRET=your-secret \
  rainbow-docs
```

### 生产环境注意事项
1. 使用强随机 JWT 密钥
2. 配置 HTTPS
3. 设置适当的数据库权限
4. 配置日志收集
5. 设置健康检查

## 贡献指南

1. Fork 项目
2. 创建功能分支
3. 提交更改
4. 推送到分支
5. 创建 Pull Request

## 许可证

MIT License

## 支持

如有问题或建议，请创建 Issue 或联系开发团队。