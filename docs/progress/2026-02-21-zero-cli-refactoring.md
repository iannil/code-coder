# Progress: Simplify zero-bot and Rename to zero-cli

**Date:** 2026-02-21
**Status:** Completed

## Overview

This document tracks the progress of simplifying `zero-bot` by removing duplicate implementations and renaming it to `zero-cli` to better reflect its role as the CLI entry point for the Zero services.

## Context

The `zero-bot` crate contained duplicate implementations of modules that already existed in workspace crates (`zero-tools`, `zero-memory`, `zero-common`, etc.). While `zero-bot/src/lib.rs` already re-exported these workspace crates, it also maintained its own duplicate implementations, leading to code redundancy and maintenance burden.

## Completed Work

### Phase 1: Migrate Unique Functionality to Workspace Crates

**Migrated to `zero-common/src/security/`:**
- `pairing.rs` - Gateway pairing mode with bearer token authentication

**Migrated to `zero-tools/src/`:**
- `memory_forget.rs` - Tool for deleting memory entries

**Kept in `zero-cli` (CLI-specific):**
- `hygiene.rs` - Memory file management (depends on CLI-specific config)
- `vault.rs` - Credential vault (depends on local secrets implementation)
- `auto_login.rs`, `browser_open.rs`, `skill_search.rs`, `registry.rs` - CLI-specific tools

### Phase 2: Delete Duplicate Modules

- Removed `zero-bot/src/security/pairing.rs` (now uses `zero_common::security::pairing`)
- Note: Full trait unification between `zero-cli` and workspace crates was not performed due to complexity. The local `Memory` and `Tool` traits remain separate from `zero_memory::Memory` and `zero_tools::Tool`.

### Phase 3: Rename zero-bot to zero-cli

**Directory Changes:**
- `services/zero-bot/` → `services/zero-cli/`

**Cargo.toml Changes:**
- Package name: `zero-bot` → `zero-cli`
- Description updated to reflect CLI focus
- Workspace members updated

**Source Code Changes:**
- CLI command name: `zero-bot` → `zero-cli`
- Environment variable: `ZERO_BOT_AUTOSTART_CHANNELS` → `ZERO_CLI_AUTOSTART_CHANNELS`
- User-facing messages updated (Status, Gateway, Daemon logs)
- Doc comments updated (`ZeroBot` → `Zero CLI`)
- Test imports updated (`zero_bot::` → `zero_cli::`)

### Phase 4: Verification

- `cargo build -p zero-cli` ✅
- `cargo test -p zero-cli` ✅ (all 1 doc test + 7 integration tests + 12 unit tests pass)
- `cargo build --workspace` ✅

## Key Decisions

1. **Conservative Migration**: Only migrated modules with clean trait-based dependencies. Modules with CLI-specific runtime context (config, security policy) remain in `zero-cli`.

2. **Trait Separation**: The local `Memory` and `Tool` traits in `zero-cli` remain separate from the workspace crate versions. Full unification would require significant refactoring and is deferred to a future phase.

3. **Naming Convention**: Chose `zero-cli` over alternatives to clearly indicate this is the command-line interface entry point for the Zero services ecosystem.

## Files Changed

### Created
- `services/zero-common/src/security/pairing.rs`
- `services/zero-tools/src/memory_forget.rs`

### Modified
- `services/zero-common/src/security/mod.rs` - Added pairing re-export
- `services/zero-tools/src/lib.rs` - Added memory_forget module
- `services/zero-cli/Cargo.toml` - Renamed package
- `services/zero-cli/src/lib.rs` - Updated MigrateCommands doc
- `services/zero-cli/src/main.rs` - Updated CLI name and messages
- `services/zero-cli/src/security/mod.rs` - Re-export pairing from zero_common
- `services/zero-cli/src/tools/mod.rs` - Updated imports
- `services/zero-cli/src/util.rs` - Updated doc test
- `services/zero-cli/tests/memory_comparison.rs` - Updated import
- `services/Cargo.toml` - Updated workspace members

### Deleted
- `services/zero-bot/src/security/pairing.rs` (moved to zero-common)

## Future Work

1. **Trait Unification**: Consider unifying `zero-cli` local traits with workspace crate traits to eliminate remaining duplication.

2. **Full Module Migration**: Migrate remaining duplicate modules (`channels`, `stt`, `tts`, `providers`) once trait unification is complete.

3. **Documentation Updates**: Update CLAUDE.md and other documentation to reflect the rename.
