# IM Agent 自动路由实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 AgentRegistry.recommend() 逻辑集成到 bridge.rs，实现 IM 消息自动路由到最佳 Agent

**Architecture:** Rust 端调用 CodeCoder 的 `/api/v1/registry/recommend` API，获取推荐 agent 后传递给 chat API。显式 @agent 命令跳过推荐调用，200ms 超时保护确保稳定性。

**Tech Stack:** Rust, tokio, reqwest, serde

**Design Doc:** `docs/plans/2026-02-27-im-agent-auto-routing-design.md`

---

## Task 1: 添加 Recommend API 类型定义

**Files:**
- Modify: `services/zero-channels/src/bridge.rs:240-260` (在 FeatureRequestInfo 后添加)

**Step 1: 添加类型定义**

在 `services/zero-channels/src/bridge.rs` 的 `FeatureRequestInfo` 结构体后（约第 260 行），添加：

```rust
// ============================================================================
// Agent Recommendation Types
// ============================================================================

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

/// Data payload from recommend API.
#[derive(Debug, Clone, Deserialize)]
pub struct RecommendData {
    /// Recommended agent metadata
    pub recommended: Option<RecommendedAgent>,
    /// Alternative agents
    #[serde(default)]
    pub alternates: Vec<RecommendedAgent>,
}

/// Recommended agent info.
#[derive(Debug, Clone, Deserialize)]
pub struct RecommendedAgent {
    /// Agent name (e.g., "macro", "code-reviewer")
    pub name: String,
    /// Display name for UI
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}
```

**Step 2: 验证编译**

Run: `cd /Users/iannil/Code/zproducts/code-coder/services && cargo check -p zero-channels`
Expected: 编译成功，无错误

**Step 3: Commit**

```bash
cd /Users/iannil/Code/zproducts/code-coder
git add services/zero-channels/src/bridge.rs
git commit -m "feat(zero-channels): add RecommendRequest/Response types for agent routing"
```

---

## Task 2: 实现 call_recommend_agent 方法

**Files:**
- Modify: `services/zero-channels/src/bridge.rs` (在 CodeCoderBridge impl 块中添加方法)

**Step 1: 定位插入点**

在 `impl CodeCoderBridge` 块中，找到 `call_compare` 方法（约第 1787 行），在其前面添加新方法。

**Step 2: 添加 call_recommend_agent 方法**

```rust
    // ========================================================================
    // Agent Recommendation
    // ========================================================================

    /// Call the recommend API with timeout protection.
    ///
    /// Returns recommended agent name, or None if:
    /// - API call failed
    /// - API timed out (>200ms)
    /// - No recommendation returned
    async fn call_recommend_agent(&self, intent: &str) -> Option<String> {
        let url = format!("{}/api/v1/registry/recommend", self.endpoint);

        tracing::debug!(
            endpoint = %url,
            intent_len = intent.len(),
            "Calling agent recommend API"
        );

        // 200ms timeout for recommend API to avoid blocking message processing
        let result = tokio::time::timeout(
            Duration::from_millis(200),
            self.http_client
                .post(&url)
                .json(&RecommendRequest {
                    intent: intent.to_string(),
                })
                .send(),
        )
        .await;

        match result {
            Ok(Ok(resp)) if resp.status().is_success() => {
                match resp.json::<RecommendResponse>().await {
                    Ok(rec_resp) if rec_resp.success => {
                        let agent_name = rec_resp
                            .data
                            .and_then(|d| d.recommended)
                            .map(|a| a.name);

                        if let Some(ref name) = agent_name {
                            tracing::debug!(
                                recommended_agent = %name,
                                "Agent recommendation successful"
                            );
                        }

                        agent_name
                    }
                    Ok(rec_resp) => {
                        tracing::debug!(
                            error = ?rec_resp.error,
                            "Recommend API returned unsuccessful response"
                        );
                        None
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to parse recommend response");
                        None
                    }
                }
            }
            Ok(Ok(resp)) => {
                tracing::warn!(
                    status = %resp.status(),
                    "Recommend API returned non-success status"
                );
                None
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

**Step 3: 验证编译**

Run: `cd /Users/iannil/Code/zproducts/code-coder/services && cargo check -p zero-channels`
Expected: 编译成功，无错误

**Step 4: Commit**

```bash
cd /Users/iannil/Code/zproducts/code-coder
git add services/zero-channels/src/bridge.rs
git commit -m "feat(zero-channels): implement call_recommend_agent with 200ms timeout"
```

---

## Task 3: 修改 process_message 集成自动路由

**Files:**
- Modify: `services/zero-channels/src/bridge.rs:730-737` (Regular chat processing 部分)

**Step 1: 定位需修改的代码**

找到 `process_message` 方法末尾的 "Regular chat processing" 部分（约第 730-737 行）：

```rust
        // Regular chat processing
        // Check if streaming should be used
        let agent_from_meta = message.metadata.get("agent").map(|s| s.as_str());
        if self.should_use_streaming(&message, agent_from_meta) {
            self.process_streaming_chat(&message, &text, None).await
        } else {
            self.process_chat_with_agent(&message, &text, None).await
        }
```

**Step 2: 替换为自动路由逻辑**

```rust
        // Regular chat processing with auto-routing
        // Check if agent is specified in metadata (e.g., from previous context)
        let agent_from_meta = message.metadata.get("agent").map(|s| s.as_str());

        // Auto-route to recommended agent if no explicit agent specified
        let recommended_agent = if agent_from_meta.is_none() {
            // No explicit agent, try to recommend based on message content
            self.call_recommend_agent(&text).await
        } else {
            // Explicit agent specified, skip recommendation
            None
        };

        // Determine final agent: metadata > recommended > None (will use default)
        let final_agent = agent_from_meta
            .map(|s| s.to_string())
            .or(recommended_agent);

        tracing::info!(
            message_id = %message.id,
            agent_from_meta = ?agent_from_meta,
            recommended = ?final_agent,
            "Processing message with agent routing"
        );

        // Check if streaming should be used
        if self.should_use_streaming(&message, final_agent.as_deref()) {
            self.process_streaming_chat(&message, &text, final_agent).await
        } else {
            self.process_chat_with_agent(&message, &text, final_agent).await
        }
