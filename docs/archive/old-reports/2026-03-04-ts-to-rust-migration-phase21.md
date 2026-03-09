# Phase 21: Security Enhancement with System Keyring

**Date**: 2026-03-04
**Status**: Completed

## Summary

Implemented system keyring integration for secure credential storage. The implementation supports macOS Keychain, Linux Secret Service (via D-Bus), and Windows Credential Manager through the `keyring` crate, with automatic fallback to encrypted file storage when system keyring is unavailable.

## Implementation

### Keyring Manager (`services/zero-core/src/security/keyring.rs`)

Created a comprehensive keyring integration module (~600 lines) that provides:

**Core Components:**

1. **KeyringManager**: Low-level interface to system keyring
   - Automatic backend selection (system keyring vs file fallback)
   - Cross-platform support via `keyring` crate
   - Encrypted file fallback with JSON storage

2. **CredentialManager**: High-level credential management
   - Structured credential types (API keys, OAuth, Login)
   - URL pattern matching for automatic credential injection
   - Service-based lookup with indexing

3. **McpAuthStore**: Specialized MCP OAuth storage
   - Token storage with URL validation
   - PKCE code verifier support
   - OAuth state management
   - Token expiration checking

### Credential Types

```rust
pub enum Credential {
    /// API key or bearer token
    ApiKey {
        service: String,
        key: String,
        patterns: Vec<String>,
    },
    /// OAuth credentials
    OAuth {
        service: String,
        client_id: String,
        client_secret: Option<String>,
        access_token: Option<String>,
        refresh_token: Option<String>,
        expires_at: Option<i64>,
        scope: Option<String>,
        patterns: Vec<String>,
    },
    /// Login credentials
    Login {
        service: String,
        username: String,
        password: String,
        totp_secret: Option<String>,
        patterns: Vec<String>,
    },
}
```

### URL Pattern Matching

Supports flexible URL matching for automatic credential injection:
- Exact match: `api.github.com`
- Wildcard prefix: `*.github.com` (matches api.github.com, raw.github.com)
- Full URL pattern: `https://api.openai.com/*`

### Feature Flags

```toml
[features]
keyring-support = ["keyring"]  # Enable system keyring integration
```

## Files Created

1. `services/zero-core/src/security/keyring.rs` (~600 lines)

## Files Modified

1. `services/Cargo.toml` - Added `keyring = "3.6"` workspace dependency
2. `services/zero-core/Cargo.toml` - Added keyring feature flag
3. `services/zero-core/src/security/mod.rs` - Export keyring module
4. `services/zero-core/src/lib.rs` - Re-export keyring types

## Test Results

```
running 5 tests
test security::keyring::tests::test_url_matching ... ok
test security::keyring::tests::test_credential_serialization ... ok
test security::keyring::tests::test_keyring_file_fallback ... ok
test security::keyring::tests::test_mcp_auth_store ... ok
test security::keyring::tests::test_credential_manager ... ok

test result: ok. 5 passed; 0 failed
```

Total zero-core tests: 243 passing

## Security Features

| Feature | Implementation |
|---------|----------------|
| System Keyring | macOS Keychain, Linux Secret Service, Windows Credential Manager |
| Fallback Storage | JSON file with 0600 permissions |
| URL Validation | MCP tokens validated against stored server URL |
| Token Expiration | Automatic expiration checking |
| PKCE Support | Code verifier storage for OAuth flows |

## Architecture

```
TypeScript (packages/ccode)
    │
    └─→ @codecoder-ai/core ─→ NAPI ─→ zero-core/src/security/keyring.rs
                                              │
                                              ├─→ KeyringManager
                                              │       ├─→ System Keyring (keyring crate)
                                              │       │       ├─→ macOS Keychain
                                              │       │       ├─→ Linux Secret Service
                                              │       │       └─→ Windows Credential Manager
                                              │       │
                                              │       └─→ File Fallback (JSON + 0600)
                                              │
                                              ├─→ CredentialManager
                                              │       └─→ Structured credentials with indexing
                                              │
                                              └─→ McpAuthStore
                                                      └─→ OAuth flow support
```

## Comparison with TypeScript

| Feature | TypeScript (mcp/auth.ts) | Rust (keyring.rs) |
|---------|-------------------------|-------------------|
| Storage | JSON file only | System keyring + file fallback |
| Encryption | None (file permissions only) | System keyring encryption |
| Cross-platform | Unix permissions | Native OS security |
| Code lines | ~136 | ~600 |
| Type safety | Runtime (zod) | Compile-time |

## Migration Path

The TypeScript `McpAuth` namespace can be updated to:

1. Import keyring functions from `@codecoder-ai/core`
2. Use native `McpAuthStore` for token storage
3. Fall back to current JSON storage for compatibility
4. Migrate existing tokens on first access

## Next Steps

1. Add NAPI bindings for keyring module (`services/zero-core/src/napi/keyring.rs`)
2. Update TypeScript MCP auth to use native storage
3. Add credential migration tool for existing JSON files
4. Add TOTP code generation support (optional)

## Verification Commands

```bash
# Run keyring tests (without system keyring)
cargo test -p zero-core security::keyring

# Run keyring tests (with system keyring support)
cargo test -p zero-core --features keyring-support security::keyring

# Run all zero-core tests
cargo test -p zero-core --lib
```
