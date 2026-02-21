/**
 * Workflows Page
 *
 * Workflow management with:
 * - Workflow list with status (from real API)
 * - Create/edit workflow definitions
 * - Trigger configuration (webhook, cron, manual)
 * - Execution history
 */

import * as React from "react"
import {
  Workflow,
  Search,
  Plus,
  Play,
  Clock,
  Webhook,
  Calendar,
  Hand,
  MoreVertical,
  Trash2,
  CheckCircle2,
  XCircle,
  Loader2,
  RefreshCw,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { Input } from "@/components/ui/Input"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Badge } from "@/components/ui/Badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { Separator } from "@/components/ui/Separator"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/Dialog"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/DropdownMenu"
import { Textarea } from "@/components/ui/Textarea"
import { Label } from "@/components/ui/Label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select"
import { useToast } from "@/hooks/use-toast"

// ============================================================================
// Types
// ============================================================================

interface WorkflowStep {
  name: string
  type: "agent" | "notify" | "shell" | "http"
  config?: Record<string, unknown>
}

interface WorkflowDef {
  name: string
  description?: string
  trigger_type: string
  steps_count: number
}

interface WorkflowExecution {
  execution_id: string
  workflow: string
  status: "running" | "success" | "failed" | "cancelled"
  started_at: number
  ended_at?: number
}

interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

// Zero-Workflow service base URL (configurable via env)
const WORKFLOW_API_BASE = import.meta.env.VITE_WORKFLOW_API_URL || "http://localhost:4405"

// ============================================================================
// API Functions
// ============================================================================

async function fetchWorkflows(): Promise<WorkflowDef[]> {
  const res = await fetch(`${WORKFLOW_API_BASE}/api/v1/workflows`)
  const data: ApiResponse<WorkflowDef[]> = await res.json()
  return data.success && data.data ? data.data : []
}

async function fetchExecutions(): Promise<WorkflowExecution[]> {
  const res = await fetch(`${WORKFLOW_API_BASE}/api/v1/executions`)
  const data: ApiResponse<WorkflowExecution[]> = await res.json()
  return data.success && data.data ? data.data : []
}

async function runWorkflow(name: string): Promise<boolean> {
  const res = await fetch(`${WORKFLOW_API_BASE}/api/v1/workflows/${name}/execute`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ context: {} }),
  })
  const data = await res.json()
  return data.success
}

async function deleteWorkflow(name: string): Promise<boolean> {
  const res = await fetch(`${WORKFLOW_API_BASE}/api/v1/workflows/${name}`, {
    method: "DELETE",
  })
  const data = await res.json()
  return data.success
}

async function createWorkflow(workflow: {
  name: string
  description?: string
  trigger: { type: string }
  steps: WorkflowStep[]
}): Promise<boolean> {
  const res = await fetch(`${WORKFLOW_API_BASE}/api/v1/workflows`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(workflow),
  })
  const data = await res.json()
  return data.success
}

// ============================================================================
// Helper Components
// ============================================================================

function TriggerIcon({ type }: { type: string }) {
  switch (type) {
    case "webhook":
      return <Webhook className="h-4 w-4" />
    case "cron":
      return <Calendar className="h-4 w-4" />
    case "manual":
    default:
      return <Hand className="h-4 w-4" />
  }
}

function StatusBadge({ status }: { status?: string }) {
  if (!status) return null
  const variants: Record<string, { variant: "default" | "secondary" | "destructive"; icon: React.ReactNode }> = {
    success: { variant: "secondary", icon: <CheckCircle2 className="h-3 w-3 text-green-500" /> },
    failed: { variant: "destructive", icon: <XCircle className="h-3 w-3" /> },
    running: { variant: "default", icon: <Loader2 className="h-3 w-3 animate-spin" /> },
    cancelled: { variant: "secondary", icon: <XCircle className="h-3 w-3" /> },
  }
  const { variant, icon } = variants[status] || { variant: "secondary", icon: null }
  return (
    <Badge variant={variant} className="gap-1">
      {icon}
      {status}
    </Badge>
  )
}

