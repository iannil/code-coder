# TUI 视觉保真度审计 — 设计规格

**日期**：2026-07-01
**审计对象**：codecoder（Rust/ratatui）TUI 渲染层 vs `archived/claude-code/`（TS/Ink）UI
**交付物**：一份新文件 `docs/audit-tui-visual-fidelity.md`，与现有 `docs/audit-tui-fidelity.md` 平级、同结构、互补
**审计轴**：A2 — 视觉保真

---

## 1. 动机与背景

codecoder 已有一份高质量的行为保真审计 `audit-tui-fidelity.md`，覆盖键位/模式、文本编辑、对话框、补全浮层、slash 命令。**视觉/渲染层从未被系统审计过**，该审计文档第 12 行明确写着「渲染层（markdown/颜色）不在本轮」。

原版 Claude Code `src/components/` 有 147+ 个 UI 组件（PromptInput 系列、permissions 系列、CustomSelect、HelpV2、diff、HighlightedCode 等），codecoder 这边 TUI 只有约 10 个渲染文件。当前视觉差距是「整条管线可能缺失」级别（例如 diff 渲染、语法高亮、表格、checkbox 列表等codecoder 可能根本没有），不是「颜色差一个色号」级别。

本审计要做的，是给视觉层补一份与行为审计同构的 backlog，让后续修复有据可依。

## 2. 范围

### 2.1 本轮交付（**只做这件事**）

- 产 `docs/audit-tui-visual-fidelity.md` 一份文件
- 该审计文档单独提交一次 git（**不**与本 spec 文档的提交合并；本 spec 自身的提交属于 brainstorming 流程的一部分，先于审计工作发生）
- **不写一行渲染代码**

### 2.2 明确不在本轮

- 任何渲染代码改动（颜色、字符、边框、间距、布局）
- 性能问题（重绘闪烁、滚动卡顿、大数据量渲染）
- 终端模拟器兼容性（italics 支持、truecolor 回落、ASCII fallback）
- 行为层（已被 `audit-tui-fidelity.md` 覆盖）

### 2.3 修复难度为 L 的「整条管线缺失」类条目

只标注 + 写出原版蓝图，**不在本轮立 spec 修**。这类条目进入裁决清单的"缺失大件"分区，等下一轮单独 brainstorm。

---

## 3. 表面分类（surface taxonomy）

按可见区域分组，每组的子项就是审计表里的行。

### A. 持久 chrome（始终可见的"外壳"）

- **A1. 整体布局与分区**：messages / input / status 三栏；外边框；padding；终端尺寸自适应
- **A2. 状态栏**：model 名、mode 指示、token 计数、cost、context %、footer 提示键、边框样式
- **A3. 输入区**：`>`/`▶` prompt 符、placeholder、多行渲染、行号、字数统计、镜像模式提示
- **A4. 消息列表容器**：滚动条形态、消息间分隔、顶部/底部留白、user 头像/角色标记

### B. 消息变体（消息内容渲染）

- **B1. User 消息**：前缀符、缩进、wrap
- **B2. Assistant 消息**：前缀符、段落间距
- **B3. System 消息**：info/warning/error 三态颜色、`[end]`/`[!]`/`[error]` 标记
- **B4. Tool 调用**：header 行（图标 + 工具名 + 参数摘要）、input 折叠、output 截断、status icon ⏳✓✗
- **B5. Reasoning**：折叠态外观、展开态、dim 颜色、前缀符 `💭`/`reasoning`
- **B6. Diff 渲染**：`+`/`-` 行、hunk header `@@`、文件头、行号、颜色

### C. 覆盖层（overlays）

- **C1. Permission 对话框**：每种工具类型（bash / file write / file edit / web fetch 等）的视觉变体
- **C2. Plan 审批对话框**：plan 文本如何展示、3 选项布局
- **C3. AskQuestion 对话框**：选项列表 + 自由文本输入区如何并列
- **C4. Confirm 对话框**
- **C5. Slash 补全浮层**：每行（命令名 + 描述 + 别名）；选中态；边框
- **C6. @ 文件补全浮层**：图标 + 路径；目录/文件区分
- **C7. 模型选择器**：每个 model 卡片（name + effort + 当前选中 ✓）
- **C8. 帮助页**：键位表布局、分组、搜索框

### D. Markdown 与代码

- **D1. 行内格式**：**bold** / *italic* / `code`
- **D2. 标题**：H1-H6 高度、颜色、底色
- **D3. 列表**：无序 `-`/`*`、有序 `1.`、嵌套缩进、checkbox `- [ ]`
- **D4. 代码块**：围栏、语言标签、**语法高亮是否有**、行号、背景色
- **D5. 表格**：对齐、边框、表头
- **D6. 引用块**：`>` 前缀、灰色条
- **D7. 链接**：颜色、下划线、编号引用
- **D8. 水平分割线**

### E. 颜色与主题

- **E1. 暗色主题**：调色板是否与原版 Claude Code 接近（原版用 ANSI 256？truecolor？）
- **E2. 亮色主题**：parity
- **E3. 选中高亮**：背景色、前景色对比度
- **E4. 次要文本**：dim 处理（原版用 `DarkGray` 还是 256 色？）
- **E5. 边框字符**：Unicode `─│┌┐└┘├┤` vs ASCII `-|` vs 粗细变体

合计约 **35-40 个审计行**。

---

## 4. 方法论

### 4.1 每个表面的 5 步流程

