---
name: product-experience
description: Product experience evaluation methodology and UX assessment
---

# Product Experience

Framework for evaluating and improving user experience across the product.

## Experience Dimensions

### 1. Usability

Can users accomplish their goals?

**Heuristics (Nielsen's):**

1. Visibility of system status
2. Match between system and real world
3. User control and freedom
4. Consistency and standards
5. Error prevention
6. Recognition rather than recall
7. Flexibility and efficiency
8. Aesthetic and minimalist design
9. Help users recognize and recover from errors
10. Help and documentation

**Assessment:**
```
Feature: Task Creation
┌────────────────────────────────────────────────────┐
│ Heuristic              │ Score │ Notes             │
├────────────────────────┼───────┼───────────────────┤
│ System status          │ 4/5   │ Loading indicator │
│ Real world match       │ 5/5   │ Familiar UI       │
│ User control           │ 3/5   │ No undo option    │
│ Consistency            │ 4/5   │ Minor variations  │
│ Error prevention       │ 2/5   │ Easy to misclick  │
└────────────────────────┴───────┴───────────────────┘
```

### 2. Utility

Does it solve the user's problem?

**Questions:**
- Does the feature address a real user need?
- Is the solution complete or partial?
- How does it compare to alternatives?
- What's the value delivered?

### 3. Desirability

Do users want to use it?

**Factors:**
- Visual design quality
- Brand alignment
- Emotional response
- Social proof

### 4. Accessibility

Can all users access it?

**WCAG Checklist:**
- [ ] Keyboard navigation works
- [ ] Screen reader compatible
- [ ] Sufficient color contrast
- [ ] Alt text for images
- [ ] Focus indicators visible
- [ ] Text resizable to 200%

### 5. Performance

Does it feel fast?

**Perceived Performance:**
| Threshold | User Perception |
|-----------|-----------------|
| 0-100ms | Instant |
| 100-300ms | Slight delay |
| 300-1000ms | Noticeable |
| 1-10s | Losing attention |
| 10s+ | Abandoned |

## Evaluation Methods

### Heuristic Evaluation

Expert review against established principles.

**Process:**
1. Define scope and heuristics
2. Multiple evaluators review independently
3. Aggregate findings
4. Prioritize by severity

### User Testing

Observe real users completing tasks.

**Task Template:**
```
Task: Create a new project and add your first task

Scenario: You just signed up and want to organize
your work. Create a project called "My Work" and
add a task called "Complete onboarding."

Success Criteria:
- Project created with correct name
- Task added to project
- User feels confident about next steps

Observations to Record:
- Time to complete
- Errors or confusion points
- Questions asked
- Emotional reactions
```

### Analytics Review

Quantitative behavior analysis.

**Key Metrics:**
- Task completion rate
- Time on task
- Error rate
- Drop-off points
- Feature adoption

### Survey/Feedback

Direct user input.

**Useful Questions:**
- How would you rate this experience? (1-5)
- What was most frustrating?
- What would you improve?
- How likely to recommend? (NPS)

## Experience Audit

### Page/Screen Review

```markdown
## Screen: Dashboard

### First Impressions
- Clean layout, but busy with information
- Primary action not immediately clear
- Good use of color to indicate status

### Information Architecture
- Logical grouping of elements
- Navigation clear
- Some secondary features hidden too deep

### Interaction Design
- Buttons clearly clickable
- Missing hover states on some elements
- Drag-and-drop not discoverable

### Content
- Labels clear and concise
- Help text adequate
- Some jargon that may confuse new users

### Issues Found
1. [High] CTA button below fold on mobile
2. [Medium] No empty state guidance
3. [Low] Inconsistent icon style
```

### Flow Review

```
Flow: User Onboarding

Step 1: Welcome Screen
├── ✅ Clear value proposition
├── ⚠️ Too much text
└── ❌ Skip option too hidden

Step 2: Account Setup
├── ✅ Only essential fields
├── ✅ Good inline validation
└── ⚠️ Password requirements unclear

Step 3: First Task
├── ❌ No guidance provided
├── ⚠️ Overwhelming options
└── ✅ Quick to complete when found

Completion Rate: 62%
Average Time: 4m 30s
Drop-off Point: Step 3 (38%)
```

## Improvement Framework

### Prioritization Matrix

| Issue | Impact | Effort | Priority |
|-------|--------|--------|----------|
| Missing undo | High | Medium | P1 |
| Slow loading | High | High | P2 |
| Icon inconsistency | Low | Low | P3 |
| Help text updates | Medium | Low | P2 |

### A/B Test Template

```markdown
## Test: Simplified Onboarding

Hypothesis: Reducing onboarding from 5 steps to 3
will increase completion rate by 20%.

Control: Current 5-step flow
Variant: 3-step condensed flow

Primary Metric: Onboarding completion rate
Secondary: D7 retention, activation rate

Sample Size: 1,000 users per variant
Duration: 2 weeks
```

## Deliverables

### Experience Report

1. Executive summary
2. Methodology used
3. Key findings (prioritized)
4. Recommendations
5. Next steps

### Improvement Roadmap

| Phase | Improvements | Timeline |
|-------|--------------|----------|
| Quick wins | Fix obvious issues | 1-2 weeks |
| Core fixes | Address major UX issues | 1 month |
| Enhancements | Improve delight factors | 2-3 months |
| Innovation | New experience patterns | Quarterly |
