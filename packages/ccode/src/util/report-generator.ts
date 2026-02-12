/**
 * Report Generator for Code-Reverse Agent
 *
 * Generates structured markdown reports and TUI-compatible data structures
 * from website reverse engineering analysis.
 */

import { Global } from "@/global"
import path from "path"
import { mkdir } from "node:fs/promises"

export interface TechnologyStack {
  frontend: {
    framework?: string
    version?: string
    uiLibrary?: string
    stateManagement?: string
    buildTool?: string
    styling?: string
  }
  backend?: {
    framework?: string
    apiStyle?: string
    apiBaseUrl?: string
  }
  infrastructure: {
    hosting?: string
    cdn?: string[]
    analytics?: string[]
    monitoring?: string[]
  }
}

export interface DesignSystem {
  colors: {
    primary?: string[]
    secondary?: string[]
    accent?: string[]
    background?: string[]
    text?: string[]
    borders?: string[]
  }
  typography: {
    headings?: { family?: string; sizes?: string[]; weights?: string[] }
    body?: { family?: string; sizes?: string[]; lineHeight?: string }
    code?: { family?: string }
  }
  layout: {
    containerMaxWidth?: string
    spacingScale?: string[]
    breakpoints?: { sm?: string; md?: string; lg?: string; xl?: string }
  }
  tokens: {
    borderRadius?: string[]
    shadows?: string[]
    transitions?: string
  }
}

export interface ApiEndpoint {
  method: string
  path: string
  description?: string
  response?: string
  auth?: boolean
}

export interface ComponentInfo {
  name: string
  purpose: string
  props?: string[]
  state?: string[]
  children?: string[]
}

export interface DevelopmentPhase {
  name: string
  tasks: string[]
  estimatedTime?: string
}

export interface ReverseAnalysisReport {
  url: string
  executiveSummary: string
  techStack: TechnologyStack
  designSystem: DesignSystem
  components: ComponentInfo[]
  apiEndpoints: ApiEndpoint[]
  phases: DevelopmentPhase[]
  fileStructure?: string
  totalEstimatedTime?: string
  notes?: string
}

export interface TuiData {
  summary: {
    url: string
    complexity: "simple" | "medium" | "complex"
    estimatedTime: string
  }
  techStack: {
    frontend: { name: string; icon?: string }[]
    backend: { name: string; icon?: string }[]
    infrastructure: { name: string; icon?: string }[]
  }
  designPreview: {
    colors: { hex: string; name?: string }[]
    fonts: { name: string; usage: string }[]
  }
  components: {
    name: string
    complexity: "low" | "medium" | "high"
    estimatedTime: string
  }[]
  apis: {
    method: string
    path: string
    auth: boolean
  }[]
  phases: {
    name: string
    taskCount: number
    estimatedTime: string
  }[]
}

export namespace ReportGenerator {
  const REPORTS_DIR = path.join(Global.Path.data, "reports", "reverse")

  /**
   * Ensure reports directory exists
   */
  async function ensureDir(): Promise<void> {
    await mkdir(REPORTS_DIR, { recursive: true })
  }

  /**
   * Generate a filename for the report
   */
  export function generateFilename(url: string): string {
    const hostname = new URL(url).hostname.replace(/^www\./, "")
    const date = new Date().toISOString().split("T")[0]
    const safeHostname = hostname.replace(/[^a-zA-Z0-9-]/g, "-")
    return `${safeHostname}-${date}.md`
  }

