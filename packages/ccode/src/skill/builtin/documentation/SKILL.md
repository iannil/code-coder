---
name: documentation
description: Documentation standards and best practices for technical writing
---

# Documentation Standards

Guidelines for creating and maintaining high-quality technical documentation.

## Document Types

### README.md

Primary entry point for any project or module.

**Required Sections:**

1. **Title and Description** - What this project does, in one sentence
2. **Quick Start** - Get running in under 2 minutes
3. **Installation** - Prerequisites and setup steps
4. **Usage** - Common use cases with examples
5. **Configuration** - Environment variables, config files
6. **API Reference** - Link to detailed docs if applicable

### CHANGELOG.md

Track all notable changes following [Keep a Changelog](https://keepachangelog.com).

```markdown
## [1.2.0] - 2024-01-15

### Added
- New feature X for Y use case

### Changed
- Improved performance of Z

### Fixed
- Bug where A caused B

### Removed
- Deprecated feature W
```

### API Documentation

Document every public interface:

```typescript
/**
 * Creates a new user account.
 *
 * @param email - User's email address (must be unique)
 * @param password - Password (min 8 chars, requires uppercase + number)
 * @returns The created user object with generated ID
 * @throws {ValidationError} If email format is invalid
 * @throws {ConflictError} If email already exists
 *
 * @example
 * const user = await createUser("user@example.com", "SecurePass123")
 * console.log(user.id) // "usr_abc123"
 */
```

## Writing Guidelines

### Be Concise

- Lead with the most important information
- One idea per sentence
- Use active voice
- Avoid filler words ("simply", "just", "basically")

### Be Specific

```markdown
# Bad
The function processes data quickly.

# Good
The function processes 10,000 records in under 100ms.
```

### Use Examples

Every concept should have a concrete example:

```markdown
# Bad
Use environment variables for configuration.

# Good
Use environment variables for configuration:
\`\`\`bash
export DATABASE_URL="postgres://localhost:5432/mydb"
export API_KEY="your-api-key"
\`\`\`
```

### Structure for Scanning

- Use descriptive headings
- Keep paragraphs short (3-4 lines max)
- Use lists for sequential steps or multiple items
- Use tables for comparing options
- Use code blocks for anything technical

## Code Examples

### Show Complete Examples

```typescript
// Bad - missing imports and context
const result = parse(data)

// Good - runnable as-is
import { parse } from "./parser"

const data = '{"name": "test"}'
const result = parse(data)
console.log(result.name) // "test"
```

### Show Common Errors

```typescript
// This will throw - parse() requires a string
const result = parse(null) // TypeError: Cannot read property...

// Handle null inputs explicitly
const result = data ? parse(data) : defaultValue
```

## Maintenance

### Review Triggers

Update documentation when:

- Adding new features
- Changing existing behavior
- Deprecating functionality
- Receiving user questions about unclear points

### Documentation Debt

Track and prioritize:

- [ ] Missing API documentation
- [ ] Outdated examples
- [ ] Broken links
- [ ] User-reported confusion

## Quality Checklist

Before publishing:

- [ ] All code examples are tested and runnable
- [ ] No broken links
- [ ] Spelling and grammar checked
- [ ] Screenshots/diagrams are up to date
- [ ] Version numbers are current
- [ ] Prerequisites are clearly stated
