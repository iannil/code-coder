import z from "zod"
import { Tool } from "./tool"
import DESCRIPTION from "./network-analyzer.txt"

export interface ApiEndpoint {
  method: string
  url: string
  path: string
  description?: string
  headers?: Record<string, string>
  queryParams?: string[]
  body?: unknown
  response?: unknown
  contentType?: string
}

export interface NetworkAnalysis {
  url: string
  endpoints: ApiEndpoint[]
  serverInfo?: {
    type?: string
    version?: string
    headers: Record<string, string>
  }
  analytics: string[]
  tracking: string[]
  cdns: string[]
  frameworks: string[]
  errors: Array<{ message: string; url?: string }>
}

export const NetworkAnalyzerTool = Tool.define("network-analyzer", {
  description: DESCRIPTION,
  parameters: z.object({
    url: z.string().describe("The URL to analyze network traffic for"),
    format: z
      .enum(["text", "markdown", "json"])
      .default("markdown")
      .describe("The output format (text, markdown, or json)"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "bash",
      patterns: [params.url],
      always: ["*"],
      metadata: {
        url: params.url,
        format: params.format,
      },
    })

    const analysis = await analyzeNetwork(params.url, ctx.abort)

    if (params.format === "json") {
      return {
        output: JSON.stringify(analysis, null, 2),
        title: `Network analysis: ${params.url}`,
        metadata: {},
      }
    }

    if (params.format === "text") {
      return {
        output: formatText(analysis),
        title: `Network analysis: ${params.url}`,
        metadata: {},
      }
    }

    // markdown format (default)
    return {
      output: formatMarkdown(analysis),
      title: `Network analysis: ${params.url}`,
      metadata: {},
    }
  },
})

