/**
 * Tool Generator
 *
 * Generates TOOL concepts - reusable code scripts for task automation.
 * Extends the existing ToolLearner patterns from the dynamic tool registry.
 *
 * Risk Level: LOW
 * Auto-approvable: YES (with sandbox validation)
 *
 * @package autonomous/builder/generators
 */

import { Log } from "@/util/log"
import { Global } from "@/util/global"
import path from "path"
import { nanoid } from "nanoid"

import type {
  ConceptType,
  ConceptGenerator,
  GeneratorInput,
  GeneratedConcept,
} from "../types"
import { getLLMSolver } from "../../execution/llm-solver"

const log = Log.create({ service: "autonomous.builder.generators.tool" })

// ============================================================================
// Tool Generator
// ============================================================================

export class ToolGenerator implements ConceptGenerator {
  readonly conceptType: ConceptType = "TOOL"

  async generate(input: GeneratorInput): Promise<GeneratedConcept> {
    log.info("Generating TOOL concept", {
      gapId: input.gap.id,
      description: input.gap.description.slice(0, 100),
    })

    // Generate code using LLM
    const llmSolver = getLLMSolver()
    const webSources = input.context.webSources?.map((s) => ({
      url: s.url,
      content: s.summary ?? "",
      summary: s.summary ?? "",
      relevantSections: [] as string[],
      confidence: 0.5,
    })) ?? []

    const codeResult = await llmSolver.generateCode({
      problem: input.gap.description,
      technology: input.gap.technology,
      webSources,
      previousAttempts: [],
    })

    if (!codeResult) {
      throw new Error("Failed to generate tool code with LLM")
    }

    // Generate unique identifier
    const identifier = this.generateIdentifier(input)

    // Generate description from gap description
    const description = this.generateDescription(input)

    // Determine target path
    const targetPath = path.join(
      Global.Path.data,
      "tools",
      `${identifier}.${this.getExtension(codeResult.language)}`,
    )

    // Build tool metadata content
    const metadataContent = this.buildMetadataContent({
      identifier,
      description,
      language: codeResult.language,
      code: codeResult.code,
      tags: this.extractTags(input),
    })

    return {
      type: "TOOL",
      identifier,
      displayName: this.toDisplayName(identifier),
      description,
      content: codeResult.code,
      targetPath,
      additionalFiles: [
        {
          path: path.join(Global.Path.data, "tools", `${identifier}.meta.json`),
          content: metadataContent,
        },
      ],
      metadata: {
        generatedAt: Date.now(),
        generatedBy: "ToolGenerator",
        version: "1.0.0",
        tags: this.extractTags(input),
        dependencies: this.extractDependencies(codeResult.code, codeResult.language),
      },
    }
  }

