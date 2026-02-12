---
name: git-workflow
description: Git workflow and version control best practices
---

# Git Workflow Skills

Git workflow and version control best practices.

## Branch Strategy

### Git Flow

```
main (master)
  +-- develop (dev)
  |   +-- feature/* (feature branches)
  |   +-- release/* (release branches)
  |   +-- hotfix/* (hotfix branches)
```

### GitHub Flow

```
main
  +-- feature/* (feature branches)
  +-- fix/* (fix branches)
```

### Branch Naming

```bash
# Feature branches
feature/add-oauth-login
feature/user-avatar-upload

# Fix branches
fix/session-timeout
fix/memory-leak

# Hotfix branches
hotfix/security-patch
hotfix/critical-bug

# Release branches
release/v1.2.0
release/v2.0.0
```

## Commit Convention

### Conventional Commits

```
<type>(<scope>): <subject>

[optional body]

[optional footer(s)]
```

### Type Reference

| Type     | Description        |
| -------- | ------------------ |
| feat     | New feature        |
| fix      | Bug fix            |
| docs     | Documentation      |
| style    | Formatting         |
| refactor | Code refactoring   |
| perf     | Performance        |
| test     | Testing            |
| chore    | Build/tools        |
| ci       | CI configuration   |
| revert   | Revert commit      |

### Examples

```bash
# Simple commit
git commit -m "feat: add user registration"

# Scoped commit
git commit -m "feat(auth): add OAuth2 Google login"

# Commit with body
git commit -m "fix(api): resolve race condition in user creation

- Add database transaction
- Add unique constraint on email
- Add proper error handling

Fixes #123"

# Breaking change
git commit -m "feat(api): redesign authentication

BREAKING CHANGE: Authentication endpoints have changed.

Old: POST /api/login
New: POST /api/auth/login"
```

## Common Commands

### View Status

```bash
git status
git log --oneline
git log --graph --oneline --all
git branch -a
git diff
git diff --staged
```

### Branch Operations

```bash
git checkout -b feature/new-feature
git switch feature/new-feature
git branch -m old-name new-name
git branch -d feature/finished
git push -u origin feature/new-feature
```

### Merge and Rebase

```bash
git merge feature/new-feature
git merge --no-ff feature/new-feature
git rebase main
git rebase -i HEAD~3
```

### Remote Operations

```bash
git pull
git pull --rebase
git push
git push -u origin new-branch
git fetch --all
git fetch --prune
```

## Conflict Resolution

### Merge Conflict

```bash
# 1. Attempt merge
git merge feature-branch

# 2. Check status
git status

# 3. Edit conflict files
# <<<<<<< HEAD
# your changes
# =======
# their changes
# >>>>>>> feature-branch

# 4. Mark resolved
git add resolved-file.ts

# 5. Complete merge
git commit
```

### Rebase Conflict

```bash
git rebase main
# resolve conflicts
git add resolved-file.ts
git rebase --continue
# or abort
git rebase --abort
```

## Undo Changes

```bash
# Undo working directory changes
git checkout -- file.ts

# Unstage file
git reset HEAD file.ts
git restore --staged file.ts

# Undo last commit (keep changes)
git reset --soft HEAD~1

# Undo last commit (discard changes)
git reset --hard HEAD~1

# Create revert commit
git revert abc123
```

## Best Practices

### 1. Commit Frequently

```bash
# Good: Small, frequent commits
git commit -m "feat: add user model"
git commit -m "feat: implement user service"
git commit -m "test: add user tests"

# Bad: Large, infrequent commits
git commit -m "add everything"
```

### 2. Clear Commit Messages

```bash
# Good
git commit -m "feat(auth): add OAuth2 Google login

- Implement Google OAuth2 flow
- Store OAuth tokens securely
- Sync user profile from Google"

# Bad
git commit -m "update"
git commit -m "fix stuff"
```

### 3. Use .gitignore

```gitignore
# Dependencies
node_modules/

# Build outputs
dist/
build/
.turbo/

# Environment
.env
.env.local

# IDE
.idea/
.vscode/

# OS
.DS_Store
```
