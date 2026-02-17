/**
 * TaskDetail Component
 * Dialog showing task details, events, and interaction options
 */

import * as React from "react"
import {
  Play,
  CheckCircle,
  XCircle,
  Clock,
  AlertCircle,
  ThumbsUp,
  ThumbsDown,
  Loader2,
  MessageSquare,
  Wrench,
  FileText,
  Activity,
} from "lucide-react"
import { Button } from "@/components/ui/Button"
import { Badge } from "@/components/ui/Badge"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Separator } from "@/components/ui/Separator"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/Dialog"
import {
  type Task,
  type TaskStatus,
  type TaskEvent,
  useTaskEvents,
  useTaskInteracting,
  useTaskStore,
} from "@/stores/task"
import { useToast } from "@/hooks/use-toast"
import { cn, formatTimestamp } from "@/lib/utils"

// ============================================================================
// Status Configuration
// ============================================================================

const statusConfig: Record<TaskStatus, { icon: typeof Clock; label: string; color: string }> = {
  pending: { icon: Clock, label: "Pending", color: "text-muted-foreground" },
  running: { icon: Play, label: "Running", color: "text-blue-500" },
  awaiting_approval: { icon: AlertCircle, label: "Awaiting Approval", color: "text-yellow-500" },
  completed: { icon: CheckCircle, label: "Completed", color: "text-green-500" },
  failed: { icon: XCircle, label: "Failed", color: "text-destructive" },
}

// ============================================================================
// Event Item Component
// ============================================================================

interface EventItemProps {
  event: TaskEvent
}

function EventItem({ event }: EventItemProps) {
  const getEventIcon = () => {
    switch (event.type) {
      case "thought":
        return <MessageSquare className="h-4 w-4 text-muted-foreground" />
      case "tool_use":
        return <Wrench className="h-4 w-4 text-blue-500" />
      case "output":
        return <FileText className="h-4 w-4 text-green-500" />
      case "confirmation":
        return <AlertCircle className="h-4 w-4 text-yellow-500" />
      case "finish":
        return event.data.success ? (
          <CheckCircle className="h-4 w-4 text-green-500" />
        ) : (
          <XCircle className="h-4 w-4 text-destructive" />
        )
      case "progress":
        return <Activity className="h-4 w-4 text-blue-500" />
      default:
        return <MessageSquare className="h-4 w-4" />
    }
  }

  const getEventContent = () => {
    switch (event.type) {
      case "thought":
        return (
          <div className="text-sm text-muted-foreground">
            <span className="font-medium">Thought: </span>
            {event.data}
          </div>
        )
      case "tool_use":
        return (
          <div className="text-sm">
            <span className="font-medium">Tool: </span>
            <code className="bg-muted px-1 rounded">{event.data.tool}</code>
            {event.data.result !== undefined && (
              <div className="mt-1 text-xs text-muted-foreground">
                Result: {typeof event.data.result === "string" ? event.data.result : JSON.stringify(event.data.result)}
              </div>
            )}
          </div>
        )
      case "output":
        return (
          <div className="text-sm">
            <span className="font-medium">Output: </span>
            {event.data}
          </div>
        )
      case "confirmation":
        return (
          <div className="text-sm">
            <span className="font-medium">Confirmation Required: </span>
            {event.data.description}
            <div className="mt-1 text-xs text-muted-foreground">
              Tool: {event.data.tool} | Actions: {event.data.actions.join(", ")}
            </div>
          </div>
        )
      case "finish":
        return (
          <div className="text-sm">
            <span className="font-medium">{event.data.success ? "Completed" : "Failed"}: </span>
            {event.data.success ? event.data.output : event.data.error}
          </div>
        )
      case "progress":
        return (
          <div className="text-sm">
            <span className="font-medium">{event.data.stage}: </span>
            {event.data.message}
            {event.data.percentage !== undefined && (
              <span className="ml-2 text-muted-foreground">({event.data.percentage}%)</span>
            )}
          </div>
        )
      default:
        return <div className="text-sm">Unknown event type</div>
    }
  }

  return (
    <div className="flex gap-3 py-2">
      <div className="shrink-0 mt-0.5">{getEventIcon()}</div>
      <div className="flex-1 min-w-0">{getEventContent()}</div>
    </div>
  )
}

// ============================================================================
// Props
// ============================================================================

