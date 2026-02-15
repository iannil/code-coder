/**
 * ToolCall Component
 *
 * Displays a tool call with:
 * - Collapsible display
 * - Tool name and parameters
 * - Tool result
 * - Status indicator (pending, running, completed, error)
 */

import * as React from "react"
import {
  ChevronDown,
  ChevronUp,
  Loader2,
  CheckCircle2,
  XCircle,
  Clock,
  Terminal,
} from "lucide-react"

import { cn } from "@/lib/utils"
import { type ToolPart } from "@/lib/types"
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card"
import { Separator } from "../ui/Separator"

// ============================================================================
// Interfaces
// ============================================================================

export interface ToolCallProps {
  part: ToolPart
  sessionId: string
  className?: string
}

// ============================================================================
// Status Indicator
// ============================================================================

interface StatusIndicatorProps {
  status: ToolPart["state"]["status"]
  className?: string
}

function StatusIndicator({ status, className }: StatusIndicatorProps) {
  const variants = {
    pending: {
      icon: Clock,
      color: "text-yellow-600 dark:text-yellow-500",
      bgColor: "bg-yellow-500/10",
      borderColor: "border-yellow-500/20",
      label: "Pending",
    },
    running: {
      icon: Loader2,
      color: "text-blue-600 dark:text-blue-500",
      bgColor: "bg-blue-500/10",
      borderColor: "border-blue-500/20",
      label: "Running",
    },
    completed: {
      icon: CheckCircle2,
      color: "text-green-600 dark:text-green-500",
      bgColor: "bg-green-500/10",
      borderColor: "border-green-500/20",
      label: "Completed",
    },
    error: {
      icon: XCircle,
      color: "text-red-600 dark:text-red-500",
      bgColor: "bg-red-500/10",
      borderColor: "border-red-500/20",
      label: "Error",
    },
  }

  const variant = variants[status]
  const Icon = variant.icon

  return (
    <div
      className={cn(
        "inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium",
        variant.bgColor,
        variant.borderColor,
        "border",
        variant.color,
        className
      )}
    >
      <Icon
        className={cn(
          "h-3 w-3",
          status === "running" && "animate-spin"
        )}
      />
      <span>{variant.label}</span>
    </div>
  )
}

// ============================================================================
// Parameter Display
// ============================================================================

interface ParameterDisplayProps {
  input: Record<string, any>
  className?: string
}

function ParameterDisplay({ input, className }: ParameterDisplayProps) {
  const [isExpanded, setIsExpanded] = React.useState(false)

  const entries = React.useMemo(() => Object.entries(input), [input])
  const hasManyParams = entries.length > 3

  if (entries.length === 0) {
    return (
      <span className={cn("text-sm text-muted-foreground italic", className)}>
        No parameters
      </span>
    )
  }

  const displayEntries = isExpanded ? entries : entries.slice(0, 3)

  return (
    <div className={cn("space-y-1", className)}>
      {displayEntries.map(([key, value]) => (
        <div key={key} className="flex gap-2 text-sm">
          <span className="font-medium text-muted-foreground shrink-0">
            {key}:
          </span>
          <span className="break-all">
            {typeof value === "object"
              ? JSON.stringify(value, null, 2)
              : String(value)}
          </span>
        </div>
      ))}
      {hasManyParams && !isExpanded && (
        <button
          onClick={() => setIsExpanded(true)}
          className="text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          +{entries.length - 3} more parameters
        </button>
      )}
    </div>
  )
}

// ============================================================================
// Result Display
// ============================================================================

interface ResultDisplayProps {
  output: string
  className?: string
}

function ResultDisplay({ output, className }: ResultDisplayProps) {
  const [isExpanded, setIsExpanded] = React.useState(false)
  const lines = output.split("\n")
  const isLong = lines.length > 10 || output.length > 1000

  const displayOutput = React.useMemo(() => {
    if (!isLong) return output
    return isExpanded
      ? output
      : lines.slice(0, 10).join("\n") + "\n..."
  }, [output, isExpanded, isLong])

  return (
    <div
      className={cn(
        "rounded-md bg-muted p-3 font-mono text-xs",
        "overflow-x-auto whitespace-pre-wrap break-all",
        className
      )}
    >
      {displayOutput}
      {isLong && (
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="mt-2 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          {isExpanded ? "Show less" : "Show more"}
        </button>
      )}
    </div>
  )
}

