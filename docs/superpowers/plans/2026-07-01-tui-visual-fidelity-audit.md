# TUI 视觉保真度审计 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 产出一份 `docs/audit-tui-visual-fidelity.md`，对照原版 Claude Code 与 codecoder 的 TUI 视觉/渲染层，覆盖 A/B/C/D/E 五组约 35-40 个审计行，与现有 `audit-tui-fidelity.md` 同构、平级、互补。

**Architecture:** 单一文档交付，无代码改动。10 个任务：1 个骨架、5 个分组审计（A/B/C/D/E，可并行）、3 个综合任务（V1/V2/V3 硬伤、附录 A 代码映射、裁决清单）、1 个自查。每个任务独立可验证、独立提交。

**Tech Stack:** Markdown 文档；grep/Glob/Read 为主要工具；不引入新依赖。

## Global Constraints

引自 spec `docs/superpowers/specs/2026-07-01-tui-visual-fidelity-audit-design.md`：

- **本轮不写一行渲染代码**，只产 `docs/audit-tui-visual-fidelity.md`
- 每条审计行必须引**两个锚点**（原版 + codecoder），锚点格式：`path/to/file.tsx:NN` 与 `src/tui/file.rs:NN`
- 找不到原版锚点的条目进附录 B「待补」，不进入裁决清单
- 严重度 5 档：⛔ 视觉 bug / 🔴 显著差距 / 🟡 细微差距 / 🟢 有意偏离（带 ADR）/ ✅ 视觉等价
- 修复难度 3 档：S < 30 min / M 1-4h / L > 4h（可能需新依赖）
- 颜色判定以源码字面值为准（hex / ANSI 编号 / `Color::*` 枚举名），不靠肉眼猜
- ADR 引用具体编号（0003 主题、0006 确认对话框等）
- 文档结构与 `audit-tui-fidelity.md` 同构：第 0 节硬伤 → 1-5 节分组 → 附录 A/B → 裁决清单

---

## File Structure

**唯一新建文件：** `docs/audit-tui-visual-fidelity.md`

任务 1 创建骨架后，后续任务以 Edit 工具向对应章节追加内容。任务之间通过文件内的章节锚点协同，没有跨文件依赖。

**只读不改的源码参考：**

codecoder 端（`src/tui/`）：
- `mod.rs` — 主渲染循环、布局
- `status_bar.rs` — 状态栏
- `input_area.rs` — 输入区
- `message_list.rs` — 消息列表
- `dialogs.rs` — 所有对话框
- `markdown.rs` — markdown 渲染
- `completion.rs` — 补全逻辑
- `theme.rs` — 颜色定义

原版端（`archived/claude-code/src/`）：
- `components/PromptInput/*` — 输入区/状态栏
- `components/permissions/*` — 权限对话框
- `components/CustomSelect/*` — 通用选择器
- `components/HelpV2/*` — 帮助页
- `components/diff/*` — diff 渲染
- `components/HighlightedCode*` — 代码高亮
- `components/messages/*` — 消息渲染（如存在）
- `main.tsx` — 主渲染（按需 grep）
- `ink/` — Ink 主题/颜色定义

---

## Task 1: 创建审计文档骨架

**Files:**
- Create: `docs/audit-tui-visual-fidelity.md`

**Interfaces:**
- Produces: 文档骨架，含所有章节标题与空表头；后续任务向表中追加行

- [ ] **Step 1: 创建骨架文件**

写入 `docs/audit-tui-visual-fidelity.md`，内容如下（占位文字稍后由后续任务替换）：

```markdown
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
<!-- TASK 2 填充 -->

### A2. 状态栏

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 2 填充 -->

### A3. 输入区

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 2 填充 -->

### A4. 消息列表容器

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
<!-- TASK 2 填充 -->

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
```

- [ ] **Step 2: 验证骨架结构**

Run: `grep -c '^### ' docs/audit-tui-visual-fidelity.md`
Expected: `31`（A 组 4 + B 组 6 + C 组 8 + D 组 8 + E 组 5 = 31 个表面小节）

