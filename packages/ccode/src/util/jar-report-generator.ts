/**
 * JAR Analysis Report Generator
 *
 * Generates structured markdown reports and TUI-compatible data structures
 * from JAR reverse engineering analysis.
 */

import path from "path"
import { mkdir } from "node:fs/promises"
import type { JarAnalysisResult } from "./jar-analyzer"

// ============================================================================
// Types
// ============================================================================

export interface JavaTechnologyStack {
  frameworks: DetectedTechnology[]
  orm: DetectedTechnology[]
  web: DetectedTechnology[]
  serialization: DetectedTechnology[]
  utilities: DetectedTechnology[]
  logging: DetectedTechnology[]
  testing: DetectedTechnology[]
  messaging: DetectedTechnology[]
  caching: DetectedTechnology[]
  validation: DetectedTechnology[]
  security: DetectedTechnology[]
  scheduling: DetectedTechnology[]
  http: DetectedTechnology[]
}

export interface DetectedTechnology {
  name: string
  category: string
  confidence: "high" | "medium" | "low"
  matchCount: number
  matches: string[]
}

export interface JarClassStructure {
  totalClasses: number
  totalPackages: number
  packageHierarchy: PackageNode[]
  keyClasses: KeyClassInfo[]
}

export interface PackageNode {
  name: string
  classCount: number
  children: PackageNode[]
}

export interface KeyClassInfo {
  name: string
  type: string
  modifiers: string[]
  description: string
}

export interface JarConfigurationFile {
  path: string
  type: string
  description: string
}

export interface JarDependency {
  groupId?: string
  artifactId?: string
  version?: string
  notation?: string
}

export interface JarDevelopmentPhase {
  name: string
  tasks: string[]
  estimatedTime: string
}

export interface JarAnalysisReport {
  jarFilename: string
  jarSize: string
  executiveSummary: string
  metadata: {
    mainClass?: string
    version?: string
    buildTool?: string
    jdkVersion?: string
    implementationTitle?: string
    bundleName?: string
  }
  techStack: JavaTechnologyStack
  classStructure: JarClassStructure
  configFiles: JarConfigurationFile[]
  dependencies: JarDependency[]
  phases: JarDevelopmentPhase[]
  fileStructure?: string
  totalEstimatedTime: string
  notes?: string
}

// ============================================================================
// Report Generator
// ============================================================================

export namespace JarReportGenerator {
  const REPORTS_DIR = "reports/jar-reverse"

  /**
   * Ensure reports directory exists
   */
  async function ensureDir(outputDir?: string): Promise<void> {
    const dir = outputDir || REPORTS_DIR
    await mkdir(dir, { recursive: true })
  }

  /**
   * Generate a filename for the report
   */
  export function generateFilename(jarName: string): string {
    const baseName = jarName.replace(/\.jar$/i, "")
    const date = new Date().toISOString().split("T")[0]
    const safeName = baseName.replace(/[^a-zA-Z0-9-._]/g, "-")
    return `${safeName}-${date}.md`
  }

  /**
   * Generate markdown report from analysis data
   */
  export async function generateMarkdown(
    analysis: JarAnalysisResult,
    options: { outputDir?: string } = {},
  ): Promise<{ content: string; filepath: string }> {
    const outputDir = options.outputDir || REPORTS_DIR
    await ensureDir(outputDir)

    const filename = generateFilename(analysis.jarName)
    const filepath = path.join(outputDir, filename)

    const report = buildReport(analysis)
    const content = renderMarkdown(report)

    return { content, filepath }
  }

  /**
   * Build the report structure from analysis result
   */
  function buildReport(analysis: JarAnalysisResult): JarAnalysisReport {
    const report: JarAnalysisReport = {
      jarFilename: analysis.jarName,
      jarSize: formatBytes(analysis.sizeBytes),
      executiveSummary: generateExecutiveSummary(analysis),
      metadata: {
        mainClass: analysis.metadata.mainClass,
        version: analysis.metadata.implementationVersion || analysis.metadata.bundleVersion,
        buildTool: analysis.metadata.buildTool,
        jdkVersion: analysis.metadata.jdkVersion,
        implementationTitle: analysis.metadata.implementationTitle || analysis.metadata.bundleName,
      },
      techStack: categorizeTechnologies(analysis),
      classStructure: buildClassStructure(analysis),
      configFiles: buildConfigFiles(analysis),
      dependencies: buildDependencies(analysis),
      phases: generateDevelopmentPhases(analysis),
      fileStructure: generateFileStructure(analysis),
      totalEstimatedTime: estimateTotalTime(analysis),
    }

    return report
  }

