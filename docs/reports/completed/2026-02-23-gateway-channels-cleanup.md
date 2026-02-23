# Gateway / Channels 功能重复清理

**日期**: 2026-02-23
**状态**: 已完成

## 概述

移除 zero-cli gateway 中与 zero-channels 重复的 IM 频道处理功能,确保所有 IM webhooks 统一由 zero-channels 服务处理。

## 变更内容

### 阶段 1: 从 Gateway 移除 IM 频道处理

**修改文件**: `services/zero-cli/src/gateway/mod.rs`

移除的代码:
- `WhatsAppVerifyQuery` 结构体
- `handle_whatsapp_verify()` 函数
- `handle_whatsapp_message()` 函数
- `verify_whatsapp_signature()` 函数
- `handle_feishu_event()` 函数
- 相关的路由注册: `/whatsapp` (GET/POST), `/feishu` (POST)
- `AppState` 中的字段: `whatsapp`, `whatsapp_app_secret`, `feishu`
- 相关的初始化代码
- 17 个 WhatsApp 签名验证测试

更新的代码:
- 添加启动提示,告知用户 IM 频道已移至 zero-channels 服务
- 清理未使用的 imports (`Bytes`, `Query`, `truncate_with_ellipsis`)

**修改文件**: `services/zero-cli/src/channels/mod.rs`
- 移除未使用的 re-exports: `FeishuChannel`, `WhatsAppChannel`

### 阶段 2: 为 Zero-Channels 添加 WhatsApp 支持

**修改文件**: `services/zero-channels/src/routes.rs`

新增:
- `WhatsAppVerifyQuery` 结构体
- `verify_whatsapp_signature()` 函数 (从 gateway 迁移)
- `whatsapp_verify()` 处理函数 (GET /webhook/whatsapp)
- `whatsapp_webhook()` 处理函数 (POST /webhook/whatsapp)
- `ChannelsState` 新增字段: `whatsapp`, `whatsapp_app_secret`
- 路由注册: `/webhook/whatsapp`
- 更新所有 state 创建函数以包含 WhatsApp 字段

**修改文件**: `services/zero-channels/src/lib.rs`

- `build_channels_router()` 新增 WhatsApp 频道初始化
- 支持从环境变量 `ZERO_BOT_WHATSAPP_APP_SECRET` 读取签名密钥
- 直接创建 `ChannelsState` 而非调用 `create_state_extended()`

**修改文件**: `services/zero-channels/src/outbound.rs`

- 新增 `whatsapp` 字段
- 新增 `with_whatsapp()` builder 方法
- 新增 `send_whatsapp()` 发送方法
- 更新 `send()` 以支持 `ChannelType::WhatsApp`

## 端口分配

清理后的端口职责更加清晰:

| 端口 | 服务 | 职责 |
|------|------|------|
| 4402 | zero-cli daemon (gateway) | Webhook (generic), Health, Pair, MCP |
| 4411 | zero-channels | 所有 IM 频道 (Telegram, Feishu, WhatsApp, WeChat Work, DingTalk) |

## 验证

- [x] `cargo build --package zero-cli --package zero-channels` 编译通过
- [x] `cargo test --package zero-channels` 全部 45 个测试通过
- [x] zero-cli 测试通过 (497/498, 1 个预先存在的环境变量测试失败)

## 迁移指南

如果用户之前配置了 WhatsApp/Feishu webhook 指向 gateway 端口 (4402):

**Before:**
```
Meta Webhook URL: https://your-domain.com:4402/whatsapp
Feishu Event URL: https://your-domain.com:4402/feishu
```

**After:**
```
Meta Webhook URL: https://your-domain.com:4411/webhook/whatsapp
Feishu Event URL: https://your-domain.com:4411/webhook/feishu
```

## 代码行数统计

| 操作 | 行数 |
|------|------|
| 删除 (gateway) | ~550 行 |
| 新增 (routes.rs) | ~180 行 |
| 新增 (lib.rs) | ~30 行 |
| 新增 (outbound.rs) | ~30 行 |
| 净减少 | ~310 行 |

通过将重复功能统一到 zero-channels,减少了约 310 行重复代码。
