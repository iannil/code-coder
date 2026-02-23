# Phase 6: 跨平台素材捕获 (Cross-platform Asset Capture)

## 完成时间
2026-02-22

## 概述

实现了跨平台内容捕获系统，允许用户通过 IM 渠道（Telegram、微信、钉钉、飞书等）转发内容到 ZeroBot，自动提取、摘要、标签，并保存到知识库（飞书文档、Notion）。

## 实现内容

### 1. 配置结构 (`services/zero-common/src/config.rs`)

新增配置结构：
- `CaptureConfig` - 主配置结构
- `FeishuDocsConfig` - 飞书文档存储配置
- `NotionConfig` - Notion 存储配置
- `AutoCaptureConfig` - 自动捕获规则配置

```json
{
  "channels": {
    "capture": {
      "enabled": true,
      "feishu_docs": {
        "app_id": "cli_xxx",
        "app_secret": "xxx",
        "folder_token": "fldcnXXX"
      },
      "notion": {
        "token": "secret_xxx",
        "database_id": "xxx"
      },
      "auto_capture": {
        "capture_forwarded": true,
        "capture_links": false,
        "trigger_prefixes": ["#收藏", "#save", "@save"]
      }
    }
  }
}
```

### 2. 捕获桥接器 (`services/zero-channels/src/capture_bridge.rs`)

核心组件（约 950 行代码）：

| 组件 | 职责 |
|------|------|
| `CaptureBridge` | 主桥接器，协调内容提取、LLM 调用、存储 |
| `FeishuDocsClient` | 飞书文档 API 客户端（创建文档、插入内容块） |
| `NotionClient` | Notion API 客户端（创建页面、构建内容块） |
| `CapturedAsset` | 捕获的素材数据结构 |
| `AssetContentType` | 内容类型枚举（Article, Tweet, Image, Document, Link, RawText） |

主要方法：
- `is_capturable()` - 检测消息是否可捕获（转发、链接、触发前缀）
- `is_capture_request()` - 检测是否为捕获请求
- `capture()` - 执行捕获流程
- `extract_link_content()` - 提取链接内容
- `summarize_and_tag()` - 使用 LLM 生成摘要和标签
- `save_to_feishu_docs()` / `save_to_notion()` - 保存到知识库
- `capture_url()` - 通过 API 直接捕获 URL

### 3. 消息处理集成 (`services/zero-channels/src/bridge.rs`)

- 添加 `capture_bridge` 字段到 `CodeCoderBridge`
- 添加 `with_capture()` 构建方法
- 在 `process()` 方法中集成捕获检测（优先级最高）

### 4. HTTP API 端点 (`services/zero-channels/src/routes.rs`)

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/capture` | POST | 捕获 URL 内容 |
| `/api/v1/capture/history` | GET | 获取捕获历史 |
| `/api/v1/capture/:asset_id` | GET | 获取单个素材详情 |
| `/api/v1/capture/:asset_id/save` | POST | 重新保存到其他平台 |

## 架构流程

```
用户转发消息 / 发送链接
    │
    ▼
CodeCoderBridge.process()
    │
    ├── is_capturable()? ─────────────┐
    │                                  │
    ▼                                  ▼
is_capture_request()            正常消息处理
    │
    ▼
CaptureBridge.capture()
    │
    ├── extract_link_content() ◄──────┐
    │   (如果是链接)                   │
    │                                  │
    ├── summarize_and_tag() ──────────┤
    │   (调用 CodeCoder LLM API)      │
    │                                  │
    ├── save_to_feishu_docs() ────────┤
    │                                  │
    └── save_to_notion() ─────────────┘
    │
    ▼
返回确认消息给用户
```

## 使用示例

### 通过 Telegram 转发收藏

```
用户: [转发一篇文章] #收藏
ZeroBot: 📥 **已捕获内容**

📝 **摘要**: 这篇文章介绍了 Rust 异步编程的最佳实践...

🏷️ **标签**: Rust, 异步编程, 技术

📌 **要点**:
  • 使用 tokio 作为异步运行时
  • 避免阻塞操作

💾 **已保存到**:
  • [feishu_docs](https://bytedance.feishu.cn/docx/xxx)
```

### 通过 API 捕获

```bash
# 捕获 URL
curl -X POST http://localhost:4405/api/v1/capture \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/article", "destination": "feishu_docs"}'

# 查看历史
curl http://localhost:4405/api/v1/capture/history?limit=10

# 获取详情
curl http://localhost:4405/api/v1/capture/asset-123

# 保存到新平台
curl -X POST http://localhost:4405/api/v1/capture/asset-123/save \
  -d '{"destination": "notion"}'
```

## 测试

16 个单元测试全部通过：

```
cargo test -p zero-channels capture
cargo test -p zero-common capture
```

测试覆盖：
- 内容类型检测
- 消息可捕获性判断
- 捕获请求检测
- URL 提取
- HTML 标题/内容提取
- LLM 响应解析
- 历史记录管理

## 文件变更

| 文件 | 操作 | 描述 |
|------|------|------|
| `services/zero-common/src/config.rs` | 修改 | 添加 CaptureConfig 及相关结构 |
| `services/zero-channels/src/capture_bridge.rs` | 新建 | 捕获桥接器核心实现 |
| `services/zero-channels/src/bridge.rs` | 修改 | 集成捕获检测 |
| `services/zero-channels/src/routes.rs` | 修改 | 添加 Capture API 端点 |
| `services/zero-channels/src/lib.rs` | 修改 | 添加模块导出 |
| `services/zero-channels/Cargo.toml` | 修改 | 添加 html-escape 依赖 |

## 依赖

新增：
- `html-escape` 0.2 - HTML 实体解码

复用现有：
- `reqwest` - HTTP 客户端
- `regex` - URL 和内容提取
- `chrono` - 时间处理
- `serde_json` - JSON 处理

## 后续扩展建议

1. **更多存储目标**
   - Obsidian（本地 Markdown）
   - Apple Notes（macOS）
   - Readwise

2. **智能分类**
   - 自动归类到不同文件夹/数据库
   - 基于历史标签的智能推荐

3. **内容增强**
   - 自动提取关键引用
   - 生成思维导图
   - 关联已有笔记

4. **批量操作**
   - 批量导入浏览器书签
   - 定时清理低价值内容

5. **持久化存储**
   - 当前使用内存缓存（最多 100 条）
   - 可扩展为 SQLite 或 PostgreSQL 持久化
