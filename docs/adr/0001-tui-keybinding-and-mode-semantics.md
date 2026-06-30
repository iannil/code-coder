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
