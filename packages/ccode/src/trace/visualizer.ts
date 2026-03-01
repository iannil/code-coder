/**
 * Trace Visualizer
 * Provides ASCII tree and text formatting for trace entries
 */

import type { LogEntry } from "../observability"

// ============================================================================
// Types
// ============================================================================

interface SpanNode {
  entry: LogEntry
  children: SpanNode[]
  duration?: number
}

// ============================================================================
// Tree Building
// ============================================================================

/**
 * Build a tree structure from flat log entries
 */
function buildSpanTree(entries: LogEntry[]): SpanNode[] {
  const roots: SpanNode[] = []
  const nodeMap = new Map<string, SpanNode>()

  // First pass: create nodes for function_start events
  for (const entry of entries) {
    if (entry.event_type === "function_start") {
      const node: SpanNode = { entry, children: [] }
      nodeMap.set(entry.span_id, node)
    }
  }

  // Second pass: link parent/child and add duration
  for (const entry of entries) {
    if (entry.event_type === "function_end") {
      const node = nodeMap.get(entry.span_id)
      if (node && entry.payload?.duration_ms !== undefined) {
        node.duration = entry.payload.duration_ms as number
      }
    }
  }

  // Third pass: build tree structure
  for (const [spanId, node] of nodeMap) {
    const parentId = node.entry.parent_span_id
    if (parentId && nodeMap.has(parentId)) {
      nodeMap.get(parentId)!.children.push(node)
    } else {
      roots.push(node)
    }
  }

  // Sort children by timestamp
  const sortChildren = (node: SpanNode): void => {
    node.children.sort((a, b) => new Date(a.entry.ts).getTime() - new Date(b.entry.ts).getTime())
    node.children.forEach(sortChildren)
  }
  roots.forEach(sortChildren)

  return roots.sort((a, b) => new Date(a.entry.ts).getTime() - new Date(b.entry.ts).getTime())
}

// ============================================================================
// Formatters
// ============================================================================

/**
 * Format entries as ASCII tree
 */
export function formatAsTree(entries: LogEntry[]): string {
  const roots = buildSpanTree(entries)
  const lines: string[] = []

  const renderNode = (node: SpanNode, prefix: string, isLast: boolean): void => {
    const connector = isLast ? "└── " : "├── "
    const funcName = node.entry.payload?.function ?? node.entry.event_type
    const service = node.entry.service
    const duration = node.duration !== undefined ? ` (${node.duration}ms)` : ""

    lines.push(`${prefix}${connector}${funcName}${duration} [${service}]`)

    const childPrefix = prefix + (isLast ? "    " : "│   ")
    node.children.forEach((child, i) => {
      renderNode(child, childPrefix, i === node.children.length - 1)
    })
  }

  roots.forEach((root, i) => {
    renderNode(root, "", i === roots.length - 1)
  })

  return lines.join("\n")
}

/**
 * Format entries as text table
 */
export function formatAsText(entries: LogEntry[]): string {
  const lines: string[] = []

  // Header
  lines.push("")
  lines.push("Timestamp                    Level   Service          Event Type       Function/Details")
  lines.push("-".repeat(100))

  for (const entry of entries) {
    const ts = new Date(entry.ts).toISOString()
    const level = entry.level.toUpperCase().padEnd(7)
    const service = entry.service.padEnd(16)
    const eventType = entry.event_type.padEnd(16)

    let details = ""
    if (entry.payload?.function) {
      details = entry.payload.function as string
      if (entry.payload?.duration_ms !== undefined) {
        details += ` (${entry.payload.duration_ms}ms)`
      }
    } else if (entry.payload?.error) {
      details = `ERROR: ${entry.payload.error}`
    } else if (entry.payload?.method && entry.payload?.path) {
      details = `${entry.payload.method} ${entry.payload.path}`
      if (entry.payload?.status) {
        details += ` -> ${entry.payload.status}`
      }
    }

    lines.push(`${ts}  ${level} ${service} ${eventType} ${details}`)
  }

  lines.push("")
  lines.push(`Total entries: ${entries.length}`)

  return lines.join("\n")
}
