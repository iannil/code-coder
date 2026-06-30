# Confirm Dialog Pattern

Status: accepted

Every operation that destroys or irreversibly overwrites user-visible state routes through a single `Dialog::Confirm { message, action: ConfirmAction }` variant. The dialog renders a Y/N prompt; on Y, the dispatcher matches on `ConfirmAction` (an enum of all known destructive actions) and executes the corresponding handler; on N or Esc, no state changes. Operations covered today: `/resume` with no argument (overwrites current messages), `/clear` and `Ctrl+L` (drops messages), message delete from browse mode (`d` key), and any future destructive action.

## Why

Audit found that `/resume` with no argument silently loaded the latest session, overwriting the current conversation with no confirmation. Combined with auto-save, the current conversation could be lost before the user realized what happened. `/clear` had the same property — one keystroke and the entire history is gone. Both are particularly dangerous because the user might type them as part of exploring commands (`/resume` to see what it does, `/clear` thinking it clears the input).

A unified `Confirm` variant beats per-operation ad-hoc checks because:
1. The set of destructive operations is small and known; enumerating them in `ConfirmAction` makes them grep-able and reviewable.
2. The dialog rendering, key handling, and Esc semantics already exist for other dialog variants; reusing them ensures consistent behavior.
3. Future destructive operations are forced through the same gate — there is no "easy" path to skipping the confirm.

## How to apply

- **To add a new destructive operation:** add a variant to `ConfirmAction`, add a match arm in the dialog handler, and route the user-facing trigger (command, key) to construct the dialog instead of executing directly.
- **An operation is destructive if** it (a) drops user-authored content (messages, drafts), (b) overwrites persisted state with no undo, or (c) sends irreversible external effects (email, deploy, payment). The first two are TUI-side; the third is an agent-tool concern.
- **"Don't ask again" is not supported in the first iteration.** Each invocation of a destructive command re-confirms. If users find this annoying for high-frequency operations (e.g., deleting many messages in browse mode), revisit then — but the safe default is to confirm every time.
- Confirm is **not** required for state changes that are local-recoverable: scrolling, mode switching, undo/redo, selection. These are reversible through the same or a simpler UI gesture.

## Considered Options

- **Per-operation ad-hoc confirm.** Rejected: scattered `if should_confirm() { dialog } else { do_it() }` blocks across the codebase; easy to forget when adding a new destructive op. The enum forces centralization.
- **Closures in the dialog (`on_confirm: Box<dyn FnOnce()>`).** Rejected: storing closures in app state is awkward in Rust (lifetime, `Send` requirements) and obscures what the dialog will actually do — `ConfirmAction` is inspectable and debuggable.
- **Change `/resume` to a selection list instead of confirming overwrite.** Rejected as the primary fix because it doesn't generalize to `/clear` and message-delete. May add the selection list as an enhancement on top of this pattern (the confirm still fires when the user picks a session different from current).

## Consequences

- Every destructive op adds one enum variant + one match arm. The cost is linear and bounded — acceptable.
- Tests must cover both Y and N paths for each variant; the enum makes this obvious from the type.
- This ADR interacts with [[0001-tui-keybinding-and-mode-semantics]]: Esc closes the confirm dialog without executing, consistent with Esc's "no-op / close overlay" semantic.
