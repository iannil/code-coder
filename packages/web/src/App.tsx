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

import { Outlet, Link, useNavigate } from "@tanstack/react-router"
import { useConfig, useConfigStore } from "@/stores/config"
import { useSessionStore } from "@/stores/session"
import { useEffect, useRef } from "react"
import { ThemeToggle } from "@/components/theme"
import { CommandPaletteProvider } from "@/components/command"

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
        <aside data-testid="sidebar" className="w-[260px] shrink-0 border-r bg-background">
          <div className="flex items-center justify-between p-4">
            <h2 className="text-sm font-semibold">Sessions</h2>
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

          <div className="px-4 pb-4">
            <Link to="/">
              <button
                type="button"
                data-testid="nav-dashboard"
                className="w-full flex items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
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
                  <rect width="7" height="9" x="3" y="3" rx="1" />
                  <rect width="7" height="5" x="14" y="3" rx="1" />
                  <rect width="7" height="9" x="14" y="12" rx="1" />
                  <rect width="7" height="5" x="3" y="16" rx="1" />
                </svg>
                Dashboard
              </button>
            </Link>
            <Link to="/files">
              <button
                type="button"
                data-testid="nav-files"
                className="w-full flex items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
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
                  <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
                  <polyline points="14 2 14 8 20 8" />
                </svg>
                Files
              </button>
            </Link>
            <Link to="/documents">
              <button
                type="button"
                className="w-full flex items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
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
                  <path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20" />
                </svg>
                Documents
              </button>
            </Link>
          </div>

          <div className="border-t" />
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
