/**
 * Sessions Page
 *
 * Full-page session management with:
 * - Complete session list with search/filter
 * - Create/delete session functionality
 * - Navigation to individual sessions
 * - Rename session dialog
 */

import * as React from "react"
import { useNavigate } from "@tanstack/react-router"
import { MessageSquare, Plus } from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent } from "@/components/ui/Card"
import { Input } from "@/components/ui/Input"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogFooter,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/Dialog"
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
  const { createSession, deleteSession, renameSession, loadSessions } = useSessionStore()
  const { toast } = useToast()

  // Rename dialog state
  const [renameDialogOpen, setRenameDialogOpen] = React.useState(false)
  const [renameSessionId, setRenameSessionId] = React.useState<string | null>(null)
  const [renameTitle, setRenameTitle] = React.useState("")
  const [isRenaming, setIsRenaming] = React.useState(false)

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
    const session = sessions.find((s) => s.id === sessionId)
    if (session) {
      setRenameSessionId(sessionId)
      setRenameTitle(session.title)
      setRenameDialogOpen(true)
    }
  }

  const handleRenameSubmit = async () => {
    if (!renameSessionId || !renameTitle.trim()) return

    setIsRenaming(true)
    try {
      await renameSession(renameSessionId, renameTitle.trim())
      setRenameDialogOpen(false)
      setRenameSessionId(null)
      setRenameTitle("")
      toast({
        title: "Session renamed",
        description: "The session has been renamed.",
      })
    } catch {
      toast({
        title: "Failed to rename session",
        description: "An error occurred. Please try again.",
        variant: "destructive",
      })
    } finally {
      setIsRenaming(false)
    }
  }

  const handleRenameCancel = () => {
    setRenameDialogOpen(false)
    setRenameSessionId(null)
    setRenameTitle("")
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

      {/* Rename Dialog */}
      <Dialog open={renameDialogOpen} onOpenChange={setRenameDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename Session</DialogTitle>
            <DialogDescription>Enter a new name for this session.</DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Input
              value={renameTitle}
              onChange={(e) => setRenameTitle(e.target.value)}
              placeholder="Session name"
              onKeyDown={(e) => {
                if (e.key === "Enter" && !isRenaming) {
                  handleRenameSubmit()
                }
              }}
              autoFocus
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={handleRenameCancel} disabled={isRenaming}>
              Cancel
            </Button>
            <Button onClick={handleRenameSubmit} disabled={isRenaming || !renameTitle.trim()}>
              {isRenaming ? "Renaming..." : "Rename"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

export default Sessions
