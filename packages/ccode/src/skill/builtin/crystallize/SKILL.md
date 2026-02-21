---
name: crystallize
description: Crystallize the current session's solution into a reusable skill
---

# /crystallize

Analyze the current session and extract reusable patterns as new skills.

## When to Use

Use this skill when:
- You've just solved a problem that might recur
- You developed a useful workflow worth preserving
- You want to save a multi-step process for future use
- You found a solution you don't want to reinvent

## How It Works

The crystallize command analyzes your session's tool calls and conversations to:

1. **Identify Patterns** - Detect tool sequences and workflows
2. **Generate Skill** - Create a structured skill definition
3. **Verify Quality** - Test the skill against scenarios
4. **Save as SKILL.md** - Persist to the skills directory

## Usage

### Basic Usage

```
/crystallize
```

Analyzes the current session and prompts you to confirm skill creation.

### With Name

```
/crystallize --name my-custom-workflow
```

Specify a custom name for the skill.

### With Type

```
/crystallize --type workflow
```

Specify the skill type: `pattern`, `workflow`, `tool`, or `agent`.

## Workflow

When you invoke `/crystallize`, the following steps occur:

### Step 1: Session Analysis

The system reviews:
- All tool calls in the session
- The problem you were solving
- The solution approach taken

### Step 2: Candidate Extraction

Based on the analysis, a skill candidate is created with:
- A descriptive name
- Problem context and trigger conditions
- The solution pattern (code, steps, or prompt)

### Step 3: Interactive Confirmation

You'll be asked to confirm:
```
Extracted skill candidate:
  Name: deploy-docker-stack
  Type: workflow
  Description: Deploy a multi-container Docker stack with health checks

  Steps:
  1. Build Docker images
  2. Run docker-compose up
  3. Wait for health checks
  4. Verify deployment

Do you want to save this skill? [Y/n]
```

### Step 4: Verification

The skill is automatically verified against test scenarios to ensure it works correctly.

### Step 5: Persistence

If verification passes, the skill is saved as a SKILL.md file:
- Project skills: `.codecoder/skills/<name>/SKILL.md`
- Global skills: `~/.codecoder/skills/<name>/SKILL.md`

## Examples

### Example 1: Database Migration Workflow

After running several database migrations:

```
/crystallize
```

Output:
```
Extracted: "database-migration-workflow"
Description: Run database migrations with backup and rollback support

Steps:
1. Create database backup
2. Run pending migrations
3. Verify migration status
4. Rollback on failure

Saved to: .codecoder/skills/database-migration-workflow/SKILL.md
```

### Example 2: API Testing Pattern

After writing API tests:

```
/crystallize --name api-test-pattern --type pattern
```

Output:
```
Extracted: "api-test-pattern"
Description: Test API endpoints with authentication and error handling

Code Pattern:
- Setup authentication
- Make request with proper headers
- Assert response status and body
- Handle error cases

Saved to: .codecoder/skills/api-test-pattern/SKILL.md
```

## Skill Types

| Type | Description | When to Use |
|------|-------------|-------------|
| `pattern` | Code patterns and snippets | Reusable code structures |
| `workflow` | Multi-step processes | Sequences of operations |
| `tool` | Tool configurations | Specific tool usage patterns |
| `agent` | Agent prompts | Specialized agent behaviors |

## Best Practices

1. **Name Clearly** - Use descriptive kebab-case names
2. **Document Context** - Include when to use the skill
3. **Keep Focused** - One skill per distinct pattern
4. **Verify Works** - Test the skill after creation
5. **Iterate** - Refine skills as you use them

## Viewing Saved Skills

To see your crystallized skills:

```bash
# List skills
ls ~/.codecoder/skills/

# View a skill
cat ~/.codecoder/skills/my-skill/SKILL.md
```

## Managing Skills

### Edit a Skill

Open and edit the SKILL.md file directly.

### Remove a Skill

Delete the skill directory:
```bash
rm -rf ~/.codecoder/skills/my-skill/
```

### Share a Skill

Copy the skill directory to share with others:
```bash
cp -r ~/.codecoder/skills/my-skill/ /path/to/share/
```

## Related Commands

- `/skills` - List available skills
- `/help skills` - Learn about the skill system

## Technical Details

### Confidence Evolution

Crystallized skills start with a confidence score based on:
- Number of tool calls in the session
- Problem and solution detail level
- Initial verification results

The confidence evolves over time:
- Successful uses increase confidence
- Failed uses decrease confidence
- Skills below 0.2 confidence are discarded

### Storage Location

Skills are stored in order of precedence:
1. Project: `.codecoder/skills/`
2. User: `~/.codecoder/skills/`
3. Global: `~/.claude/skills/`

### Auto-Detection

The bootstrap system can also automatically detect patterns worth crystallizing:
- After solving novel problems
- When detecting repeated workflows
- At session end for batch processing

Use `/crystallize` when you want explicit control over skill creation.
