---
name: tdd-workflow
description: Test-driven development methodology guide
---

# TDD Workflow

Test-driven development methodology.

## The TDD Cycle

```
RED -> GREEN -> REFACTOR
```

### RED Phase

Write a failing test that defines expected behavior.

### GREEN Phase

Write the minimum code to make the test pass.

### REFACTOR Phase

Improve the code while keeping tests green.

## Test Structure

### Arrange-Act-Assert

```typescript
test("should return user when found", async () => {
  // Arrange: Set up test data
  const userId = "user-123"
  const expectedUser = { id: userId, name: "Test" }
  mockDb.getUser.mockResolvedValue(expectedUser)

  // Act: Execute the function
  const result = await userService.getUser(userId)

  // Assert: Verify the result
  expect(result).toEqual(expectedUser)
})
```

## Test Categories

### Unit Tests

- Test single functions in isolation
- Mock all external dependencies
- Fast (< 1ms per test)

### Integration Tests

- Test multiple components together
- Use real database/service when appropriate
- Slower (< 100ms per test)

### E2E Tests

- Test full user workflows
- Use Playwright
- Slowest but most comprehensive

## Best Practices

### 1. Descriptive Test Names

```typescript
// Reads like a requirement
it("should return null when user is not found", async () => {})

// Vague - avoid
it("test user lookup", async () => {})
```

### 2. Test Behavior, Not Implementation

```typescript
// Tests behavior
test("filterUsers returns only adults", () => {
  const result = filterUsers(users, 18)
  expect(result.every((u) => u.age >= 18)).toBe(true)
})
```

### 3. Test Edge Cases

```typescript
test("should handle empty array", () => {
  expect(processItems([])).toEqual([])
})

test("should handle null input", () => {
  expect(processItems(null)).toEqual([])
})

test("should handle boundary value", () => {
  expect(isAdult(17)).toBe(false)
  expect(isAdult(18)).toBe(true)
})
```

### 4. Test Error Cases

```typescript
test("should throw when user not found", async () => {
  mockDb.getUser.mockResolvedValue(null)

  await expect(getUser("invalid-id")).rejects.toThrow(NotFoundError)
})
```

## Running Tests

```bash
# Run all tests
bun test

# Run specific file
bun test test/users.test.ts

# Run with coverage
bun test --coverage

# Watch mode
bun test --watch
```

## Coverage Requirements

| Code Type      | Coverage | Priority |
| -------------- | -------- | -------- |
| Business Logic | 90%+     | Critical |
| API Routes     | 80%+     | High     |
| Utilities      | 95%+     | High     |
| Types          | N/A      | N/A      |
| Config         | N/A      | N/A      |

## Testing Checklist

Before marking code complete:

- [ ] All tests pass
- [ ] Coverage meets requirements
- [ ] Edge cases tested
- [ ] Error cases tested
- [ ] Tests are readable
- [ ] No test interdependence
- [ ] Tests run quickly