  async validateInput(input: GeneratorInput): Promise<{ valid: boolean; issues?: string[] }> {
    const issues: string[] = []

    if (!input.gap.description || input.gap.description.length < 10) {
      issues.push("Gap description too short for tool generation")
    }

    if (input.existingConcepts.includes(input.gap.suggestedName ?? "")) {
      issues.push(`Tool with identifier "${input.gap.suggestedName}" already exists`)
    }

    return {
      valid: issues.length === 0,
      issues: issues.length > 0 ? issues : undefined,
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private generateIdentifier(input: GeneratorInput): string {
    if (input.gap.suggestedName) {
      const normalized = input.gap.suggestedName
        .toLowerCase()
        .replace(/[^a-z0-9_]/g, "_")
        .slice(0, 40)

      // Ensure unique
      if (!input.existingConcepts.includes(normalized)) {
        return normalized
      }
    }

    // Generate from description
    const words = input.gap.description
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, "")
      .split(/\s+/)
      .filter((w) => w.length > 2)
      .slice(0, 4)

    const base = words.join("_") || "tool"
    return `${base}_${nanoid(6)}`
  }

  private generateDescription(input: GeneratorInput): string {
    // Use gap description, but make it more tool-focused
    const baseDesc = input.gap.description
      .replace(/^need to /i, "")
      .replace(/^automate /i, "")

    return `Automates: ${baseDesc}`
  }

  private toDisplayName(identifier: string): string {
    return identifier
      .split("_")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ")
  }

  private getExtension(language: "python" | "nodejs" | "shell"): string {
    switch (language) {
      case "python":
        return "py"
      case "nodejs":
        return "js"
      case "shell":
        return "sh"
    }
  }

  private extractTags(input: GeneratorInput): string[] {
    const tags: string[] = []

    // Add technology as tag
    if (input.gap.technology) {
      tags.push(input.gap.technology.toLowerCase())
    }

    // Extract common keywords
    const keywords = ["api", "data", "file", "process", "analyze", "convert", "parse"]
    const descLower = input.gap.description.toLowerCase()

    for (const keyword of keywords) {
      if (descLower.includes(keyword)) {
        tags.push(keyword)
      }
    }

    return [...new Set(tags)]
  }

  private extractDependencies(code: string, language: "python" | "nodejs" | "shell"): string[] {
    const deps: string[] = []

    if (language === "python") {
      // Extract Python imports
      const importRegex = /^(?:from|import)\s+([a-zA-Z_][a-zA-Z0-9_]*)/gm
      let match: RegExpExecArray | null
      while ((match = importRegex.exec(code)) !== null) {
        const pkg = match[1]
        // Exclude standard library
        if (!this.isPythonStdlib(pkg)) {
          deps.push(pkg)
        }
      }
    } else if (language === "nodejs") {
      // Extract Node.js requires/imports
      const requireRegex = /require\s*\(\s*['"]([^'"]+)['"]\s*\)/g
      const importRegex = /import\s+.*?from\s+['"]([^'"]+)['"]/g

      let match: RegExpExecArray | null
      while ((match = requireRegex.exec(code)) !== null) {
        const pkg = match[1]
        if (!pkg.startsWith(".") && !this.isNodeBuiltin(pkg)) {
          deps.push(pkg.split("/")[0])
        }
      }
      while ((match = importRegex.exec(code)) !== null) {
        const pkg = match[1]
        if (!pkg.startsWith(".") && !this.isNodeBuiltin(pkg)) {
          deps.push(pkg.split("/")[0])
        }
      }
    }

    return [...new Set(deps)]
  }

  private isPythonStdlib(pkg: string): boolean {
    const stdlib = new Set([
      "os", "sys", "re", "json", "math", "time", "datetime", "random",
      "collections", "itertools", "functools", "pathlib", "typing",
      "subprocess", "shutil", "glob", "tempfile", "io", "urllib",
      "http", "email", "html", "xml", "logging", "unittest", "argparse",
    ])
    return stdlib.has(pkg)
  }

  private isNodeBuiltin(pkg: string): boolean {
    const builtins = new Set([
      "fs", "path", "os", "http", "https", "url", "querystring",
      "crypto", "stream", "buffer", "events", "util", "assert",
      "child_process", "cluster", "net", "dns", "readline", "repl",
    ])
    return builtins.has(pkg) || pkg.startsWith("node:")
  }

  private buildMetadataContent(params: {
    identifier: string
    description: string
    language: "python" | "nodejs" | "shell"
    code: string
    tags: string[]
  }): string {
    const metadata = {
      id: params.identifier,
      name: this.toDisplayName(params.identifier),
      description: params.description,
      language: params.language === "shell" ? "bash" : params.language,
      tags: params.tags,
      parameters: [],
      examples: [],
      metadata: {
        createdAt: Date.now(),
        updatedAt: Date.now(),
        createdBy: "agent",
        version: 1,
      },
      stats: {
        usageCount: 0,
        successCount: 0,
        failureCount: 0,
        lastUsedAt: null,
        averageExecutionTime: 0,
      },
    }

    return JSON.stringify(metadata, null, 2)
  }
}
