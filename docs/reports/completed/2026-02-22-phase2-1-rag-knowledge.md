# Phase 2.1: RAG 知识库实现报告

**日期**: 2026-02-22
**状态**: ✅ 已完成

## 实现概述

实现了 RAG (Retrieval-Augmented Generation) 知识库功能，支持文档上传、语义搜索和 ZeroBot 集成。

## 实现文件

| 文件 | 描述 |
|------|------|
| `packages/ccode/src/api/server/handlers/knowledge.ts` | 知识库 API 处理器 |
| `packages/ccode/src/api/server/router.ts` | 添加知识库路由 |
| `packages/web/src/lib/types.ts` | 添加知识库类型定义 |
| `services/zero-channels/src/bridge.rs` | ZeroBot 知识搜索集成 |

## API 端点

```
POST /api/v1/knowledge/upload     # 上传文档
GET  /api/v1/knowledge/documents  # 列出已索引文档
DELETE /api/v1/knowledge/documents/:id  # 删除文档
POST /api/v1/knowledge/search     # 语义搜索
GET  /api/v1/knowledge/health     # 健康检查
```

## 技术架构

### 1. 文档处理流程

```
Document → Markdown Chunker → Embedding API → SQLite Storage
                   ↓
            Chunk + Heading + Index
```

### 2. 搜索架构

```
Query → Embedding → Vector Search ──┐
                                    ├── Hybrid Merge (0.7 + 0.3) → Results
Query → FTS5 ─────→ BM25 Search ────┘
```

### 3. 核心组件

- **Chunking**: 基于 Markdown 标题的语义分块，保留 heading 上下文
- **Embedding**: OpenAI text-embedding-3-small (1536 维)
- **Storage**: SQLite + FTS5 全文索引
- **Search**: 混合搜索 (Vector 70% + BM25 30%)

## ZeroBot 集成

### 意图检测模式

**中文**:
- `@知识库 <query>` / `@知识 <query>`
- `帮我查一下...`
- `搜索一下...`
- `文档里有关于...的内容吗`

**英文**:
- `@knowledge <query>` / `@kb <query>`
- `search for <query>`

### 响应格式

```markdown
📚 **知识库搜索结果**

🔍 查询: <query>
📊 找到 N 条相关内容

🟢 ### 1. <heading> (85%)
<content snippet>
_来源: filename.md_

---

🟡 ### 2. <heading> (65%)
<content snippet>
_来源: filename.md_

🔄 搜索模式: hybrid
```

## 测试验证

### Rust 测试 (18 passed)

```bash
cargo test --package zero-channels -- bridge
```

- `test_knowledge_question_detection_chinese` ✅
- `test_knowledge_question_detection_english` ✅
- `test_knowledge_request_serialization` ✅
- `test_knowledge_response_deserialization` ✅
- `test_format_knowledge_response` ✅
- `test_format_knowledge_response_empty` ✅

### TypeScript 编译

```bash
bun turbo typecheck --filter=ccode
```

knowledge.ts 无错误。

## 使用示例

### 1. 上传文档

```bash
curl -X POST http://localhost:4400/api/v1/knowledge/upload \
  -H "Content-Type: application/json" \
  -d '{
    "content": "# 员工手册\n\n## 福利待遇\n\n公司提供以下福利...",
    "filename": "员工手册.md",
    "mime_type": "text/markdown"
  }'
```

### 2. 搜索

```bash
curl -X POST http://localhost:4400/api/v1/knowledge/search \
  -H "Content-Type: application/json" \
  -d '{"query": "公司福利政策", "limit": 5}'
```

### 3. ZeroBot 使用

```
用户: @知识库 公司的年假政策是什么
Bot: 📚 知识库搜索结果...
```

## 配置说明

### 环境变量

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `OPENAI_API_KEY` | 启用向量搜索 | - |
| `OPENAI_BASE_URL` | 自定义 API 端点 | `https://api.openai.com` |

### 数据存储

- 数据库路径: `~/.codecoder/knowledge/knowledge.db`
- 支持格式: `text/markdown`, `text/plain`

## 后续任务

- [ ] 添加 PDF 支持
- [ ] 支持批量上传
- [ ] Web UI 文档管理界面
- [ ] 知识库分组/标签功能
