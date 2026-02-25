/**
 * Chrome DevTools MCP Tool Execution Benchmarks
 *
 * Dynamically discovers and benchmarks all available chrome-devtools-mcp tools.
 */

import path from "path"

interface ToolCallResult {
  name: string
  durationMs: number
  success: boolean
  error?: string
}

interface LatencyStats {
  p50: number
  p95: number
  p99: number
  avg: number
  min: number
  max: number
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0
  const index = Math.ceil((p / 100) * sorted.length) - 1
  return sorted[Math.max(0, index)]
}

function calculateStats(durations: number[]): LatencyStats {
  if (durations.length === 0) {
    return { p50: 0, p95: 0, p99: 0, avg: 0, min: 0, max: 0 }
  }
  const sorted = [...durations].sort((a, b) => a - b)
  const sum = sorted.reduce((a, b) => a + b, 0)
  return {
    p50: percentile(sorted, 50),
    p95: percentile(sorted, 95),
    p99: percentile(sorted, 99),
    avg: sum / sorted.length,
    min: sorted[0],
    max: sorted[sorted.length - 1],
  }
}

// Tool configurations with safe test arguments
const TOOL_TEST_ARGS: Record<string, unknown> = {
  "list_pages": {},
  "select_page": { index: 0 },
  "snapshot": {},
  "screenshot": {},
  "navigate": { url: "https://example.com" },
  "reload": {},
  "new_page": { url: "about:blank" },
  "close_page": {},
  "evaluate_script": { expression: "1+1" },
  "click": { selector: "body" },
  "hover": { selector: "body" },
  "drag": { startSelector: "body", endSelector: "body" },
  "scroll": { direction: "down", amount: 100 },
  "fill": { selector: "input", value: "test" },
  "fill_form": { fields: [] },
  "type": { text: "test" },
  "press_key": { key: "Escape" },
  "select_option": { selector: "select", value: "test" },
  "handle_dialog": { accept: true },
  "wait": { time: 50 },
  "emulate": { device: "iPhone 12" },
  "get_console_message": {},
  "get_network_request": {},
  "read_storage": { type: "localStorage" },
  "write_storage": { type: "localStorage", key: "test", value: "test" },
  "performance_snapshot": {},
  "performance_diagnose": {},
  "styles_diagnose": { selector: "body" },
}

// Tools that are safe to run multiple times without side effects
const SAFE_TOOLS = new Set([
  "list_pages",
  "snapshot",
  "screenshot",
  "evaluate_script",
  "hover",
  "get_console_message",
  "get_network_request",
  "read_storage",
])

// Tools that require a page to be open
const PAGE_REQUIRED_TOOLS = new Set([
  "snapshot",
  "screenshot",
  "evaluate_script",
  "click",
  "hover",
  "scroll",
  "fill",
  "type",
  "get_console_message",
  "get_network_request",
  "read_storage",
  "write_storage",
  "styles_diagnose",
])

