# Input Area Multiline (V2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement V2 from `docs/audit-tui-visual-fidelity.md` — give the TUI input area a rounded top border, dynamic height (grows with content, capped at half the terminal), word-wrap with Unicode-aware width, and a placeholder. Replaces the current fixed-2-row, no-border, single-line-rendering input area.

**Architecture:** New `WrapLine` struct and three pure functions (`compute_input_lines`, `compute_input_height`, `find_cursor_position`) in `src/tui/input_area.rs` form the single source of truth for wrap math. `mod.rs::render` calls `compute_input_height` to size the layout dynamically. `input_area::render` uses the same `compute_input_lines` to render content and `find_cursor_position` to place the cursor — guaranteeing no height/render/cursor drift.

**Tech Stack:** Rust 2021; new dep `unicode-width = "0.2"` (East Asian Width for emoji/CJK); existing `ratatui` (Block/BorderType/Rounded, Paragraph); existing `tempfile` (dev-dep).

## Global Constraints

引自 spec `docs/superpowers/specs/2026-07-01-input-multiline-design.md`：

- **依赖**：只加 `unicode-width = "0.2"`，**不加 textwrap**（自包含 wrap 算法保证 byte 映射精确）
- **单一真值源**：`compute_input_lines` 是唯一 wrap 入口；高度计算/渲染/光标定位都基于同一份 `Vec<WrapLine>`
- **WrapLine 结构**：`{ text: String, start_byte: usize, end_byte: usize, display_width: usize }`
- **content_width** = `area.width.saturating_sub(2).max(1)`（减 `> ` 前缀 2 列）
- **高度公式**：`(lines + 2).min(term_height / 2)`，最低 cap 为 3
- **续行对齐**：第 1 行前缀 `> `（accent_text 色），续行 2 空格 `"  "`（同色）
- **placeholder 文本**：`"> Type your message… (Shift+Enter for newline)"`（accent_text + secondary_text）
- **超 cap 滚动**：scroll-to-bottom，`visible = lines[len - content_height..]`
- **顶边框**：`Block::default().borders(Borders::Top).border_type(BorderType::Rounded).border_style(fg(theme.accent_text))`
- **不动**：search/reverse-search 模式的渲染、Slash 补全浮层、鼠标点击定位
- 不引入新 ADR

---

## File Structure

**修改文件：**

| 文件 | 改动 |
|---|---|
| `Cargo.toml` | 加 `unicode-width = "0.2"` |
| `src/tui/input_area.rs` | 新增 `WrapLine` 结构 + `compute_input_lines` + `compute_input_height`（pub）+ `find_cursor_position`（private）；`render` 重构 |
| `src/tui/mod.rs:194-203` | `Layout::vertical` 中段从 `Length(2)` 改为 `Length(compute_input_height(...))` |

**不改：** `src/tui/dialogs.rs`、`src/tui/markdown.rs`、`src/tui/message_list.rs`、其他工具文件。

---

## Task 1: Scaffold + add unicode-width dependency

**Files:**
- Modify: `Cargo.toml`（加 `unicode-width = "0.2"`）
- Modify: `src/tui/input_area.rs`（加 `WrapLine` 结构 + stub helpers）

**Interfaces:**
- Produces: `crate::tui::input_area::WrapLine`、`compute_input_lines`、`compute_input_height` 的 stub（返回空 Vec 和 3），让后续任务可以独立编译

- [ ] **Step 1: 加 `unicode-width` 依赖**

修改 `Cargo.toml` 的 `[dependencies]` 段，加一行：

```toml
unicode-width = "0.2"
```

- [ ] **Step 2: 在 `src/tui/input_area.rs` 顶部加 `WrapLine` 结构 + stub 函数**

在 `use crate::agent::AgentCommand;` 这行（约第 14 行）之后插入：

