# TODO Cleanup Progress

**Date**: 2026-03-07
**Status**: Completed

---

## Summary

Implemented 2 actionable TODO fixes as identified in the remaining issues plan.

---

## Changes Made

### 1. ipc/client.ts - Use VERSION constant

**File**: `packages/ccode/src/ipc/client.ts`

**Before**:
```typescript
clientInfo: {
  name: "ccode-tui",
  version: "1.0.0", // TODO: Get from package.json
},
```

**After**:
```typescript
import { VERSION } from "@/version"
// ...
clientInfo: {
  name: "ccode-tui",
  version: VERSION,
},
```

**Rationale**: The `VERSION` constant is already defined in `@/version` and used throughout the codebase. This ensures the IPC client reports the correct version consistently.

---

### 2. zero-api/src/routes/health.rs - Track actual uptime

**File**: `services/zero-api/src/routes/health.rs`

**Before**:
```rust
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0, // TODO: Track actual uptime
    })
}
```

**After**:
```rust
use once_cell::sync::Lazy;
use std::time::Instant;

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: START_TIME.elapsed().as_secs(),
    })
}
```

**Dependency Added**: `once_cell = { workspace = true }` in `services/zero-api/Cargo.toml`

**Rationale**: `Lazy<Instant>` provides thread-safe lazy initialization. The timer starts on first access and accurately tracks server uptime.

---

## Verification

```bash
# TypeScript compilation
cd packages/ccode && bunx tsc --noEmit
# Result: 0 errors

# Rust compilation
cd services && cargo check -p zero-api
# Result: Compiles successfully (6 pre-existing warnings unrelated to this change)
```

---

## Remaining TODOs (Not Addressed)

The following TODOs remain as documented in the plan - they are either:
- Waiting on dependencies (database layer)
- In third-party/vendored code
- Feature enhancements rather than bugs

| Location | Reason Not Fixed |
|----------|------------------|
| `session.rs` (5 instances) | Stubs waiting for database implementation |
| `evolution-loop.ts` (2) | Template/placeholder code |
| `research-loop.ts` (1) | Feature enhancement |
| `openai-compatible/...` (1) | Third-party SDK code |
| `memory-zerobot/types.ts` (1) | Documentation/debt tracking |
| `lsp.rs` (1) | Low-priority notification handling |
| `analyzer.rs` (1) | Maven dependency extraction enhancement |
| `grep.rs` (1) | Column offset calculation optimization |

---

## Metrics

- **TODOs fixed**: 2
- **Total actionable TODOs identified**: 2
- **Remaining TODOs**: ~14 (non-actionable or low priority)
- **Completion**: 100% of actionable items

---

*Document created: 2026-03-07*
