# IM 工具结果分类策略（v2）

**日期**: 2026-02-28
**状态**: 已完成

## 问题演进

### 第一阶段：无结果返回

用户报告通过 IM (Telegram/Discord等) 调用工具时，工具结果没有返回：
- 只看到 "🚀 开始处理..."
- 然后就没有后续了

### 第二阶段：累积所有结果（v1）

修复第一阶段后，所有工具结果都被累积，导致新问题：
- **信息过载**：用户看到大量中间步骤结果（Read 文件内容、Bash 命令输出等）
- **消息过长**：超出 Telegram 4096 字符限制
- **安全风险**：敏感信息可能泄露（如 credential 工具返回密码）

### 第三阶段：精细分类策略（v2，当前）

根据工具的**实际用途**分类处理：
- **结果类工具**：累积结果（用户要的就是这个）
- **中间类工具**：不累积（AI 用于分析，不是用户要的）
- **敏感类工具**：绝不累积（安全风险）

## 工具分类

### ✅ 结果类工具（应累积）

这些工具的输出是用户**最终想要的内容**，AI 不需要额外处理：

| 工具 | 返回内容 | 原因 |
|-----|---------|------|
| `WebSearch` | 搜索结果列表 | 用户要的就是搜索结果 |
| `WebFetch` | 网页内容 | 用户要的就是网页内容 |
| `reach_youtube` | 视频信息/转录 | 用户要的就是视频内容 |
| `reach_bilibili` | 视频信息/转录 | 用户要的就是视频内容 |
| `reach_rss` | RSS 订阅内容 | 用户要的就是订阅内容 |
| `network-analyzer` | 网络诊断信息 | 用户要的就是诊断结果 |

**处理方式**: 完整显示结果（搜索类需要格式化）

---

### ❌ 中间类工具（不应累积）

这些工具的输出是**中间步骤**，AI 需要分析后给出结论：

| 工具 | 返回内容 | 原因 |
|-----|---------|------|
| `Read` | 文件内容 | AI 读取后分析，用户要的是分析结论 |
| `Write` | 写入确认 | 只是确认操作，不是用户要的结果 |
| `Edit` / `Multiedit` | 编辑确认 | 只是确认操作 |
| `Grep` / `Glob` / `CodeSearch` | 搜索结果 | AI 用这些找到代码，然后分析 |
| `Bash` | 命令输出 | AI 处理命令结果后给出结论 |
| `Lsp` | 语言服务器响应 | 是给 AI 用的，不是给用户看的 |
| `Task` | 子任务结果 | 子任务有自己的输出流程 |

**处理方式**: 不累积结果，在通知中仅显示"执行成功"

---

### 🚫 敏感类工具（绝不累积）

这些工具的结果包含敏感信息：

| 工具 | 风险 | 处理方式 |
|-----|------|---------|
| `credential` | 密码、Token 等敏感信息 | 绝不累积，仅显示"已获取凭证" |

## 实现方案

### Rust 端 (`progress.rs`)

```rust
/// 判断是否为结果类工具
fn is_result_tool(tool: &str) -> bool {
    let lower = tool.to_lowercase();
    lower.contains("websearch") || lower.contains("webfetch")
        || lower.contains("reach_youtube")
        || lower.contains("reach_bilibili")
        || lower.contains("reach_rss")
        || lower.contains("network_analyzer")
}

/// 判断是否为中间类工具
fn is_intermediate_tool(tool: &str) -> bool {
    let lower = tool.to_lowercase();
    lower.contains("read") || lower.contains("write")
        || lower.contains("edit") || lower.contains("grep")
        || lower.contains("glob") || lower.contains("bash")
        || lower.contains("lsp") || lower.contains("task")
}

/// 判断是否为敏感类工具
fn is_sensitive_tool(tool: &str) -> bool {
    let lower = tool.to_lowercase();
    lower.contains("credential") || lower.contains("secret")
}
```

**累积逻辑**:
```rust
// 只累积结果类工具
if Self::is_result_tool(tool) {
    // 累积到最终输出
} else if Self::is_intermediate_tool(tool) {
    // 只显示通知，不累积
} else if Self::is_sensitive_tool(tool) {
    // 只显示"已执行 (结果已隐藏)"
}
```

### TypeScript 端 (`task.ts`)

```typescript
function isResultTool(toolName: string): boolean {
  const lower = toolName.toLowerCase()
  return lower.includes("websearch") || lower.includes("webfetch")
      || lower.includes("reach_youtube")
      || lower.includes("reach_bilibili")
      || lower.includes("reach_rss")
}

function isIntermediateTool(toolName: string): boolean {
  const lower = toolName.toLowerCase()
  return lower.includes("read") || lower.includes("write")
      || lower.includes("edit") || lower.includes("bash")
      || lower.includes("grep") || lower.includes("glob")
}

function isSensitiveTool(toolName: string): boolean {
  const lower = toolName.toLowerCase()
  return lower.includes("credential") || lower.includes("secret")
}

function formatToolResult(toolName: string, result: string): string | null {
  // 跳过敏感和中间工具
  if (isSensitiveTool(toolName) || isIntermediateTool(toolName)) {
    return null
  }
  // ... 格式化结果类工具的输出
}
```

## 修改的文件

| 文件 | 修改内容 |
|------|----------|
| `services/zero-channels/src/progress.rs` | 将 `is_search_tool` 改为 `is_result_tool`，添加 `is_intermediate_tool` 和 `is_sensitive_tool` |
| `packages/ccode/src/api/server/handlers/task.ts` | 添加 `isResultTool`、`isIntermediateTool`、`isSensitiveTool` 函数，修改 `formatToolResult` 逻辑 |

## 测试验证

所有测试通过：
```bash
$ cargo test progress
running 6 tests
test progress::tests::test_format_tool_name ... ok
test progress::tests::test_is_sensitive_tool ... ok
test progress::tests::test_is_result_tool ... ok
test progress::tests::test_is_intermediate_tool ... ok
test progress::tests::test_message_tracker_new ... ok
test progress::tests::test_sse_parse_progress_event ... ok
```

## 实施时间线

- 2026-02-28 v1: 初始实施（累积所有工具结果）
- 2026-02-28 v2: 精细分类策略（仅累积结果类工具）

## 注意事项

- 大型输出会被截断到 2000 字符以避免超过 Telegram 消息大小限制（4096 字符）
- 敏感工具的结果**绝不会**被累积或显示
- 中间工具的结果不会被累积，但会显示"执行中..."通知
- 结果类工具的完整结果会累积到最终消息中
