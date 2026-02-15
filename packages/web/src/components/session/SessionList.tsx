/**
 * SessionList Component
 *
 * Displays a list of all sessions with:
 * - Filter/search functionality
 * - Session groups by date (Today, Yesterday, This Week, Older)
 * - Loading states
 * - Empty state
 */

import * as React from "react"
import { Search, X, Loader2, FolderOpen } from "lucide-react"

import { cn } from "@/lib/utils"
import { Input } from "@/components/ui/Input"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Separator } from "@/components/ui/Separator"
import { SessionItem } from "./SessionItem"
import { useSessions, useSessionsLoading } from "@/stores/session"
import type { SessionInfo } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

export interface SessionListProps {
  /** Currently active session ID */
  activeSessionId?: string | null

  /** Callback when a session is clicked */
  onSessionClick?: (sessionId: string) => void

  /** Callback when delete is triggered for a session */
  onSessionDelete?: (sessionId: string) => void

  /** Callback when rename is triggered for a session */
  onSessionRename?: (sessionId: string) => void

  /** Optional CSS class name */
  className?: string
}

interface SessionGroup {
  label: string
  sessions: SessionInfo[]
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Check if a date is today
 */
function isToday(timestamp: number): boolean {
  const date = new Date(timestamp)
  const today = new Date()
  return (
    date.getDate() === today.getDate() &&
    date.getMonth() === today.getMonth() &&
    date.getFullYear() === today.getFullYear()
  )
}

/**
 * Check if a date is yesterday
 */
function isYesterday(timestamp: number): boolean {
  const date = new Date(timestamp)
  const yesterday = new Date()
  yesterday.setDate(yesterday.getDate() - 1)
  return (
    date.getDate() === yesterday.getDate() &&
    date.getMonth() === yesterday.getMonth() &&
    date.getFullYear() === yesterday.getFullYear()
  )
}

/**
 * Check if a date is within the last week
 */
function isThisWeek(timestamp: number): boolean {
  const date = new Date(timestamp)
  const weekAgo = new Date()
  weekAgo.setDate(weekAgo.getDate() - 7)
  return date > weekAgo
}

/**
 * Group sessions by date category
 */
function groupSessionsByDate(sessions: SessionInfo[]): SessionGroup[] {
  const groups: Record<string, SessionInfo[]> = {
    Today: [],
    Yesterday: [],
    "This Week": [],
    Older: [],
  }

  for (const session of sessions) {
    const timestamp = session.time.updated ?? session.time.created

    if (isToday(timestamp)) {
      groups.Today.push(session)
    } else if (isYesterday(timestamp)) {
      groups.Yesterday.push(session)
    } else if (isThisWeek(timestamp)) {
      groups["This Week"].push(session)
    } else {
      groups.Older.push(session)
    }
  }

  // Convert to array and filter out empty groups
  return Object.entries(groups)
    .filter(([, sessions]) => sessions.length > 0)
    .map(([label, sessions]) => ({ label, sessions }))
}

/**
 * Filter sessions by search query
 */
function filterSessions(sessions: SessionInfo[], query: string): SessionInfo[] {
  if (!query.trim()) return sessions

  const lowerQuery = query.toLowerCase()
  return sessions.filter(
    (session) =>
      session.title.toLowerCase().includes(lowerQuery) ||
      session.id.toLowerCase().includes(lowerQuery)
  )
}

/**
 * Sort sessions by updated time (newest first)
 */
function sortSessions(sessions: SessionInfo[]): SessionInfo[] {
  return [...sessions].sort(
    (a, b) =>
      (b.time.updated ?? b.time.created) - (a.time.updated ?? a.time.created)
  )
}

// ============================================================================
// Component
// ============================================================================

export const SessionList = React.forwardRef<HTMLDivElement, SessionListProps>(
  (
    {
      activeSessionId = null,
      onSessionClick,
      onSessionDelete,
      onSessionRename,
      className,
    },
    ref
  ) => {
    const sessions = useSessions()
    const { isLoading } = useSessionsLoading()
    const [searchQuery, setSearchQuery] = React.useState("")

    // Filter and sort sessions
    const filteredSessions = React.useMemo(() => {
      return sortSessions(filterSessions(sessions, searchQuery))
    }, [sessions, searchQuery])

    // Group sessions by date
    const groupedSessions = React.useMemo(() => {
      return groupSessionsByDate(filteredSessions)
    }, [filteredSessions])

    // Handle search input
    const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      setSearchQuery(e.target.value)
    }

    // Clear search query
    const handleClearSearch = () => {
      setSearchQuery("")
    }

    // Handle session click
    const handleSessionClick = (sessionId: string) => {
      onSessionClick?.(sessionId)
    }

    // Handle session delete
    const handleSessionDelete = (sessionId: string) => {
      onSessionDelete?.(sessionId)
    }

    // Handle session rename
    const handleSessionRename = (sessionId: string) => {
      onSessionRename?.(sessionId)
    }

    return (
      <div ref={ref} className={cn("flex flex-col h-full", className)}>
        {/* Search Header */}
        <div className="flex-shrink-0 p-4 border-b">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              type="text"
              placeholder="Search sessions..."
              value={searchQuery}
              onChange={handleSearchChange}
              className="pl-9 pr-9 h-9"
            />
            {searchQuery && (
              <button
                type="button"
                onClick={handleClearSearch}
                className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
              >
                <X className="h-4 w-4" />
              </button>
            )}
          </div>
        </div>

        {/* Sessions List */}
        <ScrollArea className="flex-1">
          <div className="p-2">
            {isLoading ? (
              // Loading state
              <div className="flex items-center justify-center py-12">
                <div className="flex flex-col items-center gap-3 text-muted-foreground">
                  <Loader2 className="h-6 w-6 animate-spin" />
                  <span className="text-sm">Loading sessions...</span>
                </div>
              </div>
            ) : filteredSessions.length === 0 ? (
              // Empty state
              <div className="flex flex-col items-center justify-center py-12 text-center">
                {searchQuery ? (
                  <>
                    <div className="flex h-12 w-12 items-center justify-center rounded-full bg-muted mb-3">
                      <Search className="h-5 w-5 text-muted-foreground" />
                    </div>
                    <p className="text-sm font-medium">No sessions found</p>
                    <p className="text-xs text-muted-foreground mt-1">
                      Try a different search term
                    </p>
                  </>
                ) : (
                  <>
                    <div className="flex h-12 w-12 items-center justify-center rounded-full bg-muted mb-3">
                      <FolderOpen className="h-5 w-5 text-muted-foreground" />
                    </div>
                    <p className="text-sm font-medium">No sessions yet</p>
                    <p className="text-xs text-muted-foreground mt-1">
                      Create a new session to get started
                    </p>
                  </>
                )}
              </div>
            ) : (
              // Session groups
              <div className="space-y-4">
                {groupedSessions.map((group, groupIndex) => (
                  <div key={group.label}>
                    {/* Group label */}
                    <div className="px-3 py-1.5">
                      <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                        {group.label}
                      </span>
                    </div>

                    {/* Group sessions */}
                    <div className="space-y-1">
                      {group.sessions.map((session) => (
                        <SessionItem
                          key={session.id}
                          session={session}
                          isActive={session.id === activeSessionId}
                          onClick={() => handleSessionClick(session.id)}
                          onDelete={() => handleSessionDelete(session.id)}
                          onRename={() => handleSessionRename(session.id)}
                        />
                      ))}
                    </div>

                    {/* Separator between groups */}
                    {groupIndex < groupedSessions.length - 1 && (
                      <div className="my-3">
                        <Separator />
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </ScrollArea>

        {/* Session count footer */}
        {!isLoading && filteredSessions.length > 0 && (
          <div className="flex-shrink-0 px-4 py-2 border-t text-xs text-muted-foreground">
            {filteredSessions.length}{" "}
            {filteredSessions.length === 1 ? "session" : "sessions"}
            {searchQuery && " filtered"}
          </div>
        )}
      </div>
    )
  }
)

SessionList.displayName = "SessionList"
