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

| # | 问题 | 位置 | 级别 | 状态 |
|---|---|---|---|---|
| V1 | **Diff 缺少行号 gutter 与语法高亮**：原版有行号列 + ColorDiff 语法高亮（StructuredDiff.tsx:43-66），codecoder 无行号、无高亮（markdown.rs:314-323）详见 §2.B6 | `src/tui/markdown.rs:314-323` vs `archived/claude-code/src/components/StructuredDiff.tsx:43-66` | 🔴 | ✅ 已修 |
| V2 | **输入区缺少边框与多行支持**：原版有 round 边框 + 多行渲染（PromptInput.tsx:2237,2268），codecoder 无边框、固定 2 行（input_area.rs:40, mod.rs:197）详见 §1.A3 | `src/tui/input_area.rs:40` vs `archived/claude-code/src/components/PromptInput.tsx:2237,2268` | 🔴 | 待修 |
| V3 | **User/Assistant 消息缺少背景色与上边距**：原版有 userMessageBackground/messageActionsBackground + 上边距（UserPromptMessage.tsx:76, AssistantTextMessage.tsx:228），codecoder 无背景、无边距（message_list.rs:171,185）详见 §2.B1 / §2.B2 | `src/tui/message_list.rs:171,185` vs `archived/claude-code/src/components/{UserPromptMessage,AssistantTextMessage}.tsx:76,228` | 🔴 | 待修 |
| V4 | **System 消息缺少级别颜色与图标**：原版有 warning/error 颜色 + BLACK_CIRCLE(●)/TEARDROP_ASTERISK(✵)（SystemTextMessage.tsx:73,103,235），codecoder 无颜色、无图标（message_list.rs:242）详见 §2.B3 | `src/tui/message_list.rs:242` vs `archived/claude-code/src/components/SystemTextMessage.tsx:73,103,235` | 🔴 | 待修 |
| V5 | **Tool 调用缺少状态图标与进度消息**：原版有 ToolUseLoader 动画 + HookProgressMessage（AssistantToolUseMessage.tsx:186,240,328-358），codecoder 无图标、无进度（message_list.rs:216-234）详见 §2.B4 | `src/tui/message_list.rs:216-234` vs `archived/claude-code/src/components/AssistantToolUseMessage.tsx:186,240,328-358` | 🔴 | 待修 |

> **影响面分析**：
> - V1（Diff）影响所有代码差异显示（Edit、Read 结果），是阅读体验核心
> - V2（输入区）影响每次输入交互，是视觉识别关键
> - V3（消息背景）影响所有消息渲染，导致消息堆叠难以区分
> - V4（System 颜色）降低错误/警告可见性
> - V5（Tool 进度）消除异步操作反馈，用户无法感知工具执行状态
>
> **修复优先级**：V1 > V2 > V3 > V5 > V4。V1/V2/V3 为基础布局层，影响全局视觉一致性；V4/V5 为信息层，影响可读性但不破布局。

---

## 修复进度（V1 已落地）

- V1（B6 diff 缺 gutter + 语法高亮）→ ✅ 已修：见 `src/tui/diff.rs` 实现，commit `b7131aa`
- 裁决清单 #109（Diff 渲染.gutter 分栏）→ ✅ 已修
- 裁决清单 #110（Diff 渲染.语法高亮）→ ✅ 已修

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
| user 头像/角色标记 | 无显式前缀（UserPromptMessage.tsx 仅渲染文本） | `▶` 前缀（src/tui/message_list.rs:168） | 🔴 | S |

---

## 2. 消息变体（B 组）

### B1. User 消息

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 前缀符 | 无显式前缀（HighlightedThinkingText.tsx:91 使用 `figures.pointer` 但仅用于 thinking 内容） | `▶ ` (src/tui/message_list.rs:168) | 🔴 | S |
| 多行缩进 | 无显式缩进（Box flexDirection="column" 直接渲染） | `  ` 两空格缩进（src/tui/message_list.rs:175） | 🟡 | S |
| 文本颜色 | `text` 主题色（HighlightedThinkingText.tsx:99） | `primary` + BOLD（src/tui/message_list.rs:171） | 🟡 | S |
| 文本换行 | `wrap="wrap"` 自动换行（HighlightedThinkingText.tsx:99） | 无显式 wrap 配置 | 🔴 | M |
| 背景色 | `userMessageBackground` 或 `messageActionsBackground`（UserPromptMessage.tsx:76） | 无背景色 | 🔴 | M |
| 上边距 | `marginTop={addMargin ? 1 : 0}`（UserPromptMessage.tsx:76） | 无上边距 | 🔴 | S |
| 右内边距 | `paddingRight={useBriefLayout ? 0 : 1}`（UserPromptMessage.tsx:76） | 无内边距 | 🟡 | S |
| 截断机制 | `MAX_DISPLAY_CHARS=10000`，头 2500 + 尾 2500（UserPromptMessage.tsx:28-30,65-70） | 无截断机制 | 🔴 | M |
| thinking 高亮 | `findThinkingTriggerPositions()` 彩虹色高亮（HighlightedThinkingText.tsx:87） | 无 thinking 高亮 | 🔴 | L |

### B2. Assistant 消息

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 前缀符 | 无显式前缀（Markdown 组件直接渲染） | `▷ ` (src/tui/message_list.rs:181) | 🔴 | S |
| 多行缩进 | 无显式缩进 | `  ` 两空格缩进（src/tui/message_list.rs:193） | 🟡 | S |
| 文本颜色 | `text` 主题色（Markdown 组件继承） | `accent` 色前缀 + markdown 渲染（src/tui/message_list.rs:162,185） | 🟡 | S |
| 选中态前缀色 | `suggestion` 色（AssistantTextMessage.tsx:232） | `accent` + BOLD（src/tui/message_list.rs:162-163） | 🟡 | S |
| 选中态背景 | `messageActionsBackground`（AssistantTextMessage.tsx:229） | 无背景色 | 🔴 | M |
| 段落间距 | `addMargin ? 1 : 0` 上边距（AssistantTextMessage.tsx:228） | 无上边距 | 🔴 | S |
| 空消息过滤 | `isEmptyMessageText()` 返回 null（AssistantTextMessage.tsx:60-62） | 无空消息检查 | 🔴 | S |
| Markdown 渲染 | `<Markdown>{text}</Markdown>` 组件（AssistantTextMessage.tsx:241） | `render_markdown_with_highlight()`（src/tui/message_list.rs:182） | ✅ | S |
| 错误消息颜色 | `color="error"`（AssistantTextMessage.tsx:93,203 等） | 无特殊错误颜色 | 🔴 | M |

