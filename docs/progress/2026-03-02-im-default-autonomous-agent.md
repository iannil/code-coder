# IM 消息默认使用 Autonomous Agent

**日期**: 2026-03-02
**状态**: 已完成
**最后更新**: 2026-03-02 (移除领域关键词匹配)

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

### 2. 简化 `detect_agent` 逻辑 (2026-03-02 更新)

**文件**: `services/zero-channels/src/task_dispatcher.rs`

**移除领域关键词匹配**，只保留显式 @agent 提及检测：

```rust
// 修改前：基于关键词自动路由
if text.contains("宏观") || text.contains("经济") { return "macro"; }
if text.contains("市场") { return "picker"; }  // 问题：误匹配金融问题
// ... 更多关键词匹配

// 修改后：交给 autonomous 自己决定
// 只处理显式 @agent 提及，其他全部走 default_agent
```

**原因**：
- 关键词匹配不够精确（如"黄金市场"误匹配到 picker 而非 macro）
- Autonomous agent 有能力理解用户意图并调用合适的子 agent
- 简化路由层逻辑，让 LLM 做领域判断

### 3. 增强 Autonomous Agent 的数据获取约束 (2026-03-02 更新)

**文件**: `packages/ccode/src/agent/prompt/autonomous.txt`

新增强制使用工具获取实时数据的规则：

```
### Data Fetching (数据获取) - CRITICAL

**MANDATORY RULE**: For ANY question about current/real-time information, you MUST use tools first:

1. Questions requiring real-time data (MUST use websearch):
   - Market prices (gold, stocks, crypto, commodities)
   - Current news and events
   - Economic indicators (PMI, CPI, GDP)
   ...

2. Detection pattern:
   - "表现如何" → websearch
   - "最新" → websearch
   - "现在" → websearch
   ...

3. NEVER:
   - Fabricate prices, rates, or statistics
   - Use training data as "current" information
   - Answer market questions without tool calls
```

### 4. 添加 `/agents` 和 `/help` 命令支持

**文件**: `services/zero-channels/src/bridge.rs`

修改 `is_agent_help_request` 函数，新增支持：
- `/agents` - 显示所有可用 agent
- `/help` - 显示帮助信息
- `/?` - 快捷帮助

更新帮助信息，在会话控制部分添加：
```
• `/agents` 或 `/help` - 显示此帮助信息
```

### 5. 导出新函数

**文件**: `services/zero-channels/src/lib.rs`

### 6. 更新测试

**文件**: `services/zero-channels/src/task_dispatcher.rs`

- `test_detect_agent_explicit_mention`: 验证显式 @agent 提及
- `test_detect_agent_default`: 验证所有非显式提及的消息都走 default_agent
- `test_default_agent_for_channel`: 验证所有渠道类型的默认 agent

## Agent 选择优先级

1. **显式传入的 agent 参数** (最高优先级)
2. **消息 metadata 中的 agent 字段**
3. **消息内容中的 @agent 提及** (如 `@macro`, `@trader`)
4. **渠道默认 agent** (CLI=build, IM=autonomous)

**注意**：领域关键词匹配和 recommend API 已移除，agent 路由完全交给 autonomous agent 自行决定。

## 修复 HTTP/SSE 模式的 Agent 路由 (2026-03-02 更新)

**问题**：HTTP/SSE 模式调用了 TypeScript 的 `/api/v1/registry/recommend` API，该 API 返回 `general` agent，导致 IM 消息没有走 `autonomous`。

**修复文件**：`services/zero-channels/src/bridge.rs`

1. **修改 `process` 方法**（第 869-888 行）：
   ```rust
   // 修改前：所有渠道都调用 recommend API
   let recommended_agent = self.call_recommend_agent(text).await;

   // 修改后：只有 CLI 调用 recommend API，IM 渠道直接使用 default_agent_for_channel
   let final_agent = if agent_from_meta.is_some() {
       agent_from_meta.map(|s| s.to_string())
   } else if message.channel_type != ChannelType::Cli {
       Some(default_agent_for_channel(message.channel_type).to_string())
   } else {
       self.call_recommend_agent(text).await
   };
   ```

2. **修改 `process_streaming_chat` 方法**（第 1269-1271 行）：
   ```rust
   // 修改前：硬编码 "general" 作为 fallback
   .unwrap_or_else(|| "general".to_string());

   // 修改后：使用渠道默认 agent
   .unwrap_or_else(|| default_agent_for_channel(message.channel_type).to_string());
   ```

3. **添加 ChannelType import**：
   ```rust
   use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent};
   ```

## IM 命令一览

| 命令 | 功能 |
|------|------|
| `/new` 或 `/clear` | 清空上下文，开始新对话 |
| `/compact` 或 `/summary` | 压缩上下文，保留摘要继续对话 |
| `/agents` 或 `/help` | 显示所有可用 agent |
| `@agent名称 问题` | 使用指定 agent 处理问题 |

## 测试验证

```bash
cargo test task_dispatcher -- --nocapture
# 5 passed; 0 failed

cargo test agent_help -- --nocapture
# 2 passed; 0 failed
```

## 影响范围

- 所有 IM 渠道的未指定 agent 的消息将默认使用 `autonomous` agent
- CLI 行为不变，仍默认使用 `build` agent
- 显式指定 agent 或通过关键词匹配的消息不受影响
- IM 用户可通过 `/agents` 或 `/help` 查看所有可用 agent
