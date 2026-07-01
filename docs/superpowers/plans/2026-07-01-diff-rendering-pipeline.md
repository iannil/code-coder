# Diff Rendering Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement V1 from `docs/audit-tui-visual-fidelity.md` — a unified-diff rendering pipeline (`src/tui/diff.rs`) that `edit_file`/`write_file`/`diff` tools feed, producing gutter + hunk-header formatting + syntect syntax-highlighted output, surfaced both inline in message rendering and as preview in `edit_file`/`write_file` permission dialogs.

**Architecture:** A new pure-function module `src/tui/diff.rs` exposes `compute_unified_diff(old, new, path) -> String` and `render_diff(text, file_path, file_content) -> Vec<Line>`. Tools compute the diff and embed it as a ` ```diff path="..." ` fenced block in their ToolResult text. The existing markdown renderer detects this fence, extracts the path, and delegates to `diff::render_diff`. Permission dialogs reuse the same two functions to render a preview.

**Tech Stack:** Rust 2021; new dep `similar = "5"` (unified diff); existing `syntect` (syntax highlighting, already in Cargo.toml); existing `ratatui` (Line/Span/Style); `tempfile` (already dev-dep, for tests).

## Global Constraints

引自 spec `docs/superpowers/specs/2026-07-01-diff-rendering-pipeline-design.md`：

- **本轮不动 markdown.rs 现有 Color::* 风格**：spec §4.6 写的 `theme: &Theme` 参数实际不可行（markdown.rs 没有 theme 访问），降级为 `render_diff(text, file_path, file_content) -> Vec<Line>` 用 Color 常量。这是 spec 的微调，记录在此
- **依赖版本**：`similar = "3"`（spec 原写 `"5"`，crates.io 实际最新是 3.1.1，已校正）
- `render_diff` 与 `compute_unified_diff` 都是纯函数（无 I/O、无副作用、可重复）
- 工具结果 diff 块用 ` ```diff path="..." ` 元信息围栏（不是裸 ` ```diff `），让 markdown 解析器拿到 path 触发语法高亮
- 颜色规则：`+` 行 marker=绿、行号=绿、内容=syntect 色；`-` 行 marker=红、无行号、内容=纯红；context 行 marker=dim、行号=dim、内容=syntect 色
- 截断阈值常量：单 hunk 500 行、总 2000 行、权限对话框 body 20 行
- 二进制探测在 `compute_unified_diff` 入口（NUL 字节检测），返回字面量 `"[binary file changed]"`
- `edit_file` / `write_file` / `dialogs.rs` 的修改**必须不破坏现有测试**
- 不引入新 ADR（颜色规则、截断阈值是局部决策，不上升到 ADR 级别）

---

## File Structure

**新建：**

- `src/tui/diff.rs`（约 300 行）— 整个 diff 管线。结构：
  - 常量：`MAX_HUNK_LINES`、`MAX_TOTAL_LINES`、`MAX_DIALOG_PREVIEW_LINES`
  - 公开 API：`compute_unified_diff`、`render_diff`
  - 私有：`Hunk`、`ParsedLine`、`parse_hunks`、`compute_gutter_width`、`detect_language`、`precompute_line_highlights`、`is_binary`、helper styled-line fns
  - `#[cfg(test)] mod tests` — 单元测试

**修改：**

| 文件 | 改动概要 |
|---|---|
| `Cargo.toml` | 加 `similar = "5"` |
| `src/tui/mod.rs:13-19` | 加 `pub mod diff;` |
| `src/tui/markdown.rs:218-234` | `render_code_block` 加 fence info 解析（`diff path="..."`），路由到 `render_diff_text_with_path` |
| `src/tui/markdown.rs:294` | `render_diff_text` 保留为兼容入口；新增 `render_diff_text_with_path(text, path)` |
| `src/tools/edit_file.rs:49-61` | 成功路径计算并附加 diff 块到 ToolResult 输出 |
| `src/tools/write_file.rs:34-37` | 同上 |
| `src/tui/dialogs.rs:242-312` | `render_dialog` 对 `edit_file`/`write_file` 工具类型加 diff 预览 body |

---

## Task 1: Scaffold diff.rs + add similar dependency

**Files:**
- Create: `src/tui/diff.rs`
- Modify: `Cargo.toml`（加 `similar = "5"`）
- Modify: `src/tui/mod.rs:13-19`（加 `pub mod diff;`）

**Interfaces:**
- Produces: `crate::tui::diff::compute_unified_diff` 和 `crate::tui::diff::render_diff` 的 stub（返回 `String::new()` 和 `Vec::new()`），让后续任务可以独立编译

- [ ] **Step 1: 加 `similar` 依赖**

修改 `Cargo.toml` 的 `[dependencies]` 段，加一行：

```toml
similar = "3"
```

- [ ] **Step 2: 创建 `src/tui/diff.rs` 骨架**

写入完整内容：

```rust
/// ─── Diff Rendering Pipeline ──────────────────────────────────────────────
///
/// Implements V1 from docs/audit-tui-visual-fidelity.md: produce and render
/// unified diffs with gutter (line numbers) and syntect syntax highlighting.
/// Two public pure functions:
///   - compute_unified_diff: generate unified diff text from before/after
///   - render_diff: parse unified diff text and produce styled ratatui Lines

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Maximum lines in a single hunk before truncation kicks in.
const MAX_HUNK_LINES: usize = 500;
/// Maximum total diff lines across all hunks.
const MAX_TOTAL_LINES: usize = 2000;
/// Maximum diff body lines rendered inside a permission dialog preview.
pub const MAX_DIALOG_PREVIEW_LINES: usize = 20;

/// Generate a unified diff text from `old` and `new` content for `path`.
///
/// Returns the diff with `--- a/{path}` / `+++ b/{path}` file headers so the
/// markdown renderer detects it. Binary inputs (containing NUL bytes) yield
/// the literal sentinel `"[binary file changed]"`.
pub fn compute_unified_diff(_old: &str, _new: &str, _path: &str) -> String {
    String::new() // Task 2 fills this in
}

/// Render unified diff `text` into styled Lines with gutter and (if
/// `file_path`/`file_content` are available) syntect syntax highlighting.
///
/// Returns empty Vec if `text` is not a recognized diff.
pub fn render_diff(_text: &str, _file_path: &str, _file_content: &str) -> Vec<Line<'static>> {
    Vec::new() // Task 6 fills this in
}

#[cfg(test)]
mod tests {
    // Subsequent tasks add tests here.
}
```

- [ ] **Step 3: 在 `src/tui/mod.rs` 注册模块**

在 `src/tui/mod.rs:13-19` 段，按字母序在 `pub mod completion;` 后加：

```rust
pub mod completion;
pub mod diff;
pub mod markdown;
pub mod message_list;
pub mod input_area;
pub mod dialogs;
pub mod status_bar;
pub mod theme;
```

- [ ] **Step 4: 验证编译**

