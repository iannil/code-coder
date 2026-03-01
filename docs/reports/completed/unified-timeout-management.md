# 统一超时管理方案实施报告

**日期**: 2026-03-01
**状态**: 已完成 (Phase 1-4)

## 问题背景

`/api/v1/macro/report` 端点超时失败，根本原因：
- `AgentBridge` 的 HTTP 客户端超时默认 30 秒
- LLM 调用实际耗时 30-180+ 秒
- **60+ 处** `reqwest::Client::new()` 没有任何超时保护

## 解决方案

### Phase 1: Foundation (zero-common) ✅

**新增文件:**
- `services/zero-common/src/http_client.rs` - HTTP 客户端工厂模块

**修改文件:**
- `services/zero-common/src/config.rs` - 添加 `TimeoutConfig` 结构
- `services/zero-common/src/lib.rs` - 导出新模块
- `services/zero-common/Cargo.toml` - 添加 `http-client` feature

**TimeoutConfig 默认值:**
```rust
TimeoutConfig {
    default_secs: 30,      // 通用默认
    connect_secs: 10,      // TCP 连接
    llm_secs: 300,         // LLM API 调用 (5分钟)
    notification_secs: 15, // 通知/Webhook
    api_secs: 30,          // 外部 API
    shell_secs: 120,       // Shell 命令
}
```

**ClientCategory 枚举:**
- `Llm` → 300s (AI provider calls)
- `Notification` → 15s (IM/webhook)
- `Api` → 30s (external APIs)
- `General` → 30s (fallback)

### Phase 2: Fix zero-trading ✅

**修改文件:**
- `macro_agent/bridge.rs` - 使用 `build_client_with_timeout()`
- `macro_agent/mod.rs` - 使用 `config.timeout.llm_secs` 作为回退
- `notification.rs` - 使用 `ClientCategory::Notification`
- `macro_filter/mod.rs` - 使用 `ClientCategory::Api`
- `data/lixin.rs` - 添加 fallback 超时
- `data/itick.rs` - 添加 fallback 超时

**关键修复:**
```rust
// Before: 30s timeout, fallback with NO timeout
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .unwrap_or_else(|_| reqwest::Client::new()); // ❌ No timeout!

// After: Uses config with proper fallback
let client = build_client(&config.timeout, ClientCategory::Llm);
```

### Phase 3: Fix zero-workflow ✅

**修改文件 (8个):**
- `review_bridge.rs` - `ClientCategory::Llm`
- `ticket_bridge.rs` - `ClientCategory::Llm`
- `monitor_bridge.rs` - `ClientCategory::Api` (保留 user-agent)
- `economic_bridge.rs` - `ClientCategory::Api`
- `risk_monitor.rs` - `ClientCategory::Notification`
- `trading_review.rs` - `ClientCategory::Llm`
- `workflow.rs` - `ClientCategory::Api`/`Llm`
- `hands/notification_bridge.rs` - `ClientCategory::Notification`

### Phase 4: Fix zero-channels (部分) ✅

**修改文件:**
- `bridge.rs` - 使用 `build_client_with_timeout()` (1800s for CodeCoder)

### 配置优先级修复

```rust
// macro_agent/mod.rs - 正确的优先级链
fn create_agent_config(config: &Config) -> AgentBridgeConfig {
    let timeout_secs = trading
        .and_then(|t| t.macro_agent.as_ref())
        .map(|m| m.timeout_secs)
        .filter(|&t| t > 0)
        .unwrap_or(config.timeout.llm_secs); // ← 使用 TimeoutConfig 作为回退
}
```

### 默认值修正

```rust
// config.rs - 修正 macro_agent 默认超时
fn default_macro_agent_timeout() -> u64 {
    300 // 从 30s 改为 300s (匹配 LLM 类别)
}
```

## 验证结果

```bash
# 单元测试
cargo test --lib --features http-client
# 结果: 151 passed

# 编译检查
cargo check --all
# 结果: 所有服务编译通过
```

## 剩余工作 (Phase 5-6)

以下文件仍使用 `Client::new()` 但优先级较低:

**zero-channels (13个文件):**
- telegram/mod.rs, discord/mod.rs, slack/mod.rs
- feishu.rs, wecom.rs, dingtalk.rs, matrix.rs, whatsapp.rs
- capture_bridge.rs
- tts/openai.rs, tts/elevenlabs.rs
- stt/openai.rs, stt/compatible.rs

**zero-gateway (10个文件):**
- provider/*.rs (anthropic, openai, ollama, compatible, openrouter, gemini)
- webhook.rs, proxy.rs
- hitl/cards/*.rs

**zero-cli (6个文件):**
- alerts/mod.rs, mcp/transport.rs
- skills/hub.rs, tunnel/custom.rs
- onboard/wizard.rs (blocking client)

## 向后兼容性

- 所有现有配置字段保持有效
- 新增 `timeout` 配置段是可选的
- 硬编码值转为默认值，不改变当前默认行为

## 配置示例

```json
{
  "timeout": {
    "default_secs": 30,
    "connect_secs": 10,
    "llm_secs": 300,
    "notification_secs": 15,
    "api_secs": 30,
    "shell_secs": 120
  },
  "trading": {
    "macro_agent": {
      "timeout_secs": 180
    }
  }
}
```
