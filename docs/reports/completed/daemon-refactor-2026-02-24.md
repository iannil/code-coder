# Daemon Refactoring Report

**Date**: 2026-02-24
**Status**: Completed

## Summary

Refactored `zero-daemon` from a hybrid embedded/orchestrator mode to a pure process orchestrator architecture.

## Changes Made

### 1. daemon/mod.rs
- Enhanced `run_orchestrator` to include MCP initialization, heartbeat worker, and state file writer
- Deleted `run` function (embedded mode) - ~150 lines removed
- Deleted `has_supervised_channels` helper function

### 2. daemon/api.rs (New File)
- Created management HTTP API module with axum
- Endpoints: `/health`, `/status`, `/restart/:name`, `/stop/:name`, `/start/:name`
- Runs on port 4402 alongside the orchestrator

### 3. main.rs
- Removed `Gateway` command entirely (was redundant with zero-gateway service)
- Updated `Daemon` command to accept gateway/channels/workflow port arguments
- Changed to call `run_orchestrator` instead of `run`

### 4. gateway/mod.rs
- Deleted entire embedded gateway module (~375 lines removed)

### 5. lib.rs
- Removed `gateway` module export

### 6. ops.sh
- Removed `STANDALONE_RUST_SERVICES` variable
- Updated `ALL_SERVICES` to equal `CORE_SERVICES`
- Removed zero-gateway, zero-channels, zero-workflow from service functions
- Updated help text to reflect new architecture

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    zero-daemon (进程编排器)                       │
│  职责: spawn 子进程、健康检查、自动重启、MCP、Heartbeat           │
│                                                                  │
│  Management API:             管理的子进程:                        │
│  ┌─────────────┐            ┌──────────┐ ┌──────────┐ ┌────────┐│
│  │ :4402       │            │  zero-   │ │  zero-   │ │ zero-  ││
│  │ /health     │ ─spawn───→ │ gateway  │ │ channels │ │workflow││
│  │ /status     │            │  :4430   │ │  :4431   │ │ :4432  ││
│  │ /restart    │            └──────────┘ └──────────┘ └────────┘│
│  └─────────────┘                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## CLI Usage

```bash
# Start daemon with default ports
zero-cli daemon

# Custom ports
zero-cli daemon --gateway-port 4440 --channels-port 4441 --workflow-port 4442

# Via ops.sh
./ops.sh start zero-daemon
./ops.sh start all  # Same as ./ops.sh start
```

## Management API

```bash
# Health check
curl http://127.0.0.1:4402/health

# Detailed status
curl http://127.0.0.1:4402/status

# Restart a service
curl -X POST http://127.0.0.1:4402/restart/zero-channels
```

## Verification

```bash
# Build
cargo build --release -p zero-cli

# Check help
./services/target/release/zero-cli daemon --help

# Check port allocation
lsof -i :4402 -i :4430 -i :4431 -i :4432
```

## Benefits

1. **No Port Conflicts**: `./ops.sh start all` no longer causes conflicts
2. **Clear Architecture**: Single mode (process orchestrator) instead of dual-mode
3. **Better Fault Isolation**: Each service runs in its own process
4. **Programmatic Control**: Management API for service control
5. **Reduced Code**: ~525 lines of duplicate code removed
