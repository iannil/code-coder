# TypeScript to Rust Migration - Phase 2.4 Complete

## Date: 2026-03-05

## Summary

Successfully migrated the Provider Transform module from TypeScript to Rust (NAPI). This phase implements high-performance message transformation for different AI providers.

## Changes Made

### Rust Implementation

**New Files:**
- `services/zero-core/src/provider/mod.rs` - Module declaration
- `services/zero-core/src/provider/transform.rs` - Core implementation (~650 lines)

**Features Implemented:**
1. **Message Normalization** (`normalizeMessages`)
   - Anthropic empty content filtering
   - Claude tool ID sanitization (alphanumeric + underscore + hyphen)
   - Mistral tool ID normalization (9 alphanumeric characters)
   - Mistral message sequence fixing (tool → user requires assistant in between)
   - Interleaved reasoning extraction

2. **Cache Application** (`applyCaching`)
   - Marks first 2 system messages and last 2 messages with ephemeral cache hints
   - Supports Anthropic, OpenRouter, Bedrock, and openaiCompatible providers

3. **SDK Key Mapping** (`getSdkKey`)
   - Maps npm package names to SDK-expected providerOptions keys

4. **Provider Options Remapping** (`remapProviderOptions`)
   - Remaps stored providerID to SDK-expected key

5. **Sampling Parameters**
   - `getTemperature` - Model-specific temperature recommendations
   - `getTopP` - Model-specific top_p recommendations
   - `getTopK` - Model-specific top_k recommendations

6. **Combined Transform** (`transformMessages`)
   - Single entry point that performs normalize → cache → remap

### NAPI Bindings

**File:** `services/zero-core/src/napi/provider.rs` (~180 lines)

Exports:
- `normalizeMessages(messagesJson, modelJson) -> NormalizeMessagesResult`
- `applyCaching(messagesJson, providerId) -> ApplyCachingResult`
- `remapProviderOptions(messagesJson, fromKey, toKey) -> string`
- `getSdkKey(npm) -> string | null`
- `getTemperature(modelId) -> number | null`
- `getTopP(modelId) -> number | null`
- `getTopK(modelId) -> number | null`
- `transformMessages(messagesJson, model) -> TransformMessagesResult`

### TypeScript Integration

**Modified Files:**
- `packages/core/src/index.ts` - Added exports for new native functions
- `packages/ccode/src/provider/transform.ts` - Added native fallback pattern

**Integration Pattern:**
```typescript
// Optional native bindings - may not be available until native module is rebuilt
let transformMessagesNative: ((messagesJson: string, model: any) => { messages: string }) | undefined

try {
  const core = await import("@codecoder-ai/core")
  transformMessagesNative = core.transformMessages
} catch {
  // Native bindings not available
}

// In message() function:
if (transformMessagesNative) {
  try {
    const result = transformMessagesNative(JSON.stringify(msgs), modelInfo)
    return JSON.parse(result.messages)
  } catch {
    // Fall through to TypeScript
  }
}
// TypeScript fallback...
```

## Testing

All tests pass:
- Rust unit tests: 6 passed
- TypeScript typecheck: success

## Architecture Notes

### Design Decisions

1. **JSON-in, JSON-out**: Instead of complex type marshaling, functions accept JSON strings and return JSON strings. This minimizes NAPI overhead while keeping complex logic in Rust.

2. **Multiple Strategies**: The normalization function can apply multiple strategies (e.g., `anthropic_empty_filter+claude_tool_id`) and combines them with `+`.

3. **Graceful Degradation**: TypeScript code handles missing native bindings gracefully with try-catch and fallback to JS implementation.

4. **Immutability**: All transformations create new message arrays rather than mutating inputs.

## Files Changed

| File | Change |
|------|--------|
| services/zero-core/src/provider/mod.rs | New |
| services/zero-core/src/provider/transform.rs | New |
| services/zero-core/src/napi/provider.rs | New |
| services/zero-core/src/napi/mod.rs | Added provider module |
| services/zero-core/src/lib.rs | Added provider exports |
| packages/core/src/index.ts | Added native exports |
| packages/core/src/binding.d.ts | Regenerated |
| packages/core/src/binding.js | Regenerated |
| packages/ccode/src/provider/transform.ts | Added native integration |

## Next Steps

1. **Phase 2.6: Config Parser** - Configuration loading and validation
2. **Phase 2.7: Permission Evaluator** - Glob pattern matching and rule evaluation
3. **Performance Benchmarking** - Measure actual speedup from native implementation

## Build Commands

```bash
# Build native module
cd packages/core
npx napi build --manifest-path ../../services/zero-core/Cargo.toml \
  --platform --release --js src/binding.js --dts src/binding.d.ts \
  --features napi-bindings

# Copy bindings (workaround for napi output path)
cp ../../services/zero-core/src/binding.d.ts src/
cp ../../services/zero-core/src/binding.js src/

# Fix reserved keywords in binding.d.ts
# (extends, interface need to be renamed)

# Verify
bun turbo typecheck --filter=ccode
```
