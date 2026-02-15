/**
 * MessageItem Component
 *
 * Displays a single message with:
 * - Different styling for user and assistant messages
 * - Message parts display (text, reasoning, tool calls, files)
 * - Timestamp display
 * - Error display
 */

import * as React from "react"
import { User, Bot, AlertCircle, Clock, ChevronDown, ChevronUp } from "lucide-react"

import { cn } from "@/lib/utils"
import { formatTimestamp } from "@/lib/utils"
import { type MessageWithParts, type MessagePart } from "@/lib/types"
import { Card, CardContent } from "../ui/Card"
import { ToolCall } from "./ToolCall"

// ============================================================================
// Interfaces
// ============================================================================

export interface MessageItemProps {
  sessionId: string
  message: MessageWithParts
  className?: string
}

// ============================================================================
// Reasoning Display
// ============================================================================

interface ReasoningDisplayProps {
  text: string
  className?: string
}

function ReasoningDisplay({ text, className }: ReasoningDisplayProps) {
  const [isExpanded, setIsExpanded] = React.useState(false)

  return (
    <div
      className={cn(
        "rounded-lg bg-muted/50 border border-border/50",
        className
      )}
    >
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 w-full px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted/80 transition-colors rounded-t-lg"
      >
        {isExpanded ? (
          <ChevronUp className="h-4 w-4" />
        ) : (
          <ChevronDown className="h-4 w-4" />
        )}
        <span>Reasoning</span>
        <span className="text-xs opacity-60">
          ({text.length} chars)
        </span>
      </button>
      {isExpanded && (
        <div className="px-3 py-2 text-sm text-muted-foreground whitespace-pre-wrap border-t border-border/50">
          {text}
        </div>
      )}
    </div>
  )
}

// ============================================================================
// File Attachment Display
// ============================================================================

interface FileAttachmentProps {
  filename?: string
  url: string
  mime?: string
  className?: string
}

function FileAttachment({ filename, url, mime, className }: FileAttachmentProps) {
  const isImage = mime?.startsWith("image/")
  const displayName = filename ?? url.split("/").pop() ?? "file"

  if (isImage) {
    return (
      <div className={cn("rounded-lg overflow-hidden border", className)}>
        <img
          src={url}
          alt={displayName}
          className="max-w-full h-auto max-h-64 object-contain"
        />
      </div>
    )
  }

  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      className={cn(
        "inline-flex items-center gap-2 px-3 py-2 rounded-lg bg-muted hover:bg-muted/80 transition-colors text-sm",
        className
      )}
    >
      <span className="font-medium">{displayName}</span>
      {mime && (
        <span className="text-xs text-muted-foreground">({mime})</span>
      )}
    </a>
  )
}

// ============================================================================
// Error Display
// ============================================================================

interface ErrorDisplayProps {
  error: { name: string; message?: string; [key: string]: any }
  className?: string
}

