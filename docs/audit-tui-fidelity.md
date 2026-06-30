# TUI 行为保真度审计：codecoder (Rust/ratatui) vs claude-code (TS/Ink)

审计轴：**A1 — 紧贴原版逐项保真**。每条偏离用三元组表达：

> 原版行为 → codecoder 行为 → **ADR 背书?**

标记约定：
- 🟢 **有 ADR**：偏离是 codecoder 有意为之，已被 ADR 解释。保真层面记为"已知偏离"，进入你的"设计调整"清单。
- 🔴 **无 ADR**：没有文档解释的偏离 = 疑似遗漏 / 移植不完整 / 漂移。**需要你裁决**。
- ⛔ **实现 vs 自身 ADR 矛盾**：codecoder 写了一套 ADR，但代码没做到。这是 bug，不是设计。

范围：1 键位/模式、2 文本编辑、3 对话框、4 补全浮层。附录 = slash 命令对照。渲染层（markdown/颜色）不在本轮。

原版锚点：`keybindings/defaultBindings.ts`、`hooks/useTextInput.ts`、`components/PromptInput/`、`components/permissions/`、`components/CustomSelect/`、`commands.ts`。
codecoder 锚点：`src/tui/{mod,input_area,dialogs,completion,commands,app}.rs`。

---

## 0. 必须先看：3 个硬伤（⛔/🔴 高优先级）

| # | 问题 | 位置 | 级别 | 状态 |
|---|---|---|---|---|
| H1 | **Home/End 被绑成消息列表滚动，吞掉了光标移动**；input_area 里的 Home/End 光标代码是死代码，且与 ADR 0001 直接矛盾 | `mod.rs:373-382` vs `input_area.rs:259-264` | ⛔ | ✅ 已修：移除 mod.rs 滚动绑定，Home/End 回落为当前行首/尾光标 |
| H2 | **@ 文件补全浮层无法导航**：`completion.selected` 永远是 0，Up/Down 改成移动光标，Tab 永远只能选第一个候选 | `mod.rs:252-264`（路由缺失）+ `input_area.rs:235-258` | 🔴 | ✅ 已修：补全开启时 Up/Down 改选中项，Tab/Enter 接受高亮候选 |
| H3 | **AskQuestion 对话框丢弃了结构化选项**：原版给可选项列表，codecoder 只接受自由文本，Y/N 直接发字面量 "yes"/"no" | `mod.rs:563-567`、`dialogs.rs:430-432` | 🔴 | ✅ 已修：扩展协议做成可选列表（详见下方）|

> **H3 修复中额外发现并修掉的两个隐藏 bug**：(a) 旧 Esc 分支对 AskQuestion 发的是 `PermissionResponse` 而非 `AskUserResponse`，导致 `ask_user` 工具收不到答复、**阻塞 300 秒超时**；(b) 对话框键路由先于 input_area，旧的自由文本输入路径（`input_area.rs`）实际是**死代码**——用户根本无法把字符打进 AskQuestion，y/a/n 还会被劫持成 "yes"/"no"。两者均已随 H3 修复消除。

详见对应章节。

---

## 1. 键位 & 模式

### 1.1 退出语义

| 键 | 原版 → codecoder → ADR |
|---|---|
| **Esc** | 多义（autocomplete dismiss / 双击清空输入 / abort speculation / 双击空输入开 rewind / loading 时取消任务），**从不退出** → 级联关闭 reverse-search→search→selected_msg→completion，然后 no-op，**从不退出** → 🟢 ADR 0001 |
| **Ctrl+Q** | 未绑定（惰性，会被当字符吞掉）→ **唯一退出键** → 🟢 ADR 0001 |
| **Ctrl+C** | **双击**才退出（防误触）；单击在非空输入时清空输入；忙时取消任务 → 忙时发 `Interrupt`；**空闲时单击立即退出**，不清输入 → 🟢 ADR 0001（但丢了原版的"双击防误退"保护，且丢了"单击清空输入"，见 1.5） |
| **Ctrl+D** | 空输入双击退出；非空 = 向前删字符（readline EOF）→ **完全惰性**（无任何处理）→ 🟡 ADR 0001 只说"不再退出"，但**没说要保留向前删字符**，该行为被静默丢弃 → 🔴 部分 |