  /**
   * Generate executive summary
   */
  function generateExecutiveSummary(analysis: JarAnalysisResult): string {
    const parts: string[] = []

    parts.push(`This analysis covers **${analysis.jarName}**`)

    if (analysis.metadata.implementationTitle) {
      parts.push(`, which appears to be ${analysis.metadata.implementationTitle.toLowerCase()}`)
    }

    if (analysis.metadata.implementationVersion) {
      parts.push(` version **${analysis.metadata.implementationVersion}**`)
    }

    parts.push(`. The JAR contains **${analysis.classes.length} classes** across **${analysis.packages.length} packages**.`)

    const techCount = analysis.detectedTechs.size
    if (techCount > 0) {
      const techNames = Array.from(analysis.detectedTechs.keys()).slice(0, 5)
      parts.push(` The project uses **${techNames.join(", ")}**${techCount > 5 ? ` and ${techCount - 5} other technologies` : ""}.`)
    }

    if (analysis.metadata.buildTool) {
      parts.push(` Built with **${analysis.metadata.buildTool}**.`)
    }

    return parts.join("") + "."
  }

  /**
   * Categorize detected technologies
   */
  function categorizeTechnologies(analysis: JarAnalysisResult): JavaTechnologyStack {
    const stack: JavaTechnologyStack = {
      frameworks: [],
      orm: [],
      web: [],
      serialization: [],
      utilities: [],
      logging: [],
      testing: [],
      messaging: [],
      caching: [],
      validation: [],
      security: [],
      scheduling: [],
      http: [],
    }

    const categoryMap: Record<string, keyof JavaTechnologyStack> = {
      framework: "frameworks",
      orm: "orm",
      web: "web",
      serialization: "serialization",
      utility: "utilities",
      logging: "logging",
      testing: "testing",
      messaging: "messaging",
      caching: "caching",
      validation: "validation",
      security: "security",
      scheduling: "scheduling",
      http: "http",
    }

    for (const [name, { tech, matches }] of analysis.detectedTechs.entries()) {
      const targetCategory = categoryMap[tech.category]
      if (targetCategory) {
        const confidence = matches.some((m) => m.includes("(high)"))
          ? "high"
          : matches.some((m) => m.includes("(medium)"))
            ? "medium"
            : "low"

        stack[targetCategory].push({
          name,
          category: tech.category,
          confidence,
          matchCount: matches.length,
          matches,
        })
      }
    }

    // Sort by match count and confidence
    for (const category of Object.values(stack)) {
      category.sort((a: DetectedTechnology, b: DetectedTechnology) => {
        const confidenceOrder: Record<string, number> = { high: 3, medium: 2, low: 1 }
        if (confidenceOrder[b.confidence] !== confidenceOrder[a.confidence]) {
          return confidenceOrder[b.confidence] - confidenceOrder[a.confidence]
        }
        return b.matchCount - a.matchCount
      })
    }

    return stack
  }

  /**
   * Build class structure hierarchy
   */
  function buildClassStructure(analysis: JarAnalysisResult): JarClassStructure {
    // Build package tree
    const root: PackageNode = { name: "", classCount: 0, children: [] }

    for (const pkg of analysis.packages) {
      const parts = pkg.name.split(".")
      let current = root

      for (const part of parts) {
        let child = current.children.find((c) => c.name === part)
        if (!child) {
          child = { name: part, classCount: 0, children: [] }
          current.children.push(child)
        }
        current = child
      }

      current.classCount += pkg.classCount
    }

    // Convert to simpler structure
    function flatten(node: PackageNode, prefix: string = ""): PackageNode[] {
      const fullName = prefix ? `${prefix}.${node.name}` : node.name
      const result: PackageNode[] = []

      if (node.classCount > 0 || node.children.length > 0) {
        result.push({
          name: fullName || "(default)",
          classCount: node.classCount,
          children: node.children.map((c) => ({ name: c.name, classCount: c.classCount, children: [] })),
        })
      }

      for (const child of node.children) {
        result.push(...flatten(child, fullName))
      }

      return result
    }

    const packageHierarchy = flatten(root).slice(1) // Remove empty root

    // Identify key classes
    const keyClasses: KeyClassInfo[] = []

    // Look for main classes, controllers, services, etc.
    const keyPatterns = [
      { pattern: /Main|Application|Launcher/, description: "Application entry point" },
      { pattern: /Controller|RestController|Endpoint/, description: "Web controller/endpoint" },
      { pattern: /Service|Manager/, description: "Business logic service" },
      { pattern: /Repository|DAO/, description: "Data access object" },
      { pattern: /Config|Configuration/, description: "Configuration class" },
      { pattern: /Exception|Error/, description: "Exception/error class" },
      { pattern: /Filter|Interceptor/, description: "Request filter/interceptor" },
      { pattern: /Listener|Handler/, description: "Event listener/handler" },
    ]

    for (const cls of analysis.classes) {
      for (const { pattern, description } of keyPatterns) {
        if (pattern.test(cls.simpleName)) {
          keyClasses.push({
            name: cls.name,
            type: cls.type,
            modifiers: cls.modifiers,
            description,
          })
          break
        }
      }
    }

    // Limit key classes
    return {
      totalClasses: analysis.classes.length,
      totalPackages: analysis.packages.length,
      packageHierarchy: packageHierarchy.slice(0, 50),
      keyClasses: keyClasses.slice(0, 100),
    }
  }

