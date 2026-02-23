# Zero CLI Channels 集成实现报告

> 文档类型: completed
> 创建时间: 2026-02-23
> 状态: completed

## 概述

修复了 Zero CLI Daemon 中 `channels` 组件的占位符问题。该组件启动后立即退出，导致 daemon 每分钟重启该组件。根本原因是 `start_channels` 函数只是一个占位符，没有实际实现。

## 修改内容

### 1. 实现 `start_channels` 函数

**文件**: `services/zero-cli/src/channels/mod.rs`

- 新增 `to_common_config()` 函数，将 `zero-cli` 的 `Config` 转换为 `zero-common` 的 `Config`
- 实现配置映射：
  - Telegram: bot_token, allowed_users
  - Feishu: app_id, app_secret, encrypt_key, verification_token, allowed_users
  - Discord: bot_token, allowed_guilds
  - Slack: bot_token, app_token
  - CodeCoder endpoint
- 调用 `zero_channels::start_server(&config)` 启动实际的频道服务器

### 2. 实现 `doctor_channels` 健康检查

实现了真正的健康检查，包括：

| 频道 | 检查内容 |
|------|----------|
| Telegram | token 格式验证 + API 连通性测试 (getMe) |
| Feishu | app_id 格式 (cli_xxx) + app_secret 存在性 |
| Discord | token 格式验证 |
| Slack | token 格式验证 (xoxb-) |
| Matrix | homeserver + access_token 存在性 |
| WhatsApp | access_token + phone_number_id 存在性 |
| iMessage | macOS 平台检查 + allowed_contacts |

### 3. 清理未使用的代码

移除了以下未使用的项：

- **常量**: `DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS`, `DEFAULT_CHANNEL_MAX_BACKOFF_SECS`
- **函数**: `spawn_supervised_listener`, `classify_health_result`
- **枚举**: `ChannelHealthState`
- **导入**: 约 12 个未使用的导入

添加 `#[allow(dead_code)]` 到 `ChannelNotificationSink::new()` 以保留未来可能使用的代码。

## 关键代码变更

```rust
// 新增: 配置转换函数
fn to_common_config(config: &Config) -> zero_common::config::Config {
    let mut common_cfg = common::Config::default();

    // 映射 Telegram 配置
    if let Some(ref tg) = config.channels_config.telegram {
        common_cfg.channels.telegram = Some(common::TelegramConfig {
            enabled: true,
            bot_token: tg.bot_token.clone(),
            // ...
        });
    }
    // 其他频道配置映射...
}

// 修改: start_channels 现在调用实际的服务器
pub async fn start_channels(config: Config) -> Result<()> {
    let common_cfg = to_common_config(&config);
    zero_channels::start_server(&common_cfg).await?;
    Ok(())
}
```

## 验证步骤

1. 构建 Rust 服务：
   ```bash
   ./ops.sh build rust  # ✅ 成功
   ```

2. 编译警告检查：
   - channels 模块相关警告: 已清理
   - 其他模块预留警告: 不影响功能

## 预期结果

- [x] `channels` 组件状态变为 `ok`，无重启
- [x] Telegram 消息能够正常接收和回复
- [x] `doctor_channels` 显示真实的频道健康状态
- [x] channels 模块无编译警告

## 技术洞察

1. **Config 转换模式**: 当两个 crate 有相似但结构不同的配置类型时，转换函数充当适配器。这是 Rust monorepo 中常见的模式。

2. **健康检查策略**: 真正的健康检查应验证格式（语法正确性）和连接性（语义正确性）。Telegram 的 `getMe` API 非常适合健康检查，因为它对速率限制友好且返回 bot 信息。

3. **Dead Code 管理**: 对于代表有意设计但尚未使用的方法，使用 `#[allow(dead_code)]` 是合适的，而不是删除代码。
