/**
 * ULC-ANL-WEB-* Tests: Decision Analyst Web E2E Tests
 *
 * Playwright E2E tests for decision analysts using
 * the CodeCoder web interface for analysis workflows.
 */

import { test, expect, type Page } from "@playwright/test"

// Skip E2E tests by default unless explicitly enabled
const SKIP_E2E = process.env.SKIP_E2E !== "false"

test.describe.configure({ mode: "serial" })

test.describe("ULC-ANL-WEB: Analyst Web E2E", () => {
  test.skip(SKIP_E2E, "E2E tests skipped - set SKIP_E2E=false to run")

  test.describe("ULC-ANL-WEB-AGNT: Analyst Agent Selection", () => {
    test("ULC-ANL-WEB-AGNT-001: should have observer agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check observer agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("observer")')).toBeVisible()
    })

    test("ULC-ANL-WEB-AGNT-002: should have decision agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check decision agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("decision")')).toBeVisible()
    })

    test("ULC-ANL-WEB-AGNT-003: should have macro agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check macro agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("macro")')).toBeVisible()
    })

    test("ULC-ANL-WEB-AGNT-004: should have trader agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check trader agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("trader")')).toBeVisible()
    })

    test("ULC-ANL-WEB-AGNT-005: should have picker agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check picker agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("picker")')).toBeVisible()
    })

    test("ULC-ANL-WEB-AGNT-006: should have miniproduct agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check miniproduct agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("miniproduct")')).toBeVisible()
    })

    test("ULC-ANL-WEB-AGNT-007: should have ai-engineer agent available", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector
      await page.click('[data-testid="agent-selector"]')

      // Check ai-engineer agent is available
      await expect(page.locator('[data-testid="agent-option"]:has-text("ai-engineer")')).toBeVisible()
    })

    test("ULC-ANL-WEB-AGNT-008: should select decision agent", async ({ page }) => {
      await page.goto("/")
      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Open agent selector and select decision
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("decision")')

      // Verify selection
      await expect(page.locator('[data-testid="agent-selector"]')).toContainText(/decision/)
    })
  })

  test.describe("ULC-ANL-WEB-SESS: Analyst Session Workflow", () => {
    test("ULC-ANL-WEB-SESS-001: should create analysis session", async ({ page }) => {
      await page.goto("/")

      // Create new session
      await page.click('[data-testid="new-session-btn"]')

      // Session should be created
      await expect(page.locator('[data-testid="message-input"]')).toBeVisible()
    })

    test("ULC-ANL-WEB-SESS-002: should support analysis prompts in Chinese", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="message-input"]')

      // Type Chinese analysis request
      await page.fill('[data-testid="message-input"]', "使用CLOSE框架分析这个职业决策")

      // Send button should be enabled
      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })
  })

  test.describe("ULC-ANL-WEB-ANLZ: Analysis Workflows", () => {
    test("ULC-ANL-WEB-ANLZ-001: should support observer analysis request", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select observer agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("observer")')

      // Type observer analysis request
      await page.fill('[data-testid="message-input"]', "从观察者视角分析当前市场的可能性空间")

      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })

    test("ULC-ANL-WEB-ANLZ-002: should support CLOSE framework request", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select decision agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("decision")')

      // Type CLOSE framework request
      const prompt = `使用CLOSE五维评估框架分析这个决策：

背景：考虑是否换工作
选项A：留在当前公司
选项B：跳槽到新公司

请从C/L/O/S/E五个维度进行分析`

      await page.fill('[data-testid="message-input"]', prompt)
      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })

    test("ULC-ANL-WEB-ANLZ-003: should support macro analysis request", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select macro agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("macro")')

      // Type macro analysis request
      await page.fill('[data-testid="message-input"]', "解读最新的PMI数据：52.3，环比上升0.5个百分点")

      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })

    test("ULC-ANL-WEB-ANLZ-004: should support trading analysis request", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select trader agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("trader")')

      // Type trading analysis request
      await page.fill('[data-testid="message-input"]', "分析当前市场的情绪周期阶段")

      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })

    test("ULC-ANL-WEB-ANLZ-005: should support product selection request", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select picker agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("picker")')

      // Type picker analysis request
      await page.fill('[data-testid="message-input"]', "使用七宗罪选品法分析这个产品类目：家居收纳")

      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })

    test("ULC-ANL-WEB-ANLZ-006: should support miniproduct analysis request", async ({ page }) => {
      await page.goto("/")

      await page.click('[data-testid="new-session-btn"]')
      await page.waitForSelector('[data-testid="agent-selector"]')

      // Select miniproduct agent
      await page.click('[data-testid="agent-selector"]')
      await page.click('[data-testid="agent-option"]:has-text("miniproduct")')

      // Type miniproduct request
      await page.fill('[data-testid="message-input"]', "指导我从0到1构建一个AI写作助手工具")

      await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
    })
  })

  test.describe("ULC-ANL-WEB-MEMO: Memory Panel", () => {
    test("ULC-ANL-WEB-MEMO-001: should display memory panel", async ({ page }) => {
      await page.goto("/")

      // Look for memory panel or related UI element
      // Memory panel might be in sidebar or a dedicated page
      const memoryPanel = page.locator('[data-testid="memory-panel"]')
      const memorySidebarItem = page.locator('[data-testid="nav-memory"]')

      // Either panel or navigation item should exist
      const panelExists = await memoryPanel.isVisible().catch(() => false)
      const navExists = await memorySidebarItem.isVisible().catch(() => false)

      expect(panelExists || navExists).toBeTruthy()
    })
  })
})

// Analyst-specific helper functions

async function createAnalysisSession(page: Page): Promise<void> {
  await page.click('[data-testid="new-session-btn"]')
  await page.waitForSelector('[data-testid="agent-selector"]')

  // Select decision agent by default
  await page.click('[data-testid="agent-selector"]')
  await page.click('[data-testid="agent-option"]:has-text("decision")')
}

async function selectAnalystAgent(page: Page, agentName: string): Promise<void> {
  const validAgents = ["observer", "decision", "macro", "trader", "picker", "miniproduct", "ai-engineer"]
  if (!validAgents.includes(agentName)) {
    throw new Error(`Invalid analyst agent: ${agentName}`)
  }

  await page.click('[data-testid="agent-selector"]')
  await page.click(`[data-testid="agent-option"]:has-text("${agentName}")`)
  await page.waitForTimeout(300)
}

async function sendAnalysisRequest(page: Page, prompt: string): Promise<void> {
  await page.fill('[data-testid="message-input"]', prompt)
  await page.click('[data-testid="send-btn"]')
}
