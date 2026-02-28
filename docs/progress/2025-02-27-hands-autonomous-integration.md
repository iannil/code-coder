# Hands + Autonomous Integration Progress

**Date**: 2025-02-27
**Status**: Implementation Complete

## Overview

Integrated the Hands system (zero-workflow/Rust) with the Autonomous Orchestrator (ccode/TypeScript) to enable autonomous decision-making for scheduled hands.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    zero-workflow :4432                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐    │
│  │  scheduler  │ │   hands     │ │  Autonomous Bridge  │    │
│  │  (cron)     │ │  executor   │ │     (new Rust)       │    │
│  └──────┬──────┘ └──────┬──────┘ └──────────┬──────────┘    │
│         │                │                     │               │
│         └────────────────┴─────────────────────┘           │
│                              │                            │
└──────────────────────────────┼────────────────────────────┘
                               │
                               ▼ HTTP (new endpoint)
┌─────────────────────────────────────────────────────────────┐
│                    ccode API :4400                          │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         Autonomous Orchestrator (TypeScript)          │   │
│  │  - CLOSE Decision Framework                           │   │
│  │  - Evolution Loop                                     │   │
│  │  - Task Queue & Phase Runner                          │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Autonomous API Handler (Complete)

**File**: `packages/ccode/src/api/server/handlers/autonomous.ts`

- Created `/api/v1/autonomous/execute` endpoint
- Supports non-session mode for Hands integration
- Returns CLOSE scores, quality scores, craziness scores

**Routes registered in**: `packages/ccode/src/api/server/router.ts`
- POST `/api/v1/autonomous/execute`
- GET `/api/v1/autonomous/health`
- GET `/api/v1/autonomous/thresholds`

### Phase 2: Hands Manifest Enhancement (Complete)

**File**: `services/zero-workflow/src/hands/manifest.rs`

Added fields to HAND.md frontmatter:
- `autonomy.level`: lunatic/insane/crazy/wild/bold/timid
- `autonomy.unattended`: Enable unattended mode
- `autonomy.max_iterations`: Maximum evolution iterations
- `decision.use_close`: Use CLOSE framework
- `decision.web_search`: Enable web search
- `decision.evolution`: Enable evolution loop
- `resources.max_tokens`: Token budget
- `resources.max_cost_usd`: Cost budget
- `resources.max_duration_sec`: Time budget

### Phase 2: Autonomous Bridge Client (Complete)

**File**: `services/zero-workflow/src/hands/autonomous_bridge.rs`

- HTTP client for calling ccode autonomous API
- Type-safe request/response structs
- Error handling and retries

### Phase 2: CLOSE Framework Rust (Complete)

**File**: `services/zero-workflow/src/hands/close.rs`

- CLOSE criteria evaluation
- Autonomy level thresholds
- Pre-built criteria templates
- Score calculation matching TypeScript implementation

### Phase 3: Enhanced Executor (Complete)

**File**: `services/zero-workflow/src/hands/executor.rs`

- Checks for autonomy config in manifest
- Uses autonomous bridge when configured
- Falls back to simple agent call when not
- Records CLOSE scores in execution metadata
- Enhanced memory output with CLOSE evaluation

### Phase 3: Config Integration (Complete)

**File**: `services/zero-common/src/config.rs`

- Added `AutonomousConfig` struct
- Added to `WorkflowConfig.autonomous`
- Default endpoint: `http://127.0.0.1:4400/api/v1/autonomous/execute`

### Phase 4: Example Hands (Complete)

Created example HAND.md files:
1. `~/.codecoder/hands/market-sentinel/HAND.md` - Market monitoring with CLOSE
2. `~/.codecoder/hands/research-agent/HAND.md` - Autonomous research agent
3. `~/.codecoder/hands/simple-task/HAND.md` - Simple task (no autonomy)

## Enhanced HAND.md Format

```yaml
---
id: "market-sentinel"
name: "Market Sentinel"
version: "1.0.0"
schedule: "0 */30 * * * *"
agent: "macro"
enabled: false

# Autonomous Configuration
autonomy:
  level: "crazy"         # lunatic/insane/crazy/wild/bold/timid
  unattended: true       # No human interaction
  max_iterations: 5      # Max evolution retries

# Decision Configuration
decision:
  use_close: true        # Use CLOSE framework
  web_search: true       # Enable web search
  evolution: true        # Enable evolution loop
  auto_continue: true    # Auto-continue execution

# Resource Limits
resources:
  max_tokens: 100000
  max_cost_usd: 5.0
  max_duration_sec: 600

# Memory output
memory_path: "hands/market-sentinel/{date}.md"
---
```

