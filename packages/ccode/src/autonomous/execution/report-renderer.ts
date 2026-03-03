/**
 * Report Renderer
 *
 * Renders research reports to either inline text or files
 * based on report length.
 */

import { Log } from "@/util/log"
import { join } from "path"
import { homedir } from "os"

const log = Log.create({ service: "autonomous.report-renderer" })

export interface ReportData {
  topic: string
  summary: string
  analysis: string
  insights: string[]
  sources: Array<{
    url: string
    title: string
    snippet?: string
    credibility?: "high" | "medium" | "low"
  }>
}

export interface RenderConfig {
  /** Maximum characters for inline return (default: 1000) */
  maxInlineLength?: number
  /** Output directory for file reports */
  outputDir?: string
  /** Filename pattern: {date}-{topic}.md */
  filenamePattern?: string
}

export interface RenderResult {
  mode: "inline" | "file"
  content: string
  filePath?: string
}

const DEFAULT_CONFIG: Required<RenderConfig> = {
  maxInlineLength: 3500, // Telegram limit is 4096, leave room for PDCA summary
  outputDir: join(homedir(), ".codecoder", "workspace", "reports"),
  filenamePattern: "{date}-{topic}.md",
}

/** Format report as Markdown */
function formatReport(data: ReportData): string {
  const sections: string[] = []

  sections.push(`# ${data.topic} 分析报告\n`)
  sections.push(`**生成时间**: ${new Date().toISOString()}`)
  sections.push(`**数据来源**: ${data.sources.length} 个来源\n`)

  sections.push("## 摘要\n")
  sections.push(data.summary + "\n")

  sections.push("## 详细分析\n")
  sections.push(data.analysis + "\n")

  if (data.insights.length > 0) {
    sections.push("## 关键洞察\n")
    data.insights.forEach((insight, i) => {
      sections.push(`${i + 1}. ${insight}`)
    })
    sections.push("")
  }

  if (data.sources.length > 0) {
    sections.push("## 数据来源\n")
    data.sources.forEach((source) => {
      const credIcon = source.credibility === "high" ? "🟢" : source.credibility === "medium" ? "🟡" : "🔴"
      sections.push(`- ${credIcon} [${source.title}](${source.url})`)
    })
    sections.push("")
  }

  sections.push("---")
  sections.push("*由 CodeCoder Research Loop 自动生成*")

  return sections.join("\n")
}

/** Generate filename from pattern */
function generateFilename(pattern: string, topic: string): string {
  const date = new Date().toISOString().split("T")[0]
  const safeTopic = topic
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fa5]+/g, "-")
    .slice(0, 50)
  return pattern.replace("{date}", date).replace("{topic}", safeTopic)
}

/** Generate summary for file mode */
function generateSummary(data: ReportData, filePath: string): string {
  const lines = [
    `📊 **${data.topic}** 分析报告已生成`,
    "",
    `📝 **摘要**: ${data.summary.slice(0, 200)}${data.summary.length > 200 ? "..." : ""}`,
    "",
    `💡 **关键洞察**: ${data.insights.length} 条`,
    `📚 **数据来源**: ${data.sources.length} 个`,
    "",
    `📄 **完整报告**: \`${filePath}\``,
  ]
  return lines.join("\n")
}

/** Main render function */
export async function renderReport(
  data: ReportData,
  config: RenderConfig = {},
): Promise<RenderResult> {
  const cfg = { ...DEFAULT_CONFIG, ...config }
  const report = formatReport(data)

  log.debug("Rendering report", {
    topic: data.topic,
    length: report.length,
    maxInline: cfg.maxInlineLength,
  })

  // Check if report fits inline
  if (report.length <= cfg.maxInlineLength) {
    return {
      mode: "inline",
      content: report,
    }
  }

  // Save to file for archival purposes
  const filename = generateFilename(cfg.filenamePattern, data.topic)
  const filePath = join(cfg.outputDir, filename)

  // Ensure directory exists
  const { mkdir } = await import("fs/promises")
  await mkdir(cfg.outputDir, { recursive: true })

  // Write file
  await Bun.write(filePath, report)

  log.info("Report saved to file", { filePath, length: report.length })

  // Return full content with file reference appended
  // This ensures IM messages show the complete report, not just a summary
  const contentWithFileRef = `${report}\n\n---\n📄 完整报告已保存: \`${filePath}\``

  return {
    mode: "file",
    content: contentWithFileRef,
    filePath,
  }
}

/** Factory function to create a renderer instance */
export function createReportRenderer(config: RenderConfig = {}) {
  const cfg = { ...DEFAULT_CONFIG, ...config }

  return {
    render: (data: ReportData) => renderReport(data, cfg),
    config: cfg,
  }
}

export type ReportRenderer = ReturnType<typeof createReportRenderer>
