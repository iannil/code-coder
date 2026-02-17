import { defineConfig, devices } from "@playwright/test"

/**
 * Playwright E2E Test Configuration for CodeCoder Web
 *
 * @see https://playwright.dev/docs/test-configuration
 */
export default defineConfig({
  testDir: "./test/e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["html", { outputFolder: "playwright-report" }], ["list"]],

  // Global timeout settings
  timeout: 30000,
  expect: {
    timeout: 5000,
  },

  use: {
    // Base URL for navigation (web dev server)
    baseURL: process.env.TEST_BASE_URL || "http://localhost:3000",

    // Collect trace on failure
    trace: "on-first-retry",

    // Screenshot on failure
    screenshot: "only-on-failure",

    // Video recording
    video: "retain-on-failure",
  },

  // Configure projects for different browsers
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "firefox",
      use: { ...devices["Desktop Firefox"] },
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"] },
    },
    // Mobile viewports
    {
      name: "Mobile Chrome",
      use: { ...devices["Pixel 5"] },
    },
    {
      name: "Mobile Safari",
      use: { ...devices["iPhone 12"] },
    },
  ],

  // Web server configuration - disabled for manual server start
  // webServer: {
  //   command: "bun run dev",
  //   url: "http://localhost:5173",
  //   reuseExistingServer: !process.env.CI,
  //   timeout: 120000,
  // },

  // Output directory for test artifacts
  outputDir: "test-results",
})
