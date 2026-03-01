# AI SDK Migration Tech Debt

**Created:** 2026-03-01
**Status:** Open
**Priority:** Medium

## Summary

The `@ai-sdk/*` packages were temporarily downgraded from v3/v4 to v2 to restore build compatibility. This document tracks the required work to complete the migration properly.

## Background

Commit `b894377` upgraded `@ai-sdk/*` packages to v3/v4 versions, but did not upgrade the `ai` package from v5 to v6. This created a version mismatch:

- `@ai-sdk/provider@3.x` requires `ai@6.x`
- `ai@5.x` bundles `@ai-sdk/provider@2.x`

The incompatible types caused 50+ type errors across the codebase.

## Current State

### Package Versions (Downgraded)

```json
{
  "@ai-sdk/provider": "2.0.1",
  "@ai-sdk/provider-utils": "3.0.20",
  "@ai-sdk/anthropic": "2.0.57",
  "@ai-sdk/openai": "2.0.89",
  "ai": "5.0.119"
}
```

### Target Versions

```json
{
  "@ai-sdk/provider": "3.0.8",
  "@ai-sdk/provider-utils": "4.0.16",
  "@ai-sdk/anthropic": "3.0.50",
  "@ai-sdk/openai": "3.0.37",
  "ai": "6.0.105"
}
```

## Migration Tasks

### Phase 1: Upgrade `ai` to v6

- [ ] Update `package.json` catalog: `"ai": "6.0.105"`
- [ ] Update all `@ai-sdk/*` packages to v3/v4
- [ ] Run `bun install`

### Phase 2: Fix Breaking Changes

#### Type Changes

- [ ] `LanguageModel` → `LanguageModelV3` in `ai@6`
- [ ] Tool factory API changes:
  - `createProviderDefinedToolFactory` removed
  - Use `createProviderToolFactoryWithOutputSchema` with explicit output schema
  - Remove `name` property from tool factory configs (only `id` is supported)

#### Files Requiring Updates

| File | Changes Required |
|------|------------------|
| `src/agent/agent.ts` | Update model type references |
| `src/session/llm.ts` | Update LanguageModel types |
| `src/bootstrap/*.ts` | Update model assignments |
| `src/provider/sdk/openai-compatible/src/responses/tool/*.ts` | Update tool factory API |

### Phase 3: Testing

- [ ] Run full test suite
- [ ] Verify all providers work correctly
- [ ] Test tool invocations

## API Migration Guide

### Tool Factory (v2 → v4)

```typescript
// Before (v2)
import { createProviderDefinedToolFactory } from "@ai-sdk/provider-utils"

export const webSearch = createProviderDefinedToolFactory<InputType, ArgsType>({
  id: "openai.web_search",
  name: "web_search",  // ❌ No longer supported
  inputSchema: z.object({...}),
})

// After (v4)
import { createProviderToolFactoryWithOutputSchema } from "@ai-sdk/provider-utils"

export const webSearchOutputSchema = z.object({
  results: z.array(z.object({...})).nullable(),
})

export const webSearch = createProviderToolFactoryWithOutputSchema<InputType, OutputType, ArgsType>({
  id: "openai.web_search",
  inputSchema: z.object({...}),
  outputSchema: webSearchOutputSchema,  // ✅ Required in v4
})
```

### Model Types

```typescript
// Before
import type { LanguageModel } from "ai"

// After
import type { LanguageModelV3 } from "ai"
// Or use the provider-specific types:
import type { LanguageModelV2 } from "@ai-sdk/provider"
```

## References

- [AI SDK v6 Migration Guide](https://sdk.vercel.ai/docs/migration/v6)
- [Provider Utils v4 Changelog](https://github.com/vercel/ai/releases)

## Notes

- The downgrade was necessary to unblock development
- This tech debt should be addressed before the next major release
- Consider creating a feature branch for the migration work
