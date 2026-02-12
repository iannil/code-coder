---
name: debugging
description: Debugging techniques and troubleshooting methods
---

# Debugging Skills

Debugging techniques and troubleshooting methods.

## Debugging Workflow

### 1. Define the Problem

```
What: Specific error behavior
When: When does it occur
Where: Where does it occur
How: Steps to reproduce
```

### 2. Gather Information

```bash
# View logs
tail -f logs/app.log

# Check errors
bun test 2>&1 | tee error.log

# View recent commits
git log -10 --oneline
```

### 3. Form Hypothesis

Based on gathered information, propose possible causes.

### 4. Verify Hypothesis

```typescript
// Add logging
console.log("DEBUG: user =", user)
logger.debug("Processing", { id, data })

// Add breakpoints
debugger

// Simplify code
// Remove complex logic, test incrementally
```

### 5. Fix and Verify

```bash
# Apply fix
# Re-test
# Confirm issue resolved
# Check for side effects
```

## Common Error Patterns

### TypeError: Cannot read property

```typescript
// Error
const user = getUser()
console.log(user.name) // TypeError if user is undefined

// Fix
const user = getUser()
if (!user) {
  throw new Error("User not found")
}
console.log(user.name)

// Better: Optional chaining
console.log(user?.name)
```

### ReferenceError: variable is not defined

```typescript
// Error
console.log(apiKey) // ReferenceError

// Fix
const apiKey = process.env.API_KEY
console.log(apiKey)

// Better: Add default
const apiKey = process.env.API_KEY || "default"
```

### Promise Rejection

```typescript
// Error: Unhandled rejection
fetch("/api/users").then((res) => res.json())

// Fix: Add catch
fetch("/api/users")
  .then((res) => res.json())
  .catch((err) => console.error(err))

// Better: async/await
try {
  const res = await fetch("/api/users")
  const data = await res.json()
} catch (err) {
  console.error(err)
}
```

### Async/Await Errors

```typescript
// Error: Mixed promise and async
async function getData() {
  fetch("/api/data").then(/* ... */) // Not awaited
}

// Fix
async function getData() {
  const res = await fetch("/api/data")
  return res.json()
}
```

## Debugging Tools

### TypeScript Type Check

```bash
bun tsc --noEmit
bun tsc --noEmit --pretty false | grep "error TS"
```

### Find Code

```bash
grep -r "function getUser" src/
grep -r "import.*getUser" src/
grep -r "getUser(" src/
```

### Git Debugging

```bash
# Find commit that introduced bug
git bisect start
git bisect bad HEAD
git bisect good <working-commit>
git bisect run bun test

# View file history
git log --follow -- file.ts

# View who modified a line
git blame file.ts
```

### Network Debugging

```typescript
async function fetchWithLogging(url: string) {
  console.log(`[API] Request: ${url}`)
  const start = Date.now()

  try {
    const res = await fetch(url)
    const duration = Date.now() - start
    console.log(`[API] Response: ${res.status} (${duration}ms)`)
    return res
  } catch (err) {
    const duration = Date.now() - start
    console.error(`[API] Error: ${err} (${duration}ms)`)
    throw err
  }
}
```

## Performance Debugging

### Identify Slow Code

```typescript
// Add performance markers
console.time("expensiveOperation")
await expensiveOperation()
console.timeEnd("expensiveOperation")

// Use performance API
const start = performance.now()
await expensiveOperation()
const duration = performance.now() - start
if (duration > 100) {
  logger.warn(`Slow operation: ${duration}ms`)
}
```

### Memory Leak Detection

```typescript
setInterval(() => {
  const used = process.memoryUsage()
  console.log({
    rss: `${Math.round(used.rss / 1024 / 1024)}MB`,
    heapUsed: `${Math.round(used.heapUsed / 1024 / 1024)}MB`,
  })
}, 10000)
```

## Logging Strategy

### Log Levels

```typescript
// DEBUG: Detailed info, development use
logger.debug("Processing user", { userId, data })

// INFO: General info
logger.info("User created", { userId })

// WARN: Warning, program continues
logger.warn("Rate limit approaching", { ip, count })

// ERROR: Error, needs attention
logger.error("Database connection failed", { error: err.message })
```

### Structured Logging

```typescript
// Structured logging
logger.info("User action", {
  userId: user.id,
  action: "login",
  ip: req.ip,
  userAgent: req.headers["user-agent"],
})

// Avoid unstructured logging
console.log(`User ${user.id} logged in from ${req.ip}`)
```

## Common Debugging Techniques

### 1. Subtraction Debugging

Comment out code, progressively narrow down the problem scope.

### 2. Minimal Reproduction

Create the smallest possible reproduction case.

```typescript
function reproduceBug() {
  const input = "problematic-input"
  process(input) // Should trigger bug
}
```

### 3. Add Assertions

```typescript
function process(data: Data) {
  console.assert(data.id !== null, "ID should not be null")
  // ...
}
```

### 4. Use Debugger

```typescript
// Set breakpoint in browser DevTools or VS Code
debugger // Execution pauses here
```

## Debugging Checklist

Before debugging:

- [ ] Reproduce the issue
- [ ] Gather error information
- [ ] Review related logs
- [ ] Understand expected behavior

After debugging:

- [ ] Issue resolved
- [ ] Add tests to prevent regression
- [ ] Update documentation
- [ ] Commit the fix
