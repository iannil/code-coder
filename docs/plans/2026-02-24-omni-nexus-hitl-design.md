# Omni-Nexus HitL (Human-in-the-Loop) Design Document

**Date:** 2026-02-24
**Status:** Approved
**Author:** Claude + Human

---

## 1. Overview

This document details the technical design for implementing the Human-in-the-Loop (HitL) system as part of Omni-Nexus Phase 4. The HitL system addresses NFR-01 from `docs/standards/goals.md`: critical operations must pause automation and await human confirmation.

### 1.1 Goals

- Implement centralized approval workflow in `zero-gateway`
- Support interactive cards across 4 IM channels: Telegram, Feishu, Slack, DingTalk
- Persist approval state in SQLite with full audit trail
- Integrate with existing workflow components (review_bridge, risk_monitor, trading_review)

### 1.2 Non-Goals

- Timeout/auto-cancel (user chose "No Timeout" - requests stay pending indefinitely)
- Multi-approver consensus (single approver sufficient for MVP)
- Web UI for approval management (Phase 5 scope)

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          zero-gateway                                    │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                      HitL Service Module                           │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐   │ │
│  │  │ ApprovalStore│  │ CardRenderer │  │ ActionRegistry         │   │ │
│  │  │  (SQLite)    │  │ (per-channel)│  │ (post-approval exec)   │   │ │
│  │  └──────────────┘  └──────────────┘  └────────────────────────┘   │ │
│  │         │                 │                     │                  │ │
│  │         ▼                 ▼                     ▼                  │ │
│  │  ┌─────────────────────────────────────────────────────────────┐  │ │
│  │  │                    HitL API Routes                          │  │ │
│  │  │  POST /api/v1/hitl/request     - Create approval request    │  │ │
│  │  │  GET  /api/v1/hitl/pending     - List pending for user      │  │ │
│  │  │  GET  /api/v1/hitl/:id         - Get request status         │  │ │
│  │  │  POST /api/v1/hitl/:id/decide  - Process decision           │  │ │
│  │  │  POST /api/v1/hitl/callback/:channel - Channel webhooks     │  │ │
│  │  └─────────────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
          │                           ▲
          │ Send Card                 │ Callback
          ▼                           │
┌─────────────────────────────────────────────────────────────────────────┐
│                          zero-channels                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  Telegram   │  │   Feishu    │  │    Slack    │  │  DingTalk   │    │
│  │ InlineKeybd │  │ MessageCard │  │  Block Kit  │  │ ActionCard  │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Data Model

### 3.1 Core Types

```rust
/// Type of operation requiring approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalType {
    MergeRequest { platform: String, repo: String, mr_id: i64 },
    TradingCommand { asset: String, action: String, amount: f64 },
    ConfigChange { key: String, old_value: String, new_value: String },
    HighCostOperation { operation: String, estimated_cost: f64 },
    RiskOperation { description: String, risk_level: RiskLevel },
}

/// Risk level classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,      // Info only
    Medium,   // Single approver
    High,     // Requires specific role
    Critical, // Requires admin/executive
}

/// Approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,                  // UUID
    pub approval_type: ApprovalType,
    pub requester_id: String,
    pub approvers: Vec<String>,      // Who can approve
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub channel_type: String,        // telegram, feishu, slack, dingtalk
    pub channel_id: String,
    pub message_id: Option<String>,  // Platform message ID
    pub context: serde_json::Value,
}

/// Approval status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved { by: String, at: DateTime<Utc> },
    Rejected { by: String, reason: Option<String>, at: DateTime<Utc> },
    Cancelled { reason: String },
}
```

### 3.2 SQLite Schema

```sql
CREATE TABLE IF NOT EXISTS hitl_requests (
    id TEXT PRIMARY KEY,
    approval_type TEXT NOT NULL,          -- JSON
    requester_id TEXT NOT NULL,
    approvers TEXT NOT NULL,              -- JSON array
    status TEXT NOT NULL DEFAULT 'pending',
    decided_by TEXT,
    decided_at TEXT,
    rejection_reason TEXT,
    channel_type TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    message_id TEXT,
    context TEXT,                         -- JSON
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_hitl_status ON hitl_requests(status);
CREATE INDEX idx_hitl_requester ON hitl_requests(requester_id);
CREATE INDEX idx_hitl_channel ON hitl_requests(channel_type, channel_id);

CREATE TABLE IF NOT EXISTS hitl_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT NOT NULL REFERENCES hitl_requests(id),
    action TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    details TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_hitl_audit_request ON hitl_audit_log(request_id);
```

