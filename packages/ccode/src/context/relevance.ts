import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Fingerprint } from "./fingerprint"
import { Loader } from "./loader"
import { Cache } from "./cache"
import path from "path"
import z from "zod"

const log = Log.create({ service: "context.relevance" })

export namespace Relevance {
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

  const KEYWORD_PATTERNS = {
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
      for (const keyword of keywords) {
        if (taskLower.includes(keyword)) {
          detected.push(type)
          break
        }
      }
    }

    if (taskLower.includes("bug") || taskLower.includes("fix") || taskLower.includes("error")) {
      detected.push("debug")
    }

    if (taskLower.includes("refactor") || taskLower.includes("clean") || taskLower.includes("organize")) {
      detected.push("refactor")
    }

    if (
      taskLower.includes("add") ||
      taskLower.includes("create") ||
      taskLower.includes("implement") ||
      taskLower.includes("build")
    ) {
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

  async function findRelevantFiles(
    task: string,
    filePaths: string[] | undefined,
    taskTypes: string[],
    keywords: string[],
    options: ContextRequest,
  ): Promise<RelevantFile[]> {
    const relevant: RelevantFile[] = []

    if (filePaths && filePaths.length > 0) {
      for (const filePath of filePaths) {
        const file = await loadFileWithContext(filePath, task, "directly_referenced")
        if (file) relevant.push(file)
      }
    }

    const cache = await Cache.get()
    const fingerprint = await Fingerprint.get()
    const context = await Loader.get()

    if (!cache) {
      log.warn("cache not available, skipping context enrichment")
      return relevant
    }

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
          await addTypeFiles(cache, task, relevant)
          break
        case "debug":
          await addDebugFiles(cache, context, task, relevant)
          break
        case "refactor":
          await addRefactorFiles(cache, context, task, relevant)
          break
      }
    }

    for (const keyword of keywords) {
      await addFilesByKeyword(cache, keyword, relevant)
    }

    if (options.includeDependencies && context) {
      await addDependencyFiles(context, filePaths, relevant)
    }

