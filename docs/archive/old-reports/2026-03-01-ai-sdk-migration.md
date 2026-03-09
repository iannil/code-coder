# AI SDK Migration Tech Debt

**Created:** 2026-03-01
**Completed:** 2026-03-01
**Status:** Closed ✅
**Priority:** Medium

## Summary

The `@ai-sdk/*` packages were upgraded from v2 to v3/v4, along with `ai` from v5 to v6. All breaking changes have been resolved.

## Background

Commit `b894377` upgraded all `@ai-sdk/*` packages to v3/v4 versions and the `ai` package from v5 to v6.

## Final State

### Package Versions (Upgraded)

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

### Phase 1: Upgrade `ai` to v6 ✅

- [x] Update `package.json` catalog: `"ai": "6.0.105"`
- [x] Update all `@ai-sdk/*` packages to v3/v4
- [x] Run `bun install`

### Phase 2: Fix Breaking Changes ✅

#### Type Changes

- [x] `LanguageModel` → `LanguageModelV3` in `ai@6`
- [x] Tool factory API changes:
  - `createProviderDefinedToolFactoryWithOutputSchema` → `createProviderToolFactoryWithOutputSchema`
  - Updated in: `file-search.ts`, `image-generation.ts`, `local-shell.ts`, `code-interpreter.ts`

### Phase 3: Testing ✅

- [x] Run full test suite
- [x] Verify all providers work correctly
- [x] All 3 packages typecheck passed (0 errors)

## API Migration Guide

### Tool Factory (v2 → v4)

```typescript
// Before (v2)
import { createProviderDefinedToolFactoryWithOutputSchema } from "@ai-sdk/provider-utils"

// After (v4)
import { createProviderToolFactoryWithOutputSchema } from "@ai-sdk/provider-utils"
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
- Commit: `b894377` - feat: upgrade @ai-sdk/* packages to v3/v4

## Completion Notes

- All 17 `@ai-sdk/*` packages upgraded successfully
- Zero type errors remaining
- No runtime issues observed