Run: `grep -c '<!-- TASK' docs/audit-tui-visual-fidelity.md`
Expected: `35`（31 个表面占位 + 第 0 节 1 + 附录 A/B 2 + 裁决清单 1 = 35）

- [ ] **Step 3: 提交**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): scaffold TUI visual fidelity audit doc

Empty tables for all 31 surfaces across groups A-E, awaiting per-group
exploration in subsequent tasks.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: 审计 A 组 — 持久 chrome

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（A1-A4 四个空表）

**Interfaces:**
- Consumes: Task 1 的骨架文件
- Produces: A1-A4 四节填充完成，每行四元组完整

**审计表面（4 个）：**
- **A1. 整体布局与分区**：3 栏布局、外边框、padding、终端尺寸自适应
- **A2. 状态栏**：model 名、mode、token、cost、context %、footer 提示键、边框
- **A3. 输入区**：prompt 符、placeholder、多行渲染、行号、字数统计、镜像提示
- **A4. 消息列表容器**：滚动条、消息间分隔、留白、user 头像/角色标记

**关键审计维度（每个表面至少查这些）：**
- 边框字符（Unicode round `─│┌┐└┘` / 方角 `┌─┐` / ASCII）
- 边框颜色
- 内 padding（左右各 1 字符？无 padding？）
- 文本前缀符（`>`/`▶`/`❯`/`> `）
- 占位文字（"Type your message..." 等）
- 选中/活跃颜色
- 滚动条形态（`░`/`▒`/`█`/字符粗细）

- [ ] **Step 1: 定位 codecoder 端锚点**

Run:
```bash
grep -nE 'Layout::|Block::|borders|Borders::' src/tui/mod.rs | head -40
grep -nE 'Block::|borders|render_widget' src/tui/status_bar.rs | head -20
grep -nE 'Block::|borders|prompt|placeholder' src/tui/input_area.rs | head -30
grep -nE 'Scrollbar|scrollbar|render_widget' src/tui/message_list.rs | head -20
```
记录每个文件的关键行号。

- [ ] **Step 2: 定位原版锚点（按表面）**

对每个表面，在 `archived/claude-code/src/components/` 下用 Glob 找文件，再 grep 关键字符：

```bash
# A1 整体布局
grep -rnE 'borderStyle|BorderStyle|padding' archived/claude-code/src/components/PromptInput/ archived/claude-code/src/components/FullscreenLayout.tsx 2>/dev/null | head -30

# A2 状态栏
ls archived/claude-code/src/components/PromptInput/ | grep -i footer
grep -rnE 'useTokenCount|TokenDisplay|model.*name' archived/claude-code/src/components/PromptInput/PromptInputFooter*.tsx 2>/dev/null | head -20

# A3 输入区
grep -rnE 'placeholder|prompt.*>|ShimmeredInput' archived/claude-code/src/components/PromptInput/*.tsx 2>/dev/null | head -20

# A4 消息列表
ls archived/claude-code/src/components/messages/ 2>/dev/null || ls archived/claude-code/src/components/ | grep -iE 'message|history'
grep -rnE 'borderStyle|marginTop' archived/claude-code/src/components/FullscreenLayout.tsx 2>/dev/null | head -10
```

- [ ] **Step 3: 写入 A1-A4 的审计行**

用 Edit 工具替换 `<!-- TASK 2 填充 -->` 占位（共 4 处），每节至少 4 行审计。每行格式：

```
| 边框字符 | `<具体字符>` 原版（components/X.tsx:NN） | `<具体字符>` codecoder（src/tui/Y.rs:NN） | 🟡/🔴/✅ | S/M |
```

要点：
- 字符直接抄源码字面值（用 grep -F 找到的实际字符）
- 颜色抄源码字面值（`Color::Cyan` / `"#06b6d4"` / `chalk.cyan`）
- 锚点行号必须真实
- 找不到原版锚点的维度，标"原版锚点：未找到"，整行进附录 B（不在本节出现）

- [ ] **Step 4: 验证填充完整性**

Run:
```bash
grep -c '<!-- TASK 2 填充 -->' docs/audit-tui-visual-fidelity.md
```
Expected: `0`（4 处占位全部被替换）