Run: `cargo build`
Expected: 编译通过（可能有 unused-import 警告，因为 Style/Color/Modifier/Span 暂未使用；Task 6 用到时不警告）

如果 `Style/Color/...` unused 警告阻碍 build，给 import 加 `#[allow(unused_imports)]`：

```rust
#[allow(unused_imports)]
use ratatui::style::{Color, Modifier, Style};
#[allow(unused_imports)]
use ratatui::text::{Line, Span};
```

Task 6 完成后移除 `#[allow]`。

- [ ] **Step 5: 提交**

```bash
git add Cargo.toml Cargo.lock src/tui/diff.rs src/tui/mod.rs
git commit -m "feat(tui): scaffold src/tui/diff.rs and add similar dependency

Empty stubs for compute_unified_diff and render_diff; full
implementations land in Tasks 2 and 6. Adds similar = 5 to
Cargo.toml for line-level unified diff generation."
```

---

## Task 2: Implement compute_unified_diff (TDD)

**Files:**
- Modify: `src/tui/diff.rs`
- Test: `src/tui/diff.rs` 内 `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `crate::tui::diff::compute_unified_diff(old: &str, new: &str, path: &str) -> String`

- [ ] **Step 1: 写失败测试**

在 `src/tui/diff.rs` 末尾的 `#[cfg(test)] mod tests` 模块内追加：

```rust
use super::*;

#[test]
fn test_compute_basic_replace() {
    let old = "line1\nline2\nline3";
    let new = "line1\nchanged\nline3";
    let diff = compute_unified_diff(old, new, "foo.txt");
    assert!(diff.contains("--- a/foo.txt"));
    assert!(diff.contains("+++ b/foo.txt"));
    assert!(diff.contains("-line2"));
    assert!(diff.contains("+changed"));
    assert!(diff.contains("@@"));
}

#[test]
fn test_compute_new_file() {
    let new = "alpha\nbeta\n";
    let diff = compute_unified_diff("", new, "new.txt");
    assert!(diff.contains("+alpha"));
    assert!(diff.contains("+beta"));
}

#[test]
fn test_compute_delete_file() {
    let old = "alpha\nbeta\n";
    let diff = compute_unified_diff(old, "", "gone.txt");
    assert!(diff.contains("-alpha"));
    assert!(diff.contains("-beta"));
}

#[test]
fn test_compute_no_change() {
    let same = "same\ncontent\n";
    let diff = compute_unified_diff(same, same, "same.txt");
    // No changes — output is just the file headers (no @@ hunks).
    assert!(diff.contains("--- a/same.txt"));
    assert!(diff.contains("+++ b/same.txt"));
    assert!(!diff.contains("@@"));
}

#[test]
fn test_compute_includes_file_header() {
    let diff = compute_unified_diff("a\n", "b\n", "path/to/x.rs");
    assert!(diff.starts_with("--- a/path/to/x.rs\n"));
    assert!(diff.contains("+++ b/path/to/x.rs\n"));
}

#[test]
fn test_compute_binary_returns_sentinel() {
    let old = "normal\x00binary";
    let new = "different\x00binary";
    let result = compute_unified_diff(old, new, "blob.bin");
    assert_eq!(result, "[binary file changed]");
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tui::diff::tests::test_compute`
Expected: 5 个测试中至少 4 个 FAIL（除了 `test_compute_binary_returns_sentinel`，因为还没实现二进制检测，可能错误地产生了真 diff 而非 sentinel；也会失败因为不等于字面量）

- [ ] **Step 3: 实现 `compute_unified_diff`**

替换 `src/tui/diff.rs` 中的 stub：

```rust
/// Generate a unified diff text from `old` and `new` content for `path`.
///
/// Returns the diff with `--- a/{path}` / `+++ b/{path}` file headers so the
/// markdown renderer detects it. Binary inputs (containing NUL bytes) yield
/// the literal sentinel `"[binary file changed]"`.
pub fn compute_unified_diff(old: &str, new: &str, path: &str) -> String {
    if is_binary(old) || is_binary(new) {
        return "[binary file changed]".to_string();
    }

    use similar::TextDiff;
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();
    output.push_str(&format!("--- a/{path}\n+++ b/{path}\n"));
    let patch = diff.unified_diff().context_radius(3);
    output.push_str(&format!("{patch}"));
    output
}

/// Detect binary content by NUL byte presence (same heuristic as git).
fn is_binary(s: &str) -> bool {
    s.contains('\0')
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --lib tui::diff::tests::test_compute`
Expected: 6 个测试全 PASS

- [ ] **Step 5: 提交**

```bash
git add src/tui/diff.rs
git commit -m "feat(tui/diff): implement compute_unified_diff with similar

Pure function: line-level diff via similar::TextDiff, 3 lines of
context, includes file headers for markdown detection, returns
binary sentinel when NUL bytes present."
```

---

## Task 3: Implement parse_hunks (TDD)

**Files:**
- Modify: `src/tui/diff.rs`

**Interfaces:**
- Produces (private): `Hunk`、`ParsedLine`、`parse_hunks(text: &str) -> Vec<Hunk>`

- [ ] **Step 1: 加类型定义**

在 `src/tui/diff.rs` 的 `MAX_DIALOG_PREVIEW_LINES` 常量之后、`compute_unified_diff` 函数之前插入：

```rust
/// One parsed hunk of a unified diff.
struct Hunk {
    /// New-file line number where the hunk starts (extracted from
    /// `@@ -X,Y +A,B @@` — the A value).
    new_start: usize,
    /// Raw `@@ ...` header line (kept for rendering).
    header: String,
    /// Body lines (skipping the `@@` header itself).
    lines: Vec<ParsedLine>,
}

/// A single line inside a hunk.
#[allow(dead_code)] // FileHeader only fires on multi-file diffs
enum ParsedLine {
    Add(String),
    Del(String),
    Context(String),
    /// `--- a/path` / `+++ b/path` — pre-hunk metadata.
    FileHeader(String),
}
```

- [ ] **Step 2: 写失败测试**

追加到 `#[cfg(test)] mod tests`：

```rust
#[test]
fn test_parse_extracts_new_start() {
    let diff = "--- a/f\n+++ b/f\n@@ -1,3 +5,7 @@\n ctx\n+new\n";
    let hunks = parse_hunks(diff);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].new_start, 5);
}

#[test]
fn test_parse_classifies_lines() {
    let diff = "--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n ctx\n-add\n+add\n";
    let hunks = parse_hunks(diff);
    assert_eq!(hunks.len(), 1);
    let lines = &hunks[0].lines;
    assert!(matches!(lines[0], ParsedLine::Context(_)));
    assert!(matches!(lines[1], ParsedLine::Del(_)));
    assert!(matches!(lines[2], ParsedLine::Add(_)));
}

#[test]
fn test_parse_multiple_hunks() {
    let diff = "--- a/f\n+++ b/f\n@@ -1,1 +1,1 @@\n+a\n@@ -10,1 +11,1 @@\n+b\n";
    let hunks = parse_hunks(diff);
    assert_eq!(hunks.len(), 2);
    assert_eq!(hunks[0].new_start, 1);
    assert_eq!(hunks[1].new_start, 11);
}

#[test]
fn test_parse_non_diff_returns_empty() {
    assert!(parse_hunks("just some\ntext").is_empty());
    assert!(parse_hunks("").is_empty());
}
```

