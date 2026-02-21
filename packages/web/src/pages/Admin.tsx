/**
 * Admin Page
 *
 * Administrative dashboard with:
 * - User management
 * - Role and permission configuration
 * - Token quota and usage tracking
 * - System statistics
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
  Clock,
  Loader2,
  RefreshCw,
  AlertCircle,
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
import type { MeteringUsageResponse, MeteringUserReport } from "@/lib/types"

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
      </Tabs>
    </div>
  )
}

export default Admin
