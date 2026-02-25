# Chrome DevTools MCP Migration Report

**Date:** 2026-02-25
**Status:** Completed

## Summary

Migrated browser automation MCP integration from Playwright MCP (`@executeautomation/playwright-mcp-server`) to Chrome DevTools MCP (`chrome-devtools-mcp`).

## Background

The previous Playwright MCP server was functional but Chrome DevTools MCP offers:
- Official Google Chrome team maintenance
- Direct Chrome DevTools Protocol integration
- Built-in performance analysis tools (trace recording, insights)
- Network/CPU emulation capabilities
- Better stability with Chrome-specific features

## Changes Made

### 1. Security Policy (`packages/ccode/src/security/remote-policy.ts`)

**DANGEROUS_OPERATIONS updated:**
- Replaced `mcp__playwright__browser_*` with `mcp__chrome_devtools__*`
- New operations: `navigate_page`, `click`, `fill`, `fill_form`, `upload_file`, `evaluate_script`, `drag`, `handle_dialog`, `new_page`, `close_page`

**SAFE_OPERATIONS updated:**
- Replaced read-only Playwright tools with Chrome DevTools equivalents
- Added new performance tools:
  - `performance_start_trace`
  - `performance_stop_trace`
  - `performance_analyze_insight`
  - `emulate_cpu`
  - `emulate_network`

**describeApprovalReason() updated:**
- Updated switch cases to handle Chrome DevTools tool names
- Updated default case pattern matching

### 2. Agent Prompt (`packages/ccode/src/agent/prompt/general.txt`)

- Updated MCP tool prefix examples from `mcp__playwright__` to `mcp__chrome_devtools__`
- Updated tool name examples (e.g., `browser_navigate` -> `navigate_page`)
- Added documentation for new performance analysis tools

### 3. MCP Guide Documentation (`docs/standards/mcp-guide.md`)

- Replaced Playwright MCP section with comprehensive Chrome DevTools MCP section
- Added requirements (Node.js v20.19+, Chrome stable)
- Added complete tool mapping table (Playwright -> Chrome DevTools)
- Added performance analysis feature documentation
- Updated Available MCP Servers table
- Updated Agent-Specific MCP Access example

## Tool Name Mapping Reference

| Playwright MCP | Chrome DevTools MCP |
|----------------|---------------------|
| `browser_navigate` | `navigate_page` |
| `browser_click` | `click` |
| `browser_type` | `fill` |
| `browser_fill_form` | `fill_form` |
| `browser_file_upload` | `upload_file` |
| `browser_evaluate` | `evaluate_script` |
| `browser_drag` | `drag` |
| `browser_handle_dialog` | `handle_dialog` |
| `browser_snapshot` | `take_snapshot` |
| `browser_take_screenshot` | `take_screenshot` |
| `browser_console_messages` | `list_console_messages` |
| `browser_network_requests` | `list_network_requests` |
| `browser_tabs` | `list_pages` |
| `browser_wait_for` | `wait_for` |
| `browser_hover` | `hover` |
| `browser_resize` | `resize_page` |
| `browser_press_key` | (use `evaluate_script`) |

## New Capabilities

Chrome DevTools MCP adds these capabilities not available in Playwright MCP:

1. **Performance Tracing**: Record and analyze performance traces
2. **Performance Insights**: AI-powered analysis of recorded traces
3. **CPU Throttling**: Simulate slower devices
4. **Network Throttling**: Simulate various network conditions
5. **Network Request Details**: Get detailed request/response data

## Configuration

Users should update their `~/.codecoder/config.jsonc`:

```jsonc
{
  "mcp": {
    "chrome-devtools": {
      "type": "local",
      "command": ["npx", "chrome-devtools-mcp@latest"],
      "enabledAgents": ["code-reverse", "general", "build"],
      "timeout": 60000
    }
  }
}
```

## Verification Checklist

- [x] Security policy updated with new tool names
- [x] Agent prompt updated with new references
- [x] MCP documentation updated
- [ ] User testing with `code-reverse` agent
- [ ] User testing with `general` agent

## E2E Testing Note

The `@playwright/test` framework remains unchanged for E2E testing in `packages/web/`. This migration only affects the MCP integration layer for AI agent browser automation, not the test framework itself.

## Dependencies

No package.json changes required - Chrome DevTools MCP is invoked via `npx` at runtime.
