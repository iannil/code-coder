---
name: opportunity-discovery
description: Methodology for discovering product opportunities and market gaps
---

# Opportunity Discovery

Framework for identifying and validating product opportunities.

## Discovery Sources

### 1. User Research

**Interview Techniques:**

- Jobs-to-be-Done interviews
- Problem interviews
- Day-in-the-life studies
- Contextual inquiry

**Key Questions:**
```
1. Walk me through the last time you [did task].
2. What was the hardest part?
3. What tools/methods did you try?
4. What would make this easier?
5. How important is solving this?
```

### 2. Data Analysis

**Product Data:**
- Feature usage patterns
- Drop-off points
- Error rates
- Search queries

**External Data:**
- Market trends
- Search volumes
- Forum discussions
- Review analysis

### 3. Competitive Gaps

**Analysis Areas:**
- Features competitors lack
- Poor user experiences
- Underserved segments
- Emerging needs

### 4. Technology Trends

**Watch For:**
- New capabilities (AI, APIs)
- Platform changes
- Cost reductions
- Standard shifts

## Opportunity Framework

### Jobs-to-be-Done

```
When [situation]
I want to [motivation]
So I can [expected outcome]

Example:
When I'm collaborating with my team remotely
I want to see everyone's changes in real-time
So I can avoid conflicts and stay synchronized
```

### Problem Statement

```markdown
## Problem: [Name]

**Who** has this problem?
Developers working on distributed teams.

**What** is the problem?
Difficult to stay synchronized on code changes.

**When** does it occur?
During active development on shared codebases.

**Why** is it a problem?
Leads to merge conflicts, duplicated work, frustration.

**How** do they currently solve it?
Frequent commits, Slack messages, scheduled syncs.

**How big** is the problem?
Costs 2-4 hours per week per developer.
```

### Opportunity Sizing

```
TAM (Total Addressable Market):
- All developers worldwide: 25M
- Annual spend on dev tools: $5B

SAM (Serviceable Addressable Market):
- Developers on distributed teams: 10M
- Potential revenue: $2B

SOM (Serviceable Obtainable Market):
- Realistic capture in 3 years: 50K users
- Revenue target: $10M ARR
```

## Validation Methods

### 1. Problem Validation

**Goal:** Confirm the problem exists and matters

**Methods:**
- User interviews (5-10)
- Survey (100+)
- Existing behavior analysis

**Signals of Valid Problem:**
- Users actively seeking solutions
- Spending money on workarounds
- Significant time/money cost
- Emotional frustration

### 2. Solution Validation

**Goal:** Confirm your approach works

**Methods:**
- Prototype testing
- Wizard of Oz testing
- Concierge MVP

**Signals of Valid Solution:**
- Users willing to pay
- Repeated usage
- Word of mouth
- Feature requests (not pivots)

### 3. Market Validation

**Goal:** Confirm viable business

**Methods:**
- Landing page test
- Pre-orders
- Pilot customers
- Competitive analysis

**Signals of Valid Market:**
- Conversion rates
- Customer acquisition cost
- Willingness to pay
- Market growth

## Prioritization

### Opportunity Score

```
Score = Impact × Confidence × Feasibility

Impact (1-10):
- How much value for users?
- How many users affected?
- Strategic importance?

Confidence (1-10):
- How validated is this?
- Quality of evidence?
- Team expertise?

Feasibility (1-10):
- Technical complexity?
- Resource requirements?
- Dependencies?
```

### ICE Framework

| Opportunity | Impact | Confidence | Ease | Score |
|-------------|--------|------------|------|-------|
| Real-time sync | 8 | 7 | 4 | 224 |
| Mobile app | 6 | 8 | 3 | 144 |
| AI features | 9 | 4 | 5 | 180 |
| Integrations | 5 | 9 | 7 | 315 |

### Risk Assessment

| Risk Type | Mitigation |
|-----------|------------|
| Value risk | User testing |
| Usability risk | Prototype testing |
| Feasibility risk | Technical spike |
| Business risk | Market research |

## Documentation

### Opportunity Brief

```markdown
## Opportunity: [Name]

### Summary
One paragraph describing the opportunity.

### Evidence
- User interviews: 8/10 mentioned this pain
- Survey: 65% rated it "very important"
- Competitor X has 50K users for this

### Target User
Primary: [persona]
Secondary: [persona]

### Proposed Solution
High-level approach.

### Success Metrics
- Adoption: X% of users within 30 days
- Retention: Y% improvement
- Revenue: $Z new MRR

### Risks & Mitigations
1. Risk A → Mitigation A
2. Risk B → Mitigation B

### Next Steps
1. Validate with prototype
2. Technical feasibility study
3. Business case development
```

### Opportunity Backlog

| Opportunity | Stage | Score | Owner | Next Step |
|-------------|-------|-------|-------|-----------|
| Real-time | Validated | 85 | @name | Design |
| Mobile | Exploring | 60 | @name | Interviews |
| AI assist | Idea | 40 | - | Research |

## Process

### Discovery Cadence

**Weekly:**
- Review customer feedback
- Analyze product data
- Monitor competitors

**Monthly:**
- User interviews (3-5)
- Opportunity review meeting
- Backlog prioritization

**Quarterly:**
- Deep market research
- Strategic opportunity assessment
- Roadmap alignment
