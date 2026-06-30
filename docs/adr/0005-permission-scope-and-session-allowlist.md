# Permission Scope & Session Allowlist

Status: accepted

`AgentCommand::PermissionResponse` carries a `scope: PermScope` enum with variants `Once | AlwaysThisSession | AlwaysThisProject`. The agent thread maintains a `session_allowed: HashSet<tool_name>` (and at project scope, persists to codecoder.json). When the permission check runs for a tool call, it consults the allowlist *before* invoking the `PermissionEngine`. The TUI's dialog "A" (always-allow) option actually persists for the session; previously it sent the same `allowed: true` as "Y" while displaying "will be allowed without prompting for this session" — a security-relevant lie.

## Why

Audit verified the prior implementation was non-functional: `PermissionEngine::new()` was called fresh on every check, no session state was retained, and the `PermissionResponse` message had no field to distinguish "yes once" from "yes always". The "A" branch in `dialogs.rs` produced UI text claiming persistent permission that the agent literally could not honor.

This is worse than missing functionality — it is a false sense of safety. A user who grants "always allow" for `read_file` may then mentally relax scrutiny on subsequent `read_file` prompts (assuming they're pre-approved), when in fact each prompt is independent. The fix must eliminate the false claim and replace it with real behavior.

## How to apply

- `PermScope::Once` → behavior identical to today's `allowed: true`. Default for the "Y" key.
- `PermScope::AlwaysThisSession` → agent inserts the tool name into `session_allowed`. Subsequent calls to the same tool skip the prompt entirely.
- `PermScope::AlwaysThisProject` → agent persists the allowlist to codecoder.json under a new `permissions.allowlist` section. Future sessions load it at startup.
- **Scope is per-tool-name, not per-(tool, input) pair.** This is a deliberate simplification: input-scoped allowlists (e.g., "allow `write_file` only under `src/`") are valuable but require a path-matching engine and risk false positives. Out of scope for this ADR; revisit via [[0006-confirm-dialog-pattern]] if needed.
- The allowlist is checked *before* `PermissionEngine`. If `PermissionEngine` returns `Denied`, the denial stands regardless of allowlist — the engine's deny rules are absolute safety rails.
- The dead `Dialog::PlanReview` variant is removed in the same change (it was never constructed; only `Dialog::PlanApproval` is real).

## Considered Options

- **Input-scoped allowlist** (`HashSet<(tool_name, normalized_input_prefix)>`). Rejected: requires a normalization scheme (paths, URLs, etc.), is easy to bypass with subtle input variations, and provides marginal value over per-tool scoping for the common cases. Revisit when there is a concrete need.
- **Remove the "A" option entirely, never implement allowlist.** Rejected: per-tool prompts for every `read_file` / `list_directory` in a long session is exhausting; users will either stop using risky tools or learn to hit Y without reading. A real allowlist with explicit scoping is the safer design.

## Consequences

- Users must understand the difference between session and project scope. The dialog UI must label these clearly (e.g., `[A] session  [Shift+A] project`) and Help must explain.
- Project-scope allowlist changes codecoder.json schema → coordinate with [[0004-session-persistence-and-migration]] for forward compatibility of the config file (separate from session schema, but same principle).
- If a tool is found to be compromised or buggy, the user needs a way to revoke allowlist entries: `/permissions` command to view and clear. This is a follow-up, not blocking.
