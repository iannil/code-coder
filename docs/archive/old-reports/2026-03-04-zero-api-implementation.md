# Zero-API Service Implementation

**Date**: 2026-03-04
**Status**: ✅ Complete (Phase 1)

## Summary

Created `services/zero-api/`, a new Rust HTTP/WebSocket service that exposes zero-core functionality via REST API.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     zero-api (:4402)                        │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   /health    │  │ /api/v1/...  │  │     /ws      │      │
│  │  (health)    │  │  (REST API)  │  │ (WebSocket)  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   AppState                          │   │
│  │  - Grep, Reader, Writer, Editor, Ls                 │   │
│  │  - CodeSearch, WebFetcher, Truncator                │   │
│  │  - TodoLists (session-scoped)                       │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
└───────────────────────────┼─────────────────────────────────┘
                            │
                            ▼
               ┌────────────────────────┐
               │       zero-core        │
               │  (Rust library)        │
               └────────────────────────┘
```

## API Endpoints

### Health
- `GET /health` - Health check

### Tools API
- `GET /api/v1/tools` - List available tools
- `POST /api/v1/tools/:tool` - Execute a tool

### Session API
- `GET /api/v1/session` - List sessions
- `POST /api/v1/session` - Create session
- `GET /api/v1/session/:id` - Get session
- `GET /api/v1/session/:id/messages` - Get messages
- `POST /api/v1/session/:id/messages` - Add message

### MCP API
- `GET /api/v1/mcp/tools` - List MCP tools
- `POST /api/v1/mcp/call` - Call MCP tool

### WebSocket
- `GET /ws` - WebSocket connection for real-time streaming

## Files Created

```
services/zero-api/
├── Cargo.toml
└── src/
    ├── main.rs         # Entry point, router setup
    ├── state.rs        # Shared application state
    └── routes/
        ├── mod.rs      # Routes module
        ├── health.rs   # Health check
        ├── tools.rs    # Tools API
        ├── session.rs  # Session API
        ├── mcp.rs      # MCP API
        └── ws.rs       # WebSocket handler
```

## Available Tools (via API)

1. `grep` - Regex content search
2. `glob` - File pattern matching
3. `read` - File reading
4. `write` - File writing
5. `ls` - Directory listing
6. `codesearch` - Semantic code search
7. `webfetch` - HTTP fetching
8. `truncate` - Output truncation

## Usage

```bash
# Start the server
cargo run -p zero-api

# Or with custom port
ZERO_API_PORT=8080 cargo run -p zero-api

# Example API calls
curl http://localhost:4402/health
curl -X POST http://localhost:4402/api/v1/tools/grep \
  -H "Content-Type: application/json" \
  -d '{"params": {"pattern": "fn main", "path": "."}}'
```

## Next Steps

1. Add persistent session storage (SQLite)
2. Implement authentication/authorization
3. Add streaming support for long-running operations
4. Create TypeScript API client in packages/ccode
5. Integrate with existing services (zero-gateway)

## Verification

```bash
# Build
cargo build -p zero-api  # ✓ Success

# Check
cargo check -p zero-api  # ✓ Success (6 warnings)
```
