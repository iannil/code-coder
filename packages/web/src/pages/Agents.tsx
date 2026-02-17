/**
 * Agents Page
 *
 * Full-page agent management with:
 * - Agent list grouped by category
 * - Search/filter functionality
 * - Quick session start with selected agent
 */

import * as React from "react"
import { useNavigate } from "@tanstack/react-router"
import { Bot, Search, Play, X } from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent } from "@/components/ui/Card"
import { Input } from "@/components/ui/Input"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Separator } from "@/components/ui/Separator"
import { Skeleton } from "@/components/ui/Skeleton"
import { CompactAgentCard } from "@/components/agent/AgentCard"
import { useAgentStore, useAgentsByCategory, useAgentsLoading } from "@/stores/agent"
import { useSessionStore } from "@/stores/session"
import { useToast } from "@/hooks/use-toast"
import type { AgentInfo } from "@/lib/types"

// ============================================================================
// Constants
// ============================================================================

const CATEGORY_LABELS: Record<string, string> = {
  primary: "Main Modes",
  engineering: "Engineering Quality",
  content: "Content Creation",
  zrs: "ZRS (Zhurong Say)",
  reverse: "Reverse Engineering",
  general: "General",
}

const CATEGORY_ORDER = ["primary", "engineering", "content", "zrs", "reverse", "general"]

// ============================================================================
// Empty State Component
// ============================================================================