Run:
```bash
# 检查 A1-A4 区域内的所有锚点引用真实
awk '/^### A1\./,/^### A2\./' docs/audit-tui-visual-fidelity.md | grep -oE 'src/tui/[a-z_]+\.rs:[0-9]+' | sort -u
```
然后对每个锚点用 `sed -n 'NNp' src/tui/file.rs` 验证行号确实存在。

- [ ] **Step 5: 提交**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): fill group A surfaces (persistent chrome)

A1 layout, A2 status bar, A3 input area, A4 message list container.
Each row cites both original and codecoder source anchors.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: 审计 B 组 — 消息变体

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（B1-B6 六个空表）

**Interfaces:**
- Consumes: Task 1 骨架
- Produces: B1-B6 六节填充完成

**审计表面（6 个）：**
- **B1. User 消息**：前缀符、缩进、wrap
- **B2. Assistant 消息**：前缀符、段落间距
- **B3. System 消息**：info/warning/error 三态颜色、`[end]`/`[!]`/`[error]` 标记
- **B4. Tool 调用**：header 行、input 折叠、output 截断、status icon
- **B5. Reasoning**：折叠态、展开态、dim 颜色、前缀符
- **B6. Diff 渲染**：`+`/`-` 行、hunk header、行号、颜色

**关键审计维度：**
- 前缀符（`▶`/`▷`/`> `/`user:`/`❯`）
- 颜色（user 主文本色、assistant 主文本色、system 三态色）
- 折叠图标（`▸`/`▾`/`+`/`-`/`>`/`v`）
- 工具调用 status icon（`⏳`/`✓`/`✗`/`●`/`◉`/`○`）
- diff 行前缀（`+`/`-`/` `）与颜色
- 段落间距（一行空行？无？）

**特别注意 B6（diff）：** spec §2.3 说这类"整条管线缺失"项要带原版蓝图。若 codecoder 完全没有 diff 渲染：
- 在 B6 表里单列一行：状态 ⛔ 或 🔴 L，原版锚点填 `components/diff/*`，codecoder 锚点填"未实现"
- 在该节下面追加一段"原版蓝图"：列出原版 diff 渲染的关键文件、关键函数、关键颜色规则（约 5-10 行）

- [ ] **Step 1: 定位 codecoder 锚点**

```bash
grep -nE 'MessageItem::|User|Assistant|System|ToolCall|Reasoning' src/tui/message_list.rs | head -40
grep -nE 'Diff|diff|edit_file|FileEdit' src/tui/message_list.rs src/tui/markdown.rs 2>/dev/null | head -10
grep -nE 'fold|collapse|expand' src/tui/message_list.rs | head -10
```

- [ ] **Step 2: 定位原版锚点**

```bash
# 消息组件
ls archived/claude-code/src/components/messages/ 2>/dev/null || find archived/claude-code/src/components -maxdepth 2 -name '*Message*' -o -name '*UserMessage*' -o -name '*AssistantMessage*' 2>/dev/null

# Reasoning
find archived/claude-code/src -name '*Reasoning*' -o -name '*Thinking*' 2>/dev/null | head -10

# Tool 调用渲染
find archived/claude-code/src/components -name '*ToolUse*' -o -name '*ToolCall*' -o -name '*ToolResult*' 2>/dev/null | head -10

# Diff
ls archived/claude-code/src/components/diff/ 2>/dev/null
grep -rnE '^\+.*|^-.*|hunkHeader|@@' archived/claude-code/src/components/diff/*.tsx 2>/dev/null | head -20
```

- [ ] **Step 3: 写入 B1-B6**

用 Edit 替换 6 处 `<!-- TASK 3 填充 -->`。每节至少 3-5 行审计。B6 若整条缺失，按上面"特别注意"补原版蓝图段落。

- [ ] **Step 4: 验证**

```bash
grep -c '<!-- TASK 3 填充 -->' docs/audit-tui-visual-fidelity.md
```
Expected: `0`

```bash
# 验证 B6 蓝图段落（如果 codecoder 确实缺失）
awk '/^### B6\./,/^---/' docs/audit-tui-visual-fidelity.md | grep -c '蓝图\|blueprint'
```
Expected: `>= 1`（如果 B6 标了缺失；否则视实际情况可跳过此检查）

