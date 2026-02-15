/**
 * Session Page
 *
 * Displays a single session with:
 * - Message list
 * - Message input
 * - Agent selector
 * - Session info display
 * - SSE integration for real-time updates
 */

import * as React from "react"
import { useNavigate, useParams } from "@tanstack/react-router"
import {
  ArrowLeft,
  Bot,
  MoreVertical,
  Trash2,
  Copy,
  Share,
  Maximize2,
  Minimize2,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/DropdownMenu"
import { MessageList } from "@/components/message/MessageList"
import { MessageInput } from "@/components/message/MessageInput"
import { AgentSelector } from "@/components/agent/AgentSelector"
import { Separator } from "@/components/ui/Separator"
import { Card, CardContent } from "@/components/ui/Card"
import { useSession, useSessionStore } from "@/stores/session"
import { useMessages, useMessagesLoading, useMessageStore } from "@/stores/message"
import { useSelectedAgent, useAgents, useAgentStore } from "@/stores/agent"
import { useSSEConnected, useSSEStore } from "@/stores/sse"
import { useToast } from "@/hooks/use-toast"
import { api } from "@/lib/api"
import { cn, formatTimestamp } from "@/lib/utils"
import { SSEClient } from "@/lib/sse"

// ============================================================================
// Session Info Component
// ============================================================================

interface SessionInfoProps {
  sessionId: string
  session: ReturnType<typeof useSession> | null
  isFullscreen: boolean
  onToggleFullscreen: () => void
}

function SessionInfo({ sessionId, session, isFullscreen, onToggleFullscreen }: SessionInfoProps) {
  const navigate = useNavigate()
  const deleteSession = useSessionStore((state) => state.deleteSession)
  const { toast } = useToast()

  if (!session) return null

  const handleDelete = async () => {
    try {
      await deleteSession(sessionId)
      toast({
        title: "Session deleted",
        description: "The session has been deleted successfully.",
      })
      navigate({ to: "/" })
    } catch {
      toast({
        title: "Failed to delete session",
        description: "An error occurred while deleting the session.",
        variant: "destructive",
      })
    }
  }

  const handleCopy = () => {
    navigator.clipboard.writeText(sessionId)
    toast({
      title: "Session ID copied",
      description: "The session ID has been copied to your clipboard.",
    })
  }

  return (
    <div className="flex items-center gap-2 px-4 py-2 border-b bg-muted/30">
      <div className="flex-1 min-w-0">
        <h2 className="font-medium truncate">{session.title || "Untitled Session"}</h2>
        <p className="text-xs text-muted-foreground">
          Created {formatTimestamp(session.time.created)}
        </p>
      </div>

      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={onToggleFullscreen}
          title={isFullscreen ? "Exit fullscreen" : "Fullscreen"}
        >
          {isFullscreen ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
        </Button>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" className="h-8 w-8">
              <MoreVertical className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={handleCopy}>
              <Copy className="mr-2 h-4 w-4" />
              Copy session ID
            </DropdownMenuItem>
            <DropdownMenuItem>
              <Share className="mr-2 h-4 w-4" />
              Share session
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem className="text-destructive" onClick={handleDelete}>
              <Trash2 className="mr-2 h-4 w-4" />
              Delete session
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  )
}

// ============================================================================
// Session Stats Component
// ============================================================================

interface SessionStatsProps {
  messageCount: number
  isLoading: boolean
}

function SessionStats({ messageCount, isLoading }: SessionStatsProps) {
  return (
    <div className="flex items-center gap-4 px-4 py-2 text-xs text-muted-foreground border-t">
      <span>{messageCount} message{messageCount !== 1 ? "s" : ""}</span>
      {isLoading && (
        <span className="flex items-center gap-1">
          <div className="h-3 w-3 animate-spin rounded-full border border-current border-t-transparent" />
          Loading messages...
        </span>
      )}
    </div>
  )
}

// ============================================================================
// Empty State Component
// ============================================================================

interface SessionEmptyStateProps {
  onCreateMessage: () => void
}

function SessionEmptyState({ onCreateMessage }: SessionEmptyStateProps) {
  return (
    <div className="flex-1 flex items-center justify-center p-8">
      <Card className="max-w-md text-center">
        <CardContent className="pt-6 space-y-4">
          <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
            <Bot className="h-8 w-8 text-primary" />
          </div>
          <div className="space-y-2">
            <h3 className="text-lg font-semibold">Start a conversation</h3>
            <p className="text-sm text-muted-foreground">
              Ask questions, request code reviews, or explore your codebase with AI assistance.
            </p>
          </div>
          <Button onClick={onCreateMessage}>
            Send your first message
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Main Session Component
// ============================================================================

export function Session() {
  const params = useParams({ from: "/sessions/$sessionId" })
  const navigate = useNavigate()
  const sessionId = params.sessionId

  const session = useSession(sessionId)
  const messages = useMessages(sessionId)
  const messagesLoading = useMessagesLoading(sessionId)
  const loadMessages = useMessageStore((state) => state.loadMessages)
  const { toast } = useToast()

  const agents = useAgents()
  const selectedAgent = useSelectedAgent()
  const selectAgent = useAgentStore((state) => state.selectAgent)

  const [isFullscreen, setIsFullscreen] = React.useState(false)
  const [isSending, setIsSending] = React.useState(false)
  const messageInputRef = React.useRef<HTMLTextAreaElement>(null)

  // SSE connection
  const sseConnected = useSSEConnected()
  const connectSSE = useSSEStore((state) => state.connect)

  // Load session messages on mount
  React.useEffect(() => {
    loadMessages(sessionId)
  }, [sessionId, loadMessages])

  // Set up SSE connection for real-time updates
  // Use ref to track if SSE has been initialized for this session
  const sseInitializedRef = React.useRef<Set<string>>(new Set())

  React.useEffect(() => {
    // Skip if already initialized for this session
    if (sseInitializedRef.current.has(sessionId)) {
      return
    }
    sseInitializedRef.current.add(sessionId)

    const client = new SSEClient(
      {
        baseUrl: "/api",
        channels: ["message", "status", "error", "permission", "progress"],
      },
      {
        onMessage: (event) => {
          if (event.type === "message" && event.sessionID === sessionId) {
            // Update message in store when received via SSE
            // This would be handled by a more sophisticated message update logic
          }
        },
      }
    )
    connectSSE(() => client)

    return () => {
      client.disconnect()
      sseInitializedRef.current.delete(sessionId)
    }
  }, [sessionId, connectSSE])

  // Handle sending a message
  const handleSendMessage = async (content: string, _files?: File[]) => {
    if (!content.trim()) return

    setIsSending(true)
    try {
      await api.sendMessage(sessionId, {
        agent: selectedAgent?.id,
        parts: [{ type: "text", text: content }],
      })

      toast({
        title: "Message sent",
        description: "Your message has been sent successfully.",
      })
    } catch (error) {
      toast({
        title: "Failed to send message",
        description: error instanceof Error ? error.message : "An error occurred",
        variant: "destructive",
      })
    } finally {
      setIsSending(false)
    }
  }

  // Handle back navigation
  const handleBack = () => {
    navigate({ to: "/" })
  }

  // Handle session not found
  if (!session) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Card className="max-w-md text-center">
          <CardContent className="pt-6 space-y-4">
            <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-destructive/10">
              <Bot className="h-8 w-8 text-destructive" />
            </div>
            <div className="space-y-2">
              <h3 className="text-lg font-semibold">Session not found</h3>
              <p className="text-sm text-muted-foreground">
                The session you're looking for doesn't exist or has been deleted.
              </p>
            </div>
            <Button onClick={handleBack}>
              <ArrowLeft className="mr-2 h-4 w-4" />
              Go back to dashboard
            </Button>
          </CardContent>
        </Card>
      </div>
    )
  }

  const messageCount = messages.length
  const hasMessages = messageCount > 0

  return (
    <div
      className={cn(
        "flex flex-col h-full bg-background",
        isFullscreen && "fixed inset-0 z-50"
      )}
    >
      {/* Session Header */}
      <SessionInfo
        sessionId={sessionId}
        session={session}
        isFullscreen={isFullscreen}
        onToggleFullscreen={() => setIsFullscreen(!isFullscreen)}
      />

      {/* Agent Selector Bar */}
      {!isFullscreen && (
        <>
          <div className="flex items-center gap-2 px-4 py-2 border-b bg-muted/20">
            <span className="text-sm text-muted-foreground">Agent:</span>
            <div className="flex-1 max-w-xs">
              <AgentSelector
                agents={agents}
                selectedId={selectedAgent?.id ?? null}
                onSelect={selectAgent}
              />
            </div>
            {sseConnected && (
              <div className="flex items-center gap-1.5 text-xs text-green-500">
                <div className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
                Live
              </div>
            )}
          </div>
          <Separator />
        </>
      )}

      {/* Messages Area */}
      {hasMessages ? (
        <div className="flex-1 flex flex-col overflow-hidden">
          <MessageList sessionId={sessionId} className="flex-1" />
          <SessionStats messageCount={messageCount} isLoading={messagesLoading} />
        </div>
      ) : (
        <SessionEmptyState onCreateMessage={() => messageInputRef.current?.focus()} />
      )}

      {/* Message Input */}
      <MessageInput
        ref={messageInputRef}
        onSend={handleSendMessage}
        disabled={isSending}
        loading={isSending}
        placeholder="Type your message... (Press Enter to send, Shift+Enter for new line)"
        className="border-t"
      />
    </div>
  )
}

export default Session
