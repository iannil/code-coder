---
name: best-practices
description: General best practices and development guidelines
---

# Best Practices

General best practices and development guidelines.

## Code Review Checklist

### Functionality

- [ ] Code implements requirements
- [ ] Edge cases handled
- [ ] Error handling complete
- [ ] Logging appropriate

### Readability

- [ ] Variable names clear
- [ ] Functions have single responsibility
- [ ] Complex logic commented
- [ ] Code formatting consistent

### Maintainability

- [ ] No duplicate code
- [ ] Good modularization
- [ ] Clear interface definitions
- [ ] Easy to extend

### Performance

- [ ] No obvious performance issues
- [ ] Resource usage reasonable
- [ ] Caching strategy appropriate
- [ ] Queries optimized

### Security

- [ ] Input validated
- [ ] Output escaped
- [ ] Permissions checked
- [ ] Sensitive data protected

### Testing

- [ ] Unit test coverage
- [ ] Edge tests complete
- [ ] Integration tests adequate
- [ ] E2E tests pass

## Git Commit Convention

### Commit Message Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type Reference

| Type     | Description   | Example                            |
| -------- | ------------- | ---------------------------------- |
| feat     | New feature   | feat(auth): add OAuth2 login       |
| fix      | Bug fix       | fix(api): resolve timeout issue    |
| docs     | Documentation | docs(readme): update install guide |
| style    | Formatting    | styles: fix indentation            |
| refactor | Refactoring   | refactor(user): extract validation |
| test     | Testing       | test(auth): add login tests        |
| chore    | Build/tools   | chore: update dependencies         |
| perf     | Performance   | perf(query): add caching           |
| ci       | CI config     | ci: add GitHub Actions             |

### Examples

```bash
# Simple commit
git commit -m "feat: add user registration"

# Detailed commit
git commit -m "feat(api): add rate limiting

- Add token bucket rate limiter
- Configure limits per endpoint
- Add rate limit headers

Closes #123"
```

## Debugging Tips

### 1. Binary Search Location

```bash
git bisect start
git bisect bad HEAD
git bisect good <stable-commit>
git bisect run bun test
```

### 2. Log Debugging

```typescript
logger.debug("Processing user", { userId, action })
logger.info("User created", { userId, email })
logger.warn("Rate limit exceeded", { ip, limit })
logger.error("Database error", { error: err.message })
```

### 3. Breakpoint Debugging

```typescript
function process(data: Data) {
  debugger // Pause here
  return transform(data)
}
```

### 4. Type Checking

```bash
bun tsc --noEmit
grep -r ": any" src/
```

## Performance Optimization

### 1. Measure First

```typescript
const start = performance.now()
await expensiveOperation()
const duration = performance.now() - start
if (duration > 100) {
  logger.warn(`Slow operation: ${duration}ms`)
}
```

### 2. Batch Processing

```typescript
// Batch processing
await Promise.all(items.map((item) => process(item)))

// Avoid serial processing
for (const item of items) {
  await process(item)
}
```

### 3. Caching

```typescript
const cache = new Map<string, Data>()

async function getData(key: string): Promise<Data> {
  if (cache.has(key)) {
    return cache.get(key)!
  }
  const data = await fetchFromDB(key)
  cache.set(key, data)
  return data
}
```

## Security Checklist

### Pre-Release Check

- [ ] No hardcoded keys
- [ ] Environment variables configured
- [ ] Input validation complete
- [ ] Output escaped
- [ ] Permission checks in place
- [ ] Error handling secure
- [ ] Dependencies vulnerability-free
- [ ] HTTPS enforced

### Sensitive Operations

- [ ] Passwords hashed
- [ ] Token expiration reasonable
- [ ] Sensitive data encrypted
- [ ] Logs sanitized
- [ ] Security headers configured

## Documentation Standards

### Code Comments

```typescript
/**
 * Validates user input data.
 *
 * @param data - User data to validate
 * @returns Validation result with validity flag and error messages
 * @throws {ValidationError} When data format is invalid
 *
 * @example
 * ```ts
 * const result = validateUserData({ name: 'John', email: 'john@example.com' });
 * if (result.valid) {
 *   // Process valid data
 * }
 * ```
 */
function validateUserData(data: unknown): ValidationResult {
  // Implementation
}
```

### README Structure

```markdown
# Project Name

## Introduction
One-line project description

## Quick Start
Installation and usage instructions

## Features
Main features list

## API
API documentation link

## Contributing
How to contribute

## License
Open source license
```

## Team Collaboration

### Code Ownership

- Each module has a clear owner
- Notify owner before modifications
- Discuss major changes first

### Branch Strategy

```
main (production) <- dev (development) <- feature/* (features)
```

### Code Review Flow

1. Create PR
2. Assign reviewers
3. Respond to feedback
4. Get approval
5. Merge code

## Continuous Improvement

### Regular Reviews

- Weekly code quality review
- Monthly documentation updates
- Quarterly technical debt cleanup

### Knowledge Sharing

- Document learned patterns (`/learn`)
- Update skill documentation
- Share best practices