1. **定位原版渲染锚点**：在 `archived/claude-code/src/` 找出负责该表面的组件文件 + 具体行号。优先级：`components/PromptInput/*`、`components/permissions/*`、`components/CustomSelect/*`、`components/HelpV2/*`、`components/diff/*`、`components/HighlightedCode*`、`components/messages/*`。`main.tsx`（234KB）按需 grep。
2. **定位 codecoder 锚点**：在 `src/tui/` 找出对应 render 函数 + 行号。
3. **三元组对照**：抄原版的实际渲染（字符、颜色、布局），抄 codecoder 的实际渲染，标注差异。
4. **定性 + 难度**：按 §5 rubric 标 ⛔/🔴/🟡/🟢/✅ 与 S/M/L。
5. **引 ADR**：若 codecoder 行为有 ADR 解释（如 0003 主题结构、0006 确认对话框），引用具体 ADR 编号；否则标"无 ADR"。

### 4.2 工具策略

- 原版是 TS/Ink + React 组件树、组件极多，**不逐个通读**，按表面反查：先确定一个表面（如"权限对话框边框"），再用 grep / Glob 找出原版具体渲染代码，只读必要的几段。
- 对颜色判定：以原版源码里出现的**字面值**为准（`"#06b6d4"`、`ansi256(...)`、`chalk.cyan` 等），不靠肉眼猜；codecoder 这边以 `theme.rs` 里 `Color::*` 字面值为准。
- 对字符判定：直接 grep 字符本身（如 `grep -nF '▶'`）。

### 4.3 歧义与未找到的处理

- 找不到原版锚点 → 该行标 `原版锚点：未找到`，不靠脑补。这种行进入附录 B「待补条目」，不计入裁决清单。
- 原版用 CSS/动态样式、运行时才能确定 → 标 `需运行时验证`，并说明需要什么验证条件。
- codecoder 这边的死代码或被路由绕过的渲染（类似行为审计 H1 那种）→ 单独标记，不计入"已实现差异"。

### 4.4 自我核查

审计文档写完后回头用 §5 rubric 自查每一条：
- 有没有把"原版没有、codecoder 也没有"误标成差距
- 有没有把 ADR 背书的偏离误标成 🔴
- 有没有把"渲染管线整体缺失"压成单行（应该单列小节）

---

## 5. Rubric

### 5.1 严重度

| 标 | 含义 |
|---|---|
| ⛔ | **视觉 bug**：codecoder 渲染破烂（错位、garbage、panic、明显的渲染缺陷），不仅是"和原版不一样" |
| 🔴 | **显著差距**：用户一眼能看出不同，无 ADR 背书 |
| 🟡 | **细微差距**：差一两个字符、颜色 shade 微调、间距偏移，无 ADR |
| 🟢 | **有意偏离**：ADR 或文档明确说明 |
| ✅ | **视觉等价**：差异在合理容差内 |

### 5.2 修复难度

| 档 | 范围 | 信号 |
|---|---|---|
| S | 单一 render 函数微调（颜色、字符、边框） | < 30 min |
| M | 跨多文件 / 需加新 theme role / 需补一个子组件 | 1-4 小时 |
| L | 整条渲染管线缺失或重写（语法高亮、diff、表格） | > 4 小时，可能需要新依赖 |

---

## 6. 输出文档结构

```
# TUI 视觉保真度审计：codecoder vs claude-code

审计轴 A2 — 视觉保真。每条偏离四元组：
  原版渲染 → codecoder 渲染 → ADR 背书? → 严重度/难度

## 0. 必须先看：V1/V2/V3 视觉硬伤
（带位置说明，最严重的 3-5 条）

## 1. 持久 chrome（A 组）
### A1. 整体布局与分区
### A2. 状态栏
### A3. 输入区
### A4. 消息列表容器

## 2. 消息变体（B 组）
### B1-B6（每个变体一节，每节一张表）

## 3. 覆盖层（C 组）
### C1-C8（每种 overlay 一节）

## 4. Markdown 与代码（D 组）
### D1-D8

## 5. 颜色与主题（E 组）
### E1-E5

## 附录 A — 渲染代码映射图
codecoder 文件 ↔ 原版组件 的双向对照表

## 附录 B — 待补条目
（原版锚点未找到 / 需运行时验证 的条目集中放这）

## 裁决清单（按优先级）
先修（视觉 bug / 显著差距且难度 S/M）
  1. ...
再裁决（细微差距，逐条定性）
  ...
有意偏离（ADR 背书，仅需确认是否仍认可）
  ...
缺失大件（L 难度，需另立 spec）
  ...
```

### 6.1 每条审计行的标准格式

```
| 维度 | 原版（components/X:NN） | codecoder（src/tui/Y.rs:NN） | 状态 | 难度 |
|---|---|---|---|---|
| 边框字符 | `─│┌┐└┘` Unicode round | `─│┌┐└┘` Unicode round | ✅ | — |
| 边框颜色 | `gray` ANSI | `Theme::secondary_text` (DarkGray) | 🟡 | S |
```

每行必须引两个锚点（除非是"原版缺失"），不引锚点的判定不入裁决清单。

---

## 7. 工作量预估

- 探索原版 + codecoder 双侧源码：4-6 小时（主要是原版那 147+ 组件的反查）
- 起草审计文档：2-3 小时
- 自查与修订：30 分钟-1 小时
- 单次 commit，仅新增 `docs/audit-tui-visual-fidelity.md` 一份文件

---

## 8. 后续（非本轮）

本审计完成、用户认领裁决清单后：

1. **S/M 难度的"先修"条目** → 可直接进入 writing-plans，作为下一轮工作。
2. **L 难度的"缺失大件"** → 每项单独 brainstorm（例如「为 codecoder 加 diff 渲染管线」是一个独立 spec）。
3. 视觉审计完成后，与现有 `audit-tui-fidelity.md` 并列维护；后续渲染层改动应回填本审计的对应行（类似行为审计里"✅ 已修"的回填机制）。
