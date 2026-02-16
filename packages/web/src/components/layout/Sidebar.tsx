/**
 * Sidebar Component
 *
 * Displays the session sidebar with:
 * - Session list
 * - New session button
 * - Collapsible sections
 * - Active session highlighting
 */

import { Plus, MessageSquare, Trash2, MoreHorizontal, FolderOpen, ChevronRight } from "lucide-react"
import { Button } from "../ui/Button"
import { ScrollArea } from "../ui/ScrollArea"
import { Separator } from "../ui/Separator"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "../ui/DropdownMenu"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../ui/Collapsible"
import {
  useSessions,
  useActiveSessionId,
  useSessionStore,
  useSessionDeleting,
} from "@/stores/session"
import { cn } from "@/lib/utils"
import { useState } from "react"

export interface SidebarProps {
  onNewSession?: () => void
  onSessionSelect?: (sessionId: string) => void
  className?: string
}

interface CollapsibleSectionProps {
  title: string
  defaultOpen?: boolean
  children: React.ReactNode
}

/**
 * Collapsible section component using shadcn Collapsible
 */
function CollapsibleSection({ title, defaultOpen = true, children }: CollapsibleSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen)

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen} className="space-y-1">
      <CollapsibleTrigger asChild>
        <button
          type="button"
          className="flex w-full items-center justify-between px-2 py-1.5 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
        >
          <span>{title}</span>
          <ChevronRight
            className={cn("h-4 w-4 transition-transform", isOpen && "rotate-90")}
          />
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent className="space-y-0.5 pl-2">
        {children}
      </CollapsibleContent>
    </Collapsible>
  )
}

interface SessionItemProps {
  title: string
  isActive: boolean
  isDeleting: boolean
  onClick: () => void
  onDelete: (e: React.MouseEvent) => void
}

/**
 * Individual session item component
 */