function AgentsEmptyState() {
  return (
    <div className="flex-1 flex items-center justify-center p-8">
      <Card className="max-w-md text-center">
        <CardContent className="pt-6 space-y-4">
          <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
            <Bot className="h-8 w-8 text-primary" />
          </div>
          <div className="space-y-2">
            <h3 className="text-lg font-semibold">No agents available</h3>
            <p className="text-sm text-muted-foreground">
              Agents are specialized AI assistants for different tasks.
              Configure agents in your project settings.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Agents Header Component
// ============================================================================

interface AgentsHeaderProps {
  totalAgents: number
  searchQuery: string
  onSearchChange: (query: string) => void
}

function AgentsHeader({ totalAgents, searchQuery, onSearchChange }: AgentsHeaderProps) {
  return (
    <div className="flex flex-col gap-4 p-4 border-b bg-muted/30">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
            <Bot className="h-5 w-5 text-primary" />
          </div>
          <div>
            <h1 className="text-lg font-semibold">Agents</h1>
            <p className="text-sm text-muted-foreground">
              {totalAgents === 0
                ? "No agents"
                : `${totalAgents} agent${totalAgents !== 1 ? "s" : ""} available`}
            </p>
          </div>
        </div>
      </div>
      <div className="relative max-w-md">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          type="text"
          placeholder="Search agents..."
          value={searchQuery}
          onChange={(e) => onSearchChange(e.target.value)}
          className="pl-9 pr-9"
          data-testid="agent-search-input"
        />
        {searchQuery && (
          <button
            type="button"
            onClick={() => onSearchChange("")}
            className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Agent Category Section Component
// ============================================================================

interface AgentCategorySectionProps {
  category: string
  agents: AgentInfo[]
  selectedAgentId: string | null
  onAgentClick: (agent: AgentInfo) => void
}

function AgentCategorySection({
  category,
  agents,
  selectedAgentId,
  onAgentClick,
}: AgentCategorySectionProps) {
  const label = CATEGORY_LABELS[category] ?? category

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider px-1">
        {label}
      </h3>
      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {agents.map((agent) => (
          <CompactAgentCard
            key={agent.id}
            agent={agent}
            selected={agent.id === selectedAgentId}
            onClick={() => onAgentClick(agent)}
          />
        ))}
      </div>
    </div>
  )
}

// ============================================================================
// Agent Detail Panel Component
// ============================================================================

interface AgentDetailPanelProps {
  agent: AgentInfo
  onStartSession: () => void
  isStarting: boolean
}

function AgentDetailPanel({ agent, onStartSession, isStarting }: AgentDetailPanelProps) {
  return (
    <Card>
      <CardContent className="p-4 space-y-4">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <Bot className="h-5 w-5 text-primary" />
            </div>
            <div>
              <h3 className="font-semibold">{agent.name}</h3>
              {agent.category && (
                <p className="text-xs text-muted-foreground">
                  {CATEGORY_LABELS[agent.category] ?? agent.category}
                </p>
              )}
            </div>
          </div>
          <Button onClick={onStartSession} disabled={isStarting} data-testid="start-session-btn">
            <Play className="mr-2 h-4 w-4" />
            {isStarting ? "Starting..." : "Start Session"}
          </Button>
        </div>
        {agent.description && (
          <p className="text-sm text-muted-foreground">{agent.description}</p>
        )}
        {agent.system && agent.system.length > 0 && (
          <div className="space-y-2">
            <h4 className="text-xs font-medium text-muted-foreground uppercase">System Prompt</h4>
            <div className="rounded-md bg-muted p-3 max-h-32 overflow-y-auto">
              <p className="text-sm line-clamp-4">{agent.system[0]}</p>
              {agent.system.length > 1 && (
                <p className="text-xs text-muted-foreground mt-2">
                  +{agent.system.length - 1} more prompts
                </p>
              )}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Main Agents Page Component
// ============================================================================

export function Agents() {
  const navigate = useNavigate()
  const { loadAgents, selectAgent } = useAgentStore()
  const agentsByCategory = useAgentsByCategory()
  const { isLoading, isLoaded } = useAgentsLoading()
  const { createSession } = useSessionStore()
  const { toast } = useToast()

  const [searchQuery, setSearchQuery] = React.useState("")
  const [selectedAgent, setSelectedAgent] = React.useState<AgentInfo | null>(null)
  const [isStarting, setIsStarting] = React.useState(false)

  // Load agents on mount
  React.useEffect(() => {
    loadAgents()
  }, [loadAgents])

  // Filter agents by search query
  const filteredCategories = React.useMemo(() => {
    const result: Record<string, AgentInfo[]> = {}
    const lowerQuery = searchQuery.toLowerCase()

    for (const category of CATEGORY_ORDER) {
      const agents = agentsByCategory[category] ?? []
      const filtered = searchQuery
        ? agents.filter(
            (agent) =>
              agent.name.toLowerCase().includes(lowerQuery) ||
              agent.description?.toLowerCase().includes(lowerQuery)
          )
        : agents

      if (filtered.length > 0) {
        result[category] = filtered
      }
    }

    return result
  }, [agentsByCategory, searchQuery])

  const totalAgents = Object.values(agentsByCategory).reduce(
    (sum, agents) => sum + agents.length,
    0
  )

  const filteredCount = Object.values(filteredCategories).reduce(
    (sum, agents) => sum + agents.length,
    0
  )

  const handleAgentClick = (agent: AgentInfo) => {
    setSelectedAgent(agent)
    selectAgent(agent.id)
  }

  const handleStartSession = async () => {
    if (!selectedAgent) return

    setIsStarting(true)
    try {
      const session = await createSession({
        title: `${selectedAgent.name} Session`,
      })
      navigate({ to: "/sessions/$sessionId", params: { sessionId: session.id } })
      toast({
        title: "Session started",
        description: `Started a new session with ${selectedAgent.name}.`,
      })
    } catch {
      toast({
        title: "Failed to start session",
        description: "An error occurred. Please try again.",
        variant: "destructive",
      })
    } finally {
      setIsStarting(false)
    }
  }

  // Loading state
  if (isLoading && !isLoaded) {
    return (
      <div className="flex flex-col h-full bg-background">
        <AgentsHeader
          totalAgents={0}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
        />
        <div className="flex-1 p-4 space-y-6">
          {[1, 2, 3].map((i) => (
            <div key={i} className="space-y-3">
              <Skeleton className="h-4 w-32" />
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                {[1, 2, 3].map((j) => (
                  <Skeleton key={j} className="h-20 w-full" />
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    )
  }

  // Empty state
  if (isLoaded && totalAgents === 0) {
    return (
      <div className="flex flex-col h-full bg-background">
        <AgentsHeader
          totalAgents={0}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
        />
        <AgentsEmptyState />
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full bg-background">
      <AgentsHeader
        totalAgents={totalAgents}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
      />
      <div className="flex flex-1 overflow-hidden">
        <ScrollArea className="flex-1 p-4">
          <div className="space-y-6">
            {filteredCount === 0 ? (
              <div className="flex flex-col items-center justify-center py-12 text-center">
                <Search className="h-12 w-12 text-muted-foreground mb-4" />
                <p className="text-sm font-medium">No agents found</p>
                <p className="text-xs text-muted-foreground mt-1">
                  Try a different search term
                </p>
              </div>
            ) : (
              Object.entries(filteredCategories).map(([category, agents], index) => (
                <React.Fragment key={category}>
                  <AgentCategorySection
                    category={category}
                    agents={agents}
                    selectedAgentId={selectedAgent?.id ?? null}
                    onAgentClick={handleAgentClick}
                  />
                  {index < Object.keys(filteredCategories).length - 1 && (
                    <Separator />
                  )}
                </React.Fragment>
              ))
            )}
          </div>
        </ScrollArea>

        {/* Detail Panel */}
        {selectedAgent && (
          <div className="w-80 border-l p-4 bg-muted/20">
            <AgentDetailPanel
              agent={selectedAgent}
              onStartSession={handleStartSession}
              isStarting={isStarting}
            />
          </div>
        )}
      </div>
    </div>
  )
}

export default Agents
