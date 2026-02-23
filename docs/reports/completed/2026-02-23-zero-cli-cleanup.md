# Zero CLI 功能清理 - 完成报告

## 日期
2026-02-23

## 概述

清理了 `zero-cli` 中与 `zero-*` 服务重复的功能，遵循"CLI = 编排层/客户端"的架构原则。

## 完成的工作

### Phase 1: 迁移 Email Channel 到 zero-channels ✅

**变更的文件：**
- 创建 `services/zero-channels/src/email.rs` - 完整的 IMAP/SMTP Email 通道实现
- 更新 `services/zero-common/src/config.rs` - 添加 `EmailConfig` 配置结构
- 更新 `services/zero-channels/src/lib.rs` - 导出 EmailChannel 和集成到 build_channels_router
- 更新 `services/zero-channels/src/outbound.rs` - 添加 Email 出站路由支持
- 更新 `services/zero-channels/Cargo.toml` - 添加 email 依赖 (lettre, mail-parser, etc.)
- 删除 `services/zero-cli/src/channels/email_channel.rs`
- 更新 `services/zero-cli/src/channels/mod.rs` - 移除 email_channel 模块

**架构改进：**
- 所有 IM 通道逻辑集中在 `zero-channels`
- `zero-cli` 不再包含渠道业务逻辑

### Phase 2: 统一 Cron/Scheduler 到 zero-workflow ✅

**变更的文件：**
- 重写 `services/zero-workflow/src/scheduler.rs` - SQLite 持久化存储
- 重写 `services/zero-cli/src/cron/mod.rs` - 改为 HTTP API 客户端
- 删除 `services/zero-cli/src/cron/scheduler.rs`
- 更新 `services/zero-cli/src/config/schema.rs` - 添加 workflow_host/workflow_port 配置
- 更新 `services/zero-cli/src/daemon/mod.rs` - 移除本地 scheduler 组件
- 更新 `services/zero-cli/src/onboard/wizard.rs` - 添加新配置字段
- 更新 `services/zero-workflow/Cargo.toml` - 添加 rusqlite 依赖
- 更新 `services/zero-workflow/src/routes.rs` - 测试改用临时目录

**架构改进：**
- Cron jobs 现在由 `zero-workflow` 服务统一管理
- `zero-cli cron` 命令调用 `/api/v1/tasks` REST API
- 数据持久化在 `~/.codecoder/workflow/cron.db`

### Phase 3: 清理 zero-cli 依赖 ✅

**移除的依赖 (从 zero-cli):**
- `lettre` - SMTP 客户端 (已迁移到 zero-channels)
- `mail-parser` - IMAP 解析器 (已迁移到 zero-channels)
- `rustls-pki-types` - TLS 类型 (已迁移到 zero-channels)
- `tokio-rustls` - 异步 TLS (已迁移到 zero-channels)
- `webpki-roots` - 根证书 (已迁移到 zero-channels)
- `cron` - Cron 表达式解析 (不再在本地使用)

**保留的依赖:**
- `rusqlite` - 仍用于 memory/session 模块

## 验证结果

```bash
# 全工作区构建
cargo build --workspace  # ✅ 成功

# zero-channels 测试
cargo test --package zero-channels  # ✅ 180 passed

# zero-workflow 测试
cargo test --package zero-workflow routes::tests  # ✅ 8 passed

# zero-cli cron 测试
cargo test --package zero-cli --lib -- cron  # ✅ 2 passed
```

## API 端点

### Cron API (zero-workflow)
- `GET /api/v1/tasks` - 列出所有定时任务
- `POST /api/v1/tasks` - 创建定时任务
- `DELETE /api/v1/tasks/:id` - 删除定时任务

## 配置变更

### 新增配置项

```json
{
  "workflow_host": "127.0.0.1",  // 可选，默认 127.0.0.1
  "workflow_port": 4412          // 可选，默认 4412
}
```

### Email 配置 (zero-common)

```json
{
  "channels": {
    "email": {
      "enabled": true,
      "imap_host": "imap.example.com",
      "imap_port": 993,
      "imap_folder": "INBOX",
      "smtp_host": "smtp.example.com",
      "smtp_port": 587,
      "smtp_tls": true,
      "username": "user@example.com",
      "password": "...",
      "from_address": "bot@example.com",
      "poll_interval_secs": 60,
      "allowed_senders": ["*"]
    }
  }
}
```

## 架构图

```
┌──────────────┐     HTTP API      ┌─────────────────┐
│   zero-cli   │ ─────────────────▶│  zero-workflow  │
│  (cron cmd)  │  /api/v1/tasks    │   (scheduler)   │
└──────────────┘                   └─────────────────┘
                                          │
                                          ▼
                                   ┌─────────────────┐
                                   │  SQLite (cron)  │
                                   │  ~/.codecoder/  │
                                   │  workflow/      │
                                   └─────────────────┘

┌──────────────┐     Import        ┌─────────────────┐
│   zero-cli   │ ─────────────────▶│  zero-channels  │
│  (channels)  │  (library call)   │   (Email/IM)    │
└──────────────┘                   └─────────────────┘
```

## 风险评估

- **低风险**: Email 迁移是纯代码移动，API 不变
- **低风险**: Cron API 向后兼容，用户需确保 zero-workflow 服务运行

## 遗留问题

1. `economic_bridge::tests::test_severity_calculation` - 预先存在的测试失败，与本次变更无关
2. `memory_comparison` 集成测试失败 - 预先存在的问题，与本次变更无关

## 后续工作

1. 更新用户文档说明 cron 命令依赖 zero-workflow 服务
2. 考虑在 zero-cli daemon 中自动启动 zero-workflow