### B3. System 消息

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 前缀符 | `BLACK_CIRCLE` (●) 用于非 info 级别（SystemTextMessage.tsx:103,330,445） | 无前缀符，仅两空格缩进（src/tui/message_list.rs:242） | 🔴 | S |
| info 级别颜色 | `dimColor={true}` 灰色（SystemTextMessage.tsx:236,464） | `accent` 色非 dim（src/tui/message_list.rs:240） | 🔴 | S |
| warning 级别颜色 | `color="warning"` 黄色（SystemTextMessage.tsx:235） | 无特殊颜色 | 🔴 | M |
| error 级别颜色 | `color="error"` 红色（SystemTextMessage.tsx:103） | 无特殊颜色 | 🔴 | M |
| `[end]` 标记 | 支持，`startsWith("[end]")` 触发 dim（SystemTextMessage.tsx:237-238） | 支持，`startsWith("[end]")` 使用 `secondary` 色（src/tui/message_list.rs:237-238） | 🟡 | S |
| `[error]` 标记 | 支持，同 `[end]` dim 处理 | 支持，同 `[end]` `secondary` 色（src/tui/message_list.rs:237） | 🟡 | S |
| TEARDROP_ASTERISK 前缀 | `✵` 用于特殊类型（SystemTextMessage.tsx:73,141,164,562） | 无 | 🔴 | M |
| 上边距 | `marginTop={addMargin ? 1 : 0}`（SystemTextMessage.tsx:88,149,180） | 无上边距 | 🔴 | S |
| 背景色 | `backgroundColor={bg}` 响应选中态（SystemTextMessage.tsx:88,149） | 无背景色 | 🔴 | M |
| 宽度 | `width="100%"` 或 `width={columns-10}`（SystemTextMessage.tsx:88,149,353） | 无宽度限制 | 🔴 | S |
| info 级别隐藏 | `!verbose && message.level === "info"` 返回 null（SystemTextMessage.tsx:201-203） | 无隐藏逻辑 | 🔴 | S |

### B4. Tool 调用

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| header 前缀 | `BLACK_CIRCLE` (●) 或 ToolUseLoader 动画（AssistantToolUseMessage.tsx:186） | `⚙ ` 齿轮符号（src/tui/message_list.rs:216） | 🔴 | S |
| header 名称色 | `bold={true}` + 背景色可选（AssistantToolUseMessage.tsx:200） | `secondary` 色（src/tui/message_list.rs:217） | 🔴 | S |
| input 显示 | `renderToolUseMessage()` 格式化输入（AssistantToolUseMessage.tsx:163-167,318-327） | 直接渲染 input 文本（src/tui/message_list.rs:220-225） | 🔴 | M |
| input 折叠 | 支持折叠（verbose 控制） | 无折叠机制 | 🔴 | L |
| output 显示 | 直接渲染 output 文本（AssistantToolUseMessage.tsx:340） | 直接渲染 output 文本（src/tui/message_list.rs:228-234） | ✅ | S |
| output 截断 | 无显式截断 | 无截断机制 | ✅ | S |
| 状态图标 - queued | `BLACK_CIRCLE` + `dimColor`（AssistantToolUseMessage.tsx:186） | 无状态图标 | 🔴 | M |
| 状态图标 - in-progress | `ToolUseLoader` 组件动画（AssistantToolUseMessage.tsx:186, ToolUseLoader.tsx） | 无状态图标 | 🔴 | M |
| 状态图标 - error | `isError={true}` 传给 ToolUseLoader（AssistantToolUseMessage.tsx:186） | 无状态图标 | 🔴 | M |
| 状态图标 - resolved | 无图标（`isResolved` 时不显示 loader） | 无状态图标 | ✅ | S |
| 进度消息 | `renderToolUseProgressMessage()` + HookProgressMessage（AssistantToolUseMessage.tsx:240,328-358） | 无进度消息 | 🔴 | L |
| queued 提示 | `renderToolUseQueuedMessage()`（AssistantToolUseMessage.tsx:265,360-367） | 无 queued 提示 | 🔴 | M |
| 工具名背景色 | `backgroundColor={userFacingToolNameBackgroundColor}`（AssistantToolUseMessage.tsx:200） | 无背景色 | 🔴 | M |
| 工具名色 | `color={inverseText}` 配合背景（AssistantToolUseMessage.tsx:197,200） | 无反色 | 🔴 | M |
| 多行缩进 | 无显式缩进（Box flexDirection="column"） | `  ` 两空格缩进（src/tui/message_list.rs:222,230） | 🟡 | S |
| 上边距 | `marginTop={addMargin ? 1 : 0}`（AssistantToolUseMessage.tsx:285） | 无上边距 | 🔴 | S |
| 背景色 | `backgroundColor={bg}` 响应选中态（AssistantToolUseMessage.tsx:149,285） | 无背景色 | 🔴 | M |

### B5. Reasoning

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 前缀符（首行） | `figures.pointer` (❯ 或 ▶) + 空格（HighlightedThinkingText.tsx:91,145） | `· ` 中点符号（src/tui/message_list.rs:203） | 🔴 | S |
| 前缀符颜色 | `pointerColor` = `suggestion`（选中时）或 `subtle`（默认）（HighlightedThinkingText.tsx:24,91） | `secondary` 色（src/tui/message_list.rs:204） | 🟡 | S |
| 文本颜色 | `color="text"`（HighlightedThinkingText.tsx:99） | `secondary` 色（src/tui/message_list.rs:204） | 🔴 | S |
| 多行缩进 | 无显式缩进（所有行同列） | `  ` 两空格缩进（src/tui/message_list.rs:208） | 🔴 | S |
| 折叠态 | 支持（ThinkingToggle.tsx 控制展开/折叠） | 无折叠机制 | 🔴 | L |
| 展开态 | 完整文本渲染 | 完整文本渲染 | ✅ | S |
| dim 处理 | 无 dim（默认 text 色） | 使用 `secondary` dim 色（src/tui/message_list.rs:204） | 🟡 | S |
| thinking 触发高亮 | `findThinkingTriggerPositions()` 彩虹色高亮（HighlightedThinkingText.tsx:87） | 无触发高亮 | 🔴 | L |
| ultrathink 模式 | `isUltrathinkEnabled()` 启用特殊渲染（HighlightedThinkingText.tsx:87） | 无 ultrathink 模式 | 🔴 | L |
| brief 模式布局 | `useBriefLayout` 时简化为 "You" + timestamp（HighlightedThinkingText.tsx:25-80） | 无 brief 模式 | 🔴 | M |
| 上边距 | 无单独边距（父 Box 控制） | 无上边距 | ✅ | S |
| 背景色 | 继承父 `backgroundColor={bg}`（HighlightedThinkingText.tsx 未显式设置） | 无背景色 | 🟡 | M |

