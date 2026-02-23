/**
 * LLM-Enhanced Tool Abstractor
 *
 * Uses LLM to intelligently analyze code and extract:
 * - Parameters (including hardcoded values that should be parameterized)
 * - Tool purpose and description
 * - Usage examples
 *
 * This module enhances the heuristic-based parameter extraction in learner.ts
 * by leveraging LLM understanding of code semantics and intent.
 *
 * Part of Phase 17: LLM-Enhanced Tool Abstraction
 */

import { Log } from "@/util/log"
import { Provider } from "@/provider/provider"
import { generateText } from "ai"
import { ToolTypes } from "./types"

const log = Log.create({ service: "memory.tools.llm-abstractor" })

export namespace LLMAbstractor {
  // ============================================================================
  // Configuration
  // ============================================================================

  export interface AbstractorConfig {
    /** Analysis timeout in ms */
    timeout: number
    /** Temperature for LLM generation (lower = more deterministic) */
    temperature: number
    /** Maximum output tokens for analysis */
    maxOutputTokens: number
    /** Whether to attempt code generalization */
    generalizeCode: boolean
  }

  const DEFAULT_CONFIG: AbstractorConfig = {
    timeout: 30000,
    temperature: 0.2,
    maxOutputTokens: 2000,
    generalizeCode: true,
  }

  // ============================================================================
  // Analysis Prompt
  // ============================================================================

  const ANALYSIS_PROMPT = `Analyze this code that successfully solved a programming task.

Task: {{task}}
Language: {{language}}
Code:
\`\`\`{{language}}
{{code}}
\`\`\`
Output:
{{output}}

Your job is to analyze this code for potential reuse as a tool. Extract:
1. The purpose of this code (one-line description)
2. A good tool name (snake_case, max 50 chars)
3. Parameters - both explicit (function args, variables) and implicit (hardcoded values that should be configurable)
4. Usage examples showing different ways to use this tool

Respond with ONLY valid JSON (no markdown, no explanation):
{
  "purpose": "one-line description of what this code does",
  "toolName": "snake_case_name",
  "parameters": [
    {
      "name": "param_name",
      "type": "string|number|boolean|array|object",
      "description": "what this parameter does",
      "required": true,
      "defaultValue": null,
      "extractedFrom": "line X or 'hardcoded'"
    }
  ],
  "hardcodedValues": [
    {
      "value": "the hardcoded value",
      "line": 5,
      "shouldParameterize": true,
      "suggestedParamName": "suggested_name"
    }
  ],
  "examples": [
    {
      "description": "example use case",
      "input": {"param": "value"},
      "expectedOutput": "what it produces"
    }
  ]
}`

  // ============================================================================
  // Main Analysis Function
  // ============================================================================

  /**
   * Analyze code using LLM to extract tool metadata
   *
   * @param execution The successful execution record to analyze
   * @param config Optional configuration overrides
   * @returns LLM analysis result or null if analysis fails
   */
  export async function analyzeCode(
    execution: ToolTypes.ExecutionRecord,
    config?: Partial<AbstractorConfig>,
  ): Promise<ToolTypes.LLMAnalysisResult | null> {
    const cfg = { ...DEFAULT_CONFIG, ...config }

    try {
      log.debug("Starting LLM analysis", {
        task: execution.task.slice(0, 100),
        language: execution.language,
        codeLength: execution.code.length,
      })

      const model = await Provider.defaultModel()
      const languageModel = await Provider.getLanguage(
        await Provider.getModel(model.providerID, model.modelID),
      )

      const prompt = ANALYSIS_PROMPT
        .replace(/\{\{task\}\}/g, execution.task)
        .replace(/\{\{language\}\}/g, execution.language)
        .replace(/\{\{code\}\}/g, execution.code.slice(0, 3000))
        .replace(/\{\{output\}\}/g, execution.output.slice(0, 500))

      const result = await generateText({
        model: languageModel,
        prompt,
        maxOutputTokens: cfg.maxOutputTokens,
        temperature: cfg.temperature,
      })

      // Parse the JSON response
      const parsed = parseAnalysisResponse(result.text)
      if (!parsed) {
        log.warn("Failed to parse LLM analysis response", {
          responsePreview: result.text.slice(0, 200),
        })
        return null
      }

      // Optionally generalize the code
      if (cfg.generalizeCode && parsed.hardcodedValues.length > 0) {
        parsed.generalizedCode = generalizeCode(
          execution.code,
          parsed.hardcodedValues,
          execution.language,
        )
      }

      log.info("LLM analysis completed", {
        toolName: parsed.toolName,
        paramCount: parsed.parameters.length,
        hardcodedCount: parsed.hardcodedValues.length,
        exampleCount: parsed.examples.length,
      })

      return parsed
    } catch (error) {
      log.warn("LLM analysis failed", {
        error: error instanceof Error ? error.message : String(error),
      })
      return null
    }
  }