---

## 4. API Design

### 4.1 Create Approval Request

```
POST /api/v1/hitl/request
```

**Request:**
```json
{
  "approval_type": {
    "MergeRequest": { "platform": "github", "repo": "owner/repo", "mr_id": 123 }
  },
  "requester_id": "user-123",
  "approvers": ["admin-1", "admin-2"],
  "channel_type": "feishu",
  "channel_id": "oc_xxx",
  "context": { "title": "feat: add login", "review_summary": "Approved" }
}
```

**Response:**
```json
{
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Pending",
  "message_id": "om_xxx",
  "card_url": "https://open.feishu.cn/..."
}
```

### 4.2 Process Decision

```
POST /api/v1/hitl/:id/decide
```

**Request:**
```json
{
  "approved": true,
  "approver_id": "admin-1",
  "reason": "LGTM"
}
```

### 4.3 Channel Callback

```
POST /api/v1/hitl/callback/feishu
```

Receives platform-specific webhook payload, parses it, and routes to `process_decision`.

---

## 5. Interactive Card Formats

### 5.1 Telegram (InlineKeyboard)

Uses existing `TelegramChannel::send_with_inline_keyboard`:

```rust
let buttons = vec![
    vec![
        InlineButton::new("✅ Approve", format!("hitl:approve:{}", request.id)),
        InlineButton::new("❌ Reject", format!("hitl:reject:{}", request.id)),
    ]
];
```

### 5.2 Feishu (飞书消息卡片)

```json
{
  "msg_type": "interactive",
  "card": {
    "header": {
      "title": { "tag": "plain_text", "content": "🔐 审批请求" },
      "template": "orange"
    },
    "elements": [
      {
        "tag": "div",
        "text": { "tag": "lark_md", "content": "**合并请求 #123**\n仓库: owner/repo\n请求人: @user" }
      },
      {
        "tag": "action",
        "actions": [
          {
            "tag": "button",
            "text": { "tag": "plain_text", "content": "✅ 批准" },
            "type": "primary",
            "value": { "action": "approve", "request_id": "xxx" }
          },
          {
            "tag": "button",
            "text": { "tag": "plain_text", "content": "❌ 拒绝" },
            "type": "danger",
            "value": { "action": "reject", "request_id": "xxx" }
          }
        ]
      }
    ]
  }
}
```

### 5.3 Slack (Block Kit)

```json
{
  "blocks": [
    {
      "type": "header",
      "text": { "type": "plain_text", "text": "🔐 Approval Request" }
    },
    {
      "type": "section",
      "text": { "type": "mrkdwn", "text": "*Merge Request #123*\nRepo: owner/repo\nRequester: @user" }
    },
    {
      "type": "actions",
      "elements": [
        {
          "type": "button",
          "text": { "type": "plain_text", "text": "✅ Approve" },
          "style": "primary",
          "action_id": "hitl_approve",
          "value": "request_id"
        },
        {
          "type": "button",
          "text": { "type": "plain_text", "text": "❌ Reject" },
          "style": "danger",
          "action_id": "hitl_reject",
          "value": "request_id"
        }
      ]
    }
  ]
}
```

### 5.4 DingTalk (ActionCard)

```json
{
  "msgtype": "actionCard",
  "actionCard": {
    "title": "🔐 审批请求",
    "text": "### 合并请求 #123\n- 仓库: owner/repo\n- 请求人: @user",
    "btnOrientation": "1",
    "btns": [
      { "title": "✅ 批准", "actionURL": "https://gateway/hitl/callback/dingtalk?action=approve&id=xxx" },
      { "title": "❌ 拒绝", "actionURL": "https://gateway/hitl/callback/dingtalk?action=reject&id=xxx" }
    ]
  }
}
```

---

## 6. Card Renderer Trait

