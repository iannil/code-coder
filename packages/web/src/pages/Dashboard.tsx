/**
 * Dashboard Page
 *
 * The main dashboard showing:
 * - Welcome message
 * - Recent sessions
 * - Quick actions
 * - Stats overview
 */

import * as React from "react"
import { useNavigate } from "@tanstack/react-router"
import {
  MessageSquare,
  Plus,
  Clock,
  FileText,
  Settings,
  Folder,
  Sparkles,
  ArrowRight,
  TrendingUp,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent } from "@/components/ui/Card"
import { Skeleton } from "@/components/ui/Skeleton"
import { useSessions, useSessionStore, useSessionsLoading, useActiveSessionId } from "@/stores/session"
import { cn, formatRelativeTime } from "@/lib/utils"

// ============================================================================
// Quick Action Card Component
// ============================================================================

interface QuickAction {
  title: string
  description: string
  icon: React.ElementType
  onClick: () => void
  variant?: "default" | "primary"
}

function QuickActionCard({ action }: { action: QuickAction }) {
  const Icon = action.icon

  return (
    <button
      onClick={action.onClick}
      data-testid={action.title === "New Session" ? "create-session-btn" : undefined}
      className={cn(
        "flex flex-col items-start gap-3 rounded-lg border p-4 text-left transition-all hover:shadow-md",
        "hover:border-primary/50",
        action.variant === "primary" && "bg-primary/5 border-primary/20"
      )}
    >
      <div
        className={cn(
          "flex h-10 w-10 items-center justify-center rounded-lg",
          action.variant === "primary" ? "bg-primary text-primary-foreground" : "bg-muted"
        )}
      >
        <Icon className="h-5 w-5" />
      </div>
      <div className="space-y-1">
        <h3 className="font-medium">{action.title}</h3>
        <p className="text-sm text-muted-foreground">{action.description}</p>
      </div>
    </button>
  )
}

// ============================================================================
// Recent Session Item Component
// ============================================================================

interface RecentSessionItemProps {
  title: string
  updatedAt: number
  isActive: boolean
  onClick: () => void
}

function RecentSessionItem({
  title,
  updatedAt,
  isActive,
  onClick,
}: RecentSessionItemProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex items-center gap-3 rounded-lg border p-3 text-left transition-all",
        "hover:bg-accent hover:border-accent",
        isActive && "bg-accent border-accent"
      )}
    >
      <div
        className={cn(
          "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg",
          isActive ? "bg-primary text-primary-foreground" : "bg-muted"
        )}
      >
        <MessageSquare className="h-5 w-5" />
      </div>
      <div className="flex-1 min-w-0">
        <h4 className="font-medium truncate">{title || "Untitled Session"}</h4>
        <p className="text-sm text-muted-foreground">{formatRelativeTime(updatedAt)}</p>
      </div>
      <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
    </button>
  )
}

// ============================================================================
// Stat Card Component
// ============================================================================

interface StatCardProps {
  title: string
  value: string | number
  change?: string
  icon: React.ElementType
}