  // ============================================================================
  // Response Parsing
  // ============================================================================

  /**
   * Parse and validate the LLM response
   */
  function parseAnalysisResponse(text: string): ToolTypes.LLMAnalysisResult | null {
    try {
      // Try to extract JSON from the response
      const jsonMatch = text.match(/\{[\s\S]*\}/)
      if (!jsonMatch) {
        return null
      }

      const raw = JSON.parse(jsonMatch[0])

      // Validate with Zod schema
      const result = ToolTypes.LLMAnalysisResult.safeParse({
        purpose: raw.purpose ?? "",
        toolName: sanitizeToolName(raw.toolName ?? ""),
        parameters: (raw.parameters ?? []).map((p: Record<string, unknown>) => ({
          name: String(p.name ?? ""),
          type: normalizeType(p.type),
          description: String(p.description ?? ""),
          required: Boolean(p.required ?? true),
          defaultValue: p.defaultValue,
          extractedFrom: String(p.extractedFrom ?? "unknown"),
        })),
        hardcodedValues: (raw.hardcodedValues ?? []).map((h: Record<string, unknown>) => ({
          value: String(h.value ?? ""),
          line: Number(h.line ?? 0),
          shouldParameterize: Boolean(h.shouldParameterize ?? false),
          suggestedParamName: String(h.suggestedParamName ?? ""),
        })),
        examples: (raw.examples ?? []).map((e: Record<string, unknown>) => ({
          description: String(e.description ?? ""),
          input: (e.input as Record<string, unknown>) ?? {},
          expectedOutput: String(e.expectedOutput ?? ""),
        })),
      })

      if (!result.success) {
        log.debug("Schema validation failed", { issues: result.error.issues })
        return null
      }

      return result.data
    } catch (error) {
      log.debug("JSON parsing failed", {
        error: error instanceof Error ? error.message : String(error),
      })
      return null
    }
  }

  // ============================================================================
  // Code Generalization
  // ============================================================================

  /**
   * Replace hardcoded values with parameter placeholders
   */
  function generalizeCode(
    code: string,
    hardcodedValues: ToolTypes.HardcodedValue[],
    language: string,
  ): string {
    // Sort by line number descending to preserve positions during replacement
    const sortedValues = [...hardcodedValues]
      .filter((h) => h.shouldParameterize && h.suggestedParamName)
      .sort((a, b) => b.line - a.line)

    return sortedValues.reduce((result, hv) => {
      const placeholder = formatPlaceholder(hv.suggestedParamName, language)
      // Replace the exact value with placeholder (case-sensitive, whole value)
      const escapedValue = escapeRegex(hv.value)
      return result.replace(new RegExp(escapedValue, "g"), placeholder)
    }, code)
  }

  /**
   * Format parameter placeholder based on language
   */
  function formatPlaceholder(paramName: string, language: string): string {
    switch (language) {
      case "python":
        return `{${paramName}}`
      case "nodejs":
        return `\${${paramName}}`
      case "bash":
        return `$${paramName}`
      default:
        return `{{${paramName}}}`
    }
  }

  // ============================================================================
  // Utility Functions
  // ============================================================================

  /**
   * Sanitize tool name to snake_case
   */
  function sanitizeToolName(name: string): string {
    return name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_|_$/g, "")
      .slice(0, 50)
  }

  /**
   * Normalize type string to valid parameter type
   */
  function normalizeType(type: unknown): ToolTypes.ToolParameter["type"] {
    const normalized = String(type).toLowerCase()
    const validTypes = ["string", "number", "boolean", "array", "object"] as const

    for (const valid of validTypes) {
      if (normalized.includes(valid)) {
        return valid
      }
    }

    // Default to string for unknown types
    return "string"
  }

  /**
   * Escape special regex characters
   */
  function escapeRegex(str: string): string {
    return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
  }

  // ============================================================================
  // Conversion Helpers
  // ============================================================================

  /**
   * Convert LLM analysis result to standard ToolParameter array
   *
   * This bridges the gap between LLM output and the existing tool system.
   */
  export function toToolParameters(
    analysis: ToolTypes.LLMAnalysisResult,
  ): ToolTypes.ToolParameter[] {
    return analysis.parameters.map((p) => ({
      name: p.name,
      type: p.type,
      description: p.description,
      required: p.required,
      default: p.defaultValue,
    }))
  }

  /**
   * Convert LLM examples to standard ToolExample array
   */
  export function toToolExamples(
    analysis: ToolTypes.LLMAnalysisResult,
  ): ToolTypes.ToolExample[] {
    return analysis.examples.map((e) => ({
      description: e.description,
      input: e.input as Record<string, unknown>,
      output: e.expectedOutput,
    }))
  }
}