### B6. Diff 渲染

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| `+` 行颜色 | 绿色（colorDiff Rust NAPI，StructuredDiff.tsx:65） | `Color::Green`（src/tui/markdown.rs:314） | ✅ | S |
| `-` 行颜色 | 红色（colorDiff Rust NAPI） | `Color::Red`（src/tui/markdown.rs:318） | ✅ | S |
| `diff --git` 行 | `Color::Cyan` + `BOLD`（src/tui/markdown.rs:304） | `Color::Cyan` + `BOLD`（src/tui/markdown.rs:304） | ✅ | S |
| `---`/`+++` 行 | Cyan Bold（同 diff --git） | Cyan Bold（src/tui/markdown.rs:304） | ✅ | S |
| `@@` hunk header | 蓝色（colorDiff Rust NAPI，StructuredDiff.tsx:65） | `Color::Blue`（src/tui/markdown.rs:308） | ✅ | S |
| 上下文行（空格前缀） | 灰色 dim（colorDiff 默认） | `Color::DarkGray`（src/tui/markdown.rs:323） | 🟡 | S |
| 行号显示 | 有 gutter（行号列）（StructuredDiff.tsx:43-49,73-76） | 有行号列（src/tui/diff.rs） | ✅ | M |
| gutter 宽度 | `computeGutterWidth()` 动态计算（StructuredDiff.tsx:46-49） | 有 `compute_gutter_width()`（src/tui/diff.rs） | ✅ | M |
| gutter 分栏 | `RawAnsi` 双列渲染（gutter + content）（StructuredDiff.tsx:148-177） | 有分栏（src/tui/diff.rs） | ✅ | L |
| 语法高亮 | ColorDiff Rust NAPI 语法高亮（StructuredDiff.tsx:51-66） | 有语法高亮（syntect，src/tui/diff.rs） | ✅ | L |
| 边框 | `borderStyle="dashed"` + `borderLeft={false}` + `borderRight={false}`（FileEditToolDiff.tsx:98） | 无边框 | 🔴 | M |
| 容器背景 | 无背景（Box 直接渲染） | 无背景 | ✅ | S |
| 文件名显示 | `firstLine` 和 `filePath` 传给 ColorDiff（StructuredDiff.tsx:65,101） | 无文件名显示 | 🔴 | M |
| 折叠机制 | 无折叠（完整 diff 显示） | 无折叠 | ✅ | S |
| dim 支持 | `dim` 参数控制整体 dim（StructuredDiff.tsx:99） | 无 dim 参数 | 🔴 | M |
| 等宽字体 | RawAnsi 等宽渲染 | 等宽渲染 | ✅ | S |

> **原版蓝图**（原版 Diff 渲染实现）：
> 原版使用 `StructuredDiff` 组件渲染 diff，通过 `ColorDiff` Rust NAPI 进行语法高亮（StructuredDiff.tsx:51-66）。Diff 行颜色由 `colorDiff` 函数分配：绿色用于 `+` 行，红色用于 `-` 行，蓝色用于 `@@` hunk header（StructuredDiff.tsx:65）。行号 gutter 通过 `computeGutterWidth()` 动态计算宽度（StructuredDiff.tsx:46-49），使用 `RawAnsi` 组件进行双列渲染（gutter + content）（StructuredDiff.tsx:148-177）。文件名通过 `firstLine` 和 `filePath` 传递给 ColorDiff（StructuredDiff.tsx:65,101）。边框样式为 `borderStyle="dashed"`（FileEditToolDiff.tsx:98）。支持 `dim` 参数控制整体 dim（StructuredDiff.tsx:99）。

---

## 3. 覆盖层（C 组）

### C1. Permission 对话框

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 边框样式 | `borderStyle="round"` + `borderLeft={false}` + `borderRight={false}` + `borderBottom={false}` + `borderColor={color}`（PermissionDialog.tsx:62） | `Block::bordered()` + `BorderType::Plain`（dialogs.rs:307-308） | 🔴 | M |
| 标题行 | `Tool use` 黄色粗体（FallbackPermissionRequest.tsx:323 → PermissionRequestTitle.tsx:23） | `[!] Tool Permission` 黄色粗体（dialogs.rs:273） | 🔴 | S |
| 选项高亮 | 无选项列表（仅 Y/N/A 提示） | 无选项列表（仅 Y/N/A 提示） | ✅ | S |
| 选项分隔符 | 无选项分隔符 | 无选项分隔符 | ✅ | S |
| footer 提示行 | `Y=once  A=session  Shift+A=project  N=deny  Esc=cancel`（dialogs.rs:296） | `Y=once  A=session  Shift+A=project  N=deny  Esc=cancel`（dialogs.rs:296） | ✅ | S |
| 多选框 | 无多选框（单选 Y/N/A） | 无多选框（单选 Y/N/A） | ✅ | S |

### C2. Plan 审批对话框

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 边框样式 | `borderStyle="round" borderColor="planMode" borderLeft={false} borderRight={false} borderBottom={false}`（ExitPlanModePermissionRequest.tsx:535） | `Block::bordered()` + `BorderType::Plain`（dialogs.rs:499-500） | ✅ | M |
| 标题行 | `[~] Ready to code?` 蓝色粗体（dialogs.rs:470） | `[~] Ready to code?` 蓝色粗体（dialogs.rs:470） | ✅ | S |
| 选项高亮 | `figures.pointer`（`▸`）+ 颜色高亮（CustomSelect/select.tsx:522） | `▸` 前缀 + `selected_fg` + `selected_bg` 反白（dialogs.rs:480-482） | ✅ | S |
| 选项分隔符 | 空行分隔（CustomSelect/select.tsx:403,450） | 空行分隔（dialogs.rs:477,491） | ✅ | S |
| footer 提示行 | `↑↓ select · Enter confirm · A=auto Y=manual N=keep planning · Esc=keep planning`（dialogs.rs:492-494） | `↑↓ select · Enter confirm · A=auto Y=manual N=keep planning · Esc=keep planning`（dialogs.rs:492-494） | ✅ | S |
| 多选框 | 无多选框（单选 A/Y/N） | 无多选框（单选 A/Y/N） | ✅ | S |

