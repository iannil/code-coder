import type { Argv } from "yargs"
import { cmd } from "./cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../ui"
import { Log } from "@/util/log"
import { bootstrap } from "../bootstrap"
import { Document } from "../../document"
import * as Knowledge from "../../document/knowledge"
import { ExpansionOrchestrator } from "../../autonomous/expansion/index"

const log = Log.create({ service: "cli.book-writer" })

// ============================================================================
// Types
// ============================================================================

interface BookExpandArgs {
  input?: string
  type?: "fiction" | "nonfiction" | "auto"
  "target-words"?: number
  autonomy?: "autonomous" | "stage-confirm" | "interactive"
  output?: string
  "document-id"?: string
  agent?: "expander" | "expander-fiction" | "expander-nonfiction"
}

// ============================================================================
// Book Expand Commands (BookExpander)
// ============================================================================

/**
 * Expand a core idea into a full-length book using the BookExpander framework.
 */
const BookExpandCommand = cmd({
  command: "book-expand [input]",
  describe: "Expand short content into a full-length book with systematic framework building",
  builder: (yargs: Argv) => {
    return yargs
      .positional("input", {
        type: "string",
        describe: "Core idea or input file path",
      })
      .option("type", {
        type: "string",
        choices: ["fiction", "nonfiction", "auto"] as const,
        default: "auto",
        describe: "Content type (auto-detect if 'auto')",
      })
      .option("target-words", {
        type: "number",
        default: 100000,
        describe: "Target word count for the expanded book",
      })
      .option("autonomy", {
        type: "string",
        choices: ["autonomous", "stage-confirm", "interactive"] as const,
        default: "stage-confirm",
        describe: "Autonomy level: autonomous (no prompts), stage-confirm (prompt per phase), interactive (prompt often)",
      })
      .option("output", {
        type: "string",
        describe: "Output directory for the expanded book",
      })
      .option("document-id", {
        type: "string",
        describe: "Existing document ID to expand (for continuing expansion)",
      })
      .option("agent", {
        type: "string",
        choices: ["expander", "expander-fiction", "expander-nonfiction"] as const,
        default: "expander",
        describe: "Agent to use for expansion",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      await runExpand(args as any)
    })
  },
})

/**
 * Main expansion workflow handler.
 */
async function runExpand(args: any): Promise<void> {
  prompts.intro("BookExpander - Systematic Book Expansion")

  // Get input content
  const input = await getInput(args)
  if (!input) throw new UI.CancelledError()

  // Get parameters with interactive prompts
  const params = await getParameters(args, input)
  if (!params) throw new UI.CancelledError()

  // Show expansion plan
  prompts.outro(`Expanding "${input.slice(0, 50)}${input.length > 50 ? "..." : ""}" into a ${params.targetWords}-word book`)

  const spinner = prompts.spinner()

  try {
    // Check if continuing existing expansion
    if (args["document-id"]) {
      spinner.start("Resuming expansion...")
      await resumeExpansion(args["document-id"] as string, params)
      spinner.stop("Expansion resumed")
    } else {
      // Start new expansion
      spinner.start("Initializing expansion...")

      // Create document
      const doc = await Document.create({
        title: params.title,
        description: input.slice(0, 500),
        targetWords: params.targetWords,
        styleGuide: params.styleGuide,
      })

      spinner.stop(`Document created: ${doc.id}`)

      // Run expansion orchestration
      spinner.start("Running expansion workflow...")

      // Simplified expansion workflow (placeholder for now)
      await simulateExpansion(doc.id, params)

      spinner.stop("Expansion workflow completed")

      prompts.outro(`Expansion complete! Use "codecoder document export ${doc.id}" to export the book`)
    }
  } catch (error) {
    spinner.stop(`Expansion interrupted`)
    if (error instanceof Error) {
      log.error(error.message)
    }
  }
}

/**
 * Placeholder function to simulate expansion for now.
 */
async function simulateExpansion(documentID: string, params: any): Promise<void> {
  // This would call the actual orchestrator in production
  await new Promise((resolve) => setTimeout(resolve, 100))
}

/**
 * Get input content from file or prompt.
 */
async function getInput(args: any): Promise<string | undefined> {
  let input = args.input as string | undefined

  // Read from file if specified
  if (input && (input.startsWith("/") || input.startsWith("."))) {
    const fs = await import("fs")
    try {
      const content = (fs as any).readFileSync(input, "utf-8")
      return content.trim()
    } catch {
      log.error(`Failed to read file: ${input}`)
      return undefined
    }
  }

  if (!input) {
    const promptedInput = await prompts.text({
      message: "Enter your core idea or paste content:",
      validate: (value) => {
        if (!value || value.trim().length === 0) {
          return "Input cannot be empty"
        }
        if (value.trim().length < 20) {
          return "Please provide more detail (at least 20 characters)"
        }
        return undefined
      },
    })
    if (prompts.isCancel(promptedInput)) return undefined
    input = promptedInput as string
  }

  return input
}

/**
 * Get expansion parameters from args or interactive prompts.
 */