  /**
   * Build config files info
   */
  function buildConfigFiles(analysis: JarAnalysisResult): JarConfigurationFile[] {
    return analysis.configFiles.map((file) => ({
      path: file.path,
      type: file.type,
      description: getConfigFileDescription(file),
    }))
  }

  /**
   * Get description for config file
   */
  function getConfigFileDescription(file: { path: string; type: string }): string {
    const descriptions: Record<string, string> = {
      "spring-config": "Spring Framework configuration",
      "web-config": "Java web application configuration (web.xml)",
      "jpa-config": "JPA/persistence configuration",
      "hibernate-config": "Hibernate ORM configuration",
      "properties": "Java properties file",
      "yaml": "YAML configuration file",
      "xml": "XML configuration file",
    }

    return descriptions[file.type] || "Configuration file"
  }

  /**
   * Build dependencies list
   */
  function buildDependencies(analysis: JarAnalysisResult): JarDependency[] {
    return analysis.dependencies.map((dep) => ({
      ...dep,
      notation: dep.groupId && dep.artifactId ? `${dep.groupId}:${dep.artifactId}${dep.version ? `:${dep.version}` : ""}` : undefined,
    }))
  }

  /**
   * Generate development phases
   */
  function generateDevelopmentPhases(analysis: JarAnalysisResult): JarDevelopmentPhase[] {
    const hasSpring = Array.from(analysis.detectedTechs.keys()).some((k) => k.toLowerCase().includes("spring"))
    const hasWeb = Array.from(analysis.detectedTechs.keys()).some((k) => k.toLowerCase().includes("tomcat") || k.toLowerCase().includes("jetty"))
    const hasOrm = Array.from(analysis.detectedTechs.keys()).some((k) => k.toLowerCase().includes("hibernate") || k.toLowerCase().includes("jpa"))
    const hasKafka = Array.from(analysis.detectedTechs.keys()).some((k) => k.toLowerCase().includes("kafka"))
    const hasRedis = Array.from(analysis.detectedTechs.keys()).some((k) => k.toLowerCase().includes("redis") || k.toLowerCase().includes("caffeine"))

    const phases: JarDevelopmentPhase[] = [
      {
        name: "Phase 1: Project Setup",
        tasks: [
          hasSpring ? "Initialize Spring Boot project with Maven/Gradle" : "Initialize Java project with Maven/Gradle",
          "Configure build tool and dependencies",
          "Set up project structure (src/main/java, src/test/java)",
          "Configure logging framework",
          "Set up IDE and code style",
        ],
        estimatedTime: "2-4 hours",
      },
      {
        name: "Phase 2: Core Domain Models",
        tasks: [
          "Create entity/domain classes based on discovered packages",
          hasOrm ? "Configure ORM/persistence layer" : "Define data structures",
          "Create DTOs/value objects",
          "Add validation annotations",
        ],
        estimatedTime: "4-8 hours",
      },
      {
        name: "Phase 3: Business Logic Layer",
        tasks: [
          "Implement service classes",
          hasSpring ? "Create Spring service components" : "Create business logic classes",
          "Implement business rules",
          "Add exception handling",
        ],
        estimatedTime: "8-16 hours",
      },
      {
        name: "Phase 4: Data Access Layer",
        tasks: [
          hasOrm ? "Set up repository/DAO layer" : "Implement data access",
          hasOrm ? "Configure database connections" : "Define data sources",
          "Create query methods",
        ],
        estimatedTime: "6-12 hours",
      },
    ]

    if (hasWeb) {
      phases.push({
        name: "Phase 5: Web Layer",
        tasks: [
          "Create REST controllers or web endpoints",
          "Configure servlet/web context",
          "Implement request/response handling",
          "Add error response handling",
        ],
        estimatedTime: "6-10 hours",
      })
    }

    if (hasKafka || hasRedis) {
      phases.push({
        name: `Phase ${hasWeb ? 6 : 5}: Integration Layer`,
        tasks: [
          hasKafka ? "Configure Kafka producer/consumer" : "Configure messaging layer",
          hasRedis ? "Set up caching layer" : "Implement caching",
          "Configure connection pools",
        ],
        estimatedTime: "4-8 hours",
      })
    }

    phases.push({
      name: `Phase ${phases.length + 1}: Testing & Polish`,
      tasks: [
        "Write unit tests for core logic",
        "Write integration tests",
        hasWeb ? "Test API endpoints" : "Test public interfaces",
        "Performance optimization",
        "Documentation",
      ],
      estimatedTime: "8-12 hours",
    })

    return phases
  }

