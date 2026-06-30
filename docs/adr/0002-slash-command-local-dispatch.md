# Slash Command Local Dispatch

Status: accepted

In TUI mode, every input beginning with `/` is intercepted by a local dispatcher in `send_message` *before* anything is sent to the agent. The dispatcher matches the command against the known list (`/help`, `/exit`, `/clear`, `/reload`, `/session`, `/resume`, `/config`, `/mcp`, `/tools`, `/skills`, `/memory`, `/history`) and routes to a TUI-side handler. **Unknown slash commands are rejected with a `System` error message** rather than forwarded to the LLM.

## Why

Discovered during audit: `commands.rs` defined `handle_session_cmd`, `handle_resume_cmd`, `handle_config_cmd`, `handle_mcp_cmd` (~370 lines) but **none were ever called from the TUI runtime path** — they were dead code. The `input_area.rs` "pass through" comment admitted the dispatcher was never wired up. Every `/`-prefixed input in TUI mode fell through to `cmd_tx.send(ProcessMessage { text: "/xxx" })`, so typing `/tools` sent the literal string to the LLM, which hallucinated a response. The Help panel advertised a `/model` command that did not exist anywhere.

Two related problems collapse into one fix here:
1. Known commands must actually execute (route to local handlers).
2. Unknown commands must not silently leak to the LLM — that wastes tokens, time, and risks the model "doing something" based on a typo.

Filtering the completion list (see [[0001-tui-keybinding-and-mode-semantics]] Issue #7) is a separate concern from routing, but both live in the same dispatcher and are being added together.

Update (TUI fidelity audit): filtering was upgraded from prefix-only to **subsequence (fuzzy) matching with prefix matches ranked first** — e.g. `/cfg` now matches `/config` — to match the original Claude Code completion. `refresh_slash_completion` builds two tiers (prefix, then in-order subsequence) and stable-sorts prefix ahead. Command aliases (`/reset`,`/new` ≡ `/clear`; `/settings` ≡ `/config`) were added to the dispatcher to match the original's aliases.

## How to apply

- To add a new slash command: extend the dispatcher match, add to `SlashCompletionState` static list, and update Help. All three are required for the command to be discoverable.
- Never call `cmd_tx.send(ProcessMessage)` for any input starting with `/`. If a future feature genuinely wants "ask the LLM about a slash command", it must use a different prefix (e.g., `?` or no prefix) — `/` is permanently reserved for local commands.
- If a command needs the agent's cooperation (e.g., `/reload`), the dispatcher sends the appropriate `AgentCommand` variant; the LLM is never in the loop.

## Consequences

- Users cannot type `/some/natural/language/path` as a message and expect the LLM to see it — it will be rejected as unknown command. They must use a leading space, quote it, or use the `@` file-completion system. This is the right tradeoff (typo safety > flexibility).
- The REPL (`repl.rs`) has its own `handle_command` that duplicates much of this logic. The DRY option (extract shared dispatcher) was rejected as too costly given different state models (REPL uses `bus` + `cmd_tx` directly, TUI uses `TuiApp`). The two dispatchers will drift; accepted as the cost of two separate entry points.
- Help panel must stay in sync — `/model` removed or replaced with `/config set model` alias.
