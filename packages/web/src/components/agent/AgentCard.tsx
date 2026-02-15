/**
 * AgentCard Component
 *
 * A card component displaying detailed agent information including:
 * - Agent name and description
 * - Mode/category indicator with badge
 * - Model selection dropdown
 * - Permission summary
 */

import { Bot, Settings, Shield, Check } from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/Button"
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "@/components/ui/Card"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/DropdownMenu"
import { AgentModeBadge } from "./AgentSelector"
import type { AgentInfo } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

export interface AgentCardProps {
  /** The agent to display */
  agent: AgentInfo
  /** Available models for selection */
  models?: Array<{ id: string; name: string; provider: string }>
  /** Currently selected model ID */
  selectedModelId?: string
  /** Callback when model is changed */
  onModelChange?: (modelId: string) => void
  /** Whether to show detailed permissions */
  showPermissions?: boolean
  /** Optional className */
  className?: string
}

export interface PermissionSummaryProps {
  /** Permission ruleset */
  permissions?: Record<string, boolean | string[]>
  /** Optional className */
  className?: string
}

// ============================================================================
// Permission Summary
// ============================================================================

export function PermissionSummary({ permissions, className }: PermissionSummaryProps) {
  if (!permissions || Object.keys(permissions).length === 0) {
    return (
      <div className={cn("text-sm text-muted-foreground", className)}>
        No special permissions
      </div>
    )
  }

  const permissionEntries = Object.entries(permissions).filter(([_, value]) => {
    if (Array.isArray(value)) return value.length > 0
    return value === true
  })

  if (permissionEntries.length === 0) {
    return (
      <div className={cn("text-sm text-muted-foreground", className)}>
        No special permissions
      </div>
    )
  }

  return (
    <div className={cn("space-y-1", className)}>
      <div className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground uppercase">
        <Shield className="h-3 w-3" />
        Permissions ({permissionEntries.length})
      </div>
      <ul className="space-y-0.5">
        {permissionEntries.map(([key, value]) => (
          <li key={key} className="flex items-center gap-2 text-sm">
            <Check className="h-3 w-3 text-green-500 shrink-0" />
            <span className="truncate">{key}</span>
            {Array.isArray(value) && value.length > 0 && (
              <span className="text-xs text-muted-foreground truncate">
                ({value.join(", ")})
              </span>
            )}
          </li>
        ))}
      </ul>
    </div>
  )
}

// ============================================================================
// Model Selector
// ============================================================================

export interface ModelSelectorProps {
  /** Available models */
  models: Array<{ id: string; name: string; provider: string }>
  /** Currently selected model ID */
  selectedId?: string
  /** Callback when model is changed */
  onChange?: (modelId: string) => void
  /** Optional className */
  className?: string
}

export function ModelSelector({ models, selectedId, onChange, className }: ModelSelectorProps) {
  const selectedModel = models.find((m) => m.id === selectedId) ?? models[0]

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className={cn("justify-start gap-2", className)}>
          <Settings className="h-3.5 w-3.5" />
          <span className="truncate">{selectedModel?.name ?? "Select Model"}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-56">
        {models.map((model) => (
          <DropdownMenuItem
            key={model.id}
            onSelect={() => onChange?.(model.id)}
            className={cn("flex-col items-start gap-0.5", {
              "bg-accent": model.id === selectedId,
            })}
          >
            <div className="flex w-full items-center justify-between">
              <span className="font-medium">{model.name}</span>
              {model.id === selectedId && (
                <Check className="h-4 w-4 text-primary shrink-0" />
              )}
            </div>
            <span className="text-xs text-muted-foreground">{model.provider}</span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

// ============================================================================
// Agent Card
// ============================================================================

export function AgentCard({
  agent,
  models = [],
  selectedModelId,
  onModelChange,
  showPermissions = false,
  className,
}: AgentCardProps) {
  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2">
            <Bot className="h-5 w-5 text-primary" />
            <CardTitle className="text-lg">{agent.name}</CardTitle>
          </div>
          {agent.category && <AgentModeBadge mode={agent.category} />}
        </div>
        {agent.description && (
          <CardDescription className="mt-2">{agent.description}</CardDescription>
        )}
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Model Selection */}
        {models.length > 0 && (
          <div>
            <label className="text-xs font-medium text-muted-foreground uppercase">
              Model
            </label>
            <div className="mt-1.5">
              <ModelSelector
                models={models}
                selectedId={selectedModelId}
                onChange={onModelChange}
              />
            </div>
          </div>
        )}

        {/* System Prompt Preview */}
        {agent.system && agent.system.length > 0 && (
          <div>
            <label className="text-xs font-medium text-muted-foreground uppercase">
              System Prompt
            </label>
            <div className="mt-1.5 rounded-md bg-muted p-3">
              <p className="text-sm line-clamp-3">{agent.system[0]}</p>
              {agent.system.length > 1 && (
                <p className="mt-1 text-xs text-muted-foreground">
                  +{agent.system.length - 1} more prompts
                </p>
              )}
            </div>
          </div>
        )}

        {/* Permissions */}
        {showPermissions && (
          <PermissionSummary permissions={agent.permission} />
        )}
      </CardContent>

      {/* Optional footer with actions */}
      {agent.permission && !showPermissions && (
        <CardFooter>
          <Button variant="ghost" size="sm" className="w-full">
            <Shield className="mr-2 h-4 w-4" />
            View Permissions ({Object.keys(agent.permission).length})
          </Button>
        </CardFooter>
      )}
    </Card>
  )
}

// ============================================================================
// Compact Agent Card (for list views)
// ============================================================================

export interface CompactAgentCardProps {
  /** The agent to display */
  agent: AgentInfo
  /** Whether this agent is selected */
  selected?: boolean
  /** Click handler */
  onClick?: () => void
  /** Optional className */
  className?: string
}

export function CompactAgentCard({
  agent,
  selected = false,
  onClick,
  className,
}: CompactAgentCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "w-full flex items-start gap-3 p-3 rounded-lg border transition-colors text-left",
        "hover:bg-accent hover:border-accent",
        selected && "bg-accent border-primary",
        className
      )}
    >
      <Bot
        className={cn(
          "h-5 w-5 shrink-0 mt-0.5",
          selected ? "text-primary" : "text-muted-foreground"
        )}
      />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-medium truncate">{agent.name}</span>
          {agent.category && <AgentModeBadge mode={agent.category} />}
        </div>
        {agent.description && (
          <p className="text-sm text-muted-foreground line-clamp-1 mt-0.5">
            {agent.description}
          </p>
        )}
      </div>
    </button>
  )
}