- [ ] **Step 5: 提交**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): fill group B surfaces (message variants)

B1 user, B2 assistant, B3 system, B4 tool call, B5 reasoning, B6 diff.
B6 includes original blueprint if codecoder lacks diff rendering.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: 审计 C 组 — 覆盖层

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（C1-C8 八个空表）

**Interfaces:**
- Consumes: Task 1 骨架
- Produces: C1-C8 八节填充完成

**审计表面（8 个）：** C1 Permission / C2 Plan / C3 AskQuestion / C4 Confirm / C5 Slash / C6 @ File / C7 Model Picker / C8 Help

**关键审计维度：**
- 边框样式（粗框 / 双线 / 圆角 / 方角）
- 标题行（位置、颜色、是否带工具名）
- 选项高亮（背景色 / 前景色反白 / `▸` 前缀）
- 选项分隔符（空行 / `─` / 无）
- footer 提示行（`↑↓ select / Enter confirm / Esc cancel`）
- 数字键直选提示
- 多选 box（`[ ]` / `[x]` / `( )` / `(*)`）

**特别注意：** 行为审计 `audit-tui-fidelity.md` §3 已确认 codecoder 对话框"全部是字母键驱动，无任何方向键导航"。本组审计**只看视觉**（边框、高亮、布局），不重复行为差距。

- [ ] **Step 1: 定位 codecoder 锚点**

```bash
grep -nE 'fn render_.*dialog|fn render_.*popup|fn render_help|fn render_model' src/tui/dialogs.rs src/tui/mod.rs src/tui/completion.rs 2>/dev/null | head -30
grep -nE 'Block::|bordered|Clear' src/tui/dialogs.rs | head -30
```

- [ ] **Step 2: 定位原版锚点**

```bash
# 权限对话框
ls archived/claude-code/src/components/permissions/
grep -rnE 'PermissionDialog|PermissionRequest' archived/claude-code/src/components/permissions/PermissionDialog.tsx 2>/dev/null | head -10

# CustomSelect（通用 popup 样式）
ls archived/claude-code/src/components/CustomSelect/
grep -rnE 'borderStyle|highlight' archived/claude-code/src/components/CustomSelect/*.tsx 2>/dev/null | head -20

# Plan 审批
find archived/claude-code/src -name '*Plan*' -o -name '*Approval*' 2>/dev/null | head -10

# 帮助
ls archived/claude-code/src/components/HelpV2/ 2>/dev/null

# Slash / 文件补全
find archived/claude-code/src -name '*Slash*' -o -name '*Completion*' -o -name '*Mention*' 2>/dev/null | head -10
```

- [ ] **Step 3: 写入 C1-C8**

替换 8 处 `<!-- TASK 4 填充 -->`。每节至少 3 行。C8（帮助页）若 codecoder 缺失，按 spec §2.3 写原版蓝图。

- [ ] **Step 4: 验证**

```bash
grep -c '<!-- TASK 4 填充 -->' docs/audit-tui-visual-fidelity.md
```
Expected: `0`

- [ ] **Step 5: 提交**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): fill group C surfaces (overlays)

C1 permission, C2 plan, C3 ask-question, C4 confirm, C5 slash, C6 file
completion, C7 model picker, C8 help. Visual only; behavior already
covered in audit-tui-fidelity.md.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: 审计 D 组 — Markdown 与代码

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（D1-D8 八个空表）

**Interfaces:**
- Consumes: Task 1 骨架
- Produces: D1-D8 八节填充完成

**审计表面（8 个）：** D1 行内 / D2 标题 / D3 列表 / D4 代码块 / D5 表格 / D6 引用 / D7 链接 / D8 分割线

**关键审计维度：**
- bold 是否真粗体（终端支持 italics/bold 时）
- italic 是否真斜体
- 标题是否带底色/前缀/序号
- 列表缩进字符（`  ` / `\t` / `    `）
- checkbox 字符（`[ ]` / `[x]` / `☐` / `☑`）
- 代码块语言标签位置（顶行 / 角标 / 无）
- 代码块是否真有语法高亮（spec §2.3 — 若整条缺失带蓝图）
- 表格边框字符（`|` / `+` / `─` / Unicode）
- 引用块前缀（`> ` / `│ ` / `▌ `）
- 链接渲染（直接 URL / `[n]` 引用 / inline `[text](url)`）

