import type { Argv } from "yargs"
import { cmd } from "./cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../ui"
import { bootstrap } from "../bootstrap"
import { JarAnalyzer } from "../../util/jar-analyzer"
import { JarReportGenerator } from "../../util/jar-report-generator"
import { getJavaFingerprintsByCategory, getJavaCategories } from "../../util/java-fingerprints"
import path from "path"
import { mkdir } from "node:fs/promises"

// ============================================================================
// Jar Reverse Commands
// ============================================================================

const JarReverseAnalyzeCommand = cmd({
  command: "analyze <jarPath>",
  describe: "Analyze a JAR file and generate Java source code reconstruction plan",
  builder: (yargs: Argv) => {
    return yargs
      .positional("jarPath", {
        type: "string",
        description: "The path to the JAR file to analyze",
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
      .option("max-classes", {
        type: "number",
        default: 5000,
        describe: "Maximum number of classes to analyze",
      })
      .option("depth", {
        type: "string",
        choices: ["quick", "standard", "deep"],
        default: "standard",
        describe: "Analysis depth (quick=1000, standard=5000, deep=unlimited classes)",
      })
      .option("generate-source", {
        type: "boolean",
        default: false,
        describe: "Generate Java source code stubs (experimental)",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const jarPath = args.jarPath as string
      const outputDir = args.output as string | undefined
      const format = args.format as "markdown" | "json" | "tui"
      const maxClassesInput = args.maxClasses as number
      const depth = args.depth as "quick" | "standard" | "deep"
      const generateSource = args.generateSource as boolean

      prompts.intro("JAR Reverse Analysis")

      // Determine max classes based on depth
      const maxClasses = depth === "quick" ? 1000 : depth === "deep" ? Number.MAX_SAFE_INTEGER : maxClassesInput

      // Validate JAR path
      const jarFile = Bun.file(jarPath)
      if (!(await jarFile.exists())) {
        prompts.cancel("JAR file not found")
        process.exit(1)
      }

      // Check if file is a JAR
      if (!jarPath.toLowerCase().endsWith(".jar")) {
        const shouldContinue = await prompts.confirm({
          message: "File does not have .jar extension. Continue anyway?",
          initialValue: false,
        })

        if (!(shouldContinue === true)) {
          prompts.cancel("Analysis cancelled")
          process.exit(0)
        }
      }

      const analyzeSpinner = prompts.spinner()

      try {
        analyzeSpinner.start("Extracting and analyzing JAR...")

        const result = await JarAnalyzer.analyze(jarPath, { maxClasses })

        analyzeSpinner.stop(`Analysis complete: ${result.classes.length} classes, ${result.packages.length} packages`)

        // Show technology detection summary
        if (result.detectedTechs.size > 0) {
          const techSpinner = prompts.spinner()
          techSpinner.start(`${result.detectedTechs.size} technologies detected`)

          const techCategories = new Map<string, string[]>()
          for (const [name, { tech }] of result.detectedTechs.entries()) {
            if (!techCategories.has(tech.category)) {
              techCategories.set(tech.category, [])
            }
            techCategories.get(tech.category)!.push(name)
          }

          techSpinner.stop("Technology detection complete")
        }

        // Generate report
        if (format === "markdown" || format === "json") {
          const saveSpinner = prompts.spinner()
          saveSpinner.start("Generating report...")

          let outputPath: string

          if (format === "markdown") {
            const { content, filepath } = await JarReportGenerator.generateMarkdown(result, { outputDir })
            const reportsDir = outputDir || path.dirname(filepath)
            await mkdir(reportsDir, { recursive: true })
            outputPath = path.join(reportsDir, path.basename(filepath))
            await Bun.write(outputPath, content)
          } else {
            // JSON format
            const { content, filepath } = await JarReportGenerator.generateJson(result, { outputDir })
            const reportsDir = outputDir || path.join(process.cwd(), "reports")
            await mkdir(reportsDir, { recursive: true })
            outputPath = path.join(reportsDir, path.basename(filepath))
            await Bun.write(outputPath, content)
          }

          saveSpinner.stop(`Report saved to: ${outputPath}`)

          // Show summary
          showAnalysisSummary(result)

          prompts.outro(`Analysis complete! Report saved to: ${outputPath}`)
        } else if (format === "tui") {
          showAnalysisSummary(result)
          prompts.outro("Analysis complete!")
        }

        // Optionally generate source stubs
        if (generateSource) {
          prompts.log.warn("Source code generation is experimental and requires agent mode")
          prompts.log.info("Use 'codecoder jar-reverse' in agent mode for source generation")
        }
      } catch (error) {
        analyzeSpinner.stop("Analysis failed")
        prompts.log.error(`${error instanceof Error ? error.message : String(error)}`)
        prompts.cancel("Analysis failed")
        process.exit(1)
      }
    })
  },
})

const JarReverseListCommand = cmd({
  command: "list",
  describe: "List available Java technology fingerprints",
  builder: (yargs: Argv) => {
    return yargs.option("category", {
      type: "string",
      describe: "Filter by category (framework, orm, web, etc.)",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const categoryFilter = args.category as string | undefined

      if (categoryFilter) {
        UI.empty()
        prompts.intro(`Java Technology Fingerprints: ${categoryFilter}`)

        const categoryMap: Record<string, string> = {
          framework: "Frameworks",
          orm: "ORM / Database",
          web: "Web Servers",
          serialization: "Serialization",
          utility: "Utilities",
          logging: "Logging",
          testing: "Testing",
          messaging: "Messaging",
          caching: "Caching",
          validation: "Validation",
          security: "Security",
          scheduling: "Scheduling",
          http: "HTTP Clients",
        }

        const techs = getJavaCategories().includes(categoryFilter)
          ? getJavaFingerprintsByCategory(categoryFilter)
          : []

        if (techs.length === 0) {
          prompts.cancel(`Category "${categoryFilter}" not found`)
          const categories = getJavaCategories()
          prompts.outro("Available categories:\n" + categories.map((c) => `  - ${c} (${categoryMap[c] || c})`).join("\n"))
          process.exit(1)
        }

        for (const tech of techs) {
          const patternCount = tech.patterns.length
          prompts.log.info(`${tech.name} ${UI.Style.TEXT_DIM}(${patternCount} patterns)${UI.Style.TEXT_NORMAL}`)
          if (tech.website) {
            prompts.log.message(`  ${UI.Style.TEXT_DIM}${tech.website}${UI.Style.TEXT_NORMAL}`)
          }
        }
      } else {
        UI.empty()
        prompts.intro("Java Technology Fingerprint Categories")

        const categoryMap: Record<string, string> = {
          framework: "Frameworks",
          orm: "ORM / Database",
          web: "Web Servers",
          serialization: "Serialization",
          utility: "Utilities",
          logging: "Logging",
          testing: "Testing",
          messaging: "Messaging",
          caching: "Caching",
          validation: "Validation",
          security: "Security",
          scheduling: "Scheduling",
          http: "HTTP Clients",
        }

        const categories = getJavaCategories()
        for (const category of categories) {
          const techs = getJavaFingerprintsByCategory(category)
          const displayName = categoryMap[category] || category
          prompts.log.info(`${displayName}: ${techs.length} technologies`)
        }
      }

      prompts.outro("Use --category to view details")
    })
  },
})

// ============================================================================
// Helper Functions
// ============================================================================

function showAnalysisSummary(result: Awaited<ReturnType<typeof JarAnalyzer.analyze>>): void {
  console.log("\n" + "=".repeat(60))
  console.log(`📦 JAR REVERSE ANALYSIS: ${result.jarName}`)
  console.log("=".repeat(60) + "\n")

  // Metadata
  console.log("📋 METADATA")
  if (result.metadata.mainClass) console.log(`  Main Class: ${result.metadata.mainClass}`)
  if (result.metadata.implementationVersion) console.log(`  Version: ${result.metadata.implementationVersion}`)
  if (result.metadata.buildTool) console.log(`  Build Tool: ${result.metadata.buildTool}`)
  if (result.metadata.jdkVersion) console.log(`  JDK: ${result.metadata.jdkVersion}`)
  console.log(`  Size: ${formatBytes(result.sizeBytes)}`)

  // Technology Stack
  if (result.detectedTechs.size > 0) {
    console.log("\n🔧 DETECTED TECHNOLOGIES")

    const categories = new Map<string, string[]>()
    for (const [name, { tech }] of result.detectedTechs.entries()) {
      if (!categories.has(tech.category)) {
        categories.set(tech.category, [])
      }
      categories.get(tech.category)!.push(name)
    }

    for (const [category, techs] of categories.entries()) {
      console.log(`  ${category.toUpperCase()}: ${techs.join(", ")}`)
    }
  }

  // Structure
  console.log(`\n📊 STRUCTURE`)
  console.log(`  Classes: ${result.classes.length}`)
  console.log(`  Packages: ${result.packages.length}`)
  console.log(`  Config Files: ${result.configFiles.length}`)
  console.log(`  Dependencies: ${result.dependencies.length}`)

  // Top packages
  const topPackages = [...result.packages].sort((a, b) => b.classCount - a.classCount).slice(0, 5)
  if (topPackages.length > 0) {
    console.log(`\n📁 TOP PACKAGES`)
    for (const pkg of topPackages) {
      console.log(`  ${pkg.name} (${pkg.classCount} classes)`)
    }
  }

  console.log("\n" + "=".repeat(60) + "\n")
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`
}

export const JarReverseCommands = {
  command: "jar-reverse",
  describe: "JAR reverse engineering tools",
  builder: (yargs: Argv) => {
    return yargs.command(JarReverseAnalyzeCommand).command(JarReverseListCommand)
  },
  handler: () => {
    console.log("Use 'codecoder jar-reverse analyze <jarPath>' to analyze a JAR file")
  },
}