async function getParameters(
  args: any,
  input: string,
): Promise<{
    title: string
    targetWords: number
    contentType: "fiction" | "nonfiction" | "auto"
    autonomy: "autonomous" | "stage-confirm" | "interactive"
    styleGuide: { format: "markdown"; tone?: string | undefined; audience?: string }
  } | undefined> {
  let targetWords = (args["target-words"] as number) ?? 100000
  let contentType = (args.type as any) ?? "auto"
  let autonomy = (args.autonomy as any) ?? "stage-confirm"

  // Get title
  const titlePrompt = await prompts.text({
    message: "Book title:",
    placeholder: "Based on your core idea",
    defaultValue: generateTitleFromInput(input),
  })
  if (prompts.isCancel(titlePrompt)) return undefined

  // Get or confirm content type
  if (contentType === "auto") {
    const detectedType = detectContentType(input)
    const typeConfirm = await prompts.select({
      message: `Detected content type: ${detectedType.toUpperCase()}`,
      options: [
        { value: "fiction", label: "Fiction" },
        { value: "nonfiction", label: "Non-Fiction" },
        { value: "mixed", label: "Mixed" },
      ],
      initialValue: detectedType,
    })
    if (prompts.isCancel(typeConfirm)) return undefined
    contentType = typeConfirm as any
  }

  // Get or confirm target words
  if (targetWords === 100000) {
    const wordsPrompt = await prompts.text({
      message: "Target word count:",
      placeholder: "100000",
      validate: (value: any) => {
        const num = parseInt(value as string)
        if (isNaN(num) || num < 10000) {
          return "Must be at least 10,000 words"
        }
        return undefined
      },
    })
    if (prompts.isCancel(wordsPrompt)) return undefined
    targetWords = parseInt(wordsPrompt as string)
  }

  // Get or confirm autonomy level
  const autonomyConfirm = await prompts.select({
    message: "Autonomy level:",
    options: [
      { value: "autonomous", label: "Autonomous - Run without prompts (recommended for full expansion)" },
      { value: "stage-confirm", label: "Stage-Confirm - Prompt between each phase" },
      { value: "interactive", label: "Interactive - Prompt for key decisions" },
    ],
    initialValue: autonomy,
  })
  if (prompts.isCancel(autonomyConfirm)) return undefined
  autonomy = autonomyConfirm as any

  // Optional style parameters
  const addStyle = await prompts.confirm({
    message: "Configure writing style?",
    initialValue: false,
  })
  if (prompts.isCancel(addStyle)) return undefined

  let tone: string | undefined = undefined
  let audience: string | undefined = undefined

  if (addStyle) {
    const tonePrompt = await prompts.text({
      message: "Writing tone (optional):",
      placeholder: "e.g., formal, casual, academic",
    })
    if (!prompts.isCancel(tonePrompt)) {
      tone = tonePrompt as any
    }

    const audiencePrompt = await prompts.text({
      message: "Target audience (optional):",
      placeholder: "e.g., general public, developers, researchers",
    })
    if (!prompts.isCancel(audiencePrompt)) {
      audience = audiencePrompt as any
    }
  }

  return {
    title: typeof titlePrompt === "string" ? titlePrompt : generateTitleFromInput(input),
    targetWords,
    contentType,
    autonomy,
    styleGuide: {
      format: "markdown" as const,
      ...(tone ? { tone } : {}),
      ...(audience ? { audience } : {}),
    },
  }
}

/**
 * Resume an existing expansion.
 */
async function resumeExpansion(documentID: string, parameters: { targetWords: number }): Promise<void> {
  const progress = await ExpansionOrchestrator.getProgress(documentID)
  const progressPercent = Math.round((progress.wordsWritten / progress.targetWords) * 100)

  // Output progress info using prompts for consistency
  prompts.outro("📊 Current Progress:")
  prompts.outro(`  Progress: ${progressPercent}%`)
  prompts.outro(`  Words: ${progress.wordsWritten}/${progress.targetWords}`)

  const resume = await prompts.select({
    message: "Resume from current phase?",
    options: [
      { value: "yes", label: "Yes - Continue expansion" },
      { value: "no", label: "No - Exit to review" },
    ],
    initialValue: "yes",
  })

  if (prompts.isCancel(resume) || resume === "no") {
    return
  }

  // Resume expansion (would call orchestrator's resume function)
  log.info("Resuming expansion...")
}

/**
 * Detect content type from input text.
 */
function detectContentType(input: string): "fiction" | "nonfiction" {
  const lowerInput = input.toLowerCase()

  const fictionIndicators = [
    /\b(character|protagonist|antagonist|plot|story|narrative|tale|fantasy|sci-fi|magic)\b/i,
    /\b(once upon a time|in a galaxy far away|long ago|imaginary)\b/i,
  ]

  const nonfictionIndicators = [
    /\b(theorem|proof|argument|evidence|research|study|analysis|thesis)\b/i,
    /\b(however|therefore|furthermore|moreover|consequently)\b/i,
  ]

  const fictionScore = fictionIndicators.reduce((sum, pattern) => sum + (pattern.test(lowerInput) ? 1 : 0), 0)
  const nonfictionScore = nonfictionIndicators.reduce((sum, pattern) => sum + (pattern.test(lowerInput) ? 1 : 0), 0)

  return fictionScore > nonfictionScore ? "fiction" : "nonfiction"
}

/**
 * Generate a title from input text.
 */
function generateTitleFromInput(input: string): string {
  const sentences = input.split(/[.!?]+/).filter((s) => s.trim().length > 10)
  if (sentences.length > 0) {
    const firstSentence = sentences[0].trim()
    return firstSentence.length > 80 ? firstSentence.slice(0, 77) + "..." : firstSentence
  }
  return "Untitled Expansion"
}

export { BookExpandCommand }