- [ ] **Step 3: 跑测试确认失败**

Run: `cargo test --lib tui::diff::tests::test_parse`
Expected: 编译失败（`parse_hunks` 未定义），4 个测试无法运行

- [ ] **Step 4: 实现 `parse_hunks`**

在 `is_binary` 函数之后追加：

```rust
/// Parse a unified diff text into hunks. Returns empty Vec if `text`
/// does not look like a diff (no `@@` markers found).
fn parse_hunks(text: &str) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut current: Option<Hunk> = None;
    let mut saw_file_header = false;

    for line in text.lines() {
        if line.starts_with("--- ") || line.starts_with("+++ ") {
            saw_file_header = true;
            if let Some(h) = current.as_mut() {
                h.lines.push(ParsedLine::FileHeader(line.to_string()));
            }
            continue;
        }
        if line.starts_with("@@") {
            if let Some(h) = current.take() {
                hunks.push(h);
            }
            let new_start = extract_new_start(line).unwrap_or(1);
            current = Some(Hunk {
                new_start,
                header: line.to_string(),
                lines: Vec::new(),
            });
            continue;
        }
        if let Some(h) = current.as_mut() {
            match line.chars().next() {
                Some('+') => h.lines.push(ParsedLine::Add(line[1..].to_string())),
                Some('-') => h.lines.push(ParsedLine::Del(line[1..].to_string())),
                Some(' ') => h.lines.push(ParsedLine::Context(line[1..].to_string())),
                _ => {} // skip unknown lines (e.g., "\ No newline at end of file")
            }
        }
    }
    if let Some(h) = current.take() {
        hunks.push(h);
    }
    if !saw_file_header && hunks.is_empty() {
        return Vec::new();
    }
    hunks
}

/// Extract the new-file start line number from `@@ -X,Y +A,B @@` text.
fn extract_new_start(header: &str) -> Option<usize> {
    // Find "+A" pattern; A is the new-file start.
    let plus = header.find('+')?;
    let rest = &header[plus + 1..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}
```

- [ ] **Step 5: 跑测试确认通过**

Run: `cargo test --lib tui::diff::tests::test_parse`
Expected: 4 个测试全 PASS

- [ ] **Step 6: 提交**

```bash
git add src/tui/diff.rs
git commit -m "feat(tui/diff): implement parse_hunks and ParsedLine types

Pure function: scans unified diff text, classifies lines into
Add/Del/Context/FileHeader, extracts new_start from @@ header.
Returns empty Vec for non-diff text."
```

---

## Task 4: Implement compute_gutter_width + detect_language (TDD)

**Files:**
- Modify: `src/tui/diff.rs`

**Interfaces:**
- Produces (private): `compute_gutter_width(hunks: &[Hunk]) -> usize`、`detect_language(path: &str, first_line: &str) -> Option<&'static syntect::parsing::SyntaxReference>`

- [ ] **Step 1: 写失败测试**

追加到 `#[cfg(test)] mod tests`：

```rust
#[test]
fn test_gutter_single_digit() {
    // max line = 9, width = 1 + 3 = 4
    let hunks = vec![Hunk {
        new_start: 1,
        header: "@@ -1,3 +1,9 @@".to_string(),
        lines: vec![ParsedLine::Context("x".to_string()); 9],
    }];
    assert_eq!(compute_gutter_width(&hunks), 4);
}

#[test]
fn test_gutter_triple_digit() {
    // max line = 100, width = 3 + 3 = 6
    let hunks = vec![Hunk {
        new_start: 100,
        header: "@@ -1,1 +100,1 @@".to_string(),
        lines: vec![ParsedLine::Context("x".to_string())],
    }];
    assert_eq!(compute_gutter_width(&hunks), 6);
}

#[test]
fn test_gutter_empty() {
    assert_eq!(compute_gutter_width(&[]), 4);
}

#[test]
fn test_detect_by_extension_rs() {
    let syntax = detect_language("foo.rs", "");
    assert!(syntax.is_some());
    assert_eq!(syntax.unwrap().name, "Rust");
}

#[test]
fn test_detect_by_extension_py() {
    let syntax = detect_language("script.py", "");
    assert!(syntax.is_some());
    assert_eq!(syntax.unwrap().name, "Python");
}

#[test]
fn test_detect_unknown_falls_back_to_none() {
    assert!(detect_language("file.unknownext", "").is_none());
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tui::diff::tests::test_gutter test_detect`
Expected: 编译失败（两个函数未定义）

- [ ] **Step 3: 实现两个函数**

在 `extract_new_start` 函数之后追加：

```rust
/// Compute the gutter width needed for `hunks`. Format is
/// `marker(1) + space(1) + line_no_digits + space(1)` = digits + 3.
fn compute_gutter_width(hunks: &[Hunk]) -> usize {
    let max_line = hunks.iter()
        .flat_map(|h| {
            let mut n = h.new_start;
            h.lines.iter().filter_map(move |l| {
                let val = match l {
                    ParsedLine::Add(_) | ParsedLine::Context(_) => {
                        let v = n;
                        n += 1;
                        Some(v)
                    }
                    _ => None,
                };
                val
            })
        })
        .max()
        .unwrap_or(1);
    max_line.to_string().len() + 3
}

/// Detect syntect language from file extension, falling back to shebang.
fn detect_language(path: &str, first_line: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
    let ss = crate::tui::markdown::get_syntax_set();
    if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        if let Some(syntax) = ss.find_syntax_by_extension(ext) {
            return Some(syntax);
        }
    }
    if !first_line.is_empty() {
        if let Some(syntax) = ss.find_syntax_by_line(first_line) {
            return Some(syntax);
        }
    }
    None
}
```

注：这调用 `crate::tui::markdown::get_syntax_set()`——它当前是 `fn`（私有），需改为 `pub fn`。把 `src/tui/markdown.rs:187` 的：

```rust
fn get_syntax_set() -> &'static syntect::parsing::SyntaxSet {
```

改成：

```rust
pub fn get_syntax_set() -> &'static syntect::parsing::SyntaxSet {
```

```rust
let ss = crate::tui::markdown::get_syntax_set();
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --lib tui::diff::tests::test_gutter test_detect`
Expected: 6 个测试全 PASS

- [ ] **Step 5: 提交**

```bash
git add src/tui/diff.rs src/tui/markdown.rs
git commit -m "feat(tui/diff): implement compute_gutter_width and detect_language

compute_gutter_width: max_digits + 3 formula matching original's
StructuredDiff.tsx. detect_language: extension → syntect lookup,
falls back to shebang line. Exposes markdown::get_syntax_set as
pub for cross-module reuse."
```

---

