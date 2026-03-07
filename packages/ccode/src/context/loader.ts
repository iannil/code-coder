import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { Fingerprint } from "./fingerprint"
import {
  createContextLoader,
  type NapiFileIndex,
  type NapiDependencyGraph,
  type NapiFrameworkType,
  type NapiPackageManager,
  type NapiProjectLanguage,
} from "@codecoder-ai/core"
import path from "path"
import z from "zod"

const log = Log.create({ service: "context.loader" })

export namespace Loader {
  export const FileEntry = z.object({
    path: z.string(),
    relativePath: z.string(),
    name: z.string(),
    extension: z.string().optional(),
    directory: z.boolean(),
    size: z.number(),
    lastModified: z.number(),
  })
  export type FileEntry = z.infer<typeof FileEntry>

  export const DirectoryStructure: z.ZodObject<{
    path: z.ZodString
    name: z.ZodString
    files: z.ZodArray<z.ZodString>
    subdirectories: z.ZodArray<any>
  }> = z.object({
    path: z.string(),
    name: z.string(),
    files: z.array(z.string()),
    subdirectories: z.array(z.lazy(() => DirectoryStructure)),
  })
  export type DirectoryStructure = z.infer<typeof DirectoryStructure>

  export const FileIndex = z.object({
    byPath: z.record(z.string(), FileEntry),
    byExtension: z.record(z.string(), z.array(z.string())),
    byName: z.record(z.string(), z.array(z.string())),
    routes: z.array(z.string()),
    components: z.array(z.string()),
    tests: z.array(z.string()),
    configs: z.array(z.string()),
  })
  export type FileIndex = z.infer<typeof FileIndex>

  export const DependencyGraph = z.object({
    imports: z.record(z.string(), z.array(z.string())),
    importedBy: z.record(z.string(), z.array(z.string())),
  })
  export type DependencyGraph = z.infer<typeof DependencyGraph>

