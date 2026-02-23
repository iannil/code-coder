/**
 * App Component
 *
 * Main layout component with:
 * - Router setup with TanStack Router
 * - Layout structure with Header, Sidebar, MainPanel
 * - Error boundary
 * - Toast provider
 *
 * Note: The router is configured in router.ts and rendered in main.tsx
 * This file is kept for reference but is no longer the main entry point.
 *
 * For the actual routing structure, see router.ts which uses the following outlets:
 * - #sidebar-outlet: Renders the Sidebar component
 * - #header-outlet: Renders the Header component
 * - #main-outlet: Renders the active page/route content
 */

import { Outlet, Link, useNavigate, useMatchRoute } from "@tanstack/react-router"
import { useConfig, useConfigStore } from "@/stores/config"
import { useSessionStore, useSessions } from "@/stores/session"
import { useEffect, useRef } from "react"
import { ThemeToggle } from "@/components/theme"
import { CommandPaletteProvider } from "@/components/command"
import { cn } from "@/lib/utils"

// ============================================================================
// Navigation Item Component
// ============================================================================

interface NavItemProps {
  to: string
  icon: React.ReactNode
  label: string
  testId?: string
}

function NavItem({ to, icon, label, testId }: NavItemProps) {
  const matchRoute = useMatchRoute()
  const isActive = matchRoute({ to, fuzzy: true })

  return (
    <Link to={to}>
      <button
        type="button"
        data-testid={testId}
        className={cn(
          "w-full flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
          isActive
            ? "bg-accent text-accent-foreground font-medium"
            : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
        )}
      >
        {icon}
        {label}
      </button>
    </Link>
  )
}

// ============================================================================
// Navigation Group Component
// ============================================================================

interface NavGroupProps {
  label: string
  children: React.ReactNode
}

function NavGroup({ label, children }: NavGroupProps) {
  return (
    <div className="space-y-1">
      <h3 className="px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
        {label}
      </h3>
      <div className="space-y-0.5">{children}</div>
    </div>
  )
}

// ============================================================================
// Recent Sessions Component
// ============================================================================

function RecentSessions() {
  const sessions = useSessions()
  const navigate = useNavigate()

  // Sort by updated time and take the 5 most recent
  const recentSessions = [...sessions]
    .sort((a, b) => (b.time.updated ?? b.time.created) - (a.time.updated ?? a.time.created))
    .slice(0, 5)

  if (recentSessions.length === 0) {
    return (
      <div className="px-3 py-4 text-center">
        <p className="text-xs text-muted-foreground">No recent sessions</p>
      </div>
    )
  }

  return (
    <div className="space-y-1">
      {recentSessions.map((session) => (
        <button
          key={session.id}
          type="button"
          onClick={() => navigate({ to: "/sessions/$sessionId", params: { sessionId: session.id } })}
          className="w-full flex items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors text-left"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="shrink-0"
          >
            <path d="m3 21 1.9-5.7a8.5 8.5 0 1 1 3.8 3.8z" />
          </svg>
          <span className="truncate">{session.title || `Session ${session.id.slice(0, 8)}`}</span>
        </button>
      ))}
    </div>
  )
}

// ============================================================================
// Layout Component
// ============================================================================

