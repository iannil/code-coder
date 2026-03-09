# TypeScript to Rust Migration - Phase 4: IPC Bridging

**Date**: 2026-03-05
**Status**: ✅ Completed
**Duration**: ~30 minutes

## Overview

Implemented Phase 4 of the TypeScript to Rust migration plan - the IPC (Inter-Process Communication) bridging layer between `zero-cli` (Rust) and the TypeScript TUI.

## Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                        zero-cli (Rust Binary)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  CLI Parser  │  │   Session    │  │    Tools     │              │
│  │   (clap)     │  │   Manager    │  │   Executor   │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│                              ↕ IPC (Unix Socket / JSON-RPC)         │
└────────────────────────────────────────────────────────────────────┘
                              ↕
┌────────────────────────────────────────────────────────────────────┐
│                    ccode-tui (TypeScript/SolidJS)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  TUI Render  │  │  LLM Client  │  │  IPC Client  │              │
│  │  (OpenTUI)   │  │  (AI SDK)    │  │              │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└────────────────────────────────────────────────────────────────────┘
```

## Files Created

### Rust (services/zero-cli/src/ipc/)

| File | Purpose | Lines |
|------|---------|-------|
| `protocol.rs` | JSON-RPC 2.0 types, request/response definitions, method constants | ~350 |
| `server.rs` | Unix Domain Socket server, client handling, request routing | ~350 |
| `mod.rs` | Module exports and documentation | ~60 |

### TypeScript (packages/ccode/src/ipc/)

| File | Purpose | Lines |
|------|---------|-------|
| `types.ts` | Type definitions matching Rust protocol | ~250 |
| `protocol.ts` | JSON-RPC message framing and dispatch | ~200 |
| `client.ts` | High-level API with auto-connect and reconnect | ~300 |
| `index.ts` | Module exports and documentation | ~100 |

## Files Modified

| File | Change |
|------|--------|
| `services/zero-cli/src/main.rs` | Added `ipc` module, `serve-ipc` command |
| `services/zero-cli/src/lib.rs` | Added `ipc` module export |
| `services/zero-cli/Cargo.toml` | Added `tokio-util` dependency |

## Protocol Design

### JSON-RPC 2.0 Methods

**Requests (TUI → CLI)**:
- `ipc/initialize` - Initialize session and get tools
- `ipc/tool_call` - Execute a tool
- `ipc/get_session` - Get session history
- `ipc/list_sessions` - List all sessions
- `ipc/compact` - Compact session history
- `ipc/cancel_generation` - Cancel ongoing generation
- `ipc/ping` - Health check

**Notifications (CLI → TUI)**:
- `ipc/session_update` - Session state changed
- `ipc/tool_request` - Request tool execution
- `ipc/llm_request` - Request LLM completion
- `ipc/stream_token` - Stream token from LLM
- `ipc/error` - Error notification

### Socket Path

Default: `~/.codecoder/ipc.sock`

## Key Features

### Rust Server
- Multi-client support via `tokio::spawn`
- Broadcast channel for server-wide notifications
- Per-client message channels
- Graceful shutdown handling
- Tool execution integration

### TypeScript Client
- Auto-start CLI if not running
- Auto-reconnect on disconnect (exponential backoff)
- Type-safe event emitter
- Session state management
- Factory function for quick setup

## Usage

### Start IPC Server
```bash
zero-cli serve-ipc
zero-cli serve-ipc --socket /tmp/my-ipc.sock
```

### TypeScript Client
```typescript
import { createIpcClient } from "./ipc"

const client = await createIpcClient()
const session = await client.initialize({ cwd: process.cwd() })

// Execute tool
const result = await client.callTool("shell", { command: "ls -la" })

// Listen for streaming
client.on("stream_token", (n) => process.stdout.write(n.token))

await client.close()
```

## Verification

### Rust Build
```bash
cargo check -p zero-cli
# ✅ Compiles with warnings only (unused fields - expected for now)
```

### TypeScript Type Check
```bash
cd packages/ccode && bun run tsc --noEmit
# ✅ No errors
```

## Next Steps

1. **Integration Testing**: Create integration tests with actual CLI process
2. **TUI Refactor**: Update `packages/ccode/src/cli/cmd/tui/` to use IPC client
3. **LLM Bridge**: Implement `llm_request` handler in TUI for CLI→TUI LLM calls
4. **Session Persistence**: Add session resume after CLI restart
5. **Error Handling**: Improve error recovery and user feedback

## Dependencies Added

```toml
# services/zero-cli/Cargo.toml
tokio-util = { version = "0.7", default-features = false, features = ["codec"] }
```

## Notes

- The IPC protocol is intentionally similar to MCP for consistency
- Unix Domain Sockets are ~2-3x faster than TCP for local communication
- Named Pipes support for Windows can be added later with minimal changes
- The TypeScript client uses Node's `net.Socket` directly (no external deps)