  /**
   * Generate file structure recommendation
   */
  function generateFileStructure(analysis: JarAnalysisResult): string {
    const hasSpring = Array.from(analysis.detectedTechs.keys()).some((k) => k.toLowerCase().includes("spring"))
    const hasWeb = Array.from(analysis.detectedTechs.keys()).some((k) => k.toLowerCase().includes("tomcat") || k.toLowerCase().includes("jetty"))
    const basePackage = analysis.packages[0]?.name.split(".")[0] || "com.example"

    if (hasSpring) {
      return `src/main/java/
└── ${basePackage.replace(/\./g, "/")}
    ├── controller/      # REST controllers
    ├── service/         # Business logic
    ├── repository/      # Data access
    ├── model/           # Domain models
    ├── dto/             # Data transfer objects
    ├── exception/       # Custom exceptions
    ├── config/          # Configuration classes
    └── util/            # Utilities
src/main/resources/
├── application.yml     # Spring configuration
└── logback.xml         # Logging config
src/test/java/
└── ${basePackage.replace(/\./g, "/")}
    ├── controller/      # Controller tests
    ├── service/         # Service tests
    └── repository/      # Repository tests`
    }

    if (hasWeb) {
      return `src/main/java/
└── ${basePackage.replace(/\./g, "/")}
    ├── servlet/         # Web servlets
    ├── filter/          # Servlet filters
    ├── listener/        # Event listeners
    ├── service/         # Business logic
    ├── dao/             # Data access
    ├── model/           # Domain models
    └── util/            # Utilities
src/main/resources/
├── META-INF/
│   └── web.xml         # Web app config
└── logging.properties   # Logging config`
    }

    return `src/main/java/
└── ${basePackage.replace(/\./g, "/")}
    ├── ${analysis.classes.find((c) => c.simpleName.includes("Main"))?.simpleName || "Main"}.java  # Entry point
    ├── model/           # Domain models
    ├── service/         # Business logic
    ├── dao/             # Data access
    └── util/            # Utilities
src/main/resources/
└── config.properties    # Application config`
  }

  /**
   * Estimate total time
   */
  function estimateTotalTime(analysis: JarAnalysisResult): string {
    const classCount = analysis.classes.length
    const techCount = analysis.detectedTechs.size

    if (classCount > 500 || techCount > 10) return "3-5 weeks"
    if (classCount > 200 || techCount > 5) return "2-3 weeks"
    if (classCount > 100 || techCount > 3) return "1-2 weeks"
    return "3-5 days"
  }

