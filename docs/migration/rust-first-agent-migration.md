# Agent Module Migration Guide

> Migration from deprecated `@/agent/agent` to Rust-powered `AgentBridge`

## Overview

The TypeScript `Agent` module (`packages/ccode/src/agent/agent.ts`) has been deprecated in favor of the Rust-powered Agent API. This guide helps you migrate existing code to use the new `AgentBridge` SDK.

## Timeline

- **v1.x**: Deprecated module works but logs warnings
- **v2.0**: Module will be removed entirely

## Quick Migration Reference

| Deprecated (TypeScript) | New (AgentBridge) |
|------------------------|-------------------|
| `Agent.list()` | `bridge.list()` |
| `Agent.get(name)` | `bridge.get(name)` |
| `Agent.defaultAgent()` | `bridge.defaultAgent()` |
| `Agent.generate({...})` | `bridge.generate({...})` |

## Detailed Migration

### Setup

```typescript
// Before
import { Agent } from "@/agent/agent"

// After
import { getAgentBridge, toAgentInfo } from "@/sdk/agent-bridge"
const bridge = await getAgentBridge()
```

### List Agents

```typescript
// Before
const agents = await Agent.list()

// After
const agents = await bridge.list()
// Note: Returns AgentInfo[] with auto_approve and observer fields
```

### Get Agent by Name

```typescript
// Before
const agent = await Agent.get("build")

// After
const agent = await bridge.get("build")
// Returns undefined if not found (no exception)
```

### Get Default Agent

```typescript
// Before
const name = await Agent.defaultAgent()

// After
const name = await bridge.defaultAgent()
```

### Generate Agent via AI

```typescript
// Before
import { Agent } from "@/agent/agent"
import { Provider } from "@/provider/provider"

const model = Provider.parseModel("anthropic/claude-sonnet-4-5")
const result = await Agent.generate({
  description: "An agent that helps with testing",
  model
})

// After
const bridge = await getAgentBridge()
const result = await bridge.generate({
  description: "An agent that helps with testing",
  model: "anthropic/claude-sonnet-4-5" // Optional, string format
})
// Returns: { identifier, whenToUse, systemPrompt }
```

### Convert to Legacy Format

If you need the legacy `Agent.Info` format for compatibility:

```typescript
import { toAgentInfo } from "@/sdk/agent-bridge"

const agent = await bridge.get("explore")
const legacyFormat = toAgentInfo(agent)
// legacyFormat has camelCase fields: autoApprove, observerCapability
```

## New Features in AgentBridge

### Auto-Approve Configuration

```typescript
const agent = await bridge.get("explore")
console.log(agent.auto_approve)
// {
//   enabled: true,
//   allowed_tools: ["Read", "Glob", "Grep", "LS"],
//   risk_threshold: "low"
// }
```

### Observer Capability

```typescript
const agent = await bridge.get("explore")
console.log(agent.observer)
// {
//   can_watch: ["code"],
//   contribute_to_consensus: true,
//   report_to_meta: true
// }
```

## HTTP API Reference

The AgentBridge SDK wraps these Rust daemon endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/agents` | GET | List all visible agents |
| `/api/v1/agents/:name` | GET | Get agent details |
| `/api/v1/agents/:name/prompt` | GET | Get agent's system prompt |
| `/api/v1/definitions/agents/generate` | POST | Generate agent via AI |

## Troubleshooting

### "LLM provider not configured"

The `generate()` method requires the daemon to have an LLM provider configured. Ensure one of these environment variables is set:

- `ANTHROPIC_API_KEY`
- `OPENAI_API_KEY`
- `GOOGLE_API_KEY`

### "Agent not found"

The agent may be hidden or not loaded. Check:
1. Agent prompt file exists in `src/agent/prompt/*.txt`
2. Agent is not marked as `hidden: true`
3. Daemon is running: `curl http://localhost:4402/health`

## Related Documentation

- [Rust-First Architecture Plan](/docs/progress/rust-first-refactor.md)
- [AgentBridge SDK](/packages/ccode/src/sdk/agent-bridge.ts)
- [Rust Agent API](/services/zero-cli/src/unified_api/agents.rs)
