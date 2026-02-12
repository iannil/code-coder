---
name: planning
description: Planning methodology and prioritization frameworks for development
---

# Planning Methodology

Structured approach to planning development work and prioritizing features.

## Planning Levels

### Strategic (Quarterly/Yearly)

- Vision and goals
- Major initiatives
- Resource allocation
- Success metrics

### Tactical (Monthly/Sprint)

- Feature breakdown
- Task prioritization
- Dependency mapping
- Capacity planning

### Operational (Daily/Weekly)

- Task execution
- Blocker resolution
- Progress tracking
- Scope adjustment

## Prioritization Frameworks

### RICE Score

```
RICE = (Reach × Impact × Confidence) / Effort

Reach: How many users affected (per quarter)
Impact: How much it helps (0.25 = minimal, 3 = massive)
Confidence: How sure are we (0-100%)
Effort: Person-months required
```

### MoSCoW Method

| Category | Definition | Allocation |
|----------|------------|------------|
| Must Have | Critical for release | 60% |
| Should Have | Important but not critical | 20% |
| Could Have | Nice to have | 15% |
| Won't Have | Out of scope (this time) | 5% |

### Value vs Effort Matrix

```
High Value │ Quick Wins  │  Major Projects
           │ (Do First)  │  (Plan Carefully)
           ├─────────────┼─────────────────
Low Value  │ Fill-ins    │  Don't Do
           │ (If Time)   │  (Avoid)
           └─────────────┴─────────────────
              Low Effort    High Effort
```

## Feature Breakdown

### Epic → Story → Task

```
Epic: User Authentication
├── Story: User can sign up with email
│   ├── Task: Design signup form
│   ├── Task: Implement email validation
│   ├── Task: Create user in database
│   └── Task: Send verification email
├── Story: User can log in
│   ├── Task: Design login form
│   ├── Task: Implement password verification
│   └── Task: Generate session token
└── Story: User can reset password
    ├── Task: Design reset flow
    ├── Task: Generate reset token
    └── Task: Implement password update
```

### Story Template

```markdown
**As a** [user type]
**I want** [action]
**So that** [benefit]

**Acceptance Criteria:**
- [ ] Given X, when Y, then Z
- [ ] Edge case A is handled
- [ ] Error case B shows appropriate message
```

## Dependency Management

### Dependency Map

```
[Auth System] ───────────────────┐
      │                          │
      ▼                          ▼
[User Profile] ────────► [Permissions]
      │                          │
      ▼                          ▼
[Settings Page] ◄────── [Admin Panel]
```

### Critical Path

Identify the longest chain of dependencies—this is your minimum timeline.

1. List all tasks with durations
2. Map dependencies
3. Find the longest path
4. Optimize by parallelizing or reducing scope

## Estimation

### Relative Estimation

Use story points or t-shirt sizes for relative complexity:

| Size | Complexity | Typical Duration |
|------|------------|------------------|
| XS | Trivial change | Hours |
| S | Simple, well-understood | 1-2 days |
| M | Moderate complexity | 3-5 days |
| L | Complex, some unknowns | 1-2 weeks |
| XL | Very complex, many unknowns | 2+ weeks (break down) |

### Estimation Principles

- Never estimate alone—use planning poker or similar
- Include testing, review, and deployment time
- Add buffer for unknowns (20-50%)
- Track actual vs estimated to calibrate

## Risk Management

### Risk Matrix

```
High Impact  │ Monitor    │ Mitigate
             │ Closely    │ Actively
             ├────────────┼────────────
Low Impact   │ Accept     │ Monitor
             │            │ Occasionally
             └────────────┴────────────
              Low Probability  High Probability
```

### Mitigation Strategies

1. **Avoid** - Change plan to eliminate risk
2. **Reduce** - Take action to lower probability or impact
3. **Transfer** - Shift responsibility (insurance, outsourcing)
4. **Accept** - Acknowledge and prepare contingency

## Planning Meetings

### Sprint Planning

1. Review priorities and capacity
2. Select items from backlog
3. Break down into tasks
4. Identify dependencies and risks
5. Commit to sprint goal

### Retrospective

- What went well?
- What could improve?
- Action items for next sprint

## Planning Checklist

- [ ] Goals are clear and measurable
- [ ] Work is broken into manageable chunks
- [ ] Dependencies are mapped
- [ ] Risks are identified
- [ ] Estimates are realistic
- [ ] Team has agreed to commitments