```rust
/// ─── Multiline Wrap Helpers (V2) ───────────────────────────────────────────
///
/// Self-contained wrap algorithm. Single source of truth: same `Vec<WrapLine>`
/// feeds height calculation, content rendering, and cursor positioning — so
/// they can never drift apart.

/// One visual line after wrapping. Byte ranges map back into the original
/// input (no character splitting, no word-boundary surprises).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrapLine {
    /// The wrapped text (no `\n`).
    pub text: String,
    /// Byte offset in original input where this line starts (inclusive).
    pub start_byte: usize,
    /// Byte offset where this line ends (exclusive).
    pub end_byte: usize,
    /// Display width in terminal columns (emoji/CJK = 2).
    pub display_width: usize,
}

/// Wrap `input` to `width` columns (reserving 2 for `> ` prefix on line 1).
/// Returns one `WrapLine` per visual line. Empty input yields a single empty
/// `WrapLine`. Tasks 2 fills this in.
pub fn compute_input_lines(_input: &str, _width: u16) -> Vec<WrapLine> {
    Vec::new()
}

/// Dynamic input-area height: `lines + 2` (border + padding), capped at
/// `term_height / 2`, minimum 3. Task 3 fills this in.
pub fn compute_input_height(_input: &str, _term_height: u16, _width: u16) -> u16 {
    3
}
```

- [ ] **Step 3: 验证编译**

Run: `cargo build`
Expected: 编译通过。可能有 unused-import 警告——Tasks 2-5 用到这些 import 时不警告。

- [ ] **Step 4: 提交**

```bash
git add Cargo.toml Cargo.lock src/tui/input_area.rs
git commit -m "feat(tui/input_area): scaffold WrapLine + helper stubs for V2

Adds unicode-width = 0.2 dependency. Adds WrapLine struct
(text/start_byte/end_byte/display_width) and stub compute_input_lines /
compute_input_height returning empty/3. Full implementations land in
Tasks 2 and 3; render refactor in Task 5; mod.rs wiring in Task 6."
```

---

## Task 2: Implement compute_input_lines (TDD)

**Files:**
- Modify: `src/tui/input_area.rs`

**Interfaces:**
- Produces: `crate::tui::input_area::compute_input_lines(input: &str, width: u16) -> Vec<WrapLine>`

- [ ] **Step 1: 写失败测试**

