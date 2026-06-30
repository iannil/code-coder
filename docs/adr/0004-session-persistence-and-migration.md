# Session Persistence & Migration

Status: accepted

Persisted sessions carry a `schema_version: u32` field. Loading a session deserializes first to `serde_json::Value`, then runs through a `migrate(from, to, value)` chain that applies version-to-version transforms (`migrate_v0_to_v1`, `migrate_v1_to_v2`, etc.) before re-deserializing into the current `Session` struct. Every field on `MessageItem` and `Session` is annotated `#[serde(default)]` so minor schema drift degrades gracefully. Session saves are debounced (~5s) and executed on a background thread; the TUI main loop never blocks on disk I/O during normal operation, and exit time blocks until all pending saves flush.

## Why

Two independent audit findings drive this ADR:

**Migration framework:** `MessageItem` is an enum with five variants and growing (Phase 7 self-evolution will add more). The previous design had no version field and no migration logic — adding a field or variant would silently break `serde_json::from_slice` on every existing session file, surfacing only as `Error loading session: ...` at the user level with the data effectively lost. The first time the schema changes, all saved sessions become unreadable.

**Async save:** `auto_save_session` was called on every `send_message` and every `AgentResponse::Text` completion, doing a full O(N) serialization + truncation pass + sync file write on the TUI main thread. On slow disks (mechanical, network, WSL2 cross-FS) this causes visible frame drops. The save itself isn't urgent — it just needs to happen before exit.

## How to apply

- **Adding a new `MessageItem` variant or changing a field type:** increment `Session::CURRENT_VERSION`, write a `migrate_vN_to_vN+1` function that transforms the `Value` representation, add a unit test that feeds in an old-shape JSON blob and asserts the migrated shape. The framework handles chaining.
- **Adding a new optional field:** annotate `#[serde(default)]`; old files load with the default. No version bump needed for additive, backward-compatible changes.
- **Removing a field:** bump version and write a migration that strips it from old `Value`s.
- **Save path:** `mark_dirty()` is called anywhere data changes; the main loop checks dirty + elapsed-since-last-mark once per frame and spawns save work to a background sender if the debounce window has passed. Exit joins the sender.

## Considered Options

- **Append-only JSONL with periodic compaction.** Rejected for now: faster incremental writes, but a much larger change to the on-disk format and significantly more complex readers (line-by-line + periodic compaction jobs). Revisit if session sizes grow beyond ~10MB or if save latency is still measurable after the debounce change.
- **Field-level `#[serde(default)]` only, no version field.** Rejected: handles additive changes but not removals, renames, or type changes. The first time a field needs to be renamed, the no-version approach breaks down. Adding the framework now is cheap; adding it later requires a flag-day migration.

## Consequences

- Migration functions must be deterministic and total (never panic, never lose data). Test each one with a representative old-shape fixture.
- The background save thread introduces a join dependency at exit — exit cannot complete until pending saves finish. This is intentional (data durability > fast exit) but means a hung save thread will hang exit. Mitigation: save thread has a hard timeout; on timeout, write to `codecoder.log` and exit anyway.
- `dark_mode`, `selected_msg`, search state, etc. are *not* part of the session — they're UI state, not conversation state. Only `messages`, `model`, `token_count`, and metadata persist. This is enforced by `build_session_from_app` reading only the conversation fields.