> **裁决点（Ctrl+C）**：原版双击退出是经典防误触设计。codecoder 改为空闲单击退出（Ctrl+C≈Ctrl+Q）。ADR 0001 明确选了这个，但值得你确认是否要回补"双击确认"或至少保留"非空输入时 Ctrl+C 先清空而不是退出"。

### 1.2 Enter 语义

| 场景 | 原版 → codecoder → ADR |
|---|---|
| 普通提交 | 提交 → 提交 → ✅ 一致 |
| 换行 | `Shift`/`Meta` 或**行尾反斜杠 `\`** → 插入 `\n` | `Shift+Enter` → 插入 `\n` |
| **Alt/Meta+Enter** | 原版 = **插入换行** → codecoder = **强制提交** → 🟢 ADR 0001（但**语义反转**：原版 Meta+Enter 是换行，codecoder Alt+Enter 是提交，肌肉记忆冲突） |
| **行尾 `\` 续行** | 支持（删反斜杠并插 `\n`）→ ✅ **已修**：光标前为 `\` 时 Enter 转为换行而非提交 |
| autocomplete 打开时 | 接受并执行建议 → 见 4.1 |

### 1.3 Tab 语义

| 场景 | 原版 → codecoder → ADR |
|---|---|
| 文本中 | no-op（不插入）→ no-op（移除了旧的"插 2 空格"）→ 🟢 ADR 0001 |
| 折叠/展开消息 | 原版无此功能（折叠走 Ctrl+O transcript 等）→ Tab 折叠/展开最后一条 Reasoning/ToolCall → 🟢 ADR 0001（新增设计） |
| slash 浮层中 | **Tab = 接受/填入命令（不执行）** → **Tab = 循环到下一个候选** → 🔴 无 ADR，语义不同；codecoder 因此**没有"填入但不执行以便加参数"的能力** |
| @ 文件补全中 | Tab 补全到最长公共前缀再插入 → Tab 插入候选（无 LCP）→ 🔴 无 ADR |

### 1.4 上下移动 & 历史

| 键 | 原版 → codecoder → ADR |
|---|---|
| **Up/Down** | 光标移动（含软换行/逻辑行）→ 移不动时走历史 → 多行内移动光标；空输入时进 browse 浏览消息 → 🟢 ADR 0001 |
| **Ctrl+Up/Down** | 原版 = 词级光标移动 → codecoder = 走输入历史 → 🟢 ADR 0001 |
| 单行非空输入按 Up/Down | 走历史 → **死键**：`try_move_cursor_vertical` 仅在含 `\n` 时生效，单行非空时既不移动也不浏览也不历史 → 🔴 无 ADR，边角但确实是无反馈的死键 |

### 1.5 文本编辑键（详见第 2 章）

| 键 | 原版 → codecoder → ADR |
|---|---|
| Ctrl+A/E | 行首/行尾（当前行）→ ✅ **已修**：改为当前行首/尾（与 Home/End、ADR 0001 一致）|
| Ctrl+K/U | 删到**当前行**首/尾 + 进 kill-ring → 删到**整个输入**首/尾，无 kill-ring → 🔴 无 ADR（与下方 kill-ring 一并裁决）|
| Ctrl+W | 删前一词 + kill-ring → 删前一词，无 kill-ring → 🔴 无 ADR（功能在，kill-ring 缺失） |
| **Ctrl+Y** | **yank（粘贴 kill-ring）** → **redo（重做）** → 🔴 无 ADR，键义冲突（待裁决）|
| **Ctrl+Z** | 原版 = 保留给终端挂起（SIGTSTP，reservedShortcuts 警告）→ codecoder = undo → 🔴 无 ADR，占用了保留键（待裁决）|
| 词级光标 Alt+B/F、Ctrl+←/→ | 有 → ✅ **已修**：Ctrl/Alt+←→ 词级移动（Ctrl+↑↓ 仍归历史）|

### 1.6 全局快捷键冲突

| 键 | 原版 → codecoder → ADR |
|---|---|
| **Ctrl+L** | 重绘屏幕 → **清空消息（带确认）** → 🟢 ADR 0006（但与原版"重绘"肌肉记忆冲突） |
| **Ctrl+T** | 切换 Todos 面板 → **切换主题** → 🟡 ADR 0003 给了主题切换但**未明确指定 Ctrl+T 这个键**；与原版冲突 → 🔴 键位选择无 ADR |
| 模型选择器 | **Meta+P** → **Ctrl+P** → 🔴 无 ADR（且原版 Ctrl+P = 上一条/历史） |
| Ctrl+R | 历史搜索 → 反向搜索（近似）→ 🟡 行为相近 |
| Ctrl+O / Meta+O / Meta+T 等 | transcript / fast mode / thinking 切换 → **全缺** → 🔴 无 ADR（功能本身可能未实现，属功能差距） |

### 1.7 Esc 级联顺序：实现与 ADR 文字不符 ⛔(轻)

ADR 0001 声明单一级联顺序：`reverse-search → search → slash-completion → model-picker → help → dialog → selected-msg → file-completion`。
实现里 dialog/help/model-picker/slash-completion 在 `mod.rs:252-264`（step 2 覆盖层路由）**先于** step-3 的 Esc 级联被各自 handler 消费，step-3 只处理 `reverse-search → search → selected_msg → completion`。净效果接近，但 ADR 描述的"单一有序级联"实际被拆成两层，且 `selected_msg` 相对 `dialog` 的先后与 ADR 文字不一致。→ ⛔ 文档/实现表述不符（低危，建议改 ADR 文字或注释说明分层）。

---

## 2. 文本编辑

### H1 ⛔ Home/End 被夺，光标移动是死代码，违反 ADR 0001

- **ADR 0001 原文**：`Home/End are cursor-to-line-edge (readline convention); scrolling the message list to top/bottom is g/G (vim convention)`。
- **实现**：`mod.rs:373-382` 在 step-3 系统快捷键里把 `Home`→滚到顶、`End`→跳到底，并 `return`，**先于** `input_area::handle_input_key`。而且**没有 `app.input.is_empty()` 守卫**（对比 `g/G` 在 `mod.rs:345-358` 有 empty 守卫）。
- **后果**：
  1. 正在打字时按 Home/End 会滚动消息列表，而不是移动光标。
  2. `input_area.rs:259-264` 的 Home→`cursor_pos=0`、End→`cursor_pos=len` **永远执行不到**（死代码）。
  3. 滚动到顶/底现在**同时**绑在 `g/G`（ADR 指定）和 `Home/End`（ADR 未指定）上 —— Home/End 重复占用。
- **裁决**：要么给 Home/End 加 empty 守卫让其在打字时回落到光标移动（贴合 ADR），要么修订 ADR 承认 Home/End=滚动。当前是**实现自相矛盾**。

> 注：即便修好路由，`input_area` 的 Home/End 仍是"整段输入首/尾"而非 ADR 要求的"当前行首/尾"，多行下仍不符（与 Ctrl+A/E 同源）。

### 2.x 其他（见 1.5 表）
kill-ring 体系整体缺失（Ctrl+K/U/W 不进环、无 Ctrl+Y yank）；行级 vs 整段差异；词级移动缺失；`\` 续行缺失。均 🔴 无 ADR。

---

## 3. 对话框

> 全局结构差异：原版四类对话框**全部基于 `CustomSelect`**，统一支持方向键/`j`/`k`/`Ctrl+N/P` 导航 + 数字键直选 + Enter 接受 + Esc 取消。codecoder 四类对话框**全部是字母键驱动**（Y/N/A/Esc），**无任何方向键导航**。permission/confirm 的字母键方案有 ADR 背书（0005/0006），但 AskQuestion/Plan 丢弃导航**无 ADR**。

### 3.1 工具权限对话框

| 维度 | 原版 → codecoder → ADR |
|---|---|
| 交互模型 | 可导航列表（↑↓/数字/Enter）→ 字母键 Y/A/Shift+A/N/Esc → 🟢 ADR 0005 |
| 授权范围 | once + "always for X in cwd"（2 档）→ once/session/project（3 档）→ 🟢 ADR 0005 |
| Esc | = 拒绝 → = 拒绝（发 `allowed:false`）→ ✅ 一致 |
| **反馈/修订** | Tab 展开 reject/accept 反馈文本框（告诉 Claude 原因）→ **无** → 🔴 无 ADR，缺失 |
| Bash 前缀授权 | "don't ask again for `<prefix>`" → **无**（无命令前缀粒度）→ 🔴 无 ADR |

### 3.2 AskQuestion 对话框 — H3 ✅ 已修

**修复方案（扩展协议做成可选列表）：**
- `AgentResponse::AskUser` 与 `Dialog::AskQuestion` 新增 `options: Vec<String>`（对话框另带 `selected: usize`）。
- `ask_user` 工具入参支持可选 `options`：`{"question":"...","options":["A","B"]}`，并在工具描述里说明。
- 新增 `dialogs::render_ask_question_dialog`：有选项时渲染 ▸ 高亮的可选列表；用户可 ↑↓ 选择或直接打字输入自定义答案（"__other__" 路径——打字时高亮转移到自由文本）。无选项时退化为纯自由文本提示。
- 新增 `dialogs::handle_ask_question_key`：↑↓ 移动选中项、Enter 提交（打字优先，否则提交高亮项）、Esc 跳过、字符/退格/左右编辑自由文本。
- `handle_dialog_key` 顶部把 AskQuestion 路由给专用 handler，原 Y/A/N 三键不再劫持成 "yes"/"no"。

| 维度 | 原版 → codecoder（修复后） |
|---|---|
| 选项列表 | `Select` 渲染 options + `__other__` → ✅ 可选列表 + 自由文本 |
| 导航 | ↑↓ + Enter → ✅ ↑↓ 选择、Enter 确认 |
| 自由文本 | `__other__` → ✅ 直接打字即进入自定义答案，优先于高亮项 |
| Esc | onCancel 拒绝 → ✅ 发 `AskUserResponse{"[skipped]"}`（修掉了原来误发 PermissionResponse 致 300s 阻塞的 bug）|

> 仍与原版有差：未实现 `SelectMulti` 多选（Space 切换）与多问题 Tab 切换——当前协议每次一个问题。若后续需要多选，再扩 `options` 为带选中态的结构。

### 3.3 Plan 审批对话框

| 维度 | 原版 → codecoder → ADR |
|---|---|
| 选项 | "Yes, auto-accept edits" / "Yes, manually approve" / "No, keep planning"(+clear-context/ultraplan 变体) → **仅 Y=approved / N=rejected**（二元字符串）→ 🔴 无 ADR |
| auto-accept vs 手动 | 区分（影响后续是否逐个批准编辑）→ **丢失**，无此区分 → 🔴 无 ADR，语义缺口 |
| keep-planning + 反馈 | "No, keep planning" 是输入型选项，可附反馈，Shift+Tab 直接 auto-accept；Ctrl+G 进 `$EDITOR` 改 plan → **全缺** → 🔴 无 ADR |

### 3.4 Confirm 对话框

| 维度 | 原版 → codecoder → ADR |
|---|---|
| 键 | `y`/`Enter`=yes，`n`/`Esc`=no，方向键，Tab → `Y`=confirm，`N`/`Esc`=cancel → 🟢 ADR 0006 |
| **Enter** | = **YES** → **no-op**（`dialogs.rs:524-541` 的 Enter 分支只处理 AskQuestion，Confirm 落到 `else` 把对话框放回，等于无反应）→ 🟡 ADR 0006 的渲染提示刻意只写 `Y=confirm N=cancel Esc=cancel`，看似有意"破坏性操作必须显式 Y"，但**ADR 未明文说明 Enter 应为 no-op**，建议补一句，否则用户按 Enter 无反馈会困惑 |
| `A` 键 | （无）→ no-op，对话框保持打开 → 🟢 ADR 0006 测试已覆盖 |

---

## 4. 补全浮层

### 4.1 Slash 命令补全

| 维度 | 原版 → codecoder → ADR |
|---|---|
| 触发 | 输入以 `/` 开头，光标>0，尚无参数 → 输入以 `/` 开头且无空白 → 🟡 近似 |
| 过滤 | **Fuse.js 模糊**，按 exact-name>alias>prefix>fuzzy 排序，分组（recent/builtin/user/...）→ **纯前缀 `starts_with`**，无模糊无分组（`input_area.rs:423-455`）→ 🔴 无 ADR；例：`/cfg` 在 codecoder 匹配不到 `/config` |
| 导航 | ↑↓ + Ctrl+N/P → ↑↓（`dialogs.rs:600-609`）→ 🟡 缺 Ctrl+N/P |
| **Tab** | 接受/填入（不执行）→ **循环候选** → 🔴 见 1.3 |
| Enter | 接受并执行 → 接受并执行 → ✅ |
| Esc | dismiss + 记录"已忽略输入"阻止重弹 → dismiss（重打 `/` 立即重弹，无抑制）→ 🔴 无 ADR，轻 |
| 行内 ghost text | 中段 `/com` 显示灰字提示，Tab 展开 → **无** → 🔴 无 ADR，缺失 |
| 有参数后 | （隐藏列表）→ 浮层保持打开显示全部命令作为 hint → 🟢 ADR 0002 §7 |

### 4.2 @ 文件补全 — H2 🔴

| 维度 | 原版 → codecoder → ADR |
|---|---|
| **导航** | ↑↓ 选择候选 → **无法导航**：`completion.selected` 仅在触发时被设为 0（`input_area.rs:281,287,312`），**没有任何按键修改它**；`mod.rs:252-264` 覆盖层路由**不包含** `app.completion.active`，故 Up/Down 落到 input handler 去移动光标/浏览消息。**Tab 永远只插入第一个候选** → 🔴 无 ADR，功能缺陷 |
| 搜索算法 | 预热索引 + 模糊 → 子串 `contains`，深度仅 root+1 层，上限 10（`completion.rs`）→ 🔴 无 ADR，能力弱化 |
| 目录下钻 | 选目录追加 `/` 并重新触发 → 插入目录路径即停 → 🔴 无 ADR |
| LCP / 空格加引号 / 无 `@` 路径补全 | 有 → 全缺 → 🔴 无 ADR，次要 |
| Enter 接受 | 支持 → 仅 Tab 接受，Enter 不接受 → 🔴 轻 |

### 4.3 模型选择器

| 维度 | 原版 → codecoder → ADR |
|---|---|
| 形态 | 模态 `Select` → 自定义浮层 → 🟡 一致意图 |
| 导航 | ↑↓ + `j`/`k` + Ctrl+N/P → 仅 ↑↓ → 🔴 无 ADR，缺 vim 键 |
| effort 档位切换 | Tab / ←→ 调 effort → **无** → 🔴 无 ADR（可能功能未实现）|
| 触发键 | Meta+P → Ctrl+P → 🔴 无 ADR |

---

## 附录 A — Slash 命令对照（部分）

codecoder 实际实现（`commands.rs:98-160`）：`/help`(/h) `/exit` `/quit` `/reload` `/clear` `/history` `/tools` `/skills` `/memory` `/session` `/resume` `/config` `/mcp`。
（浮层清单 `app.rs:176-187` 与之基本一致；`model` 经 `/config model` 子命令，无独立 `/model`。）

| 原版命令 | 用途 | codecoder | 备注 |
|---|---|---|---|
| /help | 帮助 | ✅ | 原版无 `/h` 别名，codecoder 加了 |
| /clear (reset,new) | 清空+确认 | ✅ | 🔴 缺别名 reset/new |
| /exit (quit) | 退出 | ✅ | codecoder 两者皆有 |
| /model | 设模型 | ⚠️ 部分 | 仅 `/config model`，无独立 `/model`；交互改 Ctrl+P |
| /resume | 恢复会话 | ✅ | + ADR 0006 确认 |
| /config (settings) | 设置 | ✅ | 🔴 缺别名 settings |
| /mcp | MCP 管理 | ✅ | |
| /compact | 压缩上下文保留摘要 | ❌ | 🔴 缺失（agent 侧可能有，TUI 未暴露）|
| /init | 生成 CLAUDE.md | ❌ | 🔴 缺失 |
| /cost /status /usage | 成本/状态/用量 | ❌ | 🔴 缺失 |
| /review /security-review | PR 审查 | ❌ | 🔴 缺失 |
| /vim | vim 编辑模式 | ❌ | 🔴 缺失 |
| /theme | 主题 | ❌(走 Ctrl+T) | 🟡 交互化 |
| /context /files | 上下文可视化 | ❌ | 🔴 缺失 |
| /copy /export /diff | 复制/导出/diff | ❌ | 🔴 缺失 |
| /agents /skills | 子代理/技能 | 部分(/skills 占位) | 🔴 /skills 仅打印占位文案 |
| /memory | 记忆 | 部分 | 🔴 仅打印目录提示，非编辑 |
| (其余 ~50 个：/login /logout /doctor /ide /mobile /stats /tag …) | — | ❌ | 多为云/账号/IDE 功能，属功能范围而非保真缺口 |

> 大量缺失命令属"功能范围"而非"行为保真"问题，列此供你做范围取舍；真正的保真问题是已实现命令的**别名缺失**与 `/skills`、`/memory`、`/tools` 的**占位实现**（只打印文案，不做事）。

---

## 裁决清单（按优先级）

**先修（实现 bug / 自相矛盾）**
1. ⛔ H1：Home/End 路由夺权 + 死代码 + 违反 ADR 0001 —— 加 empty 守卫或修订 ADR。
2. 🔴 H2：@ 文件补全无法导航 —— 把 `app.completion.active` 加入 `mod.rs` 覆盖层路由，增加 Up/Down 改 `completion.selected` 的 handler。
3. 🔴 H3：AskQuestion 丢弃选项 —— 决定是否扩展 `AgentResponse::AskUser` 携带 options；否则移除对自由问答无意义的 Y/N/A。

**再裁决（无 ADR 的偏离，定性为"保留/回补"）**
4. Plan 审批二元化（丢 auto-accept-edits / keep-planning+反馈）。
5. kill-ring 体系缺失 + Ctrl+Y 键义冲突（redo vs yank）+ Ctrl+Z 占用保留键。
6. slash 模糊匹配 vs 前缀匹配；Tab 语义（循环 vs 填入）。
7. ✅ 已修：词级光标移动（Ctrl/Alt+←→）、`\` 续行、Ctrl+A/E 行级语义。剩 Ctrl+K/U 多行行级语义（与 kill-ring 一并，见 #5）。
8. `/skills` `/memory` `/tools` 占位实现；常用命令别名缺失。

**已知有意偏离（ADR 背书，仅需确认是否仍认可）**
9. 🟢 Esc 永不退出 + 级联（ADR 0001）。
10. 🟢 Ctrl+Q 唯一退出 / Ctrl+C 空闲单击退出（ADR 0001）—— 注意丢了原版双击防误退。
11. 🟢 Tab 折叠消息（ADR 0001）。
12. 🟢 Ctrl+Up/Down 历史、Up/Down 光标+browse（ADR 0001）。
13. 🟢 权限三档 scope 字母键（ADR 0005）、Ctrl+L 清空确认（ADR 0006）、Ctrl+T 主题（ADR 0003）。
