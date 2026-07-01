# Diff 渲染管线 — 设计规格

**日期**：2026-07-01
**目标**：实现审计裁决清单第 4 桶 V1（条目 #109 + #110）——为 `edit_file` / `write_file` / `diff` 工具调用产出 unified diff，并在消息列表内联与权限对话框预览中渲染（含行号 gutter、hunk header 格式化、syntect 全文件 context 语法高亮）。
**关联文档**：`docs/audit-tui-visual-fidelity.md` §2.B6、裁决清单第 4 桶 #109/#110；本 spec 平行于 `docs/superpowers/specs/2026-07-01-tui-visual-fidelity-audit-design.md`。

---

## 1. 范围

### 1.1 本轮交付

- 新建 `src/tui/diff.rs` 模块，含两个公开纯函数：
  - `compute_unified_diff(old: &str, new: &str, path: &str) -> String`
  - `render_diff(text: &str, file_path: &str, file_content: &str, theme: &Theme, width: u16) -> Vec<Line<'static>>`
- `edit_file.rs` / `write_file.rs` 在执行成功后调用 `compute_unified_diff`，把结果作为 ` ```diff path="..." ` 围栏代码块附在 ToolResult 输出末尾
- `markdown.rs::render_diff_text` 改为 thin wrapper，转调 `diff::render_diff`，并解析 fence info string 中的 `path="..."`
- 权限对话框（`dialogs.rs`）对 `edit_file` / `write_file` 类型预计算并展示 diff 预览
- 完整单元测试 + 工具集成测试

### 1.2 明确不在本轮

- 全屏 diff 查看器（DiffDialog / Ctrl+O 风格）
- RawAnsi 风格的双列渲染（性能优化，单列已足够）
- WeakMap 风格的渲染缓存（无性能问题不引入）
- Notebook edit / Sed edit 的 diff 预览（独立权限对话框，下一轮）
- Diff 内导航 / 操作（跳转 hunk、revert、打开到 $EDITOR）
- 性能基准测试

---

## 2. 架构

### 2.1 新增依赖

`Cargo.toml`：

```toml
similar = "3"
```

`similar` 选定理由：现代维护活跃、行级 diff API 干净、是 sled / git-tools 等项目的选择。比 `diff` crate 更现代。（最新稳定版 3.1.1。）

### 2.2 新建 `src/tui/diff.rs`

```rust
// 公开 API
pub fn compute_unified_diff(old: &str, new: &str, path: &str) -> String;
pub fn render_diff(
    text: &str,
    file_path: &str,
    file_content: &str,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>>;

// 内部辅助（private）
struct Hunk { new_start: usize, header: String, lines: Vec<ParsedLine> }
enum ParsedLine { Add(String), Del(String), Context(String), FileHeader(String) }

fn parse_hunks(text: &str) -> Vec<Hunk>;
fn compute_gutter_width(hunks: &[Hunk]) -> usize;
fn detect_language(path: &str, first_line: &str) -> Option<&'static SyntaxReference>;
fn precompute_line_highlights(
    content: &str,
    lang: &SyntaxReference,
    theme: &Theme,
) -> Vec<Vec<Span<'static>>>;
```

`src/tui/mod.rs` 增加 `pub mod diff;`。

### 2.3 修改文件清单

| 文件 | 改动 |
|---|---|
| `Cargo.toml` | 加 `similar = "5"` |
| `src/tui/mod.rs` | `pub mod diff;` |
| `src/tui/markdown.rs` | fence info string 解析 `path="..."`；`render_diff_text(text: &str)` 保留为兼容入口，新增 `render_diff_text_with_path(text: &str, path: &str)` 内部函数把 path 透传给 `diff::render_diff`。两个入口都对应同一个 `diff::render_diff` 调用 |
| `src/tools/edit_file.rs` | 成功路径加：计算 before/after → `compute_unified_diff` → 拼接 ` ```diff path="..." ` 块到 ToolResult.output |
| `src/tools/write_file.rs` | 同上（before = 已存在的文件内容或空串） |
| `src/tui/dialogs.rs` | `render_permission_dialog` 对 edit_file/write_file 类型：从入参加载 before、计算 after → 调 `compute_unified_diff` + `render_diff` → 渲染到对话框 body |

### 2.4 接口契约

- `compute_unified_diff` 与 `render_diff` 都是纯函数（无 I/O、无副作用、可重复）
- `markdown.rs::render_diff_text` 保留函数签名，内部转调（向后兼容现有测试）
- edit_file/write_file 的 ToolResult 输出格式：

```
Edited src/foo.rs. Replaced 1 occurrence (10→12 lines, 2 line diff).

```diff path="src/foo.rs"
--- a/src/foo.rs
+++ b/src/foo.rs
@@ -10,3 +10,5 @@
 context
-old
+new line 1
+new line 2
 context
```
```

---

## 3. 数据流

### 3.1 路径 A：edit_file/write_file 工具执行 → 内联展示

```
Claude calls edit_file { path, old, new }
    ↓
edit_file::execute
    ├─ read current file content (already done for old-uniqueness check)
    ├─ verify old is unique
    ├─ compute new_content = current.replace(old, new)
    ├─ write new_content to disk
    ├─ let diff_text = diff::compute_unified_diff(&current, &new_content, &path)
    └─ return ToolResult {
        output: format!("Edited {path}.\n\n```diff path=\"{path}\"\n{diff_text}\n```")
    }
    ↓
