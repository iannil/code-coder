# 测试指南

## 概述

CodeCoder 使用 Bun 内置测试运行器。本文档说明测试基础设施、运行测试的命令和编写测试的指南。

## 测试统计 (2026-02-04)

| 类型         | 文件数 | 测试用例数 |
| ------------ | ------ | --------- |
| E2E 测试     | 21     | ~500      |
| 单元测试     | 33     | ~1500     |
| 集成测试     | 18     | ~800      |
| 性能测试     | 4      | ~100      |
| 无障碍测试   | 2      | ~50       |
| **总计**     | **136** | **3031**  |

## 测试目录结构

```
packages/ccode/test/
├── e2e/              # 端到端测试
│   ├── critical/     # 关键流程测试
│   ├── high/         # 高优先级测试
│   └── medium/       # 中等优先级测试
├── unit/             # 单元测试
│   ├── tui/
│   │   ├── ui/       # UI 组件测试
│   │   └── routes/   # 路由测试
│   └── ...
├── integration/      # 集成测试
├── performance/      # 性能测试
└── a11y/             # 无障碍测试
```

## 运行测试

### 运行所有测试

```bash
# 必须在 ccode 包目录下运行
cd packages/ccode && bun test
```

### 运行特定类型的测试

```bash
# 单元测试
cd packages/ccode && bun test test/unit

# 集成测试
cd packages/ccode && bun test test/integration

# E2E 测试
cd packages/ccode && bun test test/e2e

# 性能测试
cd packages/ccode && bun test test/performance
```

### 运行单个测试文件

```bash
cd packages/ccode && bun test test/unit/agent/agent.test.ts
```

### 监听模式

```bash
cd packages/ccode && bun test --watch
```

## 编写测试

### 文件命名约定

| 模式           | 用途              | 示例                     |
| -------------- | ----------------- | ------------------------ |
| `*.test.ts`    | 普通单元测试       | `agent.test.ts`          |
| `*.test.tsx`   | UI 组件测试        | `dialog.test.tsx`        |
| `mocks.ts`     | Mock 工具          | `helpers/mocks.ts`       |

### 基础测试结构

```typescript
import { describe, test, expect } from "bun:test"

describe("Agent", () => {
  test("should create a new agent", () => {
    const agent = createAgent({ name: "test" })
    expect(agent.name).toBe("test")
  })
})
```

### UI 组件测试

```typescript
import { describe, test, expect } from "bun:test"
import { render } from "solid-js/web"
import { screen } from "solid-testing-library"
import DialogAlert from "./dialog-alert"

describe("DialogAlert", () => {
  test("should render alert message", () => {
    render(() => <DialogAlert message="Test" />)
    expect(screen.getByText("Test")).toBeTruthy()
  })
})
```

### Mock 工具

项目提供了以下 Mock 工厂:

| 工具                 | 用途              |
| -------------------- | ----------------- |
| `createMockAgent`    | 创建 Mock Agent   |
| `createMockSession`  | 创建 Mock Session |
| `createMockTool`     | 创建 Mock Tool    |

```typescript
import { createMockAgent, createMockSession } from "../helpers/mocks"

describe("Session Integration", () => {
  test("should handle agent interaction", () => {
    const agent = createMockAgent()
    const session = createMockSession({ agent })
    // 测试逻辑
  })
})
```

## 测试辅助工具

### TUI Mock

```typescript
import { createMockTUI } from "../../helpers/tui-mock"

const mockTUI = createMockTUI()
// 使用 mockTUI 进行 TUI 相关测试
```

### 测试断言

```typescript
import { expect } from "bun:test"

// 基础断言
expect(value).toBe(expected)
expect(value).toEqual(expected)
expect(value).toBeTruthy()

// 异步断言
await expect(promise).resolves.toBe(value)
await expect(promise).rejects.toThrow()

// 包含断言
expect(array).toContain(item)
expect(string).toContain(substring)
```

## 覆盖率目标

- **目标覆盖率**: 80%+
- **关键路径覆盖率**: 100%

### 查看覆盖率

```bash
cd packages/ccode && bun test --coverage
```

## 性能测试

性能测试位于 `test/performance/` 目录，使用 Bun 的 `bench` 功能:

```typescript
import { bench } from "bun:test"

bench("frecency search", () => {
  // 性能测试逻辑
})
```

## 无障碍测试

无障碍测试位于 `test/a11y/` 目录，确保 UI 组件符合无障碍标准。

## 最佳实践

1. **隔离性**: 每个测试应该独立运行，不依赖其他测试
2. **可读性**: 使用清晰的 `describe` 和 `test` 名称
3. **Mock**: 使用 Mock 工厂隔离外部依赖
4. **断言**: 每个测试应该有明确的断言
5. **清理**: 在 `afterEach` 中清理资源

## 常见问题

### Q: 为什么不能从根目录运行测试?

A: Bun 测试需要在包含 `package.json` 的目录下运行，以正确解析模块路径。

### Q: 如何调试失败的测试?

A: 使用 `bun test --debug` 或在测试中添加 `console.log`。

### Q: 如何跳过某个测试?

A: 使用 `test.skip()` 而不是注释掉测试:

```typescript
test.skip("temporarily disabled", () => {
  // 测试逻辑
})
```