async function analyzeNetwork(url: string, signal: AbortSignal): Promise<NetworkAnalysis> {
  const analysis: NetworkAnalysis = {
    url,
    endpoints: [],
    analytics: [],
    tracking: [],
    cdns: [],
    frameworks: [],
    errors: [],
  }

  const controller = new AbortController()
  const timeoutId = setTimeout(() => controller.abort(), 30000)

  try {
    const response = await fetch(url, {
      signal: AbortSignal.any([controller.signal, signal]),
      headers: {
        "User-Agent":
          "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36",
      },
    })
    clearTimeout(timeoutId)

    // Analyze response headers
    const serverHeaders: Record<string, string> = {}
    response.headers.forEach((value, key) => {
      serverHeaders[key] = value
    })

    analysis.serverInfo = {
      headers: serverHeaders,
    }

    // Detect server type from headers
    const server = response.headers.get("server")
    const poweredBy = response.headers.get("x-powered-by")
    if (server) analysis.serverInfo.type = server
    if (poweredBy) analysis.serverInfo.version = poweredBy

    // Detect hosting/CDN from headers
    if (response.headers.get("cf-ray")) {
      analysis.cdns.push("Cloudflare")
      analysis.serverInfo.type = "Cloudflare"
    }
    if (response.headers.get("x-vercel-id")) {
      analysis.cdns.push("Vercel")
      analysis.serverInfo.type = "Vercel"
    }
    if (response.headers.get("x-amz-cf-id")) {
      analysis.cdns.push("AWS CloudFront")
    }
    if (response.headers.get("x-amzn-requestid")) {
      analysis.cdns.push("AWS")
    }
    if (response.headers.get("x-rid")) {
      analysis.cdns.push("Vercel")
    }
    if (response.headers.get("x-nf-request-id")) {
      analysis.cdns.push("Netlify")
    }

    // Try to get HTML content for further analysis
    const html = await response.text()

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
          if (!analysis.analytics.includes(analytic.name)) {
            analysis.analytics.push(analytic.name)
          }
          break
        }
      }
    }

    // Detect tracking
    const trackingPatterns = [
      { name: "Facebook Pixel", patterns: ["connect.facebook.net", "fbq("] },
      { name: "LinkedIn Insight", patterns: ["analytics.linkedin.com", "lintrk"] },
      { name: "Twitter Pixel", patterns: ["static.ads-twitter.com", "twq("] },
      { name: "TikTok Pixel", patterns: ["analytics.tiktok.com", "ttq("] },
      { name: "Pinterest Tag", patterns: ["ct.pinterest.com", "pintrk("] },
    ]

    for (const tracker of trackingPatterns) {
      for (const pattern of tracker.patterns) {
        if (html.includes(pattern)) {
          if (!analysis.tracking.includes(tracker.name)) {
            analysis.tracking.push(tracker.name)
          }
          break
        }
      }
    }

    // Detect frameworks from HTML
    const frameworkPatterns = [
      { name: "React", patterns: ["__REACT__", "reactRoot", "data-reactroot"] },
      { name: "Vue", patterns: ["__vue__", "v-cloak", "data-v-"] },
      { name: "Angular", patterns: ["ng-version", "ng-app"] },
      { name: "Svelte", patterns: ["data-svelte-h"] },
      { name: "Solid", patterns: ["data-hk"] },
      { name: "Next.js", patterns: ["__NEXT_DATA__", "/_next/"] },
      { name: "Nuxt", patterns: ["__NUXT__", "/_nuxt/"] },
      { name: "Remix", patterns: ["__remixContext"] },
      { name: "Astro", patterns: ["astro-head"] },
      { name: "jQuery", patterns: ["jquery", "jQuery(", "$("] },
      { name: "Alpine.js", patterns: ["x-data", "alpine"] },
      { name: "HTMX", patterns: ["hx-", "htmx"] },
      { name: "Tailwind CSS", patterns: ["tailwindcss", "@tailwind"] },
      { name: "Bootstrap", patterns: ["bootstrap", "navbar-"] },
      { name: "Material-UI", patterns: ["MuiBox-", "MuiButton-"] },
      { name: "Ant Design", patterns: ["ant-", "antd"] },
      { name: "Chakra UI", patterns: ["chakra-"] },
    ]

    for (const framework of frameworkPatterns) {
      for (const pattern of framework.patterns) {
        if (html.includes(pattern)) {
          if (!analysis.frameworks.includes(framework.name)) {
            analysis.frameworks.push(framework.name)
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
          // Extract URL from quotes
          const urlMatch = match.match(/["']([^"']+)["']/)
          if (urlMatch) {
            foundEndpoints.add(urlMatch[1])
          }
        }
      }
    }

    // Convert found endpoints to ApiEndpoint objects
    for (const endpoint of foundEndpoints) {
      analysis.endpoints.push({
        method: "GET", // Default, would need deeper analysis to determine actual method
        url: new URL(endpoint, url).href,
        path: new URL(endpoint, url).pathname,
        description: "Detected in source code",
      })
    }

    // Try to infer REST API patterns
    const restPaths = html.match(/["']\/api\/\w+(\/\w+)?["']/g)
    if (restPaths) {
      for (const path of restPaths) {
        const cleanPath = path.replace(/["']/g, "")
        const method = cleanPath.includes("create") || cleanPath.includes("add")
          ? "POST"
          : cleanPath.includes("update") || cleanPath.includes("edit")
            ? "PUT/PATCH"
            : cleanPath.includes("delete") || cleanPath.includes("remove")
              ? "DELETE"
              : "GET"

        if (!analysis.endpoints.find((e) => e.path === cleanPath)) {
          analysis.endpoints.push({
            method,
            url: new URL(cleanPath, url).href,
            path: cleanPath,
            description: "REST API endpoint (inferred)",
          })
        }
      }
    }
  } catch (error) {
    clearTimeout(timeoutId)
    if (error instanceof Error && error.name !== "AbortError") {
      analysis.errors.push({ message: error.message })
    }
  }

  return analysis
}

function formatMarkdown(analysis: NetworkAnalysis): string {
  const lines: string[] = []

  lines.push("# Network Analysis Report")
  lines.push()
  lines.push(`**Target URL:** ${analysis.url}`)
  lines.push()

  // Server Information
  if (analysis.serverInfo) {
    lines.push("## Server Information")
    if (analysis.serverInfo.type) {
      lines.push(`- **Type:** ${analysis.serverInfo.type}`)
    }
    if (analysis.serverInfo.version) {
      lines.push(`- **Version:** ${analysis.serverInfo.version}`)
    }
    lines.push()
  }

  // Frameworks Detected
  if (analysis.frameworks.length > 0) {
    lines.push("## Frameworks Detected")
    for (const framework of analysis.frameworks) {
      lines.push(`- ${framework}`)
    }
    lines.push()
  }

  // CDN/Hosting
  if (analysis.cdns.length > 0) {
    lines.push("## CDN/Hosting")
    for (const cdn of analysis.cdns) {
      lines.push(`- ${cdn}`)
    }
    lines.push()
  }

  // Analytics
  if (analysis.analytics.length > 0) {
    lines.push("## Analytics")
    for (const analytic of analysis.analytics) {
      lines.push(`- ${analytic}`)
    }
    lines.push()
  }

  // Tracking
  if (analysis.tracking.length > 0) {
    lines.push("## Tracking Pixels")
    for (const tracker of analysis.tracking) {
      lines.push(`- ${tracker}`)
    }
    lines.push()
  }

  // API Endpoints
  if (analysis.endpoints.length > 0) {
    lines.push("## Detected API Endpoints")
    lines.push()
    lines.push("| Method | Path | Description |")
    lines.push("|--------|------|-------------|")
    for (const endpoint of analysis.endpoints) {
      lines.push(`| ${endpoint.method} | \`${endpoint.path}\` | ${endpoint.description || "-"} |`)
    }
    lines.push()
  }

  // Errors
  if (analysis.errors.length > 0) {
    lines.push("## Errors")
    for (const error of analysis.errors) {
      lines.push(`- ${error.message}`)
    }
    lines.push()
  }

  return lines.join("\n")
}

function formatText(analysis: NetworkAnalysis): string {
  const lines: string[] = []

  lines.push(`Network Analysis for: ${analysis.url}`)
  lines.push("=".repeat(50))
  lines.push()

  if (analysis.serverInfo?.type) {
    lines.push(`Server: ${analysis.serverInfo.type}`)
  }
  if (analysis.serverInfo?.version) {
    lines.push(`Powered By: ${analysis.serverInfo.version}`)
  }
  lines.push()

  if (analysis.frameworks.length > 0) {
    lines.push("Frameworks:")
    for (const fw of analysis.frameworks) {
      lines.push(`  - ${fw}`)
    }
    lines.push()
  }

  if (analysis.cdns.length > 0) {
    lines.push("CDNs:")
    for (const cdn of analysis.cdns) {
      lines.push(`  - ${cdn}`)
    }
    lines.push()
  }

  if (analysis.analytics.length > 0) {
    lines.push("Analytics:")
    for (const analytic of analysis.analytics) {
      lines.push(`  - ${analytic}`)
    }
    lines.push()
  }

  if (analysis.tracking.length > 0) {
    lines.push("Tracking:")
    for (const tracker of analysis.tracking) {
      lines.push(`  - ${tracker}`)
    }
    lines.push()
  }

  if (analysis.endpoints.length > 0) {
    lines.push("API Endpoints:")
    for (const ep of analysis.endpoints) {
      lines.push(`  ${ep.method} ${ep.path}`)
    }
    lines.push()
  }

  return lines.join("\n")
}
