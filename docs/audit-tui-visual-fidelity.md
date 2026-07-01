# TUI 视觉保真度审计：codecoder (Rust/ratatui) vs claude-code (TS/Ink)

审计轴：**A2 — 视觉保真**。每条偏离四元组：

> 原版渲染 → codecoder 渲染 → ADR 背书? → 严重度/难度

标记约定：
- ⛔ **视觉 bug**：codecoder 渲染破烂（错位、garbage、panic），不仅是"和原版不一样"
- 🔴 **显著差距**：用户一眼能看出不同，无 ADR 背书
- 🟡 **细微差距**：差几个字符、颜色 shade 微调、间距偏移，无 ADR
- 🟢 **有意偏离**：ADR 或文档明确说明
- ✅ **视觉等价**：差异在合理容差内

修复难度：S < 30 min / M 1-4h / L > 4h（可能需新依赖）。

范围：1 持久 chrome、2 消息变体、3 覆盖层、4 markdown/代码、5 颜色/主题。附录 = 渲染代码映射图 + 待补条目。**行为层已在 `audit-tui-fidelity.md` 覆盖，本轮不重复**。

原版锚点：`archived/claude-code/src/components/{PromptInput,permissions,CustomSelect,HelpV2,diff,HighlightedCode*,messages}/`、`main.tsx`、`ink/`。
codecoder 锚点：`src/tui/{mod,status_bar,input_area,message_list,dialogs,markdown,completion,theme}.rs`。

---

## 0. 必须先看：V1/V2/V3 视觉硬伤

<!-- TASK 7 填充 -->

---

## 1. 持久 chrome（A 组）

### A1. 整体布局与分区

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 三段 Flex 布局（消息区 flex_grow=1 + 输入区固定 + 状态栏固定） | `flexGrow={1}` 消息区 + 固定高度输入/状态区（FullscreenLayout.tsx:271,362） | `Constraint::Min(1)` 消息区 + `Length(2)` 输入 + `Length(1)` 状态（src/tui/mod.rs:193-199） | ✅ | S |
| 外边框 | 无外边框（全屏填充） | 无外边框（全屏填充） | ✅ | S |
| 内 padding | `paddingX={2}` 在模态框中（FullscreenLayout.tsx:427） | 无显式 padding（布局直接使用 area） | 🟡 | S |
| 终端尺寸自适应 | `useTerminalSize()` 动态重渲染（ink hooks） | 帧渲染时自动读取 `frame.area()` | ✅ | S |

### A2. 状态栏

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 模型名颜色 | Cyan Bold（StatusLine.tsx:152, 通过 renderModelName） | `Color::Cyan` + `Modifier::BOLD`（src/tui/status_bar.rs:29-32） | ✅ | S |
| 边框 | 无边框（纯文本行） | 无边框（纯文本行） | ✅ | S |
| token 显示格式 | `{}t ~${:.2}`（StatusLine.tsx:10, 通过 getTotalCost） | `{}t ~${:.2}`（src/tui/status_bar.rs:38-40） | ✅ | S |
| 右侧 CWD 显示 | `compact_cwd()` 显示 basename 或短路径（StatusLine.tsx） | `compact_cwd()` 显示 basename 或短路径（src/tui/status_bar.rs:125-146） | ✅ | S |
| context 进度条字符 | `▓`（filled） + `░`（empty）（format_context_bar） | `▓`（filled） + `░`（empty）（src/tui/status_bar.rs:158-159） | ✅ | S |
| spinner 字符 | `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`（StatusLine.tsx 或 CoordinatorAgentStatus.tsx） | `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`（src/tui/status_bar.rs:19） | ✅ | S |