function SessionItem({ title, isActive, isDeleting, onClick, onDelete }: SessionItemProps) {
  const [isHovered, setIsHovered] = useState(false)

  return (
    <div
      className={cn(
        "group relative flex items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors",
        isActive
          ? "bg-accent text-accent-foreground"
          : "hover:bg-accent/50 hover:text-accent-foreground",
        isDeleting && "opacity-50"
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <button
        type="button"
        onClick={onClick}
        disabled={isDeleting}
        className="flex flex-1 items-center gap-2 overflow-hidden text-left"
      >
        <MessageSquare className="h-4 w-4 shrink-0" />
        <span className="truncate">{title || "Untitled Session"}</span>
      </button>

      {/* Actions menu */}
      {(isHovered || isActive) && !isDeleting && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild onClick={(e: React.MouseEvent) => e.stopPropagation()}>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
            >
              <MoreHorizontal className="h-3 w-3" />
              <span className="sr-only">Session actions</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
            <DropdownMenuItem
              onClick={(e: React.MouseEvent) => {
                e.stopPropagation()
                onDelete(e)
              }}
              className="text-destructive"
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  )
}

/**
 * Main sidebar component
 */
export function Sidebar({ onNewSession, onSessionSelect, className }: SidebarProps) {
  const sessions = useSessions()
  const activeSessionId = useActiveSessionId()
  const createSession = useSessionStore((state) => state.createSession)
  const deleteSession = useSessionStore((state) => state.deleteSession)
  const setActiveSession = useSessionStore((state) => state.setActiveSession)

  // Sort sessions by updated time (most recent first)
  const sortedSessions = [...sessions].sort((a, b) => b.time.updated - a.time.updated)

  // Group sessions by time period
  const today = new Date()
  today.setHours(0, 0, 0, 0)

  const thisWeek = new Date(today)
  thisWeek.setDate(thisWeek.getDate() - 7)

  const thisMonth = new Date(today)
  thisMonth.setDate(thisMonth.getDate() - 30)

  const groups = {
    today: sortedSessions.filter((s) => s.time.updated >= today.getTime()),
    week: sortedSessions.filter(
      (s) => s.time.updated >= thisWeek.getTime() && s.time.updated < today.getTime()
    ),
    month: sortedSessions.filter(
      (s) => s.time.updated >= thisMonth.getTime() && s.time.updated < thisWeek.getTime()
    ),
    older: sortedSessions.filter((s) => s.time.updated < thisMonth.getTime()),
  }

  const handleNewSession = async () => {
    try {
      const session = await createSession()
      setActiveSession(session.id)
      onNewSession?.()
      onSessionSelect?.(session.id)
    } catch {
      // Error handling is done in the store
    }
  }

  const handleSessionClick = (sessionId: string) => {
    setActiveSession(sessionId)
    onSessionSelect?.(sessionId)
  }

  const handleDeleteSession = async (sessionId: string, e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      await deleteSession(sessionId)
    } catch {
      // Error handling is done in the store
    }
  }

  return (
    <aside
      className={cn(
        "flex flex-col border-r bg-background",
        "w-[260px] shrink-0",
        className
      )}
    >
      {/* Header with new session button */}
      <div className="flex items-center justify-between p-4">
        <h2 className="text-sm font-semibold">Sessions</h2>
        <Button
          variant="ghost"
          size="icon"
          onClick={handleNewSession}
          className="h-8 w-8"
          aria-label="New session"
        >
          <Plus className="h-4 w-4" />
        </Button>
      </div>

      <Separator />

      {/* Session list */}
      <ScrollArea className="flex-1 px-2 py-4">
        <div className="space-y-4">
          {/* Today */}
          {groups.today.length > 0 && (
            <CollapsibleSection title="Today" defaultOpen>
              {groups.today.map((session) => (
                <SessionItem
                  key={session.id}
                  title={session.title}
                  isActive={session.id === activeSessionId}
                  isDeleting={useSessionDeleting(session.id)}
                  onClick={() => handleSessionClick(session.id)}
                  onDelete={(e: React.MouseEvent) => {
                    e.stopPropagation()
                    void handleDeleteSession(session.id, e)
                  }}
                />
              ))}
            </CollapsibleSection>
          )}

          {/* This Week */}
          {groups.week.length > 0 && (
            <CollapsibleSection title="This Week" defaultOpen={false}>
              {groups.week.map((session) => (
                <SessionItem
                  key={session.id}
                  title={session.title}
                  isActive={session.id === activeSessionId}
                  isDeleting={useSessionDeleting(session.id)}
                  onClick={() => handleSessionClick(session.id)}
                  onDelete={(e: React.MouseEvent) => {
                    e.stopPropagation()
                    void handleDeleteSession(session.id, e)
                  }}
                />
              ))}
            </CollapsibleSection>
          )}

          {/* This Month */}
          {groups.month.length > 0 && (
            <CollapsibleSection title="This Month" defaultOpen={false}>
              {groups.month.map((session) => (
                <SessionItem
                  key={session.id}
                  title={session.title}
                  isActive={session.id === activeSessionId}
                  isDeleting={useSessionDeleting(session.id)}
                  onClick={() => handleSessionClick(session.id)}
                  onDelete={(e: React.MouseEvent) => {
                    e.stopPropagation()
                    void handleDeleteSession(session.id, e)
                  }}
                />
              ))}
            </CollapsibleSection>
          )}

          {/* Older */}
          {groups.older.length > 0 && (
            <CollapsibleSection title="Older" defaultOpen={false}>
              {groups.older.map((session) => (
                <SessionItem
                  key={session.id}
                  title={session.title}
                  isActive={session.id === activeSessionId}
                  isDeleting={useSessionDeleting(session.id)}
                  onClick={() => handleSessionClick(session.id)}
                  onDelete={(e: React.MouseEvent) => {
                    e.stopPropagation()
                    void handleDeleteSession(session.id, e)
                  }}
                />
              ))}
            </CollapsibleSection>
          )}

          {/* Empty state */}
          {sortedSessions.length === 0 && (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <FolderOpen className="mb-2 h-10 w-10 text-muted-foreground" />
              <p className="text-sm text-muted-foreground">No sessions yet</p>
              <p className="text-xs text-muted-foreground">Create a new session to get started</p>
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Footer with stats */}
      {sortedSessions.length > 0 && (
        <>
          <Separator />
          <div className="px-4 py-2 text-xs text-muted-foreground">
            {sortedSessions.length} session{sortedSessions.length !== 1 ? "s" : ""}
          </div>
        </>
      )}
    </aside>
  )
}
