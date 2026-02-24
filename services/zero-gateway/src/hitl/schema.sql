-- HitL (Human-in-the-Loop) Approval System Schema
-- SQLite database schema for storing approval requests and audit logs

-- Main approval requests table
CREATE TABLE IF NOT EXISTS hitl_requests (
    id TEXT PRIMARY KEY,
    approval_type TEXT NOT NULL,
    requester_id TEXT NOT NULL,
    approvers TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    decided_by TEXT,
    decided_at TEXT,
    rejection_reason TEXT,
    channel_type TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    message_id TEXT,
    context TEXT,
    title TEXT NOT NULL,
    description TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for querying by status (most common query pattern)
CREATE INDEX IF NOT EXISTS idx_hitl_status ON hitl_requests(status);

-- Index for querying by requester
CREATE INDEX IF NOT EXISTS idx_hitl_requester ON hitl_requests(requester_id);

-- Composite index for channel-based queries
CREATE INDEX IF NOT EXISTS idx_hitl_channel ON hitl_requests(channel_type, channel_id);

-- Index for expiration queries
CREATE INDEX IF NOT EXISTS idx_hitl_expires ON hitl_requests(expires_at) WHERE expires_at IS NOT NULL;

-- Audit log table for tracking all changes to approval requests
CREATE TABLE IF NOT EXISTS hitl_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    details TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for querying audit log by request
CREATE INDEX IF NOT EXISTS idx_hitl_audit_request ON hitl_audit_log(request_id);

-- Index for querying audit log by actor
CREATE INDEX IF NOT EXISTS idx_hitl_audit_actor ON hitl_audit_log(actor_id);

-- Index for time-based audit queries
CREATE INDEX IF NOT EXISTS idx_hitl_audit_timestamp ON hitl_audit_log(timestamp);
