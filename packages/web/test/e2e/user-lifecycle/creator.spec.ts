/**
 * ULC-CRT-WEB-* Tests: Content Creator Web E2E Tests
 *
 * Playwright E2E tests for content creators using
 * the CodeCoder web interface for writing workflows.
 */

import { test, expect, type Page } from "@playwright/test"

// Skip E2E tests by default unless explicitly enabled
const SKIP_E2E = process.env.SKIP_E2E !== "false"

test.describe.configure({ mode: "serial" })

test.describe("ULC-CRT-WEB: Creator Web E2E", () => {
  test.skip(SKIP_E2E, "E2E tests skipped - set SKIP_E2E=false to run")

  test.describe("ULC-CRT-WEB-AGNT: Creator Agent Selection", () => {
    test("ULC-CRT-WEB-AGNT-001: should have writer agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check writer agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("writer")')).toBeVisible()
    })

    test("ULC-CRT-WEB-AGNT-002: should have proofreader agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check proofreader agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("proofreader")')).toBeVisible()
    })

    test("ULC-CRT-WEB-AGNT-003: should have expander agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check expander agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("expander")')).toBeVisible()
    })

    test("ULC-CRT-WEB-AGNT-004: should select writer agent", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector and select writer
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("writer")')

      // Verify selection
      await expect(page.locator('[data-testid="agent-selector"]')).toContainText(/writer/)
    })

    test("ULC-CRT-WEB-AGNT-005: should display writer description", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Hover over writer agent
      await page.hover('[data-testid="agent-option"]:has-text("writer")')

      // Description should mention long-form content
      await expect(page.locator('[data-testid="agent-description"]')).toContainText(/long-form|20k/)
    })
  })

  test.describe("ULC-CRT-WEB-SESS: Creator Session Workflow", () => {
    test("ULC-CRT-WEB-SESS-001: should create writing session", async ({ page }) => {
      await page.goto("/")

      // Create new session
      await page.click('[data-testid="new-session-btn"]')

      // Session should be created
      await expect(page.locator('[data-testid="message-input"]')).toBeVisible()
    })

    test("ULC-CRT-WEB-SESS-002: should support long-form content input", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-input"]')

      // Input should support multi-line text
      const longText = `Create an outline for a book about:

Title: The Art of Creative Writing

Part 1: Foundations
- Chapter 1: Understanding Narrative Structure
- Chapter 2: Character Development

Part 2: Advanced Techniques
- Chapter 3: World Building
- Chapter 4: Dialogue and Voice`

      await page.fill('[data-testid="message-input"]', longText)

      // Verify text was entered
      const inputValue = await page.inputValue('[data-testid="message-input"]')
      expect(inputValue).toContain("The Art of Creative Writing")
    })

    test("ULC-CRT-WEB-SESS-003: should preserve session content", async ({ page }) => {
      await page.goto("/")

      // Create session
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-input"]')

      // Navigate away
      await page.click('[data-testid="nav-settings"]')

      // Navigate back
      await page.click('[data-testid="session-item"]').first()

      // Session should still be accessible
      await expect(page.locator('[data-testid="message-input"]')).toBeVisible()
    })
  })

  test.describe("ULC-CRT-WEB-DOCS: Document Handling", () => {
    test("ULC-CRT-WEB-DOCS-001: should load documents page", async ({ page }) => {
      await page.goto("/documents")

      // Documents page should load
      await expect(page.locator('[data-testid="documents-panel"]')).toBeVisible()
    })

    test("ULC-CRT-WEB-DOCS-002: should display document list", async ({ page }) => {
      await page.goto("/documents")

      // Document list should be visible
      await expect(page.locator('[data-testid="document-list"]')).toBeVisible()
    })
  })

  test.describe("ULC-CRT-WEB-WRITE: Writing Workflow", () => {
    test("ULC-CRT-WEB-WRITE-001: should support outline generation prompt", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select writer agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("writer")')

      // Type outline request
      await page.fill('[data-testid="message-input"]', "Generate an outline for a mystery novel set in 1920s Paris")

      // Send button should be enabled
      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })

    test("ULC-CRT-WEB-WRITE-002: should support chapter writing prompt", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select writer agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("writer")')

      // Type chapter writing request
      const prompt = `Write Chapter 1 based on this outline:
# The Parisian Mystery
## Chapter 1: The Arrival
- Protagonist arrives in Paris
- First encounter with the mystery
- Introduction of key characters`

      await page.fill('[data-testid="message-input"]', prompt)
      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })

    test("ULC-CRT-WEB-WRITE-003: should support proofreading request", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select proofreader agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("proofreader")')

      // Type proofreading request
      await page.fill('[data-testid="message-input"]', "Please proofread the following chapter for grammar and style...")

      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })
  })

  test.describe("ULC-CRT-WEB-EXPD: Content Expansion", () => {
    test("ULC-CRT-WEB-EXPD-001: should have expander-fiction agent", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check expander-fiction agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("expander-fiction")')).toBeVisible()
    })

    test("ULC-CRT-WEB-EXPD-002: should have expander-nonfiction agent", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check expander-nonfiction agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("expander-nonfiction")')).toBeVisible()
    })

    test("ULC-CRT-WEB-EXPD-003: should support content expansion prompt", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select expander agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("expander")')

      // Type expansion request
      await page.fill('[data-testid="message-input"]', "Expand this core idea into a full chapter: The importance of creative constraints in writing")

      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })
  })
})

// Creator-specific helper functions

async function createWritingSession(page: Page): Promise<void> {
  await page.click('[data-testid="new-session-btn"]')
  await page.waitForSelector('[data-testid="agent-selector"]')

  // Select writer agent
  await page.click('[data-testid="agent-selector"]')
  await page.click('[data-testid="agent-option"]:has-text("writer")')
}

async function selectCreatorAgent(page: Page, agentName: string): Promise<void> {
  const validAgents = ["writer", "proofreader", "expander", "expander-fiction", "expander-nonfiction"]
  if (!validAgents.includes(agentName)) {
    throw new Error(`Invalid creator agent: ${agentName}`)
  }

  await page.click('[data-testid="agent-selector"]')
  await page.click(`[data-testid="agent-option"]:has-text("${agentName}")`)
  await page.waitForTimeout(300)
}

async function sendWritingRequest(page: Page, prompt: string): Promise<void> {
  await page.fill('[data-testid="message-input"]', prompt)
  await page.click('[data-testid="send-btn"]')
}
