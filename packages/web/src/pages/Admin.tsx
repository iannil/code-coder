/**
 * Admin Page
 *
 * Administrative dashboard with:
 * - User management
 * - Role and permission configuration
 * - Token quota and usage tracking
 * - System statistics
 * - Executive dashboard (Phase 3)
 */

import * as React from "react"
import {
  Users,
  Shield,
  Gauge,
  Search,
  Plus,
  MoreVertical,
  Edit,
  Trash2,
  Key,
  UserPlus,
  CheckCircle2,
  Ban,
  Crown,
  User,
  TrendingUp,
  TrendingDown,
  Clock,
  Loader2,
  RefreshCw,
  AlertCircle,
  BarChart3,
  DollarSign,
  GitCommit,
  UsersRound,
  AlertTriangle,
  Info,
  Wallet,
  ShieldAlert,
  Bell,
  BellOff,
  Eye,
  EyeOff,
  FileWarning,
  Lock,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { Input } from "@/components/ui/Input"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Badge } from "@/components/ui/Badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/Table"
import { Label } from "@/components/ui/Label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select"
import { useToast } from "@/hooks/use-toast"
import { api } from "@/lib/api"
import type {
  MeteringUsageResponse,
  MeteringUserReport,
  ExecutiveTrendsResponse,
  ExecutiveTeamsResponse,
  ExecutiveActivityResponse,
  ExecutiveSummary,
  BudgetAlert,
  BudgetSummary,
  BudgetThreshold,
  DlpRule,
  DlpSummary,
  DlpIncident,
} from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface AdminUser {
  id: string
  name: string
  email?: string
  role: "admin" | "developer" | "viewer"
  status: "active" | "inactive" | "suspended"
  tokenUsage: number
  tokenLimit: number
  lastActive?: string
  createdAt?: string
}

interface AdminRole {
  id: string
  name: string
  description: string
  permissions: string[]
  userCount: number
}

interface UsageMetric {
  label: string
  value: number
  change: number
  trend: "up" | "down" | "neutral"
}

// ============================================================================
// Mock Roles Data (roles are typically static configuration)
// ============================================================================

const MOCK_ROLES: AdminRole[] = [
  {
    id: "role-admin",
    name: "Admin",
    description: "Full system access with user management capabilities",
    permissions: ["*:*:*"],
    userCount: 1,
  },
  {
    id: "role-developer",
    name: "Developer",
    description: "Access to all agents, skills, and workflows",
    permissions: ["agent:*:execute", "skill:*:read", "workflow:*:execute"],
    userCount: 3,
  },
  {
    id: "role-viewer",
    name: "Viewer",
    description: "Read-only access to dashboards and reports",
    permissions: ["agent:*:read", "skill:*:read", "workflow:*:read"],
    userCount: 1,
  },
]

// ============================================================================
// Helper Components
// ============================================================================

function RoleBadge({ role }: { role: AdminUser["role"] }) {
  const variants: Record<AdminUser["role"], { icon: React.ReactNode; className: string }> = {
    admin: { icon: <Crown className="h-3 w-3" />, className: "bg-yellow-500/10 text-yellow-600" },
    developer: { icon: <User className="h-3 w-3" />, className: "bg-blue-500/10 text-blue-600" },
    viewer: { icon: <User className="h-3 w-3" />, className: "bg-gray-500/10 text-gray-600" },
  }
  const { icon, className } = variants[role]
  return (
    <Badge variant="outline" className={`gap-1 ${className}`}>
      {icon}
      {role}
    </Badge>
  )
}

function StatusBadge({ status }: { status: AdminUser["status"] }) {
  const variants: Record<AdminUser["status"], { icon: React.ReactNode; className: string }> = {
    active: {
      icon: <CheckCircle2 className="h-3 w-3" />,
      className: "bg-green-500/10 text-green-600",
    },
    inactive: {
      icon: <Clock className="h-3 w-3" />,
      className: "bg-yellow-500/10 text-yellow-600",
    },
    suspended: {
      icon: <Ban className="h-3 w-3" />,
      className: "bg-red-500/10 text-red-600",
    },
  }
  const { icon, className } = variants[status]
  return (
    <Badge variant="outline" className={`gap-1 ${className}`}>
      {icon}
      {status}
    </Badge>
  )
}