  /**
   * Generate markdown report from analysis data
   */
  export async function generateMarkdown(
    report: ReverseAnalysisReport,
  ): Promise<{ content: string; filepath: string }> {
    await ensureDir()
    const filename = generateFilename(report.url)
    const filepath = path.join(REPORTS_DIR, filename)

    const lines: string[] = []

    // Header
    lines.push(`# Website Reverse Analysis: ${report.url}`)
    lines.push()
    lines.push(`**Generated:** ${new Date().toISOString()}`)
    lines.push()

    // Executive Summary
    lines.push("## Executive Summary")
    lines.push()
    lines.push(report.executiveSummary || "No summary provided.")
    lines.push()

    // Technology Stack
    lines.push("## Technology Stack")
    lines.push()

    lines.push("### Frontend")
    if (report.techStack.frontend.framework) {
      lines.push(`- **Framework:** ${report.techStack.frontend.framework}${report.techStack.frontend.version ? ` ${report.techStack.frontend.version}` : ""}`)
    }
    if (report.techStack.frontend.uiLibrary) {
      lines.push(`- **UI Library:** ${report.techStack.frontend.uiLibrary}`)
    }
    if (report.techStack.frontend.stateManagement) {
      lines.push(`- **State Management:** ${report.techStack.frontend.stateManagement}`)
    }
    if (report.techStack.frontend.buildTool) {
      lines.push(`- **Build Tool:** ${report.techStack.frontend.buildTool}`)
    }
    if (report.techStack.frontend.styling) {
      lines.push(`- **Styling:** ${report.techStack.frontend.styling}`)
    }
    lines.push()

    if (report.techStack.backend) {
      lines.push("### Backend (Inferred)")
      if (report.techStack.backend.framework) {
        lines.push(`- **Framework:** ${report.techStack.backend.framework}`)
      }
      if (report.techStack.backend.apiStyle) {
        lines.push(`- **API Style:** ${report.techStack.backend.apiStyle}`)
      }
      if (report.techStack.backend.apiBaseUrl) {
        lines.push(`- **API Base URL:** ${report.techStack.backend.apiBaseUrl}`)
      }
      lines.push()
    }

    lines.push("### Infrastructure")
    if (report.techStack.infrastructure.hosting) {
      lines.push(`- **Hosting:** ${report.techStack.infrastructure.hosting}`)
    }
    if (report.techStack.infrastructure.cdn?.length) {
      lines.push(`- **CDN:** ${report.techStack.infrastructure.cdn.join(", ")}`)
    }
    if (report.techStack.infrastructure.analytics?.length) {
      lines.push(`- **Analytics:** ${report.techStack.infrastructure.analytics.join(", ")}`)
    }
    if (report.techStack.infrastructure.monitoring?.length) {
      lines.push(`- **Monitoring:** ${report.techStack.infrastructure.monitoring.join(", ")}`)
    }
    lines.push()

    // Design System
    lines.push("## Design System")
    lines.push()

    lines.push("### Colors")
    if (report.designSystem.colors.primary?.length) {
      lines.push(`- **Primary:** ${report.designSystem.colors.primary.join(", ")}`)
    }
    if (report.designSystem.colors.secondary?.length) {
      lines.push(`- **Secondary:** ${report.designSystem.colors.secondary.join(", ")}`)
    }
    if (report.designSystem.colors.accent?.length) {
      lines.push(`- **Accent:** ${report.designSystem.colors.accent.join(", ")}`)
    }
    if (report.designSystem.colors.background?.length) {
      lines.push(`- **Background:** ${report.designSystem.colors.background.join(", ")}`)
    }
    if (report.designSystem.colors.text?.length) {
      lines.push(`- **Text:** ${report.designSystem.colors.text.join(", ")}`)
    }
    if (report.designSystem.colors.borders?.length) {
      lines.push(`- **Borders:** ${report.designSystem.colors.borders.join(", ")}`)
    }
    lines.push()

    lines.push("### Typography")
    if (report.designSystem.typography.headings) {
      const h = report.designSystem.typography.headings
      lines.push(`- **Headings:** ${h.family || "Sans-serif"}${h.sizes ? ` (${h.sizes.join(", ")})` : ""}${h.weights ? ` weights: ${h.weights.join(", ")}` : ""}`)
    }
    if (report.designSystem.typography.body) {
      const b = report.designSystem.typography.body
      lines.push(`- **Body:** ${b.family || "Sans-serif"}${b.sizes ? ` (${b.sizes.join(", ")})` : ""}${b.lineHeight ? ` line-height: ${b.lineHeight}` : ""}`)
    }
    if (report.designSystem.typography.code) {
      lines.push(`- **Code:** ${report.designSystem.typography.code.family}`)
    }
    lines.push()

    lines.push("### Layout & Spacing")
    if (report.designSystem.layout.containerMaxWidth) {
      lines.push(`- **Container max-width:** ${report.designSystem.layout.containerMaxWidth}`)
    }
    if (report.designSystem.layout.spacingScale?.length) {
      lines.push(`- **Spacing scale:** ${report.designSystem.layout.spacingScale.join(" → ")}`)
    }
    if (report.designSystem.layout.breakpoints) {
      const bp = report.designSystem.layout.breakpoints
      lines.push(`- **Breakpoints:** sm: ${bp.sm || "-"}, md: ${bp.md || "-"}, lg: ${bp.lg || "-"}, xl: ${bp.xl || "-"}`)
    }
    lines.push()

    if (report.designSystem.tokens.borderRadius?.length) {
      lines.push(`- **Border radius:** ${report.designSystem.tokens.borderRadius.join(", ")}`)
    }
    if (report.designSystem.tokens.shadows?.length) {
      lines.push(`- **Shadows:** ${report.designSystem.tokens.shadows.join(", ")}`)
    }
    if (report.designSystem.tokens.transitions) {
      lines.push(`- **Transitions:** ${report.designSystem.tokens.transitions}`)
    }
    lines.push()

    // Component Structure
    if (report.components.length > 0) {
      lines.push("## Component Structure")
      lines.push()
      for (const component of report.components) {
        lines.push(`### ${component.name}`)
        lines.push(`- **Purpose:** ${component.purpose}`)
        if (component.props?.length) {
          lines.push(`- **Props:** ${component.props.join(", ")}`)
        }
        if (component.state?.length) {
          lines.push(`- **State:** ${component.state.join(", ")}`)
        }
        if (component.children?.length) {
          lines.push(`- **Children:** ${component.children.join(", ")}`)
        }
        lines.push()
      }
    }

    // API Endpoints
    if (report.apiEndpoints.length > 0) {
      lines.push("## API Endpoints")
      lines.push()
      lines.push("| Method | Endpoint | Description | Auth |")
      lines.push("|--------|----------|-------------|------|")
      for (const endpoint of report.apiEndpoints) {
        lines.push(`| ${endpoint.method} | \`${endpoint.path}\` | ${endpoint.description || "-"} | ${endpoint.auth ? "Yes" : "No"} |`)
      }
      lines.push()
    }

    // Development Plan
    lines.push("## Development Plan")
    lines.push()

    for (const phase of report.phases) {
      lines.push(`### ${phase.name}`)
      if (phase.estimatedTime) {
        lines.push(`*Estimated time: ${phase.estimatedTime}*`)
      }
      lines.push()
      for (const task of phase.tasks) {
        lines.push(`- [ ] ${task}`)
      }
      lines.push()
    }

    // File Structure
    if (report.fileStructure) {
      lines.push("## File Structure")
      lines.push()
      lines.push("```")
      lines.push(report.fileStructure)
      lines.push("```")
      lines.push()
    }

    // Estimated Effort
    if (report.totalEstimatedTime) {
      lines.push("## Estimated Effort")
      lines.push()
      lines.push("| Phase | Time |")
      lines.push("|-------|------|")
      for (const phase of report.phases) {
        lines.push(`| ${phase.name} | ${phase.estimatedTime || "-"} |`)
      }
      lines.push(`| **Total** | **${report.totalEstimatedTime}** |`)
      lines.push()
    }

    // Notes
    if (report.notes) {
      lines.push("## Notes")
      lines.push()
      lines.push(report.notes)
      lines.push()
    }

    return {
      content: lines.join("\n"),
      filepath,
    }
  }

