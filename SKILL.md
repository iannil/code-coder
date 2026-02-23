---
name: codecoder-patterns
description: Coding patterns extracted from CodeCoder (AI programming agent monorepo)
version: 1.0.0
source: local-git-analysis
analyzed_commits: 200
---

# CodeCoder Patterns

## Commit Conventions

This project uses **minimal commit messages** (not full conventional commits):

- `fix` - Bug fixes and corrections
- `update` - Feature additions and improvements
- `init` - Initial setup and scaffolding

Messages are intentionally concise - most commits use single words without prefixes or detailed descriptions.

## Code Architecture

### Monorepo Structure

```
packages/
├── ccode/           # Main CLI tool and business logic
│   ├── src/
│   │   ├── agent/      # Core agent implementation
│   │   ├── api/        # API layer for remote operations
│   │   ├── auth/       # Authentication handling
│   │   ├── bus/        # Event bus system
│   │   ├── cli/        # CLI commands and TUI
│   │   │   └── cmd/
│   │   │       ├── tui/    # Terminal UI (SolidJS + opentui)
│   │   │       │   ├── component/  # Reusable UI components
│   │   │       │   ├── context/    # SolidJS contexts
│   │   │       │   └── routes/     # Route handlers
│   │   │       ├── run.ts      # Main run command
│   │   │       ├── auth.ts     # Auth command
│   │   │       ├── agent.ts    # Agent command
│   │   │       └── ...
│   │   ├── config/     # Configuration management
│   │   ├── context/    # Code context and relevance
│   │   ├── file/       # File operations
│   │   ├── lsp/        # Language Server Protocol
│   │   ├── mcp/        # Model Context Protocol
│   │   ├── memory/     # Knowledge and preferences storage
│   │   ├── provider/   # AI provider integrations
│   │   ├── session/    # Session management
│   │   ├── skill/      # Built-in skills
│   │   ├── tool/       # Tool implementations
│   │   ├── types/      # TypeScript types
│   │   └── util/       # Utility functions
│   ├── test/           # Tests (run from package dir)
│   └── bin/            # CLI entry point
└── util/            # Shared utilities
```

### Key Modules

- **`packages/ccode/src/index.ts`** - Main entry point, sets up yargs CLI
- **`packages/ccode/src/cli/cmd/`** - Individual command implementations
- **`packages/ccode/src/tool/`** - Tool registry and implementations
- **`packages/ccode/src/provider/`** - Multi-provider AI abstraction
- **`packages/ccode/src/memory/`** - Preferences, style, knowledge storage
- **`packages/ccode/src/context/`** - Code fingerprinting and relevance

## Workflows

### Adding a New Tool

1. Create tool file in `packages/ccode/src/tool/`
2. Export with `Tool.Info` interface including `id`, `description`, `parameters`
3. Import and add to `ToolRegistry.all()` in `packages/ccode/src/tool/registry.ts`
4. Add tests in `packages/ccode/test/tool/*.test.ts`

### Adding a CLI Command

1. Create command file in `packages/ccode/src/cli/cmd/`
2. Export command builder compatible with yargs
3. Import in `packages/ccode/src/index.ts` and register with `.command()`

### TUI Component Development

- Components use **SolidJS** with `@opentui/core`
- Located in `packages/ccode/src/cli/cmd/tui/component/`
- Context providers in `packages/ccode/src/cli/cmd/tui/context/`
- Use `createSimpleContext` helper for context creation
- Follow naming: `dialog-*.tsx` for dialogs, `*.tsx` for other components

### Provider Integration

1. Import provider SDK in `packages/ccode/src/provider/provider.ts`
2. Add to `BUNDLED_PROVIDERS` record
3. Add any necessary transformations in `ProviderTransform`
4. Test with `packages/ccode/test/provider/*.test.ts`

## Code Style

### General Principles

