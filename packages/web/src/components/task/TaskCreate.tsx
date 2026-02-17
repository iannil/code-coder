/**
 * TaskCreate Component
 * Form for creating new async tasks
 */

import * as React from "react"
import { Plus, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/Button"
import { Input } from "@/components/ui/Input"
import { Label } from "@/components/ui/Label"
import { Textarea } from "@/components/ui/Textarea"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/Select"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/Dialog"
import { useTaskStore, useTasksLoading, type CreateTaskInput } from "@/stores/task"
import { useAgents } from "@/stores/agent"
import { useToast } from "@/hooks/use-toast"
import { cn } from "@/lib/utils"

// ============================================================================
// Props
// ============================================================================

interface TaskCreateProps {
  className?: string
}

// ============================================================================
// Component
// ============================================================================

export function TaskCreate({ className }: TaskCreateProps) {
  const agents = useAgents()
  const createTask = useTaskStore((state) => state.createTask)
  const { isCreating } = useTasksLoading()
  const { toast } = useToast()

  const [open, setOpen] = React.useState(false)
  const [formData, setFormData] = React.useState({
    agent: "",
    prompt: "",
    userID: "web-user",
    platform: "web",
  })

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    if (!formData.agent || !formData.prompt.trim()) {
      toast({
        title: "Validation error",
        description: "Please select an agent and enter a prompt.",
        variant: "destructive",
      })
      return
    }

    const input: CreateTaskInput = {
      agent: formData.agent,
      prompt: formData.prompt.trim(),
      context: {
        userID: formData.userID || "web-user",
        platform: formData.platform || "web",
        source: "remote",
      },
    }

    try {
      const task = await createTask(input)
      toast({
        title: "Task created",
        description: `Task ${task.id.slice(0, 8)} has been created successfully.`,
      })
      setOpen(false)
      setFormData({
        agent: "",
        prompt: "",
        userID: "web-user",
        platform: "web",
      })
    } catch {
      toast({
        title: "Failed to create task",
        description: "An error occurred while creating the task.",
        variant: "destructive",
      })
    }
  }

  const handleReset = () => {
    setFormData({
      agent: "",
      prompt: "",
      userID: "web-user",
      platform: "web",
    })
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button className={cn("gap-2", className)}>
          <Plus className="h-4 w-4" />
          New Task
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[500px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Create New Task</DialogTitle>
            <DialogDescription>
              Create a new async task that will be executed by an agent. The task will run in the background and you
              can monitor its progress.
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-4">
            {/* Agent Selection */}
            <div className="grid gap-2">
              <Label htmlFor="agent">Agent</Label>
              <Select value={formData.agent} onValueChange={(v) => setFormData({ ...formData, agent: v })}>
                <SelectTrigger id="agent">
                  <SelectValue placeholder="Select an agent" />
                </SelectTrigger>
                <SelectContent>
                  {agents.map((agent) => (
                    <SelectItem key={agent.id} value={agent.id}>
                      <div className="flex flex-col">
                        <span>{agent.name}</span>
                        {agent.description && (
                          <span className="text-xs text-muted-foreground">{agent.description}</span>
                        )}
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Prompt */}
            <div className="grid gap-2">
              <Label htmlFor="prompt">Prompt</Label>
              <Textarea
                id="prompt"
                placeholder="Enter the task prompt..."
                value={formData.prompt}
                onChange={(e) => setFormData({ ...formData, prompt: e.target.value })}
                rows={4}
                className="resize-none"
              />
            </div>

            {/* Context Fields */}
            <div className="grid grid-cols-2 gap-4">
              <div className="grid gap-2">
                <Label htmlFor="userID">User ID</Label>
                <Input
                  id="userID"
                  placeholder="web-user"
                  value={formData.userID}
                  onChange={(e) => setFormData({ ...formData, userID: e.target.value })}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="platform">Platform</Label>
                <Select value={formData.platform} onValueChange={(v) => setFormData({ ...formData, platform: v })}>
                  <SelectTrigger id="platform">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="web">Web</SelectItem>
                    <SelectItem value="telegram">Telegram</SelectItem>
                    <SelectItem value="discord">Discord</SelectItem>
                    <SelectItem value="slack">Slack</SelectItem>
                    <SelectItem value="api">API</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={handleReset}>
              Reset
            </Button>
            <Button type="submit" disabled={isCreating}>
              {isCreating ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Creating...
                </>
              ) : (
                "Create Task"
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
