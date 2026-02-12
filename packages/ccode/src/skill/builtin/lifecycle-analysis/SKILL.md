---
name: lifecycle-analysis
description: User and product lifecycle analysis methodology
---

# Lifecycle Analysis

Framework for understanding and optimizing user journeys and product evolution.

## User Lifecycle Stages

### 1. Awareness

User discovers your product.

**Touchpoints:**
- Search engines
- Social media
- Word of mouth
- Content marketing
- Advertising

**Metrics:**
- Impressions
- Click-through rate
- Traffic sources
- Brand mentions

### 2. Acquisition

User signs up or starts using.

**Touchpoints:**
- Landing page
- Signup flow
- Onboarding
- First-run experience

**Metrics:**
- Conversion rate
- Cost per acquisition
- Signup completion rate
- Time to signup

### 3. Activation

User experiences core value.

**Touchpoints:**
- Tutorial/walkthrough
- Key feature usage
- "Aha moment"
- First success

**Metrics:**
- Activation rate
- Time to value
- Feature adoption
- Setup completion

**Activation Criteria Example:**
```
User is "activated" when they have:
- Created at least 1 project
- Completed at least 1 task
- Used product on 2+ days
Within first 7 days of signup
```

### 4. Retention

User continues to use product.

**Touchpoints:**
- Core product experience
- Notifications/reminders
- New features
- Support interactions

**Metrics:**
- DAU/WAU/MAU
- Retention curves (D1, D7, D30)
- Churn rate
- Session frequency

### 5. Revenue

User pays for value.

**Touchpoints:**
- Paywall/upgrade prompts
- Billing experience
- Value communication
- Pricing page

**Metrics:**
- Conversion to paid
- ARPU/ARPPU
- LTV
- Expansion revenue

### 6. Referral

User recommends product.

**Touchpoints:**
- Share features
- Referral programs
- NPS surveys
- Community

**Metrics:**
- NPS score
- Referral rate
- Viral coefficient
- Social shares

## Lifecycle Mapping

### Journey Map Template

```
Stage: [Activation]

┌─────────────────────────────────────────────────────────────┐
│ User Goal: Complete first meaningful task                   │
├─────────────────────────────────────────────────────────────┤
│ Actions: Create project → Add task → Mark complete          │
├─────────────────────────────────────────────────────────────┤
│ Touchpoints: Dashboard, task editor, completion modal       │
├─────────────────────────────────────────────────────────────┤
│ Emotions: Curious → Focused → Satisfied                     │
├─────────────────────────────────────────────────────────────┤
│ Pain Points: Unclear where to start, too many options       │
├─────────────────────────────────────────────────────────────┤
│ Opportunities: Guided first task, simplified UI for new     │
└─────────────────────────────────────────────────────────────┘
```

### Funnel Analysis

```
Visitors        10,000  (100%)
     ↓
Signups          1,500  (15%)    ← Acquisition
     ↓
Activated          600  (40%)    ← Activation
     ↓
Week 1 Retained    300  (50%)    ← Retention
     ↓
Converted           60  (20%)    ← Revenue
```

## Cohort Analysis

### Building Cohorts

Group users by:
- Signup date/week/month
- Acquisition channel
- Plan type
- Feature usage
- Geography

### Retention Cohort

| Cohort | Week 0 | Week 1 | Week 2 | Week 3 | Week 4 |
|--------|--------|--------|--------|--------|--------|
| Jan W1 | 100% | 45% | 35% | 30% | 28% |
| Jan W2 | 100% | 48% | 38% | 33% | 31% |
| Jan W3 | 100% | 52% | 42% | 36% | - |
| Jan W4 | 100% | 55% | 44% | - | - |

**Insight:** Retention improving over time (product changes working)

## Product Lifecycle

### Stages

1. **Introduction** - Launch, early adopters, rapid iteration
2. **Growth** - Market expansion, feature development, scaling
3. **Maturity** - Market saturation, optimization, efficiency
4. **Decline/Renewal** - Pivot, innovation, or sunsetting

### Stage Indicators

| Indicator | Introduction | Growth | Maturity | Decline |
|-----------|--------------|--------|----------|---------|
| Growth rate | Variable | High | Low | Negative |
| Competition | Few | Increasing | Many | Decreasing |
| Focus | PMF | Scale | Efficiency | Pivot/Exit |
| Pricing | Flexible | Competitive | Optimized | Discounted |

## Optimization Strategies

### By Stage

**Acquisition Issues:**
- Improve landing page messaging
- A/B test signup flow
- Reduce friction

**Activation Issues:**
- Simplify onboarding
- Highlight core value faster
- Personalize first experience

**Retention Issues:**
- Improve core product
- Add engagement features
- Better communication

**Revenue Issues:**
- Clearer value proposition
- Better pricing/packaging
- Reduce friction to upgrade

### Measurement Framework

```
Goal: Improve D7 retention from 35% to 45%

Hypothesis: Users who complete onboarding tutorial
            have 2x higher retention

Test: Show interactive tutorial to 50% of new users

Metrics:
- Tutorial completion rate
- D7 retention (test vs control)
- Activation rate impact
```

## Deliverables

### Lifecycle Report

1. Current state by stage
2. Key drop-off points
3. Improvement opportunities
4. Prioritized recommendations

### Optimization Roadmap

| Stage | Problem | Solution | Impact | Effort |
|-------|---------|----------|--------|--------|
| Activation | Low tutorial completion | Shorter tutorial | High | Medium |
| Retention | Drop after week 2 | Email re-engagement | Medium | Low |
| Revenue | Low upgrade rate | Better upgrade prompt | High | Low |