## Autonomy Levels

| Level | Approval | Caution | Description |
|-------|----------|---------|-------------|
| Lunatic | 5.0 | 3.0 | 完全自主 - 无需人工干预 |
| Insane | 5.5 | 3.5 | 高度自主 - 关键决策前通知 |
| Crazy | 6.0 | 4.0 | 显著自主 - 半自动执行 |
| Wild | 6.5 | 4.5 | 部分自主 - 仅执行简单任务 |
| Bold | 7.0 | 5.0 | 谨慎自主 - 仅执行已定义步骤 |
| Timid | 8.0 | 6.0 | 基本不自主 - 仅收集信息 |

## Testing

```bash
# Start ccode API server
bun dev serve

# Start zero-workflow service
./ops.sh start all

# List hands
curl http://localhost:4432/api/v1/hands

# Trigger a hand
curl -X POST http://localhost:4432/api/v1/hands/market-sentinel/trigger

# View execution history
curl http://localhost:4432/api/v1/hands/market-sentinel/executions
```

## Next Steps

1. ~~**Guardrails**: Add HITL confirmation for sensitive operations~~ ✅ Done
2. **Testing**: Run integration tests with actual hands
3. **Monitoring**: Add observability for CLOSE scores
4. **Evolution**: Implement knowledge sedimentation from autonomous runs
5. **UI**: Add autonomy controls to Hands dashboard

## Session 2 Updates (2026-02-27)

### Guardrails Module (Complete)

**File**: `services/zero-common/src/guardrails.rs`

Added Human-In-The-Loop (HITL) confirmation inspired by OpenFang's approval gates:

- **Risk Levels**: Safe, Low, Medium, High, Critical
- **Action Classification**: Browser, FileSystem, ExternalApi, Financial, CodeExecution, DataDeletion
- **Approval Workflow**: Pending approvals with timeout
- **IM Notifications**: Send approval requests via channels service

**Risk Threshold Mapping by Autonomy Level**:
| Level | Threshold | Allowed Without Confirmation |
|-------|-----------|------------------------------|
| Lunatic | 4 | Everything except Critical |
| Insane | 3 | Safe, Low, Medium, High |
| Crazy | 2 | Safe, Low, Medium |
| Wild | 2 | Safe, Low, Medium |
| Bold | 1 | Safe, Low |
| Timid | 0 | Safe only |

### Example Hands (Complete)

**Location**: `docs/hands-examples/`

Created three example HAND.md files demonstrating different use cases:

1. **market-sentinel** - Market monitoring with CLOSE decision framework
   - Schedule: 09:30, 14:00, 15:00 (trading hours)
   - Agent: macro
   - Autonomy: crazy (moderate autonomy)

2. **daily-digest** - Daily tech news aggregation
   - Schedule: 08:00 daily
   - Agent: writer
   - Autonomy: wild (low autonomy)

3. **close-decision** - Daily decision quality review
   - Schedule: 21:00 daily
   - Agent: decision
   - Autonomy: bold (cautious, non-unattended)

To install examples:
```bash
cp -r docs/hands-examples/* ~/.codecoder/hands/
```

### Updated lib.rs Exports

**File**: `services/zero-common/src/lib.rs`

Added exports:
- `pub mod guardrails;`
- `pub use guardrails::{Action, ActionCategory, ApprovalRequest, ApprovalStatus, Decision, Guardrails, GuardrailsConfig, RiskLevel};`

## Files Modified

### TypeScript
- `packages/ccode/src/api/server/handlers/autonomous.ts` (new)
- `packages/ccode/src/api/server/router.ts` (modified)

### Rust
- `services/zero-workflow/src/hands/manifest.rs` (modified)
- `services/zero-workflow/src/hands/executor.rs` (modified)
- `services/zero-workflow/src/hands/autonomous_bridge.rs` (new)
- `services/zero-workflow/src/hands/close.rs` (new)
- `services/zero-workflow/src/hands/mod.rs` (modified)
- `services/zero-common/src/config.rs` (modified)
- `services/zero-common/src/guardrails.rs` (new - Session 2)
- `services/zero-common/src/lib.rs` (modified - Session 2)

### Documentation
- `docs/hands-examples/market-sentinel/HAND.md` (new - Session 2)
- `docs/hands-examples/daily-digest/HAND.md` (new - Session 2)
- `docs/hands-examples/close-decision/HAND.md` (new - Session 2)
