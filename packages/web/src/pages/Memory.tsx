/**
 * Memory Page
 *
 * Full-page memory system management with:
 * - Daily notes browsing
 * - Long-term memory viewing/editing
 * - Consolidation controls and statistics
 */

import * as React from "react"
import { Brain } from "lucide-react"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { MemoryPanel } from "@/components/memory"
import { useMemoryStore, useMemorySummary } from "@/stores"

// ============================================================================
// Memory Stats Component
// ============================================================================

function MemoryStats() {
  const summary = useMemorySummary()
  const { fetchSummary } = useMemoryStore()

  React.useEffect(() => {
    fetchSummary()
  }, [fetchSummary])

  const formatSize = (bytes?: number) => {
    if (!bytes) return "0 KB"
    if (bytes < 1024) return `${bytes} B`
    return `${(bytes / 1024).toFixed(1)} KB`
  }

  const formatDate = (ts?: number) => {
    if (!ts) return "Never"
    return new Date(ts).toLocaleDateString("zh-CN", {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    })
  }

  return (
    <div className="grid grid-cols-3 gap-4">
      <Card>
        <CardContent className="p-4">
          <div className="text-center">
            <p className="text-2xl font-bold">{summary?.dailyNotesCount ?? 0}</p>
            <p className="text-sm text-muted-foreground">Daily Notes</p>
          </div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="text-center">
            <p className="text-2xl font-bold">{formatSize(summary?.longTermSize)}</p>
            <p className="text-sm text-muted-foreground">Long-term Memory</p>
          </div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="text-center">
            <p className="text-lg font-medium truncate">{formatDate(summary?.lastUpdated)}</p>
            <p className="text-sm text-muted-foreground">Last Updated</p>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Memory Header Component
// ============================================================================

function MemoryHeader() {
  return (
    <div className="flex items-center gap-3 p-4 border-b bg-muted/30">
      <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
        <Brain className="h-5 w-5 text-primary" />
      </div>
      <div>
        <h1 className="text-lg font-semibold">Memory System</h1>
        <p className="text-sm text-muted-foreground">
          Manage daily notes and long-term memory
        </p>
      </div>
    </div>
  )
}

// ============================================================================
// Main Memory Page Component
// ============================================================================

export function Memory() {
  return (
    <div data-testid="memory-panel" className="flex flex-col h-full bg-background">
      <MemoryHeader />
      <div className="flex-1 overflow-auto p-6">
        <div className="max-w-4xl mx-auto space-y-6">
          {/* Stats Overview */}
          <MemoryStats />

          {/* Memory Panel with Tabs */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Brain className="h-5 w-5 text-primary" />
                Memory Browser
              </CardTitle>
              <CardDescription>
                Browse daily notes, view long-term memory, and manage consolidation
              </CardDescription>
            </CardHeader>
            <CardContent>
              <MemoryPanel />
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}

export default Memory
