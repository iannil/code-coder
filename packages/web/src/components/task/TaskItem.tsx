/**
 * TaskItem Component
 * Displays a single task card with status, agent, and actions
 */

import * as React from "react"
import { Play, CheckCircle, XCircle, Clock, AlertCircle, Trash2, Eye } from "lucide-react"
import { Card, CardContent, CardHeader } from "@/components/ui/Card"
import { Button } from "@/components/ui/Button"
import { Badge } from "@/components/ui/Badge"
import { cn, formatTimestamp } from "@/lib/utils"
import type { Task, TaskStatus } from "@/stores/task"
import { useTaskDeleting, useTaskStore } from "@/stores/task"
import { useToast } from "@/hooks/use-toast"

// ============================================================================
// Status Configuration
// ============================================================================

const statusConfig: Record<TaskStatus, { icon: typeof Clock; label: string; variant: "default" | "secondary" | "destructive" | "outline" }> = {
  pending: { icon: Clock, label: "Pending", variant: "secondary" },
  running: { icon: Play, label: "Running", variant: "default" },
  awaiting_approval: { icon: AlertCircle, label: "Awaiting Approval", variant: "outline" },
  completed: { icon: CheckCircle, label: "Completed", variant: "default" },
  failed: { icon: XCircle, label: "Failed", variant: "destructive" },
}

// ============================================================================
// Props
// ============================================================================

interface TaskItemProps {
  task: Task
  onViewDetails: (task: Task) => void
  className?: string
}

// ============================================================================
// Component
// ============================================================================

export function TaskItem({ task, onViewDetails, className }: TaskItemProps) {
  const isDeleting = useTaskDeleting(task.id)
  const deleteTask = useTaskStore((state) => state.deleteTask)
  const { toast } = useToast()

  const config = statusConfig[task.status]
  const StatusIcon = config.icon

  const handleDelete = async (e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      await deleteTask(task.id)
      toast({
        title: "Task deleted",
        description: "The task has been deleted successfully.",
      })
    } catch {
      toast({
        title: "Failed to delete task",
        description: "An error occurred while deleting the task.",
        variant: "destructive",
      })
    }
  }

  const handleViewDetails = () => {
    onViewDetails(task)
  }

  const isActive = task.status === "running" || task.status === "pending" || task.status === "awaiting_approval"

  return (
    <Card
      data-testid="task-item"
      data-status={task.status}
      className={cn(
        "cursor-pointer transition-colors hover:bg-muted/50",
        isActive && "border-primary/50",
        className
      )}
      onClick={handleViewDetails}
    >
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <Badge variant={config.variant} className="shrink-0">
                <StatusIcon className="h-3 w-3 mr-1" />
                {config.label}
              </Badge>
              <Badge variant="outline" className="shrink-0">
                {task.agent}
              </Badge>
            </div>
            <p className="text-sm text-muted-foreground truncate">
              {task.context.platform} / {task.context.userID}
            </p>
          </div>
          <div className="flex items-center gap-1 shrink-0">
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleViewDetails}
              title="View details"
            >
              <Eye className="h-4 w-4" />
            </Button>
            {!isActive && (
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-destructive hover:text-destructive"
                onClick={handleDelete}
                disabled={isDeleting}
                title="Delete task"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent className="pt-0">
        <p className="text-sm line-clamp-2 mb-2">{task.prompt}</p>
        {task.output && (
          <p className="text-xs text-muted-foreground line-clamp-1 mb-2">
            Output: {task.output}
          </p>
        )}
        {task.error && (
          <p className="text-xs text-destructive line-clamp-1 mb-2">
            Error: {task.error}
          </p>
        )}
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>Created: {formatTimestamp(new Date(task.createdAt).getTime())}</span>
          <span>ID: {task.id.slice(0, 8)}...</span>
        </div>
      </CardContent>
    </Card>
  )
}
