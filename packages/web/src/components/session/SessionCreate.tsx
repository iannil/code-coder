/**
 * SessionCreate Component
 *
 * Provides functionality for creating new sessions:
 * - New session button
 * - Title input dialog
 * - Project selection
 * - Agent selection
 * - Model selection
 */

import * as React from "react"
import { Plus, Sparkles } from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/Button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/Dialog"
import { Input } from "@/components/ui/Input"
import { Label } from "@/components/ui/Label"
import { useSessionStore } from "@/stores/session"
import { useAgents } from "@/stores/agent"
import { useProjects, useProjectStore } from "@/stores/project"
import { useProviders } from "@/stores/provider"
import type { AgentInfo, ProjectInfo, ProviderModel } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

export interface SessionCreateProps {
  /** Optional callback when session is created */
  onCreate?: (sessionId: string) => void

  /** Button variant */
  variant?: "default" | "ghost" | "outline" | "secondary"

  /** Button size */
  size?: "default" | "sm" | "lg" | "icon"

  /** Optional CSS class name */
  className?: string

  /** Trigger element (use DialogTrigger as child) */
  children?: React.ReactNode

  /** Pre-selected project ID */
  defaultProjectId?: string
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Generate a default title for a new session
 */
function generateDefaultTitle(): string {
  const hour = new Date().getHours()
  const period = hour < 12 ? "Morning" : hour < 18 ? "Afternoon" : "Evening"
  return `New ${period} Session`
}

/**
 * Get agent display name
 */
function getAgentDisplayName(agent: AgentInfo): string {
  return agent.name ?? agent.id
}

/**
 * Get project display name
 */
function getProjectDisplayName(project: ProjectInfo): string {
  if (project.name) return project.name
  const parts = project.worktree.split("/")
  return parts[parts.length - 1] || project.worktree
}

// ============================================================================
// Component
// ============================================================================

export const SessionCreate = React.forwardRef<HTMLButtonElement, SessionCreateProps>(
  (
    {
      onCreate,
      variant = "default",
      size = "default",
      className,
      children,
      defaultProjectId,
    },
    ref
  ) => {
    const [open, setOpen] = React.useState(false)
    const [title, setTitle] = React.useState(generateDefaultTitle())
    const [selectedProjectId, setSelectedProjectId] = React.useState<string | null>(defaultProjectId ?? null)
    const [selectedAgentId, setSelectedAgentId] = React.useState<string | null>(null)
    const [selectedModel, setSelectedModel] = React.useState<string>("")
    const [isCreating, setIsCreating] = React.useState(false)
    const titleInputRef = React.useRef<HTMLInputElement>(null)

    const agents = useAgents()
    const projects = useProjects()
    const providers = useProviders()
    const createSession = useSessionStore((s) => s.createSession)
    const loadProjects = useProjectStore((s) => s.loadProjects)

    // Load projects when dialog opens
    React.useEffect(() => {
      if (open && projects.length === 0) {
        loadProjects()
      }
    }, [open, projects.length, loadProjects])

    // Get all available models from providers
    const allModels = React.useMemo(() => {
      const models: Array<{ providerId: string; model: ProviderModel }> = []
      for (const provider of providers) {
        for (const model of Object.values(provider.models)) {
          models.push({ providerId: provider.id, model })
        }
      }
      return models
    }, [providers])

    // Reset form when dialog opens
    React.useEffect(() => {
      if (open) {
        setTitle(generateDefaultTitle())
        setSelectedProjectId(defaultProjectId ?? null)
        setSelectedAgentId(null)
        setSelectedModel("")
        // Focus input after a small delay to ensure dialog is rendered
        setTimeout(() => {
          titleInputRef.current?.focus()
          titleInputRef.current?.select()
        }, 50)
      }
    }, [open, defaultProjectId])

    // Handle create session
    const handleCreate = async (e: React.FormEvent) => {
      e.preventDefault()

      const finalTitle = title.trim() || generateDefaultTitle()

      setIsCreating(true)
      try {
        const session = await createSession({
          title: finalTitle,
          projectID: selectedProjectId ?? undefined,
          agent: selectedAgentId ?? undefined,
          model: selectedModel || undefined,
        })

        setOpen(false)
        onCreate?.(session.id)
      } catch (error) {
        // Dialog stays open on error to show the issue
      } finally {
        setIsCreating(false)
      }
    }

    // Handle quick create (skip dialog)
    const handleQuickCreate = async () => {
      setIsCreating(true)
      try {
        const session = await createSession({
          title: generateDefaultTitle(),
        })
        onCreate?.(session.id)
      } catch (error) {
        // Silent fail for quick create
      } finally {
        setIsCreating(false)
      }
    }

    // Handle keydown (Ctrl+Enter to submit)
    const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
        e.currentTarget.form?.requestSubmit()
      }
    }