interface TaskDetailProps {
  task: Task | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

// ============================================================================
// Component
// ============================================================================

export function TaskDetail({ task, open, onOpenChange }: TaskDetailProps) {
  const events = useTaskEvents(task?.id ?? "")
  const isInteracting = useTaskInteracting(task?.id ?? "")
  const interactTask = useTaskStore((state) => state.interactTask)
  const subscribeToEvents = useTaskStore((state) => state.subscribeToEvents)
  const unsubscribeFromEvents = useTaskStore((state) => state.unsubscribeFromEvents)
  const { toast } = useToast()

  // Subscribe to task events when dialog opens
  React.useEffect(() => {
    if (open && task && (task.status === "running" || task.status === "awaiting_approval")) {
      subscribeToEvents(task.id)
      return () => unsubscribeFromEvents(task.id)
    }
  }, [open, task, subscribeToEvents, unsubscribeFromEvents])

  const handleApprove = async () => {
    if (!task) return
    try {
      await interactTask(task.id, { action: "approve", reply: "once" })
      toast({
        title: "Task approved",
        description: "The task has been approved and will continue.",
      })
    } catch {
      toast({
        title: "Failed to approve task",
        description: "An error occurred while approving the task.",
        variant: "destructive",
      })
    }
  }

  const handleReject = async () => {
    if (!task) return
    try {
      await interactTask(task.id, { action: "reject", reason: "Rejected by user" })
      toast({
        title: "Task rejected",
        description: "The task has been rejected.",
      })
    } catch {
      toast({
        title: "Failed to reject task",
        description: "An error occurred while rejecting the task.",
        variant: "destructive",
      })
    }
  }

  if (!task) return null

  const config = statusConfig[task.status]
  const StatusIcon = config.icon

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[600px] max-h-[80vh]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <StatusIcon className={cn("h-5 w-5", config.color)} />
            Task Details
          </DialogTitle>
          <DialogDescription>
            View task information, events, and interact with pending approvals.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Task Info */}
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-muted-foreground">ID:</span>
              <span className="ml-2 font-mono">{task.id}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Status:</span>
              <Badge variant="outline" className="ml-2">
                {config.label}
              </Badge>
            </div>
            <div>
              <span className="text-muted-foreground">Agent:</span>
              <Badge variant="secondary" className="ml-2">
                {task.agent}
              </Badge>
            </div>
            <div>
              <span className="text-muted-foreground">Platform:</span>
              <span className="ml-2">{task.context.platform}</span>
            </div>
            <div className="col-span-2">
              <span className="text-muted-foreground">User:</span>
              <span className="ml-2">{task.context.userID}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Created:</span>
              <span className="ml-2">{formatTimestamp(new Date(task.createdAt).getTime())}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Updated:</span>
              <span className="ml-2">{formatTimestamp(new Date(task.updatedAt).getTime())}</span>
            </div>
          </div>

          <Separator />

          {/* Prompt */}
          <div>
            <h4 className="text-sm font-medium mb-2">Prompt</h4>
            <div className="bg-muted p-3 rounded-md text-sm">{task.prompt}</div>
          </div>

          {/* Output or Error */}
          {task.output && (
            <div>
              <h4 className="text-sm font-medium mb-2">Output</h4>
              <div className="bg-muted p-3 rounded-md text-sm max-h-32 overflow-auto">{task.output}</div>
            </div>
          )}
          {task.error && (
            <div>
              <h4 className="text-sm font-medium mb-2 text-destructive">Error</h4>
              <div className="bg-destructive/10 border border-destructive/20 p-3 rounded-md text-sm text-destructive">
                {task.error}
              </div>
            </div>
          )}

          {/* Events */}
          {events.length > 0 && (
            <>
              <Separator />
              <div>
                <h4 className="text-sm font-medium mb-2">Events ({events.length})</h4>
                <ScrollArea className="h-48 border rounded-md p-2">
                  <div className="divide-y">
                    {events.map((event, index) => (
                      <EventItem key={index} event={event} />
                    ))}
                  </div>
                </ScrollArea>
              </div>
            </>
          )}
        </div>

        {/* Approval Actions */}
        {task.status === "awaiting_approval" && (
          <DialogFooter className="mt-4">
            <Button variant="outline" onClick={handleReject} disabled={isInteracting}>
              {isInteracting ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <ThumbsDown className="h-4 w-4 mr-2" />
              )}
              Reject
            </Button>
            <Button onClick={handleApprove} disabled={isInteracting}>
              {isInteracting ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <ThumbsUp className="h-4 w-4 mr-2" />
              )}
              Approve
            </Button>
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  )
}
