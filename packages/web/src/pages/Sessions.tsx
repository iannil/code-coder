/**
 * Sessions Page
 *
 * Full-page session management with:
 * - Complete session list with search/filter
 * - Create/delete session functionality
 * - Navigation to individual sessions
 */

import * as React from "react"
import { useNavigate } from "@tanstack/react-router"
import { MessageSquare, Plus } from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent } from "@/components/ui/Card"
import { SessionList } from "@/components/session/SessionList"
import { useSessionStore, useSessions, useSessionsLoading } from "@/stores/session"
import { useToast } from "@/hooks/use-toast"

// ============================================================================
// Empty State Component
// ============================================================================

interface SessionsEmptyStateProps {
  onCreateSession: () => void
  isCreating: boolean
}

function SessionsEmptyState({ onCreateSession, isCreating }: SessionsEmptyStateProps) {
  return (
    <div className="flex-1 flex items-center justify-center p-8">
      <Card className="max-w-md text-center">
        <CardContent className="pt-6 space-y-4">
          <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
            <MessageSquare className="h-8 w-8 text-primary" />
          </div>
          <div className="space-y-2">
            <h3 className="text-lg font-semibold">No sessions yet</h3>
            <p className="text-sm text-muted-foreground">
              Start a new conversation with CodeCoder. Sessions preserve your chat history
              and context for ongoing projects.
            </p>
          </div>
          <Button onClick={onCreateSession} disabled={isCreating}>
            <Plus className="mr-2 h-4 w-4" />
            {isCreating ? "Creating..." : "New Session"}
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Sessions Header Component
// ============================================================================

interface SessionsHeaderProps {
  totalSessions: number
  onCreateSession: () => void
  isCreating: boolean
}

function SessionsHeader({ totalSessions, onCreateSession, isCreating }: SessionsHeaderProps) {
  return (
    <div className="flex items-center justify-between p-4 border-b bg-muted/30">
      <div className="flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <MessageSquare className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-lg font-semibold">Sessions</h1>
          <p className="text-sm text-muted-foreground">
            {totalSessions === 0
              ? "No sessions"
              : `${totalSessions} session${totalSessions !== 1 ? "s" : ""}`}
          </p>
        </div>
      </div>
      <Button onClick={onCreateSession} disabled={isCreating} data-testid="create-session-btn">
        <Plus className="mr-2 h-4 w-4" />
        {isCreating ? "Creating..." : "New Session"}
      </Button>
    </div>
  )
}

// ============================================================================
// Main Sessions Page Component
// ============================================================================

export function Sessions() {
  const navigate = useNavigate()
  const sessions = useSessions()
  const { isLoaded, isCreating } = useSessionsLoading()
  const { createSession, deleteSession, loadSessions } = useSessionStore()
  const { toast } = useToast()

  // Load sessions on mount
  React.useEffect(() => {
    loadSessions()
  }, [loadSessions])

  const handleCreateSession = async () => {
    try {
      const session = await createSession({ title: "New Session" })
      navigate({ to: "/sessions/$sessionId", params: { sessionId: session.id } })
      toast({
        title: "Session created",
        description: "Your new session is ready.",
      })
    } catch {
      toast({
        title: "Failed to create session",
        description: "An error occurred. Please try again.",
        variant: "destructive",
      })
    }
  }

  const handleSessionClick = (sessionId: string) => {
    navigate({ to: "/sessions/$sessionId", params: { sessionId } })
  }

  const handleSessionDelete = async (sessionId: string) => {
    try {
      await deleteSession(sessionId)
      toast({
        title: "Session deleted",
        description: "The session has been removed.",
      })
    } catch {
      toast({
        title: "Failed to delete session",
        description: "An error occurred. Please try again.",
        variant: "destructive",
      })
    }
  }

  const handleSessionRename = (sessionId: string) => {
    // TODO: Implement rename dialog
    console.log("Rename session:", sessionId)
  }

  // Show empty state if no sessions after loading
  if (isLoaded && sessions.length === 0) {
    return (
      <div className="flex flex-col h-full bg-background">
        <SessionsHeader
          totalSessions={0}
          onCreateSession={handleCreateSession}
          isCreating={isCreating}
        />
        <SessionsEmptyState onCreateSession={handleCreateSession} isCreating={isCreating} />
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full bg-background">
      <SessionsHeader
        totalSessions={sessions.length}
        onCreateSession={handleCreateSession}
        isCreating={isCreating}
      />
      <SessionList
        onSessionClick={handleSessionClick}
        onSessionDelete={handleSessionDelete}
        onSessionRename={handleSessionRename}
        className="flex-1"
      />
    </div>
  )
}

export default Sessions
