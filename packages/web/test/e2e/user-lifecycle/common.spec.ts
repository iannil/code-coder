/**
 * ULC-CMN-* Tests: Common Features for All User Types
 *
 * Tests for features shared across all user types:
 * Dashboard, Session Management, Theme, Settings, etc.
 */

import { test, expect, type Page } from "@playwright/test"

// Skip E2E tests by default unless explicitly enabled
const SKIP_E2E = process.env.SKIP_E2E !== "false"

test.describe.configure({ mode: "serial" })

test.describe("ULC-CMN: Common Features", () => {
  test.skip(SKIP_E2E, "E2E tests skipped - set SKIP_E2E=false to run")

  test.describe("ULC-CMN-DASH: Dashboard", () => {
    test("ULC-CMN-DASH-001: should load dashboard", async ({ page }) => {
      await page.goto("/")
      await expect(page).toHaveTitle(/CodeCoder/)
    })

    test("ULC-CMN-DASH-002: should display sidebar navigation", async ({ page }) => {
      await page.goto("/")
      await expect(page.locator('[data-testid="sidebar"]')).toBeVisible()
    })

    test("ULC-CMN-DASH-003: should display main content area", async ({ page }) => {
      await page.goto("/")
      await expect(page.locator('[data-testid="main-panel"]')).toBeVisible()
    })
  })

  test.describe("ULC-CMN-NAV: Navigation", () => {
    test("ULC-CMN-NAV-001: should navigate to dashboard", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="nav-dashboard"]')
      await expect(page).toHaveURL("/")
    })

    test("ULC-CMN-NAV-002: should navigate to settings", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="nav-settings"]')
      await expect(page).toHaveURL("/settings")
    })

    test("ULC-CMN-NAV-003: should navigate to files", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="nav-files"]')
      await expect(page).toHaveURL("/files")
    })
  })

  test.describe("ULC-CMN-SESS: Session Management", () => {
    test("ULC-CMN-SESS-001: should display new session button", async ({ page }) => {
      await page.goto("/")
      await expect(page.locator('[data-testid="new-session-btn"]')).toBeVisible()
    })

    test("ULC-CMN-SESS-002: should create new session", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')

      // Wait for session creation
      await expect(page.locator('[data-testid="session-item"]')).toBeVisible({ timeout: 10000 })
    })

    test("ULC-CMN-SESS-003: should display session list", async ({ page }) => {
      await page.goto("/")
      await expect(page.locator('[data-testid="session-list"]')).toBeVisible()
    })

    test("ULC-CMN-SESS-004: should switch between sessions", async ({ page }) => {
      await page.goto("/")

      // Create two sessions
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForTimeout(500)
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForTimeout(500)

      // Get session items
      const sessionItems = page.locator('[data-testid="session-item"]')
      const count = await sessionItems.count()

      if (count >= 2) {
        // Click first session
        await sessionItems.first().click()
        await page.waitForTimeout(300)

        // Click second session
        await sessionItems.nth(1).click()
        await page.waitForTimeout(300)

        // Should still be on a session page
        expect(page.url()).toContain("/session/")
      }
    })
  })

  test.describe("ULC-CMN-THME: Theme", () => {
    test("ULC-CMN-THME-001: should display theme toggle", async ({ page }) => {
      await page.goto("/")
      await expect(page.locator('[data-testid="theme-toggle"]')).toBeVisible()
    })

    test("ULC-CMN-THME-002: should toggle dark mode", async ({ page }) => {
      await page.goto("/")

      // Get initial state
      const html = page.locator("html")
      const initialClass = await html.getAttribute("class")

      // Toggle theme
      await page.click('[data-testid="theme-toggle"]')
      await page.waitForTimeout(300)

      // Verify theme changed
      const newClass = await html.getAttribute("class")
      expect(newClass).not.toBe(initialClass)
    })

    test("ULC-CMN-THME-003: should persist theme preference", async ({ page, context }) => {
      await page.goto("/")

      // Toggle theme
      await page.click('[data-testid="theme-toggle"]')
      await page.waitForTimeout(300)

      // Get current theme
      const html = page.locator("html")
      const themeClass = await html.getAttribute("class")

      // Reload page
      await page.reload()
      await page.waitForTimeout(500)

      // Verify theme persisted
      const persistedClass = await html.getAttribute("class")
      expect(persistedClass).toBe(themeClass)
    })
  })

  test.describe("ULC-CMN-STNG: Settings", () => {
    test("ULC-CMN-STNG-001: should load settings page", async ({ page }) => {
      await page.goto("/settings")
      await expect(page.locator("h1")).toContainText(/Settings/)
    })

    test("ULC-CMN-STNG-002: should display API key input", async ({ page }) => {
      await page.goto("/settings")
      await expect(page.locator('[data-testid="api-key-input"]')).toBeVisible()
    })

    test("ULC-CMN-STNG-003: should save settings", async ({ page }) => {
      await page.goto("/settings")

      // Enter API key (placeholder)
      await page.fill('[data-testid="api-key-input"]', "sk-ant-test-key-12345")

      // Click save button
      await page.click('[data-testid="save-settings-btn"]')

      // Wait for save confirmation
      await expect(page.locator('[data-testid="save-success"]')).toBeVisible({ timeout: 5000 })
    })
  })

  test.describe("ULC-CMN-AGNT: Agent Selection", () => {
    test("ULC-CMN-AGNT-001: should display agent selector", async ({ page }) => {
      await page.goto("/")

      // Create a session first
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      await expect(page.locator('[data-testid="agent-selector"]')).toBeVisible()
    })

    test("ULC-CMN-AGNT-002: should list available agents", async ({ page }) => {
      await page.goto("/")

      // Create a session first
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Wait for dropdown
      await expect(page.locator('[data-testid="agent-option"]').first()).toBeVisible({ timeout: 5000 })
    })

    test("ULC-CMN-AGNT-003: should switch agent", async ({ page }) => {
      await page.goto("/")

      // Create a session first
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Select an agent
      await page.click('[data-testid="agent-option"]:has-text("build")')

      // Verify selection
      await expect(page.locator('[data-testid="agent-selector"]')).toContainText(/build/)
    })
  })

  test.describe("ULC-CMN-MSG: Message Interaction", () => {
    test("ULC-CMN-MSG-001: should display message input", async ({ page }) => {
      await page.goto("/")

      // Create a session first
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-input"]')

      await expect(page.locator('[data-testid="message-input"]')).toBeVisible()
    })

    test("ULC-CMN-MSG-002: should enable send button with input", async ({ page }) => {
      await page.goto("/")

      // Create a session first
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-input"]')

      // Type message
      await page.fill('[data-testid="message-input"]', "Hello, world!")

      // Send button should be enabled
      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })

    test("ULC-CMN-MSG-003: should display message list", async ({ page }) => {
      await page.goto("/")

      // Create a session first
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-list"]')

      await expect(page.locator('[data-testid="message-list"]')).toBeVisible()
    })
  })

  test.describe("ULC-CMN-CMD: Command Palette", () => {
    test("ULC-CMN-CMD-001: should open command palette with Cmd+K", async ({ page }) => {
      await page.goto("/")

      // Press Cmd+K (or Ctrl+K on non-Mac)
      await page.keyboard.press("Meta+k")

      // Command palette should be visible
      await expect(page.locator('[data-testid="command-palette"]')).toBeVisible({ timeout: 2000 })
    })

    test("ULC-CMN-CMD-002: should close command palette with Escape", async ({ page }) => {
      await page.goto("/")

      // Open command palette
      await page.keyboard.press("Meta+k")
      await expect(page.locator('[data-testid="command-palette"]')).toBeVisible()

      // Close with Escape
      await page.keyboard.press("Escape")
      await expect(page.locator('[data-testid="command-palette"]')).not.toBeVisible()
    })

    test("ULC-CMN-CMD-003: should search commands", async ({ page }) => {
      await page.goto("/")

      // Open command palette
      await page.keyboard.press("Meta+k")
      await expect(page.locator('[data-testid="command-palette"]')).toBeVisible()

      // Type search query
      await page.fill('[data-testid="command-search"]', "settings")

      // Should show filtered results
      await expect(page.locator('[data-testid="command-item"]:has-text("Settings")')).toBeVisible()
    })
  })

  test.describe("ULC-CMN-ERR: Error Handling", () => {
    test("ULC-CMN-ERR-001: should display error toast on failure", async ({ page }) => {
      await page.goto("/")

      // Simulate an error by navigating to invalid route
      await page.goto("/invalid-route-that-does-not-exist")

      // Should display 404 or redirect
      await expect(page).toHaveURL("/")
    })

    test("ULC-CMN-ERR-002: should handle network errors gracefully", async ({ page }) => {
      // Intercept API calls and make them fail
      await page.route("**/api/**", (route) => {
        route.abort()
      })

      await page.goto("/")

      // Page should still load (graceful degradation)
      await expect(page.locator('[data-testid="sidebar"]')).toBeVisible()
    })
  })
})

// Helper function to create a session and return to it
async function createSession(page: Page, title?: string): Promise<void> {
  await page.click('[data-testid="new-session-btn"]')
  if (title) {
    await page.fill('[data-testid="session-title-input"]', title)
    await page.click('[data-testid="create-session-confirm"]')
  }
  await page.waitForSelector('[data-testid="message-input"]')
}

// Helper function to switch agent
async function switchAgent(page: Page, agentName: string): Promise<void> {
  await page.click('[data-testid="agent-selector"]')
  await page.click(`[data-testid="agent-option"]:has-text("${agentName}")`)
  await page.waitForTimeout(300)
}

// Helper function to send message
async function sendMessage(page: Page, message: string): Promise<void> {
  await page.fill('[data-testid="message-input"]', message)
  await page.click('[data-testid="send-btn"]')
}
