/**
 * SessionItem Component
 *
 * Displays a single session in the session list with:
 * - Session title with truncation for long titles
 * - Timestamp (created/updated time)
 * - Active state highlighting
 * - Hover actions (delete, rename)
 */

import * as React from "react"
import { Trash2, Edit3, Clock, MessageSquare } from "lucide-react"

import { cn } from "@/lib/utils"
import { formatTimestamp } from "@/lib/utils"
import { useSessionDeleting } from "@/stores/session"
import type { SessionInfo } from "@/lib/types"

// ============================================================================
// Props Interface
// ============================================================================

export interface SessionItemProps {
  /** The session to display */
  session: SessionInfo

  /** Whether this session is currently active */
  isActive?: boolean

  /** Callback when the session is clicked */
  onClick?: () => void

  /** Callback when delete is triggered */
  onDelete?: () => void

  /** Callback when rename is triggered */
  onRename?: () => void

  /** Optional CSS class name */
  className?: string
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Truncate a string to a maximum length and add ellipsis
 */
function truncateTitle(title: string, maxLength = 40): string {
  if (title.length <= maxLength) return title
  return title.slice(0, maxLength - 3) + "..."
}

/**
 * Get the appropriate timestamp to display
 */
function getSessionTimestamp(session: SessionInfo): number {
  // Prefer updated time, fallback to created time
  return session.time.updated ?? session.time.created
}

// ============================================================================
// Component
// ============================================================================

export const SessionItem = React.forwardRef<HTMLDivElement, SessionItemProps>(
  (
    {
      session,
      isActive = false,
      onClick,
      onDelete,
      onRename,
      className,
    },
    ref
  ) => {
    const isDeleting = useSessionDeleting(session.id)
    const title = truncateTitle(session.title)
    const timestamp = getSessionTimestamp(session)

    // Handle action clicks (stop propagation to avoid triggering onClick)
    const handleDeleteClick = (e: React.MouseEvent) => {
      e.stopPropagation()
      onDelete?.()
    }

    const handleRenameClick = (e: React.MouseEvent) => {
      e.stopPropagation()
      onRename?.()
    }

    return (
      <div
        ref={ref}
        onClick={onClick}
        className={cn(
          "group relative flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors",
          "cursor-pointer select-none",
          isActive
            ? "bg-primary/10 text-primary font-medium"
            : "hover:bg-accent hover:text-accent-foreground",
          isDeleting && "opacity-50 pointer-events-none",
          className
        )}
        data-session-id={session.id}
        data-active={isActive}
      >
        {/* Icon */}
        <div
          className={cn(
            "flex-shrink-0",
            isActive ? "text-primary" : "text-muted-foreground"
          )}
        >
          <MessageSquare className="h-4 w-4" />
        </div>

        {/* Content */}
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span
              className="truncate"
              title={session.title}
            >
              {title}
            </span>
          </div>
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <Clock className="h-3 w-3 flex-shrink-0" />
            <span className="truncate">{formatTimestamp(timestamp)}</span>
          </div>
        </div>

        {/* Hover Actions */}
        <div
          className={cn(
            "flex items-center gap-1 opacity-0 transition-opacity",
            "group-hover:opacity-100",
            isActive && "opacity-100"
          )}
        >
          <button
            type="button"
            onClick={handleRenameClick}
            className={cn(
              "flex-shrink-0 rounded p-1 transition-colors",
              "hover:bg-accent-foreground/10",
              "text-muted-foreground hover:text-foreground"
            )}
            title="Rename session"
            disabled={isDeleting}
          >
            <Edit3 className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={handleDeleteClick}
            className={cn(
              "flex-shrink-0 rounded p-1 transition-colors",
              "hover:bg-destructive/10",
              "text-muted-foreground hover:text-destructive"
            )}
            title="Delete session"
            disabled={isDeleting}
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        </div>

        {/* Deleting indicator */}
        {isDeleting && (
          <div className="absolute inset-0 flex items-center justify-center rounded-lg bg-background/50">
            <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          </div>
        )}
      </div>
    )
  }
)

SessionItem.displayName = "SessionItem"