  export const ProjectContext = z.object({
    projectID: z.string(),
    fingerprint: Fingerprint.Info,
    structure: DirectoryStructure.optional(),
    index: FileIndex,
    dependencies: DependencyGraph.optional(),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type ProjectContext = z.infer<typeof ProjectContext>

  async function analyze(options?: {
    maxDepth?: number
    includeDependencies?: boolean
  }): Promise<ProjectContext> {
    if (!createContextLoader) {
      throw new Error("Native context loader not available. Please ensure @codecoder-ai/core is properly built.")
    }

    const worktree = Instance.worktree
    const projectID = Instance.project.id
    const now = Date.now()

    log.info("analyzing project (native)", { projectID, worktree })

    const fingerprint = await Fingerprint.load()

    // Use native context loader
    const loader = createContextLoader(worktree, {
      maxDepth: options?.maxDepth ?? 10,
      includeHidden: false,
    })

    // Scan directory
    const scanResult = loader.scan()

    // Mapping from lowercase to NAPI PascalCase
    const frameworkTypeMap: Record<string, NapiFrameworkType> = {
      frontend: "Frontend" as NapiFrameworkType,
      backend: "Backend" as NapiFrameworkType,
      fullstack: "Fullstack" as NapiFrameworkType,
      mobile: "Mobile" as NapiFrameworkType,
      desktop: "Desktop" as NapiFrameworkType,
      cli: "Cli" as NapiFrameworkType,
      library: "Library" as NapiFrameworkType,
    }

    const managerMap: Record<string, NapiPackageManager> = {
      npm: "Npm" as NapiPackageManager,
      bun: "Bun" as NapiPackageManager,
      yarn: "Yarn" as NapiPackageManager,
      pnpm: "Pnpm" as NapiPackageManager,
      unknown: "Unknown" as NapiPackageManager,
    }

    const languageMap: Record<string, NapiProjectLanguage> = {
      typescript: "TypeScript" as NapiProjectLanguage,
      javascript: "JavaScript" as NapiProjectLanguage,
      python: "Python" as NapiProjectLanguage,
      go: "Go" as NapiProjectLanguage,
      rust: "Rust" as NapiProjectLanguage,
      java: "Java" as NapiProjectLanguage,
      csharp: "CSharp" as NapiProjectLanguage,
      other: "Other" as NapiProjectLanguage,
    }

    // Convert fingerprint to native format for categorization
    const nativeFingerprint = {
      projectId: fingerprint.projectID,
      frameworks: fingerprint.frameworks.map((f) => ({
        name: f.name,
        version: f.version ?? null,
        frameworkType: frameworkTypeMap[f.type] ?? ("Library" as NapiFrameworkType),
      })),
      buildTools: fingerprint.buildTools.map((t) => ({
        name: t.name,
        config: t.config ?? null,
        version: t.version ?? null,
      })),
      testFrameworks: fingerprint.testFrameworks.map((t) => ({
        name: t.name,
        config: t.config ?? null,
        runner: t.runner ?? null,
      })),
      package: {
        name: fingerprint.package.name ?? null,
        version: fingerprint.package.version ?? null,
        manager: managerMap[fingerprint.package.manager] ?? ("Unknown" as NapiPackageManager),
      },
      configs: fingerprint.configs.map((c) => ({ path: c.path, name: c.name })),
      language: languageMap[fingerprint.language] ?? ("Other" as NapiProjectLanguage),
      hasTypescript: fingerprint.hasTypeScript,
      languageVersion: fingerprint.languageVersion ?? null,
      directories: {
        src: fingerprint.directories.src ?? null,
        components: fingerprint.directories.components ?? null,
        pages: fingerprint.directories.pages ?? null,
        routes: fingerprint.directories.routes ?? null,
        tests: fingerprint.directories.tests,
        lib: fingerprint.directories.lib ?? null,
        dist: fingerprint.directories.dist ?? null,
        build: fingerprint.directories.build ?? null,
        public: fingerprint.directories.public ?? null,
      },
      // Hash is computed from the fingerprint data
      hash: `${fingerprint.projectID}-${fingerprint.language}-${fingerprint.frameworks.length}`,
    }

    // Cast to native fingerprint type expected by setFingerprint
    // The TypeScript and native types are structurally compatible but named differently
    loader.setFingerprint(nativeFingerprint as Parameters<typeof loader.setFingerprint>[0])

    // Categorize files
    const index = loader.categorize(scanResult.entries)

    // Extract dependencies if needed
    let dependencies: DependencyGraph | undefined
    if (
      (options?.includeDependencies !== false && fingerprint.hasTypeScript) ||
      fingerprint.language === "javascript"
    ) {
      try {
        // Use the mapped language value for NAPI
        const napiLanguage = languageMap[fingerprint.language] ?? ("Other" as NapiProjectLanguage)
        dependencies = loader.extractDependencies(scanResult.entries, napiLanguage)
      } catch (error) {
        log.warn("failed to extract dependencies", { error })
      }
    }

    const result: ProjectContext = {
      projectID,
      fingerprint,
      structure: scanResult.structure,
      index,
      dependencies,
      time: {
        created: now,
        updated: now,
      },
    }

    log.info("project analysis complete (native)", {
      projectID,
      filesCount: Object.keys(index.byPath).length,
      componentsCount: index.components.length,
      routesCount: index.routes.length,
      testsCount: index.tests.length,
      configsCount: index.configs.length,
    })

    return result
  }

  // ==========================================================================
  // Public API
  // ==========================================================================

  export async function analyzeProject(options?: {
    maxDepth?: number
    includeDependencies?: boolean
  }): Promise<ProjectContext> {
    return analyze(options)
  }

  export async function get(): Promise<ProjectContext | undefined> {
    const projectID = Instance.project.id
    try {
      const stored = await Storage.read<ProjectContext>(["context", "loader", projectID])
      if (stored) {
        return stored
      }
    } catch {
      return undefined
    }
  }

  export async function load(options?: {
    force?: boolean
    maxDepth?: number
    includeDependencies?: boolean
  }): Promise<ProjectContext> {
    let existing = await get()

    if (!existing || options?.force) {
      existing = await analyze(options)
      await save(existing)
    }

    return existing
  }

  export async function save(context: ProjectContext): Promise<void> {
    const projectID = Instance.project.id
    context.time.updated = Date.now()
    await Storage.write(["context", "loader", projectID], context)
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["context", "loader", projectID])
  }

  export function findRelatedFiles(filePath: string, context: ProjectContext): string[] {
    // Use native implementation for dependency-based lookup
    if (createContextLoader && context.dependencies) {
      const loader = createContextLoader(Instance.worktree)
      return loader.findRelatedFiles(filePath, context.index as NapiFileIndex, context.dependencies as NapiDependencyGraph)
    }

    // Index-based lookup when dependencies not available
    const related: string[] = []
    const entry = context.index.byPath[filePath]
    if (!entry) return related

    const ext = entry.extension
    const dir = path.dirname(filePath)

    if (ext) {
      const sameExtension = context.index.byExtension[ext] || []
      for (const otherPath of sameExtension) {
        if (path.dirname(otherPath) === dir) {
          related.push(otherPath)
        }
      }
    }

    const testName = entry.name.replace(/\.(ts|tsx|js|jsx|py)$/, ".test.$1")
    if (context.index.byName[testName]) {
      related.push(...context.index.byName[testName])
    }

    const specName = entry.name.replace(/\.(ts|tsx|js|jsx)$/, ".spec.$1")
    if (context.index.byName[specName]) {
      related.push(...context.index.byName[specName])
    }

    return [...new Set(related)].filter((p) => p !== filePath)
  }
}
