import type { Argv } from "yargs"
import { cmd } from "./cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../ui"
import { bootstrap } from "../bootstrap"
import { ReportGenerator, type ReverseAnalysisReport } from "../../util/report-generator"
import { findFingerprints, FINGERPRINTS, getCategories } from "../../util/tech-fingerprints"
import { WebFetchTool } from "../../tool/webfetch"
import path from "path"
import { mkdir } from "node:fs/promises"

// ============================================================================
// Reverse Commands
// ============================================================================

const ReverseAnalyzeCommand = cmd({
  command: "analyze <url>",
  describe: "Analyze a website and generate a pixel-perfect recreation plan",
  builder: (yargs: Argv) => {
    return yargs
      .positional("url", {
        type: "string",
        description: "The URL to analyze",
      })
      .option("output", {
        type: "string",
        alias: "o",
        describe: "Output directory for the report",
      })
      .option("format", {
        type: "string",
        choices: ["markdown", "json", "tui"],
        default: "markdown",
        describe: "Output format",
      })
      .option("interactive", {
        type: "boolean",
        alias: "i",
        default: false,
        describe: "Interactive mode with TUI",
      })
      .option("depth", {
        type: "string",
        choices: ["quick", "standard", "deep"],
        default: "standard",
        describe: "Analysis depth",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const url = args.url as string
      const outputDir = args.output as string | undefined
      const format = args.format as "markdown" | "json" | "tui"
      const interactive = args.interactive as boolean

      prompts.intro("Website Reverse Analysis")

      // Validate URL
      let validatedUrl: URL
      try {
        validatedUrl = new URL(url.startsWith("http") ? url : `https://${url}`)
      } catch {
        prompts.cancel("Invalid URL provided")
        process.exit(1)
      }

      // Initialize tools
      const webFetchInit = await WebFetchTool.init({})
      const fetchSpinner = prompts.spinner()

      try {
        // Step 1: Fetch content
        fetchSpinner.start("Fetching HTML content...")
        const fetchResult = await webFetchInit.execute(
          { url: validatedUrl.href, format: "markdown" },
          {
            sessionID: "reverse",
            messageID: "0",
            agent: "code-reverse",
            abort: new AbortController().signal,
            ask: async () => undefined,
            metadata: () => {},
          },
        )
        fetchSpinner.stop("HTML content fetched")

        if (!fetchResult.output) {
          throw new Error("No content received")
        }

        const html = fetchResult.output as string

        // Step 2: Technology fingerprinting
        const fingerprintSpinner = prompts.spinner()
        fingerprintSpinner.start("Detecting technologies...")
        const fingerprints = findFingerprints(html)
        fingerprintSpinner.stop(`${fingerprints.size} technologies detected`)

        // Step 3: Extract design system
        const designSpinner = prompts.spinner()
        designSpinner.start("Extracting design system...")
        const designSystem = extractDesignSystem(html, validatedUrl.href)
        designSpinner.stop("Design system extracted")

        // Step 4: Generate report
        const reportSpinner = prompts.spinner()
        reportSpinner.start("Generating analysis report...")

        const networkData = analyzeHeaders(html)

        const report: ReverseAnalysisReport = {
          url: validatedUrl.href,
          executiveSummary: generateExecutiveSummary(validatedUrl.href, networkData, fingerprints),
          techStack: {
            frontend: extractFrontendTech(fingerprints),
            infrastructure: {
              hosting: networkData.serverInfo?.type,
              cdn: networkData.serverInfo?.cdn || [],
              analytics: networkData.analytics || [],
              monitoring: [],
            },
          },
          designSystem,
          components: extractComponents(html),
          apiEndpoints: networkData.endpoints || [],
          phases: generateDevelopmentPhases(validatedUrl.href, fingerprints),
          fileStructure: generateFileStructure(fingerprints),
          totalEstimatedTime: estimateTotalTime(fingerprints),
        }

        reportSpinner.stop("Analysis complete")

        // Save report
        if (format === "markdown" || format === "json") {
          const saveSpinner = prompts.spinner()
          saveSpinner.start("Saving report...")

          let outputPath: string

          if (format === "markdown") {
            const { content, filepath } = await ReportGenerator.generateMarkdown(report)
            const reportsDir = outputDir || path.dirname(filepath)
            await mkdir(reportsDir, { recursive: true })
            outputPath = path.join(reportsDir, path.basename(filepath))
            await Bun.write(outputPath, content)
          } else {
            // JSON format
            const reportsDir = outputDir || path.join(process.cwd(), "reports")
            await mkdir(reportsDir, { recursive: true })
            outputPath = path.join(reportsDir, ReportGenerator.generateFilename(validatedUrl.href).replace(".md", ".json"))
            await Bun.write(outputPath, JSON.stringify(report, null, 2))
          }

          saveSpinner.stop(`Report saved to: ${outputPath}`)

          prompts.outro(`Analysis complete! Report saved to: ${outputPath}`)

          if (interactive) {
            showInteractiveSummary(report)
          }
        } else if (format === "tui") {
          showInteractiveSummary(report)
        }
      } catch (error) {
        fetchSpinner.stop("Analysis failed")
        prompts.log.error(`${error instanceof Error ? error.message : String(error)}`)
        prompts.cancel("Analysis failed")
        process.exit(1)
      }
    })
  },
})