```

**Step 3: 验证编译**

Run: `cd /Users/iannil/Code/zproducts/code-coder/services && cargo check -p zero-channels`
Expected: 编译成功，无错误

**Step 4: Commit**

```bash
cd /Users/iannil/Code/zproducts/code-coder
git add services/zero-channels/src/bridge.rs
git commit -m "feat(zero-channels): integrate auto-routing in process_message"
```

---

## Task 4: 添加单元测试

**Files:**
- Modify: `services/zero-channels/src/bridge.rs` (在文件末尾的 tests 模块中添加)

**Step 1: 定位测试模块**

找到文件末尾的 `#[cfg(test)] mod tests` 块。

**Step 2: 添加测试用例**

在 tests 模块中添加：

```rust
    #[test]
    fn test_recommend_request_serialization() {
        let request = RecommendRequest {
            intent: "分析今天的GDP数据".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("intent"));
        assert!(json.contains("分析今天的GDP数据"));
    }

    #[test]
    fn test_recommend_response_deserialization() {
        let json = r#"{
            "success": true,
            "data": {
                "recommended": {
                    "name": "macro",
                    "displayName": "Macro Economist"
                },
                "alternates": []
            }
        }"#;

        let response: RecommendResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert!(response.data.is_some());

        let data = response.data.unwrap();
        assert!(data.recommended.is_some());
        assert_eq!(data.recommended.unwrap().name, "macro");
    }

    #[test]
    fn test_recommend_response_empty_recommendation() {
        let json = r#"{
            "success": true,
            "data": {
                "recommended": null,
                "alternates": []
            }
        }"#;

        let response: RecommendResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.data.unwrap().recommended.is_none());
    }

    #[test]
    fn test_recommend_response_failure() {
        let json = r#"{
            "success": false,
            "error": "Internal server error"
        }"#;

        let response: RecommendResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(response.error, Some("Internal server error".to_string()));
    }
```

**Step 3: 运行测试**

Run: `cd /Users/iannil/Code/zproducts/code-coder/services && cargo test -p zero-channels -- --test-threads=1`
Expected: 所有测试通过

**Step 4: Commit**

```bash
cd /Users/iannil/Code/zproducts/code-coder
git add services/zero-channels/src/bridge.rs
git commit -m "test(zero-channels): add unit tests for recommend types"
```

---

## Task 5: 构建并验证

**Files:**
- Build: `services/zero-channels`

**Step 1: 完整构建**

Run: `cd /Users/iannil/Code/zproducts/code-coder/services && cargo build -p zero-channels --release`
Expected: 构建成功

**Step 2: 运行所有测试**

Run: `cd /Users/iannil/Code/zproducts/code-coder/services && cargo test -p zero-channels`
Expected: 所有测试通过

**Step 3: Clippy 检查**

Run: `cd /Users/iannil/Code/zproducts/code-coder/services && cargo clippy -p zero-channels -- -D warnings`
Expected: 无警告

**Step 4: 最终 Commit**

```bash
cd /Users/iannil/Code/zproducts/code-coder
git add -A
git commit -m "feat(zero-channels): complete agent auto-routing implementation

- Add RecommendRequest/Response types for /api/v1/registry/recommend API
- Implement call_recommend_agent with 200ms timeout protection
- Integrate auto-routing in process_message (skip if explicit @agent)
- Add unit tests for serialization/deserialization

Refs: docs/plans/2026-02-27-im-agent-auto-routing-design.md"
```

---

## Task 6: 更新进度文档

**Files:**
- Create: `docs/progress/2026-02-27-im-agent-auto-routing.md`

**Step 1: 创建进度文档**

```markdown
# IM Agent 自动路由 - 实施进度

**开始时间**: 2026-02-27
**状态**: 已完成

## 变更摘要

在 `services/zero-channels/src/bridge.rs` 中实现了 IM 消息自动路由功能：

1. **新增类型** (4个结构体):
   - `RecommendRequest` - 请求体
   - `RecommendResponse` - 响应体
   - `RecommendData` - 响应数据
   - `RecommendedAgent` - 推荐的 agent 信息

2. **新增方法**:
   - `call_recommend_agent()` - 调用 recommend API，带 200ms 超时保护

3. **修改方法**:
   - `process_message()` - 集成自动路由逻辑

## 测试结果

- [ ] 单元测试通过
- [ ] Clippy 检查通过
- [ ] 手动测试：发送 "分析GDP数据" 自动路由到 macro agent
- [ ] 手动测试：发送 "@code-reviewer 审查代码" 跳过推荐

## 后续工作

无
```

**Step 2: Commit**

```bash
cd /Users/iannil/Code/zproducts/code-coder
git add docs/progress/2026-02-27-im-agent-auto-routing.md
git commit -m "docs: add progress doc for agent auto-routing"
```

---

## Summary

| Task | Description | Estimated |
|------|-------------|-----------|
| 1 | 添加 Recommend API 类型定义 | 3 min |
| 2 | 实现 call_recommend_agent 方法 | 5 min |
| 3 | 修改 process_message 集成自动路由 | 5 min |
| 4 | 添加单元测试 | 5 min |
| 5 | 构建并验证 | 3 min |
| 6 | 更新进度文档 | 2 min |

**Total: ~23 minutes**