- **Prefer `const`** over `let` - use ternary operators instead
- **Avoid `else`** statements - use early returns
- **Minimal destructuring** - use `obj.a` instead of `const { a } = obj`
- **Avoid unnecessary `try`/`catch`** - prefer `.catch()`
- **No `any` types** - always use proper typing
- **Single-letter variables** where context is clear
- **Keep logic together** - don't split into functions unless reusable
- **Use Bun APIs** where available (e.g., `Bun.file()`)

### Namespace Pattern

```typescript
export namespace Config {
  const log = Log.create({ service: "config" })

  export interface Info { ... }
  export type Info = z.infer<typeof Schema>

  export const state = Instance.state(async () => { ... })
  export async function get() { ... }
}
```

### Utility Exports

Utilities use simple function exports:

```typescript
export function lazy<T>(fn: () => T) { ... }
export function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> { ... }
```

### TUI Context Pattern

```typescript
export const { use: useSDK, provider: SDKProvider } = createSimpleContext({
  name: "SDK",
  init: (props) => { ... }
})
```

## Testing Patterns

- **Test location**: `packages/ccode/test/` mirroring src structure
- **Test files**: `*.test.ts` suffix
- **Test directories**:
  - `test/e2e/` - End-to-end tests (critical/, medium/, high/)
  - `test/lifecycle/` - User journey tests
  - `test/unit/` - Component tests (by module)
- **Fixtures**: `packages/ccode/test/fixture/fixture.ts`
- **Run from package**: `cd packages/ccode && bun test`
- **Coverage**: `bun test --coverage`

### Test Helpers

```typescript
// Create temp directory with optional git/config
const dir = await tmpdir({ git: true, config: { ... } })

// Use Symbol.asyncDispose for cleanup
await dir[Symbol.asyncDispose]()
```

## Formatting

- **EditorConfig**: 2 spaces, max 80 char line width, LF line endings
- **No explicit Prettier config** - uses EditorConfig conventions
- **TypeScript**: Strict mode, no `any` types

## Development Commands

```bash
# Install dependencies (Bun 1.3+)
bun install

# Run TUI
bun dev

# Run API server (port 4400)
bun dev serve

# Typecheck all packages
bun turbo typecheck

# Run tests (must be in package directory)
cd packages/ccode && bun test

# Build standalone executable
bun run --cwd packages/ccode build

# Regenerate SDK from OpenAPI
./script/generate.ts
```

## Key Technical Decisions

1. **Bun over Node** - For performance and built-in APIs
2. **Turborepo** - For monorepo build orchestration
3. **SolidJS for TUI** - With opentui for terminal UI
4. **Namespace exports** - For module organization
5. **Instance-based state** - Using `Instance.state()` for per-project state
6. **Multi-provider AI** - Abstracted through custom Provider namespace
7. **MCP support** - Model Context Protocol for extensibility

## Common Patterns

### Logging

```typescript
import { Log } from "@/util/log"

const log = Log.create({ service: "module.name" })
log.debug("message", { data })
log.info("message", { data })
log.warn("message", { data })
log.error("message", { data })
```

### Error Handling

```typescript
import { NamedError } from "@codecoder-ai/util/error"

export const MyError = NamedError("MyError", {
  message: "Something went wrong",
})
```

### Configuration

```typescript
import { Config } from "@/config/config"

const config = await Config.get()
// Access config.instructions, config.model, etc.
```

### Provider Usage

```typescript
import { Provider } from "@/provider/provider"

const provider = Provider.create("anthropic")
const model = provider("claude-3-5-sonnet-20241022")
```

## Built-in Skills

The project includes 17 built-in skills in `packages/ccode/src/skill/builtin/`:

- `best-practices`
- `code-audit`
- `coding-standards`
- `commercialization`
- `competitive-analysis`
- `debugging`
- `documentation`
- `entry-points`
- `feature-acceptance`
- `git-workflow`
- `lifecycle-analysis`
- `opportunity-discovery`
- `planning`
- `product-experience`
- `tdd-workflow`
- `verification-loop`

Each skill has its own SKILL.md with specific patterns.