### C3. AskQuestion 对话框

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 边框样式 | `borderTop={true} borderColor="inactive"`（SubmitQuestionsView.tsx:135） | `Block::bordered()` + `BorderType::Plain`（dialogs.rs:437-438） | 🔴 | M |
| 标题行 | `[?] Question` 蓝色粗体（dialogs.rs:396） | `[?] Question` 蓝色粗体（dialogs.rs:396） | ✅ | S |
| 选项高亮 | `figures.pointer` + 颜色高亮（CustomSelect/select.tsx:522） | `▸` 前缀 + `selected_fg` + `selected_bg` 反白（dialogs.rs:409-412） | ✅ | S |
| 选项分隔符 | 空行分隔（CustomSelect/select.tsx:403） | 空行分隔（dialogs.rs:404,422） | ✅ | S |
| footer 提示行 | `↑↓ select · Enter confirm · or type a custom answer · Esc skip`（dialogs.rs:427） | `↑↓ select · Enter confirm · or type a custom answer · Esc skip`（dialogs.rs:427） | ✅ | S |
| 多选框 | 无多选框（单选或自由输入） | 无多选框（单选或自由输入） | ✅ | S |

### C4. Confirm 对话框

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 标题行 | `⚠ Confirm` 黄色粗体（dialogs.rs:334） | `⚠ Confirm` 黄色粗体（dialogs.rs:334） | ✅ | S |
| 选项高亮 | 无选项列表（仅 Y/N 提示） | 无选项列表（仅 Y/N 提示） | ✅ | S |
| 选项分隔符 | 无选项分隔符 | 无选项分隔符 | ✅ | S |
| footer 提示行 | `Y=confirm  N=cancel  Esc=cancel`（dialogs.rs:339） | `Y=confirm  N=cancel  Esc=cancel`（dialogs.rs:339） | ✅ | S |
| 多选框 | 无多选框（单选 Y/N） | 无多选框（单选 Y/N） | ✅ | S |

### C5. Slash 补全浮层

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 边框样式 | 无边框（CustomSelect 容器仅 `flexDirection: "column"`）（select.tsx:651） | `Block::default()` 无边框（dialogs.rs:99） | ✅ | S |
| 标题行 | `Commands` 标题（dialogs.rs:100） | `Commands` 标题（dialogs.rs:100） | ✅ | S |
| 选项高亮 | `figures.pointer` + `suggestion` 色（select.tsx:522） | `▸` 前缀 + `selected_fg` + `selected_bg` 反白（dialogs.rs:85-87） | ✅ | S |
| 选项分隔符 | 空行分隔（CustomSelect 默认布局） | 空行分隔（dialogs.rs:97） | ✅ | S |
| footer 提示行 | 无显式 footer 提示（CustomSelect 内嵌） | 无显式 footer 提示 | ✅ | S |
| 数字键直选提示 | `${i}.` 前缀（select.tsx:450） | 无数字键直选提示 | 🔴 | S |

### C6. @ 文件补全浮层

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 标题行 | `Files` 标题（dialogs.rs:536） | `Files` 标题（dialogs.rs:536） | ✅ | S |
| 选项高亮 | `figures.pointer` + 颜色高亮（CustomSelect/select.tsx:522） | `▸` 前缀 + `selected_fg` + `selected_bg` 反白（dialogs.rs:519-521） | ✅ | S |
| 选项分隔符 | 空行分隔（CustomSelect 默认布局） | 空行分隔（dialogs.rs:533） | ✅ | S |
| footer 提示行 | 无显式 footer 提示 | 无显式 footer 提示 | ✅ | S |
| 数字键直选提示 | `${i}.` 前缀（select.tsx:450） | 无数字键直选提示 | 🔴 | S |

### C7. 模型选择器

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 标题行 | `Model` 标题（dialogs.rs:135） | `Model` 标题（dialogs.rs:135） | ✅ | S |
| 选项高亮 | `figures.pointer` + `suggestion` 色（CustomSelect/select.tsx:522） | `▸` 前缀 + `selected_fg` + `selected_bg` 反白（dialogs.rs:119-121） | ✅ | S |
| 选项分隔符 | 空行分隔（CustomSelect 默认布局） | 空行分隔（dialogs.rs:133） | ✅ | S |
| footer 提示行 | 无显式 footer 提示 | 无显式 footer 提示 | ✅ | S |
| 当前标记 | `figures.tick`（✓）（select.tsx:522） | `✓` 符号（dialogs.rs:120,125） | ✅ | S |

### C8. 帮助页

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 边框样式 | `Pane` 组件无完整边框，仅有顶部 `Divider` 线（Pane.tsx:52） | `Block::default()` 无边框（dialogs.rs:233） | ✅ | S |
| 标题行 | `Help` 标题（dialogs.rs:234） | `Help` 标题（dialogs.rs:234） | ✅ | S |
| 分类标题 | `Editing` / `Navigation` / `Mode & Tools` / `Commands` 蓝色粗体（dialogs.rs:174,189,200,212） | `Editing` / `Navigation` / `Mode & Tools` / `Commands` 蓝色粗体（dialogs.rs:174,189,200,212） | ✅ | S |
| 按键格式 | `{:<14}` 左对齐 14 字符宽度（dialogs.rs:156） | `{:<14}` 左对齐 14 字符宽度（dialogs.rs:156） | ✅ | S |
| 选项分隔符 | 空行分隔（HelpV2.tsx:176,188,199,211） | 空行分隔（dialogs.rs:175,188,199,211） | ✅ | S |
| footer 提示行 | `Esc to close`（dialogs.rs:228） | `Esc to close`（dialogs.rs:228） | ✅ | S |

---

## 4. Markdown 与代码（D 组）

