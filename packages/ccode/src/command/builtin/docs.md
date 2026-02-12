---
name: docs
description: Initialize or update project documentation
---

Analyze this codebase and create/update project documentation.

$ARGUMENTS

## Task

Review the current state of documentation in this project and improve it:

1. **Inventory existing docs** - Find all markdown files, README, CHANGELOG, API docs
2. **Assess completeness** - What's missing or outdated?
3. **Create/Update documentation** following the documentation skill guidelines

## Documentation to Create/Update

### README.md (Required)

Ensure it contains:
- Clear project description (one sentence)
- Quick start (get running in 2 minutes)
- Installation instructions
- Basic usage examples
- Configuration options
- Links to detailed docs

### CHANGELOG.md (If releases exist)

- Follow Keep a Changelog format
- Document all notable changes
- Group by version and date

### API Documentation (If applicable)

- Document all public interfaces
- Include TypeScript types/signatures
- Provide usage examples

### Architecture Docs (For complex projects)

- High-level system design
- Component interactions
- Key design decisions

## Guidelines

- Be concise - lead with the most important information
- Include working code examples
- Use consistent formatting
- Don't create documentation for documentation's sake

## Output

After completing the documentation updates, summarize:
1. Files created or updated
2. Major sections added
3. Recommendations for future documentation needs