**特别注意 D4（代码块语法高亮）：** 若 codecoder `markdown.rs` 无语法高亮，按 spec §2.3 写原版蓝图（含原版用的语法高亮库/方法）。

- [ ] **Step 1: 定位 codecoder 锚点**

```bash
grep -nE 'fn render_|fn format_|Heading|List|Code|Table|BlockQuote|Link|Rule' src/tui/markdown.rs | head -40
grep -nE 'syntax|highlight|syntect|tree-sitter' src/tui/markdown.rs Cargo.toml 2>/dev/null
```

- [ ] **Step 2: 定位原版锚点**

```bash
# HighlightedCode（语法高亮）
ls archived/claude-code/src/components/HighlightedCode* 2>/dev/null
find archived/claude-code/src/components/HighlightedCode -type f 2>/dev/null
grep -rnE 'highlight|syntax|Prism|shiki|tokenize' archived/claude-code/src/components/HighlightedCode* 2>/dev/null | head -20

# Markdown 渲染主入口
find archived/claude-code/src -name '*Markdown*' 2>/dev/null | head -10
```

- [ ] **Step 3: 写入 D1-D8**

替换 8 处 `<!-- TASK 5 填充 -->`。每节至少 2-4 行。D4 若整条缺失，加原版蓝图段落。

- [ ] **Step 4: 验证**

```bash
grep -c '<!-- TASK 5 填充 -->' docs/audit-tui-visual-fidelity.md
```
Expected: `0`

- [ ] **Step 5: 提交**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): fill group D surfaces (markdown and code)

D1 inline, D2 headings, D3 lists, D4 code blocks, D5 tables, D6
blockquotes, D7 links, D8 rules. D4 includes syntax-highlight blueprint
if codecoder lacks it.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: 审计 E 组 — 颜色与主题

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（E1-E5 五个空表）

**Interfaces:**
- Consumes: Task 1 骨架
- Produces: E1-E5 五节填充完成

**审计表面（5 个）：** E1 暗色调色板 / E2 亮色 parity / E3 选中高亮 / E4 dim 文本 / E5 边框字符集

**关键审计维度（spec §4.2 要求颜色到字面值级别）：**
- 暗色主文本色：原版（`"#xxxxxx"` 或 ANSI 编号）vs codecoder（`Color::White` 等）
- 暗色次要文本：原版 dim 处理（`chalk.dim` / ANSI 256 / `DarkGray`）vs codecoder（`Color::DarkGray`）
- 暗色 accent 色：原版 cyan 具体色号 vs codecoder `Color::Cyan`
- 暗色 warning：原版 yellow 具体色号 vs codecoder `Color::Yellow`
- 选中高亮：原版反白具体处理 vs codecoder `selected_fg / selected_bg`
- 边框字符集：原版用的是 `─│┌┐└┘` Unicode / ASCII / 双线 / 圆角 vs codecoder 用的（grep `Block::default().borders(Borders::ALL)` 看具体 symbol set）

**特别注意：** 原版 `ink/` 目录是 Ink 内部，可能不是主题定义点；主题更可能定义在 `constants/colors.ts`、`theme.ts`、或直接散落在组件里。Step 1 要先找原版的颜色定义点。

- [ ] **Step 1: 定位 codecoder 颜色定义**

Read: `src/tui/theme.rs`（已知，整文件 131 行）

- [ ] **Step 2: 定位原版颜色定义点**

```bash
# 找颜色定义文件
find archived/claude-code/src -name 'color*' -o -name 'theme*' -o -name 'palette*' 2>/dev/null | head -20
ls archived/claude-code/src/constants/ 2>/dev/null

# 搜字面 hex 色
grep -rnE '"#[0-9a-fA-F]{6}"' archived/claude-code/src/constants/ archived/claude-code/src/components/design-system/ 2>/dev/null | head -30

# 搜 ANSI 256
grep -rnE 'ansi256|ansi\(' archived/claude-code/src/ 2>/dev/null | head -10

# 搜 chalk / colors 调用
grep -rnE 'chalk\.|colors\.' archived/claude-code/src/constants/ 2>/dev/null | head -20
```