  /**
   * Generate TUI-compatible data structure from analysis
   */
  export function generateTuiData(report: ReverseAnalysisReport): TuiData {
    // Calculate complexity
    const componentCount = report.components.length
    const apiCount = report.apiEndpoints.length
    let complexity: "simple" | "medium" | "complex" = "simple"
    if (componentCount > 10 || apiCount > 5) {
      complexity = "complex"
    } else if (componentCount > 5 || apiCount > 2) {
      complexity = "medium"
    }

    // Get icons for technologies
    const getTechIcon = (name: string): string | undefined => {
      const icons: Record<string, string> = {
        React: "⚛️",
        Vue: "💚",
        Angular: "🅰️",
        Svelte: "🔥",
        Next: "▲",
        Nuxt: "🟢",
        "Tailwind CSS": "🎨",
        "Material-UI": "🧱",
        Vercel: "▲",
        Netlify: "🟢",
        Cloudflare: "☁️",
        AWS: "🟠",
      }
      for (const [key, icon] of Object.entries(icons)) {
        if (name.toLowerCase().includes(key.toLowerCase())) {
          return icon
        }
      }
      return undefined
    }

    const frontendTechs: Array<{ name: string; icon?: string }> = []
    if (report.techStack.frontend.framework) {
      frontendTechs.push({ name: report.techStack.frontend.framework, icon: getTechIcon(report.techStack.frontend.framework) })
    }
    if (report.techStack.frontend.uiLibrary) {
      frontendTechs.push({ name: report.techStack.frontend.uiLibrary, icon: getTechIcon(report.techStack.frontend.uiLibrary) })
    }
    if (report.techStack.frontend.styling) {
      frontendTechs.push({ name: report.techStack.frontend.styling, icon: getTechIcon(report.techStack.frontend.styling) })
    }

    const infraTechs: Array<{ name: string; icon?: string }> = []
    if (report.techStack.infrastructure.hosting) {
      infraTechs.push({ name: report.techStack.infrastructure.hosting, icon: getTechIcon(report.techStack.infrastructure.hosting) })
    }
    for (const cdn of report.techStack.infrastructure.cdn || []) {
      infraTechs.push({ name: cdn, icon: getTechIcon(cdn) })
    }

    return {
      summary: {
        url: report.url,
        complexity,
        estimatedTime: report.totalEstimatedTime || "Unknown",
      },
      techStack: {
        frontend: frontendTechs,
        backend: report.techStack.backend?.framework
          ? [{ name: report.techStack.backend.framework, icon: getTechIcon(report.techStack.backend.framework) }]
          : [],
        infrastructure: infraTechs,
      },
      designPreview: {
        colors: [
          ...(report.designSystem.colors.primary?.map((hex) => ({ hex, name: "primary" })) || []),
          ...(report.designSystem.colors.secondary?.map((hex) => ({ hex, name: "secondary" })) || []),
          ...(report.designSystem.colors.accent?.map((hex) => ({ hex, name: "accent" })) || []),
        ],
        fonts: [
          report.designSystem.typography.headings?.family
            ? { name: report.designSystem.typography.headings.family, usage: "headings" }
            : undefined,
          report.designSystem.typography.body?.family
            ? { name: report.designSystem.typography.body.family, usage: "body" }
            : undefined,
          report.designSystem.typography.code?.family
            ? { name: report.designSystem.typography.code.family, usage: "code" }
            : undefined,
        ].filter((x): x is { name: string; usage: string } => x !== undefined),
      },
      components: report.components.map((c) => ({
        name: c.name,
        complexity: c.props && c.props.length > 5 ? "high" : c.props && c.props.length > 2 ? "medium" : "low",
        estimatedTime: c.props && c.props.length > 5 ? "2-3h" : c.props && c.props.length > 2 ? "1-2h" : "0.5-1h",
      })),
      apis: report.apiEndpoints.map((api) => ({
        method: api.method,
        path: api.path,
        auth: api.auth || false,
      })),
      phases: report.phases.map((p) => ({
        name: p.name,
        taskCount: p.tasks.length,
        estimatedTime: p.estimatedTime || "-",
      })),
    }
  }