### A3. 输入区

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| prompt 符号 | 无显式 `>` 符号（TextInput 组件直接渲染） | `"> "`（src/tui/input_area.rs:51） | 🟡 | S |
| 占位文字 | `usePromptInputPlaceholder()` 返回动态提示（usePromptInputPlaceholder.ts:25-60） | 无占位文字（空时仅显示 `"> "`）（src/tui/input_area.rs:52-53） | 🔴 | M |
| 上分隔线字符 | `'─'.repeat(columns)`（PromptInput.tsx:2253,2259,2267） | `"─".repeat(area.width.saturating_sub(1))`（src/tui/input_area.rs:40） | ✅ | S |
| 上分隔线颜色 | `swarmBanner.bgColor` 或 `promptBorder`（PromptInput.tsx:2252,2267） | `app.theme.secondary_text`（src/tui/input_area.rs:41） | 🟡 | S |
| 边框样式 | `borderStyle="round"` + `borderLeft={false}` + `borderRight={false}` + `borderBottom`（PromptInput.tsx:2237,2268） | 无边框（仅分隔线） | 🔴 | M |
| 边框颜色 | `promptBorder` → `rgb(153,153,153)`（theme.ts:126） | N/A（无边框） | 🔴 | M |
| 多行渲染 | TextInput 组件支持多行（TextInput.tsx） | 固定 2 行高度（src/tui/mod.rs:197） | 🔴 | L |

### A4. 消息列表容器

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 滚动条形态 | ScrollBox 组件内置滚动条（ink/ScrollBox.tsx） | `Scrollbar::new(ScrollbarOrientation::VerticalRight)`（src/tui/message_list.rs:344） | ✅ | S |
| 滚动条 thumb 颜色 | 主题色（ScrollBox 内部） | `app.theme.primary_text`（src/tui/message_list.rs:343） | ✅ | S |
| 滚动条 track 符号 | `begin_symbol(None)` + `end_symbol(None)`（src/tui/message_list.rs:345-346） | 无显式 track 符号 | ✅ | S |
| 容器边框 | 无边框（ScrollBox 直接填充） | `Borders::TOP | Borders::BOTTOM`（src/tui/message_list.rs:277） | 🟡 | S |
| 容器边框类型 | 无边框 | `BorderType::Plain`（src/tui/message_list.rs:278） | 🟡 | S |
| 消息间分隔 | 消息行之间无显式分隔符 | 消息行之间无显式分隔符 | ✅ | S |
| user 头像/角色标记 | `▶` 前缀（Messages.tsx 或 MessageRow.tsx） | `▶` 前缀（需在 B1 节确认） | ✅ | S |

---

## 2. 消息变体（B 组）

### B1. User 消息

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 3 填充 -->

### B2. Assistant 消息

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 3 填充 -->

### B3. System 消息

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 3 填充 -->

### B4. Tool 调用

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 3 填充 -->

### B5. Reasoning

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 3 填充 -->

### B6. Diff 渲染

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 3 填充 -->

---

## 3. 覆盖层（C 组）

### C1. Permission 对话框

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 4 填充 -->

### C2. Plan 审批对话框

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 4 填充 -->

### C3. AskQuestion 对话框

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 4 填充 -->

### C4. Confirm 对话框

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 4 填充 -->

### C5. Slash 补全浮层

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 4 填充 -->

### C6. @ 文件补全浮层

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 4 填充 -->

### C7. 模型选择器

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 4 填充 -->

### C8. 帮助页

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 4 填充 -->

---

## 4. Markdown 与代码（D 组）

### D1. 行内格式（bold/italic/code）

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 5 填充 -->

### D2. 标题 H1-H6

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 5 填充 -->

### D3. 列表（无序/有序/嵌套/checkbox）

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 5 填充 -->

### D4. 代码块（围栏/语言标签/语法高亮/行号/背景）

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 5 填充 -->

### D5. 表格

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 5 填充 -->

### D6. 引用块

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 5 填充 -->

### D7. 链接

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 5 填充 -->

### D8. 水平分割线

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 5 填充 -->

---

## 5. 颜色与主题（E 组）

### E1. 暗色主题调色板

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 6 填充 -->

### E2. 亮色主题 parity

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 6 填充 -->

### E3. 选中高亮

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 6 填充 -->

### E4. 次要文本 dim 处理

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 6 填充 -->

### E5. 边框字符集

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 6 填充 -->

---

## 附录 A — 渲染代码映射图

<!-- TASK 8 填充 -->

---

## 附录 B — 待补条目

<!-- TASK 8 填充 -->

---

## 裁决清单（按优先级）

<!-- TASK 9 填充 -->
