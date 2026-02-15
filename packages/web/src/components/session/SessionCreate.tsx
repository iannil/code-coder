/**
 * SessionCreate Component
 *
 * Provides functionality for creating new sessions:
 * - New session button
 * - Title input dialog
 * - Agent selection
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
import type { AgentInfo } from "@/lib/types"

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
    },
    ref
  ) => {
    const [open, setOpen] = React.useState(false)
    const [title, setTitle] = React.useState(generateDefaultTitle())
    const [selectedAgentId, setSelectedAgentId] = React.useState<string | null>(null)
    const [isCreating, setIsCreating] = React.useState(false)
    const titleInputRef = React.useRef<HTMLInputElement>(null)

    const agents = useAgents()
    const createSession = useSessionStore((s) => s.createSession)

    // Reset form when dialog opens
    React.useEffect(() => {
      if (open) {
        setTitle(generateDefaultTitle())
        setSelectedAgentId(null)
        // Focus input after a small delay to ensure dialog is rendered
        setTimeout(() => {
          titleInputRef.current?.focus()
          titleInputRef.current?.select()
        }, 50)
      }
    }, [open])

    // Handle create session
    const handleCreate = async (e: React.FormEvent) => {
      e.preventDefault()

      const finalTitle = title.trim() || generateDefaultTitle()

      setIsCreating(true)
      try {
        const session = await createSession({
          title: finalTitle,
          // parentID can be added here if needed
        })

        setOpen(false)
        onCreate?.(session.id)
      } catch (error) {
        console.error("Failed to create session:", error)
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
        console.error("Failed to create session:", error)
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
                Start a new conversation. You can optionally provide a title and select an agent.
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
                <p className="text-xs text-muted-foreground">
                  A descriptive title helps you organize your sessions.
                </p>
              </div>

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
