/**
 * AgentSelector Component
 *
 * A dropdown component for selecting agents with search/filter functionality
 * and badges indicating agent modes (primary, subagent).
 */

import * as React from "react"
import { Search, Check, Bot } from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/Button"
import { Badge } from "@/components/ui/Badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/DropdownMenu"
import { Input } from "@/components/ui/Input"
import type { AgentInfo } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

export interface AgentSelectorProps {
  /** All available agents */
  agents: AgentInfo[]
  /** Currently selected agent ID */
  selectedId: string | null
  /** Callback when an agent is selected */
  onSelect: (agentId: string) => void
  /** Optional placeholder text */
  placeholder?: string
  /** Optional className */
  className?: string
}

export interface AgentModeBadgeProps {
  /** The mode/category of the agent */
  mode: string
  /** Optional className */
  className?: string
}

// ============================================================================
// Agent Mode Badge
// ============================================================================

type BadgeVariant = "default" | "secondary" | "destructive" | "outline" | "success" | "warning" | "info" | "purple" | "pink" | "orange"

const MODE_VARIANTS: Record<string, BadgeVariant> = {
  primary: "default",
  subagent: "secondary",
  build: "info",
  plan: "purple",
  code: "success",
  content: "orange",
  zrs: "pink",
  default: "outline",
}

const MODE_LABELS: Record<string, string> = {
  primary: "Primary",
  subagent: "Subagent",
  build: "Build",
  plan: "Plan",
  code: "Code",
  content: "Content",
  zrs: "ZRS",
  default: "Agent",
}

function getModeVariant(mode: string): BadgeVariant {
  return MODE_VARIANTS[mode.toLowerCase()] ?? MODE_VARIANTS.default
}

function getModeLabel(mode: string): string {
  return MODE_LABELS[mode.toLowerCase()] ?? MODE_LABELS.default
}

export function AgentModeBadge({ mode, className }: AgentModeBadgeProps) {
  return (
    <Badge variant={getModeVariant(mode)} className={className}>
      {getModeLabel(mode)}
    </Badge>
  )
}

// ============================================================================
// Agent Selector
// ============================================================================

export function AgentSelector({
  agents,
  selectedId,
  onSelect,
  placeholder = "Select an agent",
  className,
}: AgentSelectorProps) {
  const [searchQuery, setSearchQuery] = React.useState("")
  const [open, setOpen] = React.useState(false)

  // Group agents by category
  const groupedAgents = React.useMemo(() => {
    const groups = new Map<string, AgentInfo[]>()
    const filtered = agents.filter(
      (agent) =>
        agent.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        agent.description?.toLowerCase().includes(searchQuery.toLowerCase())
    )

    for (const agent of filtered) {
      const category = agent.category ?? "general"
      const currentAgents = groups.get(category) ?? []
      currentAgents.push(agent)
      groups.set(category, currentAgents)
    }

    return groups
  }, [agents, searchQuery])

  const selectedAgent = agents.find((a) => a.id === selectedId)

  const handleSelect = (agentId: string) => {
    onSelect(agentId)
    setOpen(false)
    setSearchQuery("")
  }

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" data-testid="agent-selector" className={cn("justify-between", className)}>
          <span className="flex items-center gap-2 truncate">
            {selectedAgent ? (
              <>
                <Bot className="h-4 w-4 shrink-0" />
                <span className="truncate">{selectedAgent.name}</span>
                {selectedAgent.category && (
                  <AgentModeBadge mode={selectedAgent.category} />
                )}
              </>
            ) : (
              <>
                <Bot className="h-4 w-4 shrink-0 text-muted-foreground" />
                <span className="text-muted-foreground">{placeholder}</span>
              </>
            )}
          </span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-72 p-2" align="start">
        {/* Search Input */}
        <div className="px-2 pb-2">
          <div className="relative">
            <Search className="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search agents..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 h-9"
            />
          </div>
        </div>

        <DropdownMenuSeparator />

        {/* Agent List */}
        {groupedAgents.size === 0 ? (
          <div className="py-4 text-center text-sm text-muted-foreground">
            No agents found
          </div>
        ) : (
          <DropdownMenuGroup className="max-h-64 overflow-y-auto">
            {Array.from(groupedAgents.entries()).map(([category, categoryAgents]) => (
              <React.Fragment key={category}>
                <DropdownMenuLabel className="px-2 py-1.5 text-xs font-semibold uppercase text-muted-foreground">
                  {category}
                </DropdownMenuLabel>
                {categoryAgents.map((agent) => (
                  <DropdownMenuItem
                    key={agent.id}
                    onSelect={() => handleSelect(agent.id)}
                    data-testid="agent-option"
                    className="flex flex-col items-start gap-1 px-2 py-2"
                  >
                    <div className="flex items-center gap-2 w-full">
                      <Bot className="h-4 w-4 shrink-0 text-muted-foreground" />
                      <span className="flex-1 truncate">{agent.name}</span>
                      {agent.category && <AgentModeBadge mode={agent.category} />}
                      {agent.id === selectedId && (
                        <Check className="h-4 w-4 text-primary shrink-0" />
                      )}
                    </div>
                    {agent.description && (
                      <span data-testid="agent-description" className="text-xs text-muted-foreground pl-6 truncate max-w-full">
                        {agent.description}
                      </span>
                    )}
                  </DropdownMenuItem>
                ))}
              </React.Fragment>
            ))}
          </DropdownMenuGroup>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

// ============================================================================
// Selected Agent Display
// ============================================================================

export interface SelectedAgentDisplayProps {
  /** The selected agent */
  agent: AgentInfo | null
  /** Optional className */
  className?: string
}

export function SelectedAgentDisplay({ agent, className }: SelectedAgentDisplayProps) {
  if (!agent) {
    return (
      <div className={cn("flex items-center gap-2 text-muted-foreground", className)}>
        <Bot className="h-4 w-4" />
        <span className="text-sm">No agent selected</span>
      </div>
    )
  }

  return (
    <div className={cn("flex items-center gap-2", className)}>
      <Bot className="h-4 w-4 text-primary" />
      <span className="font-medium">{agent.name}</span>
      {agent.category && <AgentModeBadge mode={agent.category} />}
    </div>
  )
}
