/**
 * Puppeteer 脚本：打开携程机票页面
 *
 * 功能：
 * 1. 打开浏览器（可见模式）
 * 2. 访问携程机票页面
 * 3. 等待页面加载完成
 * 4. 保持浏览器打开供用户手动操作
 *
 * 运行方式：
 * bun run script/puppeteer-ctrip.ts
 */

import puppeteer from "puppeteer-core"

const CTRIP_FLIGHTS_URL = "https://flights.ctrip.com"

// macOS Chrome 路径
const CHROME_PATH = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

async function main() {
  console.log("🚀 启动浏览器...")

  const browser = await puppeteer.launch({
    headless: false, // 可见模式，方便用户手动操作
    defaultViewport: null, // 使用默认视口大小
    executablePath: CHROME_PATH, // 使用系统 Chrome
    args: [
      "--start-maximized", // 最大化窗口
      "--disable-blink-features=AutomationControlled", // 隐藏自动化特征
    ],
  })

  console.log("✅ 浏览器已启动")

  const page = await browser.newPage()

  // 设置 User-Agent，避免被检测为爬虫
  await page.setUserAgent(
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
  )

  console.log(`🌐 正在访问: ${CTRIP_FLIGHTS_URL}`)

  // 访问页面
  await page.goto(CTRIP_FLIGHTS_URL, {
    waitUntil: "networkidle2", // 等待网络空闲
    timeout: 60000, // 60秒超时
  })

  console.log("✅ 页面加载完成")

  // 等待主要内容区域出现
  try {
    await page.waitForSelector(".flight-search, .search-form, body", {
      timeout: 10000,
    })
    console.log("✅ 主要内容已加载")
  } catch {
    console.log("⚠️  未检测到特定的内容区域，但页面已加载")
  }

  console.log("")
  console.log("=".repeat(50))
  console.log("📌 浏览器保持打开状态，您可以手动操作")
  console.log("📌 按 Ctrl+C 退出程序并关闭浏览器")
  console.log("=".repeat(50))
  console.log("")

  // 保持浏览器打开，直到用户手动终止
  // 监听浏览器关闭事件
  browser.on("disconnected", () => {
    console.log("👋 浏览器已关闭")
    process.exit(0)
  })

  // 保持进程运行
  await new Promise<void>(() => {
    // 无限等待，直到用户按 Ctrl+C
  })
}

// 处理退出信号
process.on("SIGINT", async () => {
  console.log("\n🛑 正在关闭...")
  process.exit(0)
})

main().catch((error) => {
  console.error("❌ 发生错误:", error)
  process.exit(1)
})