async function runBenchmark() {
  console.log("╔════════════════════════════════════════════════════════════════╗")
  console.log("║     Chrome DevTools MCP 完整工具性能基准测试                    ║")
  console.log("╚════════════════════════════════════════════════════════════════╝\n")

  const projectRoot = path.resolve(import.meta.dir, "../../..")

  try {
    const { MCP } = await import("../src/mcp/index")
    const { Instance } = await import("../src/project/instance")

    await Instance.provide({
      directory: projectRoot,
      fn: async () => {
        const status = await MCP.status()
        const chromeStatus = status["chrome-devtools"]

        if (!chromeStatus || chromeStatus.status !== "connected") {
          console.log("❌ chrome-devtools-mcp 未连接")
          console.log("\n请确保 Chrome 浏览器已打开并且 MCP 已配置。")
          return
        }

        console.log("✅ chrome-devtools-mcp 已连接\n")

        const tools = await MCP.tools()
        const chromeTools = Object.entries(tools).filter(([name]) =>
          name.startsWith("chrome-devtools_")
        )

        console.log(`发现 ${chromeTools.length} 个工具:\n`)

        // Group tools by category
        const categories: Record<string, string[]> = {
          "页面管理": [],
          "内容获取": [],
          "DOM交互": [],
          "输入操作": [],
          "开发者工具": [],
          "其他": [],
        }

        for (const [fullName] of chromeTools) {
          const shortName = fullName.replace("chrome-devtools_", "")

          if (["list_pages", "select_page", "new_page", "close_page", "navigate", "reload"].includes(shortName)) {
            categories["页面管理"].push(shortName)
          } else if (["snapshot", "screenshot"].includes(shortName)) {
            categories["内容获取"].push(shortName)
          } else if (["click", "hover", "drag", "scroll", "fill", "fill_form", "select_option"].includes(shortName)) {
            categories["DOM交互"].push(shortName)
          } else if (["type", "press_key", "handle_dialog", "wait"].includes(shortName)) {
            categories["输入操作"].push(shortName)
          } else if (["evaluate_script", "get_console_message", "get_network_request", "read_storage", "write_storage", "performance_snapshot", "performance_diagnose", "styles_diagnose", "emulate"].includes(shortName)) {
            categories["开发者工具"].push(shortName)
          } else {
            categories["其他"].push(shortName)
          }
        }

        for (const [cat, toolList] of Object.entries(categories)) {
          if (toolList.length > 0) {
            console.log(`  ${cat}: ${toolList.join(", ")}`)
          }
        }

        const results: Record<string, ToolCallResult[]> = {}
        const ITERATIONS = 3

        console.log("\n" + "─".repeat(70))
        console.log("开始性能测试 (每个工具测试 " + ITERATIONS + " 次)")
        console.log("─".repeat(70) + "\n")

        for (const [fullName, tool] of chromeTools) {
          const shortName = fullName.replace("chrome-devtools_", "")
          const args = TOOL_TEST_ARGS[shortName] || {}
          const isSafe = SAFE_TOOLS.has(shortName)
          const iterations = isSafe ? ITERATIONS : 1

          process.stdout.write(`  ${shortName.padEnd(25)}`)

          results[shortName] = []

          for (let i = 0; i < iterations; i++) {
            const startTime = performance.now()
            try {
              await (tool as any).execute(args)
              const durationMs = performance.now() - startTime
              results[shortName].push({
                name: shortName,
                durationMs,
                success: true,
              })
            } catch (error) {
              const durationMs = performance.now() - startTime
              results[shortName].push({
                name: shortName,
                durationMs,
                success: false,
                error: error instanceof Error ? error.message.slice(0, 50) : String(error).slice(0, 50),
              })
              break
            }
          }

          const successResults = results[shortName].filter(r => r.success)
          if (successResults.length > 0) {
            const avg = successResults.reduce((a, b) => a + b.durationMs, 0) / successResults.length
            const icon = avg < 50 ? "⚡" : avg < 200 ? "🔶" : avg < 1000 ? "🐢" : "🔴"
            console.log(`${icon} ${avg.toFixed(0).padStart(6)}ms  (${successResults.length}/${results[shortName].length})`)
          } else {
            const err = results[shortName][0]?.error || "unknown"
            console.log(`❌ 失败: ${err}`)
          }
        }

        // Print summary table
        console.log("\n" + "═".repeat(80))
        console.log("性能测试结果汇总")
        console.log("═".repeat(80) + "\n")

        console.log("| 工具 | 平均 | P50 | P95 | Min | Max | 成功率 |")
        console.log("|------|------|-----|-----|-----|-----|--------|")

        const allDurations: number[] = []

        for (const [name, toolResults] of Object.entries(results)) {
          const successResults = toolResults.filter(r => r.success)
          const durations = successResults.map(r => r.durationMs)
          allDurations.push(...durations)

          const stats = calculateStats(durations)
          const rate = `${successResults.length}/${toolResults.length}`

          if (durations.length > 0) {
            console.log(
              `| ${name.padEnd(25)} | ${stats.avg.toFixed(0).padStart(4)}ms | ${stats.p50.toFixed(0).padStart(3)}ms | ${stats.p95.toFixed(0).padStart(3)}ms | ${stats.min.toFixed(0).padStart(3)}ms | ${stats.max.toFixed(0).padStart(3)}ms | ${rate.padStart(5)} |`
            )
          } else {
            console.log(`| ${name.padEnd(25)} | ${"N/A".padStart(6)} | ${"N/A".padStart(5)} | ${"N/A".padStart(5)} | ${"N/A".padStart(5)} | ${"N/A".padStart(5)} | ${rate.padStart(5)} |`)
          }
        }

        // Overall summary
        const overallStats = calculateStats(allDurations)
        const totalCalls = Object.values(results).flat().length
        const successCalls = Object.values(results).flat().filter(r => r.success).length

        console.log("\n" + "═".repeat(80))
        console.log("\n总结:")
        console.log(`  工具数量: ${chromeTools.length}`)
        console.log(`  总调用次数: ${totalCalls}`)
        console.log(`  成功: ${successCalls} (${(successCalls/totalCalls*100).toFixed(0)}%)`)
        console.log(`  失败: ${totalCalls - successCalls}`)

        if (allDurations.length > 0) {
          console.log(`\n  延迟统计:`)
          console.log(`    平均: ${overallStats.avg.toFixed(0)}ms`)
          console.log(`    P50:  ${overallStats.p50.toFixed(0)}ms`)
          console.log(`    P95:  ${overallStats.p95.toFixed(0)}ms`)
          console.log(`    P99:  ${overallStats.p99.toFixed(0)}ms`)
          console.log(`    最小: ${overallStats.min.toFixed(0)}ms`)
          console.log(`    最大: ${overallStats.max.toFixed(0)}ms`)
        }

        // Categorize by speed
        const fast: string[] = []
        const medium: string[] = []
        const slow: string[] = []

        for (const [name, toolResults] of Object.entries(results)) {
          const successResults = toolResults.filter(r => r.success)
          if (successResults.length === 0) continue
          const avg = successResults.reduce((a, b) => a + b.durationMs, 0) / successResults.length
          if (avg < 50) fast.push(name)
          else if (avg < 200) medium.push(name)
          else slow.push(name)
        }

        console.log(`\n  性能分类:`)
        if (fast.length) console.log(`    ⚡ 极快 (<50ms): ${fast.length} 个工具`)
        if (medium.length) console.log(`    🔶 中等 (50-200ms): ${medium.length} 个工具`)
        if (slow.length) console.log(`    🐢 较慢 (>200ms): ${slow.length} 个工具`)

        console.log("\n" + "═".repeat(80))
      },
    })
  } catch (error) {
    console.error("Benchmark failed:", error)
  }
}

if (import.meta.main) {
  runBenchmark()
}

export { runBenchmark as runChromeDevToolsBenchmark }
