# HITL 审批系统用户指南

Human-in-the-Loop (HITL) 是 CodeCoder 的审批队列系统，用于在执行高风险操作前获取人工确认。支持通过多种 IM 渠道（Telegram、Slack、飞书、钉钉）进行审批。

## 概述

### 什么是 HITL？

HITL 系统提供：
- 集中式审批工作流
- 多 IM 渠道集成
- 风险级别评估
- 审批历史和审计日志
- 超时自动处理

### 工作流程

```
操作请求 → HITL 系统 → IM 渠道卡片 → 用户决策 → 执行/拒绝
                ↓
          审批存储 (持久化)
```

## 审批类型

HITL 支持以下类型的审批请求：

### 1. 代码合并 (MergeRequest)

用于代码审查和合并批准：

```json
{
  "type": "merge_request",
  "platform": "github",
  "repo": "org/repo",
  "mr_id": 123
}
```

### 2. 交易命令 (TradingCommand)

用于高风险交易操作：

```json
{
  "type": "trading_command",
  "asset": "BTC",
  "action": "buy",
  "amount": 1.5
}
```

### 3. 配置变更 (ConfigChange)

用于系统配置修改：

```json
{
  "type": "config_change",
  "key": "max_tokens",
  "old_value": "1000",
  "new_value": "2000"
}
```

### 4. 高成本操作 (HighCostOperation)

用于成本高昂的操作：

```json
{
  "type": "high_cost_operation",
  "operation": "deploy_cluster",
  "estimated_cost": 1500.0
}
```

### 5. 风险操作 (RiskOperation)

用于评估风险的通用操作：

```json
{
  "type": "risk_operation",
  "description": "Delete production data",
  "risk_level": "Critical"
}
```

## 风险级别

| 级别 | 值 | 说明 |
|------|-----|------|
| `Low` | 1 | 低风险 - 影响最小，易于恢复 |
| `Medium` | 2 | 中风险 - 中等影响，可逆 |
| `High` | 3 | 高风险 - 重大影响，难以恢复 |
| `Critical` | 4 | 关键风险 - 严重影响，不可逆 |

## 审批状态

| 状态 | 说明 |
|------|------|
| `Pending` | 等待审批 |
| `Approved` | 已批准 |
| `Rejected` | 已拒绝 |
| `Cancelled` | 已取消（超时或手动取消） |

## IM 渠道集成

### 支持的渠道

| 渠道 | 状态 | 说明 |
|------|------|------|
| Telegram | ✅ 已实现 | 支持 inline 按钮 |
| Slack | ✅ 已实现 | 支持 Block Kit |
| 飞书 (Feishu) | ✅ 已实现 | 支持消息卡片 |
| 钉钉 (DingTalk) | ✅ 已实现 | 支持互动卡片 |

### 审批卡片示例

**Telegram:**
```
🔔 审批请求

类型: 交易命令
标题: Buy 0.5 BTC @ $65,000
请求者: trader-hand
风险: High

[✅ 批准] [❌ 拒绝]
```

**Slack:**
```
┌─────────────────────────────────────┐
│ 🔔 审批请求                          │
│                                      │
│ **类型:** 交易命令                    │
│ **标题:** Buy 0.5 BTC @ $65,000     │
│ **请求者:** trader-hand              │
│ **风险:** 🔴 High                    │
│                                      │
│ [批准] [拒绝]                        │
└─────────────────────────────────────┘
```

## HTTP API

HITL 服务运行在 `zero-gateway` (端口 4430)：

### 创建审批请求

```http
POST /api/v1/hitl/request
Content-Type: application/json

{
  "approval_type": {
    "type": "trading_command",
    "asset": "BTC",
    "action": "buy",
    "amount": 0.5
  },
  "requester": "trader-hand",
  "approvers": ["admin", "risk-manager"],
  "title": "Buy 0.5 BTC @ $65,000",
  "description": "Market order from trader hand",
  "channel": "telegram",
  "metadata": {
    "exchange": "binance"
  },
  "ttl_seconds": 3600
}
```

**响应:**
```json
{
  "success": true,
  "approval": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "approval_type": { ... },
    "status": { "status": "pending" },
    "created_at": "2026-02-28T10:30:00Z",
    ...
  }
}
```

