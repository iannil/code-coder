---
name: verification-loop
description: Continuous verification throughout development
---

# Verification Loop

Continuous verification throughout development.

## Verification Levels

### Quick (30 seconds)

- Critical tests only
- Type check
- Lint check

### Standard (2 minutes)

- All tests
- Type check
- Lint check
- Build verification

### Full (5+ minutes)

- Standard checks
- E2E tests
- Security scan
- Performance check

## Commands

```bash
/verify quick        # Quick smoke tests
/verify              # Standard verification
/verify full         # Comprehensive verification
/verify [checkpoint] # Verify against specific checkpoint
```

## Checkpoint System

### Creating Checkpoints

```markdown
# Checkpoint: user-auth-oauth

## State

- Branch: feature/user-auth
- Files: 15 changed
- Tests: 12 passing

## Success Criteria

- [ ] OAuth flow completes successfully
- [ ] User session persisted correctly
- [ ] Error handling works for failures
- [ ] Tests cover all code paths

## Verification Commands

bun test test/auth/oauth.test.ts
bun run typecheck

## Rollback

git checkout checkpoint-user-auth-oauth
```

### Verifying Against Checkpoints

Run the verification commands and ensure all success criteria are met.

## Continuous Verification

### Pre-Commit Checklist

- [ ] All tests pass
- [ ] Type check passes
- [ ] Linter passes
- [ ] Build succeeds
- [ ] No console.log statements
- [ ] No hardcoded secrets

### Pre-PR Checklist

- [ ] All pre-commit items
- [ ] E2E tests pass
- [ ] Security review complete
- [ ] Documentation updated
- [ ] PR description complete

## Verification Report

```markdown
# Verification Report

## Tests
- 42 passing
- 2 failing

## Type Check
- No errors

## Linter
- 3 warnings

## Build
- Success

## Status: FAILED
Fix failing tests before committing.

## Failing Tests
1. test/auth/login.spec.ts:15 - should handle invalid password
2. test/auth/login.spec.ts:23 - should handle network errors
```

## Auto-Verification Hooks

Configure hooks to run verification automatically:

```json
{
  "hooks": {
    "PreToolUse": {
      "verify_before_commit": {
        "pattern": "Bash",
        "command_pattern": "git commit",
        "actions": [
          {
            "type": "run_command",
            "command": "bun test && bun tsc --noEmit"
          }
        ]
      }
    }
  }
}
```

## Monitoring

Set up continuous monitoring in CI:

```yaml
# .github/workflows/verify.yml
name: Verify
on: [push, pull_request]

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: bun install
      - run: bun test
      - run: bun run typecheck
      - run: bun run lint
```

## Rollback Strategy

When verification fails:

1. Identify breaking change
2. Revert to last checkpoint
3. Fix the issue
4. Re-verify
5. Move forward

```bash
# List recent checkpoints
git log --oneline --grep="checkpoint"

# Checkout specific checkpoint
git checkout <checkpoint-sha>

# Or use git reset
git reset --hard <checkpoint-sha>
```
