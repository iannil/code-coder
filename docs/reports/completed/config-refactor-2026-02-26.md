# 配置文件重构进展报告

**日期**: 2026-02-26
**状态**: 已完成

## 完成的工作

### Phase 1: Rust 配置定义更新 ✅

**文件**: `services/zero-common/src/config.rs`

添加了以下新配置结构:
- `NetworkConfig` - 全局网络配置 (bind 地址)
- `ServicesConfig` / `ServicePortConfig` - 简化的服务端口配置
- `SecretsConfig` - 分组的密钥配置 (LLM、渠道、外部服务)
- `LlmConfig` / `LlmProviderConfig` - 简化的 LLM 配置
- `AuthConfig` - 简化的认证配置
- `VoiceConfig` - 语音配置 (TTS/STT)
- `ChannelEnableConfig` - 简化的渠道启用配置

更新了 `Config` 结构体:
- 添加新配置块到 Config 结构体
- 更新 endpoint 方法使用 `network.bind` 和 `services.*.port`
- 添加 `get_api_key()` 方法支持新旧两种格式
- 添加 `migrate_to_new_format()` 迁移方法
- 添加 `is_new_format()` 检测方法

### Phase 2: TypeScript 配置定义更新 ✅

**文件**: `packages/util/src/config.ts`

- 添加新配置类型定义 (NetworkConfig, ServicesConfig, SecretsConfig, etc.)
- 更新 ConfigSchema (Zod 验证)
- 更新 DEFAULT_CONFIG
- 添加 ConfigManager 辅助方法:
  - `getBindAddress()` - 获取有效的绑定地址
  - `getCodeCoderPort()`, `getGatewayPort()`, etc. - 获取服务端口
  - `getCodeCoderEndpoint()`, `getGatewayEndpoint()`, etc. - 获取完整端点 URL

### Phase 3: 清理硬编码 localhost 引用 ✅

更新的文件:
- `packages/ccode/src/api/server/handlers/scheduler.ts` - 使用 ConfigManager
- `packages/ccode/src/api/server/handlers/executive.ts` - 使用 ConfigManager
- `packages/ccode/src/api/server/handlers/executive-ws.ts` - 使用 ConfigManager

保留的硬编码 (有正当理由):
- OAuth callback 文件 - 必须使用 127.0.0.1 (安全原因)
- CORS 通配符模式 - 用于本地开发
- 测试文件中的 mock 数据

### Phase 4 & 5: 配置迁移逻辑 ✅

**创建文件**: `packages/ccode/src/config/migrate.ts`

迁移脚本功能:
- 读取现有配置文件
- 从旧字段提取值到新字段
- 保留所有旧字段 (向后兼容)
- 自动备份旧配置
- 输出迁移摘要

### Phase 6: 验证 ✅

- ✅ `cargo build` - Rust 编译成功 (只有警告)
- ✅ `tsc --noEmit` - TypeScript 类型检查通过
- ✅ 配置迁移脚本运行成功

## 迁移后的配置结构

```json
{
  "$schema": "https://code-coder.com/config.json",

  // ===== 新简化配置 =====
  "network": {
    "bind": "127.0.0.1"
  },
  "services": {
    "codecoder": { "port": 4400 },
    "gateway": { "port": 4430 },
    "channels": { "port": 4431 },
    "workflow": { "port": 4432 },
    "trading": { "port": 4434 }
  },
  "auth": {
    "mode": "pairing"
  },
  "secrets": {
    "llm": { "deepseek": "sk-xxx" },
    "channels": { "telegram_bot_token": "xxx" },
    "external": { "cloudflare_tunnel": "xxx" }
  },
  "llm": {
    "default": "deepseek/deepseek-chat"
  },
  "voice": {
    "tts": { "provider": "compatible", "voice": "nova" },
    "stt": { "provider": "local", "model": "base" }
  },
  "tunnel": {
    "provider": "cloudflare",
    "cloudflare_token": "xxx"
  },

  // ===== 保留的旧配置 (向后兼容) =====
  "gateway": { ... },
  "channels": { ... },
  "api_keys": { ... },
  ...
}
```

## 关键设计决策

1. **非破坏性迁移**: 保留所有旧字段，只添加新字段。这确保了向后兼容性。

2. **优先级链**: 新配置 → 旧配置 → 默认值。例如 `network.bind` 优先于 `gateway.host`。

3. **安全考虑**: OAuth callback 保持使用 127.0.0.1，这是 OAuth 安全要求。

4. **密钥分组**: 按类型组织密钥 (LLM、渠道、外部服务)，便于安全管理。

## 后续工作建议

1. ~~在未来版本中，可以考虑移除冗余的旧字段 (需要主版本升级)~~ **已完成**
2. 添加配置验证 CLI 命令
3. 为 Web UI 添加配置编辑界面

---

## Phase 7: 移除旧版兼容性代码 (2026-02-26) ✅

确认配置文件已完整迁移后，移除冗余的兼容性代码。

### 7.1 删除迁移脚本 ✅

删除文件: `packages/ccode/src/config/migrate.ts`

### 7.2 重构 TypeScript 配置 ✅

**文件**: `packages/util/src/config.ts`

**删除的旧版接口:**
- `GatewayConfig`, `ChannelsConfig`, `TelegramConfig`, `DiscordConfig`, `SlackConfig`, `FeishuConfig`
- `TtsConfig`, `SttConfig`
- `WorkflowConfig`, `CronConfig`, `CronTask`, `WebhookConfig`, `GitIntegrationConfig`
- `CodeCoderConfig`, `ObservabilityConfig`, `MemoryConfig`

