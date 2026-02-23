/**
 * TanStack Router Configuration
 *
 * Sets up the router with route definitions for the CodeCoder web application
 */

import {
  createRouter,
  createRoute,
  createRootRoute,
  lazyRouteComponent,
} from "@tanstack/react-router"
import { AppLayout } from "./App"

// ============================================================================
// Root Route with Layout
// ============================================================================

const rootRoute = createRootRoute({
  component: AppLayout,
})

// ============================================================================
// Index (Dashboard) Route
// ============================================================================

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: lazyRouteComponent(() => import("./pages/Dashboard"), "Dashboard"),
})

// ============================================================================
// Sessions Routes
// ============================================================================

const sessionsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/sessions",
})

const sessionsIndexRoute = createRoute({
  getParentRoute: () => sessionsRoute,
  path: "/",
  component: lazyRouteComponent(() => import("./pages/Sessions"), "Sessions"),
})

const sessionRoute = createRoute({
  getParentRoute: () => sessionsRoute,
  path: "/$sessionId",
  component: lazyRouteComponent(() => import("./pages/Session"), "Session"),
})

// ============================================================================
// Agents Route
// ============================================================================

const agentsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/agents",
  component: lazyRouteComponent(() => import("./pages/Agents"), "Agents"),
})

// ============================================================================
// Memory Route
// ============================================================================

const memoryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/memory",
  component: lazyRouteComponent(() => import("./pages/Memory"), "Memory"),
})

// ============================================================================
// Infrastructure Route
// ============================================================================

const infrastructureRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/infrastructure",
  component: lazyRouteComponent(() => import("./pages/Infrastructure"), "Infrastructure"),
})

// ============================================================================
// Settings Route
// ============================================================================

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: lazyRouteComponent(() => import("./pages/Settings"), "Settings"),
})

// ============================================================================
// Chat Route
// ============================================================================

const chatRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/chat",
  component: lazyRouteComponent(() => import("./pages/Chat"), "Chat"),
})

// ============================================================================
// Skills Route
// ============================================================================

const skillsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/skills",
  component: lazyRouteComponent(() => import("./pages/Skills"), "Skills"),
})

// ============================================================================
// Workflows Route
// ============================================================================

const workflowsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/workflows",
  component: lazyRouteComponent(() => import("./pages/Workflows"), "Workflows"),
})

// ============================================================================
// Admin Route
// ============================================================================

const adminRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/admin",
  component: lazyRouteComponent(() => import("./pages/Admin"), "Admin"),
})

// ============================================================================
// Compare Route (Multi-model A/B Testing)
// ============================================================================

const compareRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/compare",
  component: lazyRouteComponent(() => import("./pages/Compare"), "Compare"),
})

// ============================================================================
// Knowledge Route
// ============================================================================

const knowledgeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/knowledge",
  component: lazyRouteComponent(() => import("./pages/Knowledge"), "Knowledge"),
})

// ============================================================================
// Files Route
// ============================================================================

const filesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/files",
  component: lazyRouteComponent(() => import("./pages/Files"), "Files"),
})

// ============================================================================
// Documents Route
// ============================================================================

const documentsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/documents",
  component: lazyRouteComponent(() => import("./pages/Documents"), "Documents"),
})

// ============================================================================
// Projects Route
// ============================================================================

const projectsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/projects",
  component: lazyRouteComponent(() => import("./pages/Projects"), "Projects"),
})

// ============================================================================
// Tasks Route
// ============================================================================

const tasksRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tasks",
  component: lazyRouteComponent(() => import("./pages/Tasks"), "Tasks"),
})

// ============================================================================
// Route Tree Assembly
// ============================================================================

const routeTree = rootRoute.addChildren([
  indexRoute,
  sessionsRoute.addChildren([sessionsIndexRoute, sessionRoute]),
  settingsRoute,
  filesRoute,
  documentsRoute,
  projectsRoute,
  tasksRoute,
  agentsRoute,
  memoryRoute,
  infrastructureRoute,
  chatRoute,
  compareRoute,
  knowledgeRoute,
  skillsRoute,
  workflowsRoute,
  adminRoute,
])

// ============================================================================
// Router Creation
// ============================================================================

export const router = createRouter({
  routeTree,
  defaultPreload: "intent",
  defaultPreloadStaleTime: 0,
})

// ============================================================================
// Type Exports
// ============================================================================

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router
  }
}
