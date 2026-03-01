# CodeCoder Testing Guide

This document describes the testing strategy, structure, and commands for CodeCoder.

## Test Structure

```
packages/ccode/test/
├── accessibility/          # A11y tests (contrast, keyboard navigation)
├── agent/                  # Agent system tests
├── autonomous/             # Autonomous mode tests
├── cli/                    # CLI-specific tests
├── config/                 # Configuration tests
├── e2e/                    # End-to-end tests
│   ├── critical/           # Must-pass tests
│   ├── high/               # High priority
│   ├── medium/             # Medium priority
│   ├── tui/                # TUI-specific E2E
│   └── visual/             # Visual regression
├── edge/                   # Edge cases (large files, concurrent ops)
├── file/                   # File system tests (security, ignore)
├── fixture/                # Test data factories
├── helpers/                # Test utilities
├── integration/            # Integration tests
├── lifecycle/              # User lifecycle tests
├── lsp/                    # Language Server Protocol tests
├── mock/                   # Mock implementations
├── patch/                  # Patch/diff tests
├── perf/                   # Benchmark tests
├── performance/            # Performance tests
├── permission/             # Permission system tests
├── project/                # Project management tests
├── provider/               # LLM provider tests
├── question/               # User question/dialog tests
├── session/                # Session management tests
├── snapshot/               # Snapshot tests
├── tool/                   # Tool implementation tests
├── unit/                   # Unit tests
└── util/                   # Utility function tests
```

## Test Commands

### Running Tests

```bash
# All tests
bun test

# Watch mode
bun test:watch

# By level
bun test:unit           # Unit tests only
bun test:integration    # Integration tests
bun test:e2e            # End-to-end tests
bun test:e2e:critical   # Critical E2E (must pass)

# By category
bun test:tui            # TUI tests (unit + integration + critical E2E)
bun test:perf           # Performance tests
bun test:a11y           # Accessibility tests
bun test:edge           # Edge case tests
bun test:security       # Security tests

# Coverage
bun test:coverage       # All tests with coverage
bun test:unit:coverage  # Unit tests with coverage
bun test:ci             # CI pipeline tests
```

### Test Scripts Reference

| Script | Description |
|--------|-------------|
| `test` | Run all tests |
| `test:watch` | Watch mode |
| `test:unit` | Unit tests |
| `test:integration` | Integration tests |
| `test:e2e` | All E2E tests |
| `test:e2e:critical` | Critical path E2E |
| `test:e2e:high` | High priority E2E |
| `test:tui` | TUI component tests |
| `test:perf` | Performance tests |
| `test:a11y` | Accessibility tests |
| `test:security` | Security tests |
| `test:ci` | CI pipeline subset |

## Test Helpers

### Mock Utilities (`test/mock/`)

```typescript
import { createMockProvider, setupAnthropicMock } from "./mock"

// Mock an LLM provider
const mock = createMockProvider({
  model: "claude-sonnet-4-5",
  responses: [{ role: "assistant", content: "Hello!" }]
})
```

### TUI Mock (`test/helpers/tui-mock.ts`)

```typescript
import {
  createMockRenderer,
  createMockKeyboardEvent,
  createMockSession
} from "./helpers"

// Mock renderer
const renderer = createMockRenderer()

// Mock keyboard event
const event = createMockKeyboardEvent({ name: "enter", ctrl: false })

// Mock session
const session = createMockSession({ title: "Test Session" })
```

### E2E Helper (`test/helpers/e2e-helper.ts`)

```typescript
import { createE2ETest } from "./helpers/e2e-helper"

const ctx = await createE2ETest({
  cmd: "bun",
  args: ["run", "./src/index.ts"],
  rows: 40,
  cols: 120
})

await ctx.waitForOutput("Ready")
ctx.write("hello")
ctx.sendEnter()
ctx.cleanup()
```

### Fixtures (`test/fixture/`)

```typescript
import { tmpdir } from "./fixture/fixture"

// Create isolated temp directory
await using dir = await tmpdir({ git: true })
// dir.path contains the temp directory path
// Directory is cleaned up automatically
```

## Writing Tests

### Unit Test Pattern

```typescript
import { describe, it, expect } from "bun:test"

describe("MyFunction", () => {
  it("should handle valid input", () => {
    const result = myFunction("valid")
    expect(result).toBe("expected")
  })

  it("should throw on invalid input", () => {
    expect(() => myFunction(null)).toThrow()
  })
})
```

### Integration Test Pattern

```typescript
import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import { tmpdir } from "../fixture/fixture"

describe("Feature Integration", () => {
  let dir: Awaited<ReturnType<typeof tmpdir>>

  beforeEach(async () => {
    dir = await tmpdir({ git: true })
  })

  afterEach(async () => {
    await dir[Symbol.asyncDispose]()
  })

  it("should work end-to-end", async () => {
    // Test implementation using dir.path
  })
})
```

### E2E Test Pattern

```typescript
import { describe, it, expect, afterEach } from "bun:test"
import { createE2ETest, type E2ETestContext } from "../helpers/e2e-helper"

describe("User Journey", () => {
  let ctx: E2ETestContext

  afterEach(() => {
    ctx?.cleanup()
  })

  it("should complete user flow", async () => {
    ctx = await createE2ETest({
      cmd: "bun",
      args: ["run", "./src/index.ts"],
    })

    await ctx.waitForOutput("Welcome")
    ctx.write("@agent hello")
    ctx.sendEnter()
    await ctx.waitForOutput("Response")
  })
})
```

## Coverage Requirements

| Metric | Target |
|--------|--------|
| Line Coverage | 80% |
| Branch Coverage | 75% |
| Function Coverage | 85% |

## CI/CD Pipeline

Tests are run automatically on push and PR via GitHub Actions:

1. **Static Analysis** - TypeScript check, lint, format
2. **Unit Tests** - Fast unit tests with coverage
3. **Integration Tests** - Component integration
4. **E2E Tests (Critical)** - Must-pass user journeys
5. **E2E Tests (High)** - High priority tests
6. **Non-functional** - Performance, accessibility, security

## Best Practices

1. **Test Isolation** - Each test should be independent
2. **Mock External Services** - Never call real APIs in tests
3. **Use Fixtures** - Use the fixture factory for test data
4. **Clean Up** - Always clean up resources (use `afterEach`)
5. **Descriptive Names** - Test names should describe behavior
6. **Small Tests** - Each test should verify one thing
7. **No Console Logs** - Remove debug logs before committing