const ReverseListCommand = cmd({
  command: "list",
  describe: "List available technology fingerprints",
  builder: (yargs: Argv) => {
    return yargs.option("category", {
      type: "string",
      describe: "Filter by category",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const categoryFilter = args.category as string | undefined

      if (categoryFilter) {
        UI.empty()
        prompts.intro(`Technology Fingerprints: ${categoryFilter}`)
        const techs = FINGERPRINTS[categoryFilter]
        if (!techs) {
          prompts.cancel(`Category "${categoryFilter}" not found`)
          prompts.outro("Available categories: " + getCategories().join(", "))
          process.exit(1)
        }

        for (const tech of techs) {
          prompts.log.info(`${tech.name} ${UI.Style.TEXT_DIM}(${tech.patterns.length} patterns)`)
        }
      } else {
        UI.empty()
        prompts.intro("Technology Fingerprint Categories")
        const categories = getCategories()
        for (const category of categories) {
          const count = FINGERPRINTS[category].length
          prompts.log.info(`${category}: ${count} technologies`)
        }
      }

      prompts.outro("Use --category to view details")
    })
  },
})

// ============================================================================
// Helper Functions
// ============================================================================

function analyzeHeaders(html: string) {
  const networkData: any = {
    serverInfo: {},
    analytics: [],
    endpoints: [],
  }

  // Detect analytics from HTML
  const analyticsPatterns = [
    { name: "Google Analytics", patterns: ["googletagmanager.com", "google-analytics.com", "GA_MEASUREMENT_ID", "gtag("] },
    { name: "Plausible", patterns: ["plausible.io"] },
    { name: "PostHog", patterns: ["posthog"] },
    { name: "Hotjar", patterns: ["static.hotjar.com", "hj"] },
    { name: "FullStory", patterns: ["fullstory.com", "__fs"] },
    { name: "Segment", patterns: ["segment.com", "analytics.js"] },
    { name: "Mixpanel", patterns: ["mixpanel"] },
    { name: "Amplitude", patterns: ["amplitude"] },
    { name: "Heap", patterns: ["heapanalytics.com", "heap"] },
    { name: "Fathom", patterns: ["fathom"] },
    { name: "Umami", patterns: ["umami"] },
    { name: "Clarity", patterns: ["clarity.ms", "hot.js"] },
  ]

  for (const analytic of analyticsPatterns) {
    for (const pattern of analytic.patterns) {
      if (html.includes(pattern)) {
        if (!networkData.analytics.includes(analytic.name)) {
          networkData.analytics.push(analytic.name)
        }
        break
      }
    }
  }

  // Extract API endpoints from script tags and inline scripts
  const apiPatterns = [
    /["']\/api\/[^"']+["']/g,
    /["']https?:\/\/[^"']*\/api\/[^"']*["']/g,
    /["']\/v\d+\/[^"']+["']/g,
    /["']\/graphql["']/g,
  ]

  const foundEndpoints = new Set<string>()
  for (const pattern of apiPatterns) {
    const matches = html.match(pattern)
    if (matches) {
      for (const match of matches) {
        const urlMatch = match.match(/["']([^"']+)["']/)
        if (urlMatch) {
          foundEndpoints.add(urlMatch[1])
        }
      }
    }
  }

  // Convert found endpoints to objects
  for (const endpoint of foundEndpoints) {
    networkData.endpoints.push({
      method: "GET",
      url: endpoint,
      path: endpoint,
      description: "Detected in source code",
    })
  }

  // Detect hosting from HTML
  if (html.includes("vercel")) {
    networkData.serverInfo.type = "Vercel"
    networkData.serverInfo.cdn = ["Vercel Edge"]
  } else if (html.includes("netlify")) {
    networkData.serverInfo.type = "Netlify"
  } else if (html.includes("cloudflare")) {
    networkData.serverInfo.type = "Cloudflare"
    networkData.serverInfo.cdn = ["Cloudflare"]
  }

  return networkData
}

function generateExecutiveSummary(
  url: string,
  networkData: any,
  fingerprints: Map<string, { tech: any; matches: string[] }>,
): string {
  const hostname = new URL(url).hostname
  const detectedTechs = Array.from(fingerprints.keys()).slice(0, 5)
  const hosting = networkData.serverInfo?.type || "Unknown"
  const analytics = networkData.analytics?.length || 0

  return `This analysis covers **${hostname}**, a website hosted on **${hosting}**. ` +
    `The site uses ${detectedTechs.length > 0 ? detectedTechs.join(", ") : "modern web technologies"} ` +
    `and implements ${analytics} analytics/tracking solution${analytics !== 1 ? "s" : ""}. ` +
    `The following report provides a comprehensive breakdown for pixel-perfect recreation.`
}

function extractFrontendTech(fingerprints: Map<string, { tech: any; matches: string[] }>) {
  const frontend: any = {}

  for (const [name, { tech }] of fingerprints) {
    if (tech.category === "framework") {
      frontend.framework = name
    } else if (tech.category === "ui") {
      frontend.uiLibrary = name
    } else if (tech.category === "state") {
      frontend.stateManagement = name
    } else if (tech.category === "build") {
      frontend.buildTool = name
    } else if (tech.category === "styling") {
      frontend.styling = name
    }
  }

  return frontend
}

function extractDesignSystem(html: string, baseUrl: string) {
  const design: any = {
    colors: {},
    typography: {},
    layout: {},
    tokens: {},
  }

  // Extract colors from inline styles and CSS
  const colorMatches = html.match(/#[0-9a-fA-F]{3,6}|rgb\([^)]+\)|rgba\([^)]+\)/g)
  if (colorMatches) {
    const uniqueColors = [...new Set(colorMatches)].slice(0, 20)
    design.colors.primary = uniqueColors.filter((c) => c.includes("#")).slice(0, 5)
    design.colors.text = ["#000000", "#ffffff", "#333333", "#666666"]
    design.colors.background = ["#ffffff", "#f5f5f5", "#fafafa"]
  }

  // Extract fonts
  const fontMatches = html.match(/font-family:\s*[^;]+/gi)
  if (fontMatches) {
    const fonts = fontMatches.map((f) => f.replace(/font-family:\s*/i, "").replace(/['"]/g, "").split(",")[0])
    const uniqueFonts = [...new Set(fonts)]
    design.typography.body = { family: uniqueFonts[0] || "sans-serif", sizes: ["14px", "16px", "18px"] }
    design.typography.headings = { family: uniqueFonts[0] || "sans-serif", sizes: ["24px", "32px", "48px"] }
  }

  // Common layout patterns
  design.layout.spacingScale = ["4px", "8px", "16px", "24px", "32px", "48px"]
  design.layout.breakpoints = { sm: "640px", md: "768px", lg: "1024px", xl: "1280px" }
  design.tokens.borderRadius = ["4px", "8px", "16px"]

  return design
}

function extractComponents(html: string) {
  const components: any[] = []

  // Common semantic components
  if (html.includes("<nav") || html.includes('role="navigation"')) {
    components.push({ name: "Navbar", purpose: "Site navigation" })
  }
  if (html.includes("<footer") || html.includes('role="contentinfo"')) {
    components.push({ name: "Footer", purpose: "Site footer with links and info" })
  }
  if (html.includes("<button")) {
    components.push({ name: "Button", purpose: "Interactive button component", props: ["variant", "size", "disabled"] })
  }
  if (html.includes("<input") || html.includes("<textarea")) {
    components.push({ name: "Input", purpose: "Form input field", props: ["type", "placeholder", "value"] })
  }
  if (html.includes("<article") || html.includes('role="article"')) {
    components.push({ name: "Article", purpose: "Content article/card" })
  }
  if (html.includes("modal") || html.includes("dialog")) {
    components.push({ name: "Modal", purpose: "Modal dialog overlay" })
  }
  if (html.includes("dropdown") || html.includes("menu")) {
    components.push({ name: "Dropdown", purpose: "Dropdown menu" })
  }

  return components
}

function generateDevelopmentPhases(url: string, fingerprints: Map<string, { tech: any; matches: string[] }>) {
  const hasReact = Array.from(fingerprints.keys()).some((k) => k.toLowerCase().includes("react"))
  const hasVue = Array.from(fingerprints.keys()).some((k) => k.toLowerCase().includes("vue"))
  const hasNext = Array.from(fingerprints.keys()).some((k) => k.toLowerCase().includes("next"))
  const hasTailwind = Array.from(fingerprints.keys()).some((k) => k.toLowerCase().includes("tailwind"))

  const framework = hasNext ? "Next.js" : hasReact ? "React" : hasVue ? "Vue" : "Unknown"

  return [
    {
      name: "Phase 1: Project Setup",
      tasks: [
        `Initialize ${framework} project with TypeScript`,
        hasTailwind ? "Configure Tailwind CSS" : "Set up CSS/styling solution",
        "Configure ESLint and Prettier",
        "Set up project structure",
      ],
      estimatedTime: "2-3 hours",
    },
    {
      name: "Phase 2: Core Layout Components",
      tasks: [
        "Create layout shell (header, main, footer)",
        "Implement responsive navigation",
        "Set up routing structure",
        "Create base page templates",
      ],
      estimatedTime: "4-6 hours",
    },
    {
      name: "Phase 3: UI Components",
      tasks: [
        "Create button component with variants",
        "Create input/form components",
        "Create card components",
        "Create modal/dialog components",
      ],
      estimatedTime: "6-8 hours",
    },
    {
      name: "Phase 4: Page Implementation",
      tasks: [
        "Implement home page",
        "Implement additional pages",
        "Add responsive adjustments",
        "Optimize performance",
      ],
      estimatedTime: "8-12 hours",
    },
    {
      name: "Phase 5: Polish & Deploy",
      tasks: [
        "Accessibility audit and fixes",
        "Performance optimization",
        "Testing across browsers",
        "Deploy to production",
      ],
      estimatedTime: "4-6 hours",
    },
  ]
}

function generateFileStructure(fingerprints: Map<string, { tech: any; matches: string[] }>) {
  const hasReact = Array.from(fingerprints.keys()).some((k) => k.toLowerCase().includes("react"))
  const hasVue = Array.from(fingerprints.keys()).some((k) => k.toLowerCase().includes("vue"))
  const hasNext = Array.from(fingerprints.keys()).some((k) => k.toLowerCase().includes("next"))

  if (hasNext) {
    return `app/
├── layout.tsx
├── page.tsx
├── globals.css
└── api/
components/
├── ui/
│   ├── button.tsx
│   ├── input.tsx
│   └── ...
├── layout/
│   ├── header.tsx
│   ├── footer.tsx
│   └── navigation.tsx
lib/
├── utils.ts
└── api.ts
public/
└── images/`
  }

  if (hasReact) {
    return `src/
├── App.tsx
├── main.tsx
├── index.css
components/
├── ui/
│   ├── Button.tsx
│   ├── Input.tsx
│   └── ...
├── layout/
│   ├── Header.tsx
│   ├── Footer.tsx
│   └── Navigation.tsx
hooks/
├── useState.ts
└── useApi.ts
utils/
└── helpers.ts
assets/
└── images/`
  }

  if (hasVue) {
    return `src/
├── App.vue
├── main.ts
├── style.css
components/
├── Button.vue
├── Input.vue
├── Header.vue
├── Footer.vue
└── ...
views/
├── HomeView.vue
└── ...
router/
└── index.ts
stores/
└── ...
assets/
└── images/`
  }

  return `src/
├── index.html
├── styles.css
components/
├── Header.ts
├── Footer.ts
└── ...
pages/
├── index.ts
└── ...
assets/
└── images/`
}

function estimateTotalTime(fingerprints: Map<string, { tech: any; matches: string[] }>) {
  const techCount = fingerprints.size
  if (techCount > 10) return "3-5 days"
  if (techCount > 5) return "2-3 days"
  return "1-2 days"
}

function showInteractiveSummary(report: ReverseAnalysisReport) {
  console.log("\n" + "=".repeat(60))
  console.log(`🔍 WEBSITE REVERSE ANALYSIS: ${new URL(report.url).hostname}`)
  console.log("=".repeat(60) + "\n")

  console.log("📊 TECHNOLOGY STACK")
  console.log("  Frontend:")
  if (report.techStack.frontend.framework) console.log(`    • Framework: ${report.techStack.frontend.framework}`)
  if (report.techStack.frontend.uiLibrary) console.log(`    • UI Library: ${report.techStack.frontend.uiLibrary}`)
  if (report.techStack.frontend.styling) console.log(`    • Styling: ${report.techStack.frontend.styling}`)

  if (report.techStack.infrastructure.hosting) {
    console.log("\n  Infrastructure:")
    console.log(`    • Hosting: ${report.techStack.infrastructure.hosting}`)
  }
  if (report.techStack.infrastructure.cdn?.length) {
    console.log(`    • CDN: ${report.techStack.infrastructure.cdn.join(", ")}`)
  }
  if (report.techStack.infrastructure.analytics?.length) {
    console.log(`    • Analytics: ${report.techStack.infrastructure.analytics.join(", ")}`)
  }

  console.log("\n🎨 DESIGN SYSTEM")
  if (report.designSystem.colors.primary?.length) {
    console.log(`  Primary Colors: ${report.designSystem.colors.primary.join(", ")}`)
  }
  if (report.designSystem.typography.body?.family) {
    console.log(`  Font Family: ${report.designSystem.typography.body.family}`)
  }

  console.log(`\n🔧 COMPONENTS DETECTED: ${report.components.length}`)
  console.log(`🌐 API ENDPOINTS: ${report.apiEndpoints.length}`)

  console.log("\n⏱️  ESTIMATED EFFORT")
  for (const phase of report.phases) {
    console.log(`  ${phase.name}: ${phase.estimatedTime}`)
  }
  console.log(`  **TOTAL: ${report.totalEstimatedTime}**`)

  console.log("\n" + "=".repeat(60) + "\n")
}

export const ReverseCommands = {
  command: "reverse",
  describe: "Website reverse engineering tools",
  builder: (yargs: Argv) => {
    return yargs
      .command(ReverseAnalyzeCommand)
      .command(ReverseListCommand)
  },
  handler: () => {
    // Show help if no subcommand
    console.log("Use 'codecoder reverse analyze <url>' to analyze a website")
  },
}
