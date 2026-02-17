/**
 * Tasks Page
 *
 * Displays async task management interface with:
 * - Task list with filtering by status
 * - Task creation form
 * - Task detail dialog with events
 * - Real-time updates via SSE
 */

import * as React from "react"
import { ClipboardList } from "lucide-react"
import { Card, CardContent } from "@/components/ui/Card"
import { TaskList, TaskCreate, TaskDetail } from "@/components/task"
import { useTaskStore, useTasksLoading, useTaskCounts, type Task } from "@/stores/task"
import { useAgentStore } from "@/stores/agent"

// ============================================================================
// Empty State Component
// ============================================================================

function TasksEmptyState() {
  return (
    <div className="flex-1 flex items-center justify-center p-8">
      <Card className="max-w-md text-center">
        <CardContent className="pt-6 space-y-4">
          <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
            <ClipboardList className="h-8 w-8 text-primary" />
          </div>
          <div className="space-y-2">
            <h3 className="text-lg font-semibold">No tasks yet</h3>
            <p className="text-sm text-muted-foreground">
              Create async tasks that run in the background. Tasks can be monitored
              and require approval for sensitive operations.
            </p>
          </div>
          <TaskCreate />
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Tasks Header Component
// ============================================================================

interface TasksHeaderProps {
  totalTasks: number
}

function TasksHeader({ totalTasks }: TasksHeaderProps) {
  return (
    <div className="flex items-center justify-between p-4 border-b bg-muted/30">
      <div className="flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <ClipboardList className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-lg font-semibold">Tasks</h1>
          <p className="text-sm text-muted-foreground">
            {totalTasks === 0
              ? "No tasks"
              : `${totalTasks} task${totalTasks !== 1 ? "s" : ""}`}
          </p>
        </div>
      </div>
      <TaskCreate />
    </div>
  )
}

// ============================================================================
// Main Tasks Page Component
// ============================================================================

export function Tasks() {
  const loadTasks = useTaskStore((state) => state.loadTasks)
  const loadAgents = useAgentStore((state) => state.loadAgents)
  const { isLoaded } = useTasksLoading()
  const counts = useTaskCounts()

  const [selectedTask, setSelectedTask] = React.useState<Task | null>(null)
  const [detailOpen, setDetailOpen] = React.useState(false)

  // Load tasks and agents on mount
  React.useEffect(() => {
    loadTasks()
    loadAgents()
  }, [loadTasks, loadAgents])

  const handleViewDetails = (task: Task) => {
    setSelectedTask(task)
    setDetailOpen(true)
  }

  const handleDetailClose = (open: boolean) => {
    setDetailOpen(open)
    if (!open) {
      setSelectedTask(null)
    }
  }

  // Show empty state if no tasks
  if (isLoaded && counts.total === 0) {
    return (
      <div data-testid="tasks-panel" className="flex flex-col h-full bg-background">
        <TasksHeader totalTasks={0} />
        <TasksEmptyState />
        <TaskDetail task={selectedTask} open={detailOpen} onOpenChange={handleDetailClose} />
      </div>
    )
  }

  return (
    <div data-testid="tasks-panel" className="flex flex-col h-full bg-background">
      <TasksHeader totalTasks={counts.total} />
      <TaskList onViewDetails={handleViewDetails} className="flex-1" />
      <TaskDetail task={selectedTask} open={detailOpen} onOpenChange={handleDetailClose} />
    </div>
  )
}

export default Tasks