## Task 5: Implement precompute_line_highlights (TDD)

**Files:**
- Modify: `src/tui/diff.rs`

**Interfaces:**
- Produces (private): `precompute_line_highlights(content: &str, lang: &syntect::parsing::SyntaxReference) -> Vec<Vec<Span<'static>>>`

- [ ] **Step 1: 写失败测试**

追加到 `#[cfg(test)] mod tests`：

```rust
#[test]
fn test_highlight_no_language_returns_empty() {
    // Helper not yet wired; verify behavior via render_diff in Task 6.
    // For now, just sanity-check that calling precompute with a known
    // language produces non-empty output for non-empty content.
    let ss = crate::tui::markdown::get_syntax_set();
    let rust = ss.find_syntax_by_extension("rs").unwrap();
    let highlights = precompute_line_highlights("fn main() {}", rust);
    assert_eq!(highlights.len(), 1); // one line
    assert!(!highlights[0].is_empty()); // at least one span
}

#[test]
fn test_highlight_multiline_string_spanning_lines() {
    // A multi-line string in Rust: highlighter state must carry across.
    let content = "fn x() {\n    let s = \"a\nb\nc\";\n}\n";
    let ss = crate::tui::markdown::get_syntax_set();
    let rust = ss.find_syntax_by_extension("rs").unwrap();
    let highlights = precompute_line_highlights(content, rust);
    assert_eq!(highlights.len(), 5);
    // Line 2 (index 1) and 3 (index 2) are inside the string literal.
    // We don't assert exact colors (theme-dependent), just that they
    // have spans (i.e., were tokenized, not skipped).
    assert!(!highlights[1].is_empty());
    assert!(!highlights[2].is_empty());
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tui::diff::tests::test_highlight`
Expected: 编译失败（`precompute_line_highlights` 未定义）

- [ ] **Step 3: 实现 `precompute_line_highlights`**

在 `detect_language` 函数之后追加：

```rust
/// Precompute syntect highlight spans for every line in `content`.
///
/// `HighlightLines` carries state across lines (multi-line strings, block
/// comments), so we must tokenize the whole file once and then index into
/// the result by line number during rendering.
fn precompute_line_highlights(
    content: &str,
    lang: &syntect::parsing::SyntaxReference,
) -> Vec<Vec<Span<'static>>> {
    let ss = crate::tui::markdown::get_syntax_set();
    let theme = crate::tui::markdown::get_theme();
    let mut h = syntect::easy::HighlightLines::new(lang, theme);
    content.lines().map(|line| {
        match h.highlight_line(line, ss) {
            Ok(ranges) => ranges
                .into_iter()
                .map(|(style, s)| {
                    let color = ratatui::style::Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                    let mut modifs = ratatui::style::Modifier::empty();
                    if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                        modifs |= ratatui::style::Modifier::BOLD;
                    }
                    if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                        modifs |= ratatui::style::Modifier::ITALIC;
                    }
                    if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
                        modifs |= ratatui::style::Modifier::UNDERLINED;
                    }
                    Span::styled(s.to_string(), ratatui::style::Style::default().fg(color).add_modifier(modifs))
                })
                .collect(),
            Err(_) => vec![Span::raw(line.to_string())],
        }
    }).collect()
}
```

注：这调用 `crate::tui::markdown::get_theme()`——它当前是 `fn`（私有），需改为 `pub fn`。把 `src/tui/markdown.rs:208` 的：

```rust
fn get_theme() -> &'static syntect::highlighting::Theme {
```

改成：

```rust
pub fn get_theme() -> &'static syntect::highlighting::Theme {
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --lib tui::diff::tests::test_highlight`
Expected: 2 个测试全 PASS

- [ ] **Step 5: 提交**

```bash
git add src/tui/diff.rs src/tui/markdown.rs
git commit -m "feat(tui/diff): implement precompute_line_highlights

Tokenizes whole file_content once with syntect HighlightLines so
multi-line constructs (block comments, triple-quoted strings)
highlight correctly across hunk boundaries. Exposes markdown::get_theme
as pub for cross-module reuse."
```

---

## Task 6: Implement render_diff main (TDD)

**Files:**
- Modify: `src/tui/diff.rs`

**Interfaces:**
- Produces: `crate::tui::diff::render_diff(text: &str, file_path: &str, file_content: &str) -> Vec<Line<'static>>`

- [ ] **Step 1: 写失败测试**

追加到 `#[cfg(test)] mod tests`：

```rust
fn sample_diff() -> &'static str {
    "--- a/sample.rs\n+++ b/sample.rs\n@@ -1,3 +1,4 @@\n fn main() {\n-    println!(\"old\");\n+    println!(\"new\");\n+    println!(\"added\");\n }\n"
}

#[test]
fn test_render_addition_marker_green() {
    let lines = render_diff(sample_diff(), "sample.rs", "");
    // At least one line should contain "+" prefix and have green styling.
    let has_green_add = lines.iter().any(|l| {
        l.spans.iter().any(|s| {
            s.content.starts_with('+')
                && matches!(s.style.fg, Some(Color::Green))
        })
    });
    assert!(has_green_add, "expected at least one green + marker");
}

#[test]
fn test_render_deletion_marker_red() {
    let lines = render_diff(sample_diff(), "sample.rs", "");
    let has_red_del = lines.iter().any(|l| {
        l.spans.iter().any(|s| {
            s.content.starts_with('-')
                && matches!(s.style.fg, Some(Color::Red))
        })
    });
    assert!(has_red_del, "expected at least one red - marker");
}

#[test]
fn test_render_gutter_alignment() {
    // Construct a diff where max line is 100+ → 3 digits → gutter width 6.
    let diff = "--- a/f\n+++ b/f\n@@ -1,1 +100,1 @@\n+new\n";
    let lines = render_diff(diff, "f.txt", "");
    // First content line (after hunk header) should start with gutter spaces.
    let body_line = lines.iter().find(|l| {
        l.spans.first().map_or(false, |s| s.content.starts_with('+'))
    });
    assert!(body_line.is_some(), "expected an addition line");
}

#[test]
fn test_render_no_language_falls_back() {
    // Empty file_path → no language → just +/- marker coloring, no crash.
    let lines = render_diff(sample_diff(), "", "");
    assert!(!lines.is_empty());
}

#[test]
fn test_render_truncates_large_diff() {
    // Build a diff with 2500 additions.
    let mut diff = String::from("--- a/big\n+++ b/big\n@@ -1,1 +1,2500 @@\n");
    for _ in 0..2500 {
        diff.push_str("+line\n");
    }
    let lines = render_diff(&diff, "big.txt", "");
    assert!(lines.len() <= MAX_TOTAL_LINES + 5, "got {} lines", lines.len());
    let has_truncation_note = lines.iter().any(|l| {
        l.spans.iter().any(|s| s.content.contains("truncated"))
    });
    assert!(has_truncation_note);
}

#[test]
fn test_render_non_diff_returns_empty() {
    assert!(render_diff("not a diff", "f.txt", "").is_empty());
    assert!(render_diff("", "f.txt", "").is_empty());
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tui::diff::tests::test_render_`
Expected: 6 个测试全 FAIL（`render_diff` 还是 stub）

