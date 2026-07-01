# 输入区多行扩展（V2）— 设计规格

**日期**：2026-07-01
**目标**：实现审计裁决清单第 4 桶 V2（条目 #102）——为 TUI 输入区加入 round 顶边框、动态高度（随内容增长，封顶屏 50%）、自动 word-wrap、placeholder。当前 codecoder 输入区是固定 2 行、单行渲染、无边框。
**关联文档**：`docs/audit-tui-visual-fidelity.md` §1.A3、裁决清单第 4 桶 #102；本 spec 平行于 V1 spec（`docs/superpowers/specs/2026-07-01-diff-rendering-pipeline-design.md`）。

---

## 1. 范围

### 1.1 本轮交付

- 新依赖 `unicode-width = "0.2"`（用于按 East Asian Width 算显示宽度）
- 新结构 `WrapLine { text, start_byte, end_byte, display_width }`（`src/tui/input_area.rs` 内）
- 新 helper（公开）：
  - `compute_input_lines(input: &str, width: u16) -> Vec<WrapLine>` — 自包含 wrap（不走 textwrap），byte 映射精确
  - `compute_input_height(input: &str, term_height: u16, width: u16) -> u16` — 动态高度，封顶 `term_height / 2`
- 新 helper（private）：`find_cursor_position(input, wrap_lines, cursor_pos) -> (virtual_line, col)` — 在 `WrapLine` 列表上 binary search
- `src/tui/mod.rs::render` 布局改为 `Layout::vertical([Min(1), Length(compute_input_height(...)), Length(1)])`
- `src/tui/input_area.rs::render` 重构：
  - 顶边框 `Block::borders(TOP).border_type(Rounded).border_style(fg(accent_text))`
  - 内容用预 wrap 的 `Vec<WrapLine>` 渲染 `.text`，第一行带 `> ` 前缀、续行 2 空格对齐
  - 空输入显示 placeholder `"> Type your message… (Shift+Enter for newline)"`（dim 色）
  - 超过 cap 时滚动到底（`visible_lines = lines[len - content_height..]`）
- 完整单元测试 + 冒烟测试

### 1.2 明确不在本轮

- 历史/搜索/反向搜索模式的渲染（不改）
- Slash 补全浮层的位置调整（依赖 input_area 高度，但浮层用绝对坐标，会自然适应——本轮不专门测试）
- 软/硬换行视觉指示（不区分 `\n` 类型）
- 输入字符数/token 计数显示（原版 footer 有，下一轮）
- 鼠标点击定位光标（原版有 MeasuredText reverse mapping，ratatui 鼠标支持弱，本轮不做）
- Search 模式的多行 wrap（search prefix 长度变化，本轮不专门处理）

---

## 2. 架构

### 2.1 新增依赖

`Cargo.toml`：

```toml
unicode-width = "0.2"
```

`unicode-width` 选定理由：按 East Asian Width 算字符显示宽度（emoji/中文 = 2 列），用于 wrap 宽度计算和光标列偏移。手写 wrap 算法（~30 行）替代 textwrap，保证 byte 映射精确、cursor 定位无误。

### 2.2 修改文件清单

| 文件 | 改动 |
|---|---|
| `Cargo.toml` | 加 `textwrap` + `unicode-width` |
| `src/tui/input_area.rs` | 新增 `compute_input_lines`、`compute_input_height`（pub）、`find_cursor_position`（private）；`render` 重构（border + wrap + placeholder + 新光标算法） |
| `src/tui/mod.rs:194-203` | 布局 `Layout::vertical` 中段从 `Length(2)` 改为 `Length(compute_input_height(&app.input, area.height, area.width))` |

### 2.3 核心数据流

```
mod.rs::render
    │
    ├─ let input_h = compute_input_height(&app.input, area.height, area.width)
    │    └─ compute_input_lines(...) → textwrap::wrap(segment, content_width) per \n segment
    │    └─ lines.len() + 2, capped at term_height / 2 (min 3)
    │
    ├─ Layout::vertical([Min(1), Length(input_h), Length(1)])
    │
    └─ input_area::render(frame, input_area_rect, app, frame_count)
         │
         ├─ block = Block::default().borders(TOP).border_type(Rounded)
         │         .border_style(Style::fg(theme.accent_text))
         │
         ├─ lines = compute_input_lines(&app.input, area.width)  ← 同一份 wrap 结果
         │
         ├─ if app.input.is_empty():
         │     render placeholder "> Type your message… (Shift+Enter for newline)"
         │  else:
         │     let ratatui_lines = lines.iter().enumerate().map(|(i, l)| {
         │         let prefix = if i == 0 { "> " } else { "  " };  // 2 空格对齐
         │         Line::from([Span::styled(prefix, accent), Span::styled(l, primary)])
         │     });
         │     visible = scroll_to_bottom(ratatui_lines, content_height)
         │     render Paragraph::new(visible).block(block)
         │
         └─ let (vline, col) = find_cursor_position(&app.input, app.cursor_pos, &lines)
            frame.set_cursor_position(x = area.x + 2 + col, y = area.y + 1 + vline)
```