function ErrorDisplay({ error, className }: ErrorDisplayProps) {
  return (
    <div
      className={cn(
        "flex items-start gap-2 p-3 rounded-lg bg-destructive/10 border border-destructive/20 text-destructive",
        className
      )}
    >
      <AlertCircle className="h-5 w-5 shrink-0 mt-0.5" />
      <div className="flex-1 min-w-0">
        <p className="font-medium text-sm">{error.name}</p>
        {error.message && (
          <p className="text-sm mt-1 opacity-90">{error.message}</p>
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Agent Badge
// ============================================================================

interface AgentBadgeProps {
  agent?: string
  model?: { providerID: string; modelID: string }
  className?: string
}

function AgentBadge({ agent, model, className }: AgentBadgeProps) {
  if (!agent) return null

  return (
    <div
      className={cn(
        "inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-primary/10 text-primary text-xs font-medium",
        className
      )}
    >
      <Bot className="h-3 w-3" />
      <span>{agent}</span>
      {model && (
        <span className="opacity-60">
          ({model.providerID}/{model.modelID})
        </span>
      )}
    </div>
  )
}

// ============================================================================
// Message Part Renderer
// ============================================================================

interface MessagePartProps {
  part: MessagePart
  sessionId: string
}

function MessagePartRenderer({ part, sessionId }: MessagePartProps) {
  switch (part.type) {
    case "text":
      if (part.ignored || part.synthetic) return null
      return (
        <p className="text-sm whitespace-pre-wrap break-words">
          {part.text}
        </p>
      )

    case "reasoning":
      return <ReasoningDisplay text={part.text} />

    case "file":
      return <FileAttachment filename={part.filename} url={part.url} mime={part.mime} />

    case "tool":
      return <ToolCall part={part} sessionId={sessionId} />

    case "agent":
      return (
        <div className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-muted text-xs">
          <Bot className="h-3 w-3" />
          <span>{part.name}</span>
        </div>
      )

    case "subtask":
      return (
        <Card className="bg-muted/50 border-border/50">
          <CardContent className="p-3">
            <div className="flex items-center gap-2 mb-2">
              <Bot className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm font-medium">Subtask: {part.agent}</span>
            </div>
            <p className="text-sm text-muted-foreground">{part.description}</p>
          </CardContent>
        </Card>
      )

    case "retry":
      return (
        <div className="flex items-center gap-2 p-2 rounded bg-yellow-500/10 border border-yellow-500/20 text-yellow-600 dark:text-yellow-500">
          <AlertCircle className="h-4 w-4" />
          <span className="text-sm">Retry attempt {part.attempt}</span>
        </div>
      )

    case "decision":
      return (
        <Card className="bg-blue-500/10 border-blue-500/20">
          <CardContent className="p-3">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-medium">Decision: {part.tool}</span>
              <span
                className={cn(
                  "text-xs px-2 py-0.5 rounded-full",
                  part.action === "proceed"
                    ? "bg-green-500/20 text-green-600 dark:text-green-400"
                    : part.action === "block"
                      ? "bg-red-500/20 text-red-600 dark:text-red-400"
                      : "bg-yellow-500/20 text-yellow-600 dark:text-yellow-400"
                )}
              >
                {part.action}
              </span>
            </div>
            <p className="text-sm text-muted-foreground">{part.reasoning}</p>
          </CardContent>
        </Card>
      )

    case "step-start":
      return (
        <div className="text-xs text-muted-foreground italic">
          Starting step...
        </div>
      )

    case "step-finish":
      return (
        <div className="text-xs text-muted-foreground italic">
          Step complete. Cost: ${part.cost.toFixed(4)}
        </div>
      )

    case "snapshot":
      return (
        <div className="text-xs text-muted-foreground italic">
          Snapshot: {part.snapshot}
        </div>
      )

    case "patch":
      return (
        <div className="text-xs text-muted-foreground">
          Patch: {part.hash} ({part.files.length} files)
        </div>
      )

    case "compaction":
      return (
        <div className="text-xs text-muted-foreground italic">
          {part.auto ? "Auto" : "Manual"} compaction
        </div>
      )

    default:
      return null
  }
}

// ============================================================================
// Main Component
// ============================================================================

export function MessageItem({ sessionId, message, className }: MessageItemProps) {
  const { info, parts } = message
  const isUser = info.role === "user"
  const timestamp = formatTimestamp(info.time.created)

  return (
    <div
      className={cn(
        "flex gap-3 p-4 animate-in fade-in slide-in-from-bottom-2 duration-300",
        isUser ? "flex-row-reverse" : "flex-row",
        className
      )}
    >
      {/* Avatar */}
      <div
        className={cn(
          "flex h-8 w-8 shrink-0 items-center justify-center rounded-full",
          isUser
            ? "bg-primary text-primary-foreground"
            : "bg-muted text-muted-foreground"
        )}
      >
        {isUser ? (
          <User className="h-4 w-4" />
        ) : (
          <Bot className="h-4 w-4" />
        )}
      </div>

      {/* Message Content */}
      <div
        className={cn(
          "flex flex-col gap-2 max-w-[85%]",
          isUser ? "items-end" : "items-start"
        )}
      >
        {/* Header: Agent/Model badge + Timestamp */}
        <div
          className={cn(
            "flex items-center gap-2 text-xs text-muted-foreground",
            isUser ? "flex-row-reverse" : "flex-row"
          )}
        >
          {!isUser && <AgentBadge agent={info.agent} model={info.model} />}
          <div className="flex items-center gap-1">
            <Clock className="h-3 w-3" />
            <span>{timestamp}</span>
          </div>
          {info.tokens && (
            <span className="opacity-60">
              {info.tokens.input + info.tokens.output + (info.tokens.reasoning || 0)} tokens
            </span>
          )}
        </div>

        {/* Message Parts */}
        <div
          className={cn(
            "rounded-2xl px-4 py-3",
            isUser
              ? "bg-primary text-primary-foreground"
              : "bg-muted/50 text-foreground"
          )}
        >
          {parts.map((part, index) => (
            <MessagePartRenderer
              key={part.id ?? index}
              part={part}
              sessionId={sessionId}
            />
          ))}
        </div>

        {/* Error Display */}
        {info.error && <ErrorDisplay error={info.error} />}

        {/* Summary (for user messages) */}
        {info.summary && info.summary.diffs.length > 0 && (
          <div className="text-xs text-muted-foreground">
            {info.summary.diffs.length} file{info.summary.diffs.length !== 1 ? "s" : ""} changed
          </div>
        )}
      </div>
    </div>
  )
}
