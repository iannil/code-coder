/**
 * Cron Panel Component
 *
 * Displays and manages scheduled tasks:
 * - Job list with status
 * - Create/delete jobs
 * - Manual execution
 * - Execution history
 */

import * as React from "react"
import {
  Clock,
  Play,
  Pause,
  Trash2,
  Plus,
  RefreshCw,
  CheckCircle,
  XCircle,
  History,
  ChevronRight,
} from "lucide-react"
import { Card, CardContent, CardHeader } from "@/components/ui/Card"
import { Button } from "@/components/ui/Button"
import { Badge } from "@/components/ui/Badge"
import { Skeleton } from "@/components/ui/Skeleton"
import { Input } from "@/components/ui/Input"
import { Label } from "@/components/ui/Label"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogFooter,
  DialogClose,
} from "@/components/ui/Dialog"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import {
  useCronStore,
  useCronJobs,
  useCronHistory,
  useCronLoading,
  useCronCounts,
} from "@/stores"
import { cn } from "@/lib/utils"
import type { CronJob, CronJobStatus } from "@/lib/types"

// ============================================================================
// Status Badge Component
// ============================================================================

function StatusBadge({ status }: { status?: CronJobStatus }) {
  if (!status) return null

  const variants: Record<
    CronJobStatus,
    { variant: "success" | "destructive" | "warning" | "outline"; icon: React.ReactNode }
  > = {
    success: { variant: "success", icon: <CheckCircle className="h-3 w-3" /> },
    failed: { variant: "destructive", icon: <XCircle className="h-3 w-3" /> },
    running: { variant: "warning", icon: <RefreshCw className="h-3 w-3 animate-spin" /> },
    pending: { variant: "outline", icon: <Clock className="h-3 w-3" /> },
  }

  const config = variants[status]

  return (
    <Badge variant={config.variant} className="gap-1 capitalize">
      {config.icon}
      {status}
    </Badge>
  )
}

// ============================================================================
// Job Card Component
// ============================================================================

interface JobCardProps {
  job: CronJob
  isRunning: boolean
  onToggle: () => void
  onRun: () => void
  onDelete: () => void
}

function JobCard({ job, isRunning, onToggle, onRun, onDelete }: JobCardProps) {
  const [expanded, setExpanded] = React.useState(false)

  const formatTime = (ts?: number) => {
    if (!ts) return "Never"
    return new Date(ts).toLocaleString()
  }

  return (
    <Card className={cn(!job.enabled && "opacity-75")}>
      <CardHeader
        className="cursor-pointer py-3"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div
              className={cn(
                "flex h-10 w-10 items-center justify-center rounded-lg",
                job.enabled ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground"
              )}
            >
              <Clock className="h-5 w-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h4 className="font-medium">{job.name}</h4>
                {job.lastStatus && <StatusBadge status={job.lastStatus} />}
              </div>
              <code className="text-xs text-muted-foreground">{job.expression}</code>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <ChevronRight
              className={cn("h-4 w-4 transition-transform", expanded && "rotate-90")}
            />
          </div>
        </div>
      </CardHeader>

      {expanded && (
        <CardContent className="pt-0 space-y-4">
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-muted-foreground">Command:</span>
              <code className="ml-2 px-2 py-0.5 bg-muted rounded">{job.command}</code>
            </div>
            <div>
              <span className="text-muted-foreground">Next run:</span>
              <span className="ml-2">{formatTime(job.nextRun)}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Last run:</span>
              <span className="ml-2">{formatTime(job.lastRun)}</span>
            </div>
            {job.lastError && (
              <div className="col-span-2">
                <span className="text-muted-foreground">Error:</span>
                <span className="ml-2 text-destructive">{job.lastError}</span>
              </div>
            )}
          </div>

          <div className="flex items-center gap-2 pt-2">
            <Button
              variant={job.enabled ? "outline" : "default"}
              size="sm"
              onClick={(e) => {
                e.stopPropagation()
                onToggle()
              }}
            >
              {job.enabled ? (
                <>
                  <Pause className="mr-2 h-3 w-3" />
                  Disable
                </>
              ) : (
                <>
                  <Play className="mr-2 h-3 w-3" />
                  Enable
                </>
              )}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={(e) => {
                e.stopPropagation()
                onRun()
              }}
              disabled={isRunning}
            >
              {isRunning ? (
                <RefreshCw className="mr-2 h-3 w-3 animate-spin" />
              ) : (
                <Play className="mr-2 h-3 w-3" />
              )}
              Run Now
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="text-destructive hover:text-destructive"
              onClick={(e) => {
                e.stopPropagation()
                onDelete()
              }}
            >
              <Trash2 className="h-3 w-3" />
            </Button>
          </div>
        </CardContent>
      )}
    </Card>
  )
}

// ============================================================================
// Create Job Dialog Component
// ============================================================================

