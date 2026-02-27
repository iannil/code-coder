# IM Agent 自动路由设计

**日期**: 2026-02-27
**状态**: 已批准

## 概述

将 TypeScript 的 `AgentRegistry.recommend()` 逻辑集成到 Rust 的 `bridge.rs`，实现 IM 消息自动路由到最佳 Agent。

## 设计决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 架构层 | Rust 端调用 recommend API | 复用现有 AgentRegistry 逻辑，易维护 |
| 调用时机 | 始终调用 (无显式 @agent 时) | 简单统一的逻辑 |
| 用户反馈 | 静默自动路由 | 无缝体验 |
| 实现方式 | 条件调用 + 超时保护 | 显式命令零延迟，超时保护稳定性 |

## 架构

```
                           IM 消息到达
                               │
                               ▼
                    ┌──────────────────────┐
                    │   bridge.rs          │
                    │   process_message()  │
                    └──────────┬───────────┘
                               │
              ┌────────────────┴────────────────┐
              │ 已有显式 @agent 命令?            │
              └────────────────┬────────────────┘
                      ├── 是 ──┘
                      │         │
                      │         ▼
                      │   直接使用指定 agent
                      │
                      └── 否 ──▶ call_recommend_agent()
                                       │
                                       ▼
                            ┌────────────────────┐
                            │ POST /api/v1/      │
                            │ registry/recommend │
                            │ (200ms 超时)       │
                            └────────┬───────────┘
                                     │
                       ┌─────────────┴─────────────┐
                       │ 成功?                     │
                       └─────────────┬─────────────┘
                              ├── 是 ──┘
                              │         │
                              │         ▼
                              │   使用推荐的 agent
                              │
                              └── 否 ──▶ fallback 到 "general"
                                               │
                                               ▼
                                      process_chat_with_agent()
```

## 代码变更

### 1. 新增结构体（bridge.rs）

```rust
/// Request to CodeCoder agent recommend API.
#[derive(Debug, Clone, Serialize)]
pub struct RecommendRequest {
    /// User intent/message content
    pub intent: String,
}

/// Response from CodeCoder agent recommend API.
#[derive(Debug, Clone, Deserialize)]
pub struct RecommendResponse {
    pub success: bool,
    pub data: Option<RecommendData>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecommendData {
    /// Recommended agent metadata
    pub recommended: Option<RecommendedAgent>,
    /// Alternative agents
    pub alternates: Vec<RecommendedAgent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecommendedAgent {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}
```

### 2. 新增方法（CodeCoderBridge impl）

```rust
/// Call the recommend API with timeout protection.
/// Returns recommended agent name, or None if failed/timeout.
async fn call_recommend_agent(&self, intent: &str) -> Option<String> {
    let url = format!("{}/api/v1/registry/recommend", self.endpoint);

    // 200ms timeout for recommend API
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        self.http_client.post(&url)
            .json(&RecommendRequest { intent: intent.to_string() })
            .send()
    ).await;

    match result {
        Ok(Ok(resp)) if resp.status().is_success() => {
            resp.json::<RecommendResponse>().await
                .ok()
                .and_then(|r| r.data)
                .and_then(|d| d.recommended)
                .map(|a| a.name)
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "Recommend API call failed");
            None
        }
        Err(_) => {
            tracing::warn!("Recommend API timed out (200ms)");
            None
        }
    }
}
```

### 3. 修改 process_message

在 `process_message()` 的末尾替换默认逻辑：

```rust
// Auto-route to recommended agent (skip if explicit @agent command exists)
let agent_from_meta = message.metadata.get("agent").map(|s| s.as_str());

let recommended_agent = if agent_from_meta.is_none() {
    // No explicit agent, try to recommend
    self.call_recommend_agent(&text).await
} else {
    None
};

let final_agent = agent_from_meta
    .map(|s| s.to_string())
    .or(recommended_agent);

if self.should_use_streaming(&message, final_agent.as_deref()) {
    self.process_streaming_chat(&message, &text, final_agent).await
} else {
    self.process_chat_with_agent(&message, &text, final_agent).await
}
```

## 错误处理

| 场景 | 行为 | 日志级别 |
|------|------|----------|
| recommend API 成功 | 使用推荐 agent | DEBUG |
| recommend API 超时 (>200ms) | fallback 到 `general` | WARN |
| recommend API 返回错误 | fallback 到 `general` | WARN |
| recommend API 返回空推荐 | fallback 到 `general` | DEBUG |
| 网络连接失败 | fallback 到 `general` | WARN |

## 测试策略

### 单元测试

- `test_recommend_agent_success`: 验证成功推荐
- `test_recommend_agent_timeout`: 验证 200ms 超时
- `test_recommend_agent_error`: 验证错误处理
- `test_explicit_agent_skips_recommend`: 验证显式命令跳过推荐

### 集成测试场景

| 测试用例 | 输入 | 预期 agent |
|----------|------|------------|
| 显式命令 | `@macro 解读PMI` | `macro` |
| 宏观经济 | `分析今天的GDP数据` | `macro` (推荐) |
| 代码审查 | `review 这个PR` | `code-reviewer` (推荐) |
| 无明显意图 | `你好` | `general` (fallback) |
| API 超时 | 任意（mock 超时） | `general` (fallback) |

## 影响范围

- **文件变更**: `services/zero-channels/src/bridge.rs`
- **新增依赖**: 无
- **向后兼容**: 完全兼容，现有 @agent 命令行为不变
