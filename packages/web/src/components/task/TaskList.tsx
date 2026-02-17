/**
 * TaskList Component
 * Displays a filterable list of tasks
 */

import * as React from "react"
import { RefreshCw, Filter } from "lucide-react"
import { Button } from "@/components/ui/Button"
import { Badge } from "@/components/ui/Badge"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { TaskItem } from "./TaskItem"
import { useTasks, useTaskCounts, useTasksLoading, useTaskStore, type Task, type TaskStatus } from "@/stores/task"
import { cn } from "@/lib/utils"

// ============================================================================
// Props
// ============================================================================

interface TaskListProps {
  onViewDetails: (task: Task) => void
  className?: string
}

// ============================================================================
// Component
// ============================================================================

export function TaskList({ onViewDetails, className }: TaskListProps) {
  const tasks = useTasks()
  const counts = useTaskCounts()
  const { isLoading } = useTasksLoading()
  const loadTasks = useTaskStore((state) => state.loadTasks)

  const [filter, setFilter] = React.useState<TaskStatus | "all">("all")

  const filteredTasks = filter === "all" ? tasks : tasks.filter((t) => t.status === filter)

  const handleRefresh = () => {
    loadTasks()
  }

  return (
    <div className={cn("flex flex-col h-full", className)}>
      {/* Filter Bar */}
      <div className="flex items-center justify-between p-4 border-b bg-muted/30">
        <Tabs value={filter} onValueChange={(v) => setFilter(v as TaskStatus | "all")}>
          <TabsList>
            <TabsTrigger value="all" className="gap-1">
              All
              <Badge variant="secondary" className="ml-1 h-5 px-1.5">
                {counts.total}
              </Badge>
            </TabsTrigger>
            <TabsTrigger value="running" className="gap-1">
              Running
              {counts.running > 0 && (
                <Badge variant="default" className="ml-1 h-5 px-1.5">
                  {counts.running}
                </Badge>
              )}
            </TabsTrigger>
            <TabsTrigger value="awaiting_approval" className="gap-1">
              Approval
              {counts.awaitingApproval > 0 && (
                <Badge variant="outline" className="ml-1 h-5 px-1.5">
                  {counts.awaitingApproval}
                </Badge>
              )}
            </TabsTrigger>
            <TabsTrigger value="completed" className="gap-1">
              Completed
              {counts.completed > 0 && (
                <Badge variant="secondary" className="ml-1 h-5 px-1.5">
                  {counts.completed}
                </Badge>
              )}
            </TabsTrigger>
            <TabsTrigger value="failed" className="gap-1">
              Failed
              {counts.failed > 0 && (
                <Badge variant="destructive" className="ml-1 h-5 px-1.5">
                  {counts.failed}
                </Badge>
              )}
            </TabsTrigger>
          </TabsList>
        </Tabs>

        <Button
          variant="ghost"
          size="icon"
          onClick={handleRefresh}
          disabled={isLoading}
          title="Refresh tasks"
        >
          <RefreshCw className={cn("h-4 w-4", isLoading && "animate-spin")} />
        </Button>
      </div>

      {/* Task List */}
      <ScrollArea className="flex-1">
        <div data-testid="task-list" className="p-4 space-y-3">
          {filteredTasks.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <Filter className="h-12 w-12 text-muted-foreground mb-4" />
              <h3 className="text-lg font-medium mb-1">No tasks found</h3>
              <p className="text-sm text-muted-foreground">
                {filter === "all"
                  ? "Create a new task to get started."
                  : `No ${filter.replace("_", " ")} tasks found.`}
              </p>
            </div>
          ) : (
            filteredTasks.map((task) => (
              <TaskItem key={task.id} task={task} onViewDetails={onViewDetails} />
            ))
          )}
        </div>
      </ScrollArea>
    </div>
  )
}
