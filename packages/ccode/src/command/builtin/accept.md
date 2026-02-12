---
name: accept
description: Run feature acceptance verification
subtask: true
---

Perform acceptance testing for a feature or change.

Target: $ARGUMENTS

## Acceptance Process

### 1. Identify What to Test

If a specific feature/file/PR is provided, focus on that.
Otherwise, identify recent changes that need verification:
- Check git status for uncommitted changes
- Review recent commits
- Look for newly added features

### 2. Gather Acceptance Criteria

Look for:
- Requirements in issue/PR descriptions
- Acceptance criteria in specs
- User stories or feature docs
- Test files that describe expected behavior

### 3. Execute Verification

#### Functional Testing
- [ ] Core functionality works as specified
- [ ] All acceptance criteria pass
- [ ] Edge cases handled appropriately
- [ ] Error states handled gracefully

#### Integration Testing
- [ ] Works with existing features
- [ ] No regression in related areas
- [ ] API contracts maintained
- [ ] Database operations correct

#### Quality Checks
- [ ] Tests pass (`bun test` or equivalent)
- [ ] Type checks pass
- [ ] No new linting errors
- [ ] Performance acceptable

### 4. Document Results

For each criterion tested:

```
✅ PASS: [Criterion] - [Notes if any]
❌ FAIL: [Criterion] - [Reason for failure]
⚠️ PARTIAL: [Criterion] - [What works, what doesn't]
⏭️ SKIP: [Criterion] - [Why skipped]
```

## Output

### Summary

- **Feature**: [Name/Description]
- **Status**: Ready / Needs Work / Blocked
- **Tests**: X passed, Y failed, Z skipped

### Issues Found

List any problems discovered:
1. [Issue description] - [Severity: Critical/High/Medium/Low]

### Recommendations

- Required fixes before merge/release
- Suggested improvements (non-blocking)
- Follow-up items for later

### Sign-off

If all criteria pass:
```
✅ Feature accepted - Ready for release
```

If issues found:
```
❌ Feature not accepted - [X] issues to resolve
```