### 列出待审批请求

```http
GET /api/v1/hitl/pending
GET /api/v1/hitl/pending?approver_id=admin
```

**响应:**
```json
{
  "requests": [
    {
      "id": "...",
      "title": "Buy 0.5 BTC",
      "status": { "status": "pending" },
      ...
    }
  ],
  "total": 1
}
```

### 获取审批详情

```http
GET /api/v1/hitl/:id
```

### 处理审批决策

```http
POST /api/v1/hitl/:id/decide
Content-Type: application/json

{
  "decided_by": "admin",
  "approved": true,
  "reason": null
}
```

或拒绝：
```json
{
  "decided_by": "admin",
  "approved": false,
  "reason": "风险过高，需要更多信息"
}
```

### IM 渠道回调

各 IM 平台的回调端点：

```http
POST /api/v1/hitl/callback/telegram
POST /api/v1/hitl/callback/slack
POST /api/v1/hitl/callback/feishu
POST /api/v1/hitl/callback/dingtalk
```

## TypeScript 客户端

```typescript
// 创建 HITL 客户端（规划中）
import { HitLClient } from "@/hitl/client"

const client = new HitLClient({
  baseUrl: "http://127.0.0.1:4430"
})

// 创建审批请求
const approval = await client.createRequest({
  approvalType: {
    type: "trading_command",
    asset: "BTC",
    action: "buy",
    amount: 0.5
  },
  requester: "my-hand",
  approvers: ["admin"],
  title: "Buy BTC",
  channel: "telegram"
})

// 列出待审批
const pending = await client.listPending()
const myPending = await client.listPending("admin")

// 获取详情
const detail = await client.get(approval.id)

// 批准/拒绝
await client.approve(approval.id, "admin")
await client.reject(approval.id, "admin", "风险过高")
```

## 审计日志

所有审批操作都会记录审计日志，存储在 SQLite 数据库中：

| 字段 | 说明 |
|------|------|
| `request_id` | 审批请求 ID |
| `action` | 操作类型 (create/approve/reject/cancel) |
| `actor` | 操作者 |
| `timestamp` | 时间戳 |
| `details` | 详细信息 (JSON) |

## 与 Hands 系统集成

Hands 可以通过 HITL 系统请求人工审批：

```yaml
---
id: "trading-executor"
agent: "trader"
autonomy:
  level: "wild"
  auto_approve:
    enabled: true
    risk_threshold: "medium"  # medium 以上需要审批
---

# Trading Executor

执行交易时，高风险操作会自动创建审批请求。
```

当 Hand 执行遇到高风险操作时：
1. 自动创建 HITL 审批请求
2. 发送到配置的 IM 渠道
3. 等待人工批准或拒绝
4. 根据结果继续或中止执行

## 配置

在 `~/.codecoder/config.json` 中配置 HITL：

```json
{
  "hitl": {
    "default_channel": "telegram",
    "default_approvers": ["admin"],
    "ttl_seconds": 3600,
    "channels": {
      "telegram": {
        "bot_token": "...",
        "chat_id": "..."
      },
      "slack": {
        "webhook_url": "..."
      }
    }
  }
}
```

## 故障排除

### 审批卡片未发送

1. 检查 IM 渠道配置
2. 验证 Bot Token/Webhook URL 有效
3. 确认 `zero-gateway` 服务运行中
4. 查看日志：`./ops.sh logs zero-gateway`

### 回调处理失败

1. 检查回调 URL 是否可访问
2. 验证 Webhook 签名配置
3. 查看网络连接状态

### 审批超时

审批请求默认 1 小时后超时（可通过 `ttl_seconds` 配置）。超时后状态变为 `Cancelled`。

## 安全考虑

1. **权限控制**: 只有指定的 approvers 可以批准请求
2. **审计追踪**: 所有操作都有完整的审计日志
3. **超时机制**: 防止请求无限期挂起
4. **渠道验证**: 验证 IM 平台回调的签名

## 相关文档

- [Hands 系统](./HANDS.md) - 自动化任务系统
- [Agent 架构](../architecture/README.md) - Agent 系统概述