  /**
   * Save report to file
   */
  export async function saveReport(report: ReverseAnalysisReport): Promise<string> {
    const { content, filepath } = await generateMarkdown(report)
    await Bun.write(filepath, content)
    return filepath
  }

  /**
   * Parse analysis from structured LLM response
   */
  export function parseAnalysisResponse(response: string): Partial<ReverseAnalysisReport> {
    const result: Partial<ReverseAnalysisReport> = {
      techStack: {
        frontend: {},
        infrastructure: {},
      },
      designSystem: {
        colors: {},
        typography: {},
        layout: {},
        tokens: {},
      },
      components: [],
      apiEndpoints: [],
      phases: [],
    }

    // Extract URL from response
    const urlMatch = response.match(/(?:URL|Target|Website):\s*(https?:\/\/[^\s]+)/i)
    if (urlMatch) {
      result.url = urlMatch[1]
    }

    // Extract executive summary
    const summaryMatch = response.match(/##?\s*Executive Summary\s*\n+(.*?)(?:##|\n\n|$)/is)
    if (summaryMatch) {
      result.executiveSummary = summaryMatch[1].trim()
    }

    // Extract frontend framework
    const frameworkMatch = response.match(/[-*]\s*\*\*Framework:\*\*\s*([^\n]+)/i)
    if (frameworkMatch) {
      result.techStack!.frontend!.framework = frameworkMatch[1].trim()
    }

    // Extract UI library
    const uiMatch = response.match(/[-*]\s*\*\*UI Library:\*\*\s*([^\n]+)/i)
    if (uiMatch) {
      result.techStack!.frontend!.uiLibrary = uiMatch[1].trim()
    }

    // Extract hosting
    const hostingMatch = response.match(/[-*]\s*\*\*Hosting:\*\*\s*([^\n]+)/i)
    if (hostingMatch) {
      result.techStack!.infrastructure!.hosting = hostingMatch[1].trim()
    }

    // Extract colors
    const colorsMatch = response.match(/[-*]\s*\*\*Primary:\*\*\s*([^\n]+)/i)
    if (colorsMatch) {
      result.designSystem!.colors!.primary = colorsMatch[1].trim().split(/,|、/).map((c) => c.trim())
    }

    // Extract API endpoints from markdown table
    const apiTableMatch = response.match(/\|\s*Method\s*\|\s*Endpoint[^\n]*\|\n\|[-|\s]+\|\n((?:\|[^|\n]*\|\n)+)/i)
    if (apiTableMatch) {
      const rows = apiTableMatch[1].trim().split("\n")
      for (const row of rows) {
        const cells = row.split("|").filter((c) => c.trim())
        if (cells.length >= 2) {
          result.apiEndpoints!.push({
            method: cells[0].trim(),
            path: cells[1].trim().replace(/`/g, ""),
            description: cells[2]?.trim(),
          })
        }
      }
    }

    return result
  }
}