MessageList renders ToolCall
    ↓
markdown.rs detects ```diff``` fence, extracts path from info string
    ↓
render_diff_text(text, file_path) → diff::render_diff(text, file_path, file_content?, theme, width)
    （file_content 读盘获取；读不到则降级，见 §5.5）
    ↓
Vec<Line> inserted into message rendering
```

### 3.2 路径 B：`/diff` 工具（git diff）→ 内联展示

```
User runs /diff or Claude calls diff tool
    ↓
diff::execute runs `git diff`, returns unified diff text as ToolResult.output
    ↓
（路径同 A 后半段；fence info string 不含 path，语法高亮降级）
```

### 3.3 路径 C：权限对话框预览

```
Claude calls edit_file { path, old, new }
    ↓
PermissionRequest::EditFile { path, old, new } constructed
    ↓
Dialog renders
    ├─ load before = current file content (read disk)
    ├─ compute after = before.replace(old, new)
    ├─ let diff_text = diff::compute_unified_diff(&before, &after, &path)
    ├─ let lines = diff::render_diff(&diff_text, &path, &after, theme, width)
    └─ render Block:
        ┌────────────────────────────────┐
        │ [!] Tool Permission            │
        │                                │
        │ edit_file: src/foo.rs          │
        │ <diff preview lines, ≤ 20 行>  │
        │ ...                            │
        │                                │
        │ Y=once  A=session  Shift+A=... │
        └────────────────────────────────┘
```

---

## 4. 关键算法

### 4.1 `compute_unified_diff(old, new, path)`

```rust
pub fn compute_unified_diff(old: &str, new: &str, path: &str) -> String {
    use similar::TextDiff;
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();
    output.push_str(&format!("--- a/{path}\n+++ b/{path}\n"));
    let patch = diff.unified_diff().context_radius(3);
    output.push_str(&format!("{patch}"));
    output
}
```

- 行级 diff（`from_lines`），3 行 context（git 默认）
- 文件头必加（`markdown.rs` 依赖 `--- `/`+++ ` 判断 in-diff 状态）
- old 或 new 为空时，similar 产出全 `+` 或全 `-` hunk

### 4.2 `parse_hunks(text)` — unified diff 解析

```rust
struct Hunk {
    new_start: usize,       // 从 @@ -X,Y +A,B @@ 里的 A 提取
    header: String,         // 原始 @@ 行（用于染色）
    lines: Vec<ParsedLine>,
}

