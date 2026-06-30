# CodeCoder

自主 AI agent 系统 — 事件驱动，文件系统即自我。本文件是术语表，定义项目中那些「不读代码就会理解错」的关键词。**只列项目特有概念**，通用编程术语（timeout、callback、iterator 等）不在此列。

## Interaction & UI

**Mode**:
The TUI's current interaction context, governing how key presses are interpreted. Concrete modes: `INSERT` (normal input), `SEARCH` (Ctrl+F), `R-SEARCH` (Ctrl+R reverse search), `DIALOG` (permission/plan/ask overlay open), `HELP`, `MODEL` (model picker), `SLASH` (slash command popup), `BROWSE` (message-list browse after Up/Down on empty input). Exactly one mode is active per frame; the active mode is shown in the status bar.
_Avoid_: state, screen, view, panel.

**Dialog**:
A modal overlay that blocks the underlying UI and demands user input before any other action. Constructed as the `Dialog` enum (ToolPermission / PlanApproval / AskQuestion / Confirm). Only one dialog can be active at a time. While a dialog is open, the input box is in `DIALOG` mode.
_Avoid_: popup (use "popup" only for the non-blocking slash/file-completion lists), modal, window.

**Popup**:
A non-modal overlay that appears above the input area but does not block the rest of the UI — slash-completion list, file-completion list, model picker. Multiple popups cannot be active simultaneously, but a popup can be dismissed without effect (unlike a dialog).
_Avoid_: dialog, menu, dropdown.

**Overlay**:
Generic term covering both Dialog and Popup — anything rendered above the standard 3-zone layout (messages / input / status). When the docs say "Esc closes overlays", it means dialogs and popups alike.
_Avoid_: window, layer.

**Reasoning**:
A collapsible message variant (`MessageItem::Reasoning`) holding the LLM's chain-of-thought text. Rendered dimmed and collapsed by default; expanded via `Tab` ([[0001-tui-keybinding-and-mode-semantics]]). Distinct from `Assistant` (the final answer) and `System` (UI chrome).
_Avoid_: thinking, CoT, explanation, rationale.

**Frame**:
One render pass of the TUI main loop (~60fps). `frame_count: u64` is the monotonic counter used for animations (spinner, cursor blink). A frame reads the current `TuiApp` state and produces one terminal draw; it must not mutate app state.
_Avoid_: tick, refresh, repaint, iteration.

## Permissions

**Permission Scope**:
The durability of a permission grant, expressed as the `PermScope` enum: `Once` (re-prompts next time), `AlwaysThisSession` (no more prompts this session), `AlwaysThisProject` (persisted to codecoder.json). See [[0005-permission-scope-and-session-allowlist]].
_Avoid_: permission level, duration, persistence mode.

**Session Allowlist**:
The in-memory `HashSet<tool_name>` kept by the agent thread for the current session. Tools in this set skip the permission prompt entirely. Cleared when the process exits. Distinct from the project-scope allowlist persisted in codecoder.json.
_Avoid_: whitelist, allowed tools, permission cache.

## Persistence

**Session**:
A saved conversation: a JSON file under `sessions/` containing `messages`, `model`, `token_count`, metadata, and `schema_version`. Loaded via `/resume`. See [[0004-session-persistence-and-migration]].
_Avoid_: conversation, chat, history (history is the in-memory input buffer for Up/Down navigation — see below).

**History** (input history):
The in-memory `Vec<String>` of previously submitted user inputs, navigated via `Ctrl+Up`/`Ctrl+Down`. Not persisted. Distinct from Session.
_Avoid_: log, recents, message history.

## Code Conventions

**Slash Command**:
An input beginning with `/` that is intercepted by the local dispatcher in TUI mode and never forwarded to the LLM. See [[0002-slash-command-local-dispatch]]. Unknown commands produce a System error.
_Avoid_: command (too generic — use "shell command" or "agent command" for other meanings), macro.

**Prompt-Injecting Slash Command**:
A slash command that constructs an expanded prompt and forwards it via `AgentCommand::ProcessMessage`. ADR 0002's typo-safety invariant is preserved because the dispatcher's own expansion (not user-typed) is what reaches the LLM. The visible TUI message shows the raw `/cmd args` the user typed; the LLM sees the expanded prompt for that turn only. `/grill-me` is the first instance. See [[0007-prompt-injecting-slash-commands]].
_Avoid_: prompt command (ambiguous — could mean "command for prompts"), macro, template.

**Agent Command**:
A message sent from the TUI thread to the agent thread via the `cmd_tx` channel, typed as the `AgentCommand` enum (ProcessMessage, Shutdown, PermissionResponse, etc.). Distinct from slash commands (which are user-typed) and shell commands (which agent tools execute).
_Avoid_: command (unqualified), request, message.

**Theme**:
A struct (`Theme`) holding all color definitions used by the TUI, held by `TuiApp` and read by every render function. Swappable between `dark()` and `light()` constructors. See [[0003-central-theme-struct]].
_Avoid_: color scheme, palette, skin, style (style refers to a single `ratatui::style::Style` instance, not the global theme).
