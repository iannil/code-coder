# TUI Keybinding & Mode Semantics

Status: accepted

The TUI assigns each key a single semantic role and forbids overloading. `Esc` never quits the application — it only cascades through and closes active overlays (reverse-search → search → slash-completion → model-picker → help → dialog → selected-msg → file-completion) and is a no-op when nothing is open. The application is exited through `Ctrl+Q` only; `Ctrl+C` interrupts the in-flight agent request when the agent is busy, otherwise it acts as `Ctrl+Q`. `Enter` always means "submit the current submittable thing" and never "guess what I mean" — it sends a message, accepts a `selected_msg` for editing, or no-ops. Folding/expanding collapsible blocks (Reasoning, ToolCall output) is done exclusively through `Tab`. History navigation is `Ctrl+Up`/`Ctrl+Down` (input-content agnostic); plain `Up`/`Down` moves the cursor inside the multi-line input or, when input is empty, enters message-browse mode. `Home`/`End` are cursor-to-line-edge (readline convention); scrolling the message list to top/bottom is `g`/`G` (vim convention).

## Why

Before this decision the keymap had four exit shortcuts (`Ctrl+Q/C/D/Esc`) all doing the same thing, plus `Esc` would silently quit when no overlay was open — a frequent cause of accidental data loss. `Enter` on empty input did five different things depending on hidden state (toggle reasoning, accept completion, jump-scroll, etc.), making its behavior unpredictable. `Up`/`Down` simultaneously meant history navigation, message selection, and (in code) cursor movement, with hidden priority rules based on whether input was empty. These are all mode-coupling pathologies where one key did too many things.

The unifying principle: one key, one semantic. Discoverability (#10), mode indication (#10), and the Help panel rewrite (#14) all rest on this — they cannot be coherent if the same key does radically different things in different states.

## How to apply

When adding a new interaction:
- Pick the key whose existing semantic matches the new action; do not overload.
- If no existing key fits, prefer a modifier combination (`Ctrl+`/`Alt+`/`Shift+`) over reusing a base key.
- Document the binding in the Help panel (#14) — undocumented keys effectively do not exist.
- Destructive operations (quit, clear, interrupt, delete) require an explicit modifier (`Ctrl+`) or a confirm dialog ([[0006-confirm-dialog-pattern]]); plain printable keys are reserved for safe, local-state actions.

## Considered Options

- **Three-key quit with confirmation dialog on third Esc press.** Rejected: confirmation dialogs on every accidental Esc is hostile UX, and the dialog itself becomes a new mode to manage.
- **Make `Esc` user-configurable.** Rejected for now: introducing a keymap config layer is a large change and conflates "this is the right default" with "let users remap". Revisit if real users push back; for now, lock the defaults.

## Consequences

- Users coming from CLIs where `Esc` exits (e.g., some REPLs) will need to relearn. Mitigation: Help panel lists `Ctrl+Q` for exit prominently.
- `Ctrl+C` no longer raises `SIGINT` to the codecoder process in TUI mode; the agent loop catches it as an interrupt. The terminal itself still receives `SIGINT` if crossterm's raw-mode `ISIG` handling is disabled — verify the crossterm version when implementing.
- Plugin/extension authors cannot overload `Esc` or `Enter` — they must layer new interactions through overlays or new modifier keys.

## Phase B: real mid-call cancellation (implemented)

The original Phase A sent `AgentCommand::Interrupt` but the agent's `handle_message` had no way to observe it — the in-flight LLM call ran to completion, the next message was just dropped from the queue. Phase B closes that gap:

- `handle_message` and `react_loop` accept a `cancel: Arc<AtomicBool>` parameter.
- The agent main loop declares one `Arc<AtomicBool>` per thread, resets it before each `ProcessMessage`, and shares the clone into `handle_message`.
- `AgentCommand::Interrupt` flips the flag.
- `react_loop` checks the flag at four points each round: (1) top of round, (2) after each LLM streaming delta, (3) after the response completes before tool calls, (4) before each tool call.
- When observed, `react_loop` returns `"[interrupted by user]"`, which flows back to the TUI as a normal `AgentResponse::Text` and clears `agent_busy`.

This is cooperative cancellation, not pre-emption — the current `await` still completes (one more LLM delta may arrive). But no further rounds, no tool calls, no self-evolution. The agent stays alive and ready for the next message. Sub-agents spawned via `ask_agent` get their own never-set token (the user's Ctrl+C applies to the parent request only).

## Kill-ring & undo/redo bindings (TUI fidelity audit)

Aligned the editor keys with the original Claude Code editor:

- `Ctrl+K` (kill to line end), `Ctrl+U` (kill to line start), `Ctrl+W` (kill word before) all feed a **kill-ring**; consecutive kills accumulate (K appends, U/W prepend).
- `Ctrl+Y` is **yank** (paste the kill-ring at the cursor) — it was previously bound to redo.
- Redo moved to `Ctrl+Shift+Z`; `Ctrl+Z` remains undo. `Ctrl+Z` does not suspend the process because raw mode disables `ISIG`; Claude's own editor leaves `Ctrl+Z` unbound, so this is additive rather than conflicting.
- `Ctrl+A/E` and `Home/End` are line-edge (current line) for multi-line input; `Ctrl/Alt+Left/Right` move by word; a trailing `\` before the cursor + `Enter` inserts a newline (line continuation).

The Plan approval dialog is a 3-option navigable list (↑↓ + Enter, or shortcuts `A`=auto-accept edits, `Y`=manually approve, `N`/`Esc`=keep planning). Choosing auto-accept adds `write_file`/`edit_file` to the session allowlist ([[0005-permission-scope-and-session-allowlist]]) so edits proceed without prompting for the rest of the session. The AskQuestion dialog is likewise a list (options + free-text custom answer) rather than a Y/N prompt.