enum ParsedLine {
    Add(String),
    Del(String),
    Context(String),
    FileHeader(String),
}
```

逐行扫描，跳过纯空行；用 `@@` 前缀识别 hunk header，正则 `^\@\@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? \@\@` 抽取 new_start。

### 4.3 `compute_gutter_width(hunks)` — gutter 宽度

```rust
fn compute_gutter_width(hunks: &[Hunk]) -> usize {
    let max_line = hunks.iter()
        .flat_map(|h| {
            let mut n = h.new_start;
            h.lines.iter().filter_map(move |l| {
                let v = matches!(l, Add(_) | Context(_)).then_some(n);
                if matches!(l, Add(_) | Context(_)) { n += 1; }
                v
            })
        })
        .max()
        .unwrap_or(1);
    max_line.to_string().len() + 3  // marker(1) + space(1) + digits + space(1)
}
```

匹配原版 `StructuredDiff.tsx::computeGutterWidth` 的 `max_digits + 3` 公式。

### 4.4 `detect_language(path, first_line)`

```rust
fn detect_language(path: &str, first_line: &str) -> Option<&'static SyntaxReference> {
    let syntax_set = get_syntax_set(); // 复用 markdown.rs 已有 singleton
    if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        if let Some(syntax) = syntax_set.find_syntax_by_extension(ext) {
            return Some(syntax);
        }
    }
    syntax_set.find_syntax_by_line(first_line)
}
```

### 4.5 `precompute_line_highlights(content, lang, theme)`

syntect 的 `HighlightLines` 内部维护状态机（多行字符串、块注释），必须按行顺序喂。所以**预先对整个 file_content 做一次完整 tokenization**，得到 per-line 的 spans，再按 hunk 行号取用。

```rust
fn precompute_line_highlights(
    content: &str,
    lang: &SyntaxReference,
    theme: &Theme,
) -> Vec<Vec<Span<'static>>> {
    let syntax_set = get_syntax_set();
    let theme_set = get_theme(); // 复用 markdown.rs 全局 theme
    let mut h = HighlightLines::new(lang, theme_set);
    content.lines().map(|line| {
        h.highlight_line(line, syntax_set)
            .unwrap_or_default()
            .into_iter()
            .map(|(style, s)| Span::styled(s, syntect_style_to_ratatui(style, theme)))
            .collect()
    }).collect()
}
```

### 4.6 `render_diff(...)` — 主渲染

伪代码（精简）：

```rust
pub fn render_diff(text, file_path, file_content, theme, width) -> Vec<Line<'static>> {
    let hunks = parse_hunks(text);
    if hunks.is_empty() { return Vec::new(); }  // 不是 diff

    let gutter_w = compute_gutter_width(&hunks);
    let lang = detect_language(file_path, first_line_of(file_content));
    let highlights = lang.map(|l| precompute_line_highlights(file_content, l, theme));

    let mut out = Vec::new();
    let mut total_lines = 0;
    const MAX_TOTAL_LINES: usize = 2000;

    for hunk in &hunks {
        if total_lines >= MAX_TOTAL_LINES { break; }
        out.push(styled_line(&hunk.header, Color::Blue, BOLD));
        let mut line_no = hunk.new_start;

        for line in &hunk.lines {
            if total_lines >= MAX_TOTAL_LINES {
                out.push(styled_line(
                    format!("... (diff truncated, {} more lines)", remaining),
                    theme.secondary_text,
                ));
                return out;
            }
            match line {
                Add(content) => {
                    let spans = pick_highlight(&highlights, line_no, content, theme)
                        .map_or_else(|| vec![plain_span(content)], |h| h);
                    out.push(guttered_line("+", Some(line_no), spans, gutter_w, theme.success_text));
                    line_no += 1;
                }
                Del(content) => {
                    // 不做语法高亮（节约一份 before highlights）
                    out.push(guttered_line_red("-", None, content, gutter_w, theme.warning_text));
                }
                Context(content) => {
                    let spans = pick_highlight(&highlights, line_no, content, theme)
                        .map_or_else(|| vec![plain_span(content)], |h| h);
                    out.push(guttered_line(" ", Some(line_no), spans, gutter_w, theme.secondary_text));
                    line_no += 1;
                }
                FileHeader(s) => out.push(styled_line(s, theme.accent_text, BOLD)),
            }
            total_lines += 1;
        }
    }
    out
}
```

### 4.7 颜色叠加规则（关键决策）

| 行类型 | marker 色 | 行号色 | 行内容色 |
|---|---|---|---|
| `+` 新增 | `theme.success_text` (绿) | `theme.success_text` | syntect 语法色 |
| `-` 删除 | `theme.warning_text` (红) | —（不显示行号） | `theme.warning_text`（纯红，无语法高亮） |
| ` ` context | `theme.secondary_text` (dim) | `theme.secondary_text` | syntect 语法色 |
| `@@ ...` hunk header | — | — | `Color::Blue` + BOLD |
| `--- ` / `+++ ` file header | — | — | `theme.accent_text` + BOLD |

`-` 行不做语法高亮的理由：要避免再算一份 before 版本的 highlights（内存与时间翻倍）。`-` 行用纯红 marker + 纯红内容，靠 marker 与上下文识别已足够。

---

## 5. 边界情况与降级

### 5.1 markdown 路径无 file_path（无围栏元信息）

- `file_path = ""` → `detect_language` 返回 `None` → 所有行只染 marker 色，不做语法高亮
- 行渲染退化到"按 +/- marker 染色 + gutter + hunk header 格式化"
- 视觉与现有 `render_diff_text` 接近，但加了 gutter

### 5.2 markdown 围栏元信息 `path="..."`

edit_file/write_file 的 ToolResult 输出必须用 ` ```diff path="..." ` 围栏（不是裸 ` ```diff `），才能在内联展示时触发语法高亮。markdown 解析器在 fence info string 中提取 `path="..."`，传给 `render_diff_text`。

### 5.3 file_content 读不到 / 文件已被改

- markdown 路径：尝试 `fs::read_to_string(path)`；失败则降级（无语法高亮）
- 权限对话框路径：file_content 来自 disk 当前内容，已读到即用于高亮 after 版本
- 极少情况下 disk 文件在 tool 执行后被外部改了：高亮可能不匹配，可接受

### 5.4 二进制文件

- `compute_unified_diff` 入口处探测 old/new 是否含 NUL 字节
- 若是，返回 `"[binary file changed]"` 字面量
- renderer 直接渲染为单行系统消息，不尝试高亮

### 5.5 巨型 diff 截断

- 单个 hunk 超过 500 行：在 hunk 边界截断，附加 `... (hunk truncated, N more lines)`
- 总行数超过 2000：renderer 停止追加，附加 `... (diff truncated, N more lines)`
- 权限对话框预览额外限制：最多 20 行 body，超出截断
- 阈值常量在 diff.rs 顶部声明，便于调整

### 5.6 syntect 找不到语言定义

`detect_language` 返回 `None` → 所有行只染 +/- marker 色，不做语法高亮。**降级，不是错误**。

### 5.7 空文件 / 新建 / 删除

- 新建（old=""）：全 `+` hunk
- 删除（new=""）：全 `-` hunk
- old==new：compute_unified_diff 返回空 hunks（只有 `---`/`+++` 文件头），renderer 检测到 hunks 为空，返回 `Vec::new()`，不渲染任何东西

### 5.8 Windows 换行 / 无尾换行

`similar::TextDiff::from_lines` 对 `\r\n` 和 `\n` 都正确处理；无尾换行的最后一行也视为完整行。

---

## 6. 测试策略

### 6.1 单元测试（`src/tui/diff.rs` `#[cfg(test)]` 模块）

