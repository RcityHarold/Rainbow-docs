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
- **全文搜索**: 快速搜索文档内容
- **标签系统**: 灵活的文档分类和标记
- **智能推荐**: 基于用户行为的内容推荐

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
- Rust 1.70.0 或更高版本
- SurrealDB
- Rainbow-Auth (集成模式)

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
```

### 独立模式
独立运行，使用内置用户系统：

```env
RAINBOW_AUTH_INTEGRATION=false
JWT_SECRET=your-jwt-secret
```

## API 文档

### 文档空间管理
- `GET /api/spaces` - 获取文档空间列表
- `POST /api/spaces` - 创建新的文档空间
- `GET /api/spaces/{slug}` - 获取空间详情
- `PUT /api/spaces/{slug}` - 更新空间信息

### 文档管理
- `GET /api/spaces/{space}/docs` - 获取文档列表
- `POST /api/spaces/{space}/docs` - 创建新文档
- `GET /api/spaces/{space}/docs/{slug}` - 获取文档内容
- `PUT /api/spaces/{space}/docs/{slug}` - 更新文档
- `DELETE /api/spaces/{space}/docs/{slug}` - 删除文档

### 搜索功能
- `GET /api/search` - 全文搜索
- `GET /api/search/suggestions` - 搜索建议

### 评论系统
- `GET /api/docs/{id}/comments` - 获取评论列表
- `POST /api/docs/{id}/comments` - 添加评论
- `PUT /api/comments/{id}` - 更新评论
- `DELETE /api/comments/{id}` - 删除评论

## 数据库架构

### 核心表结构
- `space` - 文档空间
- `document` - 文档内容
- `document_version` - 版本历史
- `document_permission` - 权限控制
- `comment` - 评论系统
- `tag` - 标签系统
- `search_index` - 搜索索引

## 开发指南

### 项目结构
```
src/
├── main.rs              # 应用入口
├── config.rs            # 配置管理
├── error.rs             # 错误处理
├── models/              # 数据模型
├── routes/              # 路由处理
├── services/            # 业务逻辑
└── utils/               # 工具函数
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