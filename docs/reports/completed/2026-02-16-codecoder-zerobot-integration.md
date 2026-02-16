# CodeCoder + ZeroBot 整合实施报告

**日期**: 2026-02-16
**状态**: 已完成阶段 0-3

## 目录

- [实施概述](#实施概述)
- [完成的工作](#完成的工作)
- [技术细节](#技术细节)
- [验证步骤](#验证步骤)
- [后续工作](#后续工作)

## 实施概述

本次整合实现了 CodeCoder 与 ZeroBot 的融合，使两个项目能够协同工作：
- ZeroBot 通过 HTTP 调用 CodeCoder 的 23 个 AI Agent
- CodeCoder 可以访问 ZeroBot 的 SQLite 记忆数据库
- 新增 Agent HTTP API 端点供外部系统调用

## 完成的工作

### 阶段 0：合并代码库 ✅

**目录结构**:
```
agents-e374082453/
├── packages/           # TypeScript 包
│   └── ccode/          # CodeCoder 主包
├── services/           # 独立服务
│   └── zero-bot/       # ZeroBot (Rust) ← 新增
└── ...
```

**修改文件**:
- `.gitignore`: 添加 `services/zero-bot/target/`

### 阶段 1：基础集成 (HTTP 桥接) ✅

#### 1.1 创建 CodeCoder Tool (Rust)

**新建文件**: `services/zero-bot/src/tools/codecoder.rs`

功能：
- 调用 CodeCoder API 创建会话
- 发送消息到指定 Agent
- 轮询等待响应

可用参数：
- `agent`: Agent 名称 (build, plan, decision, macro, trader 等)
- `prompt`: 发送给 Agent 的消息
- `session_id`: 可选，复用已有会话

#### 1.2 注册 Tool 和配置

**修改文件**:
- `services/zero-bot/src/tools/mod.rs`: 导出 CodeCoderTool
- `services/zero-bot/src/config/schema.rs`: 添加 CodeCoderConfig
- `services/zero-bot/src/config/mod.rs`: 导出配置类型
- `services/zero-bot/src/agent/loop_.rs`: 传递配置到 all_tools
- `services/zero-bot/src/onboard/wizard.rs`: 初始化默认配置

#### 1.3 配置示例

**新建文件**: `services/zero-bot/examples/config.toml.example`

配置示例：
```toml
[codecoder]
enabled = true
endpoint = "http://localhost:4096"
```

### 阶段 2：记忆共享 ✅

**新建文件**:
- `packages/ccode/src/memory-zerobot/types.ts`: 类型定义
- `packages/ccode/src/memory-zerobot/provider.ts`: SQLite 读写实现
- `packages/ccode/src/memory-zerobot/index.ts`: 模块导出

功能：
- 直接读写 ZeroBot 的 SQLite 数据库 (`~/.codecoder/workspace/memory/brain.db`)
- 支持 FTS5 全文搜索
- 支持 store/recall/get/list/forget/count 操作

使用示例：
```typescript
import { createZeroBotMemory } from "@/memory-zerobot"

const memory = createZeroBotMemory()
if (memory.isAvailable()) {
  memory.store("user_preference", "Prefers Rust", "core")
  const results = memory.recall("programming", 5)
}
```

### 阶段 3：添加 Agent HTTP 端点 ✅

**新建文件**: `packages/ccode/src/api/server/handlers/agent.ts`

**修改文件**: `packages/ccode/src/api/server/router.ts`

新增 API 端点：
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/agents` | 列出所有可用 Agent |
| GET | `/api/agent/:agentId` | 获取单个 Agent 信息 |
| POST | `/api/agent/invoke` | 调用 Agent |

调用示例：
```bash
curl -X POST http://localhost:4096/api/agent/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "agent": "decision",
    "prompt": "使用 CLOSE 框架分析这个职业选择"
  }'
```

## 技术细节

### ZeroBot CodeCoder Tool 架构

```
ZeroBot Agent Loop
       │
       ▼
   codecoder tool
       │
       ├── POST /api/sessions (创建会话)
       │
       ├── POST /api/sessions/:id/messages (发送消息)
       │
       └── GET /api/sessions/:id/messages (轮询响应)
              │
              ▼
       CodeCoder API Server
```

### Memory 共享架构

```
CodeCoder (TypeScript)                ZeroBot (Rust)
         │                                  │
         │    bun:sqlite                    │    rusqlite
         │       │                          │       │
         └───────┼──────────────────────────┼───────┘
                 │                          │
                 ▼                          ▼
         ~/.codecoder/workspace/memory/brain.db
```

### SQLite Schema (from ZeroBot)

```sql
CREATE TABLE memories (
    id          TEXT PRIMARY KEY,
    key         TEXT NOT NULL UNIQUE,
    content     TEXT NOT NULL,
    category    TEXT NOT NULL DEFAULT 'core',
    embedding   BLOB,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

## 验证步骤

### 阶段 1 验证

```bash
# 1. 启动 CodeCoder API
cd packages/ccode && bun dev serve

# 2. 测试 API
curl http://localhost:4096/api/agents

# 3. 在另一个终端，编译 ZeroBot
cd services/zero-bot && cargo build

# 4. 配置 ZeroBot
# 编辑 ~/.codecoder/config.toml:
# [codecoder]
# enabled = true
# endpoint = "http://localhost:4096"

# 5. 启动 ZeroBot daemon
cargo run -- daemon

# 6. 通过 Telegram/CLI 发送测试消息
```

### 阶段 2 验证

```bash
# 1. 确保 ZeroBot 已初始化 (运行过 onboard)
zero-bot status

# 2. 在 CodeCoder 中测试 memory
# 添加测试代码使用 ZeroBotMemoryProvider
```

### 阶段 3 验证

```bash
# 列出 Agents
curl http://localhost:4096/api/agents

# 调用 Agent
curl -X POST http://localhost:4096/api/agent/invoke \
  -H "Content-Type: application/json" \
  -d '{"agent": "build", "prompt": "Hello"}'
```

## 后续工作

1. **SSE 实时响应**: 当前 CodeCoder Tool 使用轮询方式，可改为 SSE
2. **@agent 路由**: 在 ZeroBot Agent 循环中检测 `@agent-name` 指令自动路由
3. **记忆同步**: 添加双向同步，CodeCoder 的记忆也写入 ZeroBot
4. **Cron 任务**: 配置 ZeroBot 的定时任务调用 CodeCoder Agent
5. **向量嵌入**: 共享 embedding provider 配置

## 文件变更摘要

### 新增文件 (9)
- `services/zero-bot/` (整个目录从 `/Users/iannil/Code/zproducts/zero-bot` 复制)
- `services/zero-bot/src/tools/codecoder.rs`
- `services/zero-bot/examples/config.toml.example`
- `packages/ccode/src/memory-zerobot/types.ts`
- `packages/ccode/src/memory-zerobot/provider.ts`
- `packages/ccode/src/memory-zerobot/index.ts`
- `packages/ccode/src/api/server/handlers/agent.ts`

### 修改文件 (8)
- `.gitignore`
- `services/zero-bot/src/tools/mod.rs`
- `services/zero-bot/src/config/schema.rs`
- `services/zero-bot/src/config/mod.rs`
- `services/zero-bot/src/agent/loop_.rs`
- `services/zero-bot/src/onboard/wizard.rs`
- `services/zero-bot/CLAUDE.md`
- `packages/ccode/src/api/server/router.ts`
