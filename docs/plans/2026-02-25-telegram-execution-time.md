# Telegram 执行时间显示实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在 Telegram 渠道的 AI 响应消息末尾显示执行时间。

**Architecture:** 在 `CodeCoderBridge::process_chat_with_agent` 方法中计算请求耗时，格式化为人类可读字符串，追加到响应消息末尾后发送。

**Tech Stack:** Rust, zero-channels 服务

---

### Task 1: 添加时间格式化函数

**Files:**
- Modify: `services/zero-channels/src/bridge.rs`

**Step 1: 添加时间格式化辅助函数**

在 `impl CodeCoderBridge` 块内添加：

```rust
/// Format duration in human-readable form.
/// - < 1s: "850ms"
/// - 1-60s: "3.2s"
/// - > 60s: "1m25s"
fn format_duration(duration_ms: u64) -> String {
    if duration_ms < 1000 {
        format!("{}ms", duration_ms)
    } else if duration_ms < 60_000 {
        format!("{:.1}s", duration_ms as f64 / 1000.0)
    } else {
        let minutes = duration_ms / 60_000;
        let seconds = (duration_ms % 60_000) / 1000;
        format!("{}m{}s", minutes, seconds)
    }
}
```

**Step 2: 编译验证**

Run: `cd services/zero-channels && cargo check`
Expected: 编译成功，无错误

**Step 3: Commit**

```bash
git add services/zero-channels/src/bridge.rs
git commit -m "feat(zero-channels): add duration formatting helper"
```

---

### Task 2: 修改 process_chat_with_agent 添加时间记录

**Files:**
- Modify: `services/zero-channels/src/bridge.rs:797-859`

**Step 1: 在方法开始处添加计时**

在 `process_chat_with_agent` 方法开始处（约第 798 行，`let ctx = RequestContext` 之前）添加：

```rust
let start = Instant::now();
```

**Step 2: 在发送响应时添加时间后缀**

找到发送响应的代码（约第 834-837 行）：

```rust
Ok(resp) => {
    let content = OutgoingContent::Markdown { text: resp.message };
    let result = self.router.respond(&message.id, content).await;
```

修改为：

```rust
Ok(resp) => {
    let duration_ms = start.elapsed().as_millis() as u64;
    let duration_text = Self::format_duration(duration_ms);
    let text_with_time = format!("{}\n\n_⏱ {}_", resp.message, duration_text);
    let content = OutgoingContent::Markdown { text: text_with_time };
    let result = self.router.respond(&message.id, content).await;
```

**Step 3: 编译验证**

Run: `cd services/zero-channels && cargo check`
Expected: 编译成功，无错误

**Step 4: Commit**

```bash
git add services/zero-channels/src/bridge.rs
git commit -m "feat(zero-channels): add execution time to Telegram responses"
```

---

### Task 3: 添加单元测试

**Files:**
- Modify: `services/zero-channels/src/bridge.rs`

**Step 1: 在文件末尾的 tests 模块中添加测试**

找到 `#[cfg(test)] mod tests` 块，添加测试：

```rust
#[test]
fn test_format_duration_milliseconds() {
    assert_eq!(CodeCoderBridge::format_duration(500), "500ms");
    assert_eq!(CodeCoderBridge::format_duration(0), "0ms");
    assert_eq!(CodeCoderBridge::format_duration(999), "999ms");
}

#[test]
fn test_format_duration_seconds() {
    assert_eq!(CodeCoderBridge::format_duration(1000), "1.0s");
    assert_eq!(CodeCoderBridge::format_duration(3200), "3.2s");
    assert_eq!(CodeCoderBridge::format_duration(59999), "60.0s");
}

#[test]
fn test_format_duration_minutes() {
    assert_eq!(CodeCoderBridge::format_duration(60000), "1m0s");
    assert_eq!(CodeCoderBridge::format_duration(85000), "1m25s");
    assert_eq!(CodeCoderBridge::format_duration(125000), "2m5s");
}
```

**Step 2: 运行测试**

Run: `cd services/zero-channels && cargo test format_duration`
Expected: 3 tests passed

**Step 3: Commit**

```bash
git add services/zero-channels/src/bridge.rs
git commit -m "test(zero-channels): add unit tests for duration formatting"
```

---

### Task 4: 构建并手动验证

**Step 1: 构建 release 版本**

Run: `cd services/zero-channels && cargo build --release`
Expected: 编译成功

**Step 2: 手动测试（可选）**

通过 Telegram 发送消息，验证响应末尾显示时间。

**Step 3: 最终 Commit**

```bash
git add -A
git commit -m "feat(zero-channels): complete execution time display for Telegram"
```

---

## 验收标准

- [x] `format_duration` 函数正确格式化各种时间范围
- [x] Telegram 响应消息末尾显示 `_⏱ Xs_` 格式的执行时间
- [x] 所有单元测试通过
- [x] 代码编译无警告
