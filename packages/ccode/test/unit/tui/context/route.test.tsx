/**
 * Unit Tests: Route Context
 * Testing route navigation and state management
 */

import { describe, test, expect } from "bun:test"
import { type Route, type HomeRoute, type SessionRoute } from "@/cli/cmd/tui/context/route"
import type { PromptInfo } from "@/cli/cmd/tui/component/prompt/history"

// Mock PromptInfo for testing (matching the actual type)
function createPromptInfo(input: string): PromptInfo {
  return {
    input,
    parts: [{ type: "text", text: input }],
  } as PromptInfo
}

describe("Route Context", () => {
  describe("Route types", () => {
    test("should accept home route", () => {
      const route: HomeRoute = {
        type: "home",
      }
      expect(route.type).toBe("home")
    })

    test("should accept home route with initial prompt", () => {
      const route: HomeRoute = {
        type: "home",
        initialPrompt: createPromptInfo("test prompt"),
      }
      expect(route.type).toBe("home")
      expect(route.initialPrompt?.input).toBe("test prompt")
    })

    test("should accept session route", () => {
      const route: SessionRoute = {
        type: "session",
        sessionID: "ses_test123",
      }
      expect(route.type).toBe("session")
      expect(route.sessionID).toBe("ses_test123")
    })

    test("should accept session route with initial prompt", () => {
      const route: SessionRoute = {
        type: "session",
        sessionID: "ses_test123",
        initialPrompt: createPromptInfo("test prompt"),
      }
      expect(route.type).toBe("session")
      expect(route.initialPrompt?.input).toBe("test prompt")
    })
  })

  describe("Route type guards", () => {
    test("should distinguish home from session route", () => {
      const homeRoute: Route = { type: "home" }
      const sessionRoute: Route = { type: "session", sessionID: "ses_123" }

      const isHome = homeRoute.type === "home"
      const isSession = sessionRoute.type === "session"

      expect(isHome).toBe(true)
      expect(sessionRoute.type === "session").toBe(true)
    })

    test("should narrow route type with type guard", () => {
      const routes: Route[] = [
        { type: "home" },
        { type: "session", sessionID: "ses_1" },
        { type: "session", sessionID: "ses_2", initialPrompt: createPromptInfo("test") },
      ]

      const sessionRoutes = routes.filter((r): r is SessionRoute => r.type === "session")

      expect(sessionRoutes).toHaveLength(2)
      expect(sessionRoutes[0].sessionID).toBe("ses_1")
      expect(sessionRoutes[1].sessionID).toBe("ses_2")
    })
  })

  describe("Route serialization", () => {
    test("should serialize home route to JSON", () => {
      const route: HomeRoute = { type: "home" }
      const json = JSON.stringify(route)
      const parsed = JSON.parse(json) as Route

      expect(parsed).toEqual(route)
    })

    test("should serialize session route to JSON", () => {
      const route: SessionRoute = {
        type: "session",
        sessionID: "ses_abc123",
      }
      const json = JSON.stringify(route)
      const parsed = JSON.parse(json) as Route

      expect(parsed).toEqual(route)
    })
  })

  describe("Route navigation", () => {
    test("should update route state", () => {
      // Simulating route state updates
      let currentRoute: Route = { type: "home" }
      const newRoute: SessionRoute = { type: "session", sessionID: "ses_update" }

      // Simulate navigate function
      currentRoute = newRoute

      expect(currentRoute.type).toBe("session")
      expect((currentRoute as SessionRoute).sessionID).toBe("ses_update")
    })
  })

  describe("useRouteData", () => {
    test("should extract route data by type", () => {
      // Simulating useRouteData behavior
      const routes: Route[] = [
        { type: "home" },
        { type: "session", sessionID: "ses_1" },
        { type: "home", initialPrompt: createPromptInfo("test") },
      ]

      // Extract home routes
      const homeRoutes = routes.filter((r) => r.type === "home") as HomeRoute[]
      expect(homeRoutes).toHaveLength(2)

      // Extract session routes
      const sessionRoutes = routes.filter((r) => r.type === "session") as SessionRoute[]
      expect(sessionRoutes).toHaveLength(1)
      expect(sessionRoutes[0].sessionID).toBe("ses_1")
    })

    test("should preserve initialPrompt in route data", () => {
      const prompt = createPromptInfo("What is the tech stack?")
      const route: Route = {
        type: "home",
        initialPrompt: prompt,
      }

      if (route.type === "home") {
        expect(route.initialPrompt?.input).toBe("What is the tech stack?")
        expect(route.initialPrompt?.parts).toHaveLength(1)
      }
    })
  })
})
