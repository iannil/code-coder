/**
 * MessageDetails Component
 *
 * Expandable panel showing detailed message information:
 * - Model info (provider, model ID)
 * - Token usage (input, output, cache)
 * - Cost breakdown
 * - Timing information
 */

import * as React from "react"
import {
  ChevronDown,
  ChevronUp,
  Clock,
  Coins,
  Cpu,
  Database,
  Sparkles,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type { MessageInfo, MessageTokens } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface MessageDetailsProps {
  info: MessageInfo
  className?: string
}

// ============================================================================
// Token Display Component
// ============================================================================

interface TokenDisplayProps {
  tokens: MessageTokens
  className?: string
}

function TokenDisplay({ tokens, className }: TokenDisplayProps) {
  const total = tokens.input + tokens.output + (tokens.reasoning || 0)
  const cacheTotal = (tokens.cache?.read || 0) + (tokens.cache?.write || 0)

  return (
    <div className={cn("grid grid-cols-2 gap-2 text-xs", className)}>
      <div className="flex items-center gap-2">
        <span className="text-muted-foreground">Input:</span>
        <span className="font-mono">{tokens.input.toLocaleString()}</span>
      </div>
      <div className="flex items-center gap-2">
        <span className="text-muted-foreground">Output:</span>
        <span className="font-mono">{tokens.output.toLocaleString()}</span>
      </div>
      {tokens.reasoning > 0 && (
        <div className="flex items-center gap-2">
          <span className="text-muted-foreground">Reasoning:</span>
          <span className="font-mono">{tokens.reasoning.toLocaleString()}</span>
        </div>
      )}
      {cacheTotal > 0 && (
        <div className="flex items-center gap-2">
          <span className="text-muted-foreground">Cache:</span>
          <span className="font-mono text-green-600">
            +{tokens.cache?.read?.toLocaleString() || 0} / -{tokens.cache?.write?.toLocaleString() || 0}
          </span>
        </div>
      )}
      <div className="flex items-center gap-2 col-span-2 pt-1 border-t border-border/50">
        <span className="text-muted-foreground">Total:</span>
        <span className="font-mono font-medium">{total.toLocaleString()}</span>
      </div>
    </div>
  )
}

// ============================================================================
// Timing Display Component
// ============================================================================

interface TimingDisplayProps {
  created: number
  completed?: number
  className?: string
}

function TimingDisplay({ created, completed, className }: TimingDisplayProps) {
  const formatDate = (timestamp: number) => {
    return new Date(timestamp).toLocaleString()
  }

  const duration = completed ? completed - created : null

  return (
    <div className={cn("space-y-1 text-xs", className)}>
      <div className="flex items-center gap-2">
        <span className="text-muted-foreground">Started:</span>
        <span className="font-mono">{formatDate(created)}</span>
      </div>
      {completed && (
        <div className="flex items-center gap-2">
          <span className="text-muted-foreground">Completed:</span>
          <span className="font-mono">{formatDate(completed)}</span>
        </div>
      )}
      {duration && (
        <div className="flex items-center gap-2 pt-1 border-t border-border/50">
          <span className="text-muted-foreground">Duration:</span>
          <span className="font-mono font-medium">
            {duration < 1000
              ? `${duration}ms`
              : duration < 60000
                ? `${(duration / 1000).toFixed(1)}s`
                : `${(duration / 60000).toFixed(1)}m`}
          </span>
        </div>
      )}
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function MessageDetails({ info, className }: MessageDetailsProps) {
  const [isExpanded, setIsExpanded] = React.useState(false)

  // Only show for assistant messages with detailed info
  const hasDetails = info.role === "assistant" && (info.tokens || info.cost || info.modelID)

  if (!hasDetails) return null

  const formatCost = (cost: number) => {
    return cost < 0.01 ? `<$0.01` : `$${cost.toFixed(4)}`
  }

  return (
    <div className={cn("mt-2", className)}>
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
      >
        {isExpanded ? (
          <ChevronUp className="h-3 w-3" />
        ) : (
          <ChevronDown className="h-3 w-3" />
        )}
        <span>Details</span>
        {info.cost !== undefined && (
          <span className="ml-1 text-green-600">{formatCost(info.cost)}</span>
        )}
      </button>

      {isExpanded && (
        <div className="mt-2 p-3 rounded-lg bg-muted/30 border border-border/50 space-y-4 animate-in fade-in slide-in-from-top-1 duration-200">
          {/* Model Info */}
          {(info.modelID || info.providerID) && (
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                <Sparkles className="h-3 w-3" />
                <span>Model</span>
              </div>
              <div className="text-sm font-mono">
                {info.providerID && <span className="text-primary">{info.providerID}</span>}
                {info.providerID && info.modelID && <span className="text-muted-foreground">/</span>}
                {info.modelID && <span>{info.modelID}</span>}
              </div>
            </div>
          )}

          {/* Token Usage */}
          {info.tokens && (
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                <Cpu className="h-3 w-3" />
                <span>Token Usage</span>
              </div>
              <TokenDisplay tokens={info.tokens} />
            </div>
          )}

          {/* Cost */}
          {info.cost !== undefined && (
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                <Coins className="h-3 w-3" />
                <span>Cost</span>
              </div>
              <div className="text-sm font-mono text-green-600">
                {formatCost(info.cost)}
              </div>
            </div>
          )}

          {/* Timing */}
          <div className="space-y-1">
            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
              <Clock className="h-3 w-3" />
              <span>Timing</span>
            </div>
            <TimingDisplay created={info.time.created} completed={info.time.completed} />
          </div>

          {/* Path Info */}
          {info.path && (
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                <Database className="h-3 w-3" />
                <span>Context</span>
              </div>
              <div className="text-xs font-mono text-muted-foreground truncate">
                {info.path.cwd}
              </div>
            </div>
          )}

          {/* Finish Reason */}
          {info.finish && (
            <div className="text-xs">
              <span className="text-muted-foreground">Finish reason: </span>
              <span className={cn(
                "font-medium",
                info.finish === "stop" ? "text-green-600" : "text-yellow-600"
              )}>
                {info.finish}
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export default MessageDetails