### D1. 行内格式（bold/italic/code）

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| Bold (`**text**`) | `chalk.bold()` 真粗体（markdown.ts:98-101） | `Modifier::BOLD` 真粗体（markdown.rs:503-518） | ✅ | S |
| Italic (`*text*`) | `chalk.italic()` 真斜体（markdown.ts:92-95） | `Modifier::ITALIC` 真斜体（markdown.rs:521-540） | ✅ | S |
| Inline code (`` `code` ``) | `color('permission')` Cyan 色（markdown.ts:88-91） | `Color::Cyan`（markdown.rs:478-500） | ✅ | S |
| 链接下划线 | 无显式下划线（ hyperlink.ts 处理） | `Modifier::UNDERLINED`（markdown.rs:563-568） | 🟡 | S |

### D2. 标题 H1-H6

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| H1 样式 | `bold.italic.underline`（markdown.ts:106-115） | `BOLD` + `Color::White`（markdown.rs:84-87） | 🔴 | S |
| H2 样式 | `bold`（markdown.ts:116-124） | `BOLD` + `Color::White`（markdown.rs:79-83） | 🟡 | S |
| H3 样式 | `underline`（markdown.ts:125-133） | `BOLD` + `Color::White`（markdown.rs:74-78） | 🔴 | S |
| H4-H6 样式 | 无特殊样式（默认文本） | 无 H4-H6 实现 | 🔴 | M |
| 标题前缀/序号 | 无前缀序号 | 无前缀序号 | ✅ | S |

### D3. 列表（无序/有序/嵌套/checkbox）

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 无序列表标记 | `-`（markdown.ts:202） | `-`（markdown.rs:91-96） | ✅ | S |
| 有序列表标记 | 数字 + `.`（markdown.ts:169,347-358） | 固定 `1.`（markdown.rs:99-106） | 🔴 | M |
| 嵌套缩进 | `'  '.repeat(listDepth)` 空格缩进（markdown.ts:180） | 无嵌套列表实现 | 🔴 | M |
| checkbox 支持 | 无 checkbox 实现（原版 marked.js 可能支持） | 无 checkbox 实现 | 🔴 | M |
| checkbox 字符 | N/A | N/A | ✅ | S |

### D4. 代码块（围栏/语言标签/语法高亮/行号/背景）

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 代码块围栏 | `` ``` `` 标记（markdown.ts:72-90） | `` ``` `` 标记（markdown.rs:40-56） | ✅ | S |
| 语言标签位置 | 顶行 `` ```lang ``（markdown.ts:82） | 顶行 `` ```lang ``（markdown.rs:53） | ✅ | S |
| 语法高亮引擎 | `cliHighlight` + Shiki（HighlightedCode/Fallback.tsx:29,154） | `syntect`（markdown.rs:187-286,Cargo.toml:17） | 🟢 ADR 0003 | S |
| 语法高亮实现 | `hl.highlight(code, {language})`（Fallback.tsx:29） | `syntect::easy::HighlightLines::new()`（markdown.rs:249） | 🟢 ADR 0003 | S |
| 行号显示 | gutterWidth + CodeLine 组件（HighlightedCode.tsx:121,137-189） | 无行号列 | 🔴 | M |
| 代码块背景色 | 无背景（仅 ANSI 色彩） | 无背景（仅 ratatui Color） | ✅ | S |
| 边框样式 | 无边框（RawAnsi 直接渲染） | 无边框（仅 `"  "` 前缀缩进） | ✅ | S |

> **原版蓝图**（原版语法高亮实现）：
> 原版使用 Shiki 语法高亮库（`cliHighlight.ts`），通过 `getCliHighlightPromise()` 延迟加载（Markdown.tsx:106,HighlightedCode/Fallback.tsx:29）。代码块渲染优先使用 ColorFile Rust NAPI（`color-diff-n`），如果不可用则回退到 `HighlightedCodeFallback` 组件，该组件使用 `hl.highlight(code, {language})` 进行语法高亮（HighlightedCode.tsx:31,123;Fallback.tsx:29,154）。语言检测基于文件扩展名（Fallback.tsx:79-86），不支持的语言回退到 markdown 高亮（Fallback.tsx:145-152）。高亮结果为 ANSI 字符串，通过 `<Ansi>` 组件渲染（Fallback.tsx:185,90）。

### D5. 表格

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 边框字符 | `│ ─ ┬ ┼ ┌ ┐ └ ┘ ├ ┤`（MarkdownTable.tsx:227-237） | `│ ─ ├ ┤`（markdown.rs:392,420-424） | 🔴 | M |
| 表头样式 | `center` 对齐 + BOLD（MarkdownTable.tsx:217,403-407） | BOLD + `Color::White`（markdown.rs:403-407） | ✅ | S |
| 单元格对齐 | 支持 `left`/`center`/`right`（MarkdownTable.tsx:217-218） | 固定左对齐（markdown.rs:无对齐逻辑） | 🔴 | M |
| 列宽计算 | 动态分配 + min/ideal 宽度（MarkdownTable.tsx:108-156） | 固定 `max_col_width=40`（markdown.rs:371-374） | 🔴 | M |
| 文本换行 | `wrapAnsi` ANSI 感知换行（MarkdownTable.tsx:52-62） | `take(width)` 简单截断（markdown.rs:396-397） | 🔴 | M |
| 垂直格式 | 窄终端自动切换（MarkdownTable.tsx:183-288） | 无垂直格式 | 🔴 | L |

### D6. 引用块

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 前缀字符 | `▎` (U+258E)（figures.ts:34,markdown.ts:64） | 无引用块实现 | 🔴 | M |
| 文本样式 | `italic` + `dim` 前缀（markdown.ts:62-68） | 无引用块实现 | 🔴 | M |
| 多行处理 | 每行前缀 + split(EOL)（markdown.ts:65-69） | 无引用块实现 | 🔴 | M |

### D7. 链接

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| Inline 链接 `[text](url)` | `createHyperlink(url, text)`（markdown.ts:141-160） | Cyan + UNDERLINED（markdown.rs:542-574） | 🔴 | M |
| mailto 链接 | 提取 email 显示为纯文本（markdown.ts:143-146） | 无 mailto 特殊处理 | 🔴 | S |
| 自动链接 | 支持（markdown.ts:156-160） | 无自动链接 | 🔴 | S |
| URL 显示 | 隐藏 URL（仅显示文本）（hyperlink.ts） | 隐藏 URL（仅显示文本）（markdown.rs:563-568） | ✅ | S |

### D8. 水平分割线

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 字符 | `---`（markdown.ts:137-138） | 无水平分割线实现 | 🔴 | S |
| 样式 | 纯文本 `---` | 无实现 | 🔴 | S |

