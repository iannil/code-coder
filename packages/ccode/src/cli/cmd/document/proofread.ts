/**
 * Proofread Commands
 *
 * Commands for document proofreading, grammar checking, and readability analysis.
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../../ui"
import { bootstrap } from "../../bootstrap"
import { Document, Proofreader } from "../../../document"
import { escapeShellArg } from "../../../util/security"
import { $ } from "bun"

// ============================================================================
// Check Command
// ============================================================================

export const ProofreadCheckCommand = cmd({
  command: "check <documentID>",
  describe: "proofread document for grammar, spelling, and style issues",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "check specific chapter",
      })
      .option("depth", {
        type: "string",
        alias: "d",
        choices: ["quick", "standard", "deep"],
        describe: "checking depth",
        default: "standard",
      })
      .option("auto", {
        type: "boolean",
        describe: "automatically run proofreader agent",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const doc = await Document.get(args.documentID as string)
      if (!doc) {
        UI.error("Document not found")
        process.exit(1)
      }

      prompts.intro(`Proofreading: ${doc.title}`)

      const spinner = prompts.spinner()
      spinner.start("Generating proofreading prompt...")

      const prompt = await Proofreader.checkPrompt({
        documentID: args.documentID as string,
        chapterID: args.chapter as string,
        depth: args.depth as "quick" | "standard" | "deep",
      })

      spinner.stop("Prompt ready")

      if (args.auto) {
        spinner.start("Running proofreader agent...")

        try {
          const tempFile = `/tmp/prompt_${Date.now()}.txt`
          await Bun.write(tempFile, prompt)
          const result = await $`bun run dev run --agent proofreader "$(cat ${escapeShellArg(tempFile)})"`.quiet()
            .text()
          await Bun.file(tempFile).delete()

          spinner.stop("Processing results...")

          const scope = args.chapter ? "chapter" : "document"
          const report = await Proofreader.saveReport(
            args.documentID as string,
            result,
            scope,
            args.chapter as string,
          )

          console.log()
          console.log(`Report generated: ${report.id}`)
          console.log(`Issues found: ${report.issues.length}`)
          console.log(`  Critical: ${report.summary.bySeverity.critical ?? 0}`)
          console.log(`  High: ${report.summary.bySeverity.high ?? 0}`)
          console.log(`  Medium: ${report.summary.bySeverity.medium ?? 0}`)
          console.log(`  Low: ${report.summary.bySeverity.low ?? 0}`)
          console.log()

          if (report.readabilityScore !== undefined) {
            console.log(`Readability Score: ${report.readabilityScore}/100`)
            console.log()
          }

          console.log(`View report: codecoder document proofread show ${args.documentID} ${report.id}`)
        } catch (error) {
          spinner.stop(`Error: ${error}`)
          throw error
        }
      } else {
        console.log()
        console.log("Use this prompt with the proofreader agent:")
        console.log("```")
        console.log(prompt.slice(0, 2000))
        if (prompt.length > 2000) console.log("...")
        console.log("```")
        console.log()
        console.log("Or use --auto to automatically run the proofreader agent")
      }
    })
  },
})

// ============================================================================
// Reports Command
// ============================================================================

export const ProofreadReportsCommand = cmd({
  command: "reports <documentID>",
  describe: "list all proofreading reports",
  builder: (yargs: Argv) => {
    return yargs.positional("documentID", {
      type: "string",
      describe: "document ID",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const reports = await Proofreader.listReports(args.documentID as string)

      if (reports.length === 0) {
        console.log("No proofreading reports found.")
        return
      }

      console.log()
      console.log(`Proofreading Reports (${reports.length}):`)
      console.log()

      for (const report of reports) {
        const date = new Date(report.timestamp).toLocaleString()
        const critical = report.summary.bySeverity.critical ?? 0
        const high = report.summary.bySeverity.high ?? 0
        const medium = report.summary.bySeverity.medium ?? 0
        const low = report.summary.bySeverity.low ?? 0

        console.log(`${report.id}`)
        console.log(`  ${date}`)
        console.log(`  Scope: ${report.scope}`)
        if (report.chapterID) console.log(`  Chapter: ${report.chapterID}`)
        console.log(`  Issues: ${report.issues.length} total (🔴${critical} 🟠${high} 🟡${medium} 🟢${low})`)
        if (report.readabilityScore !== undefined) {
          console.log(`  Readability: ${report.readabilityScore}/100`)
        }
        console.log()
      }
    })
  },
})

// ============================================================================
// Show Command
// ============================================================================

export const ProofreadShowCommand = cmd({
  command: "show <documentID> <reportID>",
  describe: "show a specific proofreading report",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .positional("reportID", {
        type: "string",
        describe: "report ID",
      })
      .option("output", {
        type: "string",
        alias: "o",
        describe: "save report to file",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const report = await Proofreader.getReport(args.documentID as string, args.reportID as string)

      if (!report) {
        UI.error("Report not found")
        process.exit(1)
      }

      const markdown = Proofreader.formatReportAsMarkdown(report)

      if (args.output) {
        await Bun.write(args.output as string, markdown)
        console.log(`Report saved to: ${args.output}`)
      } else {
        console.log()
        console.log(markdown)
      }
    })
  },
})

// ============================================================================
// Fix Command
// ============================================================================

export const ProofreadFixCommand = cmd({
  command: "fix <documentID>",
  describe: "generate fix prompt for proofreading issues",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "chapter ID",
        demandOption: true,
      })
      .option("severity", {
        type: "string",
        choices: ["critical", "high", "medium", "low"],
        describe: "minimum severity to fix",
        default: "medium",
      })
      .option("type", {
        type: "string",
        choices: ["grammar", "spelling", "punctuation", "terminology", "style", "flow", "readability", "structure"],
        describe: "only fix issues of this type",
      })
      .option("dry-run", {
        type: "boolean",
        describe: "show issues without generating fix prompt",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const reports = await Proofreader.listReports(args.documentID as string)

      if (reports.length === 0) {
        console.log("No proofreading reports found. Run 'proofread check' first.")
        return
      }

      const latestReport = reports[0]

      // Filter issues by severity
      const severityOrder = { critical: 4, high: 3, medium: 2, low: 1 }
      const minSeverity = args.severity as "critical" | "high" | "medium" | "low"
      const minLevel = severityOrder[minSeverity] || 2

      let issuesToFix = latestReport!.issues.filter((issue) => {
        return (severityOrder[issue.severity] || 0) >= minLevel
      })

      // Filter by type if specified
      if (args.type) {
        issuesToFix = issuesToFix.filter((issue) => issue.type === args.type)
      }

      // Only include issues for the specified chapter
      issuesToFix = issuesToFix.filter((issue) => {
        return issue.location?.chapterID === args.chapter
      })

      if (issuesToFix.length === 0) {
        console.log()
        console.log("No issues to fix with the given filters.")
        return
      }

      console.log()
      console.log(`Issues to fix: ${issuesToFix.length}`)
      console.log()

      if (args.dryRun) {
        for (const issue of issuesToFix) {
          const icon = { critical: "🔴", high: "🟠", medium: "🟡", low: "🟢" }[issue.severity] || "⚪"
          console.log(`${icon} [${issue.severity}] ${issue.type}`)
          console.log(`  ${issue.description}`)
          if (issue.suggestion) console.log(`  Fix: ${issue.suggestion}`)
          console.log()
        }
        return
      }

      const spinner = prompts.spinner()
      spinner.start("Generating fix prompt...")

      const prompt = await Proofreader.generateFixPrompt({
        documentID: args.documentID as string,
        chapterID: args.chapter as string,
        issues: issuesToFix,
      })

      spinner.stop("Prompt ready")

      console.log()
      console.log("--- FIX PROMPT ---")
      console.log()
      console.log(prompt)
      console.log()
    })
  },
})

// ============================================================================
// Readability Command
// ============================================================================

export const ProofreadReadabilityCommand = cmd({
  command: "readability <documentID>",
  describe: "analyze document readability",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "analyze specific chapter",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const result = await Proofreader.analyzeReadability(
        args.documentID as string,
        args.chapter as string,
      )

      console.log()
      console.log("## Readability Analysis")
      console.log()
      console.log(`**Score:** ${result.readabilityScore}/100 (${result.assessment})`)
      console.log()
      console.log("### Metrics")
      console.log()
      console.log(`- Average Sentence Length: ${result.metrics.avgSentenceLength.toFixed(1)} words`)
      console.log(`- Average Word Length: ${result.metrics.avgWordLength.toFixed(1)} characters`)
      console.log(`- Complex Words: ${result.metrics.complexWords}`)
      console.log(`- Total Sentences: ${result.metrics.totalSentences}`)
      console.log(`- Total Words: ${result.metrics.totalWords}`)
      console.log()

      // Assessment
      if (result.readabilityScore >= 80) {
        console.log("✓ Very readable content")
      } else if (result.readabilityScore >= 60) {
        console.log("✓ Standard readability")
      } else if (result.readabilityScore >= 40) {
        console.log("⚠ Content may be difficult for some readers")
      } else {
        console.log("⚠ Content may be very difficult to read")
      }
      console.log()
    })
  },
})

// ============================================================================
// Terminology Command
// ============================================================================

export const ProofreadTerminologyCommand = cmd({
  command: "terminology <documentID>",
  describe: "check terminology consistency",
  builder: (yargs: Argv) => {
    return yargs.positional("documentID", {
      type: "string",
      describe: "document ID",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const issues = await Proofreader.checkTerminology(args.documentID as string)

      if (issues.length === 0) {
        console.log()
        console.log("No terminology issues found.")
        return
      }

      console.log()
      console.log(`Found ${issues.length} terminology issue(s):`)
      console.log()

      for (const issue of issues) {
        const icon = { low: "🟢", medium: "🟡", high: "🟠" }[issue.severity] || "⚪"
        console.log(`${icon} [${issue.severity.toUpperCase()}] ${issue.description}`)
        console.log()

        for (const occ of issue.occurrences.slice(0, 5)) {
          console.log(`  - "${occ.term}" in ${occ.chapter}`)
        }
        if (issue.occurrences.length > 5) {
          console.log(`  ... and ${issue.occurrences.length - 5} more`)
        }
        console.log()
      }
    })
  },
})

// ============================================================================
// Batch Command
// ============================================================================

export const ProofreadBatchCommand = cmd({
  command: "batch <documentID>",
  describe: "batch proofread multiple chapters",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("parallel", {
        type: "number",
        alias: "p",
        describe: "number of parallel checks",
        default: 3,
      })
      .option("depth", {
        type: "string",
        alias: "d",
        choices: ["quick", "standard", "deep"],
        describe: "checking depth",
        default: "standard",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const chapters = await Document.Chapter.list(args.documentID as string)
      const completed = chapters.filter((c) => c.status === "completed")

      if (completed.length === 0) {
        console.log("No completed chapters to check.")
        return
      }

      prompts.intro(`Batch Proofreading ${completed.length} chapters`)

      const spinner = prompts.spinner()
      spinner.start("Generating prompts...")

      const promptsList = await Proofreader.batchCheck({
        documentID: args.documentID as string,
        chapterIDs: completed.map((c) => c.id),
        depth: args.depth as "quick" | "standard" | "deep",
      })

      spinner.stop(`${promptsList.length} prompts generated`)

      console.log()
      console.log("To proofread chapters, run:")
      console.log()
      for (const item of promptsList.slice(0, 3)) {
        console.log(`  codecoder document proofread check ${args.documentID} --chapter ${item.chapterID}`)
      }
      if (promptsList.length > 3) {
        console.log(`  ... and ${promptsList.length - 3} more`)
      }
      console.log()
    })
  },
})

// ============================================================================
// Quick Command
// ============================================================================

export const ProofreadQuickCommand = cmd({
  command: "quick <documentID>",
  describe: "quick local grammar check (no AI)",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "check specific chapter",
        demandOption: true,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const issues = await Proofreader.quickGrammarCheck(
        args.documentID as string,
        args.chapter as string,
      )

      if (issues.length === 0) {
        console.log()
        console.log("No issues found in quick check.")
        return
      }

      console.log()
      console.log(`Found ${issues.length} issue(s):`)
      console.log()

      for (const issue of issues) {
        console.log(`[${issue.type}] ${issue.description}`)
        console.log(`  Location: ${issue.location}`)
        console.log(`  Suggestion: ${issue.suggestion}`)
        console.log()
      }
    })
  },
})

// ============================================================================
// Main Proofread Command (groups all subcommands)
// ============================================================================

export const ProofreadCommand = cmd({
  command: "proofread",
  describe: "proofread document for errors and improvements",
  builder: (yargs) =>
    yargs
      .command(ProofreadCheckCommand)
      .command(ProofreadReportsCommand)
      .command(ProofreadShowCommand)
      .command(ProofreadFixCommand)
      .command(ProofreadReadabilityCommand)
      .command(ProofreadTerminologyCommand)
      .command(ProofreadBatchCommand)
      .command(ProofreadQuickCommand)
      .demandCommand(),
  async handler() {},
})