### 2.4 关键设计原则：单一真值源

`compute_input_lines` 是**唯一**的 wrap 算法入口。三个消费者（高度计算、内容渲染、光标定位）都基于同一份 `Vec<String>` 输出。这意味着：
- 永远不会出现"高度算 5 行但实际渲染 6 行"的错位
- 光标 y-offset 一定落在 wrap 后的虚拟行里
- 不依赖 ratatui 内部的 wrap（其算法对外不可见）

---

## 3. 关键算法

### 3.0 单一真值源：`WrapLine` 三元组

为避免 textwrap 的 word-boundary wrap 与光标遍历不一致，本设计采用**自包含 wrap 算法**——不依赖 textwrap 的内部 wrap，而是手写一个返回 `(text, start_byte, end_byte, display_width)` 的 wrap 函数。textwrap 仅作为**字符宽度查询**的间接依赖（通过 `unicode-width` 直接调用）。

> 决定：移除 `textwrap` 依赖，只保留 `unicode-width`。手写 wrap 函数 ~30 行，但保证 byte 映射精确。

### 3.1 `WrapLine` 结构 + `compute_input_lines`

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrapLine {
    pub text: String,        // wrap 后的文本（不含 \n）
    pub start_byte: usize,   // 在原 input 中的起始字节偏移
    pub end_byte: usize,     // 结束字节偏移（exclusive）
    pub display_width: usize, // 显示列数（emoji 算 2）
}