- [ ] **Step 3: 写入 E1-E5**

替换 5 处 `<!-- TASK 6 填充 -->`。每节至少 3-5 行。颜色对比**必须引字面值**，例如：

```
| 主文本色 | `#ffffff` "white"（constants/colors.ts:NN） | `Color::White`（src/tui/theme.rs:47） | ✅ | — |
| accent | `#06b6d4` "cyan-500"（constants/colors.ts:NN） | `Color::Cyan`（src/tui/theme.rs:49） | 🟡 | S |
```

E5（边框字符集）：列出原版用的所有 Unicode 边框字符 vs codecoder 用的，并 grep 双侧的 `borderStyle` / `bordered` 设置。

- [ ] **Step 4: 验证**

```bash
grep -c '<!-- TASK 6 填充 -->' docs/audit-tui-visual-fidelity.md
```
Expected: `0`

```bash
# 颜色对比必须引字面值
awk '/^### E1\./,/^### E2\./' docs/audit-tui-visual-fidelity.md | grep -cE 'Color::|#[0-9a-fA-F]{3,6}|ansi'
```
Expected: `>= 4`（每行至少 2 处颜色引用 × 至少 2 行）

- [ ] **Step 5: 提交**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): fill group E surfaces (colors and theme)

E1 dark palette, E2 light parity, E3 selection highlight, E4 dim text,
E5 border charset. Each color row cites literal values from both sides.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: 综合 V1/V2/V3 视觉硬伤

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（第 0 节 `<!-- TASK 7 填充 -->`）

**Interfaces:**
- Consumes: Tasks 2-6 已完成的所有审计行
- Produces: 第 0 节"必须先看"列出 3-5 条最严重的视觉硬伤

- [ ] **Step 1: 通读已填充的 1-5 节，挑硬伤**

Read: `docs/audit-tui-visual-fidelity.md`（整个文件）

挑选标准（按优先级）：
1. 所有 ⛔ 视觉 bug 必入
2. 🔴 显著差距中影响面最大的（一个差距影响多个表面优先）
3. 🔴 L 难度的整条管线缺失（如 diff、语法高亮，若 B6/D4 标了缺失）
4. 与原版"肌肉记忆"冲突的（例如 prompt 符从 `>` 改成 `▶`）

挑 3-5 条，超过 5 条要再筛。

- [ ] **Step 2: 写入第 0 节**

替换 `<!-- TASK 7 填充 -->`，每条硬伤用类似行为审计 H1/H2/H3 的格式：

```markdown
| # | 问题 | 位置 | 级别 | 状态 |
|---|---|---|---|---|
| V1 | **<一句话描述>**：<具体差异> | <codecoder 文件:行> vs <原版组件:行> | ⛔/🔴 | 待修 |

> <可选：影响面、修复路径提示>
```

每条要引回具体表面的章节锚点（如"详见 §2.B6"）。

- [ ] **Step 3: 验证**

```bash
# V1-V5 的数量在 3-5 之间
awk '/^## 0\./,/^---$/' docs/audit-tui-visual-fidelity.md | grep -cE '^\| V[0-9]'
```
Expected: `>= 3` 且 `<= 5`

- [ ] **Step 4: 提交**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): synthesize top visual hard issues (V1-Vn)

Reads all group A-E rows, picks the 3-5 most severe visual gaps into
section 0 for fast triage.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: 写附录 A（代码映射图）与附录 B（待补条目）

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（附录 A 与附录 B 两处 `<!-- TASK 8 填充 -->`）

**Interfaces:**
- Consumes: Tasks 2-6 的所有锚点
- Produces: 双向映射表 + 未找到原版锚点的条目集中存放

- [ ] **Step 1: 收集所有锚点**

```bash
# codecoder 锚点
grep -oE 'src/tui/[a-z_]+\.rs:[0-9]+' docs/audit-tui-visual-fidelity.md | sort -u

