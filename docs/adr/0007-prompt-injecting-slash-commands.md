# Prompt-Injecting Slash Commands

Status: accepted

Slash commands that drive a multi-turn LLM flow — `/grill-me <topic>` is the first instance — construct an expanded prompt at dispatch time and forward it via `AgentCommand::ProcessMessage`. ADR 0002's invariant ("no user-typed `/`-prefixed input reaches the LLM") is preserved because the constructed prompt does not start with `/`; only the dispatcher's own expansion reaches the LLM.

## Why

`/grill-me` needs the LLM in the loop — it runs a relentless interview driven by model output. This breaks the assumption behind every prior slash command (`/reload`, `/clear`, `/config`, …), which is "execute locally, never consult the LLM". ADR 0002's blanket ban on `cmd_tx.send(ProcessMessage)` for `/`-prefixed inputs exists to prevent typo leakage (`/hlep` should not be paraphrased by the model); it was never intended to forbid the dispatcher from constructing and sending its own prompt.

Two designs were rejected:

1. **New `AgentCommand::RunPrompt { label, prompt }` variant.** Cleanest in principle — separates the user-facing label from the LLM-facing prompt. Rejected because it would duplicate the ~80-line permission-check closure that lives inside the `ProcessMessage` arm of the agent loop, with no easy extraction (the closure borrows `&cmd_rx` from the loop's stack). The duplication cost is not worth the labelling benefit.

2. **Fully TUI-side interview mode** (à la `ultraplan` in the original Claude Code). Rejected because the original `grill-me` / `grill-with-docs` skill is a prompt, not a state machine — the model drives the question sequence. A TUI-driven mode would diverge from the source behaviour we are replicating.

## How to apply

To add another prompt-injecting slash command (e.g., a hypothetical `/mentor-me`):

1. Add the command to `SlashCompletionState::default()` (commands + descriptions vectors must stay aligned).
2. Add a match arm in `dispatch_slash_command` that calls a `handle_<name>_cmd` helper.
3. The handler:
   - Parses args from the user's input.
   - Shows usage on missing args (no agent forward).
   - Pushes a `MessageItem::User { text: input.trim() }` so the TUI displays what the user typed — this is also what session save/resume preserves.
   - Builds the expanded prompt with a `build_<name>_prompt(args)` helper.
   - Sends `AgentCommand::ProcessMessage { text: expanded_prompt }`.
   - Mirrors `send_message`'s post-send state updates (`thinking_start_time`, `agent_busy`, `[send]` system marker).
4. Add a row to the help panel in `dialogs.rs::render_help_panel`.
5. Test: assert the user-typed slash input is what appears in `app.messages`, and the expanded prompt is what comes out of `cmd_tx`.

The dispatcher owns prompt construction. The agent loop is unmodified — it cannot distinguish a `/grill-me` expansion from a regular user message, which is exactly the point.

## Consequences

- `/resume` does not preserve the LLM context across sessions — agent history is in-memory only and is rebuilt fresh on every session start. This is the same as every other command. The visible TUI history shows `/grill-me <topic>`; if the user wants to continue the interview after resume, they type a new message.
- The TUI display and the LLM's first turn use different strings for `/grill-me`. This is intentional and matches Claude Code's behaviour for `prompt`-type commands.
- `dispatch_slash_command` still returns `true` for every `/`-prefixed input — `send_message` clears the input box on that signal, regardless of whether the dispatcher eventually forwards via `ProcessMessage`.
- ADR 0002's typo-safety invariant is intact: every other `/`-prefixed input still rejects unknown commands instead of leaking to the LLM.