pub fn compute_input_lines(input: &str, width: u16) -> Vec<WrapLine> {
    let content_width = width.saturating_sub(2).max(1) as usize;
    let mut out = Vec::new();

    for (seg_start, segment) in input.split('\n').enumerate() {
        // seg_start_byte 在循环里推算（split 不给偏移，手动算）
        // —— 简化：直接遍历 input.char_indices() 推算
        // （实现细节，下面 3.1.1 给出完整代码）
    }
    out
}
```

#### 3.1.1 完整实现

```rust
pub fn compute_input_lines(input: &str, width: u16) -> Vec<WrapLine> {
    let content_width = width.saturating_sub(2).max(1) as usize;
    let mut out = Vec::new();

    let mut line_start_byte = 0usize;
    let mut line_text = String::new();
    let mut line_width = 0usize;

    for (byte_idx, ch) in input.char_indices() {
        if ch == '\n' {
            // Close current line
            out.push(WrapLine {
                text: std::mem::take(&mut line_text),
                start_byte: line_start_byte,
                end_byte: byte_idx,
                display_width: line_width,
            });
            line_start_byte = byte_idx + 1; // past \n
            line_width = 0;
            continue;
        }
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if line_width + w > content_width && !line_text.is_empty() {
            // Wrap: flush current line, start new
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
    // Flush final line (even if empty)
    out.push(WrapLine {
        text: line_text,
        start_byte: line_start_byte,
        end_byte: input.len(),
        display_width: line_width,
    });
    out
}
```

要点：
- 自包含：不依赖 textwrap，byte 映射精确
- 字符宽度走 `unicode-width`（emoji/中文按 East Asian Width 算 2 列）
- 空输入 → 输出单个空 `WrapLine`
- `split('\n')` 不再用——遍历 `char_indices`，遇到 `\n` 即关闭当前行

### 3.2 `compute_input_height(input, term_height, width) -> u16`

```rust
pub fn compute_input_height(input: &str, term_height: u16, width: u16) -> u16 {
    let lines = compute_input_lines(input, width).len() as u16;
    let cap = (term_height / 2).max(3);
    (lines + 2).min(cap)
}
```

- `+ 2` = 顶边框（1）+ 底部留白（1）
- `cap = term_height / 2`，最低 3 行（防止极小终端退化）

### 3.3 `find_cursor_position`（private）

```rust
fn find_cursor_position(
    wrap_lines: &[WrapLine],
    cursor_pos: usize,
) -> (usize, usize) {
    // Returns (virtual_line_index, col_in_that_line_by_display_width)
    let cursor_byte = cursor_pos.min(input_len_placeholder(wrap_lines));

    // Binary search for the wrap line containing cursor_byte
    let idx = wrap_lines.partition_point(|wl| wl.end_byte <= cursor_byte);
    let idx = idx.min(wrap_lines.len().saturating_sub(1));

    let line = &wrap_lines[idx];
    // col = display width from line.start_byte to cursor_byte
    let prefix = &input_slice_from(wrap_lines, line.start_byte, cursor_byte);
    let col = prefix
        .chars()
        .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
        .sum();
    (idx, col)
}
```

要点：
- **Binary search on `end_byte`**：直接定位 cursor 所在的 wrap line，O(log n)
- col 通过遍历 `start_byte..cursor_byte` 的字符累加显示宽度
- 不再需要遍历整个 input

> 注：`input_len_placeholder` 和 `input_slice_from` 是辅助——`find_cursor_position` 实际签名要带 `input: &str` 才能切片：

```rust
fn find_cursor_position(
    input: &str,
    wrap_lines: &[WrapLine],
    cursor_pos: usize,
) -> (usize, usize) {
    let cursor_byte = cursor_pos.min(input.len());
    let idx = wrap_lines.partition_point(|wl| wl.end_byte <= cursor_byte);
    let idx = idx.min(wrap_lines.len().saturating_sub(1));
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

### 3.4 渲染（伪代码）

```rust
pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp, frame_count: u64) {
    let _ = frame_count;

    let block = Block::default()
        .borders(Borders::Top)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.accent_text));

    if app.input.is_empty() {
        let placeholder = Line::from(vec![
            Span::styled("> ", Style::default().fg(app.theme.accent_text)),
            Span::styled(
                "Type your message… (Shift+Enter for newline)",
                Style::default().fg(app.theme.secondary_text),
            ),
        ]);
        frame.render_widget(Paragraph::new(placeholder).block(block), area);
    } else {
        let lines = compute_input_lines(&app.input, area.width);
        let ratatui_lines: Vec<Line> = lines.iter().enumerate().map(|(i, l)| {
            let prefix: String = if i == 0 { "> ".to_string() } else { "  ".to_string() };
            Line::from(vec![
                Span::styled(prefix, Style::default().fg(app.theme.accent_text)),
                Span::styled(l.clone(), Style::default().fg(app.theme.primary_text)),
            ])
        }).collect();

        let content_height = (area.height.saturating_sub(1)) as usize; // -1 for top border
        let visible = if ratatui_lines.len() > content_height {
            ratatui_lines[ratatui_lines.len() - content_height..].to_vec()
        } else {
            ratatui_lines
        };
        frame.render_widget(Paragraph::new(visible).block(block), area);
    }

    // Cursor
    let cursor_pos = app.cursor_pos.min(app.input.len());
    let lines = compute_input_lines(&app.input, area.width);
    let (vline, col) = find_cursor_position(&app.input, cursor_pos, &lines);
    frame.set_cursor_position(ratatui::layout::Position {
        x: area.x + 2 + col as u16,
        y: area.y + 1 + vline as u16,
    });
}
```

- 渲染时不调 `Paragraph::wrap`（已预 wrap）；用默认无 wrap
- 超过 content_height 时手动 scroll-to-bottom（用户始终看到正在打的那行）
- 光标设置在渲染之后

---

## 4. 边界情况与降级

### 4.1 极小终端（width < 10）

- `content_width = max(width - 2, 1)`，至少 1 列
- textwrap 在 1 列下每字符一行——视觉糟糕但不 panic
- 高度 cap `term_height / 2` 最低 3 行

### 4.2 空输入

- `compute_input_lines("", width) = [""]`
- `compute_input_height("", ...) = 3`
- 渲染走 placeholder 分支
- 光标位置：`(0, 0)` → x = area.x + 2, y = area.y + 1

### 4.3 单行非空 / 多行（`\n`）/ 长 wrap

- 各场景按 §3.1 算法处理，渲染时正确显示
- 续行 2 空格对齐

### 4.4 超 cap（输入超过 `term_height / 2`）

- `compute_input_height` 返回 cap
- 渲染走 scroll-to-bottom
- 光标若在被裁掉的虚拟行：用户按任意字符键时光标回到最后一行（可接受降级）

### 4.5 全宽字符（emoji / 中文）

- textwrap 按 East Asian Width 算 `🚀` = 2 列
- wrap 宽度算的是**显示列数**
- `unicode-width` 同样按 East Asian Width 算 cursor col——保证光标在 emoji 后停在正确列

### 4.6 鼠标点击定位

**不在本轮范围**——鼠标点击定位光标需要 MeasuredText reverse mapping，原版有但 codecoder 本轮不做。

### 4.7 历史 / search / 反向搜索模式

- 这些模式下 `app.mode` 切换，`app.input` 仍是当前输入
- 本任务不改这些模式的渲染——它们的 prefix 长度变化（如 `search: `），如果输入很长会被 wrap，但 prefix 不会对齐到续行
- 可接受降级：search 模式下 wrap 不完美，但搜索查询通常很短

### 4.8 Slash 补全浮层

- 浮层渲染在 input_area 上方，用绝对坐标
- input_area 高度变化时浮层位置会自然适应
- **本轮不专门测试**——手动验证阶段确认浮层位置不破

---

## 5. 测试策略

### 5.1 单元测试（`src/tui/input_area.rs` `#[cfg(test)]`）

**`compute_input_lines`（6 个）：**

- `test_compute_lines_empty` — `""` → 1 个空 WrapLine（start_byte=0, end_byte=0）
- `test_compute_lines_single_short` — `"hello"` → 1 个 WrapLine，text="hello"，display_width=5
- `test_compute_lines_newline` — `"a\nb"` → 2 个 WrapLine，byte 范围 [0,1) 和 [2,3)
- `test_compute_lines_long_wrap` — 50 字符 + 宽 30 → ≥2 行，byte 范围连续不重叠
- `test_compute_lines_wide_chars` — `"🚀".repeat(10)` + 内容宽 10 → 2 行，每行 display_width=10
- `test_compute_lines_narrow_width` — 宽 < 10 不 panic，每字符一行

**`compute_input_height`（4 个）：**

- `test_compute_height_empty` — `("", 20, 80)` → 3
- `test_compute_height_single_line` — `("hello", 20, 80)` → 3
- `test_compute_height_multiline` — `("a\nb\nc", 20, 80)` → 5
- `test_compute_height_capped_at_half_term` — 超长 + `term_height = 10` → cap 在 5

**`find_cursor_position`（5 个）：**

- `test_cursor_start_of_input` — cursor=0 → `(0, 0)`
- `test_cursor_end_of_short_line` — `"hello"`, cursor=5 → `(0, 5)`
- `test_cursor_across_newline` — `"a\nb"`, cursor=2 → `(1, 0)`
- `test_cursor_within_wrap` — 50 字符长行 + 宽 30 + cursor 在第 2 行中段 → `(1, col)`
- `test_cursor_at_emoji` — `"🚀x"`, cursor=4 → `(0, 3)`（emoji 占 2 列 + x 占 1 列）

### 5.2 冒烟测试（3 个，用 TestBackend）

- `test_render_empty_shows_placeholder` — 输入空，buffer 含 `T`（placeholder 起始字符）
- `test_render_shows_top_border` — buffer 第一行含 `╭` 或 `╮`（round 边框角标）
- `test_render_multiline_shows_all_lines` — 输入 `"a\nb"`，buffer 含 `> a` 和 `  b`

### 5.3 不做的测试

- ratatui 后端完整快照测试（维护成本过高）
- Layout 集成测试（高度变化靠手动验证）
- search 模式 wrap 测试（4.7 已说明本轮不处理）

### 5.4 手动验证清单（提交前）

- [ ] `cargo test` 全绿
- [ ] `cargo clippy` 无新警告
- [ ] 启动 codecoder，输入 1 行短文本 → 看到 round 顶边、`> ` 前缀、光标位置正确
- [ ] Shift+Enter 插入换行 → 输入区高度增加，多行渲染正确
- [ ] 输入超长 1 行 → 自动 wrap 到第 2 行，续行 2 空格对齐
- [ ] 输入到 20+ 行 → 高度封顶，scroll 到底部，最新输入可见
- [ ] 输入空 → placeholder 显示
- [ ] 输入中文/emoji → wrap 正确，光标列偏移正确
- [ ] 终端 resize 到 40 列 → 输入区高度变化，wrap 重新生效

---

## 6. 后续（非本轮）

本 spec 完成后：

- 若 search 模式 wrap 成痛点，下轮加 search prefix 感知的 wrap 算法
- 若鼠标点击定位成为需求，引入 MeasuredText 风格的 reverse mapping
- 字符数/token 计数 footer 与原版 PromptInputFooter 对齐，作为独立 spec