function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleString()
}

// ============================================================================
// Workflow Card Component
// ============================================================================

interface WorkflowCardProps {
  workflow: WorkflowDef
  onRun: () => void
  onDelete: () => void
  onViewDetails: () => void
}

function WorkflowCard({
  workflow,
  onRun,
  onDelete,
  onViewDetails,
}: WorkflowCardProps) {
  return (
    <Card
      className="cursor-pointer hover:border-primary/50 transition-colors"
      onClick={onViewDetails}
    >
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <Workflow className="h-5 w-5 text-primary" />
            </div>
            <div>
              <CardTitle className="text-base flex items-center gap-2">
                {workflow.name}
              </CardTitle>
              <div className="flex items-center gap-2 text-xs text-muted-foreground mt-1">
                <TriggerIcon type={workflow.trigger_type} />
                <span className="capitalize">{workflow.trigger_type}</span>
                <span>•</span>
                <span>{workflow.steps_count} steps</span>
              </div>
            </div>
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <MoreVertical className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={(e) => {
                  e.stopPropagation()
                  onRun()
                }}
              >
                <Play className="h-4 w-4 mr-2" />
                Run now
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                className="text-destructive"
                onClick={(e) => {
                  e.stopPropagation()
                  onDelete()
                }}
              >
                <Trash2 className="h-4 w-4 mr-2" />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </CardHeader>
      <CardContent>
        {workflow.description && (
          <CardDescription className="line-clamp-2">{workflow.description}</CardDescription>
        )}
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Workflows Page Component
// ============================================================================

export function Workflows() {
  const { toast } = useToast()

  const [workflows, setWorkflows] = React.useState<WorkflowDef[]>([])
  const [executions, setExecutions] = React.useState<WorkflowExecution[]>([])
  const [searchQuery, setSearchQuery] = React.useState("")
  const [activeTab, setActiveTab] = React.useState("workflows")
  const [selectedWorkflow, setSelectedWorkflow] = React.useState<WorkflowDef | null>(null)
  const [isCreateOpen, setIsCreateOpen] = React.useState(false)
  const [isLoading, setIsLoading] = React.useState(true)

  // Form state for create dialog
  const [newName, setNewName] = React.useState("")
  const [newDescription, setNewDescription] = React.useState("")
  const [newTrigger, setNewTrigger] = React.useState("manual")
  const [newStepsYaml, setNewStepsYaml] = React.useState("")

  // Fetch data
  const loadData = React.useCallback(async () => {
    setIsLoading(true)
    try {
      const [wfs, execs] = await Promise.all([fetchWorkflows(), fetchExecutions()])
      setWorkflows(wfs)
      setExecutions(execs)
    } catch (err) {
      toast({
        title: "Failed to load workflows",
        description: "Could not connect to workflow service.",
        variant: "destructive",
      })
    } finally {
      setIsLoading(false)
    }
  }, [toast])

  React.useEffect(() => {
    loadData()
  }, [loadData])

  const filteredWorkflows = React.useMemo(() => {
    if (!searchQuery.trim()) return workflows
    const query = searchQuery.toLowerCase()
    return workflows.filter(
      (w) =>
        w.name.toLowerCase().includes(query) ||
        w.description?.toLowerCase().includes(query)
    )
  }, [workflows, searchQuery])

  const handleRun = async (name: string) => {
    const success = await runWorkflow(name)
    if (success) {
      toast({
        title: "Workflow triggered",
        description: `${name} is now running.`,
      })
      loadData()
    } else {
      toast({
        title: "Failed to run workflow",
        variant: "destructive",
      })
    }
  }

  const handleDelete = async (name: string) => {
    const success = await deleteWorkflow(name)
    if (success) {
      setWorkflows((prev) => prev.filter((w) => w.name !== name))
      toast({
        title: "Workflow deleted",
        description: `${name} has been deleted.`,
      })
    } else {
      toast({
        title: "Failed to delete workflow",
        variant: "destructive",
      })
    }
  }

  const handleCreate = async () => {
    if (!newName.trim()) {
      toast({ title: "Name is required", variant: "destructive" })
      return
    }

    // Parse steps from YAML (simplified)
    let steps: WorkflowStep[] = []
    try {
      // Simple YAML parsing - in production use a proper YAML parser
      const lines = newStepsYaml.split("\n")
      let currentStep: Partial<WorkflowStep> = {}
      for (const line of lines) {
        const trimmed = line.trim()
        if (trimmed.startsWith("- name:")) {
          if (currentStep.name) steps.push(currentStep as WorkflowStep)
          currentStep = { name: trimmed.replace("- name:", "").trim() }
        } else if (trimmed.startsWith("type:")) {
          currentStep.type = trimmed.replace("type:", "").trim() as WorkflowStep["type"]
        }
      }
      if (currentStep.name) steps.push(currentStep as WorkflowStep)
    } catch {
      steps = [{ name: "step1", type: "shell" }]
    }

    const success = await createWorkflow({
      name: newName,
      description: newDescription || undefined,
      trigger: { type: newTrigger },
      steps,
    })

    if (success) {
      toast({
        title: "Workflow created",
        description: `${newName} has been created.`,
      })
      setIsCreateOpen(false)
      setNewName("")
      setNewDescription("")
      setNewTrigger("manual")
      setNewStepsYaml("")
      loadData()
    } else {
      toast({
        title: "Failed to create workflow",
        variant: "destructive",
      })
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex flex-col gap-4 p-4 border-b bg-muted/30">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <Workflow className="h-5 w-5 text-primary" />
            </div>
            <div>
              <h1 className="text-lg font-semibold">Workflows</h1>
              <p className="text-sm text-muted-foreground">
                {workflows.length} workflow{workflows.length !== 1 ? "s" : ""}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={loadData} disabled={isLoading}>
              {isLoading ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <RefreshCw className="h-4 w-4" />
              )}
            </Button>
            <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Plus className="h-4 w-4 mr-2" />
                  New Workflow
                </Button>
              </DialogTrigger>
              <DialogContent className="max-w-2xl">
                <DialogHeader>
                  <DialogTitle>Create Workflow</DialogTitle>
                  <DialogDescription>
                    Define a new automated workflow with triggers and steps.
                  </DialogDescription>
                </DialogHeader>
                <div className="space-y-4 py-4">
                  <div className="space-y-2">
                    <Label>Name</Label>
                    <Input
                      placeholder="my-workflow"
                      value={newName}
                      onChange={(e) => setNewName(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label>Description</Label>
                    <Textarea
                      placeholder="What does this workflow do?"
                      value={newDescription}
                      onChange={(e) => setNewDescription(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label>Trigger</Label>
                    <Select value={newTrigger} onValueChange={setNewTrigger}>
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="webhook">Webhook</SelectItem>
                        <SelectItem value="cron">Cron Schedule</SelectItem>
                        <SelectItem value="manual">Manual</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="space-y-2">
                    <Label>Steps (YAML)</Label>
                    <Textarea
                      className="font-mono text-sm"
                      placeholder="- name: step1&#10;  type: shell"
                      rows={8}
                      value={newStepsYaml}
                      onChange={(e) => setNewStepsYaml(e.target.value)}
                    />
                  </div>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => setIsCreateOpen(false)}>
                    Cancel
                  </Button>
                  <Button onClick={handleCreate}>Create</Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>
        </div>

        <div className="relative max-w-md">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            type="text"
            placeholder="Search workflows..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-10"
          />
        </div>
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
        <div className="border-b px-4">
          <TabsList className="h-12">
            <TabsTrigger value="workflows">Workflows</TabsTrigger>
            <TabsTrigger value="history">History ({executions.length})</TabsTrigger>
          </TabsList>
        </div>

        <TabsContent value="workflows" className="flex-1 m-0">
          <ScrollArea className="h-full p-4">
            {isLoading && workflows.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-64">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                <p className="text-sm text-muted-foreground mt-2">Loading workflows...</p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 md:grid-cols-2">
                {filteredWorkflows.map((workflow) => (
                  <WorkflowCard
                    key={workflow.name}
                    workflow={workflow}
                    onRun={() => handleRun(workflow.name)}
                    onDelete={() => handleDelete(workflow.name)}
                    onViewDetails={() => setSelectedWorkflow(workflow)}
                  />
                ))}
              </div>
            )}
            {!isLoading && filteredWorkflows.length === 0 && (
              <div className="flex flex-col items-center justify-center h-64 text-center">
                <Workflow className="h-12 w-12 text-muted-foreground/50 mb-4" />
                <h3 className="text-lg font-semibold">No workflows found</h3>
                <p className="text-sm text-muted-foreground">
                  Create your first workflow to automate tasks.
                </p>
              </div>
            )}
          </ScrollArea>
        </TabsContent>

        <TabsContent value="history" className="flex-1 m-0">
          <ScrollArea className="h-full p-4">
            <div className="space-y-4">
              {executions.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-64 text-center">
                  <Clock className="h-12 w-12 text-muted-foreground/50 mb-4" />
                  <h3 className="text-lg font-semibold">No executions yet</h3>
                  <p className="text-sm text-muted-foreground">
                    Run a workflow to see execution history.
                  </p>
                </div>
              ) : (
                executions.map((exec) => (
                  <Card key={exec.execution_id}>
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <StatusBadge status={exec.status} />
                          <span className="font-medium">{exec.workflow}</span>
                        </div>
                        <span className="text-sm text-muted-foreground">
                          {formatTimestamp(exec.started_at)}
                        </span>
                      </div>
                    </CardHeader>
                    <CardContent>
                      <div className="text-xs text-muted-foreground">
                        ID: {exec.execution_id}
                        {exec.ended_at && (
                          <span className="ml-4">
                            Duration: {Math.round((exec.ended_at - exec.started_at) / 1000)}s
                          </span>
                        )}
                      </div>
                    </CardContent>
                  </Card>
                ))
              )}
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>

      {/* Workflow Details Dialog */}
      <Dialog open={!!selectedWorkflow} onOpenChange={() => setSelectedWorkflow(null)}>
        <DialogContent className="max-w-2xl">
          {selectedWorkflow && (
            <>
              <DialogHeader>
                <div className="flex items-center gap-3">
                  <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10">
                    <Workflow className="h-6 w-6 text-primary" />
                  </div>
                  <div>
                    <DialogTitle className="text-xl">{selectedWorkflow.name}</DialogTitle>
                    <DialogDescription className="flex items-center gap-2">
                      <TriggerIcon type={selectedWorkflow.trigger_type} />
                      <span className="capitalize">{selectedWorkflow.trigger_type}</span>
                      <span>•</span>
                      <span>{selectedWorkflow.steps_count} steps</span>
                    </DialogDescription>
                  </div>
                </div>
              </DialogHeader>
              <div className="space-y-4">
                {selectedWorkflow.description && (
                  <p className="text-sm text-muted-foreground">{selectedWorkflow.description}</p>
                )}
                <Separator />
                <div className="flex justify-end gap-2">
                  <Button
                    variant="destructive"
                    onClick={() => {
                      handleDelete(selectedWorkflow.name)
                      setSelectedWorkflow(null)
                    }}
                  >
                    <Trash2 className="h-4 w-4 mr-2" />
                    Delete
                  </Button>
                  <Button onClick={() => handleRun(selectedWorkflow.name)}>
                    <Play className="h-4 w-4 mr-2" />
                    Run Now
                  </Button>
                </div>
              </div>
            </>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}

export default Workflows
