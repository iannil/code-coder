---
name: issues
description: Generate a prioritized list of issues and improvements
subtask: true
---

Analyze this codebase and generate a prioritized list of issues, improvements, and opportunities.

$ARGUMENTS

## Analysis Scope

### 1. Code Quality Issues

Using the code-audit skill methodology, identify:

- **Bugs** - Incorrect logic, edge case failures, error handling gaps
- **Security** - Vulnerabilities, unsafe patterns, exposed secrets
- **Performance** - Inefficient algorithms, N+1 queries, memory issues
- **Maintainability** - Complex code, missing tests, documentation gaps

### 2. Technical Debt

- Outdated dependencies
- Deprecated API usage
- TODO/FIXME comments
- Workarounds that need proper fixes
- Dead code

### 3. Improvement Opportunities

- Feature gaps compared to requirements
- UX/DX improvements
- Performance optimizations
- Test coverage gaps

## Analysis Process

1. Run static analysis tools if available
2. Review code structure and patterns
3. Check dependency health
4. Analyze test coverage
5. Review recent issues/PRs for patterns

## Output Format

Generate issues in this format:

### Critical (Fix Immediately)

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| 1 | [Description] | `file:line` | [Impact] |

### High Priority (Fix Soon)

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| 1 | [Description] | `file:line` | [Impact] |

### Medium Priority (Plan to Fix)

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| 1 | [Description] | `file:line` | [Impact] |

### Low Priority (Nice to Have)

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| 1 | [Description] | `file:line` | [Impact] |

### Summary

- Total issues found: X
- Critical: X
- High: X
- Medium: X
- Low: X

### Recommended Actions

1. [First priority action]
2. [Second priority action]
3. [Third priority action]

## Notes

- Focus on actionable items with clear locations
- Prioritize by impact and effort
- Group related issues when possible
- Distinguish between bugs and improvements
