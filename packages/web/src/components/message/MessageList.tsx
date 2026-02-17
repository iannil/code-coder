/**
 * MessageList Component
 *
 * Displays a list of messages for a session with:
 * - Virtual scrolling support for long conversations
 * - Auto-scroll to bottom on new messages
 * - Loading skeleton
 * - Empty state
 */

import { useRef, useEffect } from "react"
import { ChevronRight, MessageSquare } from "lucide-react"

import { cn } from "@/lib/utils"
import { useMessages, useMessagesLoading } from "@/stores/message"
import { MessageItem } from "./MessageItem"
import { ScrollArea } from "../ui/ScrollArea"
import { Button } from "../ui/Button"
import { Skeleton } from "../ui/Skeleton"

// ============================================================================
// Interfaces
// ============================================================================

export interface MessageListProps {
  sessionId: string
  className?: string
}

// ============================================================================
// Loading Skeleton
// ============================================================================

interface MessageSkeletonProps {
  className?: string
}

function MessageSkeleton({ className }: MessageSkeletonProps) {
  return (
    <div className={cn("flex gap-3 p-4", className)}>
      <Skeleton className="h-8 w-8 shrink-0 rounded-full" />
      <div className="flex-1 space-y-2">
        <Skeleton className="h-4 w-24" />
        <div className="space-y-1">
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-5/6" />
          <Skeleton className="h-4 w-4/6" />
        </div>
      </div>
    </div>
  )
}

function MessageListSkeleton() {
  return (
    <div className="space-y-4 p-4">
      <MessageSkeleton />
      <MessageSkeleton />
      <MessageSkeleton />
    </div>
  )
}

// ============================================================================
// Empty State
// ============================================================================

interface EmptyStateProps {
  onStartConversation?: () => void
}

function EmptyState({ onStartConversation }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center h-full p-8 text-center" data-testid="message-list">
      <div className="flex h-16 w-16 items-center justify-center rounded-full bg-muted mb-4">
        <MessageSquare className="h-8 w-8 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-semibold mb-2">No messages yet</h3>
      <p className="text-sm text-muted-foreground mb-4 max-w-md">
        Start a conversation by sending a message. Ask questions, request code
        reviews, or explore your codebase.
      </p>
      {onStartConversation && (
        <Button onClick={onStartConversation} variant="default">
          <ChevronRight className="h-4 w-4" />
          Start conversation
        </Button>
      )}
    </div>
  )
}

// ============================================================================
// Simple Scrolling Variant (default)
// ============================================================================

export function MessageListSimple({
  sessionId,
  className,
}: MessageListProps) {
  const messages = useMessages(sessionId)
  const isLoading = useMessagesLoading(sessionId)

  const scrollRef = useRef<HTMLDivElement>(null)

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (scrollRef.current && messages.length > 0) {
      scrollRef.current.scrollIntoView({ behavior: "smooth" })
    }
  }, [messages.length])

  // Show loading skeleton
  if (isLoading && messages.length === 0) {
    return <MessageListSkeleton />
  }

  // Show empty state
  if (messages.length === 0) {
    return <EmptyState />
  }

  return (
    <ScrollArea className={cn("flex-1 h-full", className)} data-testid="message-list">
      <div className="space-y-4 p-4">
        {messages.map((message) => (
          <MessageItem
            key={message.info.id}
            sessionId={sessionId}
            message={message}
          />
        ))}
        <div ref={scrollRef} />
      </div>
    </ScrollArea>
  )
}

// ============================================================================
// Main Component (auto-select variant)
// ============================================================================

export function MessageList({
  sessionId,
  className,
}: MessageListProps) {
  return <MessageListSimple sessionId={sessionId} className={className} />
}
