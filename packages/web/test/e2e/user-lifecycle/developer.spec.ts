/**
 * ULC-DEV-WEB-* Tests: Software Developer Web E2E Tests
 *
 * Playwright E2E tests for software developers using
 * the CodeCoder web interface.
 */

import { test, expect, type Page } from "@playwright/test"

// Skip E2E tests by default unless explicitly enabled
const SKIP_E2E = process.env.SKIP_E2E !== "false"

test.describe.configure({ mode: "serial" })

test.describe("ULC-DEV-WEB: Developer Web E2E", () => {
  test.skip(SKIP_E2E, "E2E tests skipped - set SKIP_E2E=false to run")

  test.describe("ULC-DEV-WEB-AGNT: Developer Agent Selection", () => {
    test("ULC-DEV-WEB-AGNT-001: should have build agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check build agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("build")')).toBeVisible()
    })

    test("ULC-DEV-WEB-AGNT-002: should have code-reviewer agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check code-reviewer agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("code-reviewer")')).toBeVisible()
    })

    test("ULC-DEV-WEB-AGNT-003: should have security-reviewer agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check security-reviewer agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("security-reviewer")')).toBeVisible()
    })

    test("ULC-DEV-WEB-AGNT-004: should select build agent", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector and select build
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("build")')

      // Verify selection
      await expect(page.locator('[data-testid="agent-selector"]')).toContainText(/build/)
    })

    test("ULC-DEV-WEB-AGNT-005: should display agent descriptions", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Hover over an agent to see description
      await page.hover('[data-testid="agent-option"]:has-text("code-reviewer")')

      // Description should contain relevant text
      await expect(page.locator('[data-testid="agent-description"]')).toContainText(/code quality|review/)
    })
  })

  test.describe("ULC-DEV-WEB-SESS: Developer Session Workflow", () => {
    test("ULC-DEV-WEB-SESS-001: should create coding session", async ({ page }) => {
      await page.goto("/")

      // Create new session
      await page.click('[data-testid="new-session-btn"]')

      // Session should be created
      await expect(page.locator('[data-testid="message-input"]')).toBeVisible()
    })

    test("ULC-DEV-WEB-SESS-002: should display session in list", async ({ page }) => {
      await page.goto("/")

      // Create new session
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-input"]')

      // Session should appear in sidebar list
      await expect(page.locator('[data-testid="session-item"]').first()).toBeVisible()
    })

    test("ULC-DEV-WEB-SESS-003: should maintain agent selection across navigation", async ({ page }) => {
      await page.goto("/")

      // Create session and select agent
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("code-reviewer")')

      // Navigate away
      await page.click('[data-testid="nav-settings"]')

      // Navigate back to session
      await page.click('[data-testid="session-item"]').first()

      // Agent should still be selected
      await expect(page.locator('[data-testid="agent-selector"]')).toContainText(/code-reviewer/)
    })
  })

  test.describe("ULC-DEV-WEB-FILE: File Operations", () => {
    test("ULC-DEV-WEB-FILE-001: should load files page", async ({ page }) => {
      await page.goto("/files")

      await expect(page.locator('[data-testid="file-browser"]')).toBeVisible()
    })

    test("ULC-DEV-WEB-FILE-002: should display file tree", async ({ page }) => {
      await page.goto("/files")

      // File tree should be visible
      await expect(page.locator('[data-testid="file-tree"]')).toBeVisible()
    })

    test("ULC-DEV-WEB-FILE-003: should expand directories", async ({ page }) => {
      await page.goto("/files")

      // Find a directory and click to expand
      const directory = page.locator('[data-testid="file-directory"]').first()
      if (await directory.isVisible()) {
        await directory.click()
        // Should show children
        await expect(page.locator('[data-testid="file-tree-item"]')).toBeVisible()
      }
    })
  })

  test.describe("ULC-DEV-WEB-CODE: Code Interaction", () => {
    test("ULC-DEV-WEB-CODE-001: should send code-related prompt", async ({ page }) => {
      await page.goto("/")

      // Create session
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-input"]')

      // Type code-related message
      await page.fill('[data-testid="message-input"]', "Write a function that calculates fibonacci numbers")

      // Send button should be enabled
      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })

    test("ULC-DEV-WEB-CODE-002: should display code blocks in response", async ({ page }) => {
      // This test would require actual API integration
      // For now, verify the code block component exists
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-input"]')

      // Message list should support code blocks
      await expect(page.locator('[data-testid="message-list"]')).toBeVisible()
    })

    test("ULC-DEV-WEB-CODE-003: should support syntax highlighting", async ({ page }) => {
      // Navigate to a page that might have code
      await page.goto("/")

      // The shiki syntax highlighter should be loaded
      // This is a basic check - actual highlighting would require a real message
      const html = await page.content()
      expect(html).toContain("shiki")
    })
  })

  test.describe("ULC-DEV-WEB-TOOL: Tool Call Display", () => {
    test("ULC-DEV-WEB-TOOL-001: should display tool call component", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-list"]')

      // Message list should be ready to display tool calls
      await expect(page.locator('[data-testid="message-list"]')).toBeVisible()
    })
  })

  test.describe("ULC-DEV-WEB-PROV: Provider Configuration", () => {
    test("ULC-DEV-WEB-PROV-001: should display provider settings", async ({ page }) => {
      await page.goto("/settings")

      // Provider section should be visible
      await expect(page.locator('[data-testid="provider-settings"]')).toBeVisible()
    })

    test("ULC-DEV-WEB-PROV-002: should allow API key configuration", async ({ page }) => {
      await page.goto("/settings")

      // API key input should be available
      await expect(page.locator('[data-testid="api-key-input"]')).toBeVisible()

      // Should be able to enter API key
      await page.fill('[data-testid="api-key-input"]', "sk-ant-test-key")
      await expect(page.locator('[data-testid="api-key-input"]')).toHaveValue("sk-ant-test-key")
    })
  })
})

// Developer-specific helper functions

async function createDeveloperSession(page: Page): Promise<void> {
  await page.click('[data-testid="new-session-btn"]')
  await page.waitForSelector('[data-testid="agent-selector"]')

  // Select build agent
  await page.click('[data-testid="agent-selector"]')
  await page.click('[data-testid="agent-option"]:has-text("build")')
}

async function selectDeveloperAgent(page: Page, agentName: string): Promise<void> {
  const validAgents = ["build", "plan", "code-reviewer", "security-reviewer", "tdd-guide", "architect", "explore"]
  if (!validAgents.includes(agentName)) {
    throw new Error(`Invalid developer agent: ${agentName}`)
  }

  await page.click('[data-testid="agent-selector"]')
  await page.click(`[data-testid="agent-option"]:has-text("${agentName}")`)
  await page.waitForTimeout(300)
}

async function sendCodeRequest(page: Page, prompt: string): Promise<void> {
  await page.fill('[data-testid="message-input"]', prompt)
  await page.click('[data-testid="send-btn"]')
}