# 原版锚点
grep -oE 'components/[A-Za-z0-9/_-]+(\.tsx)?:[0-9]+' docs/audit-tui-visual-fidelity.md | sort -u
```

- [ ] **Step 2: 写附录 A（映射图）**

替换附录 A 的 `<!-- TASK 8 填充 -->`，产出一张三列映射表：

```markdown
| codecoder 文件 | 原版对应组件 | 备注 |
|---|---|---|
| src/tui/status_bar.rs | components/PromptInput/PromptInputFooter*.tsx | 状态栏 |
| src/tui/input_area.rs | components/PromptInput/PromptInput.tsx + ShimmeredInput.tsx | 输入框 |
| src/tui/dialogs.rs | components/permissions/* + components/CustomSelect/* | 对话框 |
| src/tui/markdown.rs | components/HighlightedCode* + (markdown 渲染入口) | markdown |
| ... | ... | ... |
```

每个 codecoder 文件至少一行。原版无对应的，标"无对应（codecoder 自创）"或"原版无对应"。

- [ ] **Step 3: 写附录 B（待补条目）**

通读全文，把所有 `原版锚点：未找到` 或 `需运行时验证` 的条目集中到附录 B，格式：

```markdown
- **<表面编号>.<维度>**：<差异简述>。原版锚点未找到（已查 <文件列表>）。需运行时验证 / 需补充原版组件路径。
```

如果通读后没有这类条目，附录 B 写"本轮审计所有条目均已定位双侧锚点"。

- [ ] **Step 4: 验证**

```bash
# 附录 A 至少 8 行（每个 codecoder TUI 文件一行）
awk '/^## 附录 A/,/^## 附录 B/' docs/audit-tui-visual-fidelity.md | grep -cE '^\| .*\.rs'
```
Expected: `>= 8`

```bash
grep -c '<!-- TASK 8 填充 -->' docs/audit-tui-visual-fidelity.md
```
Expected: `0`

- [ ] **Step 5: 提交**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): add appendix A (code map) and B (pending items)

Appendix A: bidirectional codecoder-to-original file mapping.
Appendix B: items where the original anchor could not be located.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: 写裁决清单

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（最末 `<!-- TASK 9 填充 -->`）

**Interfaces:**
- Consumes: Tasks 2-8 的所有产出
- Produces: 末尾裁决清单，按优先级分 4 桶

- [ ] **Step 1: 通读全文，按桶分类**

Read: 整个 `docs/audit-tui-visual-fidelity.md`

把所有审计行（除 ✅ 视觉等价）分到 4 桶：
- **先修**：⛔ 视觉 bug + 🔴 显著差距且难度 S/M
- **再裁决**：🟡 细微差距，逐条定性（保留 / 回补）
- **有意偏离**：🟢 已有 ADR 背书（如 0003 主题、0006 确认对话框），仅需确认是否仍认可
- **缺失大件**：🔴 L 难度的整条管线缺失，需另立 spec（如 diff 渲染、语法高亮、表格、帮助页）

- [ ] **Step 2: 写入裁决清单**

替换 `<!-- TASK 9 填充 -->`，格式与行为审计裁决清单同构：

```markdown
**先修（视觉 bug / 显著差距且难度 S/M）**
1. <表面.维度> — <一句话>（<锚点>）难度 S/M
2. ...

**再裁决（细微差距，逐条定性）**
3. <表面.维度> — <一句话>（<锚点>）建议：保留/回补
4. ...

**有意偏离（ADR 背书，仅需确认是否仍认可）**
5. <表面.维度> — <一句话>（ADR 000X）
6. ...

**缺失大件（L 难度，需另立 spec）**
7. <表面> — <一句话>，原版蓝图见 §<章节>。建议下一轮单独 brainstorm。
8. ...
```

每条都要能链回前面章节的具体行号或表面编号。

- [ ] **Step 3: 验证**

```bash
# 4 个桶都存在
grep -cE '^\*\*(先修|再裁决|有意偏离|缺失大件)' docs/audit-tui-visual-fidelity.md
```
Expected: `4`

```bash
grep -c '<!-- TASK 9 填充 -->' docs/audit-tui-visual-fidelity.md
```
Expected: `0`

```bash
# 没有遗留占位
grep -c '<!-- TASK' docs/audit-tui-visual-fidelity.md
```
Expected: `0`

- [ ] **Step 4: 提交**

```bash
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): write adjudication list (4 priority buckets)

Pre-fix (S/M difficulty), re-adjudicate (subtle gaps), intentional
(ADR-backed), and missing-large (L difficulty, needs separate spec).
Each entry cross-references its source row.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: 自查与修订

**Files:**
- Modify: `docs/audit-tui-visual-fidelity.md`（按自查结果修订）

**Interfaces:**
- Consumes: Tasks 1-9 的完整产出
- Produces: 通过 spec §4.4 自查清单的最终版本

- [ ] **Step 1: 自查 spec §4.4 的 3 项**

通读全文，按 spec `docs/superpowers/specs/2026-07-01-tui-visual-fidelity-audit-design.md` §4.4 自查：

- [ ] 有没有把"原版没有、codecoder 也没有"误标成差距？（应改为 ✅ 或删除）
- [ ] 有没有把 ADR 背书的偏离误标成 🔴？（应改为 🟢 并引 ADR 编号）
- [ ] 有没有把"渲染管线整体缺失"压成单行？（应单列小节，并加原版蓝图）

记录问题清单。

- [ ] **Step 2: 自查锚点真实性**

抽检 5 个锚点（每个 group 至少 1 个），验证 cited 行号确实存在：

```bash
# 例如检查锚点 src/tui/dialogs.rs:NNN
sed -n 'NNNp' src/tui/dialogs.rs
# 例如检查锚点 components/X.tsx:NNN
sed -n 'NNNp' archived/claude-code/src/components/X.tsx
```

若锚点偏移（例如 cited 第 100 行实际不是 cited 的内容），修正锚点。

- [ ] **Step 3: 自查 rubric 一致性**

```bash
# 严重度符号使用次数
grep -oE '⛔|🔴|🟡|🟢|✅' docs/audit-tui-visual-fidelity.md | sort | uniq -c
```

合理性检查：
- ⛔ 应该极少（≤ 5 条，全在第 0 节硬伤 + 散落在原文）
- ✅ 应该有一定数量（视觉等价的部分）
- 🟢 应该都引 ADR 编号

```bash
# 每个 🟢 都应引 ADR 编号
grep -B1 -A1 '🟢' docs/audit-tui-visual-fidelity.md | grep -c 'ADR'
```
Expected: `>=` 🟢 出现次数（每个 🟢 至少对应 1 处 ADR 引用）

- [ ] **Step 4: 应用修订**

用 Edit 工具修订发现的问题。

- [ ] **Step 5: 最终验证**

```bash
# 总审计行数（不算表头）
grep -cE '^\|.*src/tui/' docs/audit-tui-visual-fidelity.md
```
Expected: `>= 35`（spec §3 约 35-40 行）

```bash
# 总字数（粗估）
wc -l docs/audit-tui-visual-fidelity.md
```
Expected: `>= 400`（spec §6 结构 + 内容估计）

- [ ] **Step 6: 提交（如有修订）**

只有实际改动了才提交：

```bash
git status --short docs/audit-tui-visual-fidelity.md
# 若有改动：
git add docs/audit-tui-visual-fidelity.md
git commit -m "$(cat <<'EOF'
docs(audit): self-review fixes

Apply spec §4.4 self-review checklist: fix mis-classified severities,
correct drifted anchors, expand pipeline-missing items into subsections.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

若 Step 1-3 未发现问题，跳过本步，整个审计工作完成。

---

## 完成判据

整个审计工作完成的判据：

1. `docs/audit-tui-visual-fidelity.md` 存在
2. 文件内**没有** `<!-- TASK` 占位
3. 至少 35 行审计（`grep -cE '^\|.*src/tui/' docs/audit-tui-visual-fidelity.md` ≥ 35）
4. 第 0 节有 3-5 条 V1-Vn 硬伤
5. 附录 A 有 ≥ 8 行 codecoder ↔ 原版映射
6. 末尾裁决清单 4 桶齐全
7. Tasks 1-10 各自的 commit 都在 git 历史里