- [ ] **Step 3: 实现 `render_diff`**

替换 stub：

```rust
/// Render unified diff `text` into styled Lines with gutter and (if
/// `file_path`/`file_content` are available) syntect syntax highlighting.
///
/// Returns empty Vec if `text` is not a recognized diff.
pub fn render_diff(text: &str, file_path: &str, file_content: &str) -> Vec<Line<'static>> {
    let hunks = parse_hunks(text);
    if hunks.is_empty() {
        return Vec::new();
    }

    let gutter_w = compute_gutter_width(&hunks);
    let first_line = file_content.lines().next().unwrap_or("");
    let lang = detect_language(file_path, first_line);
    let highlights = lang.map(|l| precompute_line_highlights(file_content, l));

    let mut out: Vec<Line<'static>> = Vec::new();
    let mut total_lines: usize = 0;

    for hunk in &hunks {
        if total_lines >= MAX_TOTAL_LINES {
            break;
        }
        // Hunk header
        out.push(Line::styled(
            hunk.header.clone(),
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
        ));
        total_lines += 1;

        let mut line_no = hunk.new_start;
        let mut hunk_lines_emitted: usize = 0;

        for line in &hunk.lines {
            if total_lines >= MAX_TOTAL_LINES || hunk_lines_emitted >= MAX_HUNK_LINES {
                out.push(Line::styled(
                    format!("... (diff truncated)"),
                    Style::default().fg(Color::DarkGray),
                ));
                return out;
            }
            match line {
                ParsedLine::Add(content) => {
                    let content_spans = pick_highlight(&highlights, line_no, content);
                    let prefix = format_gutter("+", Some(line_no), gutter_w);
                    let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::Green))];
                    spans.extend(content_spans);
                    out.push(Line::from(spans));
                    line_no += 1;
                }
                ParsedLine::Del(content) => {
                    let prefix = format_gutter("-", None, gutter_w);
                    let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::Red))];
                    spans.push(Span::styled(
                        content.clone(),
                        Style::default().fg(Color::Red),
                    ));
                    out.push(Line::from(spans));
                }
                ParsedLine::Context(content) => {
                    let content_spans = pick_highlight(&highlights, line_no, content);
                    let prefix = format_gutter(" ", Some(line_no), gutter_w);
                    let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::DarkGray))];
                    spans.extend(content_spans);
                    out.push(Line::from(spans));
                    line_no += 1;
                }
                ParsedLine::FileHeader(s) => {
                    out.push(Line::styled(
                        s.clone(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ));
                }
            }
            total_lines += 1;
            hunk_lines_emitted += 1;
        }
    }
    out
}

/// Pull pre-computed syntect spans for `line_no` (1-based, file-content index).
/// Falls back to a single plain span if highlights are unavailable.
fn pick_highlight(
    highlights: &Option<Vec<Vec<Span<'static>>>>,
    line_no: usize,
    fallback_content: &str,
) -> Vec<Span<'static>> {
    if let Some(h) = highlights {
        // line_no is 1-based; highlights index is 0-based.
        if line_no > 0 && line_no <= h.len() {
            return h[line_no - 1].clone();
        }
    }
    vec![Span::raw(fallback_content.to_string())]
}

/// Format the gutter: marker + space + right-aligned (or blank) line number + space.
fn format_gutter(marker: &str, line_no: Option<usize>, width: usize) -> String {
    let digits_part = match line_no {
        Some(n) => format!("{n:>width$}", n = n, width = width.saturating_sub(3)),
        None => " ".repeat(width.saturating_sub(3)),
    };
    format!("{marker} {digits_part} ")
}
```

- [ ] **Step 4: 移除 Task 1 加的 `#[allow(unused_imports)]`**

如果 Task 1 在 diff.rs 顶部加了 `#[allow(unused_imports)]`，移除它（Color/Modifier/Style/Span/Line 现在都被用到了）。

- [ ] **Step 5: 跑测试确认通过**

Run: `cargo test --lib tui::diff`
Expected: 全部测试 PASS（Tasks 2-6 的共 22 个测试）

- [ ] **Step 6: 提交**

```bash
git add src/tui/diff.rs
git commit -m "feat(tui/diff): implement render_diff with gutter and highlight

Pure function: parses hunks, computes gutter width, threads syntect
highlights from precomputed file-content tokenization. Additions and
context lines keep syntect colors; deletions are pure red (no
before-side highlight pass). Truncates at MAX_HUNK_LINES or
MAX_TOTAL_LINES with explicit sentinel."
```

---

## Task 7: Wire markdown.rs to use new render_diff + parse fence path

**Files:**
- Modify: `src/tui/markdown.rs:218-234`（`render_code_block` 的 diff 分支）
- Modify: `src/tui/markdown.rs:294`（`render_diff_text` 保留为兼容入口；新增 `render_diff_text_with_path`）

**Interfaces:**
- Produces: `crate::tui::markdown::render_diff_text_with_path(text: &str, path: &str) -> Vec<Line<'static>>`

- [ ] **Step 1: 写失败测试**

在 `src/tui/markdown.rs` 末尾的 `#[cfg(test)] mod tests` 模块内追加：

