---
name: readme
description: Generate or update the project README
---

Analyze this codebase and create or improve the README.md file.

$ARGUMENTS

## README Structure

Create a README with these sections:

### 1. Header
- Project name
- One-line description
- Badges (if applicable): build status, version, license

### 2. Overview
- What does this project do?
- Who is it for?
- Key features (3-5 bullets)

### 3. Quick Start
Get the user running in under 2 minutes:
```bash
# Installation
# Basic usage
```

### 4. Installation
- Prerequisites (runtime versions, dependencies)
- Step-by-step installation
- Verification that it works

### 5. Usage
- Common use cases with examples
- Code snippets that work as-is
- Link to detailed documentation if it exists

### 6. Configuration (if applicable)
- Environment variables
- Config files
- Command-line options

### 7. Development (for open source)
- How to set up dev environment
- How to run tests
- Contribution guidelines (or link)

### 8. License
- License type
- Link to LICENSE file

## Guidelines

- **Be concise** - Every sentence should add value
- **Test examples** - All code snippets should be runnable
- **Update, don't bloat** - If README exists, improve it, don't just add
- **Lead with value** - Most important info first
- **No fluff** - Skip marketing speak and unnecessary adjectives

## Analysis Steps

1. Read existing README.md (if any)
2. Analyze package.json/pyproject.toml/Cargo.toml for metadata
3. Review source code for key functionality
4. Check for existing docs to reference
5. Write/update README following the structure above

## Output

Write the README.md file with the improvements.
After writing, summarize what was added or changed.
