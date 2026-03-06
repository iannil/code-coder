/**
 * Relevance Scoring Module (Native-Only)
 *
 * Provides intelligent file and content relevance scoring for context building.
 * Uses native Rust implementation exclusively for high-performance scoring.
 * Throws error if native bindings unavailable.
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Fingerprint } from "./fingerprint"
import { Loader } from "./loader"
import { Cache } from "./cache"
import {
  scoreRelevanceNative,
  scoreFilesNative,
  contentHashNative,
  isNativeAvailable,
  isUsingNative as isNativeRelevanceAvailable,
  type NapiFileMetadata,
} from "./relevance-native"
import path from "path"
import z from "zod"

const log = Log.create({ service: "context.relevance" })

export namespace Relevance {
  // ============================================================================
  // Types
  // ============================================================================

  export const ContextRequest = z.object({
    task: z.string(),
    filePaths: z.array(z.string()).optional(),
    maxTokens: z.number().optional(),
    includeTests: z.boolean().optional(),
    includeConfigs: z.boolean().optional(),
    includeDependencies: z.boolean().optional(),
  })
  export type ContextRequest = z.infer<typeof ContextRequest>

  export const RelevantFile = z.object({
    path: z.string(),
    reason: z.string(),
    priority: z.number(),
    content: z.string().optional(),
    size: z.number(),
    lastModified: z.number(),
  })
  export type RelevantFile = z.infer<typeof RelevantFile>

  export const ContextResult = z.object({
    files: z.array(RelevantFile),
    summary: z.string(),
    totalSize: z.number(),
    estimatedTokens: z.number(),
    truncated: z.boolean(),
  })
  export type ContextResult = z.infer<typeof ContextResult>

  // ============================================================================
  // Task Type Detection
  // ============================================================================

  const KEYWORD_PATTERNS: Record<string, string[]> = {
    component: ["component", "ui", "view", "page", "screen", "layout"],
    hook: ["hook", "use", "effect", "state", "reducer"],
    api: ["api", "endpoint", "route", "handler", "controller", "service"],
    test: ["test", "spec", "mock", "fixture", "assert"],
    config: ["config", "setting", "env", "constant"],
    util: ["util", "helper", "format", "parse", "validate", "transform"],
    type: ["type", "interface", "enum", "schema", "model", "dto"],
    style: ["style", "css", "theme", "design", "color"],
  }

  function detectTaskType(task: string): string[] {
    const taskLower = task.toLowerCase()
    const detected: string[] = []

    for (const [type, keywords] of Object.entries(KEYWORD_PATTERNS)) {
      if (keywords.some((keyword) => taskLower.includes(keyword))) {
        detected.push(type)
      }
    }

    if (taskLower.includes("bug") || taskLower.includes("fix") || taskLower.includes("error")) {
      detected.push("debug")
    }

    if (taskLower.includes("refactor") || taskLower.includes("clean") || taskLower.includes("organize")) {
      detected.push("refactor")
    }

    if (["add", "create", "implement", "build"].some((w) => taskLower.includes(w))) {
      detected.push("create")
    }

    return detected.length > 0 ? detected : ["general"]
  }

  function extractKeywords(task: string): string[] {
    const words = task
      .toLowerCase()
      .replace(/[^\w\s]/g, " ")
      .split(/\s+/)
      .filter((w) => w.length > 2)

    const unique = new Set(words)
    const techTerms = new Set<string>()

    for (const word of unique) {
      if (word.includes("component") || word.includes("service") || word.includes("hook")) {
        techTerms.add(word)
      }
      if (/^(use|get|set|create|update|delete|fetch|handle)/.test(word)) {
        techTerms.add(word)
      }
    }

    return Array.from(techTerms)
  }

  // ============================================================================
  // Relevance Scoring (Native-Only)
  // ============================================================================

  /**
   * Calculate relevance score using native implementation.
   * @throws Error if native bindings unavailable
   */
  export async function scoreRelevance(target: string, query: string): Promise<number> {
    const result = await scoreRelevanceNative(query, target)
    return result.score
  }

  /**
   * Score multiple files using native batch scoring.
   * @throws Error if native bindings unavailable
   */
  async function scoreFilesWithNative(
    task: string,
    files: Array<{ path: string; content: string; modified: number }>,
  ): Promise<Map<string, number>> {
    const scoreMap = new Map<string, number>()

    const napiFiles: NapiFileMetadata[] = files.map((f) => ({
      path: f.path,
      content: f.content,
      modified: Math.floor(f.modified / 1000),
      extension: path.extname(f.path).slice(1) || undefined,
    }))

    const nativeScores = await scoreFilesNative(task, napiFiles)
    for (const scored of nativeScores) {
      scoreMap.set(scored.path, scored.score.score)
    }

    return scoreMap
  }

  // ============================================================================
  // File Discovery
  // ============================================================================

  async function findRelevantFiles(
    task: string,
    filePaths: string[] | undefined,
    taskTypes: string[],
    keywords: string[],
    options: ContextRequest,
  ): Promise<RelevantFile[]> {
    const relevant: RelevantFile[] = []

    // Add directly referenced files
    if (filePaths && filePaths.length > 0) {
      for (const filePath of filePaths) {
        const file = await loadFileWithContext(filePath, task, "directly_referenced")
        if (file) relevant.push(file)
      }
    }

    const cache = await Cache.get()
    if (!cache) {
      log.warn("cache not available, skipping context enrichment")
      return relevant
    }

    // Add files based on task types
    for (const taskType of taskTypes) {
      switch (taskType) {
        case "component":
          await addComponentFiles(cache, task, relevant)
          break
        case "api":
          await addApiFiles(cache, task, relevant)
          break
        case "test":
          if (options.includeTests !== false) {
            await addTestFiles(cache, task, relevant)
          }
          break
        case "config":
          if (options.includeConfigs !== false) {
            await addConfigFiles(cache, task, relevant)
          }
          break
        case "hook":
          await addHookFiles(cache, task, relevant)
          break
        case "util":
          await addUtilFiles(cache, task, relevant)
          break
        case "type":
          await addTypeFiles(cache, relevant)
          break
        case "debug":
          await addDebugFiles(cache, task, relevant)
          break
        case "refactor":
          await addRefactorFiles(cache, task, relevant)
          break
      }
    }

    // Add files matching keywords
    for (const keyword of keywords) {
      await addFilesByKeyword(cache, keyword, relevant)
    }

    // Add dependency files
    if (options.includeDependencies) {
      const context = await Loader.get()
      if (context) {
        await addDependencyFiles(context, filePaths, relevant)
      }
    }

    return deduplicateAndPrioritize(relevant)
  }

  async function addComponentFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const component of cache.components) {
      if (component.type !== "component" && component.type !== "layout") continue
      if (relevant.some((f) => f.path === component.path)) continue

      const score = await scoreRelevance(component.name, task)
      if (score > 0.3) {
        relevant.push({
          path: component.path,
          reason: `component matching task (${score.toFixed(2)})`,
          priority: Math.round(score * 100),
          size: 0,
          lastModified: Date.now(),
        })
      }
    }
  }

  async function addApiFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const route of cache.routes) {
      if (route.type !== "api") continue
      if (relevant.some((f) => f.path === route.path)) continue

      const score = await scoreRelevance(path.basename(route.path), task)
      if (score > 0.3) {
        relevant.push({
          path: route.path,
          reason: "API route matching task",
          priority: Math.round(score * 100),
          size: 0,
          lastModified: Date.now(),
        })
      }
    }
  }

  async function addTestFiles(cache: Cache.Info, _task: string, relevant: RelevantFile[]): Promise<void> {
    for (const testPath of cache.testFiles) {
      if (relevant.some((f) => f.path === testPath)) continue

      const testBase = path.basename(testPath).replace(/\.(test|spec)\./, ".")

      for (const file of relevant) {
        const fileBase = path.basename(file.path)
        if (fileBase.replace(/\.(ts|tsx|js|jsx)$/, "") === testBase.replace(/\.(ts|tsx|js|jsx)$/, "")) {
          relevant.push({
            path: testPath,
            reason: `test for ${file.path}`,
            priority: 80,
            size: 0,
            lastModified: Date.now(),
          })
          break
        }
      }
    }
  }

  async function addConfigFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    const importantConfigs = ["tsconfig.json", "package.json", "tailwind.config.ts"]

    for (const config of cache.configs) {
      if (relevant.some((f) => f.path === config.path)) continue

      const score = await scoreRelevance(config.name, task)
      if (score > 0.2 || importantConfigs.includes(config.name)) {
        relevant.push({
          path: config.path,
          reason: "configuration file",
          priority: Math.round(score * 60) + 20,
          size: 0,
          lastModified: Date.now(),
        })
      }
    }
  }

  async function addHookFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const component of cache.components) {
      if (component.type !== "hook") continue
      if (relevant.some((f) => f.path === component.path)) continue

      const score = await scoreRelevance(component.name, task)
      if (score > 0.3) {
        relevant.push({
          path: component.path,
          reason: "hook matching task",
          priority: Math.round(score * 100),
          size: 0,
          lastModified: Date.now(),
        })
      }
    }
  }

  async function addUtilFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const component of cache.components) {
      if (component.type !== "util") continue
      if (relevant.some((f) => f.path === component.path)) continue

      const score = await scoreRelevance(component.name, task)
      if (score > 0.3) {
        relevant.push({
          path: component.path,
          reason: "utility function matching task",
          priority: Math.round(score * 90),
          size: 0,
          lastModified: Date.now(),
        })
      }
    }
  }

  async function addTypeFiles(cache: Cache.Info, relevant: RelevantFile[]): Promise<void> {
    for (const entry of Object.values(cache.entries)) {
      if (!entry.path.includes("types") && !entry.path.includes("interfaces")) continue
      if (relevant.some((f) => f.path === entry.path)) continue

      relevant.push({
        path: entry.path,
        reason: "type definition file",
        priority: 50,
        size: entry.size,
        lastModified: entry.lastModified,
      })
    }
  }

  async function addDebugFiles(cache: Cache.Info, _task: string, relevant: RelevantFile[]): Promise<void> {
    for (const entry of Object.values(cache.entries)) {
      const pathLower = entry.path.toLowerCase()
      if (!pathLower.includes("error") && !pathLower.includes("exception")) continue
      if (relevant.some((f) => f.path === entry.path)) continue

      relevant.push({
        path: entry.path,
        reason: "error handling file",
        priority: 70,
        size: entry.size,
        lastModified: entry.lastModified,
      })
    }
  }

  async function addRefactorFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    const taskLower = task.toLowerCase()
    const targetMatch = taskLower.match(/refactor\s+(\w+)/)

    if (targetMatch) {
      const target = targetMatch[1]
      for (const entry of Object.values(cache.entries)) {
        if (!entry.path.toLowerCase().includes(target)) continue
        if (relevant.some((f) => f.path === entry.path)) continue

        relevant.push({
          path: entry.path,
          reason: `target of refactor: ${target}`,
          priority: 100,
          size: entry.size,
          lastModified: entry.lastModified,
        })
      }
    }
  }

  async function addFilesByKeyword(cache: Cache.Info, keyword: string, relevant: RelevantFile[]): Promise<void> {
    const lowerKeyword = keyword.toLowerCase()

    for (const component of cache.components) {
      if (!component.name.toLowerCase().includes(lowerKeyword)) continue
      if (relevant.some((f) => f.path === component.path)) continue

      relevant.push({
        path: component.path,
        reason: `matches keyword: ${keyword}`,
        priority: 60,
        size: 0,
        lastModified: Date.now(),
      })
    }
  }

  async function addDependencyFiles(
    context: Loader.ProjectContext,
    filePaths: string[] | undefined,
    relevant: RelevantFile[],
  ): Promise<void> {
    if (!context.dependencies || !filePaths) return

    for (const filePath of filePaths) {
      const imports = context.dependencies.imports[filePath] || []
      const importedBy = context.dependencies.importedBy[filePath] || []

      for (const imp of imports) {
        if (relevant.some((f) => f.path === imp)) continue
        relevant.push({
          path: imp,
          reason: `imported by ${path.basename(filePath)}`,
          priority: 75,
          size: 0,
          lastModified: Date.now(),
        })
      }

      for (const imp of importedBy) {
        if (relevant.some((f) => f.path === imp)) continue
        relevant.push({
          path: imp,
          reason: `imports ${path.basename(filePath)}`,
          priority: 70,
          size: 0,
          lastModified: Date.now(),
        })
      }
    }
  }

  // ============================================================================
  // Utilities
  // ============================================================================

  async function loadFileWithContext(
    filePath: string,
    _task: string,
    reason: string,
  ): Promise<RelevantFile | undefined> {
    const fullPath = path.join(Instance.worktree, filePath)

    try {
      const content = await Bun.file(fullPath).text()
      const stat = await Bun.file(fullPath).stat()

      return {
        path: filePath,
        reason,
        priority: 100,
        content,
        size: stat.size,
        lastModified: stat.mtime?.getTime() ?? Date.now(),
      }
    } catch (error) {
      log.warn("failed to load file", { path: filePath, error })
      return undefined
    }
  }

  function deduplicateAndPrioritize(files: RelevantFile[]): RelevantFile[] {
    const unique = new Map<string, RelevantFile>()

    for (const file of files) {
      const existing = unique.get(file.path)
      if (!existing || file.priority > existing.priority) {
        unique.set(file.path, file)
      }
    }

    return Array.from(unique.values()).sort((a, b) => b.priority - a.priority)
  }

  async function loadFileContents(files: RelevantFile[], maxTokens?: number): Promise<RelevantFile[]> {
    const TOKENS_PER_CHAR = 0.25
    const maxChars = maxTokens ? maxTokens / TOKENS_PER_CHAR : undefined
    let totalChars = 0

    for (const file of files) {
      if (maxChars && totalChars >= maxChars) break
      if (file.content !== undefined) continue

      const fullPath = path.join(Instance.worktree, file.path)
      try {
        const content = await Bun.file(fullPath).text()
        const remainingChars = maxChars ? maxChars - totalChars : undefined

        if (remainingChars !== undefined && content.length > remainingChars) {
          file.content = content.slice(0, remainingChars) + "\n... (truncated)"
          totalChars += remainingChars
        } else {
          file.content = content
          totalChars += content.length
        }

        file.size = content.length
      } catch (error) {
        log.warn("failed to load file content", { path: file.path, error })
      }
    }

    return files
  }

  function generateSummary(files: RelevantFile[], taskTypes: string[], keywords: string[]): string {
    const parts: string[] = []
    parts.push(`Task types: ${taskTypes.join(", ")}`)

    if (keywords.length > 0) {
      parts.push(`Keywords: ${keywords.slice(0, 5).join(", ")}`)
    }

    parts.push(`Selected ${files.length} relevant files`)

    if (files.length > 0) {
      const topFiles = files.slice(0, 5)
      parts.push("\nTop files:")
      for (const file of topFiles) {
        parts.push(`  - ${file.path} (${file.reason})`)
      }
    }

    return parts.join("\n")
  }

  // ============================================================================
  // Public API
  // ============================================================================

  export async function getRelevantContext(request: ContextRequest): Promise<ContextResult> {
    const startTime = Date.now()
    const taskTypes = detectTaskType(request.task)
    const keywords = extractKeywords(request.task)
    const usingNative = await isNativeAvailable()

    log.info("finding relevant context", {
      task: request.task.slice(0, 50),
      taskTypes,
      keywords,
      nativeAvailable: usingNative,
    })

    let files = await findRelevantFiles(request.task, request.filePaths, taskTypes, keywords, request)
    files = await loadFileContents(files, request.maxTokens)

    const totalSize = files.reduce((sum, f) => sum + (f.content?.length ?? f.size), 0)
    const estimatedTokens = Math.round(totalSize * 0.25)
    const truncated = request.maxTokens ? estimatedTokens > request.maxTokens : false
    const summary = generateSummary(files, taskTypes, keywords)

    log.info("context selection complete", {
      filesCount: files.length,
      totalSize,
      estimatedTokens,
      truncated,
      duration: Date.now() - startTime,
      usingNative,
    })

    return { files, summary, totalSize, estimatedTokens, truncated }
  }

  export async function getRelatedFiles(filePath: string): Promise<string[]> {
    const cache = await Cache.get()
    const context = await Loader.get()
    const related: string[] = []

    if (!cache || !context) return related

    const fileName = path.basename(filePath)
    const baseName = fileName.replace(/\.(test|spec)\./, ".").replace(/\.[^.]+$/, "")

    for (const testPath of cache.testFiles) {
      if (testPath.includes(baseName)) {
        related.push(testPath)
      }
    }

    if (context.dependencies) {
      const imports = context.dependencies.imports[filePath] || []
      const importedBy = context.dependencies.importedBy[filePath] || []
      related.push(...imports, ...importedBy)
    }

    for (const component of cache.components) {
      if (component.imports?.some((imp) => imp.includes(baseName))) {
        related.push(component.path)
      }
    }

    return [...new Set(related)].filter((p) => p !== filePath)
  }

  export async function getContextForEdit(filePath: string): Promise<RelevantFile[]> {
    const related = await getRelatedFiles(filePath)
    const request: ContextRequest = {
      task: `edit ${path.basename(filePath)}`,
      filePaths: [filePath, ...related.slice(0, 5)],
      includeTests: true,
      includeDependencies: true,
    }

    const result = await getRelevantContext(request)
    return result.files
  }

  /**
   * Check if native relevance implementation is being used
   */
  export function isUsingNative(): boolean {
    return isNativeRelevanceAvailable()
  }

  /**
   * Compute content hash for deduplication using native implementation
   * @throws Error if native bindings unavailable
   */
  export async function contentHash(content: string): Promise<string> {
    return contentHashNative(content)
  }
}