```rust
#[async_trait]
pub trait CardRenderer: Send + Sync {
    /// Send approval card, return message ID
    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        channel_id: &str,
    ) -> Result<String>;

    /// Update existing card after decision
    async fn update_card(
        &self,
        request: &ApprovalRequest,
        message_id: &str,
    ) -> Result<()>;

    /// Parse callback from platform webhook
    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData>;
}

pub struct CallbackData {
    pub request_id: String,
    pub action: CallbackAction,
    pub user_id: String,
    pub platform_callback_id: String,
}

pub enum CallbackAction {
    Approve,
    Reject { reason: Option<String> },
}
```

---

## 7. Workflow Integration

### 7.1 HitL Client

```rust
// services/zero-common/src/hitl_client.rs

pub struct HitLClient {
    endpoint: String,
    client: reqwest::Client,
}

impl HitLClient {
    pub fn new(gateway_endpoint: &str) -> Self;

    pub async fn create_request(&self, req: CreateApprovalRequest) -> Result<ApprovalResponse>;

    pub async fn check_status(&self, request_id: &str) -> Result<ApprovalStatus>;

    pub async fn cancel(&self, request_id: &str, reason: &str) -> Result<()>;
}
```

### 7.2 Integration Points

| Component | Trigger | ApprovalType |
|-----------|---------|--------------|
| `review_bridge` | Code review approved + auto-merge enabled | `MergeRequest` |
| `risk_monitor` | High/Critical margin alert | `RiskOperation` |
| `trading_review` | Trade order submission | `TradingCommand` |
| Config API | Quota/permission changes | `ConfigChange` |
| Cost tracker | Operation exceeds threshold | `HighCostOperation` |

---

## 8. Action Handlers

```rust
#[async_trait]
pub trait ApprovalAction: Send + Sync {
    async fn execute(&self, request: &ApprovalRequest) -> Result<ActionResult>;
    async fn rollback(&self, request: &ApprovalRequest) -> Result<()>;
}

pub struct ActionRegistry {
    handlers: HashMap<String, Arc<dyn ApprovalAction>>,
}
```

**Implementations:**

| ApprovalType | Action |
|--------------|--------|
| `MergeRequest` | Call GitHub/GitLab merge API |
| `TradingCommand` | Execute trade via broker API |
| `ConfigChange` | Apply config change |
| `HighCostOperation` | Proceed with operation |
| `RiskOperation` | Continue workflow |

---

## 9. File Structure

```
services/
├── zero-gateway/
│   └── src/
│       ├── hitl/
│       │   ├── mod.rs           # Module entry, HitLService
│       │   ├── store.rs         # SQLite persistence
│       │   ├── routes.rs        # API endpoints
│       │   ├── actions.rs       # Post-approval execution
│       │   └── cards/
│       │       ├── mod.rs       # CardRenderer trait
│       │       ├── telegram.rs  # Telegram InlineKeyboard
│       │       ├── feishu.rs    # Feishu MessageCard
│       │       ├── slack.rs     # Slack Block Kit
│       │       └── dingtalk.rs  # DingTalk ActionCard
│       └── migrations/
│           └── 003_hitl.sql     # Schema migration
├── zero-common/
│   └── src/
│       └── hitl_client.rs       # Client for other services
└── zero-workflow/
    └── src/
        └── review_bridge.rs     # [MODIFY] Add HitL integration
```

---

## 10. Testing Strategy

- **Unit Tests:** Mock card renderers, test state transitions
- **Integration Tests:** Real SQLite, test full approval flow
- **E2E Tests:** Send actual cards to test channels, verify callbacks
- **Coverage Target:** 80%

---

## 11. Success Criteria

- [ ] Approval cards render correctly on all 4 channels
- [ ] Callbacks correctly update request status
- [ ] Post-approval actions execute successfully
- [ ] Audit log captures all decisions
- [ ] Integration with review_bridge working
- [ ] 80% test coverage

---

## Appendix A: Configuration

```json
{
  "hitl": {
    "enabled": true,
    "default_approvers": ["admin"],
    "channels_endpoint": "http://localhost:4431",
    "callback_base_url": "https://gateway.example.com/api/v1/hitl/callback"
  }
}
```
