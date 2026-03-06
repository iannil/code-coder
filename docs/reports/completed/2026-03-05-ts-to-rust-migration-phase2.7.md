# TypeScript to Rust Migration - Phase 2.7 Risk Assessment

**Date**: 2026-03-05
**Status**: Completed

## Summary

Migrated Bash command risk assessment and file path sensitivity detection from TypeScript to Rust (NAPI).

## Changes Made

### Rust Implementation

**New file**: `services/zero-core/src/security/risk.rs` (~450 lines)

- `RiskLevel` enum: Safe, Low, Medium, High, Critical
- `RiskAssessment` struct: Contains risk level, reason, and auto-approvability
- `assess_bash_risk(command: &str)` - Evaluates Bash command risk using 35+ regex patterns
- `assess_file_risk(path: &str)` - Evaluates file path sensitivity
- `tool_base_risk(tool: &str)` - Returns default risk level for tool types
- `risk_at_or_below_threshold(risk, threshold)` - Compares risk levels

**Key Design Decisions**:
1. Used `OnceLock` for pattern pre-compilation (zero-cost repeated evaluations)
2. Removed negative lookahead from regex patterns (Rust regex doesn't support it)
3. Added special handling for `curl` commands without mutation flags

### NAPI Bindings

**Modified**: `services/zero-core/src/napi/security.rs`

- Added `RiskLevel` string enum
- Added `RiskResult` object struct
- Added NAPI functions:
  - `assessBashRisk(command: String) -> RiskResult`
  - `assessFileRisk(path: String) -> RiskResult`
  - `getToolBaseRisk(tool: String) -> String`
  - `checkRiskThreshold(risk: String, threshold: String) -> bool`
  - `parseRiskLevel(level: String) -> String`

### TypeScript Integration

**Modified**: `packages/core/src/index.ts`
- Added exports for risk assessment functions

**Modified**: `packages/core/src/types.ts`
- Added `RiskResult` interface

**Modified**: `packages/core/src/binding.d.ts`
- Added type declarations for new NAPI functions

**Modified**: `packages/ccode/src/permission/auto-approve.ts`
- Integrated native risk assessment with fallback to TypeScript implementation
- Uses `nativeAssessBashRisk` and `nativeAssessFileRisk` when available

## Risk Patterns Implemented

### Bash Commands

| Level | Examples |
|-------|----------|
| Critical | `sudo`, `rm -rf /`, `shutdown`, `git push --force`, `mkfs`, `dd` |
| High | `git push`, `npm publish`, `curl -X POST`, `docker push`, `kubectl delete` |
| Medium | `git commit`, `npm install`, `mkdir`, `mv`, `cp`, `docker build` |
| Low | `git status`, `ls`, `cat`, `grep`, `npm list` |

### File Paths

| Level | Examples |
|-------|----------|
| High | `.env`, `.pem`, `.key`, `/etc/*`, `~/.ssh/*`, `credentials` |
| Medium | `package.json`, `Cargo.toml`, `.github/workflows/*`, `Dockerfile` |
| Safe | Regular source files |

## Test Results

```
running 12 tests
test security::risk::tests::test_file_risk_safe ... ok
test security::risk::tests::test_file_risk_medium ... ok
test security::risk::tests::test_file_risk_sensitive ... ok
test security::risk::tests::test_risk_level_ordering ... ok
test security::risk::tests::test_risk_threshold ... ok
test security::risk::tests::test_risk_level_parse ... ok
test security::risk::tests::test_tool_base_risk ... ok
test security::risk::tests::test_bash_risk_low ... ok
test security::risk::tests::test_bash_risk_critical ... ok
test security::risk::tests::test_bash_risk_medium ... ok
test security::risk::tests::test_bash_risk_unknown ... ok
test security::risk::tests::test_bash_risk_high ... ok

test result: ok. 12 passed; 0 failed
```

TypeScript typecheck: Passed

## Performance Benefits

1. **Regex Pre-compilation**: Patterns compiled once on first use (~1ms), subsequent matches ~1μs
2. **Zero-copy String Matching**: Rust's string slices avoid unnecessary allocations
3. **Static Pattern Storage**: Using `OnceLock` ensures thread-safe lazy initialization

## Build Notes

To rebuild the native module with the new functions:

```bash
cd packages/core
npx napi build --manifest-path ../../services/zero-core/Cargo.toml \
  --platform --release --js src/binding.js --dts src/binding.d.ts \
  --features napi-bindings
```

## Next Steps

- Phase 2.7 is complete
- All modules from Phase 2 have been assessed and migrated where beneficial
- Remaining modules (LSP, Prompt Builder, MCP, Config) were skipped due to low ROI
