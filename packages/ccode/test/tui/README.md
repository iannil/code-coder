# TUI Testing Documentation

This directory contains the test infrastructure and tests for the CodeCoder TUI (Terminal User Interface).

## Test Structure

```
test/
├── helpers/                    # Test utilities and mocks
│   ├── index.ts               # Main exports
│   ├── tui-mock.ts            # OpenTUI primitives mock
│   ├── test-context.tsx       # Mock context providers
│   └── render-test.tsx        # Component test renderer
│
├── unit/tui/                   # Unit tests
│   ├── util/
│   │   ├── keybind.test.ts   # Keybind parsing/matching (100% coverage)
│   │   ├── clipboard.test.ts # OSC 52 encoding
│   │   └── terminal.test.ts  # Terminal colors
│   └── context/
│       ├── route.test.tsx    # Route navigation
│       ├── keybind.test.tsx  # Keybind context logic
│       └── prompt/
│           └── frecency.test.tsx # Frecency calculations
│
└── integration/tui/            # Integration tests
    ├── prompt-flow.test.ts    # Prompt submission/autocomplete
    ├── session-navigation.test.ts # Session switching
    └── keybind-commands.test.ts # Keybind to command mapping
```

## Running Tests

```bash
# Run all TUI tests
bun run test:tui

# Run only unit tests
bun run test:tui:unit

# Run only integration tests
bun run test:tui:integration

# Run with coverage
bun run test:tui:coverage
```

## Coverage

| Module | Coverage |
|--------|----------|
| `src/util/keybind.ts` | 100% |
| `test/helpers/*` | 100% |
| TUI utilities | ~60-70% |
| Overall | ~67% Functions / ~69% Lines |

## Test Helpers

### `tui-mock.ts`

Provides mock implementations for:
- `createMockRenderer()` - Mock OpenTUI renderer
- `createMockKeyboardEvent()` - Mock keyboard events
- `createMockDimensions()` - Mock terminal size
- `createMockTheme()` - Mock theme data
- `createMockKeybindConfig()` - Mock keybind config

### `test-context.tsx`

Mock SolidJS context providers:
- `TestRouteProvider` - Route context
- `TestKeybindProvider` - Keybind context
- `TestSyncProvider` - Sync context
- `TestThemeProvider` - Theme context
- `TestDialogProvider` - Dialog context
- `TestRendererProvider` - Renderer context
- `TestProviders` - Combined provider for convenience

### `render-test.tsx`

Component testing utilities:
- `renderComponent()` - Render SolidJS component
- `renderTest()` - Render with auto-cleanup
- `waitFor()` - Wait for async conditions
- `act()` - Run async actions

## Writing New Tests

### Unit Tests

```typescript
import { describe, test, expect } from "bun:test"
import { Keybind } from "@/util/keybind"

describe("Keybind Utility", () => {
  test("should parse simple key", () => {
    const result = Keybind.parse("a")
    expect(result).toHaveLength(1)
  })
})
```

### Integration Tests

```typescript
import { describe, test, expect } from "bun:test"

describe("Prompt Flow", () => {
  test("should submit prompt", () => {
    const prompt = { text: "test", files: [] }
    // Test prompt submission logic
  })
})
```

## Notes

- Tests use Bun's built-in test runner
- SolidJS components are tested with direct rendering (no DOM required)
- Terminal-dependent code is mocked or tested for structure only
- `vi.doMock` is not available in Bun - use simpler mocking approaches