---

## 5. 颜色与主题（E 组）

### E1. 暗色主题调色板

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 主文本色 | `rgb(255,255,255)` "white"（utils/theme.ts:453） | `Color::White`（src/tui/theme.rs:47） | ✅ | S |
| 次要文本 | `rgb(80,80,80)` "dark gray"（utils/theme.ts:457） | `Color::DarkGray`（src/tui/theme.rs:48） | ✅ | S |
| accent 色（模型名、提示符、标题） | `rgb(177,185,249)` "light blue-purple"（utils/theme.ts:458） | `Color::Cyan`（src/tui/theme.rs:49） | 🟡 | S |
| warning 色（[!] 标记、破坏性操作） | `rgb(255,193,7)` "bright amber"（utils/theme.ts:463） | `Color::Yellow`（src/tui/theme.rs:50） | 🟡 | S |
| success 色（✓ 当前模型标记） | `rgb(78,186,101)` "bright green"（utils/theme.ts:461） | `Color::Green`（src/tui/theme.rs:51） | ✅ | S |

### E2. 亮色主题 parity

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 主文本色 | `rgb(0,0,0)` "black"（utils/theme.ts:128） | `Color::Black`（src/tui/theme.rs:64） | ✅ | S |
| 次要文本 | `rgb(175,175,175)` "light gray"（utils/theme.ts:132） | `Color::Gray`（src/tui/theme.rs:65） | 🟡 | S |
| accent 色 | `rgb(87,105,247)` "medium blue"（utils/theme.ts:133） | `Color::Blue`（src/tui/theme.rs:66） | 🟡 | S |
| warning 色 | `rgb(150,108,30)` "amber"（utils/theme.ts:138） | `Color::Red`（src/tui/theme.rs:67） | 🔴 | S |
| success 色 | `rgb(44,122,57)` "green"（utils/theme.ts:136） | `Color::Green`（src/tui/theme.rs:68） | ✅ | S |

### E3. 选中高亮

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 选中行前景色（暗色） | `rgb(0,0,0)` "black" via `selectionBg`（utils/theme.ts:491） | `Color::Black`（src/tui/theme.rs:52） | ✅ | S |
| 选中行背景色（暗色） | `rgb(38,79,120)` "selection blue"（utils/theme.ts:491） | `Color::White`（src/tui/theme.rs:53） | 🔴 | S |
| 选中行前景色（亮色） | `rgb(255,255,255)` "white"（utils/theme.ts:166） | `Color::White`（src/tui/theme.rs:69） | ✅ | S |
| 选中行背景色（亮色） | `rgb(180,213,255)` "selection blue"（utils/theme.ts:166） | `Color::Black`（src/tui/theme.rs:70） | 🔴 | S |

### E4. 次要文本 dim 处理

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| dim 原语（chalk） | `chalk.dim()`（ink/colorize.ts:77） | `Color::DarkGray`（src/tui/theme.rs:48） | 🟡 | S |
| dim 颜色值（暗色） | `rgb(80,80,80)` "dark gray" + dim（utils/theme.ts:457） | `Color::DarkGray`（src/tui/theme.rs:48） | ✅ | S |
| dim 颜色值（亮色） | `rgb(175,175,175)` "light gray" + dim（utils/theme.ts:132） | `Color::Gray`（src/tui/theme.rs:65） | 🟡 | S |
| `[end]`/`[error]` 标记 dim | `dimColor={true}` 传给 Text（SystemTextMessage.tsx:236-238） | `secondary_text` 色（src/tui/message_list.rs:237-238） | 🟡 | S |

### E5. 边框字符集

| 维度 | 原版 | codecoder | 状态 | 难度 |
|---|---|---|---|---|
| 对话框边框样式 | `borderStyle="round"` 使用 `─│┌┐└┘`（ink/render-border.ts via cli-boxes） | `BorderType::Plain` 使用 `─│┌┐└┘`（src/tui/dialogs.rs:308,500） | 🔴 | S |
| 单线边框字符 | `─│┌┐└┘├┤`（cli-boxes "single"） | `─│┌┐└┘`（ratatui BorderType::Plain） | 🔴 | S |
| 虚线边框字符 | `╌╎`（ink/render-border.ts:17-22 CUSTOM_BORDER_STYLES.dashed） | 无虚线实现 | 🔴 | M |
| 圆角边框字符 | `╭╮╰╯`（cli-boxes "round"） | 无圆角边框类型 | 🔴 | M |
| 双线边框字符 | `══║╗╝╚╔`（cli-boxes "double"） | 无双线边框类型 | 🔴 | M |

---

## 附录 A — 渲染代码映射图

| codecoder 文件 | 原版对应组件 | 备注 |
|---|---|---|
| `src/tui/mod.rs` | `FullscreenLayout.tsx` | 整体布局（三段 Flex 布局） |
| `src/tui/status_bar.rs` | `StatusLine.tsx`、`CoordinatorAgentStatus.tsx` | 状态栏（模型名、token 显示、CWD、context 进度条、spinner） |
| `src/tui/input_area.rs` | `PromptInput.tsx`、`TextInput.tsx`、`usePromptInputPlaceholder.ts`、`ShimmeredInput.tsx` | 输入区（prompt 符号、占位文字、分隔线、边框、多行渲染） |
| `src/tui/message_list.rs` | `ScrollBox.tsx`、`Messages.tsx`、`MessageRow.tsx`、`UserPromptMessage.tsx`、`AssistantTextMessage.tsx`、`SystemTextMessage.tsx`、`AssistantToolUseMessage.tsx`、`ToolUseLoader.tsx`、`HighlightedThinkingText.tsx`、`ThinkingToggle.tsx` | 消息列表容器与所有消息变体（User、Assistant、System、Tool、Reasoning） |
| `src/tui/dialogs.rs` | `PermissionDialog.tsx`、`FallbackPermissionRequest.tsx`、`PermissionRequestTitle.tsx`、`ExitPlanModePermissionRequest.tsx`、`SubmitQuestionsView.tsx`、`CustomSelect/select.tsx`、`HelpV2.tsx`、`Pane.tsx`、`ink/render-border.ts` | 所有对话框与补全浮层（Permission、Plan、AskQuestion、Confirm、Slash、@文件、模型选择、帮助） |
| `src/tui/markdown.rs` | `Markdown.tsx`、`MarkdownTable.tsx`、`HighlightedCode.tsx`、`HighlightedCode/Fallback.tsx`、`StructuredDiff.tsx`、`FileEditToolDiff.tsx`、`colorDiff.ts`、`markdown.ts`、`hyperlink.ts`、`cliHighlight.ts` | Markdown 与代码渲染（行内格式、标题、列表、代码块、表格、引用块、链接、Diff） |
| `src/tui/completion.rs` | 无对应（codecoder 自创） | 补全系统（无原版对比） |
| `src/tui/theme.rs` | `theme.ts`、`ink/colorize.ts`、`utils/theme.ts` | 颜色与主题（暗色/亮色调色板、选中高亮、dim 处理） |

