---
name: coding-standards
description: Language best practices and coding conventions
---

# Coding Standards

Language best practices and conventions.

## TypeScript Standards

### Type Definitions

```typescript
// Use interface for object shapes
interface User {
  id: string
  name: string
  email: string
}

// Use type for unions, aliases
type Status = "pending" | "active" | "inactive"
type ID = string

// Use enum sparingly, prefer union types
type Direction = "up" | "down" | "left" | "right"
```

### Function Signatures

```typescript
// Explicit return types
function getUser(id: string): Promise<User> {
  return db.findUser(id)
}

// Use generics for reusable functions
function first<T>(array: T[]): T | undefined {
  return array[0]
}
```

### Error Handling

```typescript
// Custom error types
class ValidationError extends Error {
  constructor(
    public field: string,
    message: string,
  ) {
    super(message)
    this.name = "ValidationError"
  }
}

// Error handling pattern
try {
  const result = await riskyOperation()
  return { success: true, data: result }
} catch (error) {
  if (error instanceof ValidationError) {
    return { success: false, error: error.message }
  }
  throw error
}
```

## Async Patterns

```typescript
// Promise.all for parallel operations
const [users, posts] = await Promise.all([fetchUsers(), fetchPosts()])

// Promise.allSettled for independent operations
const results = await Promise.allSettled([task1(), task2(), task3()])
```

## Naming Conventions

| Type         | Format           | Example         |
| ------------ | ---------------- | --------------- |
| Variables    | camelCase        | `userName`      |
| Constants    | UPPER_SNAKE_CASE | `MAX_RETRIES`   |
| Functions    | camelCase        | `getUserData`   |
| Classes      | PascalCase       | `UserService`   |
| Interfaces   | PascalCase       | `UserData`      |
| Types        | PascalCase       | `UserStatus`    |
| Enum Members | PascalCase       | `Status.Active` |

## Code Style Guidelines

### Prefer const over let

```typescript
// Prefer
const user = getUser()
const name = user ? user.name : "Guest"

// Avoid
let name
if (user) {
  name = user.name
} else {
  name = "Guest"
}
```

### Use early returns

```typescript
// Prefer
function processUser(user: User | null) {
  if (!user) return null
  if (!user.isActive) return null
  return processActiveUser(user)
}

// Avoid
function processUser(user: User | null) {
  if (user) {
    if (user.isActive) {
      return processActiveUser(user)
    } else {
      return null
    }
  } else {
    return null
  }
}
```

### Avoid unnecessary destructuring

```typescript
// Prefer when accessing few properties
const name = user.name
const email = user.email

// Use destructuring for multiple properties
const { name, email, age, role } = user
```