// ============================================================================
// Error Display
// ============================================================================

interface ToolErrorDisplayProps {
  error: string
  className?: string
}

function ToolErrorDisplay({ error, className }: ToolErrorDisplayProps) {
  const [isExpanded, setIsExpanded] = React.useState(false)

  return (
    <div
      className={cn(
        "rounded-md bg-destructive/10 border border-destructive/20 p-3",
        className
      )}
    >
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 text-sm font-medium text-destructive w-full"
      >
        <XCircle className="h-4 w-4" />
        <span>Error</span>
        {isExpanded ? (
          <ChevronUp className="h-4 w-4 ml-auto" />
        ) : (
          <ChevronDown className="h-4 w-4 ml-auto" />
        )}
      </button>
      {isExpanded && (
        <p className="mt-2 text-sm text-destructive/90 whitespace-pre-wrap">
          {error}
        </p>
      )}
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function ToolCall({ part, className }: ToolCallProps) {
  const [isExpanded, setIsExpanded] = React.useState(
    part.state.status === "error"
  )
  const state = part.state

  // Format duration for running/completed states
  const formatDuration = React.useMemo(() => {
    if (state.status === "pending") return null
    const start = state.time.start
    const end = state.status === "running" ? Date.now() : state.time.end
    const duration = Math.floor((end - start) / 1000)
    if (duration < 60) return `${duration}s`
    const mins = Math.floor(duration / 60)
    const secs = duration % 60
    return `${mins}m ${secs}s`
  }, [state])

  return (
    <Card
      className={cn(
        "border-border/50 bg-muted/30",
        state.status === "error" && "border-destructive/30 bg-destructive/5",
        className
      )}
    >
      <CardHeader className="py-3 px-4">
        <div className="flex items-center gap-2">
          <Terminal className="h-4 w-4 text-muted-foreground" />
          <CardTitle className="text-sm font-medium">
            {part.tool}
          </CardTitle>
          <StatusIndicator status={state.status} />
          {formatDuration && (
            <span className="text-xs text-muted-foreground">
              {formatDuration}
            </span>
          )}
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="ml-auto p-1 hover:bg-muted rounded transition-colors"
          >
            {isExpanded ? (
              <ChevronUp className="h-4 w-4 text-muted-foreground" />
            ) : (
              <ChevronDown className="h-4 w-4 text-muted-foreground" />
            )}
          </button>
        </div>
      </CardHeader>

      {isExpanded && (
        <>
          <Separator />
          <CardContent className="p-4 space-y-4">
            {/* Parameters */}
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-2">
                Parameters
              </h4>
              <ParameterDisplay input={state.input} />
            </div>

            {/* Result (for completed/error states) */}
            {state.status === "completed" && state.output && (
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-2">
                  Result
                </h4>
                <ResultDisplay output={state.output} />
              </div>
            )}

            {/* Error */}
            {state.status === "error" && state.error && (
              <ToolErrorDisplay error={state.error} />
            )}

            {/* Title (for completed state) */}
            {state.status === "completed" && state.title && (
              <div className="text-sm text-muted-foreground">
                <span className="font-medium">Title: </span>
                {state.title}
              </div>
            )}

            {/* Metadata (if present) */}
            {"metadata" in state && state.metadata && Object.keys(state.metadata).length > 0 && (
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-2">
                  Metadata
                </h4>
                <div className="text-xs space-y-1">
                  {Object.entries(state.metadata).map(([key, value]) => (
                    <div key={key} className="flex gap-2">
                      <span className="font-medium text-muted-foreground">
                        {key}:
                      </span>
                      <span className="break-all">
                        {typeof value === "object"
                          ? JSON.stringify(value)
                          : String(value)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Attachments (if present) */}
            {state.status === "completed" &&
              state.attachments &&
              state.attachments.length > 0 && (
                <div>
                  <h4 className="text-xs font-medium text-muted-foreground mb-2">
                    Attachments
                  </h4>
                  <div className="flex flex-wrap gap-2">
                    {state.attachments.map((file, index) => (
                      <a
                        key={index}
                        href={file.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-xs px-2 py-1 rounded bg-muted hover:bg-muted/80 transition-colors"
                      >
                        {file.filename ?? file.url.split("/").pop()}
                      </a>
                    ))}
                  </div>
                </div>
              )}
          </CardContent>
        </>
      )}
    </Card>
  )
}