---

## 附录 B — 待补条目

- **A2 状态栏 spinner 字符**：`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` — codecoder 实现匹配（src/tui/status_bar.rs:19），原版锚点无法在 archived/claude-code/src/components/ 中定位（StatusLine.tsx 和 CoordinatorAgentStatus.tsx 均不包含此字符数组）。该 spinner 字符序列为标准 Unicode Braille Patterns，推测可能在原版 npm 依赖或运行时生成，本轮无法定位源码锚点。

除上述条目外，本轮审计其他所有条目均已定位双侧锚点（codecoder TUI 文件与原版组件均有明确行号引用）。

---

## 裁决清单（按优先级）

**先修（视觉 bug / 显著差距且难度 S/M）**
1. 输入区.占位文字 — 缺少动态占位提示（§A3）难度 M
2. 输入区.边框样式 — 缺少 round 边框（§A3）难度 M
3. 输入区.边框颜色 — 缺少 promptBorder 颜色（§A3）难度 M
4. 消息列表容器.容器边框 — 上下边框与原版无边框不符（§A4）难度 S
5. User 消息.前缀符 — 原版无前缀，codecoder 有 `▶`（§B1）难度 S
6. User 消息.文本换行 — 无显式 wrap 配置（§B1）难度 M
7. User 消息.背景色 — 缺少 userMessageBackground（§B1）难度 M
8. User 消息.上边距 — 无上边距（§B1）难度 S
9. User 消息.截断机制 — 无 MAX_DISPLAY_CHARS 截断（§B1）难度 M
10. Assistant 消息.前缀符 — 原版无前缀，codecoder 有 `▷`（§B2）难度 S
11. Assistant 消息.选中态背景 — 缺少 messageActionsBackground（§B2）难度 M
12. Assistant 消息.段落间距 — 无上边距（§B2）难度 S
13. Assistant 消息.空消息过滤 — 无 isEmptyMessageText 检查（§B2）难度 S
14. Assistant 消息.错误消息颜色 — 缺少 error 专用颜色（§B2）难度 M
15. System 消息.前缀符 — 缺少 BLACK_CIRCLE (●)（§B3）难度 S
16. System 消息.info 级别颜色 — 缺少 dim 灰色处理（§B3）难度 S
17. System 消息.warning 级别颜色 — 缺少 warning 黄色（§B3）难度 M
18. System 消息.error 级别颜色 — 缺少 error 红色（§B3）难度 M
19. System 消息.TEARDROP_ASTERISK 前缀 — 缺少 `✵` 前缀（§B3）难度 M
20. System 消息.上边距 — 无上边距（§B3）难度 S
21. System 消息.背景色 — 无背景色响应选中态（§B3）难度 M
22. System 消息.宽度 — 无宽度限制（§B3）难度 S
23. System 消息.info 级别隐藏 — 无 verbose 条件隐藏（§B3）难度 S
24. Tool 调用.header 前缀 — 使用 `⚙` 而非 BLACK_CIRCLE（§B4）难度 S
25. Tool 调用.header 名称色 — 使用 secondary 色而非 bold（§B4）难度 S
26. Tool 调用.input 显示 — 未格式化直接渲染（§B4）难度 M
27. Tool 调用.状态图标-queued — 无 BLACK_CIRCLE dim（§B4）难度 M
28. Tool 调用.状态图标-in-progress — 无 ToolUseLoader 动画（§B4）难度 M
29. Tool 调用.状态图标-error — 无 isError 传给 loader（§B4）难度 M
30. Tool 调用.queued 提示 — 无 renderToolUseQueuedMessage（§B4）难度 M
31. Tool 调用.工具名背景色 — 缺少 userFacingToolNameBackgroundColor（§B4）难度 M
32. Tool 调用.工具名色 — 缺少 inverseText 反色（§B4）难度 M
33. Tool 调用.上边距 — 无上边距（§B4）难度 S
34. Tool 调用.背景色 — 无背景色响应选中态（§B4）难度 M
35. Reasoning.前缀符（首行） — 使用 `·` 而非 figures.pointer（§B5）难度 S
36. Reasoning.文本颜色 — 使用 secondary 色而非 text 色（§B5）难度 S
37. Reasoning.多行缩进 — 有缩进而原版无（§B5）难度 S
38. Reasoning.brief 模式布局 — 无 brief 模式（§B5）难度 M
39. Diff 渲染.上下文行 — 使用 DarkGray 而非 dim（§B6）难度 S
40. Diff 渲染.行号显示 — 缺少 gutter 列（§B6）难度 M
41. Diff 渲染.gutter 宽度 — 无 computeGutterWidth（§B6）难度 M
42. Diff 渲染.边框 — 缺少 dashed 边框（§B6）难度 M
43. Diff 渲染.文件名显示 — 无文件名显示（§B6）难度 M
44. Diff 渲染.dim 支持 — 无 dim 参数（§B6）难度 M
45. Permission 对话框.边框样式 — 使用 Plain 而非 round（§C1）难度 M
46. Permission 对话框.标题行 — 标题格式 `[!] Tool Permission`（§C1）难度 S
47. AskQuestion 对话框.边框样式 — 使用 Plain 而无 borderTop（§C3）难度 M
48. Slash 补全浮层.数字键直选提示 — 缺少 `${i}.` 前缀（§C5）难度 S
49. @ 文件补全浮层.数字键直选提示 — 缺少 `${i}.` 前缀（§C6）难度 S
50. Markdown 标题.H1 样式 — 缺少 italic/underline（§D2）难度 S
51. Markdown 标题.H3 样式 — 缺少 underline（§D2）难度 S
52. Markdown 标题.H4-H6 样式 — 无 H4-H6 实现（§D2）难度 M
53. 列表.有序列表标记 — 固定 `1.` 而非递增数字（§D3）难度 M
54. 列表.嵌套缩进 — 无嵌套列表（§D3）难度 M
55. 列表.checkbox 支持 — 无 checkbox（§D3）难度 M
56. 代码块.行号显示 — 缺少 gutter（§D4）难度 M
57. 表格.边框字符 — 缺少完整集合（§D5）难度 M
58. 表格.单元格对齐 — 固定左对齐（§D5）难度 M
59. 表格.列宽计算 — 固定 max_col_width=40（§D5）难度 M
60. 表格.文本换行 — 简单截断而非 ANSI 感知换行（§D5）难度 M
61. 引用块.前缀字符 — 无引用块实现（§D6）难度 M
62. 引用块.文本样式 — 无 italic+dim（§D6）难度 M
63. 引用块.多行处理 — 无引用块实现（§D6）难度 M
64. 链接.Inline 链接 — 使用 Cyan+UNDERLINED 而非 hyperlink（§D7）难度 M
65. 链接.mailto 链接 — 无 mailto 处理（§D7）难度 S
66. 链接.自动链接 — 无自动链接（§D7）难度 S
67. 水平分割线.字符 — 无水平分割线实现（§D8）难度 S
68. 暗色主题.accent 色 — 使用 Cyan 而非 light blue-purple（§E1）难度 S
69. 暗色主题.warning 色 — 使用 Yellow 而非 bright amber（§E1）难度 S
70. 亮色主题.次要文本 — 使用 Gray 而非 light gray（§E2）难度 S
71. 亮色主题.accent 色 — 使用 Blue 而非 medium blue（§E2）难度 S
72. 亮色主题.warning 色 — 使用 Red 而非 amber（§E2）难度 S
73. 选中高亮.选中行背景色（暗色） — 使用 White 而非 selection blue（§E3）难度 S
74. 选中高亮.选中行背景色（亮色） — 使用 Black 而非 selection blue（§E3）难度 S
75. 边框字符集.对话框边框样式 — 使用 Plain 而非 round（§E5）难度 S
76. 边框字符集.单线边框字符 — 缺少 `├┤`（§E5）难度 S
77. 边框字符集.虚线边框字符 — 无虚线实现（§E5）难度 M
78. 边框字符集.圆角边框字符 — 无圆角边框（§E5）难度 M
79. 边框字符集.双线边框字符 — 无双线边框（§E5）难度 M