    // Default trigger button
    const defaultTrigger = (
      <Button
        ref={ref}
        variant={variant}
        size={size}
        className={cn("gap-2", className)}
      >
        <Plus className="h-4 w-4" />
        <span>New Session</span>
      </Button>
    )

    return (
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          {children ?? defaultTrigger}
        </DialogTrigger>
        <DialogContent className="sm:max-w-md">
          <form onSubmit={handleCreate}>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Sparkles className="h-5 w-5 text-primary" />
                Create New Session
              </DialogTitle>
              <DialogDescription>
                Start a new conversation. Configure the session settings below.
              </DialogDescription>
            </DialogHeader>

            <div className="grid gap-4 py-4">
              {/* Title Input */}
              <div className="grid gap-2">
                <Label htmlFor="session-title">Title</Label>
                <Input
                  ref={titleInputRef}
                  id="session-title"
                  placeholder="Session title..."
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  onKeyDown={handleKeyDown}
                  disabled={isCreating}
                />
              </div>

              {/* Project Selection */}
              {projects.length > 0 && (
                <div className="grid gap-2">
                  <Label htmlFor="project-select">Project</Label>
                  <select
                    id="project-select"
                    className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    value={selectedProjectId ?? ""}
                    onChange={(e) => setSelectedProjectId(e.target.value || null)}
                    disabled={isCreating}
                  >
                    <option value="">Current Project</option>
                    {projects.map((project) => (
                      <option key={project.id} value={project.id}>
                        {getProjectDisplayName(project)}
                      </option>
                    ))}
                  </select>
                  <p className="text-xs text-muted-foreground">
                    Associate the session with a project.
                  </p>
                </div>
              )}

              {/* Agent Selection */}
              {agents.length > 0 && (
                <div className="grid gap-2">
                  <Label htmlFor="agent-select">Agent</Label>
                  <select
                    id="agent-select"
                    className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    value={selectedAgentId ?? ""}
                    onChange={(e) => setSelectedAgentId(e.target.value || null)}
                    disabled={isCreating}
                  >
                    <option value="">Default Agent</option>
                    {agents.map((agent) => (
                      <option key={agent.id} value={agent.id}>
                        {getAgentDisplayName(agent)}
                      </option>
                    ))}
                  </select>
                  <p className="text-xs text-muted-foreground">
                    Select an agent to specialize the session behavior.
                  </p>
                </div>
              )}

              {/* Model Selection */}
              {allModels.length > 0 && (
                <div className="grid gap-2">
                  <Label htmlFor="model-select">Model</Label>
                  <select
                    id="model-select"
                    className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    value={selectedModel}
                    onChange={(e) => setSelectedModel(e.target.value)}
                    disabled={isCreating}
                  >
                    <option value="">Default Model</option>
                    {allModels.map(({ providerId, model }) => (
                      <option key={`${providerId}:${model.id}`} value={`${providerId}:${model.id}`}>
                        {model.name || model.id} ({providerId})
                      </option>
                    ))}
                  </select>
                  <p className="text-xs text-muted-foreground">
                    Choose the AI model to use for this session.
                  </p>
                </div>
              )}

              {/* Keyboard hint */}
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <kbd className="inline-flex items-center rounded-md border border-border bg-muted px-1.5 py-0.5 font-sans text-xs">
                  {/^Mac/i.test(navigator.platform) ? "Cmd" : "Ctrl"}
                </kbd>
                <span>+</span>
                <kbd className="inline-flex items-center rounded-md border border-border bg-muted px-1.5 py-0.5 font-sans text-xs">
                  Enter
                </kbd>
                <span>to create</span>
              </div>
            </div>

            <DialogFooter className="gap-2 sm:gap-0">
              {/* Quick create button */}
              <Button
                type="button"
                variant="ghost"
                onClick={handleQuickCreate}
                disabled={isCreating}
              >
                Quick Create
              </Button>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setOpen(false)}
                  disabled={isCreating}
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={isCreating}>
                  {isCreating ? (
                    <>
                      <span className="h-4 w-4 animate-spin rounded-full border-2 border-primary-foreground border-t-transparent mr-2" />
                      Creating...
                    </>
                  ) : (
                    "Create Session"
                  )}
                </Button>
              </div>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    )
  }
)

SessionCreate.displayName = "SessionCreate"