function StatCard({ title, value, change, icon: Icon }: StatCardProps) {
  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-muted-foreground">{title}</p>
            <p className="text-2xl font-bold mt-1">{value}</p>
            {change && (
              <p className="text-xs text-muted-foreground mt-1 flex items-center gap-1">
                <TrendingUp className="h-3 w-3" />
                {change}
              </p>
            )}
          </div>
          <div className="h-12 w-12 rounded-full bg-primary/10 flex items-center justify-center">
            <Icon className="h-6 w-6 text-primary" />
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Main Dashboard Component
// ============================================================================

export function Dashboard() {
  const navigate = useNavigate()
  const sessions = useSessions()
  const { isLoading, isLoaded } = useSessionsLoading()
  const activeSessionId = useActiveSessionId()
  const createSession = useSessionStore((state) => state.createSession)
  const setActiveSession = useSessionStore((state) => state.setActiveSession)

  // Get recent sessions (last 5)
  const recentSessions = React.useMemo(
    () => [...sessions].sort((a, b) => b.time.updated - a.time.updated).slice(0, 5),
    [sessions]
  )

  // Calculate stats
  const stats = React.useMemo(
    () => ({
      totalSessions: sessions.length,
      todaySessions: sessions.filter((s) => {
        const today = new Date()
        today.setHours(0, 0, 0, 0)
        return s.time.updated >= today.getTime()
      }).length,
      activeSession: sessions.find((s) => s.id === activeSessionId),
    }),
    [sessions, activeSessionId]
  )

  // Quick actions
  const quickActions: QuickAction[] = React.useMemo(
    () => [
      {
        title: "New Session",
        description: "Start a new conversation",
        icon: Plus,
        variant: "primary",
        onClick: async () => {
          try {
            const session = await createSession()
            setActiveSession(session.id)
            navigate({ to: "/sessions/$sessionId", params: { sessionId: session.id } })
          } catch {
            // Error handled in store
          }
        },
      },
      {
        title: "Browse Files",
        description: "Explore your codebase",
        icon: Folder,
        onClick: () => navigate({ to: "/files" }),
      },
      {
        title: "Settings",
        description: "Configure preferences",
        icon: Settings,
        onClick: () => navigate({ to: "/settings" }),
      },
    ],
    [navigate, createSession, setActiveSession]
  )

  const handleSessionClick = (sessionId: string) => {
    setActiveSession(sessionId)
    navigate({ to: "/sessions/$sessionId", params: { sessionId } })
  }

  return (
    <div className="flex-1 overflow-auto">
      <div className="container max-w-6xl mx-auto p-6 space-y-8">
        {/* Welcome Section */}
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <Sparkles className="h-6 w-6 text-primary" />
            <h1 className="text-3xl font-bold tracking-tight">Welcome to CodeCoder</h1>
          </div>
          <p className="text-muted-foreground text-lg">
            Your AI-powered coding companion for building, reviewing, and understanding code.
          </p>
        </div>

        {/* Stats Overview */}
        {isLoaded && sessions.length > 0 && (
          <div className="grid gap-4 md:grid-cols-3">
            <StatCard
              title="Total Sessions"
              value={stats.totalSessions}
              icon={MessageSquare}
            />
            <StatCard
              title="Active Today"
              value={stats.todaySessions}
              change="sessions"
              icon={Clock}
            />
            <StatCard
              title="Total Messages"
              value="—"
              icon={FileText}
            />
          </div>
        )}

        {/* Quick Actions */}
        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Quick Actions</h2>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {quickActions.map((action) => (
              <QuickActionCard key={action.title} action={action} />
            ))}
          </div>
        </div>

        {/* Recent Sessions */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-semibold">Recent Sessions</h2>
            {sessions.length > 5 && (
              <Button variant="ghost" size="sm" onClick={() => navigate({ to: "/" })}>
                View all
                <ArrowRight className="ml-2 h-4 w-4" />
              </Button>
            )}
          </div>

          {isLoading && !isLoaded ? (
            <div className="grid gap-3">
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 rounded-lg border p-3"
                >
                  <Skeleton className="h-10 w-10 rounded-lg" />
                  <div className="flex-1 space-y-2">
                    <Skeleton className="h-4 w-32" />
                    <Skeleton className="h-3 w-24" />
                  </div>
                </div>
              ))}
            </div>
          ) : recentSessions.length > 0 ? (
            <div className="grid gap-3">
              {recentSessions.map((session) => (
                <RecentSessionItem
                  key={session.id}
                  title={session.title}
                  updatedAt={session.time.updated}
                  isActive={session.id === activeSessionId}
                  onClick={() => handleSessionClick(session.id)}
                />
              ))}
            </div>
          ) : (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12">
                <MessageSquare className="h-12 w-12 text-muted-foreground mb-4" />
                <h3 className="text-lg font-semibold mb-2">No sessions yet</h3>
                <p className="text-sm text-muted-foreground mb-4 text-center">
                  Create your first session to start chatting with CodeCoder
                </p>
                <Button data-testid="create-session-btn" onClick={() => quickActions[0].onClick()}>
                  <Plus className="mr-2 h-4 w-4" />
                  New Session
                </Button>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  )
}
