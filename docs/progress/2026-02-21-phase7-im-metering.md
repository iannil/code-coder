# Phase 7: IM 双向消息 + Token 计量

**日期**: 2026-02-21
**状态**: ✅ 已完成

## 实现概要

Phase 7 实现了 IM 双向消息和 Token 计量系统的核心功能，使 zero-channels 和 zero-gateway 能够实现完整的双向通信和用量计费。

## 完成的任务

### 1. OutboundRouter (zero-channels/src/outbound.rs)
- ✅ 统一的出站消息路由接口
- ✅ Pending 消息追踪（用于响应路由）
- ✅ 多渠道支持（Telegram/Feishu）
- ✅ TTL 清理机制（防止内存泄漏）

### 2. CodeCoderBridge (zero-channels/src/bridge.rs)
- ✅ 完整的消息流处理
- ✅ CodeCoder API 调用封装
- ✅ 响应路由回原渠道
- ✅ 错误处理和用户提示
- ✅ 后台处理器模式 (`spawn_processor`)

### 3. Token Metering (zero-gateway/src/metering.rs)
- ✅ 中间件拦截请求/响应
- ✅ 支持 Anthropic 和 OpenAI 格式的 usage 字段
- ✅ 配额检查（请求前）
- ✅ 用量记录（响应后）
- ✅ UsageReport 生成

### 4. Quota API Endpoints (zero-gateway/src/routes.rs)
- ✅ `GET /api/v1/quota` - 获取当前用户配额
- ✅ `GET /api/v1/quota/:user_id` - 获取指定用户配额
- ✅ `PUT /api/v1/quota/:user_id` - 设置用户配额限制

## 数据流

```
用户 IM 消息 → webhook → zero-channels → CodeCoderBridge
                              ↓
                    register_pending()
                              ↓
              forward to CodeCoder API
                              ↓
                    extract response
                              ↓
                 OutboundRouter.respond()
                              ↓
用户 ← send via channel ← route to original
```

## 新增文件

| 文件 | 描述 |
|------|------|
| `services/zero-channels/src/outbound.rs` | 出站消息路由系统 |
| `services/zero-channels/src/bridge.rs` | CodeCoder API 桥接 |
| `services/zero-gateway/src/metering.rs` | Token 计量中间件 |

## 修改的文件

| 文件 | 修改 |
|------|------|
| `services/zero-channels/src/lib.rs` | 添加新模块导出，更新 `start_server` |
| `services/zero-gateway/src/lib.rs` | 添加 metering 模块 |
| `services/zero-gateway/src/routes.rs` | 添加配额 API，集成 metering 中间件 |

## 测试覆盖

### Gateway 测试 (22 tests)
- `test_get_my_quota` - 获取当前用户配额
- `test_get_quota_unauthenticated` - 未认证访问被拒绝
- `test_set_user_quota_as_admin` - 管理员设置用户配额
- `test_regular_user_cannot_set_quota` - 普通用户不能设置配额
- `test_user_can_check_own_quota` - 用户可以查看自己的配额

### Channels 测试 (23 tests)
- `test_outbound_router_pending_registration` - Pending 消息注册
- `test_outbound_router_take_pending` - 取出 Pending 消息
- `test_outbound_router_cleanup_stale` - 清理过期消息
- `test_outbound_router_send_without_channel` - 无渠道发送失败
- `test_outbound_router_respond_without_pending` - 无 Pending 响应失败
- `test_bridge_creation` - Bridge 创建
- `test_chat_request_serialization` - 请求序列化
- `test_chat_response_deserialization` - 响应反序列化

## API 格式

### 配额响应格式
```json
{
  "user_id": "user-123",
  "usage": {
    "user_id": "user-123",
    "period": "daily",
    "input_tokens": 1500,
    "output_tokens": 800,
    "total_tokens": 2300,
    "requests": 5,
    "limit_input": 1000000,
    "limit_output": 500000,
    "percentage_used": 0.23
  },
  "limits": {
    "daily_input_tokens": 1000000,
    "daily_output_tokens": 500000,
    "daily_requests": 1000,
    "monthly_input_tokens": 10000000,
    "monthly_output_tokens": 5000000
  }
}
```

### Token Usage 提取
支持两种格式:
- Anthropic: `{ "usage": { "input_tokens": N, "output_tokens": M } }`
- OpenAI: `{ "usage": { "prompt_tokens": N, "completion_tokens": M } }`

## 后续工作 (Phase 8)

1. Git Code Review 自动化
   - 解析 GitHub/GitLab PR 事件
   - 调用 code-reviewer Agent
   - 将结果评论回 PR

2. 多模型并行推理 (Phase 9)
   - `/api/v1/parallel` 端点
   - 并行调用多个 provider

---

*记录时间: 2026-02-21*
*总测试数: 107 (全部通过)*