**保留的简化接口:**
- `ServicesConfig` - 服务端口配置
- `ChannelEnableConfig` - 渠道启用配置
- `SecretsConfig` - 密钥配置

**简化端点获取方法:**
```typescript
// 移除 fallback 逻辑
getGatewayPort(): number {
  return this.config.services?.gateway?.port ?? 4430
}
```

### 7.3 重构 Rust 配置 ✅

**文件**: `services/zero-common/src/config.rs`

**删除的迁移方法:**
- `migrate_to_new_format()`
- `migrate_api_keys_to_secrets()`
- `migrate_channel_credentials_to_secrets()`
- `migrate_voice_config()`
- `migrate_llm_config()`
- `migrate_and_save()`

**简化端口访问方法:**
```rust
// Before:
pub fn gateway_port(&self) -> u16 {
    self.services.gateway.port.unwrap_or(self.gateway.port)
}

// After:
pub fn gateway_port(&self) -> u16 {
    self.services.gateway.port.unwrap_or(4430)
}
```

**简化 API key 方法:**
- 移除 `api_keys` fallback，直接从 `secrets.llm` 读取

**简化渠道凭证方法:**
- 移除 `channels.<channel>.bot_token` fallback
- 直接从 `secrets.channels` 读取

### 7.4 更新服务代码 ✅

| 文件 | 修改内容 |
|------|----------|
| `services/zero-gateway/src/lib.rs` | 使用 `config.bind_address()` 和 `config.gateway_port()` |
| `services/zero-channels/src/lib.rs` | 使用 `config.bind_address()` 和 `config.channels_port()` |
| `services/zero-cli/src/channels/mod.rs` | 修复 `TelegramConfig` 构造 (`Option<String>` bot_token) |
| `services/zero-common/src/validation.rs` | 修复 telegram bot_token 验证逻辑 |

### 7.5 验证结果 ✅

| 检查项 | 结果 |
|--------|------|
| TypeScript typecheck | ✅ 通过 |
| Rust build (workspace) | ✅ 通过 |
| Rust tests (zero-common) | ✅ 119/119 通过 |

### 注意事项

1. **保留结构体定义**: Rust 中的旧版结构体定义保留，以支持配置文件读取的向后兼容
2. **移除运行时 fallback**: 关键改变是移除运行时的 fallback 逻辑，不是结构体本身
3. **服务使用访问方法**: 服务现在使用提供合理默认值的访问方法

---

## Phase 8: 配置字段名一致性修复 (2026-02-26) ✅

分析并修复了配置文件与代码之间的字段名不匹配问题。

### 8.1 发现的不匹配问题

#### `observability` 字段名

| 配置文件 | Rust 代码 | 状态 |
|----------|-----------|------|
| `level` | `log_level` | 不匹配 |
| `format` | `log_format` | 不匹配 |

#### `trading` 字段名

| 配置文件 | Rust 代码 | 状态 |
|----------|-----------|------|
| `mode: "paper"` | `paper_trading: bool` | 不匹配 |
| `enabled: true` | (无对应字段) | 冗余 |
| `notification` | `telegram_notification` | 不匹配 |
| `loop` | `loop_config` | 不匹配 |

### 8.2 解决方案: Serde 别名

使用 `#[serde(alias = "...")]` 添加向后兼容别名，无需修改现有配置文件。

**ObservabilityConfig:**
```rust
#[serde(default = "default_log_level", alias = "level")]
pub log_level: String,

#[serde(default = "default_log_format", alias = "format")]
pub log_format: String,
```

**TradingConfig:**
```rust
#[serde(alias = "notification")]
pub telegram_notification: Option<TradingNotificationConfig>,

#[serde(alias = "loop")]
pub loop_config: Option<TradingLoopConfig>,
```

### 8.3 修改的文件

| 文件 | 修改内容 |
|------|----------|
| `services/zero-common/src/config.rs` | ObservabilityConfig 添加 level/format 别名 |
| `services/zero-common/src/config.rs` | TradingConfig 添加 notification/loop 别名 |
| `services/zero-common/src/validation.rs` | 更新验证错误消息使用规范字段名 |

### 8.4 仅 TypeScript 使用的字段

以下字段只在 `packages/ccode/src/config/config.ts` 中定义，Rust 服务不会读取：

- `provider` - 自定义 LLM 提供商配置
- `mcp` - Model Context Protocol 服务器配置
- `keybinds` - 快捷键配置
- `formatter` - 代码格式化配置
- `lsp` - LSP 服务器配置
- `zerobot` - ZeroBot 守护进程配置
- `compaction` - 会话压缩配置
- `autonomousMode` - 自主模式配置

### 8.5 文档输出

创建了两个配置示例文件：

1. `docs/config-example-full.jsonc` - 包含所有可配置选项的完整示例
2. `docs/config-example-minimal.json` - 最简配置示例

### 8.6 建议

新配置文件建议使用规范字段名：
- 使用 `log_level` 而非 `level`
- 使用 `log_format` 而非 `format`
- 使用 `telegram_notification` 而非 `notification`
- 使用 `loop_config` 而非 `loop`
- 使用 `paper_trading: true` 而非 `mode: "paper"`