function MetricCard({ metric, isLoading }: { metric: UsageMetric; isLoading?: boolean }) {
  const formatValue = (value: number) => {
    if (value >= 1000000) return `${(value / 1000000).toFixed(1)}M`
    if (value >= 1000) return `${(value / 1000).toFixed(1)}K`
    return value.toString()
  }

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardDescription>{metric.label}</CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex items-center gap-2">
            <Loader2 className="h-4 w-4 animate-spin" />
            <span className="text-sm text-muted-foreground">Loading...</span>
          </div>
        ) : (
          <div className="flex items-end justify-between">
            <div className="text-2xl font-bold">{formatValue(metric.value)}</div>
            <div
              className={`flex items-center gap-1 text-sm ${
                metric.trend === "up"
                  ? "text-green-600"
                  : metric.trend === "down"
                    ? "text-red-600"
                    : "text-muted-foreground"
              }`}
            >
              <TrendingUp
                className={`h-4 w-4 ${metric.trend === "down" ? "rotate-180" : ""}`}
              />
              {metric.change}%
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

function UsageBar({ used, limit }: { used: number; limit: number }) {
  const percentage = Math.min((used / limit) * 100, 100)
  const isWarning = percentage > 80
  const isCritical = percentage > 95

  return (
    <div className="space-y-1">
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>{(used / 1000).toFixed(0)}K / {(limit / 1000).toFixed(0)}K tokens</span>
        <span>{percentage.toFixed(0)}%</span>
      </div>
      <div className="h-2 bg-muted rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full transition-all ${
            isCritical
              ? "bg-red-500"
              : isWarning
                ? "bg-yellow-500"
                : "bg-primary"
          }`}
          style={{ width: `${percentage}%` }}
        />
      </div>
    </div>
  )
}

function LoadingState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center p-8">
      <div className="flex flex-col items-center gap-4">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        <p className="text-sm text-muted-foreground">{message}</p>
      </div>
    </div>
  )
}

// ============================================================================
// Data Transformation
// ============================================================================

function transformMeteringToUsers(reports: MeteringUserReport[]): AdminUser[] {
  return reports.map((report) => ({
    id: report.user_id,
    name: report.name,
    email: report.email,
    role: (report.role as AdminUser["role"]) || "developer",
    status: "active" as const,
    tokenUsage: report.daily_usage.input_tokens + report.daily_usage.output_tokens,
    tokenLimit: report.quota.daily_input_limit + report.quota.daily_output_limit,
    lastActive: report.last_active,
  }))
}

function transformUsageToMetrics(usage: MeteringUsageResponse): UsageMetric[] {
  return [
    { label: "Total Users", value: usage.total_users, change: 0, trend: "neutral" as const },
    { label: "Active Users (24h)", value: usage.active_users_24h, change: 0, trend: "neutral" as const },
    { label: "Tokens Used (24h)", value: usage.tokens_used_24h, change: 0, trend: "up" as const },
    { label: "Requests (24h)", value: usage.requests_24h, change: 0, trend: "up" as const },
  ]
}

// ============================================================================
// Admin Page Component
// ============================================================================

export function Admin() {
  const { toast } = useToast()

  // State
  const [users, setUsers] = React.useState<AdminUser[]>([])
  const [metrics, setMetrics] = React.useState<UsageMetric[]>([])
  const [searchQuery, setSearchQuery] = React.useState("")
  const [activeTab, setActiveTab] = React.useState("users")
  const [isAddUserOpen, setIsAddUserOpen] = React.useState(false)
  const [isLoading, setIsLoading] = React.useState(true)
  const [error, setError] = React.useState<string | null>(null)

  // Executive Dashboard State
  const [executivePeriod, setExecutivePeriod] = React.useState<"daily" | "weekly" | "monthly">("weekly")
  const [executiveSummary, setExecutiveSummary] = React.useState<ExecutiveSummary | null>(null)
  const [executiveTrends, setExecutiveTrends] = React.useState<ExecutiveTrendsResponse | null>(null)
  const [executiveTeams, setExecutiveTeams] = React.useState<ExecutiveTeamsResponse | null>(null)
  const [executiveActivity, setExecutiveActivity] = React.useState<ExecutiveActivityResponse | null>(null)
  const [isExecutiveLoading, setIsExecutiveLoading] = React.useState(false)

  // Budget State
  const [budgetSummary, setBudgetSummary] = React.useState<BudgetSummary | null>(null)
  const [budgetAlerts, setBudgetAlerts] = React.useState<BudgetAlert[]>([])
  const [isBudgetLoading, setIsBudgetLoading] = React.useState(false)
  const [isAddBudgetOpen, setIsAddBudgetOpen] = React.useState(false)
  const [newBudgetName, setNewBudgetName] = React.useState("")
  const [newBudgetAmount, setNewBudgetAmount] = React.useState("1000")
  const [newBudgetPeriod, setNewBudgetPeriod] = React.useState<"daily" | "weekly" | "monthly">("monthly")

  // WebSocket State for Real-time Updates
  const [wsConnected, setWsConnected] = React.useState(false)
  const wsRef = React.useRef<WebSocket | null>(null)

  // WebSocket connection for executive dashboard real-time updates
  React.useEffect(() => {
    if (activeTab !== "executive") {
      // Disconnect WebSocket when leaving executive tab
      if (wsRef.current) {
        wsRef.current.close()
        wsRef.current = null
        setWsConnected(false)
      }
      return
    }

    // Connect to WebSocket
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:"
    const wsUrl = `${protocol}//${window.location.hostname}:4400/api/v1/executive/ws`

    try {
      const ws = new WebSocket(wsUrl)
      wsRef.current = ws

      ws.onopen = () => {
        setWsConnected(true)
        // Subscribe to all executive channels
        ws.send(JSON.stringify({
          type: "subscribe",
          channels: ["executive.metrics", "executive.alerts", "executive.activity", "executive.cost"],
        }))
      }

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data)

          // Handle different channel updates
          if (data.channel === "executive.metrics" && data.data) {
            // Update summary with new metrics
            setExecutiveSummary((prev) => prev ? { ...prev, ...data.data } : prev)
          } else if (data.channel === "executive.alerts" && data.data) {
            // Show toast for alerts
            toast({
              title: data.data.type === "warning" ? "Warning" : "Alert",
              description: data.data.message,
              variant: data.data.type === "warning" ? "destructive" : "default",
            })
          } else if (data.channel === "executive.activity" && data.data) {
            // Update activity data
            setExecutiveActivity((prev) => prev ? { ...prev, ...data.data } : prev)
          }
        } catch {
          // Ignore parse errors
        }
      }

      ws.onclose = () => {
        setWsConnected(false)
        wsRef.current = null
      }

      ws.onerror = () => {
        setWsConnected(false)
      }

      return () => {
        ws.close()
        wsRef.current = null
      }
    } catch {
      // WebSocket not supported or connection failed
      setWsConnected(false)
    }
  }, [activeTab, toast])

  // DLP State
  const [dlpSummary, setDlpSummary] = React.useState<DlpSummary | null>(null)
  const [dlpRules, setDlpRules] = React.useState<DlpRule[]>([])
  const [dlpIncidents, setDlpIncidents] = React.useState<DlpIncident[]>([])
  const [isDlpLoading, setIsDlpLoading] = React.useState(false)
  const [isAddRuleOpen, setIsAddRuleOpen] = React.useState(false)
  const [newRuleName, setNewRuleName] = React.useState("")
  const [newRulePattern, setNewRulePattern] = React.useState("")
  const [newRuleAction, setNewRuleAction] = React.useState<"block" | "redact" | "warn" | "log">("warn")

  // Fetch data from API
  const fetchData = React.useCallback(async () => {
    setIsLoading(true)
    setError(null)

    try {
      const [usageResponse, usersResponse] = await Promise.all([
        api.getMeteringUsage(),
        api.getMeteringUsers(),
      ])

      setMetrics(transformUsageToMetrics(usageResponse))
      setUsers(transformMeteringToUsers(usersResponse))
    } catch (err) {
      console.error("Failed to fetch metering data:", err)
      setError(err instanceof Error ? err.message : "Failed to fetch data")
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Initial data fetch
  React.useEffect(() => {
    fetchData()
  }, [fetchData])

  // Fetch executive data
  const fetchExecutiveData = React.useCallback(async () => {
    setIsExecutiveLoading(true)
    try {
      const [summary, trends, teams, activity] = await Promise.all([
        api.getExecutiveSummary(executivePeriod),
        api.getExecutiveTrends(executivePeriod),
        api.getExecutiveTeams(),
        api.getExecutiveActivity(),
      ])
      setExecutiveSummary(summary)
      setExecutiveTrends(trends)
      setExecutiveTeams(teams)
      setExecutiveActivity(activity)
    } catch (err) {
      console.error("Failed to fetch executive data:", err)
      toast({
        title: "Failed to load executive data",
        description: err instanceof Error ? err.message : "An error occurred",
        variant: "destructive",
      })
    } finally {
      setIsExecutiveLoading(false)
    }
  }, [executivePeriod, toast])

  // Fetch executive data when tab changes to executive or period changes
  React.useEffect(() => {
    if (activeTab === "executive") {
      fetchExecutiveData()
    }
  }, [activeTab, fetchExecutiveData])

  // Fetch budget data
  const fetchBudgetData = React.useCallback(async () => {
    setIsBudgetLoading(true)
    try {
      const [summary, alerts] = await Promise.all([
        api.getBudgetSummary(),
        api.listBudgetAlerts(false),
      ])
      setBudgetSummary(summary)
      setBudgetAlerts(alerts)
    } catch (err) {
      console.error("Failed to fetch budget data:", err)
      // Use mock data for demo
      setBudgetSummary({
        total_budget_usd: 5000,
        total_spend_usd: 3250,
        percentage_used: 65,
        period: "monthly",
        active_alerts: 1,
        budgets: [
          { id: "1", name: "Engineering", budget_usd: 2000, spend_usd: 1500, percentage: 75, status: "warning" },
          { id: "2", name: "Product", budget_usd: 1500, spend_usd: 950, percentage: 63, status: "ok" },
          { id: "3", name: "Research", budget_usd: 1500, spend_usd: 800, percentage: 53, status: "ok" },
        ],
      })
      setBudgetAlerts([
        {
          id: "alert-1",
          budget_id: "1",
          budget_name: "Engineering",
          severity: "warning",
          message: "Engineering budget has exceeded 70% threshold",
          threshold_percentage: 70,
          current_percentage: 75,
          current_spend_usd: 1500,
          budget_usd: 2000,
          triggered_at: new Date().toISOString(),
          acknowledged: false,
        },
      ])
    } finally {
      setIsBudgetLoading(false)
    }
  }, [])

  // Fetch budget data when tab changes
  React.useEffect(() => {
    if (activeTab === "budgets") {
      fetchBudgetData()
    }
  }, [activeTab, fetchBudgetData])

  // Fetch DLP data
  const fetchDlpData = React.useCallback(async () => {
    setIsDlpLoading(true)
    try {
      const [summary, rules, incidents] = await Promise.all([
        api.getDlpSummary(),
        api.listDlpRules(),
        api.listDlpIncidents(20),
      ])
      setDlpSummary(summary)
      setDlpRules(rules)
      setDlpIncidents(incidents)
    } catch (err) {
      console.error("Failed to fetch DLP data:", err)
      // Use mock data for demo
      setDlpSummary({
        total_rules: 12,
        active_rules: 10,
        incidents_24h: 3,
        incidents_7d: 15,
        top_triggered_rules: [
          { rule_id: "1", rule_name: "API Keys", count: 8 },
          { rule_id: "2", rule_name: "Credit Cards", count: 4 },
          { rule_id: "3", rule_name: "AWS Secrets", count: 3 },
        ],
      })
      setDlpRules([
        {
          id: "1",
          name: "API Keys",
          description: "Detect API keys in format sk-xxx",
          type: "regex",
          pattern: "sk[-_][a-zA-Z0-9]{20,}",
          action: "redact",
          enabled: true,
          categories: ["credentials"],
          replacement: "[REDACTED_API_KEY]",
          priority: 1,
          match_count: 245,
          created_at: "2024-01-01",
          updated_at: "2024-02-01",
        },
        {
          id: "2",
          name: "Credit Cards",
          description: "Detect credit card numbers",
          type: "regex",
          pattern: "\\b(?:\\d{4}[- ]?){3}\\d{4}\\b",
          action: "block",
          enabled: true,
          categories: ["pii", "financial"],
          replacement: "[REDACTED_CC]",
          priority: 2,
          match_count: 42,
          created_at: "2024-01-01",
          updated_at: "2024-02-01",
        },
        {
          id: "3",
          name: "AWS Access Keys",
          description: "Detect AWS access key IDs",
          type: "regex",
          pattern: "AKIA[0-9A-Z]{16}",
          action: "block",
          enabled: true,
          categories: ["credentials", "cloud"],
          replacement: "[REDACTED_AWS_KEY]",
          priority: 1,
          match_count: 18,
          created_at: "2024-01-01",
          updated_at: "2024-02-01",
        },
      ])
      setDlpIncidents([
        {
          id: "inc-1",
          rule_id: "1",
          rule_name: "API Keys",
          action_taken: "redact",
          content_preview: "Using API key: sk-[REDACTED]...",
          user_id: "user-1",
          triggered_at: new Date(Date.now() - 1000 * 60 * 30).toISOString(),
        },
        {
          id: "inc-2",
          rule_id: "2",
          rule_name: "Credit Cards",
          action_taken: "block",
          content_preview: "Payment with card: 4111-****-****-1111",
          user_id: "user-2",
          triggered_at: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
        },
      ])
    } finally {
      setIsDlpLoading(false)
    }
  }, [])

  // Fetch DLP data when tab changes
  React.useEffect(() => {
    if (activeTab === "dlp") {
      fetchDlpData()
    }
  }, [activeTab, fetchDlpData])

  // Handle budget actions
  const handleCreateBudget = async () => {
    if (!newBudgetName.trim() || !newBudgetAmount) return

    try {
      const thresholds: BudgetThreshold[] = [
        { percentage: 70, severity: "warning", notify: true },
        { percentage: 90, severity: "critical", notify: true },
      ]

      await api.createBudget({
        name: newBudgetName,
        period: newBudgetPeriod,
        budget_usd: parseFloat(newBudgetAmount),
        thresholds,
        enabled: true,
      })

      toast({ title: "Budget created", description: `${newBudgetName} budget has been created.` })
      setIsAddBudgetOpen(false)
      setNewBudgetName("")
      setNewBudgetAmount("1000")
      fetchBudgetData()
    } catch (err) {
      toast({ title: "Failed to create budget", variant: "destructive" })
    }
  }

  const handleAcknowledgeAlert = async (alertId: string) => {
    try {
      await api.acknowledgeBudgetAlert(alertId)
      setBudgetAlerts((prev) => prev.filter((a) => a.id !== alertId))
      toast({ title: "Alert acknowledged" })
    } catch (err) {
      // For demo, just remove from state
      setBudgetAlerts((prev) => prev.filter((a) => a.id !== alertId))
      toast({ title: "Alert acknowledged" })
    }
  }

  // Handle DLP actions
  const handleCreateRule = async () => {
    if (!newRuleName.trim() || !newRulePattern.trim()) return

    try {
      await api.createDlpRule({
        name: newRuleName,
        type: "regex",
        pattern: newRulePattern,
        action: newRuleAction,
        enabled: true,
        categories: [],
        priority: 10,
      })

      toast({ title: "Rule created", description: `${newRuleName} rule has been created.` })
      setIsAddRuleOpen(false)
      setNewRuleName("")
      setNewRulePattern("")
      fetchDlpData()
    } catch (err) {
      toast({ title: "Failed to create rule", variant: "destructive" })
    }
  }

  const handleToggleRule = async (ruleId: string, enabled: boolean) => {
    try {
      await api.updateDlpRule(ruleId, { enabled })
      setDlpRules((prev) =>
        prev.map((r) => (r.id === ruleId ? { ...r, enabled } : r))
      )
      toast({ title: enabled ? "Rule enabled" : "Rule disabled" })
    } catch (err) {
      // For demo, just update state
      setDlpRules((prev) =>
        prev.map((r) => (r.id === ruleId ? { ...r, enabled } : r))
      )
      toast({ title: enabled ? "Rule enabled" : "Rule disabled" })
    }
  }

  // Filter users based on search
  const filteredUsers = React.useMemo(() => {
    if (!searchQuery.trim()) return users
    const query = searchQuery.toLowerCase()
    return users.filter(
      (u) =>
        u.name.toLowerCase().includes(query) ||
        (u.email?.toLowerCase().includes(query)) ||
        u.role.toLowerCase().includes(query)
    )
  }, [users, searchQuery])

  const handleSuspendUser = (id: string) => {
    setUsers((prev) =>
      prev.map((u) =>
        u.id === id
          ? { ...u, status: u.status === "suspended" ? ("active" as const) : ("suspended" as const) }
          : u
      )
    )
    const user = users.find((u) => u.id === id)
    toast({
      title: user?.status === "suspended" ? "User activated" : "User suspended",
      description: `${user?.name} has been ${user?.status === "suspended" ? "activated" : "suspended"}.`,
    })
  }

  const handleDeleteUser = (id: string) => {
    const user = users.find((u) => u.id === id)
    setUsers((prev) => prev.filter((u) => u.id !== id))
    toast({
      title: "User deleted",
      description: `${user?.name} has been removed.`,
    })
  }

  const activeUsers = users.filter((u) => u.status === "active").length
  const totalTokenUsage = users.reduce((sum, u) => sum + u.tokenUsage, 0)

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex flex-col gap-4 p-4 border-b bg-muted/30">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <Shield className="h-5 w-5 text-primary" />
            </div>
            <div>
              <h1 className="text-lg font-semibold">Admin</h1>
              <p className="text-sm text-muted-foreground">
                {users.length} users • {activeUsers} active
              </p>
            </div>
          </div>
          <Button variant="outline" size="sm" onClick={fetchData} disabled={isLoading}>
            <RefreshCw className={`h-4 w-4 mr-2 ${isLoading ? "animate-spin" : ""}`} />
            Refresh
          </Button>
        </div>
      </div>

      {/* Metrics */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 p-4">
        {isLoading && metrics.length === 0 ? (
          <>
            {[1, 2, 3, 4].map((i) => (
              <MetricCard
                key={i}
                metric={{ label: "Loading...", value: 0, change: 0, trend: "neutral" }}
                isLoading
              />
            ))}
          </>
        ) : (
          metrics.map((metric) => (
            <MetricCard key={metric.label} metric={metric} />
          ))
        )}
      </div>

      {/* Error State */}
      {error && (
        <div className="px-4">
          <Card className="border-destructive">
            <CardContent className="py-4">
              <div className="flex items-center gap-2 text-destructive">
                <AlertCircle className="h-4 w-4" />
                <span>{error}</span>
                <Button variant="ghost" size="sm" className="ml-auto" onClick={fetchData}>
                  Retry
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
        <div className="border-b px-4">
          <TabsList className="h-12">
            <TabsTrigger value="users" className="gap-2">
              <Users className="h-4 w-4" />
              Users
            </TabsTrigger>
            <TabsTrigger value="roles" className="gap-2">
              <Shield className="h-4 w-4" />
              Roles
            </TabsTrigger>
            <TabsTrigger value="quotas" className="gap-2">
              <Gauge className="h-4 w-4" />
              Quotas
            </TabsTrigger>
            <TabsTrigger value="budgets" className="gap-2">
              <Wallet className="h-4 w-4" />
              Budgets
              {budgetAlerts.length > 0 && (
                <Badge variant="destructive" className="h-5 px-1.5 text-xs">
                  {budgetAlerts.length}
                </Badge>
              )}
            </TabsTrigger>
            <TabsTrigger value="dlp" className="gap-2">
              <ShieldAlert className="h-4 w-4" />
              DLP
            </TabsTrigger>
            <TabsTrigger value="executive" className="gap-2">
              <BarChart3 className="h-4 w-4" />
              Executive
            </TabsTrigger>
          </TabsList>
        </div>

        {/* Users Tab */}
        <TabsContent value="users" className="flex-1 m-0 flex flex-col">
          <div className="flex items-center justify-between p-4 border-b">
            <div className="relative max-w-md flex-1">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                type="text"
                placeholder="Search users..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-10"
              />
            </div>
            <Dialog open={isAddUserOpen} onOpenChange={setIsAddUserOpen}>
              <DialogTrigger asChild>
                <Button>
                  <UserPlus className="h-4 w-4 mr-2" />
                  Add User
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Add User</DialogTitle>
                  <DialogDescription>
                    Invite a new user to the platform.
                  </DialogDescription>
                </DialogHeader>
                <div className="space-y-4 py-4">
                  <div className="space-y-2">
                    <Label>Name</Label>
                    <Input placeholder="John Doe" />
                  </div>
                  <div className="space-y-2">
                    <Label>Email</Label>
                    <Input type="email" placeholder="john@company.com" />
                  </div>
                  <div className="space-y-2">
                    <Label>Role</Label>
                    <Select defaultValue="developer">
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="admin">Admin</SelectItem>
                        <SelectItem value="developer">Developer</SelectItem>
                        <SelectItem value="viewer">Viewer</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="space-y-2">
                    <Label>Token Limit</Label>
                    <Input type="number" placeholder="500000" defaultValue="500000" />
                  </div>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => setIsAddUserOpen(false)}>
                    Cancel
                  </Button>
                  <Button onClick={() => setIsAddUserOpen(false)}>
                    Send Invite
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>

          {isLoading && users.length === 0 ? (
            <LoadingState message="Loading users..." />
          ) : (
            <ScrollArea className="flex-1">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>User</TableHead>
                    <TableHead>Role</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Token Usage</TableHead>
                    <TableHead>Last Active</TableHead>
                    <TableHead className="w-[50px]"></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filteredUsers.map((user) => (
                    <TableRow key={user.id}>
                      <TableCell>
                        <div>
                          <div className="font-medium">{user.name}</div>
                          <div className="text-sm text-muted-foreground">{user.email}</div>
                        </div>
                      </TableCell>
                      <TableCell>
                        <RoleBadge role={user.role} />
                      </TableCell>
                      <TableCell>
                        <StatusBadge status={user.status} />
                      </TableCell>
                      <TableCell>
                        <UsageBar used={user.tokenUsage} limit={user.tokenLimit} />
                      </TableCell>
                      <TableCell className="text-muted-foreground text-sm">
                        {user.lastActive || "Never"}
                      </TableCell>
                      <TableCell>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="icon" className="h-8 w-8">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem>
                              <Edit className="h-4 w-4 mr-2" />
                              Edit
                            </DropdownMenuItem>
                            <DropdownMenuItem>
                              <Key className="h-4 w-4 mr-2" />
                              Reset Password
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem onClick={() => handleSuspendUser(user.id)}>
                              <Ban className="h-4 w-4 mr-2" />
                              {user.status === "suspended" ? "Activate" : "Suspend"}
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              className="text-destructive"
                              onClick={() => handleDeleteUser(user.id)}
                            >
                              <Trash2 className="h-4 w-4 mr-2" />
                              Delete
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </ScrollArea>
          )}
        </TabsContent>

        {/* Roles Tab */}
        <TabsContent value="roles" className="flex-1 m-0">
          <ScrollArea className="h-full p-4">
            <div className="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
              {MOCK_ROLES.map((role) => (
                <Card key={role.id}>
                  <CardHeader>
                    <div className="flex items-start justify-between">
                      <div className="flex items-center gap-2">
                        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                          <Shield className="h-5 w-5 text-primary" />
                        </div>
                        <div>
                          <CardTitle className="text-base">{role.name}</CardTitle>
                          <CardDescription className="text-xs">
                            {role.userCount} user{role.userCount !== 1 ? "s" : ""}
                          </CardDescription>
                        </div>
                      </div>
                      <Button variant="ghost" size="icon" className="h-8 w-8">
                        <Edit className="h-4 w-4" />
                      </Button>
                    </div>
                  </CardHeader>
                  <CardContent>
                    <p className="text-sm text-muted-foreground mb-3">{role.description}</p>
                    <div className="flex flex-wrap gap-1">
                      {role.permissions.map((perm) => (
                        <Badge key={perm} variant="secondary" className="text-xs font-mono">
                          {perm}
                        </Badge>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              ))}
              <Card className="border-dashed cursor-pointer hover:border-primary/50">
                <CardContent className="flex flex-col items-center justify-center h-full min-h-[200px]">
                  <Plus className="h-8 w-8 text-muted-foreground mb-2" />
                  <span className="text-sm text-muted-foreground">Create Role</span>
                </CardContent>
              </Card>
            </div>
          </ScrollArea>
        </TabsContent>

        {/* Quotas Tab */}
        <TabsContent value="quotas" className="flex-1 m-0">
          <ScrollArea className="h-full p-4">
            <div className="space-y-6">
              <Card>
                <CardHeader>
                  <CardTitle>Token Usage Overview</CardTitle>
                  <CardDescription>
                    Total platform token consumption and allocation
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="text-2xl font-bold">
                        {(totalTokenUsage / 1000000).toFixed(2)}M
                      </div>
                      <div className="text-sm text-muted-foreground">tokens used today</div>
                    </div>
                    <div className="text-right">
                      <div className="text-2xl font-bold text-muted-foreground">5M</div>
                      <div className="text-sm text-muted-foreground">daily limit</div>
                    </div>
                  </div>
                  <UsageBar used={totalTokenUsage} limit={5000000} />
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>User Quotas</CardTitle>
                  <CardDescription>Individual user token allocations</CardDescription>
                </CardHeader>
                <CardContent>
                  {isLoading && users.length === 0 ? (
                    <LoadingState message="Loading quotas..." />
                  ) : (
                    <div className="space-y-4">
                      {users
                        .sort((a, b) => b.tokenUsage - a.tokenUsage)
                        .map((user) => (
                          <div key={user.id} className="space-y-2">
                            <div className="flex items-center justify-between">
                              <div className="flex items-center gap-2">
                                <span className="font-medium">{user.name}</span>
                                <RoleBadge role={user.role} />
                              </div>
                              <Button variant="ghost" size="sm">
                                <Edit className="h-4 w-4" />
                              </Button>
                            </div>
                            <UsageBar used={user.tokenUsage} limit={user.tokenLimit} />
                          </div>
                        ))}
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          </ScrollArea>
        </TabsContent>

        {/* Budgets Tab */}
        <TabsContent value="budgets" className="flex-1 m-0">
          <ScrollArea className="h-full p-4">
            <div className="space-y-6">
              {/* Header */}
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-lg font-semibold">Budget Management</h2>
                  <p className="text-sm text-muted-foreground">
                    Set spending limits and receive alerts when thresholds are exceeded
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={fetchBudgetData}
                    disabled={isBudgetLoading}
                  >
                    <RefreshCw className={`h-4 w-4 ${isBudgetLoading ? "animate-spin" : ""}`} />
                  </Button>
                  <Dialog open={isAddBudgetOpen} onOpenChange={setIsAddBudgetOpen}>
                    <DialogTrigger asChild>
                      <Button>
                        <Plus className="h-4 w-4 mr-2" />
                        Add Budget
                      </Button>
                    </DialogTrigger>
                    <DialogContent>
                      <DialogHeader>
                        <DialogTitle>Create Budget</DialogTitle>
                        <DialogDescription>
                          Set up a new budget with spending alerts
                        </DialogDescription>
                      </DialogHeader>
                      <div className="space-y-4 py-4">
                        <div className="space-y-2">
                          <Label>Budget Name</Label>
                          <Input
                            placeholder="e.g., Engineering Team"
                            value={newBudgetName}
                            onChange={(e) => setNewBudgetName(e.target.value)}
                          />
                        </div>
                        <div className="space-y-2">
                          <Label>Period</Label>
                          <Select value={newBudgetPeriod} onValueChange={(v) => setNewBudgetPeriod(v as any)}>
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="daily">Daily</SelectItem>
                              <SelectItem value="weekly">Weekly</SelectItem>
                              <SelectItem value="monthly">Monthly</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                        <div className="space-y-2">
                          <Label>Budget Amount (USD)</Label>
                          <Input
                            type="number"
                            placeholder="1000"
                            value={newBudgetAmount}
                            onChange={(e) => setNewBudgetAmount(e.target.value)}
                          />
                        </div>
                        <div className="rounded-lg border p-3 bg-muted/50">
                          <p className="text-sm font-medium mb-2">Default Alert Thresholds</p>
                          <div className="space-y-1 text-sm text-muted-foreground">
                            <div className="flex items-center gap-2">
                              <AlertTriangle className="h-3 w-3 text-yellow-500" />
                              <span>Warning at 70%</span>
                            </div>
                            <div className="flex items-center gap-2">
                              <AlertCircle className="h-3 w-3 text-red-500" />
                              <span>Critical at 90%</span>
                            </div>
                          </div>
                        </div>
                      </div>
                      <DialogFooter>
                        <Button variant="outline" onClick={() => setIsAddBudgetOpen(false)}>
                          Cancel
                        </Button>
                        <Button onClick={handleCreateBudget}>Create Budget</Button>
                      </DialogFooter>
                    </DialogContent>
                  </Dialog>
                </div>
              </div>

              {/* Active Alerts */}
              {budgetAlerts.length > 0 && (
                <div className="space-y-2">
                  <h3 className="text-sm font-medium flex items-center gap-2">
                    <Bell className="h-4 w-4 text-destructive" />
                    Active Alerts ({budgetAlerts.length})
                  </h3>
                  {budgetAlerts.map((alert) => (
                    <div
                      key={alert.id}
                      className={`flex items-center justify-between p-3 rounded-lg ${
                        alert.severity === "critical"
                          ? "bg-red-500/10 border border-red-500/20"
                          : "bg-yellow-500/10 border border-yellow-500/20"
                      }`}
                    >
                      <div className="flex items-center gap-3">
                        {alert.severity === "critical" ? (
                          <AlertCircle className="h-5 w-5 text-red-500" />
                        ) : (
                          <AlertTriangle className="h-5 w-5 text-yellow-500" />
                        )}
                        <div>
                          <p className="font-medium">{alert.budget_name}</p>
                          <p className="text-sm text-muted-foreground">{alert.message}</p>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge variant={alert.severity === "critical" ? "destructive" : "outline"}>
                          {alert.current_percentage.toFixed(0)}% used
                        </Badge>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleAcknowledgeAlert(alert.id)}
                        >
                          <BellOff className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {/* Budget Summary */}
              {isBudgetLoading && !budgetSummary ? (
                <LoadingState message="Loading budgets..." />
              ) : budgetSummary && (
                <>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <Card>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2 text-muted-foreground mb-1">
                          <DollarSign className="h-4 w-4" />
                          <span className="text-xs">Total Budget</span>
                        </div>
                        <div className="text-2xl font-bold">${budgetSummary.total_budget_usd.toLocaleString()}</div>
                      </CardContent>
                    </Card>
                    <Card>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2 text-muted-foreground mb-1">
                          <Gauge className="h-4 w-4" />
                          <span className="text-xs">Total Spend</span>
                        </div>
                        <div className="text-2xl font-bold">${budgetSummary.total_spend_usd.toLocaleString()}</div>
                      </CardContent>
                    </Card>
                    <Card>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2 text-muted-foreground mb-1">
                          <TrendingUp className="h-4 w-4" />
                          <span className="text-xs">Usage</span>
                        </div>
                        <div className="text-2xl font-bold">{budgetSummary.percentage_used}%</div>
                      </CardContent>
                    </Card>
                    <Card>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2 text-muted-foreground mb-1">
                          <AlertTriangle className="h-4 w-4" />
                          <span className="text-xs">Active Alerts</span>
                        </div>
                        <div className="text-2xl font-bold">{budgetSummary.active_alerts}</div>
                      </CardContent>
                    </Card>
                  </div>

                  {/* Budget Cards */}
                  <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                    {budgetSummary.budgets.map((budget) => (
                      <Card key={budget.id}>
                        <CardHeader className="pb-2">
                          <div className="flex items-center justify-between">
                            <CardTitle className="text-base">{budget.name}</CardTitle>
                            <Badge
                              variant={
                                budget.status === "critical"
                                  ? "destructive"
                                  : budget.status === "warning"
                                    ? "outline"
                                    : "secondary"
                              }
                              className={
                                budget.status === "warning" ? "text-yellow-600 border-yellow-500" : ""
                              }
                            >
                              {budget.status === "ok" ? "On Track" : budget.status.toUpperCase()}
                            </Badge>
                          </div>
                        </CardHeader>
                        <CardContent>
                          <div className="space-y-3">
                            <div className="flex items-end justify-between">
                              <div>
                                <div className="text-2xl font-bold">${budget.spend_usd.toLocaleString()}</div>
                                <div className="text-sm text-muted-foreground">
                                  of ${budget.budget_usd.toLocaleString()}
                                </div>
                              </div>
                              <div className="text-right">
                                <div className="text-lg font-semibold">{budget.percentage}%</div>
                                <div className="text-xs text-muted-foreground">used</div>
                              </div>
                            </div>
                            <div className="h-2 bg-muted rounded-full overflow-hidden">
                              <div
                                className={`h-full rounded-full transition-all ${
                                  budget.status === "critical"
                                    ? "bg-red-500"
                                    : budget.status === "warning"
                                      ? "bg-yellow-500"
                                      : "bg-primary"
                                }`}
                                style={{ width: `${Math.min(budget.percentage, 100)}%` }}
                              />
                            </div>
                          </div>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </>
              )}
            </div>
          </ScrollArea>
        </TabsContent>

        {/* DLP Tab */}
        <TabsContent value="dlp" className="flex-1 m-0">
          <ScrollArea className="h-full p-4">
            <div className="space-y-6">
              {/* Header */}
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-lg font-semibold">Data Leakage Prevention</h2>
                  <p className="text-sm text-muted-foreground">
                    Configure rules to detect and protect sensitive data
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={fetchDlpData}
                    disabled={isDlpLoading}
                  >
                    <RefreshCw className={`h-4 w-4 ${isDlpLoading ? "animate-spin" : ""}`} />
                  </Button>
                  <Dialog open={isAddRuleOpen} onOpenChange={setIsAddRuleOpen}>
                    <DialogTrigger asChild>
                      <Button>
                        <Plus className="h-4 w-4 mr-2" />
                        Add Rule
                      </Button>
                    </DialogTrigger>
                    <DialogContent>
                      <DialogHeader>
                        <DialogTitle>Create DLP Rule</DialogTitle>
                        <DialogDescription>
                          Define a pattern to detect sensitive data
                        </DialogDescription>
                      </DialogHeader>
                      <div className="space-y-4 py-4">
                        <div className="space-y-2">
                          <Label>Rule Name</Label>
                          <Input
                            placeholder="e.g., Internal Project IDs"
                            value={newRuleName}
                            onChange={(e) => setNewRuleName(e.target.value)}
                          />
                        </div>
                        <div className="space-y-2">
                          <Label>Pattern (Regex)</Label>
                          <Input
                            placeholder="e.g., PROJ-[0-9]{6}"
                            value={newRulePattern}
                            onChange={(e) => setNewRulePattern(e.target.value)}
                            className="font-mono text-sm"
                          />
                        </div>
                        <div className="space-y-2">
                          <Label>Action</Label>
                          <Select value={newRuleAction} onValueChange={(v) => setNewRuleAction(v as any)}>
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="log">Log Only</SelectItem>
                              <SelectItem value="warn">Warn User</SelectItem>
                              <SelectItem value="redact">Redact Content</SelectItem>
                              <SelectItem value="block">Block Request</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                      </div>
                      <DialogFooter>
                        <Button variant="outline" onClick={() => setIsAddRuleOpen(false)}>
                          Cancel
                        </Button>
                        <Button onClick={handleCreateRule}>Create Rule</Button>
                      </DialogFooter>
                    </DialogContent>
                  </Dialog>
                </div>
              </div>

              {/* Summary Cards */}
              {isDlpLoading && !dlpSummary ? (
                <LoadingState message="Loading DLP data..." />
              ) : dlpSummary && (
                <>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <Card>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2 text-muted-foreground mb-1">
                          <Shield className="h-4 w-4" />
                          <span className="text-xs">Total Rules</span>
                        </div>
                        <div className="text-2xl font-bold">{dlpSummary.total_rules}</div>
                        <div className="text-xs text-muted-foreground">
                          {dlpSummary.active_rules} active
                        </div>
                      </CardContent>
                    </Card>
                    <Card>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2 text-muted-foreground mb-1">
                          <FileWarning className="h-4 w-4" />
                          <span className="text-xs">Incidents (24h)</span>
                        </div>
                        <div className="text-2xl font-bold">{dlpSummary.incidents_24h}</div>
                      </CardContent>
                    </Card>
                    <Card>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2 text-muted-foreground mb-1">
                          <AlertTriangle className="h-4 w-4" />
                          <span className="text-xs">Incidents (7d)</span>
                        </div>
                        <div className="text-2xl font-bold">{dlpSummary.incidents_7d}</div>
                      </CardContent>
                    </Card>
                    <Card>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2 text-muted-foreground mb-1">
                          <Lock className="h-4 w-4" />
                          <span className="text-xs">Top Triggered</span>
                        </div>
                        <div className="text-sm font-medium truncate">
                          {dlpSummary.top_triggered_rules[0]?.rule_name || "N/A"}
                        </div>
                      </CardContent>
                    </Card>
                  </div>

                  {/* Rules Table */}
                  <Card>
                    <CardHeader>
                      <CardTitle className="text-base">DLP Rules</CardTitle>
                      <CardDescription>Manage sensitive data detection patterns</CardDescription>
                    </CardHeader>
                    <CardContent>
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead>Rule</TableHead>
                            <TableHead>Pattern</TableHead>
                            <TableHead>Action</TableHead>
                            <TableHead className="text-right">Matches</TableHead>
                            <TableHead className="w-[80px]">Status</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {dlpRules.map((rule) => (
                            <TableRow key={rule.id}>
                              <TableCell>
                                <div>
                                  <div className="font-medium">{rule.name}</div>
                                  {rule.description && (
                                    <div className="text-xs text-muted-foreground">
                                      {rule.description}
                                    </div>
                                  )}
                                </div>
                              </TableCell>
                              <TableCell>
                                <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                                  {rule.pattern.length > 30
                                    ? rule.pattern.slice(0, 30) + "..."
                                    : rule.pattern}
                                </code>
                              </TableCell>
                              <TableCell>
                                <Badge
                                  variant={
                                    rule.action === "block"
                                      ? "destructive"
                                      : rule.action === "redact"
                                        ? "outline"
                                        : "secondary"
                                  }
                                >
                                  {rule.action}
                                </Badge>
                              </TableCell>
                              <TableCell className="text-right font-mono">
                                {rule.match_count.toLocaleString()}
                              </TableCell>
                              <TableCell>
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="h-8 w-8"
                                  onClick={() => handleToggleRule(rule.id, !rule.enabled)}
                                >
                                  {rule.enabled ? (
                                    <Eye className="h-4 w-4 text-green-500" />
                                  ) : (
                                    <EyeOff className="h-4 w-4 text-muted-foreground" />
                                  )}
                                </Button>
                              </TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </CardContent>
                  </Card>

                  {/* Recent Incidents */}
                  {dlpIncidents.length > 0 && (
                    <Card>
                      <CardHeader>
                        <CardTitle className="text-base">Recent Incidents</CardTitle>
                        <CardDescription>Latest sensitive data detections</CardDescription>
                      </CardHeader>
                      <CardContent>
                        <div className="space-y-3">
                          {dlpIncidents.map((incident) => (
                            <div
                              key={incident.id}
                              className="flex items-start justify-between p-3 rounded-lg bg-muted/50"
                            >
                              <div className="flex items-start gap-3">
                                <div
                                  className={`mt-0.5 p-1.5 rounded ${
                                    incident.action_taken === "block"
                                      ? "bg-red-500/10"
                                      : incident.action_taken === "redact"
                                        ? "bg-yellow-500/10"
                                        : "bg-blue-500/10"
                                  }`}
                                >
                                  {incident.action_taken === "block" ? (
                                    <Ban className="h-4 w-4 text-red-500" />
                                  ) : incident.action_taken === "redact" ? (
                                    <EyeOff className="h-4 w-4 text-yellow-500" />
                                  ) : (
                                    <FileWarning className="h-4 w-4 text-blue-500" />
                                  )}
                                </div>
                                <div>
                                  <div className="font-medium text-sm">{incident.rule_name}</div>
                                  <div className="text-xs text-muted-foreground mt-0.5">
                                    {incident.content_preview}
                                  </div>
                                </div>
                              </div>
                              <div className="text-right">
                                <Badge variant="outline" className="text-xs">
                                  {incident.action_taken}
                                </Badge>
                                <div className="text-xs text-muted-foreground mt-1">
                                  {new Date(incident.triggered_at).toLocaleTimeString()}
                                </div>
                              </div>
                            </div>
                          ))}
                        </div>
                      </CardContent>
                    </Card>
                  )}
                </>
              )}
            </div>
          </ScrollArea>
        </TabsContent>

        {/* Executive Tab */}
        <TabsContent value="executive" className="flex-1 m-0">
          <ScrollArea className="h-full p-4">
            <div className="space-y-6">
              {/* Period Selector */}
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-lg font-semibold">Executive Overview</h2>
                  <p className="text-sm text-muted-foreground">
                    Cost trends, team usage, and project activity
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Select
                    value={executivePeriod}
                    onValueChange={(value) => setExecutivePeriod(value as "daily" | "weekly" | "monthly")}
                  >
                    <SelectTrigger className="w-[140px]">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="daily">Daily</SelectItem>
                      <SelectItem value="weekly">Weekly</SelectItem>
                      <SelectItem value="monthly">Monthly</SelectItem>
                    </SelectContent>
                  </Select>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={fetchExecutiveData}
                    disabled={isExecutiveLoading}
                  >
                    <RefreshCw className={`h-4 w-4 ${isExecutiveLoading ? "animate-spin" : ""}`} />
                  </Button>
                  {/* WebSocket Connection Indicator */}
                  <Badge
                    variant={wsConnected ? "default" : "secondary"}
                    className={`text-xs ${wsConnected ? "bg-green-500" : "bg-gray-400"}`}
                  >
                    {wsConnected ? "Live" : "Polling"}
                  </Badge>
                </div>
              </div>

              {/* Summary Cards */}
              {isExecutiveLoading && !executiveSummary ? (
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  {[1, 2, 3, 4].map((i) => (
                    <Card key={i}>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2">
                          <Loader2 className="h-4 w-4 animate-spin" />
                          <span className="text-sm text-muted-foreground">Loading...</span>
                        </div>
                      </CardContent>
                    </Card>
                  ))}
                </div>
              ) : executiveSummary ? (
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  <Card>
                    <CardContent className="p-4">
                      <div className="flex items-center gap-2 text-muted-foreground mb-1">
                        <DollarSign className="h-4 w-4" />
                        <span className="text-xs">Total Cost</span>
                      </div>
                      <div className="flex items-end justify-between">
                        <div className="text-2xl font-bold">${executiveSummary.total_cost_usd.toFixed(2)}</div>
                        <div
                          className={`flex items-center gap-1 text-sm ${
                            executiveSummary.cost_change_percent > 0
                              ? "text-red-600"
                              : executiveSummary.cost_change_percent < 0
                                ? "text-green-600"
                                : "text-muted-foreground"
                          }`}
                        >
                          {executiveSummary.cost_change_percent > 0 ? (
                            <TrendingUp className="h-4 w-4" />
                          ) : executiveSummary.cost_change_percent < 0 ? (
                            <TrendingDown className="h-4 w-4" />
                          ) : null}
                          {Math.abs(executiveSummary.cost_change_percent).toFixed(1)}%
                        </div>
                      </div>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardContent className="p-4">
                      <div className="flex items-center gap-2 text-muted-foreground mb-1">
                        <Gauge className="h-4 w-4" />
                        <span className="text-xs">Total Tokens</span>
                      </div>
                      <div className="text-2xl font-bold">
                        {(executiveSummary.total_tokens / 1_000_000).toFixed(1)}M
                      </div>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardContent className="p-4">
                      <div className="flex items-center gap-2 text-muted-foreground mb-1">
                        <Users className="h-4 w-4" />
                        <span className="text-xs">Active Users</span>
                      </div>
                      <div className="text-2xl font-bold">{executiveSummary.active_users}</div>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardContent className="p-4">
                      <div className="flex items-center gap-2 text-muted-foreground mb-1">
                        <GitCommit className="h-4 w-4" />
                        <span className="text-xs">Active Projects</span>
                      </div>
                      <div className="text-2xl font-bold">{executiveSummary.active_projects}</div>
                    </CardContent>
                  </Card>
                </div>
              ) : null}

              {/* Alerts */}
              {executiveSummary?.alerts && executiveSummary.alerts.length > 0 && (
                <div className="space-y-2">
                  {executiveSummary.alerts.map((alert, i) => (
                    <div
                      key={i}
                      className={`flex items-center gap-2 p-3 rounded-lg ${
                        alert.type === "critical"
                          ? "bg-red-500/10 text-red-600"
                          : alert.type === "warning"
                            ? "bg-yellow-500/10 text-yellow-600"
                            : "bg-blue-500/10 text-blue-600"
                      }`}
                    >
                      {alert.type === "critical" ? (
                        <AlertCircle className="h-4 w-4" />
                      ) : alert.type === "warning" ? (
                        <AlertTriangle className="h-4 w-4" />
                      ) : (
                        <Info className="h-4 w-4" />
                      )}
                      <span className="text-sm">{alert.message}</span>
                    </div>
                  ))}
                </div>
              )}

              {/* Cost Trends */}
              {executiveTrends && (
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base">Cost Trends</CardTitle>
                    <CardDescription>
                      Token usage and cost over the past {executiveTrends.days} days
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      {/* Simple bar chart using CSS */}
                      <div className="flex items-end gap-1 h-32">
                        {executiveTrends.trends.map((point, i) => {
                          const maxCost = Math.max(...executiveTrends.trends.map((p) => p.cost_usd))
                          const height = maxCost > 0 ? (point.cost_usd / maxCost) * 100 : 0
                          return (
                            <div
                              key={i}
                              className="flex-1 group relative"
                              title={`${point.date}: $${point.cost_usd.toFixed(2)}`}
                            >
                              <div
                                className="w-full bg-primary/80 rounded-t transition-all hover:bg-primary"
                                style={{ height: `${height}%` }}
                              />
                              <div className="absolute -top-8 left-1/2 -translate-x-1/2 opacity-0 group-hover:opacity-100 bg-popover border px-2 py-1 rounded text-xs whitespace-nowrap z-10">
                                ${point.cost_usd.toFixed(2)}
                              </div>
                            </div>
                          )
                        })}
                      </div>
                      {/* X-axis labels */}
                      <div className="flex gap-1 text-xs text-muted-foreground">
                        {executiveTrends.trends.map((point, i) => (
                          <div key={i} className="flex-1 text-center truncate">
                            {point.date.slice(5)}
                          </div>
                        ))}
                      </div>
                    </div>
                    <div className="mt-4 pt-4 border-t grid grid-cols-3 gap-4 text-sm">
                      <div>
                        <div className="text-muted-foreground">Total Cost</div>
                        <div className="font-semibold">${executiveTrends.totals.cost_usd.toFixed(2)}</div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">Total Tokens</div>
                        <div className="font-semibold">
                          {(executiveTrends.totals.total_tokens / 1_000_000).toFixed(2)}M
                        </div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">Total Requests</div>
                        <div className="font-semibold">{executiveTrends.totals.requests}</div>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Team Breakdown */}
              {executiveTeams && (
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base">Team Usage</CardTitle>
                    <CardDescription>Token consumption by department</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-4">
                      {executiveTeams.teams.map((team) => (
                        <div key={team.team_id} className="space-y-2">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                              <UsersRound className="h-4 w-4 text-muted-foreground" />
                              <span className="font-medium">{team.team_name}</span>
                              <Badge variant="secondary" className="text-xs">
                                {team.member_count} members
                              </Badge>
                            </div>
                            <div className="text-sm text-muted-foreground">
                              {(team.tokens_used / 1_000_000).toFixed(2)}M tokens ({team.percentage}%)
                            </div>
                          </div>
                          <div className="h-2 bg-muted rounded-full overflow-hidden">
                            <div
                              className="h-full bg-primary rounded-full transition-all"
                              style={{ width: `${team.percentage}%` }}
                            />
                          </div>
                          {team.top_users.length > 0 && (
                            <div className="flex gap-2 ml-6 text-xs text-muted-foreground">
                              Top: {team.top_users.slice(0, 3).map((u) => u.name).join(", ")}
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Project Activity */}
              {executiveActivity && (
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base">Project Activity</CardTitle>
                    <CardDescription>Git commits and AI sessions by project</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Project</TableHead>
                          <TableHead className="text-right">Today</TableHead>
                          <TableHead className="text-right">This Week</TableHead>
                          <TableHead className="text-right">AI Sessions</TableHead>
                          <TableHead className="text-right">Contributors</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {executiveActivity.projects.map((project) => (
                          <TableRow key={project.project_id}>
                            <TableCell className="font-medium">{project.project_name}</TableCell>
                            <TableCell className="text-right">
                              <Badge variant="outline">{project.commits_today} commits</Badge>
                            </TableCell>
                            <TableCell className="text-right">
                              <Badge variant="secondary">{project.commits_week} commits</Badge>
                            </TableCell>
                            <TableCell className="text-right">{project.ai_sessions}</TableCell>
                            <TableCell className="text-right">{project.active_contributors}</TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                    <div className="mt-4 pt-4 border-t grid grid-cols-3 gap-4 text-sm">
                      <div>
                        <div className="text-muted-foreground">Commits Today</div>
                        <div className="font-semibold">{executiveActivity.totals.commits_today}</div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">Commits This Week</div>
                        <div className="font-semibold">{executiveActivity.totals.commits_week}</div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">AI Sessions</div>
                        <div className="font-semibold">{executiveActivity.totals.ai_sessions}</div>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Model Usage */}
              {executiveSummary?.top_models && (
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base">Model Usage</CardTitle>
                    <CardDescription>Cost distribution by AI model</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-3">
                      {executiveSummary.top_models.map((model) => (
                        <div key={model.model} className="space-y-1">
                          <div className="flex items-center justify-between text-sm">
                            <span className="font-medium">{model.model}</span>
                            <span className="text-muted-foreground">
                              ${model.cost_usd.toFixed(2)} ({model.usage_percent}%)
                            </span>
                          </div>
                          <div className="h-2 bg-muted rounded-full overflow-hidden">
                            <div
                              className="h-full bg-primary/60 rounded-full"
                              style={{ width: `${model.usage_percent}%` }}
                            />
                          </div>
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </div>
  )
}

export default Admin