function CreateJobDialog() {
  const [name, setName] = React.useState("")
  const [expression, setExpression] = React.useState("")
  const [command, setCommand] = React.useState("")
  const { isCreating } = useCronLoading()
  const createJob = useCronStore((s) => s.createJob)

  const handleCreate = async () => {
    await createJob({ name, expression, command, enabled: true })
    setName("")
    setExpression("")
    setCommand("")
  }

  const isValid = name.trim() && expression.trim() && command.trim()

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button>
          <Plus className="mr-2 h-4 w-4" />
          Create Job
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create Cron Job</DialogTitle>
          <DialogDescription>
            Schedule a new task to run automatically
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              placeholder="Daily Backup"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="expression">Cron Expression</Label>
            <Input
              id="expression"
              placeholder="0 2 * * *"
              value={expression}
              onChange={(e) => setExpression(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Format: minute hour day month weekday
            </p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="command">Command</Label>
            <Input
              id="command"
              placeholder="backup-data --all"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
            />
          </div>
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Cancel</Button>
          </DialogClose>
          <DialogClose asChild>
            <Button onClick={handleCreate} disabled={!isValid || isCreating}>
              {isCreating ? (
                <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Plus className="mr-2 h-4 w-4" />
              )}
              Create
            </Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// ============================================================================
// History List Component
// ============================================================================

function HistoryList() {
  const history = useCronHistory()
  const fetchHistory = useCronStore((s) => s.fetchHistory)

  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchHistory()
  }, [fetchHistory])

  if (history.length === 0) {
    return (
      <div className="text-center py-12">
        <History className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <p className="text-muted-foreground">No execution history</p>
      </div>
    )
  }

  return (
    <div className="space-y-2">
      {history.map((entry) => (
        <Card key={entry.id}>
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <StatusBadge status={entry.status} />
                <span className="font-medium">{entry.jobName}</span>
              </div>
              <div className="text-sm text-muted-foreground">
                {new Date(entry.startTime).toLocaleString()}
                {entry.endTime && (
                  <span className="ml-2">
                    ({Math.round((entry.endTime - entry.startTime) / 1000)}s)
                  </span>
                )}
              </div>
            </div>
            {entry.output && (
              <p className="mt-2 text-sm text-muted-foreground">{entry.output}</p>
            )}
            {entry.error && (
              <p className="mt-2 text-sm text-destructive">{entry.error}</p>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

// ============================================================================
// Summary Cards Component
// ============================================================================

function SummaryCards() {
  const counts = useCronCounts()

  return (
    <div className="grid grid-cols-3 gap-3">
      <Card>
        <CardContent className="p-4">
          <div className="text-2xl font-bold">{counts.total}</div>
          <div className="text-sm text-muted-foreground">Total Jobs</div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="text-2xl font-bold text-green-600">{counts.enabled}</div>
          <div className="text-sm text-muted-foreground">Enabled</div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="text-2xl font-bold text-red-600">{counts.failed}</div>
          <div className="text-sm text-muted-foreground">Failed</div>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Main Cron Panel Component
// ============================================================================

export function CronPanel() {
  const jobs = useCronJobs()
  const { isLoading, isRunning, error } = useCronLoading()
  const fetchJobs = useCronStore((s) => s.fetchJobs)
  const toggleJob = useCronStore((s) => s.toggleJob)
  const runJob = useCronStore((s) => s.runJob)
  const deleteJob = useCronStore((s) => s.deleteJob)

  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchJobs()
  }, [fetchJobs])

  if (isLoading) {
    return (
      <div className="space-y-4">
        <div className="grid grid-cols-3 gap-3">
          {[1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-20" />
          ))}
        </div>
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-24" />
        ))}
      </div>
    )
  }

  if (error) {
    return (
      <Card className="border-destructive">
        <CardContent className="p-6 text-center">
          <XCircle className="h-12 w-12 text-destructive mx-auto mb-4" />
          <p className="text-destructive font-medium">Failed to load cron jobs</p>
          <p className="text-sm text-muted-foreground mt-1">{error}</p>
          <Button variant="outline" className="mt-4" onClick={() => fetchJobs()}>
            <RefreshCw className="mr-2 h-4 w-4" />
            Retry
          </Button>
        </CardContent>
      </Card>
    )
  }

  return (
    <Tabs defaultValue="jobs" className="space-y-4">
      <div className="flex items-center justify-between">
        <TabsList>
          <TabsTrigger value="jobs" className="flex items-center gap-2">
            <Clock className="h-4 w-4" />
            Jobs
          </TabsTrigger>
          <TabsTrigger value="history" className="flex items-center gap-2">
            <History className="h-4 w-4" />
            History
          </TabsTrigger>
        </TabsList>
        <CreateJobDialog />
      </div>

      <TabsContent value="jobs" className="space-y-6">
        <SummaryCards />

        {jobs.length === 0 ? (
          <Card>
            <CardContent className="p-12 text-center">
              <Clock className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
              <p className="text-muted-foreground">No cron jobs configured</p>
              <p className="text-sm text-muted-foreground mt-1">
                Create a job to schedule automated tasks
              </p>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-3">
            {jobs.map((job) => (
              <JobCard
                key={job.id}
                job={job}
                isRunning={isRunning === job.id}
                onToggle={() => toggleJob(job.id)}
                onRun={() => runJob(job.id)}
                onDelete={() => deleteJob(job.id)}
              />
            ))}
          </div>
        )}
      </TabsContent>

      <TabsContent value="history">
        <HistoryList />
      </TabsContent>
    </Tabs>
  )
}