**再裁决（细微差距，逐条定性）**
80. 整体布局.内 padding — 原版 paddingX={2}，codecoder 无（§A1）建议：保留
81. 输入区.prompt 符号 — 原版无，codecoder 有 `> `（§A3）建议：回补
82. 输入区.上分隔线颜色 — 使用 secondary_text 而非 swarmBanner（§A3）建议：保留
83. 消息列表容器.容器边框类型 — 有 Plain 边框而原版无边框（§A4）建议：保留
84. User 消息.多行缩进 — 有两空格缩进（§B1）建议：保留
85. User 消息.文本颜色 — 使用 primary+BOLD（§B1）建议：保留
86. User 消息.右内边距 — 无 paddingRight（§B1）建议：保留
87. Assistant 消息.多行缩进 — 有两空格缩进（§B2）建议：保留
88. Assistant 消息.文本颜色 — 使用 accent 色前缀（§B2）建议：保留
89. Assistant 消息.选中态前缀色 — 使用 accent+BOLD（§B2）建议：保留
90. System 消息.`[end]` 标记 — 使用 secondary 色而非 dim（§B3）建议：保留
91. System 消息.`[error]` 标记 — 使用 secondary 色而非 dim（§B3）建议：保留
92. Tool 调用.多行缩进 — 有两空格缩进（§B4）建议：保留
93. Reasoning.前缀符颜色 — 使用 secondary 色（§B5）建议：保留
94. Reasoning.dim 处理 — 使用 secondary dim 色（§B5）建议：保留
95. Reasoning.背景色 — 无背景色（§B5）建议：保留
96. 次要文本 dim 处理.dim 原语 — 使用 Color::DarkGray（§E4）建议：保留
97. 次要文本 dim 处理.dim 颜色值（亮色） — 使用 Color::Gray（§E4）建议：保留
98. 次要文本 dim 处理.`[end]`/`[error]` 标记 dim — 使用 secondary_text（§E4）建议：保留
99. Markdown 行内格式.链接下划线 — 有 UNDERLINED（§D1）建议：保留

**有意偏离（ADR 背书，仅需确认是否仍认可）**
100. Markdown 与代码.语法高亮引擎 — 使用 syntect 而非 Shiki（ADR 0003 主题系统）
101. Markdown 与代码.语法高亮实现 — 使用 syntect::easy::HighlightLines（ADR 0003）

**缺失大件（L 难度，需另立 spec）**
102. 输入区.多行渲染 — 固定 2 行高度，原版 TextInput 支持多行（§A3）。建议下一轮单独 brainstorm：输入区多行扩展管线。
103. User 消息.thinking 高亮 — 无彩虹色高亮（§B1）。建议下一轮单独 brainstorm：thinking 触发词高亮管线。
104. Tool 调用.input 折叠 — 无折叠机制（§B4）。建议下一轮单独 brainstorm：Tool input 折叠交互。
105. Tool 调用.进度消息 — 无 HookProgressMessage（§B4）。建议下一轮单独 brainstorm：Tool 进度消息管线。
106. Reasoning.折叠态 — 无 ThinkingToggle 控制展开/折叠（§B5）。建议下一轮单独 brainstorm：Reasoning 折叠交互。
107. Reasoning.thinking 触发高亮 — 无彩虹色高亮（§B5）。建议下一轮单独 brainstorm：thinking 触发词高亮管线。
108. Reasoning.ultrathink 模式 — 无 ultrathink 模式（§B5）。建议下一轮单独 brainstorm：ultrathink 特殊渲染管线。
109. ✅ 已修：Diff 渲染.gutter 分栏 — 无 RawAnsi 双列渲染（§B6）。建议下一轮单独 brainstorm：Diff gutter 分栏渲染。
110. ✅ 已修：Diff 渲染.语法高亮 — 无 ColorDiff Rust NAPI 语法高亮（§B6）。建议下一轮单独 brainstorm：Diff 语法高亮管线。
111. 表格.垂直格式 — 窄终端自动切换（§D5）。建议下一轮单独 brainstorm：表格垂直格式渲染。
