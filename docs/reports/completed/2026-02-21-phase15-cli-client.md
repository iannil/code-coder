# Phase 15: ZeroBot CLI Client Refactoring

**Date:** 2026-02-21
**Status:** In Progress (Foundation Complete)

## Overview

Phase 15 focuses on refactoring ZeroBot from a monolithic application to a lightweight CLI client that uses the Zero Service crates.

## Completed Work

### 1. Added Zero Service Dependencies

Updated `zero-bot/Cargo.toml` to depend on the new Zero Service crates:

```toml
[dependencies]
# Internal crates (Zero Services)
zero-common = { workspace = true }
zero-agent = { workspace = true }
zero-tools = { workspace = true }
zero-memory = { workspace = true }
```

### 2. Created HTTP Client Module

Created `zero-bot/src/client.rs` with HTTP client for Zero Services:

**Features:**
- `ZeroClient` struct with configurable endpoints
- Gateway API: `chat()`, `parallel_inference()`, `get_quota()`
- Channels API: `list_channels()`, `send_message()`, `channel_health()`
- Workflow API: `list_cron_jobs()`, `add_cron_job()`, `delete_cron_job()`
- Health check: `health_check()` for all services

**Endpoints:**
| Service | Default Port | Purpose |
|---------|--------------|---------|
| Gateway | 4404 | LLM routing, auth, quota |
| Channels | 4405 | Message channel management |
| Workflow | 4406 | Cron, git webhooks |

### 3. Updated lib.rs

- Added `pub mod client;`
- Added re-exports for Zero Service crates:
  ```rust
  pub use zero_agent;
  pub use zero_common;
  pub use zero_memory;
  pub use zero_tools;
  ```

## Test Results

- All 1,148 tests pass in zero-bot
- No regressions from adding new dependencies

## Architecture Changes

```
Before:
┌─────────────────────────────────────────────┐
│              zero-bot (monolith)            │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐       │
│  │providers│ │ memory  │ │  tools  │  ...  │
│  └─────────┘ └─────────┘ └─────────┘       │
└─────────────────────────────────────────────┘

After:
┌─────────────────────────────────────────────┐
│           zero-bot (CLI client)             │
│  ┌──────────────────────────────────┐       │
│  │           client.rs              │       │
│  │  HTTP calls to Zero Services     │       │
│  └──────────────────────────────────┘       │
│           │                                  │
│  ┌────────┴────────────────────────┐        │
│  │ Re-exports from Zero crates:    │        │
│  │ • zero_agent                    │        │
│  │ • zero_tools                    │        │
│  │ • zero_memory                   │        │
│  │ • zero_common                   │        │
│  └─────────────────────────────────┘        │
└─────────────────────────────────────────────┘
              │
              │ HTTP/Library calls
              ↓
┌─────────────────────────────────────────────┐
│           Zero Service Crates               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐    │
│  │zero-agent│ │zero-tools│ │zero-memory│   │
│  └──────────┘ └──────────┘ └──────────┘    │
└─────────────────────────────────────────────┘
```

## Remaining Work

### Future Iterations

1. **Gradual Code Migration** - Replace local implementations with calls to Zero crates
2. **Service Mode** - When Zero services are deployed, use HTTP client
3. **Legacy Removal** - After full migration, remove duplicated local modules:
   - `providers/` → use zero-gateway
   - `memory/` → use zero-memory
   - `agent/` → use zero-agent
   - `tools/` → use zero-tools

### Not Changed (Kept Local)

These modules remain in zero-bot as they're CLI-specific:
- `onboard/` - Setup wizard
- `config/` - Configuration loading
- `service/` - OS service management
- `doctor/` - Diagnostics
- `skills/` - Local skill management
- `credential/` - Credential management

## Files Modified

| File | Change |
|------|--------|
| `services/zero-bot/Cargo.toml` | Added zero-* dependencies |
| `services/zero-bot/src/lib.rs` | Added client module and re-exports |
| `services/zero-bot/src/client.rs` | Created HTTP client module |

## Verification

```bash
cargo build --package zero-bot    # ✅ Success
cargo test --package zero-bot     # ✅ 1,148 tests pass
```
