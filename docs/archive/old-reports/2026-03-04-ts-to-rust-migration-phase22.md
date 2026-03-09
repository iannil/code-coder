# Phase 22: NAPI Bindings for Autonomous and Keyring Modules

**Date**: 2026-03-04
**Status**: ✅ Complete

## Overview

Added NAPI bindings to expose the autonomous (state machine + task queue) and keyring (credential manager + MCP auth store) modules from Rust to TypeScript.

## Changes Made

### Files Created

1. **`services/zero-core/src/napi/autonomous.rs`** (~580 lines)
   - State machine bindings (`StateMachineHandle`)
     - `create_state_machine()` - Factory function
     - `state()`, `category()`, `is_terminal()`, `is_recoverable()`
     - `transition()`, `force_transition()`
     - `history()`, `previous_state()`, `state_visit_count()`
     - `detect_loop()`, `reset()`
     - `time_in_current_state()`, `total_time_in_state()`
     - `serialize()`
   - Task queue bindings (`TaskQueueHandle`)
     - `create_task_queue()` - Factory function
     - `add_task()`, `add_task_with_deps()`
     - `get_task()`, `all_tasks()`, `runnable_tasks()`
     - `start_task()`, `complete_task()`, `fail_task()`
     - `skip_task()`, `block_task()`, `unblock_task()`, `retry_task()`
     - `stats()`, `is_complete()`, `has_failures()`
     - `failed_tasks()`, `task_chain()`, `clear()`
     - `serialize()`

2. **`services/zero-core/src/napi/keyring.rs`** (~350 lines)
   - Keyring manager bindings (`KeyringManagerHandle`)
     - `create_keyring_manager()`, `create_file_keyring_manager()` - Factory functions
     - `is_keyring_available()` - Global utility
     - `backend()`, `set()`, `get()`, `delete()`, `exists()`
   - Credential manager bindings (`CredentialManagerHandle`)
     - `create_credential_manager()`, `create_file_credential_manager()`
     - `store_api_key()`, `store_oauth()`, `store_login()`
     - `get()`, `delete()`, `find_by_service()`, `find_for_url()`
     - `list_ids()`
   - MCP auth store bindings (`McpAuthStoreHandle`)
     - `create_mcp_auth_store()`, `create_file_mcp_auth_store()`
     - `store_tokens()`, `get()`, `get_for_url()`
     - `is_expired()`, `update_tokens()`
     - `store_code_verifier()`, `store_oauth_state()`
     - `delete()`

### Files Modified

- **`services/zero-core/src/napi/mod.rs`** - Added `autonomous` and `keyring` module exports

## Technical Details

### Design Patterns Used

1. **Handle Pattern**: Long-lived objects wrapped in `Arc<Mutex<T>>` for thread-safe sharing:
   ```rust
   pub struct StateMachineHandle {
       inner: Arc<Mutex<RustStateMachine>>,
   }
   ```

2. **Type Conversion**: Rust enums converted via `#[napi(string_enum)]` for JavaScript interop:
   ```rust
   #[napi(string_enum)]
   pub enum AutonomousState { Idle, Planning, ... }
   ```

3. **Error Mapping**: Rust `Result` types automatically map to JavaScript exceptions:
   ```rust
   .map_err(|e| Error::from_reason(e.to_string()))
   ```

### NAPI Types Exposed

| Category | Rust Type | NAPI Type |
|----------|-----------|-----------|
| State Machine | `AutonomousState` | `AutonomousState` (string enum) |
| State Machine | `StateCategory` | `StateCategory` (string enum) |
| State Machine | `StateMetadata` | `NapiStateMetadata` (object) |
| Task Queue | `TaskPriority` | `TaskPriority` (string enum) |
| Task Queue | `TaskStatus` | `TaskStatus` (string enum) |
| Task Queue | `Task` | `NapiTask` (object) |
| Task Queue | `TaskQueueStats` | `NapiTaskQueueStats` (object) |
| Keyring | `KeyringBackend` | `KeyringBackend` (string enum) |
| Keyring | `Credential` | `NapiCredential` (object) |
| Keyring | `McpAuthEntry` | `NapiMcpAuthEntry` (object) |

## Verification

### Build Check
```bash
cargo build -p zero-core --features napi-bindings
# ✅ Build successful (5 pre-existing warnings)
```

### Test Results
```bash
cargo test -p zero-core --lib
# ✅ 243 tests passed

cargo test -p zero-core --lib autonomous
# ✅ 22 tests passed (state machine + task queue)

cargo test -p zero-core --lib keyring
# ✅ 5 tests passed (credential manager + MCP auth)
```

## TypeScript Usage Example

```typescript
import {
  createStateMachine,
  createTaskQueue,
  createCredentialManager,
  createMcpAuthStore,
  AutonomousState,
  TaskPriority
} from '@codecoder/core';

// State machine
const sm = createStateMachine();
console.log(sm.state()); // "idle"

const result = sm.transition(AutonomousState.Planning, "Start planning");
if (result.success) {
  console.log(`Transitioned to ${result.toState}`);
}

// Task queue
const queue = createTaskQueue("session-123");
const taskId = queue.addTask("Build feature", "Description", TaskPriority.High);
queue.startTask(taskId);
queue.completeTask(taskId);
console.log(queue.stats()); // { total: 1, completed: 1, ... }

// Credential manager
const creds = createCredentialManager();
creds.storeApiKey("github", "github", "ghp_xxx", ["*.github.com"]);

// MCP auth store
const mcpAuth = createMcpAuthStore();
mcpAuth.storeTokens("my-mcp", "access_token", "refresh_token", null, null, "https://mcp.example.com");
```

## Next Steps

1. Build the native Node.js module with `napi-cli`
2. Generate TypeScript type definitions
3. Integrate with `packages/core` TypeScript package
4. Write integration tests in TypeScript

## Migration Progress

| Phase | Description | Status |
|-------|-------------|--------|
| 17 | Vector operations unification | ✅ Complete |
| 18 | File search engine | ✅ Complete |
| 19 | Unified storage layer (SQLite) | ✅ Complete |
| 20 | State machine and task queue | ✅ Complete |
| 21 | Security enhancement (keyring) | ✅ Complete |
| **22** | **NAPI bindings (autonomous + keyring)** | **✅ Complete** |