    return deduplicateAndPrioritize(relevant)
  }

  async function addComponentFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const component of cache.components) {
      if (component.type === "component" || component.type === "layout") {
        const existing = relevant.find((f) => f.path === component.path)
        if (existing) continue

        const score = calculateRelevanceScore(component.name, task)
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
  }

  async function addApiFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const route of cache.routes) {
      if (route.type === "api") {
        const existing = relevant.find((f) => f.path === route.path)
        if (existing) continue

        const score = calculateRelevanceScore(path.basename(route.path), task)
        if (score > 0.3) {
          relevant.push({
            path: route.path,
            reason: `API route matching task`,
            priority: Math.round(score * 100),
            size: 0,
            lastModified: Date.now(),
          })
        }
      }
    }
  }

  async function addTestFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const testPath of cache.testFiles) {
      const existing = relevant.find((f) => f.path === testPath)
      if (existing) continue

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
    for (const config of cache.configs) {
      const existing = relevant.find((f) => f.path === config.path)
      if (existing) continue

      const score = calculateRelevanceScore(config.name, task)
      if (score > 0.2 || ["tsconfig.json", "package.json", "tailwind.config.ts"].includes(config.name)) {
        relevant.push({
          path: config.path,
          reason: `configuration file`,
          priority: Math.round(score * 60) + 20,
          size: 0,
          lastModified: Date.now(),
        })
      }
    }
  }

  async function addHookFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const component of cache.components) {
      if (component.type === "hook") {
        const existing = relevant.find((f) => f.path === component.path)
        if (existing) continue

        const score = calculateRelevanceScore(component.name, task)
        if (score > 0.3) {
          relevant.push({
            path: component.path,
            reason: `hook matching task`,
            priority: Math.round(score * 100),
            size: 0,
            lastModified: Date.now(),
          })
        }
      }
    }
  }

  async function addUtilFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const component of cache.components) {
      if (component.type === "util") {
        const existing = relevant.find((f) => f.path === component.path)
        if (existing) continue

        const score = calculateRelevanceScore(component.name, task)
        if (score > 0.3) {
          relevant.push({
            path: component.path,
            reason: `utility function matching task`,
            priority: Math.round(score * 90),
            size: 0,
            lastModified: Date.now(),
          })
        }
      }
    }
  }

  async function addTypeFiles(cache: Cache.Info, task: string, relevant: RelevantFile[]): Promise<void> {
    for (const entry of Object.values(cache.entries)) {
      if (entry.path.includes("types") || entry.path.includes("interfaces")) {
        const existing = relevant.find((f) => f.path === entry.path)
        if (existing) continue

        relevant.push({
          path: entry.path,
          reason: `type definition file`,
          priority: 50,
          size: entry.size,
          lastModified: entry.lastModified,
        })
      }
    }
  }

  async function addDebugFiles(
    cache: Cache.Info,
    context: Loader.ProjectContext | undefined,
    task: string,
    relevant: RelevantFile[],
  ): Promise<void> {
    if (!context) return

    for (const filePath in context.index.byPath) {
      if (filePath.toLowerCase().includes("error") || filePath.toLowerCase().includes("exception")) {
        relevant.push({
          path: filePath,
          reason: "error handling file",
          priority: 70,
          size: 0,
          lastModified: Date.now(),
        })
      }
    }
  }

  async function addRefactorFiles(
    cache: Cache.Info,
    context: Loader.ProjectContext | undefined,
    task: string,
    relevant: RelevantFile[],
  ): Promise<void> {
    if (!context) return

    const taskLower = task.toLowerCase()
    const targetMatch = taskLower.match(/refactor\s+(\w+)/)

    if (targetMatch) {
      const target = targetMatch[1]
      for (const filePath in context.index.byPath) {
        if (filePath.toLowerCase().includes(target)) {
          relevant.push({
            path: filePath,
            reason: `target of refactor: ${target}`,
            priority: 100,
            size: 0,
            lastModified: Date.now(),
          })
        }
      }
    }
  }

  async function addFilesByKeyword(cache: Cache.Info, keyword: string, relevant: RelevantFile[]): Promise<void> {
    const lowerKeyword = keyword.toLowerCase()

    for (const component of cache.components) {
      if (component.name.toLowerCase().includes(lowerKeyword)) {
        const existing = relevant.find((f) => f.path === component.path)
        if (!existing) {
          relevant.push({
            path: component.path,
            reason: `matches keyword: ${keyword}`,
            priority: 60,
            size: 0,
            lastModified: Date.now(),
          })
        }
      }
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
        const existing = relevant.find((f) => f.path === imp)
        if (!existing) {
          relevant.push({
            path: imp,
            reason: `imported by ${path.basename(filePath)}`,
            priority: 75,
            size: 0,
            lastModified: Date.now(),
          })
        }
      }

      for (const imp of importedBy) {
        const existing = relevant.find((f) => f.path === imp)
        if (!existing) {
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
  }

  async function loadFileWithContext(
    filePath: string,
    task: string,
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

  function calculateRelevanceScore(target: string, query: string): number {
    const targetLower = target.toLowerCase()
    const queryLower = query.toLowerCase()

    if (targetLower === queryLower) return 1.0

    if (targetLower.includes(queryLower) || queryLower.includes(targetLower)) {
      return 0.8
    }

    const targetWords = targetLower.split(/[^a-z0-9]+/).filter(Boolean)
    const queryWords = queryLower.split(/[^a-z0-9]+/).filter(Boolean)

    let matches = 0
    for (const queryWord of queryWords) {
      if (targetWords.includes(queryWord)) matches++
    }

    if (matches > 0) {
      return matches / Math.max(queryWords.length, 1)
    }

    const targetInitials = targetWords.map((w) => w[0]).join("")
    const queryInitials = queryWords.map((w) => w[0]).join("")

    if (targetLower.startsWith(queryInitials) || queryLower.startsWith(targetInitials)) {
      return 0.5
    }

    return 0
  }

  function deduplicateAndPrioritize(files: RelevantFile[]): RelevantFile[] {
    const unique = new Map<string, RelevantFile>()

    for (const file of files) {
      const existing = unique.get(file.path)
      if (existing) {
        if (file.priority > existing.priority) {
          unique.set(file.path, file)
        }
      } else {
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
      if (maxChars && totalChars >= maxChars) {
        break
      }

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

  export async function getRelevantContext(request: ContextRequest): Promise<ContextResult> {
    const startTime = Date.now()
    const taskTypes = detectTaskType(request.task)
    const keywords = extractKeywords(request.task)

    log.info("finding relevant context", {
      task: request.task.slice(0, 50),
      taskTypes,
      keywords,
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
    })

    return {
      files,
      summary,
      totalSize,
      estimatedTokens,
      truncated,
    }
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
}