  /**
   * Render report as markdown
   */
  function renderMarkdown(report: JarAnalysisReport): string {
    const lines: string[] = []

    // Header
    lines.push(`# JAR Reverse Analysis: ${report.jarFilename}`)
    lines.push()
    lines.push(`**Generated:** ${new Date().toISOString()}`)
    lines.push(`**File Size:** ${report.jarSize}`)
    lines.push()

    // Executive Summary
    lines.push("## Executive Summary")
    lines.push()
    lines.push(report.executiveSummary)
    lines.push()

    // Metadata
    lines.push("## Metadata")
    lines.push()
    const meta = report.metadata
    if (meta.mainClass) lines.push(`- **Main Class:** \`${meta.mainClass}\``)
    if (meta.version) lines.push(`- **Version:** ${meta.version}`)
    if (meta.implementationTitle) lines.push(`- **Title:** ${meta.implementationTitle}`)
    if (meta.buildTool) lines.push(`- **Build Tool:** ${meta.buildTool}`)
    if (meta.jdkVersion) lines.push(`- **JDK Version:** ${meta.jdkVersion}`)
    lines.push()

    // Technology Stack
    lines.push("## Technology Stack")
    lines.push()

    const stack = report.techStack
    const categories = [
      { key: "frameworks", title: "Frameworks" },
      { key: "web", title: "Web Servers" },
      { key: "orm", title: "ORM / Database" },
      { key: "serialization", title: "Serialization" },
      { key: "utilities", title: "Utilities" },
      { key: "logging", title: "Logging" },
      { key: "testing", title: "Testing" },
      { key: "messaging", title: "Messaging" },
      { key: "caching", title: "Caching" },
      { key: "security", title: "Security" },
      { key: "http", title: "HTTP Clients" },
      { key: "validation", title: "Validation" },
      { key: "scheduling", title: "Scheduling" },
    ] as const

    for (const { key, title } of categories) {
      const items = stack[key]
      if (items.length > 0) {
        lines.push(`### ${title}`)
        for (const item of items) {
          const confidenceIcon = item.confidence === "high" ? "✓" : item.confidence === "medium" ? "~" : "?"
          lines.push(`- **${item.name}** ${confidenceIcon}`)
        }
        lines.push()
      }
    }

    // Class Structure
    lines.push("## Class Structure")
    lines.push()
    lines.push(`- **Total Classes:** ${report.classStructure.totalClasses}`)
    lines.push(`- **Total Packages:** ${report.classStructure.totalPackages}`)
    lines.push()

    if (report.classStructure.packageHierarchy.length > 0) {
      lines.push("### Package Hierarchy")
      for (const pkg of report.classStructure.packageHierarchy.slice(0, 30)) {
        const indent = "  ".repeat(pkg.name.split(".").length - 1)
        lines.push(`${indent}- \`${pkg.name}\` (${pkg.classCount} classes)`)
      }
      lines.push()
    }

    if (report.classStructure.keyClasses.length > 0) {
      lines.push("### Key Classes")
      lines.push()
      lines.push("| Class | Type | Description |")
      lines.push("|-------|------|-------------|")
      for (const cls of report.classStructure.keyClasses.slice(0, 50)) {
        lines.push(`| \`${cls.name}\` | ${cls.type} | ${cls.description} |`)
      }
      lines.push()
    }

    // Configuration Files
    if (report.configFiles.length > 0) {
      lines.push("## Configuration Files")
      lines.push()
      for (const file of report.configFiles) {
        lines.push(`### ${file.path}`)
        lines.push(`- **Type:** ${file.type}`)
        lines.push(`- **Description:** ${file.description}`)
        lines.push()
      }
    }

    // Dependencies
    if (report.dependencies.length > 0) {
      lines.push("## Dependencies")
      lines.push()
      for (const dep of report.dependencies.slice(0, 50)) {
        if (dep.notation) {
          lines.push(`- \`${dep.notation}\``)
        } else if (dep.artifactId) {
          lines.push(`- ${dep.artifactId}`)
        }
      }
      lines.push()
    }

    // Development Plan
    lines.push("## Development Plan")
    lines.push()

    for (const phase of report.phases) {
      lines.push(`### ${phase.name}`)
      lines.push(`*Estimated time: ${phase.estimatedTime}*`)
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
    lines.push("## Estimated Effort")
    lines.push()
    lines.push("| Phase | Time |")
    lines.push("|-------|------|")
    for (const phase of report.phases) {
      lines.push(`| ${phase.name} | ${phase.estimatedTime} |`)
    }
    lines.push(`| **Total** | **${report.totalEstimatedTime}** |`)
    lines.push()

    // Notes
    if (report.notes) {
      lines.push("## Notes")
      lines.push()
      lines.push(report.notes)
      lines.push()
    }

    return lines.join("\n")
  }

  /**
   * Save report to file
   */
  export async function saveReport(
    analysis: JarAnalysisResult,
    options: { outputDir?: string } = {},
  ): Promise<string> {
    const { content, filepath } = await generateMarkdown(analysis, options)
    await Bun.write(filepath, content)
    return filepath
  }

  /**
   * Generate JSON report
   */
  export async function generateJson(
    analysis: JarAnalysisResult,
    options: { outputDir?: string } = {},
  ): Promise<{ content: string; filepath: string }> {
    const outputDir = options.outputDir || REPORTS_DIR
    await ensureDir(outputDir)

    const filename = generateFilename(analysis.jarName).replace(".md", ".json")
    const filepath = path.join(outputDir, filename)

    const report = buildReport(analysis)
    const content = JSON.stringify(report, null, 2)

    return { content, filepath }
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`
}
