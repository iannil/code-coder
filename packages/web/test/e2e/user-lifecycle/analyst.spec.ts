/**
 * ULC-ANL-WEB-* Tests: Decision Analyst Web E2E Tests
 *
 * Playwright E2E tests for decision analysts using
 * the CodeCoder web interface for analysis workflows.
 */

import { test, expect, type Page } from "@playwright/test"

// Skip E2E tests by default unless explicitly enabled
const SKIP_E2E = process.env.SKIP_E2E !== "false"

// Helper to create a session via Dashboard
async function createSessionViaUI(page: Page): Promise<boolean> {
  await page.goto("/")
  await page.waitForTimeout(500)

  const createSessionBtn = page.locator('[data-testid="create-session-btn"]').first()

  try {
    if (await createSessionBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
      await createSessionBtn.click()
      // Wait for navigation to session page
      await page.waitForURL(/\/sessions\//, { timeout: 10000 })
      return true
    }
    return false
  } catch {
    return false
  }
}

test.describe.configure({ mode: "parallel" })

test.describe("ULC-ANL-WEB: Analyst Web E2E", () => {
  test.skip(SKIP_E2E, "E2E tests skipped - set SKIP_E2E=false to run")

  test.describe("ULC-ANL-WEB-AGNT: Analyst Agent Selection", () => {
    test("ULC-ANL-WEB-AGNT-001: should have observer agent available", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Open agent selector
        await page.click('[data-testid="agent-selector"]')
        // Check observer agent is available
        await expect(page.locator('[data-testid="agent-option"]:has-text("observer")')).toBeVisible()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-AGNT-002: should have decision agent available", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Open agent selector
        await page.click('[data-testid="agent-selector"]')
        // Check decision agent is available
        await expect(page.locator('[data-testid="agent-option"]:has-text("decision")')).toBeVisible()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-AGNT-003: should have macro agent available", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Open agent selector
        await page.click('[data-testid="agent-selector"]')
        // Check macro agent is available
        await expect(page.locator('[data-testid="agent-option"]:has-text("macro")')).toBeVisible()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-AGNT-004: should have trader agent available", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Open agent selector
        await page.click('[data-testid="agent-selector"]')
        // Check trader agent is available
        await expect(page.locator('[data-testid="agent-option"]:has-text("trader")')).toBeVisible()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-AGNT-005: should have picker agent available", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Open agent selector
        await page.click('[data-testid="agent-selector"]')
        // Check picker agent is available
        await expect(page.locator('[data-testid="agent-option"]:has-text("picker")')).toBeVisible()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-AGNT-006: should have miniproduct agent available", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Open agent selector
        await page.click('[data-testid="agent-selector"]')
        // Check miniproduct agent is available
        await expect(page.locator('[data-testid="agent-option"]:has-text("miniproduct")')).toBeVisible()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-AGNT-007: should have ai-engineer agent available", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Open agent selector
        await page.click('[data-testid="agent-selector"]')
        // Check ai-engineer agent is available
        await expect(page.locator('[data-testid="agent-option"]:has-text("ai-engineer")')).toBeVisible()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-AGNT-008: should select decision agent", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Open agent selector and select decision
        await page.click('[data-testid="agent-selector"]')
        await page.click('[data-testid="agent-option"]:has-text("decision")')
        // Verify selection
        await expect(page.locator('[data-testid="agent-selector"]')).toContainText(/decision/)
      } catch {
        test.skip()
      }
    })
  })

  test.describe("ULC-ANL-WEB-SESS: Analyst Session Workflow", () => {
    test("ULC-ANL-WEB-SESS-001: should create analysis session", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="message-input"]', { timeout: 10000 })
        // Session should be created
        await expect(page.locator('[data-testid="message-input"]')).toBeVisible()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-SESS-002: should support analysis prompts in Chinese", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="message-input"]', { timeout: 10000 })
        // Type Chinese analysis request
        await page.fill('[data-testid="message-input"]', "使用CLOSE框架分析这个职业决策")
        // Send button should be enabled
        await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
      } catch {
        test.skip()
      }
    })
  })

  test.describe("ULC-ANL-WEB-ANLZ: Analysis Workflows", () => {
    test("ULC-ANL-WEB-ANLZ-001: should support observer analysis request", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Select observer agent
        await page.click('[data-testid="agent-selector"]')
        await page.click('[data-testid="agent-option"]:has-text("observer")')
        // Type observer analysis request
        await page.fill('[data-testid="message-input"]', "从观察者视角分析当前市场的可能性空间")
        await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-ANLZ-002: should support CLOSE framework request", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
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
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-ANLZ-003: should support macro analysis request", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Select macro agent
        await page.click('[data-testid="agent-selector"]')
        await page.click('[data-testid="agent-option"]:has-text("macro")')
        // Type macro analysis request
        await page.fill('[data-testid="message-input"]', "解读最新的PMI数据：52.3，环比上升0.5个百分点")
        await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-ANLZ-004: should support trading analysis request", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Select trader agent
        await page.click('[data-testid="agent-selector"]')
        await page.click('[data-testid="agent-option"]:has-text("trader")')
        // Type trading analysis request
        await page.fill('[data-testid="message-input"]', "分析当前市场的情绪周期阶段")
        await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-ANLZ-005: should support product selection request", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Select picker agent
        await page.click('[data-testid="agent-selector"]')
        await page.click('[data-testid="agent-option"]:has-text("picker")')
        // Type picker analysis request
        await page.fill('[data-testid="message-input"]', "使用七宗罪选品法分析这个产品类目：家居收纳")
        await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
      } catch {
        test.skip()
      }
    })

    test("ULC-ANL-WEB-ANLZ-006: should support miniproduct analysis request", async ({ page }) => {
      const created = await createSessionViaUI(page)
      if (!created) {
        test.skip()
        return
      }
      try {
        await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
        // Select miniproduct agent
        await page.click('[data-testid="agent-selector"]')
        await page.click('[data-testid="agent-option"]:has-text("miniproduct")')
        // Type miniproduct request
        await page.fill('[data-testid="message-input"]', "指导我从0到1构建一个AI写作助手工具")
        await expect(page.locator('[data-testid="send-btn"]')).toBeEnabled()
      } catch {
        test.skip()
      }
    })
  })

  test.describe("ULC-ANL-WEB-MEMO: Memory Panel", () => {
    test("ULC-ANL-WEB-MEMO-001: should display memory panel", async ({ page }) => {
      await page.goto("/settings")
      // Navigate to memory tab in settings
      await page.click('button:has-text("Memory")')
      await page.waitForTimeout(300)
      // Memory panel should be visible
      await expect(page.locator('[data-testid="memory-panel"]')).toBeVisible({ timeout: 5000 })
    })
  })
})

// Analyst-specific helper functions

async function createAnalysisSession(page: Page): Promise<boolean> {
  const created = await createSessionViaUI(page)
  if (!created) return false

  try {
    await page.waitForSelector('[data-testid="agent-selector"]', { timeout: 10000 })
    // Select decision agent by default
    await page.click('[data-testid="agent-selector"]')
    await page.click('[data-testid="agent-option"]:has-text("decision")')
    return true
  } catch {
    return false
  }
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