在 `src/tui/input_area.rs` 末尾的 `#[cfg(test)] mod tests` 模块内（如果不存在则在文件末尾新建）追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_lines_empty() {
        let lines = compute_input_lines("", 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "");
        assert_eq!(lines[0].start_byte, 0);
        assert_eq!(lines[0].end_byte, 0);
        assert_eq!(lines[0].display_width, 0);
    }

    #[test]
    fn test_compute_lines_single_short() {
        let lines = compute_input_lines("hello", 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "hello");
        assert_eq!(lines[0].start_byte, 0);
        assert_eq!(lines[0].end_byte, 5);
        assert_eq!(lines[0].display_width, 5);
    }

    #[test]
    fn test_compute_lines_newline() {
        let lines = compute_input_lines("a\nb", 80);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "a");
        assert_eq!(lines[0].start_byte, 0);
        assert_eq!(lines[0].end_byte, 1);
        assert_eq!(lines[1].text, "b");
        assert_eq!(lines[1].start_byte, 2);
        assert_eq!(lines[1].end_byte, 3);
    }

    #[test]
    fn test_compute_lines_long_wrap() {
        // 50 chars, content width 30 (width 32 minus 2 for `> `).
        let input = "a".repeat(50);
        let lines = compute_input_lines(&input, 32);
        assert!(lines.len() >= 2, "got {} lines", lines.len());
        // Byte ranges should be contiguous and non-overlapping.
        for i in 1..lines.len() {
            assert_eq!(lines[i].start_byte, lines[i - 1].end_byte,
                "gap/overlap at index {i}");
        }
        assert_eq!(lines[0].start_byte, 0);
        assert_eq!(lines.last().unwrap().end_byte, 50);
        // Each non-final line should be at most 30 display width.
        for l in &lines {
            assert!(l.display_width <= 30, "line {:?} exceeds content width", l.text);
        }
    }

    #[test]
    fn test_compute_lines_wide_chars() {
        // Each 🚀 is display-width 2. content width 10 → 5 per line.
        let input = "🚀".repeat(10);
        let lines = compute_input_lines(&input, 12); // width 12 → content 10
        assert_eq!(lines.len(), 2, "got {} lines", lines.len());
        assert_eq!(lines[0].display_width, 10);
        assert_eq!(lines[1].display_width, 10);
    }

    #[test]
    fn test_compute_lines_narrow_width() {
        // Width < 10 should not panic; content_width = max(width-2, 1) = 1.
        let lines = compute_input_lines("abc", 3);
        // Each character on its own line (content width 1).
        assert_eq!(lines.len(), 3);
        for l in &lines {
            assert!(l.display_width <= 1);
        }
    }
}
```

如果文件已存在 `#[cfg(test)] mod tests`，请合并而不是重复声明。

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tui::input_area::tests::test_compute_lines`
Expected: 6 个测试全部 FAIL（stub 返回空 Vec，长度对不上）

- [ ] **Step 3: 实现 `compute_input_lines`**

替换 Task 1 的 stub：

```rust
/// Wrap `input` to `width` columns (reserving 2 for `> ` prefix on line 1).
/// Returns one `WrapLine` per visual line. Empty input yields a single empty
/// `WrapLine`.
pub fn compute_input_lines(input: &str, width: u16) -> Vec<WrapLine> {
    let content_width = width.saturating_sub(2).max(1) as usize;
    let mut out = Vec::new();

    let mut line_start_byte = 0usize;
    let mut line_text = String::new();
    let mut line_width = 0usize;

    for (byte_idx, ch) in input.char_indices() {
        if ch == '\n' {
            // Close current line at the \n boundary
            out.push(WrapLine {
                text: std::mem::take(&mut line_text),
                start_byte: line_start_byte,
                end_byte: byte_idx,
                display_width: line_width,
            });
            line_start_byte = byte_idx + 1; // skip past \n
            line_width = 0;
            continue;
        }
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if line_width + w > content_width && !line_text.is_empty() {
            // Wrap: flush current line, start new at this char
            out.push(WrapLine {
                text: std::mem::take(&mut line_text),
                start_byte: line_start_byte,
                end_byte: byte_idx,
                display_width: line_width,
            });
            line_start_byte = byte_idx;
            line_width = 0;
        }
        line_text.push(ch);
        line_width += w;
    }
    // Flush final line (even if empty — preserves the "input has n+1 lines"
    // invariant for n newlines).
    out.push(WrapLine {
        text: line_text,
        start_byte: line_start_byte,
        end_byte: input.len(),
        display_width: line_width,
    });
    out
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --lib tui::input_area::tests::test_compute_lines`
Expected: 6 个测试全 PASS

- [ ] **Step 5: 提交**

```bash
git add src/tui/input_area.rs
git commit -m "feat(tui/input_area): implement compute_input_lines

Self-contained wrap: walks char_indices, accumulates display width
(unicode-width for emoji/CJK = 2 cols), splits when exceeding
content_width. Tracks start_byte/end_byte per line for O(log n)
cursor positioning in Task 4."
```

---

## Task 3: Implement compute_input_height (TDD)

**Files:**
- Modify: `src/tui/input_area.rs`

**Interfaces:**
- Produces: `crate::tui::input_area::compute_input_height(input: &str, term_height: u16, width: u16) -> u16`

- [ ] **Step 1: 写失败测试**

在 `src/tui/input_area.rs` 的 `#[cfg(test)] mod tests` 内追加：

```rust
#[test]
fn test_compute_height_empty() {
    // Empty input: 1 line + 2 = 3
    assert_eq!(compute_input_height("", 20, 80), 3);
}

#[test]
fn test_compute_height_single_line() {
    // 1 line + 2 = 3
    assert_eq!(compute_input_height("hello", 20, 80), 3);
}

#[test]
fn test_compute_height_multiline() {
    // 3 lines + 2 = 5
    assert_eq!(compute_input_height("a\nb\nc", 20, 80), 5);
}

#[test]
fn test_compute_height_capped_at_half_term() {
    // 50 chars in width 32 → 2 wrap lines per logical line × 1 logical line = 2.
    // But if we make input huge: 50 lines × 5 chars each = 50 lines + 2 = 52,
    // term_height=10 → cap=5.
    let input: String = (0..50).map(|_| "x\n").collect();
    let h = compute_input_height(&input, 10, 80);
    assert_eq!(h, 5, "expected cap at term_height/2 = 5");
}

#[test]
fn test_compute_height_min_cap_when_term_tiny() {
    // term_height = 4 → cap = max(2, 3) = 3
    assert_eq!(compute_input_height("a\nb\nc\nd\ne", 4, 80), 3);
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tui::input_area::tests::test_compute_height`
Expected: 至少 3 个 FAIL（stub 总是返回 3，但 multiline 测试期望 5）

- [ ] **Step 3: 实现 `compute_input_height`**

替换 Task 1 的 stub：

```rust
/// Dynamic input-area height: `lines + 2` (top border + bottom padding),
/// capped at `term_height / 2`, minimum 3.
pub fn compute_input_height(input: &str, term_height: u16, width: u16) -> u16 {
    let lines = compute_input_lines(input, width).len() as u16;
    let cap = (term_height / 2).max(3);
    (lines + 2).min(cap)
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --lib tui::input_area::tests::test_compute_height`
Expected: 5 个测试全 PASS

- [ ] **Step 5: 提交**

```bash
git add src/tui/input_area.rs
git commit -m "feat(tui/input_area): implement compute_input_height

(lines + 2).min(term_height / 2).max(3) — top border + padding,
half-terminal cap, tiny-terminal floor."
```

---

## Task 4: Implement find_cursor_position (TDD)

**Files:**
- Modify: `src/tui/input_area.rs`

**Interfaces:**
- Produces (private): `fn find_cursor_position(input: &str, wrap_lines: &[WrapLine], cursor_pos: usize) -> (usize, usize)` — returns `(virtual_line_index, col_in_line_by_display_width)`

- [ ] **Step 1: 写失败测试**

在 `src/tui/input_area.rs` 的 `#[cfg(test)] mod tests` 内追加：

```rust
#[test]
fn test_cursor_start_of_input() {
    let lines = compute_input_lines("hello", 80);
    let (vline, col) = find_cursor_position("hello", &lines, 0);
    assert_eq!(vline, 0);
    assert_eq!(col, 0);
}

#[test]
fn test_cursor_end_of_short_line() {
    let lines = compute_input_lines("hello", 80);
    let (vline, col) = find_cursor_position("hello", &lines, 5);
    assert_eq!(vline, 0);
    assert_eq!(col, 5);
}

#[test]
fn test_cursor_across_newline() {
    // Input "a\nb", cursor at byte 2 (just past \n, before 'b')
    let lines = compute_input_lines("a\nb", 80);
    let (vline, col) = find_cursor_position("a\nb", &lines, 2);
    assert_eq!(vline, 1);
    assert_eq!(col, 0);
}

#[test]
fn test_cursor_within_wrap() {
    // 50 chars in width 32 → content_width 30 → 2 lines: [0..30) and [30..50)
    let input = "a".repeat(50);
    let lines = compute_input_lines(&input, 32);
    assert!(lines.len() >= 2, "expected wrap, got {} lines", lines.len());
    // Cursor at byte 35 → in second wrap line, col = 35 - 30 = 5
    let (vline, col) = find_cursor_position(&input, &lines, 35);
    assert_eq!(vline, 1);
    assert_eq!(col, 5);
}

#[test]
fn test_cursor_at_emoji() {
    // "🚀x" — 🚀 is 4 bytes (display 2), x is 1 byte (display 1).
    // Cursor at byte 5 (after x) → col = 2 + 1 = 3
    let input = "🚀x";
    let lines = compute_input_lines(input, 80);
    let (vline, col) = find_cursor_position(input, &lines, 5);
    assert_eq!(vline, 0);
    assert_eq!(col, 3);
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --lib tui::input_area::tests::test_cursor`
Expected: 编译失败（`find_cursor_position` 未定义）

- [ ] **Step 3: 实现 `find_cursor_position`**

在 `compute_input_height` 函数之后追加：

```rust
/// Find cursor's (virtual_line_index, col_in_line) given wrap_lines.
///
/// Binary-searches wrap_lines for the one containing `cursor_pos`, then
/// sums display widths from that line's start_byte to cursor_pos.
/// Returns `(0, 0)` for empty input.
fn find_cursor_position(
    input: &str,
    wrap_lines: &[WrapLine],
    cursor_pos: usize,
) -> (usize, usize) {
    if wrap_lines.is_empty() {
        return (0, 0);
    }
    let cursor_byte = cursor_pos.min(input.len());

    // partition_point returns the first index where the predicate is false.
    // Predicate `wl.end_byte <= cursor_byte` is monotonic (end_bytes grow).
    // - If cursor_byte < wl[0].end_byte → returns 0 (cursor in first line)
    // - If wl[i-1].end_byte <= cursor_byte < wl[i].end_byte → returns i
    // - If cursor_byte >= wl[last].end_byte → returns len (cap to len-1)
    let idx = wrap_lines
        .partition_point(|wl| wl.end_byte <= cursor_byte)
        .min(wrap_lines.len() - 1);

    let line = &wrap_lines[idx];
    let start = line.start_byte.min(input.len());
    let end = cursor_byte.max(start).min(input.len());
    let col: usize = input[start..end]
        .chars()
        .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
        .sum();
    (idx, col)
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --lib tui::input_area::tests::test_cursor`
Expected: 5 个测试全 PASS

如果某测试失败（例如 `test_cursor_within_wrap` 算出错的 vline），调试 binary search 边界条件。常见的 bug 是 `cursor_byte < wl.end_byte` 在 cursor 正好在边界上时选错行——这时应该选**下一行**（cursor 在新行的 col 0）。

- [ ] **Step 5: 提交**

```bash
git add src/tui/input_area.rs
git commit -m "feat(tui/input_area): implement find_cursor_position

Binary-searches wrap_lines for the line containing cursor_byte, then
sums display widths from line.start_byte to cursor. O(log n) per
cursor placement; same Vec<WrapLine> as render guarantees no drift."
```

---

## Task 5: Refactor render function

**Files:**
- Modify: `src/tui/input_area.rs:35-92`（整个 `render` 函数）

**Interfaces:**
- Consumes: Task 2/3/4 的 `compute_input_lines`、`compute_input_height`、`find_cursor_position`
- Produces: 新的 `render` 行为（round border + multiline + placeholder + scroll-to-bottom）

- [ ] **Step 1: 替换 `render` 函数**

把 `src/tui/input_area.rs:35-92` 的整个 `pub fn render(...)` 函数体替换为：

```rust
/// Render the input area: rounded top border + multiline content (with
/// `> ` prefix on line 1, 2-space indent on continuation lines) +
/// placeholder when empty + scroll-to-bottom when overflowing.
///
/// Dynamic height is computed in `mod.rs::render` via `compute_input_height`
/// and used to size the Layout — this function just renders into the given
/// `area`, which is already correctly sized.
pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp, frame_count: u64) {
    let _ = frame_count; // reserved for future cursor blink animation

    let block = Block::default()
        .borders(Borders::Top)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.accent_text));

    if app.input.is_empty() {
        // Placeholder
        let placeholder = Line::from(vec![
            Span::styled("> ", Style::default().fg(app.theme.accent_text)),
            Span::styled(
                "Type your message… (Shift+Enter for newline)",
                Style::default().fg(app.theme.secondary_text),
            ),
        ]);
        frame.render_widget(Paragraph::new(placeholder).block(block), area);
    } else {
        // Multiline content
        let lines = compute_input_lines(&app.input, area.width);
        let ratatui_lines: Vec<Line> = lines
            .iter()
            .enumerate()
            .map(|(i, wl)| {
                let prefix: &str = if i == 0 { "> " } else { "  " };
                Line::from(vec![
                    Span::styled(prefix, Style::default().fg(app.theme.accent_text)),
                    Span::styled(
                        wl.text.clone(),
                        Style::default().fg(app.theme.primary_text),
                    ),
                ])
            })
            .collect();

        // content_height = area.height minus top border (1)
        let content_height = (area.height.saturating_sub(1)) as usize;
        let visible: Vec<Line> = if ratatui_lines.len() > content_height {
            // Scroll to bottom
            ratatui_lines[ratatui_lines.len() - content_height..].to_vec()
        } else {
            ratatui_lines
        };
        frame.render_widget(Paragraph::new(visible).block(block), area);
    }

    // Cursor position (only meaningful when input is non-empty)
    if !app.input.is_empty() {
        let cursor_pos = app.cursor_pos.min(app.input.len());
        let lines = compute_input_lines(&app.input, area.width);
        let (vline, col) = find_cursor_position(&app.input, &lines, cursor_pos);
        // y offset: +1 to skip top border, +vline for virtual line within content
        // x offset: +2 to skip `> ` or `  ` prefix, +col for cursor within line
        let y = area.y
            .saturating_add(1)
            .saturating_add(vline as u16);
        // If cursor is scrolled off (vline beyond content_height), don't draw it
        let content_height = area.height.saturating_sub(1) as u16;
        let scroll_offset = if lines.len() as u16 > content_height {
            lines.len() as u16 - content_height
        } else {
            0
        };
        let visible_vline = vline as u16;
        if visible_vline >= scroll_offset {
            frame.set_cursor_position(ratatui::layout::Position {
                x: area.x.saturating_add(2).saturating_add(col as u16),
                y: y.saturating_sub(scroll_offset),
            });
        }
        // If cursor is in the scrolled-off region, skip set_cursor_position
        // (cursor simply isn't drawn that frame).
    } else {
        // Empty input: cursor right after "> "
        frame.set_cursor_position(ratatui::layout::Position {
            x: area.x.saturating_add(2),
            y: area.y.saturating_add(1),
        });
    }
}
```

注意：
- 顶边框由 `Block::borders(Borders::Top)` 提供，**不**手画 `─` 重复线
- Paragraph 渲染时 `.block(block)` 让 ratatui 自动在 border 下留 padding
- 滚动到底：`visible = lines[len - content_height..]`，并相应调整光标 y 偏移减去 scroll_offset

- [ ] **Step 2: 加必要的 imports**

确保 `src/tui/input_area.rs` 顶部 use 含：

```rust
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
```

`Block`、`BorderType`、`Borders` 可能是新加的（旧代码只用了 `Paragraph`）。

- [ ] **Step 3: 跑现有测试确认不破坏**

Run: `cargo test --lib tui::input_area`
Expected: 所有现有测试 PASS（旧的 render 测试如果有，可能需要更新断言；新代码渲染逻辑变化大，旧测试可能失败——见 Step 4）

- [ ] **Step 4: 处理失败测试**

如果有现有 render 相关测试失败：
- 若失败原因是"渲染输出格式变化"（例如断言 `─` 重复行存在）→ 更新断言以匹配新输出（round border）
- 若失败原因是逻辑 bug → 修 bug

具体可能失败的测试：
- 任何调用 `render(...)` 并检查 buffer 的测试
- 任何检查 separator line 的测试

如果有 `test_render_*` 测试存在且失败，**更新它们的断言**——不要回退实现。

- [ ] **Step 5: 提交**

```bash
git add src/tui/input_area.rs
git commit -m "feat(tui/input_area): refactor render for multiline + border

Replaces fixed-2-row layout with: rounded top border via
Block::borders(TOP), pre-wrapped Vec<WrapLine> rendering with
\`> \` prefix on line 1 and 2-space indent on continuations,
placeholder for empty input, scroll-to-bottom when content
overflows the available height."
```

---

## Task 6: Wire dynamic height into mod.rs Layout

**Files:**
- Modify: `src/tui/mod.rs:194-203`（`Layout::vertical` 调用）

**Interfaces:**
- Consumes: `crate::tui::input_area::compute_input_height`

- [ ] **Step 1: 改 Layout 中段约束**

把 `src/tui/mod.rs:194-203` 的：

```rust
let [msg_area, input_area_rect, status_area] = ratatui::layout::Layout::new(
    ratatui::layout::Direction::Vertical,
    [
        ratatui::layout::Constraint::Min(1),
        ratatui::layout::Constraint::Length(2),
        ratatui::layout::Constraint::Length(1),
    ],
)
.flex(Flex::Start)
.areas(area);
```

改为：

```rust
// V2: input area height is dynamic — grows with content, capped at
// half the terminal height. compute_input_height is pure (no I/O).
let input_height = crate::tui::input_area::compute_input_height(
    &app.input,
    area.height,
    area.width,
);

let [msg_area, input_area_rect, status_area] = ratatui::layout::Layout::new(
    ratatui::layout::Direction::Vertical,
    [
        ratatui::layout::Constraint::Min(1),
        ratatui::layout::Constraint::Length(input_height),
        ratatui::layout::Constraint::Length(1),
    ],
)
.flex(Flex::Start)
.areas(area);
```

- [ ] **Step 2: 验证编译**

Run: `cargo build`
Expected: 编译通过

- [ ] **Step 3: 跑全测试套件确认无回归**

Run: `cargo test`
Expected: 所有现有测试 PASS（render 输出可能变化导致少量测试失败——更新断言）

- [ ] **Step 4: 处理失败测试（如有）**

若有 `test_render_*` 类测试失败：
- 检查失败原因：是 input 高度从 2 变成动态值导致的 buffer 大小变化？
- 更新测试 setup：传入更大的 `area`（例如 `Rect::new(0, 0, 80, 24)` 而不是 `Rect::new(0, 0, 80, 3)`）
- 或更新断言：匹配新的输出位置（多了 1 行 round border）

- [ ] **Step 5: 提交**

```bash
git add src/tui/mod.rs
git commit -m "feat(tui/mod): use dynamic input height via compute_input_height

Layout::vertical middle constraint changes from fixed Length(2) to
Length(compute_input_height(...)). Message area shrinks as input
grows; input capped at term_height/2."
```

---

## Task 7: Add smoke tests (TestBackend)

**Files:**
- Modify: `src/tui/input_area.rs`（测试模块）

**Interfaces:**
- Consumes: Task 5 的 render、Task 2 的 compute_input_lines

- [ ] **Step 1: 写 smoke tests**

在 `src/tui/input_area.rs` 的 `#[cfg(test)] mod tests` 内追加：

```rust
#[test]
fn test_render_empty_shows_placeholder() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::default();
    app.input = String::new();
    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 40, 5);
            render(f, area, &app, 0);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    // The placeholder text starts with "T" (after "> ").
    // Find any cell containing 'T' as the placeholder marker.
    let has_t = buffer.content().iter().any(|c| c.symbol() == "T");
    assert!(has_t, "expected placeholder 'T' in buffer");
}

#[test]
fn test_render_shows_top_border() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::default();
    app.input = "hello".to_string();
    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 40, 5);
            render(f, area, &app, 0);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    // Top row should contain round-border characters.
    // ratatui BorderType::Rounded uses '╭' and '╮' for corners, '─' for line.
    let top_row: String = (0..40)
        .map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()).unwrap_or_default())
        .collect();
    assert!(
        top_row.contains('─') || top_row.contains('╭') || top_row.contains('╮'),
        "expected round border in top row, got: {top_row}"
    );
}

#[test]
fn test_render_multiline_shows_all_lines() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::default();
    app.input = "a\nb".to_string();
    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 40, 5);
            render(f, area, &app, 0);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    // Row 1 (index 1, after border) should contain "> a"
    let row1: String = (0..10)
        .map(|x| buffer.cell((x, 1)).map(|c| c.symbol().to_string()).unwrap_or_default())
        .collect();
    assert!(row1.starts_with("> a"), "row 1: {row1:?}");
    // Row 2 (index 2) should contain "  b" (2 spaces + b for continuation)
    let row2: String = (0..10)
        .map(|x| buffer.cell((x, 2)).map(|c| c.symbol().to_string()).unwrap_or_default())
        .collect();
    assert!(row2.starts_with("  b"), "row 2: {row2:?}");
}
```

如果 `TuiApp::default()` 不可用（私有字段），改用现有的 test helper（grep `TuiApp::test_app` 或类似）。如果没有 helper，参考 input_area.rs 现有测试看怎么构造 TuiApp。

- [ ] **Step 2: 跑测试**

Run: `cargo test --lib tui::input_area::tests::test_render`
Expected: 3 个测试全 PASS

如果 `test_render_shows_top_border` 失败（buffer 第一行没有 round border 字符），检查：
- `Borders::Top` 是否真的画了顶部
- `BorderType::Rounded` 是否生效（如果终端不支持 Unicode，可能回落到 ASCII——但 TestBackend 应该用 Unicode）

- [ ] **Step 3: 提交**

```bash
git add src/tui/input_area.rs
git commit -m "test(tui/input_area): add 3 smoke tests for render

TestBackend-based: empty input shows placeholder; non-empty shows
round top border; multiline input 'a\nb' renders '> a' on row 1
and '  b' on row 2."
```

---

## Task 8: Final verification + audit roll-forward

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（标记 V2 已修）

- [ ] **Step 1: 跑全测试套件**

Run: `cargo test`
Expected: 所有测试 PASS（V2 新加的 14 个 + 现有所有）

如果失败：
- 因输出格式变化的现有测试 → 更新断言
- 因实现 bug 的新测试 → 修 bug

- [ ] **Step 2: 跑 clippy**

Run: `cargo clippy --all-targets`
Expected: 在 `src/tui/input_area.rs`、`src/tui/mod.rs` 内**无新警告**（与 V1 完成时的 baseline 相比）

如果有新警告：
- unused imports → 删除
- 复杂度警告 → 看情况

- [ ] **Step 3: 手动验证（sandbox 跑不了交互 TUI 则跳过）**

Run: `cargo run`

逐项验证：
1. 启动 codecoder，输入 1 行短文本 → 看到 round 顶边、`> ` 前缀、光标位置正确
2. Shift+Enter 插入换行 → 输入区高度增加，多行渲染正确
3. 输入超长 1 行（>30 字符）→ 自动 wrap 到第 2 行，续行 2 空格对齐
4. 输入到 20+ 行 → 高度封顶，scroll 到底部
5. 输入空 → placeholder 显示
6. 输入中文/emoji → wrap 正确，光标列偏移正确
7. 终端 resize 到 40 列 → 输入区高度变化，wrap 重新生效

如果跑不了交互 TUI：在 report 里标 deferred，继续 Step 4。

- [ ] **Step 4: 更新审计文档**

把 `docs/audit-tui-visual-fidelity.md` 的：
- 裁决清单第 4 桶 #102（输入区.多行渲染）→ 标记 "✅ 已修"
- Section 0 V2 行的"待修" → 改为 "✅ 已修"
- 在"## 修复进度（V1 已落地）"段后追加 V2 段：

```markdown
## 修复进度（V2 已落地）

- V2（A3 输入区缺 round 边框 + 多行渲染）→ ✅ 已修：见 `src/tui/input_area.rs` 重构，commit `<SHA>`
- 裁决清单 #102（输入区.多行渲染）→ ✅ 已修
```

把 `<SHA>` 替换为 Task 7 commit 的 SHA。

- [ ] **Step 5: 提交审计回填**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "docs(audit): mark V2 (input area multiline) as resolved

Rolls forward V2 and adjudication list #102: input area now has
dynamic height, rounded top border, word-wrap, and placeholder.
Single-source-of-truth WrapLine struct guarantees height/render/
cursor never drift."
```

- [ ] **Step 6: 若 Step 1-3 有 fix，单独提交**

只有实际改动了代码才提交：

```bash
git status --short
# 若有改动：
git add -p
git commit -m "fix(tui/input_area): post-verification fixes

<specific fixes>"
```

---

## 完成判据

V2 工作完成的判据：

1. `src/tui/input_area.rs` 含 `WrapLine` 结构 + `compute_input_lines` + `compute_input_height`（公开）+ `find_cursor_position`（私有）
2. `Cargo.toml` 含 `unicode-width = "0.2"`
3. `src/tui/mod.rs::render` 用 `compute_input_height` 算 Layout 中段高度
4. `render` 函数使用 round 顶边、placeholder、scroll-to-bottom、新光标算法
5. `cargo test` 全绿（含新加的 14 个测试）
6. `cargo clippy --all-targets` 在 input_area.rs / mod.rs 内无新警告
7. 手动跑 codecoder 三场景（短输入、Shift+Enter 多行、超长 wrap）都正确
8. 审计文档 `docs/audit-tui-visual-fidelity.md` 中 V2 + #102 标记为已修
