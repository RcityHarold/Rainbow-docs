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
- **多租户支持**: 完整的空间成员管理和邀请系统
- **细粒度权限**: 支持空间级别和文档级别的权限控制
- **角色管理**: 支持多种角色：所有者、管理员、编辑者、成员、阅读者
- **邀请机制**: 通过邮箱或用户ID邀请成员加入空间

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
- **空间成员管理**: 邀请、管理、移除成员
- **角色权限控制**: 基于角色的精细化权限管理
- **邀请系统**: 支持邮箱邀请和用户ID邀请，邮件通知和站内通知
- **评论系统**: 文档评论和讨论
- **通知系统**: 站内通知，支持空间邀请、文档更新等多种通知类型
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
- Rainbow-Auth (集成模式，可选)
- 足够的磁盘空间用于文件存储

## 🚀 安装和使用

Rainbow-Docs 提供了两种运行模式：**安装向导模式** 和 **生产模式**。

### 方式一：使用安装向导（推荐新用户）

安装向导提供类似 WordPress/Discuz 的 Web 界面安装体验，无需手动配置文件。

#### 开发环境测试
```bash
# 1. 克隆项目
git clone https://github.com/yourusername/rainbow-docs.git
cd rainbow-docs

# 2. 启动安装模式（编译并运行）
cargo run --features installer

# 3. 浏览器访问安装向导
# 访问 http://localhost:3000
# 系统会自动检测未安装状态并显示安装向导
```

#### 生产环境部署
```bash
# 1. 构建安装版本
./build.sh installer

# 2. 运行编译好的程序
./target/release/rainbow-docs

# 3. 浏览器访问 http://localhost:3000 完成安装
```

#### 安装向导流程
1. **环境检查** - 检查系统环境和依赖
2. **数据库配置** - 配置 SurrealDB 连接
3. **管理员账户** - 创建系统管理员账户
4. **站点配置** - 配置站点基本信息
5. **完成安装** - 保存配置并初始化系统

#### Cargo Run vs Cargo Build 的区别

**开发时使用 Cargo Run:**
```bash
cargo run                        # 编译+运行开发版本
cargo run --features installer   # 编译+运行安装版本
```
- 每次检查代码变化并重新编译
- 适合开发和测试
- 程序运行在前台，Ctrl+C 停止

**生产部署使用 Cargo Build:**
```bash
cargo build --release            # 只编译，生成优化的可执行文件
./target/release/rainbow-docs     # 运行编译好的程序
```
- 生成独立的二进制文件，可部署到其他机器
- 性能更好，适合生产环境

### 方式二：手动配置（高级用户）

#### 1. 克隆项目
```bash
git clone https://github.com/yourusername/rainbow-docs.git
cd rainbow-docs
```

#### 2. 配置环境变量
```bash
cp .env.example .env
# 编辑 .env 文件配置数据库和认证信息
```

#### 3. 初始化数据库
```bash
# 连接到 SurrealDB
surreal sql --conn http://localhost:8000 --user root --pass root --ns docs --db main

# 导入数据库架构
surreal import --conn http://localhost:8000 --user root --pass root --ns docs --db main schemas/docs_schema.sql
```

#### 4. 构建和运行
```bash
# 构建生产版本（不包含安装向导）
cargo build --release
cargo run

# 或直接运行
./target/release/rainbow-docs
```

#### 5. 验证安装
```bash
curl http://localhost:3000/health
```

### 🔄 安装状态检查机制

系统通过以下机制判断是否需要显示安装向导：

1. **首次启动**: 没有 `.rainbow_docs_installed` 文件 → 显示安装向导
2. **安装完成**: 创建 `.rainbow_docs_installed` 文件 → 正常运行模式  
3. **重新安装**: 删除 `.rainbow_docs_installed` 文件 → 重新显示安装向导

```bash
# 重新进入安装模式（用于测试）
rm -f .rainbow_docs_installed
cargo run --features installer
```

### 🛠️ 构建脚本使用

项目提供了便捷的构建脚本：

```bash
# 构建安装版本（包含安装向导）
./build.sh installer

# 构建生产版本（不包含安装向导）
./build.sh production

# 构建开发版本
./build.sh dev

# 查看帮助
./build.sh
```

### 📋 前后端配合说明

#### 前端检测逻辑
前端通过 API 请求检查安装状态：
```javascript
// 前端发送请求检查安装状态
const response = await fetch('/api/install/status')
const data = await response.json()

if (!data.data.is_installed) {
  // 显示安装向导界面
  return <InstallWizard />
} else {
  // 显示正常应用界面
  return <NormalApp />
}
```

#### 请求路由
- 前端访问: `/api/install/status`
- 通过代理转发到后端: `http://localhost:3000/api/install/status`
- 后端检查 `.rainbow_docs_installed` 文件并返回状态

