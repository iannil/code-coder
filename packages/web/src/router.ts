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

const sessionRoute = createRoute({
  getParentRoute: () => sessionsRoute,
  path: "/$sessionId",
  component: lazyRouteComponent(() => import("./pages/Session"), "Session"),
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
// Files Route
// ============================================================================

const filesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/files",
  component: lazyRouteComponent(() => import("./pages/Files"), "Files"),
})

// ============================================================================
// Route Tree Assembly
// ============================================================================

const routeTree = rootRoute.addChildren([
  indexRoute,
  sessionsRoute.addChildren([sessionRoute]),
  settingsRoute,
  filesRoute,
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
