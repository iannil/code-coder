# Conversation Store Redis 设计文档

**日期：** 2026-02-25
**状态：** 已批准
**作者：** Claude + User

## 背景

当前 `chat.ts` 中的 `conversationToSession` 使用内存 Map 存储 IM 渠道的 conversation_id 到内部 session_id 的映射。这导致：

1. **服务重启后映射丢失** - 用户上下文关联断开
2. **多实例无法共享** - 分布式部署时各实例维护独立映射
3. **无持久化** - 数据只存在于进程内存

## 需求

| 项目 | 选择 |
|------|------|
| 部署模式 | 多实例/分布式 |
| 共享存储 | Redis |
| TTL 策略 | 永不过期 |
| 性能策略 | 纯 Redis，无本地缓存 |

## 方案选型

评估了三种方案：

| 方案 | 描述 | 优缺点 |
|------|------|--------|
| **A. ioredis 直连** | 使用成熟的 Node.js Redis 客户端 | ✅ 生产验证、TypeScript 友好、支持 Cluster |
| B. Bun 原生 TCP | 自己实现 RESP 协议 | ❌ 维护成本高、不支持 Cluster |
| C. zero-gateway 代理 | 通过 HTTP 访问 Redis | ❌ 额外跳转、延迟更高 |

**选择方案 A：ioredis 直连**

## 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    API Server (多实例)                       │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    chat.ts                           │   │
│  │  - chat()                                            │   │
│  │  - clearConversation()                               │   │
│  │  - compactConversation()                             │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                  │
│  ┌───────────────────────▼─────────────────────────────┐   │
│  │              ConversationStore                       │   │
│  │  - get(conversationId) → sessionId                   │   │
│  │  - set(conversationId, sessionId)                    │   │
│  │  - delete(conversationId)                            │   │
│  │  - exists(conversationId) → boolean                  │   │
│  └───────────────────────┬─────────────────────────────┘   │
└──────────────────────────┼──────────────────────────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │    Redis     │
                    │  (共享存储)   │
                    └──────────────┘
```

## 数据模型

### Redis Key 格式

```
codecoder:conv:{conversation_id} → {session_id}
```

### 示例

```
codecoder:conv:telegram:123456789  →  "sess_abc123def456"
codecoder:conv:slack:C04ABCD1234   →  "sess_xyz789ghi012"
codecoder:conv:discord:987654321   →  "sess_qwe456rty789"
```

### 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| Key 前缀 | `codecoder:conv:` | 避免与其他服务冲突，便于批量管理 |
| 值类型 | String | 简单映射，无需复杂结构 |
| TTL | 不设置 | 用户选择永不过期 |
| 编码 | UTF-8 | conversation_id 可能包含中文 |

## 模块接口

**文件：** `packages/ccode/src/api/server/store/conversation.ts`

```typescript
import Redis from "ioredis"

export namespace ConversationStore {
  const KEY_PREFIX = "codecoder:conv:"

  let client: Redis | null = null

  /** 初始化 Redis 连接 */
  export async function init(redisUrl?: string): Promise<void>

  /** 获取 session_id，不存在返回 null */
  export async function get(conversationId: string): Promise<string | null>

  /** 设置映射关系 */
  export async function set(conversationId: string, sessionId: string): Promise<void>

  /** 删除映射（用于 /clear） */
  export async function delete_(conversationId: string): Promise<boolean>

  /** 检查映射是否存在 */
  export async function exists(conversationId: string): Promise<boolean>

  /** 健康检查 */
  export async function healthCheck(): Promise<boolean>

  /** 关闭连接（优雅退出） */
  export async function close(): Promise<void>
}
```

## chat.ts 改造

### Before (内存 Map)

```typescript
const conversationToSession = new Map<string, string>()

async function getOrCreateSession(conversationId: string | undefined): Promise<string> {
  if (conversationId) {
    const existingSessionId = conversationToSession.get(conversationId)
    if (existingSessionId) {
      // ...
    }
  }
  conversationToSession.set(conversationId, session.id)
  return session.id
}
```

### After (Redis)

```typescript
import { ConversationStore } from "../store/conversation"

