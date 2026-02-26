# 配置文件优化：统一服务 endpoint 管理

**完成时间**: 2026-02-26
**状态**: 已完成

## 目标

统一管理分散在配置文件中的服务 endpoint 定义，消除重复配置，提高可维护性。

## 变更概要

### 1. CodeCoderConfig 结构重构

**文件**: `services/zero-common/src/config.rs`

将 `CodeCoderConfig` 从使用单一 `endpoint` 字符串改为使用 `host` 和 `port` 字段：

```rust
// Before
pub struct CodeCoderConfig {
    pub enabled: bool,
    pub endpoint: String,       // 如 "http://127.0.0.1:4400"
    pub timeout_secs: u64,
}

// After
pub struct CodeCoderConfig {
    pub enabled: bool,
    pub port: u16,              // 如 4400
    pub host: String,           // 如 "127.0.0.1"
    pub timeout_secs: u64,
    endpoint: Option<String>,   // 向后兼容的旧字段（私有，跳过序列化）
}
```

### 2. 添加 Config endpoint 便捷方法

在 `Config` 上添加了统一的 endpoint 获取方法：

```rust
impl Config {
    pub fn codecoder_endpoint(&self) -> String;   // http://127.0.0.1:4400
    pub fn gateway_endpoint(&self) -> String;      // http://127.0.0.1:4430
    pub fn channels_endpoint(&self) -> String;     // http://127.0.0.1:4431
    pub fn workflow_endpoint(&self) -> String;     // http://127.0.0.1:4432
    pub fn trading_endpoint(&self) -> String;      // http://127.0.0.1:4434
}
```

### 3. 移除重复的 endpoint 字段

以下字段已标记为 deprecated（私有，跳过序列化，但仍可反序列化以保持向后兼容）：

| 原字段 | 替代方案 |
|--------|----------|
| `gateway.codecoder_endpoint` | `config.codecoder_endpoint()` |
| `trading.telegram_notification.channels_endpoint` | `config.channels_endpoint()` |
| `trading.macro_agent.codecoder_endpoint` | `config.codecoder_endpoint()` |
| `trading.workflow_endpoint` | `config.workflow_endpoint()` |
| `hitl.channels_endpoint` | `config.channels_endpoint()` |

### 4. 更新使用代码

以下文件已更新为使用新的 Config 便捷方法：

- `services/zero-gateway/src/lib.rs` - 使用 `config.codecoder_endpoint()`
- `services/zero-gateway/src/routes.rs` - 使用 `config.codecoder_endpoint()`
- `services/zero-channels/src/lib.rs` - 使用 `config.codecoder_endpoint()`
- `services/zero-workflow/src/lib.rs` - 使用 `config.codecoder_endpoint()` 和 `config.channels_endpoint()`
- `services/zero-trading/src/notification.rs` - 使用 `config.channels_endpoint()`
- `services/zero-trading/src/macro_filter/mod.rs` - 使用 `config.workflow_endpoint()`
- `services/zero-trading/src/macro_agent/mod.rs` - 使用 `config.codecoder_endpoint()`
- `services/zero-trading/src/routes.rs` - 使用 `config.codecoder_endpoint()`
- `services/zero-cli/src/channels/mod.rs` - 解析 endpoint URL 提取 host/port
- `services/zero-cli/src/client.rs` - 更新默认端口为 4430/4431/4432

### 5. 更新验证逻辑

**文件**: `services/zero-common/src/validation.rs`

- 移除了对已删除 endpoint 字段的验证
- 添加了 `codecoder.port` 的端口冲突检测
- 添加了 `trading.port` 的端口冲突检测

### 6. 添加依赖

- `zero-common/Cargo.toml` - 添加 `url` crate 用于 endpoint URL 解析
- `zero-cli/Cargo.toml` - 添加 `url` crate 用于 endpoint URL 解析

## 向后兼容性

### 旧配置文件支持

旧配置文件格式仍然可以加载：

```json
{
  "codecoder": { "endpoint": "http://127.0.0.1:4400" },
  "gateway": { "codecoder_endpoint": "http://127.0.0.1:4400" }
}
```

这些旧字段会被解析但不会被序列化回去。下次保存配置时会使用新格式。

### 环境变量支持

新增环境变量覆盖：
- `CODECODER_PORT` - 覆盖 codecoder 端口
- `CODECODER_HOST` - 覆盖 codecoder 主机
- `CODECODER_ENDPOINT` - 旧格式，解析 URL 提取 host/port

## 新配置文件格式示例

```json
{
  "gateway": { "port": 4430, "host": "127.0.0.1" },
  "channels": { "port": 4431, "host": "127.0.0.1" },
  "workflow": { "port": 4432, "host": "127.0.0.1" },
  "codecoder": { "port": 4400, "host": "127.0.0.1", "enabled": true },
  "trading": {
    "port": 4434,
    "host": "127.0.0.1",
    "telegram_notification": {
      "enabled": true,
      "channel_type": "telegram"
    },
    "macro_agent": {
      "enabled": true,
      "timeout_secs": 30
    }
  }
}
```

## 验证

- ✅ `cargo build --workspace` - 编译通过
- ✅ `cargo test -p zero-common` - 119 个测试通过
- ✅ 向后兼容性测试通过

## 技术亮点

1. **DRY 原则**: 消除了 4+ 处重复的 endpoint 定义
2. **类型安全**: 使用 `u16` 端口类型而非字符串
3. **向后兼容**: 旧配置文件无需修改即可加载
4. **可测试性**: 所有更改都有对应的单元测试覆盖