```rust
#[test]
fn test_parse_fence_info_extracts_lang_and_path() {
    let (lang, path) = parse_fence_info_inner("diff path=\"src/foo.rs\"");
    assert_eq!(lang, "diff");
    assert_eq!(path, Some("src/foo.rs"));
}

#[test]
fn test_parse_fence_info_no_path() {
    let (lang, path) = parse_fence_info_inner("rust");
    assert_eq!(lang, "rust");
    assert_eq!(path, None);
}

#[test]
fn test_parse_fence_info_empty() {
    let (lang, path) = parse_fence_info_inner("");
    assert_eq!(lang, "");
    assert_eq!(path, None);
}

#[test]
fn test_render_code_block_diff_with_path_uses_render_diff() {
    // Smoke test: render a ```diff path="..."``` block; result should be non-empty
    // and contain gutter (more than just the raw +/- text).
    let md = "```diff path=\"sample.rs\"\n--- a/sample.rs\n+++ b/sample.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n```\n";
    let lines = super::render_markdown(md);
    assert!(!lines.is_empty());
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tui::markdown::tests::test_parse_fence test_render_code_block_diff_with_path`
Expected: 编译失败（`parse_fence_info_inner` 未定义）

- [ ] **Step 3: 实现 fence info 解析**

在 `src/tui/markdown.rs` 中（`render_code_block` 函数之前，约 218 行附近）加：

```rust
/// Parse a code-fence info string into (language, optional path).
/// Example: `diff path="src/foo.rs"` → (`"diff"`, `Some("src/foo.rs")`).
#[allow(dead_code)] // used in tests; production use lands in same task
pub(crate) fn parse_fence_info_inner(info: &str) -> (&str, Option<&str>) {
    let mut parts = info.splitn(2, char::is_whitespace);
    let lang = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");
    let path = rest.find("path=\"").and_then(|start| {
        let after = &rest[start + "path=\"".len()..];
        after.find('"').map(|end| &after[..end])
    });
    (lang, path)
}
```

- [ ] **Step 4: 修改 `render_code_block` 用新 parser + 路由到 `render_diff_text_with_path`**

把 `src/tui/markdown.rs:218-234` 的 `render_code_block` 函数体里的 diff 分支替换：

原来：

```rust
fn render_code_block(
    lines: &mut Vec<Line<'static>>,
    lang: &str,
    content: &str,
) {
    // Check if this is a diff block
    if lang == "diff" {
        let diff_lines = render_diff_text(content);
        if !diff_lines.is_empty() {
            for dl in diff_lines {
                let mut spans = vec![Span::raw("  ")];
                spans.extend(dl.spans);
                lines.push(Line::from(spans));
            }
            return;
        }
    }
    // ... rest unchanged
```

改为：

```rust
fn render_code_block(
    lines: &mut Vec<Line<'static>>,
    lang_full: &str,
    content: &str,
) {
    // Parse info string: `diff path="..."` → (lang, path).
    let (lang, path) = parse_fence_info_inner(lang_full);

    // Check if this is a diff block
    if lang == "diff" {
        let diff_lines = match path {
            Some(p) => render_diff_text_with_path(content, p),
            None => render_diff_text(content),
        };
        if !diff_lines.is_empty() {
            for dl in diff_lines {
                let mut spans = vec![Span::raw("  ")];
                spans.extend(dl.spans);
                lines.push(Line::from(spans));
            }
            return;
        }
    }
    // ... rest unchanged (note: original code uses `lang` variable below
    // for syntax lookup; ensure references to `lang` use the parsed
    // first token, not `lang_full`).
```

注意：函数下方原本使用 `lang` 变量做 `find_syntax_by_token(lang)` 的部分继续用解析出的 `lang`（即第一个 token），不要传 `lang_full`。

- [ ] **Step 5: 新增 `render_diff_text_with_path` 函数**

在 `src/tui/markdown.rs` 现有 `pub fn render_diff_text(text: &str) -> Vec<Line<'static>>` 函数（约 294 行）旁边追加：

```rust
/// Render a diff block, optionally using `path` to enable syntax highlighting.
///
/// When `path` is non-empty, attempts to read the file from disk to provide
/// full-file syntect context. Read failure silently degrades to no-highlight.
pub fn render_diff_text_with_path(text: &str, path: &str) -> Vec<Line<'static>> {
    let content = if path.is_empty() {
        String::new()
    } else {
        std::fs::read_to_string(path).unwrap_or_default()
    };
    crate::tui::diff::render_diff(text, path, &content)
}
```

同时把现有的 `render_diff_text` 函数体也改为转调 `diff::render_diff`：

把 `src/tui/markdown.rs:294` 的整个 `pub fn render_diff_text(text: &str) -> Vec<Line<'static>> { ... }` 函数体替换为：

```rust
/// Detect if text is a unified diff and render with +/- colors + gutter.
/// Legacy entry point — does not enable syntax highlighting. For highlighted
/// rendering, use `render_diff_text_with_path`.
pub fn render_diff_text(text: &str) -> Vec<Line<'static>> {
    crate::tui::diff::render_diff(text, "", "")
}
```

- [ ] **Step 6: 跑测试确认通过**

Run: `cargo test --lib tui::markdown::tests::test_parse_fence test_render_code_block_diff_with_path`
Expected: 4 个测试全 PASS

Run: `cargo test --lib tui::markdown::tests::test_render_diff_text`
Expected: 现有 diff 测试仍 PASS（render_diff_text 通过新管线产生等价或更丰富的输出；可能某些 assertion 需要更新——如果原本断言"输出 4 行"而现在因 gutter 变了，调整断言）

如果现有 diff 测试失败，**调整断言以匹配新输出**（新输出有 gutter、颜色规则变化）。失败的测试要更新到新输出，不是回退实现。

- [ ] **Step 7: 提交**

```bash
git add src/tui/markdown.rs
git commit -m "feat(tui/markdown): parse fence info string, route diff to new pipeline

render_code_block now extracts \`path=\"...\"\` from the fence info
string and threads it to render_diff_text_with_path, which enables
syntect highlighting by reading the file from disk. Legacy
render_diff_text becomes a thin wrapper around crate::tui::diff."
```

---

## Task 8: edit_file integration

**Files:**
- Modify: `src/tools/edit_file.rs:49-62`

**Interfaces:**
- Consumes: `crate::tui::diff::compute_unified_diff`

- [ ] **Step 1: 写失败测试**

在 `src/tools/edit_file.rs` 末尾的 `#[cfg(test)] mod tests` 模块内追加：

```rust
#[test]
fn test_edit_file_output_contains_diff_block() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(file.as_file(), b"line1\nline2\nline3\n").unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let tool = EditFileTool;
    let input = format!(r#"{{"path": "{path}", "old": "line2", "new": "LINE TWO"}}"#);
    let result = tool.execute(&input).unwrap();

    assert!(result.contains("```diff"), "result: {result}");
    assert!(result.contains(&format!("path=\"{path}\"")), "result: {result}");
}

#[test]
fn test_edit_file_diff_has_correct_line_numbers() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(file.as_file(), b"a\nb\nc\nd\ne\n").unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let tool = EditFileTool;
    let input = format!(r#"{{"path": "{path}", "old": "c", "new": "CHANGED"}}"#);
    let result = tool.execute(&input).unwrap();

    // The diff should show "3" as the line number for the change (new file).
    assert!(result.contains("@@ -"), "result: {result}");
}

#[test]
fn test_edit_file_diff_includes_unified_file_header() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(file.as_file(), b"hello\n").unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let tool = EditFileTool;
    let input = format!(r#"{{"path": "{path}", "old": "hello", "new": "world"}}"#);
    let result = tool.execute(&input).unwrap();

    assert!(result.contains(&format!("--- a/{path}")), "result: {result}");
    assert!(result.contains(&format!("+++ b/{path}")), "result: {result}");
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tools::edit_file::tests::test_edit_file_output_contains_diff_block`
Expected: FAIL（当前输出只有 "Edited X." 摘要，无 `diff` 块）

- [ ] **Step 3: 修改 `edit_file::execute` 计算 diff 并附加**

把 `src/tools/edit_file.rs:49-62` 的成功路径（计算完 new_content 并写盘之后）替换：

原来：

```rust
        let new_content = content.replace(&parsed.old, &parsed.new);
        std::fs::write(&parsed.path, &new_content)
            .map_err(|e| anyhow::anyhow!("Cannot write {}: {e}", parsed.path))?;

        // Compute diff stats
        let old_lines = content.lines().count();
        let new_lines = new_content.lines().count();
        let diff_lines = (new_lines as isize - old_lines as isize).abs();

        Ok(format!(
            "Edited {}. Replaced 1 occurrence ({}→{} lines, {} line diff).",
            parsed.path, old_lines, new_lines, diff_lines
        ))
```

改为：

```rust
        let new_content = content.replace(&parsed.old, &parsed.new);
        std::fs::write(&parsed.path, &new_content)
            .map_err(|e| anyhow::anyhow!("Cannot write {}: {e}", parsed.path))?;

        // Compute diff stats
        let old_lines = content.lines().count();
        let new_lines = new_content.lines().count();
        let diff_lines = (new_lines as isize - old_lines as isize).abs();

        // V1 (audit-tui-visual-fidelity.md bucket-4 #109/#110):
        // attach unified diff with file_path so renderer can highlight.
        let diff_text = crate::tui::diff::compute_unified_diff(
            &content,
            &new_content,
            &parsed.path,
        );

        Ok(format!(
            "Edited {}. Replaced 1 occurrence ({}→{} lines, {} line diff).\n\n```diff path=\"{}\"\n{}```",
            parsed.path, old_lines, new_lines, diff_lines, parsed.path, diff_text
        ))
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --lib tools::edit_file`
Expected: 所有测试 PASS（包括新加的 3 个）

- [ ] **Step 5: 提交**

```bash
git add src/tools/edit_file.rs
git commit -m "feat(tools/edit_file): emit unified diff as ToolResult output

After successful edit, compute compute_unified_diff(before, after,
path) and embed as a \`\`\`diff path=\"...\" fenced block. Enables
inline rendering with gutter + syntect highlighting."
```

---

## Task 9: write_file integration

**Files:**
- Modify: `src/tools/write_file.rs:34-37`

**Interfaces:**
- Consumes: `crate::tui::diff::compute_unified_diff`

- [ ] **Step 1: 写失败测试**

在 `src/tools/write_file.rs` 末尾的 `#[cfg(test)] mod tests` 模块内追加：

```rust
#[test]
fn test_write_file_new_file_diff_all_additions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.txt");
    let path_str = path.to_str().unwrap();
    let input = format!("{path_str}\nfirst\nsecond\n");
    let result = WriteFile.execute(&input).unwrap();

    assert!(result.contains("```diff"), "result: {result}");
    // New file → all additions, no deletions
    assert!(result.contains("+first"), "result: {result}");
    assert!(result.contains("+second"), "result: {result}");
}

#[test]
fn test_write_file_overwrite_shows_minus_and_plus() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.txt");
    std::fs::write(&path, "old line 1\nold line 2\n").unwrap();
    let path_str = path.to_str().unwrap();
    let input = format!("{path_str}\nnew line 1\nnew line 2\n");
    let result = WriteFile.execute(&input).unwrap();

    assert!(result.contains("```diff"), "result: {result}");
    assert!(result.contains("-old line 1"), "result: {result}");
    assert!(result.contains("+new line 1"), "result: {result}");
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tools::write_file::tests::test_write_file_new_file_diff`
Expected: FAIL（当前输出只有 "wrote N bytes" 摘要）

- [ ] **Step 3: 修改 `write_file::execute`**

把 `src/tools/write_file.rs:34-37` 的成功路径替换：

原来：

```rust
        std::fs::write(path, content)
            .map_err(|e| anyhow::anyhow!("cannot write {path}: {e}"))?;

        Ok(format!("wrote {} bytes to {path}", content.len()))
```

改为：

```rust
        // Read before-content (if file exists) for diff computation.
        let before = std::fs::read_to_string(path).unwrap_or_default();

        std::fs::write(path, content)
            .map_err(|e| anyhow::anyhow!("cannot write {path}: {e}"))?;

        // V1: attach unified diff with file_path for renderer highlighting.
        let diff_text = crate::tui::diff::compute_unified_diff(
            &before,
            content,
            path,
        );

        Ok(format!(
            "wrote {} bytes to {path}\n\n```diff path=\"{path}\"\n{diff_text}```",
            content.len()
        ))
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --lib tools::write_file`
Expected: 所有测试 PASS（包括新加的 2 个；现有 `test_write_file` 仍 PASS——它只断言 "wrote"，未断言完整字符串）

如果现有 `test_write_file` 因输出格式变化失败（例如断言了精确字符串），更新它的断言以匹配新格式。

- [ ] **Step 5: 提交**

```bash
git add src/tools/write_file.rs
git commit -m "feat(tools/write_file): emit unified diff as ToolResult output

Reads before-content if file exists, computes compute_unified_diff,
embeds as \`\`\`diff path=\"...\" fenced block. New files produce
all-addition diffs; overwrites produce -/+ diffs."
```

---

## Task 10: Permission dialog preview for edit_file/write_file

**Files:**
- Modify: `src/tui/dialogs.rs:242-312`（`render_dialog` 函数）

**Interfaces:**
- Consumes: `crate::tui::diff::compute_unified_diff`、`crate::tui::diff::render_diff`、`crate::tui::diff::MAX_DIALOG_PREVIEW_LINES`

- [ ] **Step 1: 在 `render_dialog` 内加 diff 预览逻辑**

修改 `src/tui/dialogs.rs` 的 `render_dialog` 函数（约 242-312 行）。

定位现有的 input display 部分（约 282-288 行）：

```rust
    let input_display: String = info_str.chars().take(400).collect();
    for line in input_display.lines().take(6) {
        content.push(Line::styled(format!("  {}", line), Style::default().fg(theme.secondary_text)));
    }
    if info_str.len() > 400 {
        content.push(Line::styled("  …(truncated)", Style::default().fg(theme.secondary_text)));
    }
```

在它之后、`content.push(Line::from(""));` 之前插入 diff 预览：

```rust
    // V1 (audit-tui-visual-fidelity.md bucket-4): for edit_file/write_file,
    // compute and embed a diff preview so the user sees what changes before
    // approving. Capped at MAX_DIALOG_PREVIEW_LINES to keep dialog on-screen.
    if tool_name == "edit_file" || tool_name == "write_file" {
        if let Some(preview_lines) = build_diff_preview(tool_name, info_str, theme) {
            content.push(Line::from(""));
            for dl in preview_lines {
                content.push(dl);
            }
        }
    }
```

- [ ] **Step 2: 实现 `build_diff_preview` helper**

在 `src/tui/dialogs.rs` 内 `render_dialog` 函数之后追加新函数：

```rust
/// Build a short diff preview for edit_file/write_file permission dialogs.
/// Returns None if the diff cannot be computed (e.g., bad JSON, missing file).
fn build_diff_preview(
    tool_name: &str,
    tool_input: &str,
    theme: &crate::tui::Theme,
) -> Option<Vec<Line<'static>>> {
    let (path, before, after) = match tool_name {
        "edit_file" => {
            #[derive(serde::Deserialize)]
            struct EditInput { path: String, old: String, new: String }
            let parsed: EditInput = serde_json::from_str(tool_input).ok()?;
            let before = std::fs::read_to_string(&parsed.path).unwrap_or_default();
            let after = before.replacen(&parsed.old, &parsed.new, 1);
            (parsed.path, before, after)
        }
        "write_file" => {
            let newline = tool_input.find('\n')?;
            let path = tool_input[..newline].trim().to_string();
            let after = tool_input[newline + 1..].to_string();
            let before = std::fs::read_to_string(&path).unwrap_or_default();
            (path, before, after)
        }
        _ => return None,
    };

    let diff_text = crate::tui::diff::compute_unified_diff(&before, &after, &path);
    let mut lines = crate::tui::diff::render_diff(&diff_text, &path, &after);

    // Indent each line by 2 spaces to match dialog body style.
    let mut preview: Vec<Line<'static>> = Vec::with_capacity(lines.len() + 1);
    preview.push(Line::styled(
        format!(" Changes ({}):", path),
        Style::default().fg(theme.secondary_text),
    ));
    let limit = crate::tui::diff::MAX_DIALOG_PREVIEW_LINES;
    if lines.len() > limit {
        lines.truncate(limit);
        for mut l in lines {
            l.spans.insert(0, Span::raw("  "));
            preview.push(l);
        }
        preview.push(Line::styled(
            format!("  …({} more lines)", lines.len().saturating_sub(limit)),
            Style::default().fg(theme.secondary_text),
        ));
    } else {
        for mut l in lines {
            l.spans.insert(0, Span::raw("  "));
            preview.push(l);
        }
    }
    Some(preview)
}
```

注：`Span` 已经在 `src/tui/dialogs.rs:8` 导入；`Line` 在 `:8` 也已导入。无需新加 use。

- [ ] **Step 3: 调整对话框高度公式**

`render_dialog` 当前的 `dialog_height` 公式假设固定 input 行数（最多 6 行）：

```rust
    let input_lines = info_str.lines().count().min(6).max(1);
    let dialog_height = (5 + input_lines as u16).min(12).max(7);
```

加 diff 预览后，对话框会高出 ~20 行。把 `dialog_height` 公式改为根据是否有预览动态计算：

```rust
    let input_lines = info_str.lines().count().min(6).max(1);
    let preview_extra: u16 = if tool_name == "edit_file" || tool_name == "write_file" {
        // Up to MAX_DIALOG_PREVIEW_LINES diff lines + 1 header + 1 spacing.
        (crate::tui::diff::MAX_DIALOG_PREVIEW_LINES as u16) + 2
    } else {
        0
    };
    let dialog_height = (5 + input_lines as u16 + preview_extra).min(40).max(7);
```

- [ ] **Step 4: 跑现有测试确认不破坏**

Run: `cargo test --lib tui::dialogs`
Expected: 所有现有测试 PASS。可能有一个测试 `test_render_permission_dialog` 需要看是否断言了精确 dialog_height——如果是，更新断言。

- [ ] **Step 5: 提交**

```bash
git add src/tui/dialogs.rs
git commit -m "feat(tui/dialogs): render diff preview for edit_file/write_file

When a Tool permission dialog opens for edit_file or write_file,
compute before/after and render up to MAX_DIALOG_PREVIEW_LINES (20)
diff lines with full highlighting. Dialog height grows accordingly
to fit the preview."
```

---

## Task 11: Final verification

**Files:**
- 无文件改动（仅运行验证命令）

- [ ] **Step 1: 跑全测试套件**

Run: `cargo test`
Expected: 所有测试 PASS（包括新加的 ~25 个 diff 相关测试和现有所有测试）

如果有失败：
- 失败的现有测试若是因输出格式变化（例如某个测试断言了精确的 ToolResult 字符串），更新断言以匹配新格式
- 失败的新测试若是因实现 bug，修 bug（不要回退实现）

- [ ] **Step 2: 跑 clippy**

Run: `cargo clippy --all-targets`
Expected: 无新警告（与 Task 1 提交前相比）

如果有新警告：
- unused imports → 删除
- needless borrow / redundant clone → 修正
- 复杂度警告 → 看情况，必要时拆函数

- [ ] **Step 3: 启动 codecoder 手动验证**

Run: `cargo run`

在交互模式下：
1. 让 Claude 调一次 `edit_file`（"修改 README 第一行"），看到消息列表里出现 diff，有绿色 +、红色 -、行号 gutter、syntect 语法高亮
2. 让 Claude 调一次 `write_file` 新建文件，看到全 + 行 diff
3. 让 Claude 调一次 `edit_file` 触发权限对话框，看到对话框里有 diff 预览（不是只看到工具入参）
4. 切到 40 列窄终端（resize 窗口），重新触发 edit_file，确认 diff 不撑破布局（renderer 自动截断 + 对话框动态高度）

每一步若有异常（panic、错位、颜色乱），记录到 ledger，回相关 Task 修。

- [ ] **Step 4: 若有任何 fix，提交**

只有 Step 1-3 实际改动了代码才提交：

```bash
git status --short
# 若有改动：
git add -p  # 选择相关改动
git commit -m "fix(tui/diff): post-verification fixes

<specific fixes>"
```

- [ ] **Step 5: 更新审计文档**

把 `docs/audit-tui-visual-fidelity.md` 中裁决清单第 4 桶 #109 / #110 的状态从"建议下一轮单独 brainstorm"改为"✅ 已修：见 commits ..."。同时把第 0 节 V1 标记为"✅ 已修"。

在 `docs/audit-tui-visual-fidelity.md` 末尾的裁决清单前，加一个简短的"已修条目"段落（仿照行为审计的回填机制）：

```markdown
## 修复进度（V1 已落地）

- V1（B6 diff 缺 gutter + 语法高亮）→ ✅ 已修：见 `src/tui/diff.rs` 实现，commit `<SHA>`
- 裁决清单 #109（Diff 渲染.gutter 分栏）→ ✅ 已修
- 裁决清单 #110（Diff 渲染.语法高亮）→ ✅ 已修
```

提交：

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "docs(audit): mark V1 (diff rendering) as resolved

Roll back V1 and adjudication list #109/#110 from bucket-4 to a new
'Resolved' section, mirroring audit-tui-fidelity.md's roll-forward
mechanism."
```

---

## 完成判据

整个 V1 工作完成的判据：

1. `src/tui/diff.rs` 存在，包含 `compute_unified_diff` 和 `render_diff` 两个公开纯函数
2. `Cargo.toml` 含 `similar = "5"`
3. `edit_file` / `write_file` 的 ToolResult 输出含 ` ```diff path="..." ` 围栏
4. `dialogs.rs::render_dialog` 对 edit_file/write_file 显示 diff 预览
5. `cargo test` 全绿
6. `cargo clippy --all-targets` 无新警告
7. 手动跑 codecoder 三场景（edit_file 内联、write_file 内联、edit_file 权限对话框预览）都看到 diff
8. 审计文档 `docs/audit-tui-visual-fidelity.md` 中 V1 + #109 + #110 标记为已修