export function AppLayout() {
  const navigate = useNavigate()
  const loadConfig = useConfigStore((state) => state.loadConfig)
  const loadSessions = useSessionStore((state) => state.loadSessions)
  const config = useConfig()

  // Track if initialization has happened
  const initialized = useRef(false)

  // Initialize app data once
  useEffect(() => {
    if (initialized.current) return
    initialized.current = true

    loadConfig()
    loadSessions()
  }, [loadConfig, loadSessions])

  // Set up SSE connection
  useEffect(() => {
    // SSE is initialized in the Session page when needed
    // This is a placeholder for global SSE initialization
  }, [])

  const handleNewSession = () => {
    navigate({ to: "/" })
  }

  const appName = (config?.appName as string | undefined) ?? "CodeCoder"

  return (
    <CommandPaletteProvider>
      <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
        {/* Header */}
        <header className="flex h-14 items-center justify-between border-b bg-background px-4 sm:px-6 shrink-0">
        <div className="flex items-center gap-3">
          <Link to="/">
            <div className="flex h-8 w-8 cursor-pointer items-center justify-center rounded-lg bg-primary text-primary-foreground">
              <span className="text-sm font-bold">CC</span>
            </div>
          </Link>
          <h1 className="text-lg font-semibold">{appName}</h1>
        </div>

        <div className="flex items-center gap-2">
          <ThemeToggle />
          <Link to="/settings">
            <button
              type="button"
              data-testid="nav-settings"
              className="rounded-md p-2 text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
              aria-label="Open settings"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
                <circle cx="12" cy="12" r="3" />
              </svg>
            </button>
          </Link>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <aside data-testid="sidebar" className="w-[260px] shrink-0 border-r bg-background flex flex-col">
          <div className="flex items-center justify-between p-4">
            <h2 className="text-sm font-semibold">Navigation</h2>
            <button
              type="button"
              onClick={handleNewSession}
              data-testid="new-session-btn"
              className="rounded-md p-1.5 text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
              aria-label="New session"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M5 12h14" />
                <path d="M12 5v14" />
              </svg>
            </button>
          </div>

          <div className="px-4 pb-4 space-y-4 flex-1 overflow-y-auto">
            {/* Main Section */}
            <NavGroup label="Main">
              <NavItem
                to="/"
                testId="nav-dashboard"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <rect width="7" height="9" x="3" y="3" rx="1" />
                    <rect width="7" height="5" x="14" y="3" rx="1" />
                    <rect width="7" height="9" x="14" y="12" rx="1" />
                    <rect width="7" height="5" x="3" y="16" rx="1" />
                  </svg>
                }
                label="Dashboard"
              />
              <NavItem
                to="/chat"
                testId="nav-chat"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
                    <path d="M12 17h.01" />
                    <circle cx="12" cy="12" r="10" />
                  </svg>
                }
                label="Chat"
              />
              <NavItem
                to="/compare"
                testId="nav-compare"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M16 3h5v5" />
                    <path d="M8 3H3v5" />
                    <path d="M21 3l-7 7" />
                    <path d="M3 3l7 7" />
                    <path d="M16 21h5v-5" />
                    <path d="M8 21H3v-5" />
                    <path d="M21 21l-7-7" />
                    <path d="M3 21l7-7" />
                  </svg>
                }
                label="Compare"
              />
              <NavItem
                to="/sessions"
                testId="nav-sessions"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="m3 21 1.9-5.7a8.5 8.5 0 1 1 3.8 3.8z" />
                  </svg>
                }
                label="Sessions"
              />
              <NavItem
                to="/tasks"
                testId="nav-tasks"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <rect x="3" y="5" width="6" height="6" rx="1" />
                    <path d="m3 17 2 2 4-4" />
                    <path d="M13 6h8" />
                    <path d="M13 12h8" />
                    <path d="M13 18h8" />
                  </svg>
                }
                label="Tasks"
              />
            </NavGroup>

            {/* Workspace Section */}
            <NavGroup label="Workspace">
              <NavItem
                to="/projects"
                testId="nav-projects"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
                    <path d="M2 10h20" />
                  </svg>
                }
                label="Projects"
              />
              <NavItem
                to="/files"
                testId="nav-files"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
                    <polyline points="14 2 14 8 20 8" />
                  </svg>
                }
                label="Files"
              />
              <NavItem
                to="/documents"
                testId="nav-documents"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20" />
                  </svg>
                }
                label="Documents"
              />
            </NavGroup>

            {/* Assistants Section */}
            <NavGroup label="Assistants">
              <NavItem
                to="/agents"
                testId="nav-agents"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M12 8V4H8" />
                    <rect width="16" height="12" x="4" y="8" rx="2" />
                    <path d="M2 14h2" />
                    <path d="M20 14h2" />
                    <path d="M15 13v2" />
                    <path d="M9 13v2" />
                  </svg>
                }
                label="Agents"
              />
              <NavItem
                to="/skills"
                testId="nav-skills"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M19.439 7.85c-.049.322.059.648.289.878l1.568 1.568c.47.47.706 1.087.706 1.704s-.235 1.233-.706 1.704l-1.611 1.611a.98.98 0 0 1-.837.276c-.47-.07-.802-.48-.743-.95a.646.646 0 0 0-.129-.525.646.646 0 0 0-.511-.212h-5.432c-.706 0-1.279-.573-1.279-1.279V9.375c0-.706.573-1.279 1.279-1.279h5.432c.199 0 .386-.077.525-.212a.646.646 0 0 0 .13-.525c-.06-.47.27-.88.742-.95a.98.98 0 0 1 .837.276l1.612 1.611c.47.47.706 1.087.706 1.704s-.236 1.234-.706 1.704l-1.568 1.568a.985.985 0 0 1-.29.878Z" />
                    <path d="M4 22V4c0-.5.2-1 .6-1.4C5 2.2 5.5 2 6 2h8.5L20 7.5V22c0 .5-.2 1-.6 1.4-.4.4-.9.6-1.4.6H6c-.5 0-1-.2-1.4-.6C4.2 23 4 22.5 4 22Z" />
                  </svg>
                }
                label="Skills"
              />
              <NavItem
                to="/memory"
                testId="nav-memory"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 4.44-2.54" />
                    <path d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-4.44-2.54" />
                  </svg>
                }
                label="Memory"
              />
            </NavGroup>

            {/* Automation Section */}
            <NavGroup label="Automation">
              <NavItem
                to="/workflows"
                testId="nav-workflows"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <circle cx="5" cy="6" r="3" />
                    <path d="M5 9v6" />
                    <circle cx="5" cy="18" r="3" />
                    <path d="M12 3v18" />
                    <circle cx="19" cy="6" r="3" />
                    <path d="M16 15.7A9 9 0 0 0 19 9" />
                  </svg>
                }
                label="Workflows"
              />
            </NavGroup>

            {/* Infrastructure Section */}
            <NavGroup label="Infrastructure">
              <NavItem
                to="/infrastructure"
                testId="nav-infrastructure"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <rect width="20" height="8" x="2" y="2" rx="2" ry="2" />
                    <rect width="20" height="8" x="2" y="14" rx="2" ry="2" />
                    <line x1="6" x2="6.01" y1="6" y2="6" />
                    <line x1="6" x2="6.01" y1="18" y2="18" />
                  </svg>
                }
                label="Infrastructure"
              />
              <NavItem
                to="/admin"
                testId="nav-admin"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10" />
                    <path d="m9 12 2 2 4-4" />
                  </svg>
                }
                label="Admin"
              />
            </NavGroup>
          </div>

          <div className="border-t" />

          {/* Recent Sessions */}
          <div className="p-4">
            <h3 className="px-3 mb-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
              Recent
            </h3>
            <RecentSessions />
          </div>
        </aside>

        {/* Main Content */}
        <main data-testid="main-panel" className="flex-1 overflow-auto">
          <Outlet />
        </main>
      </div>
    </div>
    </CommandPaletteProvider>
  )
}

// ============================================================================
// Legacy Default Export (kept for compatibility)
// ============================================================================

export default function App() {
  return <AppLayout />
}