#### 完整测试流程
```bash
# 1. 启动后端（安装模式）
cd Rainbow-Docs
cargo run --features installer

# 2. 启动前端（新终端）
cd Rainbow-Docs-Front
npm run dev

# 3. 浏览器访问 http://localhost:5173
# 4. 前端自动检测并显示安装向导
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

### 🧑‍🤝‍🧑 空间成员管理

#### 获取空间成员列表
```http
GET /api/docs/spaces/{space_slug}/members
```

**权限要求**: 需要 `members.manage` 权限

**响应示例:**
```json
{
  "success": true,
  "data": [
    {
      "id": "member:123",
      "space_id": "space:456",
      "user_id": "user:789",
      "role": "editor",
      "permissions": ["docs.read", "docs.write"],
      "status": "accepted",
      "invited_at": "2024-01-01T00:00:00Z",
      "accepted_at": "2024-01-01T01:00:00Z",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ],
  "message": "Members retrieved successfully"
}
```

#### 邀请成员加入空间
```http
POST /api/docs/spaces/{space_slug}/invite
Content-Type: application/json
```

**权限要求**: 需要 `members.invite` 权限

**请求体:**
```json
{
  "email": "user@example.com",
  "role": "editor",
  "message": "欢迎加入我们的文档空间！",
  "expires_in_days": 7
}
```

**或者通过用户ID直接邀请:**
```json
{
  "user_id": "user:123",
  "role": "member",
  "message": "邀请你加入我们的项目文档",
  "expires_in_days": 30
}
```

**角色说明:**
- `owner` - 空间所有者（自动拥有所有权限）
- `admin` - 管理员（可管理文档、邀请和管理成员）
- `editor` - 编辑者（可读写文档）
- `member` - 成员（可读写文档）
- `viewer` - 阅读者（只读权限）

**响应示例:**
```json
{
  "success": true,
  "data": {
    "id": "invitation:789",
    "space_id": "space:456",
    "email": "user@example.com",
    "invite_token": "abc123-def456-ghi789",
    "role": "editor",
    "permissions": ["docs.read", "docs.write"],
    "invited_by": "user:current",
    "message": "欢迎加入我们的文档空间！",
    "expires_at": "2024-01-22T10:30:00Z",
    "created_at": "2024-01-15T10:30:00Z"
  },
  "message": "Invitation sent successfully"
}
```

#### 接受邀请
```http
POST /api/docs/spaces/invitations/accept
Content-Type: application/json
```

**请求体:**
```json
{
  "invite_token": "abc123-def456-ghi789"
}
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "id": "member:new123",
    "space_id": "space:456",
    "user_id": "user:current",
    "role": "editor",
    "permissions": ["docs.read", "docs.write"],
    "status": "accepted",
    "invited_at": "2024-01-15T10:30:00Z",
    "accepted_at": "2024-01-15T11:00:00Z",
    "created_at": "2024-01-15T11:00:00Z",
    "updated_at": "2024-01-15T11:00:00Z"
  },
  "message": "Invitation accepted successfully"
}
```

#### 更新成员权限
```http
PUT /api/docs/spaces/{space_slug}/members/{user_id}
Content-Type: application/json
```

**权限要求**: 需要 `members.manage` 权限

**请求体:**
```json
{
  "role": "admin",
  "permissions": ["docs.read", "docs.write", "docs.admin", "members.invite"]
}
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "id": "member:123",
    "space_id": "space:456",
    "user_id": "user:789",
    "role": "admin",
    "permissions": ["docs.read", "docs.write", "docs.admin", "members.invite"],
    "status": "accepted",
    "updated_at": "2024-01-15T12:00:00Z"
  },
  "message": "Member updated successfully"
}
```

#### 移除成员
```http
DELETE /api/docs/spaces/{space_slug}/members/{user_id}
```

**权限要求**: 需要 `members.remove` 权限

**响应示例:**
```json
{
  "success": true,
  "data": null,
  "message": "Member removed successfully"
}
```

**注意事项:**
- 空间所有者不能被移除
- 用户不能移除自己
- 只有拥有 `members.remove` 权限的用户可以移除其他成员

### 权限系统详解

#### 角色权限表

| 角色 | 权限 | 说明 |
|-----|-----|------|
| **Owner** | 所有权限 | 空间创建者，拥有所有操作权限 |
| **Admin** | `docs.*`, `members.*` | 可管理文档和成员 |
| **Editor** | `docs.read`, `docs.write` | 可读写文档 |
| **Member** | `docs.read`, `docs.write` | 可读写文档（与Editor相同） |
| **Viewer** | `docs.read` | 只读权限 |

#### 权限代码说明

| 权限代码 | 说明 |
|---------|------|
| `docs.read` | 读取文档 |
| `docs.write` | 创建和编辑文档 |
| `docs.delete` | 删除文档 |
| `docs.admin` | 文档管理权限 |
| `space.admin` | 空间管理权限 |
| `space.delete` | 删除空间权限 |
| `members.invite` | 邀请成员 |
| `members.manage` | 管理成员权限 |
| `members.remove` | 移除成员 |

#### 权限继承

1. **空间权限**: 用户在空间中的权限决定了对该空间内所有文档的访问权限
2. **公开空间**: 任何人都可以读取公开空间的文档（如果文档也是公开的）
3. **私有空间**: 只有空间成员可以访问私有空间的文档
4. **所有者权限**: 空间所有者自动拥有该空间的所有权限

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

### 🔔 通知系统

#### 获取通知列表
```http
GET /api/docs/notifications
```

**查询参数:**
- `page` (可选): 页码，默认为1
- `limit` (可选): 每页数量，默认为20
- `unread_only` (可选): 是否只显示未读通知，默认为false

**响应示例:**
```json
{
  "success": true,
  "data": {
    "notifications": [
      {
        "id": "notification:123",
        "user_id": "user:456",
        "type": "space_invitation",
        "title": "张三 邀请您加入 项目文档 空间",
        "content": "张三 邀请您以 编辑者 的身份加入 项目文档 空间。",
        "data": {
          "space_name": "项目文档",
          "invite_token": "abc123-def456",
          "role": "editor",
          "inviter_name": "张三"
        },
        "is_read": false,
        "read_at": null,
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-15T10:30:00Z"
      }
    ],
    "total": 5
  },
  "message": "Notifications retrieved successfully"
}
```

#### 获取未读通知数量
```http
GET /api/docs/notifications/unread-count
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "count": 3
  },
  "message": "Unread count retrieved successfully"
}
```

#### 标记通知为已读
```http
PUT /api/docs/notifications/{notification_id}
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "id": "notification:123",
    "is_read": true,
    "read_at": "2024-01-15T11:00:00Z"
  },
  "message": "Notification marked as read"
}
```

#### 标记所有通知为已读
```http
POST /api/docs/notifications/mark-all-read
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "updated_count": 5
  },
  "message": "All notifications marked as read"
}
```

#### 删除通知
```http
DELETE /api/docs/notifications/{notification_id}
```

**响应示例:**
```json
{
  "success": true,
  "data": null,
  "message": "Notification deleted successfully"
}
```

#### 通知类型说明

| 通知类型 | 说明 | 数据字段 |
|---------|------|----------|
| `space_invitation` | 空间邀请通知 | `space_name`, `invite_token`, `role`, `inviter_name` |
| `document_shared` | 文档分享通知 | `document_id`, `document_title`, `sharer_name` |
| `comment_mention` | 评论提及通知 | `comment_id`, `document_id`, `commenter_name` |
| `document_update` | 文档更新通知 | `document_id`, `document_title`, `updater_name` |
| `system` | 系统通知 | 自定义数据 |

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

#### 空间成员表 (space_member)
- `id` - 成员关系唯一标识
- `space_id` - 所属空间
- `user_id` - 成员用户ID
- `role` - 成员角色（owner/admin/editor/member/viewer）
- `permissions` - 权限列表
- `invited_by` - 邀请者用户ID
- `invited_at` - 邀请时间
- `accepted_at` - 接受时间
- `status` - 状态（pending/accepted/rejected/removed）
- `expires_at` - 邀请过期时间

#### 空间邀请表 (space_invitation)
- `id` - 邀请唯一标识
- `space_id` - 目标空间
- `email` - 被邀请人邮箱
- `user_id` - 被邀请人用户ID（如果已注册）
- `invite_token` - 唯一邀请令牌
- `role` - 预设角色
- `permissions` - 预设权限列表
- `invited_by` - 邀请者
- `message` - 邀请消息
- `max_uses` - 最大使用次数
- `used_count` - 已使用次数
- `expires_at` - 邀请过期时间

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
- `user_id` - 接收者用户ID
- `type` - 通知类型（space_invitation/document_shared/comment_mention/document_update/system）
- `title` - 通知标题
- `content` - 通知内容
- `data` - 额外数据（JSON格式，如邀请令牌、空间名称等）
- `is_read` - 是否已读
- `read_at` - 阅读时间
- `created_at` - 创建时间
- `updated_at` - 更新时间

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
│   ├── space_member.rs  # 空间成员模型
│   ├── document.rs      # 文档模型
│   ├── version.rs       # 版本模型
│   ├── comment.rs       # 评论模型
│   ├── permission.rs    # 权限模型
│   ├── tag.rs          # 标签模型
│   ├── file.rs         # 文件模型
│   └── search.rs       # 搜索模型
├── routes/              # 路由处理
│   ├── spaces.rs       # 空间路由
│   ├── space_members.rs # 空间成员管理路由
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
│   ├── space_member.rs # 空间成员服务
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

### 数据库Schema更新

如需应用新的多租户权限系统，请执行以下数据库Schema更新：

```sql
-- 连接到 SurrealDB
surreal sql --conn http://localhost:8000 --user root --pass root --ns docs --db main

-- 应用新的空间成员和邀请表
```

然后复制并执行 `schemas/space_member_schema.sql` 中的内容，该文件包含：

1. **空间成员表** (`space_member`) - 管理用户在空间中的成员关系
2. **空间邀请表** (`space_invitation`) - 管理邀请流程
3. **相关索引** - 优化查询性能

**重要说明:**
- 新的权限系统向后兼容，现有空间所有者权限不受影响
- 升级后，空间所有者可以开始邀请成员并分配权限
- 公开空间的访问行为保持不变

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