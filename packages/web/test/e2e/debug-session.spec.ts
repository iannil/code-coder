import { test, expect } from "@playwright/test"

test("debug: check create-session-btn visibility", async ({ page }) => {
  await page.goto("/")
  await page.waitForTimeout(1000)

  // Take a screenshot
  await page.screenshot({ path: "debug-screenshot.png" })

  // Log the page content
  const html = await page.content()
  console.log("Page contains create-session-btn:", html.includes("create-session-btn"))
  console.log("Page contains New Session:", html.includes("New Session"))
  console.log("Page contains Quick Actions:", html.includes("Quick Actions"))

  // Check for the button
  const btn = page.locator('[data-testid="create-session-btn"]')
  const count = await btn.count()
  console.log("create-session-btn count:", count)

  // Also check if Quick Actions section exists
  const quickActions = page.locator('h2:has-text("Quick Actions")')
  const qaCount = await quickActions.count()
  console.log("Quick Actions heading count:", qaCount)

  // Check if loading is finished
  const loading = page.locator('[data-testid="loading"]')
  const loadingCount = await loading.count()
  console.log("Loading indicator count:", loadingCount)

  // Always pass for debugging
  expect(true).toBe(true)
})

test("debug: test session creation with network monitoring", async ({ page }) => {
  // Monitor all API requests
  const apiRequests: string[] = []
  const apiResponses: { url: string; status: number; body: any }[] = []
  const consoleMessages: string[] = []
  const consoleErrors: string[] = []
  const pageErrors: string[] = []

  page.on('request', request => {
    if (request.url().includes('/api/')) {
      apiRequests.push(`${request.method()} ${request.url()}`)
    }
  })

  page.on('response', async response => {
    if (response.url().includes('/api/')) {
      let body: any = null
      try {
        body = await response.json()
      } catch {}
      apiResponses.push({
        url: response.url(),
        status: response.status(),
        body
      })
    }
  })

  // Capture all console messages
  page.on('console', msg => {
    const text = msg.text()
    if (text.includes('[Dashboard]')) {
      consoleMessages.push(`[${msg.type()}] ${text}`)
    }
    if (msg.type() === 'error') {
      consoleErrors.push(text)
    }
  })

  // Capture page errors
  page.on('pageerror', error => {
    pageErrors.push(error.message)
  })

  await page.goto("/")
  await page.waitForTimeout(1500)

  console.log("Initial API requests:", apiRequests)

  const createBtn = page.locator('[data-testid="create-session-btn"]').first()
  console.log("Button visible:", await createBtn.isVisible())

  // Click the button
  console.log("Clicking button...")
  await createBtn.click()

  // Wait for any session creation request
  await page.waitForTimeout(3000)

  console.log("Dashboard logs:", consoleMessages)
  console.log("API requests after click:", apiRequests)
  console.log("Console errors:", consoleErrors)
  console.log("Page errors:", pageErrors)

  // Check current URL
  console.log("Current URL after click:", page.url())

  expect(true).toBe(true)
})
