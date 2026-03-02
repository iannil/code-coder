# IM 消息默认使用 Autonomous Agent

**日期**: 2026-03-02
**状态**: 已完成

## 背景

来自 IM 渠道（Telegram、Discord、Slack 等）的消息通常是自主任务，需要独立决策和多步骤执行能力。之前所有渠道都默认使用 `build` agent，这不适合 IM 场景。

## 修改内容

### 1. 新增 `default_agent_for_channel` 函数

**文件**: `services/zero-channels/src/task_dispatcher.rs`

根据渠道类型返回适当的默认 agent：
- **CLI** → `"build"` (交互式开发工作流)
- **所有 IM 渠道** → `"autonomous"` (自主任务处理)

支持的 IM 渠道：
- Telegram
- Discord
- Slack
- Feishu (飞书)
- WeCom (企业微信)
- DingTalk (钉钉)
- WhatsApp
- Matrix
- iMessage
- Email

### 2. 修改 `bridge.rs` 调用逻辑

**文件**: `services/zero-channels/src/bridge.rs`

```rust
// 修改前
let agent_name = agent
    .or_else(|| message.metadata.get("agent").cloned())
    .unwrap_or_else(|| detect_agent(text, "build").to_string());

// 修改后
let default_agent = default_agent_for_channel(message.channel_type);
let agent_name = agent
    .or_else(|| message.metadata.get("agent").cloned())
    .unwrap_or_else(|| detect_agent(text, default_agent).to_string());
```

### 3. 导出新函数

**文件**: `services/zero-channels/src/lib.rs`

### 4. 添加测试

**文件**: `services/zero-channels/src/task_dispatcher.rs`

- `test_default_agent_for_channel`: 验证所有渠道类型的默认 agent
- 扩展 `test_detect_agent_default`: 测试 autonomous 作为默认值

## Agent 选择优先级

1. **显式传入的 agent 参数** (最高优先级)
2. **消息 metadata 中的 agent 字段**
3. **消息内容中的 @agent 提及** (如 `@macro`, `@trader`)
4. **领域关键词检测** (如 "宏观", "交易", "选品")
5. **任务类型关键词** (如 "代码审查", "安全", "测试")
6. **渠道默认 agent** (CLI=build, IM=autonomous)

## 测试验证

```bash
cargo test task_dispatcher -- --nocapture
# 5 passed; 0 failed
```

## 影响范围

- 所有 IM 渠道的未指定 agent 的消息将默认使用 `autonomous` agent
- CLI 行为不变，仍默认使用 `build` agent
- 显式指定 agent 或通过关键词匹配的消息不受影响
