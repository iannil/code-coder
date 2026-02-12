---
name: code-audit
description: Code audit methodology for identifying issues and technical debt
---

# Code Audit

Systematic approach to reviewing codebase health and identifying improvements.

## Audit Categories

### 1. Code Quality

**Complexity Analysis**

- Cyclomatic complexity per function
- Nesting depth
- Function/file length
- Dependency count

```
High Risk Indicators:
- Functions > 50 lines
- Cyclomatic complexity > 10
- Nesting depth > 4
- Files > 500 lines
```

**Code Smells**

- [ ] Duplicated code
- [ ] Long parameter lists
- [ ] Dead code
- [ ] Magic numbers/strings
- [ ] God classes/functions
- [ ] Feature envy
- [ ] Inappropriate intimacy

### 2. Architecture

**Module Analysis**

- Clear separation of concerns
- Appropriate abstraction levels
- Circular dependencies
- Layer violations

**Dependency Health**

```
Check for each dependency:
- [ ] Still maintained?
- [ ] Known vulnerabilities?
- [ ] Major version behind?
- [ ] Unused?
- [ ] Duplicate functionality?
```

### 3. Security

**Common Vulnerabilities**

- [ ] Injection flaws (SQL, command, XSS)
- [ ] Broken authentication
- [ ] Sensitive data exposure
- [ ] Security misconfigurations
- [ ] Missing access controls
- [ ] Insecure dependencies

**Secret Scanning**

- [ ] Hardcoded credentials
- [ ] API keys in code
- [ ] Private keys committed
- [ ] Environment files in repo

### 4. Performance

**Hotspot Analysis**

- [ ] N+1 queries
- [ ] Unbounded loops
- [ ] Missing pagination
- [ ] Inefficient algorithms
- [ ] Memory leaks
- [ ] Synchronous blocking

**Resource Usage**

- [ ] Database connection pooling
- [ ] Cache utilization
- [ ] Asset optimization
- [ ] Bundle size

### 5. Maintainability

**Documentation**

- [ ] README up to date
- [ ] API documentation exists
- [ ] Complex logic explained
- [ ] Setup instructions work

**Testing**

- [ ] Test coverage adequate
- [ ] Tests actually test behavior
- [ ] Flaky tests identified
- [ ] Test data management

## Audit Process

### 1. Automated Scans

Run tooling first to gather data:

```bash
# Linting
eslint . --report-unused-disable-directives

# Type checking
tsc --noEmit

# Security scanning
npm audit

# Dependency analysis
npx depcheck

# Complexity analysis
npx complexity-report src/
```

### 2. Manual Review

Focus areas for human review:

1. **Critical paths** - Authentication, payment, data handling
2. **Recent changes** - High churn areas
3. **Known problem areas** - Historical issues
4. **Integration points** - API boundaries

### 3. Prioritization

Score each finding:

| Factor | Weight |
|--------|--------|
| Security impact | 5x |
| User impact | 4x |
| Frequency | 3x |
| Fix complexity | -2x |
| Technical debt | 2x |

## Issue Classification

### Severity Matrix

| Type | Critical | High | Medium | Low |
|------|----------|------|--------|-----|
| Security | Data breach risk | Auth bypass | Info disclosure | Minor leak |
| Bug | Data loss | Feature broken | Feature degraded | Cosmetic |
| Performance | System down | Major slowdown | Noticeable lag | Minor delay |
| Debt | Blocking work | Slowing work | Annoying | Nice to fix |

### Issue Template

```markdown
## Issue: [Title]

**Category:** Security / Bug / Performance / Debt
**Severity:** Critical / High / Medium / Low
**Location:** `path/to/file.ts:123`

### Description
What the issue is.

### Impact
Who is affected and how.

### Evidence
Code snippet, metrics, or reproduction steps.

### Recommendation
How to fix it.

### Effort
Estimated fix time: S/M/L/XL
```

## Reporting

### Executive Summary

One page for stakeholders:

- Overall health score (A-F)
- Critical issues count
- Top 3 recommendations
- Estimated remediation effort

### Detailed Report

For engineering team:

- All findings with evidence
- Prioritized fix list
- Technical recommendations
- Tooling suggestions

### Tracking Dashboard

Metrics to monitor:

| Metric | Current | Target | Trend |
|--------|---------|--------|-------|
| Critical issues | 2 | 0 | ↓ |
| Test coverage | 65% | 80% | ↑ |
| Dependency age | 6 mo avg | < 3 mo | → |
| Build time | 4 min | < 2 min | ↓ |

## Follow-up

### Remediation Plan

1. Fix critical security issues immediately
2. Address high-severity bugs in current sprint
3. Schedule medium issues in backlog
4. Track low issues for opportunistic fixes

### Re-audit Schedule

- Critical paths: Monthly
- Full audit: Quarterly
- Dependency audit: Weekly (automated)