**`compute_unified_diff`（5 个）：**

- `test_compute_basic_replace`
- `test_compute_new_file` — old=""
- `test_compute_delete_file` — new=""
- `test_compute_no_change` — old==new
- `test_compute_includes_file_header`

**`parse_hunks`（4 个）：**

- `test_parse_extracts_new_start`
- `test_parse_classifies_lines`
- `test_parse_multiple_hunks`
- `test_parse_non_diff_returns_empty`

**`compute_gutter_width`（3 个）：**

- `test_gutter_single_digit` — max=9 → width=4
- `test_gutter_triple_digit` — max=100 → width=6
- `test_gutter_empty`

**`detect_language`（3 个）：**

- `test_detect_by_extension_rs`
- `test_detect_by_extension_py`
- `test_detect_unknown_falls_back_to_none`

**`render_diff`（6 个）：**

- `test_render_addition_marker_green`
- `test_render_deletion_marker_red`
- `test_render_context_marker_dim`
- `test_render_gutter_alignment`
- `test_render_no_language_falls_back`
- `test_render_truncates_large_diff`

**`highlight`（2 个）：**

- `test_highlight_multiline_string_spanning_hunks`
- `test_highlight_no_language_returns_unstyled`

### 6.2 工具集成测试

**`edit_file`（3 个）：**

- `test_edit_file_output_contains_diff_block`
- `test_edit_file_diff_has_correct_line_numbers`
- `test_edit_file_diff_includes_unified_file_header`

**`write_file`（2 个）：**

- `test_write_file_new_file_diff_all_additions`
- `test_write_file_overwrite_shows_minus_and_plus`

### 6.3 不做的测试

- markdown 路径集成测试（已有 markdown.rs 测试覆盖）
- 权限对话框 ratatui 后端快照测试（维护成本过高，改为手动 review）
- 性能基准测试

### 6.4 手动验证清单（提交前）

- [ ] `cargo test` 全绿
- [ ] `cargo clippy` 无新警告
- [ ] 启动 codecoder，让 Claude 调一次 edit_file → 看到内联 diff、有 gutter、有语法高亮
- [ ] 权限对话框弹出时看到 diff 预览
- [ ] 切窄终端（40 列），diff 不撑破布局

---

## 7. 后续（非本轮）

本 spec 完成后：

- 若权限对话框预览引入了新的 dialog body 渲染模式（变高对话框），可作为后续 OtherToolPermission 类型扩展的参考实现
- 若性能在实际使用中成为问题（巨型 diff 渲染卡顿），再单独立 spec 引入缓存层（对应原版 WeakMap）
- 全屏 diff 查看器（DiffDialog）作为独立 spec，依赖本管线的 `render_diff` 函数