async function getOrCreateSession(conversationId: string | undefined): Promise<string> {
  if (conversationId) {
    const existingSessionId = await ConversationStore.get(conversationId)
    if (existingSessionId) {
      try {
        await LocalSession.get(existingSessionId)
        return existingSessionId
      } catch {
        await ConversationStore.delete_(conversationId)
      }
    }
  }

  const session = await LocalSession.create({
    title: `Chat: ${new Date().toISOString()}`,
  })

  if (conversationId) {
    await ConversationStore.set(conversationId, session.id)
  }

  return session.id
}
```

### 改造点汇总

| 函数 | 改造内容 |
|------|----------|
| `getOrCreateSession()` | Map 操作改为 await ConversationStore 调用 |
| `clearConversation()` | `conversationToSession.delete()` → `await ConversationStore.delete_()` |
| `compactConversation()` | `conversationToSession.get/set()` → `await ConversationStore.get/set()` |

## 错误处理

| 场景 | 处理方式 |
|------|----------|
| Redis 连接失败（启动时） | 抛出错误，阻止服务启动 |
| Redis 连接断开（运行时） | ioredis 自动重连，操作等待重连 |
| Redis 操作超时 | 返回错误，由上层决定是否创建新 session |
| Session 不存在但映射存在 | 删除失效映射，创建新 session |

### 降级策略

```typescript
async function getOrCreateSession(conversationId: string | undefined): Promise<string> {
  if (conversationId) {
    try {
      const existingSessionId = await ConversationStore.get(conversationId)
      if (existingSessionId) {
        // ...正常流程
      }
    } catch (redisError) {
      // Redis 不可用时降级：直接创建新 session
      logLifecycleEvent(ctx, "error", {
        function: "getOrCreateSession",
        error: "Redis unavailable, creating new session",
        originalError: redisError.message,
      })
    }
  }
  // 创建新 session...
}
```

## 配置

### ~/.codecoder/config.json

```json
{
  "redis": {
    "url": "redis://localhost:6379",
    "password": null,
    "db": 0,
    "keyPrefix": "codecoder:",
    "connectTimeout": 5000,
    "commandTimeout": 3000,
    "maxRetriesPerRequest": 3
  }
}
```

### 环境变量覆盖

| 环境变量 | 作用 |
|----------|------|
| `REDIS_URL` | 完整连接 URL，覆盖 config.json |
| `REDIS_PASSWORD` | 密码，用于生产环境 |

### 优先级

```
redisUrl 参数 > REDIS_URL 环境变量 > config.json > 默认值
```

## 测试策略

### 单元测试

`packages/ccode/test/unit/api/store/conversation.test.ts`

- set and get
- delete
- exists
- healthCheck

### 集成测试

`packages/ccode/test/integration/chat-redis.test.ts`

| 测试场景 | 验证内容 |
|----------|----------|
| 新对话创建 | 发送消息 → 映射写入 Redis → 返回 session_id |
| 对话续接 | 同一 conversation_id → 返回同一 session_id |
| /clear 命令 | 映射从 Redis 删除 → 下次创建新 session |
| /compact 命令 | 映射更新为新 session_id |
| 多实例一致性 | 实例 A 写入 → 实例 B 能读取 |
| Redis 断连恢复 | 断开 → 重连 → 操作恢复正常 |

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `packages/ccode/package.json` | 修改 | 添加 `ioredis` 依赖 |
| `packages/ccode/src/api/server/store/conversation.ts` | 新增 | ConversationStore 模块 |
| `packages/ccode/src/api/server/handlers/chat.ts` | 修改 | 改用 ConversationStore |
| `packages/ccode/src/api/server/index.ts` | 修改 | 启动时初始化 Redis |
| `packages/ccode/src/config/config.ts` | 修改 | 添加 redis 配置 schema |
| `packages/ccode/test/unit/api/store/conversation.test.ts` | 新增 | 单元测试 |
| `packages/ccode/test/integration/chat-redis.test.ts` | 新增 | 集成测试 |

## 数据流

```
用户消息 (Telegram)
    ↓
zero-channels (bridge.rs)
    ↓ POST /api/v1/chat { conversation_id: "telegram:123" }
CodeCoder API Server
    ↓
ConversationStore.get("telegram:123")
    ↓ Redis GET codecoder:conv:telegram:123
    ↓
Session 存在? → 继续对话
Session 不存在? → 创建新 Session → ConversationStore.set()
    ↓
返回响应
```
