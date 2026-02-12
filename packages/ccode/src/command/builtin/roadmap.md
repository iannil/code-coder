---
name: roadmap
description: Generate or update the project roadmap
---

Analyze this project and create or update a ROADMAP.md file.

$ARGUMENTS

## Roadmap Structure

### 1. Vision Statement

One paragraph describing where the project is headed.

### 2. Current Status

- Current version
- Recent major achievements
- Key metrics (if available)

### 3. Short-term Goals (Next Release)

Concrete features and fixes planned:
- [ ] Feature/fix 1
- [ ] Feature/fix 2
- [ ] Feature/fix 3

### 4. Medium-term Goals (1-3 Releases)

Planned capabilities:
- Capability 1
- Capability 2
- Capability 3

### 5. Long-term Vision (6+ Months)

Strategic direction:
- Major initiative 1
- Major initiative 2
- Major initiative 3

### 6. Non-Goals

What we explicitly won't do (and why):
- Non-goal 1 - Reason
- Non-goal 2 - Reason

## Analysis Steps

1. Review existing ROADMAP.md, CHANGELOG.md, and planning docs
2. Check GitHub issues/milestones for planned work
3. Analyze package.json/code for version and features
4. Review recent commits for development direction
5. Look for TODO comments indicating planned work

## Guidelines

- **Be specific** - Vague goals aren't useful
- **Be realistic** - Don't promise what can't be delivered
- **Prioritize** - Order by importance and dependency
- **Update regularly** - Stale roadmaps are worse than none
- **Link to issues** - Reference tracking issues where possible

## Output Format

Write the ROADMAP.md file with:

```markdown
# Project Roadmap

## Vision

[Vision statement]

## Current Status

Version: X.Y.Z
Last updated: YYYY-MM-DD

## Near-term (Next Release)

### Planned
- [ ] Item 1 (#issue)
- [ ] Item 2 (#issue)

### In Progress
- [ ] Item 3 (#issue)

## Medium-term (Q1/Q2 2024)

- Feature category 1
  - Specific capability
  - Specific capability
- Feature category 2
  - Specific capability

## Long-term Vision

[Strategic direction]

## Non-Goals

- Non-goal 1: Reason
- Non-goal 2: Reason

---
*This roadmap is subject to change based on user feedback and priorities.*
```

After writing, summarize the key priorities identified.
