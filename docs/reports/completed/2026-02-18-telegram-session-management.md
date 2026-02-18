# Telegram 会话管理实现报告

## 修改时间

2026-02-18

## 问题背景

当前 ZeroBot 的 Telegram 集成没有维护多轮对话上下文。每条消息都是独立处理的，用户说"继续刚才的话题"时，Agent 无法理解上文。

## 解决方案

实现按 `chat_id` 隔离的持久化会话管理，支持：
- 连续对话（默认保持上下文）
- `/new` 命令重开新会话
- `/compact` 命令手动压缩上下文
- 超出模型上限时自动压缩

## 实现详情

### 新增模块: `src/session/`

| 文件 | 功能 |
|------|------|
| `mod.rs` | 模块导出 |
| `types.rs` | `SessionMessage`, `MessageRole`, `SessionConfig` 数据结构 |
| `store.rs` | SQLite `SessionStore` 实现，持久化会话消息 |
| `compactor.rs` | `SessionCompactor` 上下文压缩器，使用 LLM 生成摘要 |

### 数据库 Schema

```sql
CREATE TABLE sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_key     TEXT NOT NULL,      -- "{channel}:{sender}"
    role            TEXT NOT NULL,      -- "user" | "assistant" | "system"
    content         TEXT NOT NULL,
    token_estimate  INTEGER NOT NULL,
    created_at      INTEGER NOT NULL
);
CREATE INDEX idx_session_key ON sessions(session_key);
CREATE INDEX idx_session_created ON sessions(session_key, created_at);
```

### 配置扩展

在 `~/.codecoder/config.json` 的 `zerobot` 部分添加：

```json
{
  "zerobot": {
    "session": {
      "enabled": true,
      "context_window": 128000,
      "compact_threshold": 0.8,
      "keep_recent": 5
    }
  }
}
```

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `enabled` | `true` | 是否启用会话管理 |
| `context_window` | `128000` | 模型上下文窗口大小（token） |
| `compact_threshold` | `0.8` | 触发自动压缩的阈值比例 |
| `keep_recent` | `5` | 压缩后保留的最近消息数 |

### 消息循环集成

修改 `src/channels/mod.rs`:

1. **初始化**: 创建 `SessionStore` 和 `SessionCompactor`
2. **命令处理**: 检测 `/new` 和 `/compact` 命令
3. **自动压缩**: 当 token 数超过阈值时自动压缩
4. **上下文注入**: 将历史消息格式化后添加到当前消息前
5. **消息保存**: 将用户消息和 AI 回复保存到会话

### Token 估算

使用简单的字符数估算：`token_count ≈ chars.div_ceil(4)`

对于压缩阈值判断足够准确，如需更精确可后续引入 tiktoken-rs。

## 文件修改列表

| 文件 | 操作 |
|------|------|
| `src/session/mod.rs` | 新建 |
| `src/session/types.rs` | 新建 |
| `src/session/store.rs` | 新建 |
| `src/session/compactor.rs` | 新建 |
| `src/lib.rs` | 添加 `session` 模块 |
| `src/main.rs` | 添加 `session` 模块 |
| `src/config/mod.rs` | 导出 `SessionConfig` |
| `src/config/schema.rs` | 添加 `SessionConfig` 和相关解析 |
| `src/channels/mod.rs` | 集成会话管理 |
| `src/onboard/wizard.rs` | 添加 session 字段到 Config 构造 |

## 测试

运行所有 session 相关测试：

```bash
cargo test session
```

测试覆盖：
- `SessionMessage` 和 `MessageRole` 序列化
- `SessionStore` CRUD 操作
- 会话隔离
- 压缩逻辑
- 持久化
- Unicode 和长内容

## 验证方案

```bash
# 启动 daemon
zero-bot daemon

# Telegram 测试流程
1. 发送 "你好"
2. 发送 "记住我喜欢 Rust"
3. 发送 "我喜欢什么语言？" → 应回答 Rust
4. 发送 "/new"
5. 发送 "我喜欢什么语言？" → 应不知道
6. 发送多条消息直到触发自动压缩
7. 验证压缩后仍能理解关键上下文
```

## 用户体验

| 命令 | 效果 |
|------|------|
| `/new` | 清空当前会话，开始新对话 |
| `/compact` | 手动压缩上下文，保留最近 N 条 + 摘要 |
| (自动) | 达到 80% 上下文窗口时自动压缩 |
