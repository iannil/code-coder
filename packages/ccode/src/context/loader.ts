import { Log } from "@/util/log"
import { Filesystem } from "@/util/filesystem"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { Fingerprint } from "./fingerprint"
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

  const IGNORED_DIRECTORIES = [
    "node_modules",
    ".git",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    "target",
    "bin",
    "obj",
    ".vscode",
    ".idea",
    "coverage",
    ".cache",
    ".turbo",
    ".sst",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    "env",
  ]

  const CONFIG_FILE_PATTERNS = [
    "*.config.js",
    "*.config.ts",
    "*.config.json",
    "*.config.mjs",
    ".*rc*",
    "tsconfig.json",
    "package.json",
    "pyproject.toml",
    "go.mod",
    "Cargo.toml",
    "*.toml",
    "*.yaml",
    "*.yml",
    ".env*",
  ]

  function isIgnoredDirectory(name: string): boolean {
    return IGNORED_DIRECTORIES.includes(name) || name.startsWith(".")
  }

  async function scanDirectory(
    dirPath: string,
    relativePath: string = "",
    maxDepth: number = 10,
    currentDepth: number = 0,
  ): Promise<{ entries: FileEntry[]; structure: DirectoryStructure }> {
    if (currentDepth >= maxDepth)
      return { entries: [], structure: { path: dirPath, name: path.basename(dirPath), files: [], subdirectories: [] } }

    const entries: FileEntry[] = []
    const files: string[] = []
    const subdirs: DirectoryStructure[] = []

    try {
      const glob = new Bun.Glob("*")
      for await (const item of glob.scan({
        cwd: dirPath,
        absolute: false,
      })) {
        const fullPath = path.join(dirPath, item)
        const itemRelativePath = path.join(relativePath, item)

        try {
          const stat = await Bun.file(fullPath).stat()
          const entry: FileEntry = {
            path: fullPath,
            relativePath: itemRelativePath,
            name: item,
            extension: item.includes(".") ? item.split(".").pop() : undefined,
            directory: stat.isDirectory(),
            size: stat.size,
            lastModified: stat.mtime?.getTime() ?? Date.now(),
          }

          if (stat.isDirectory()) {
            if (isIgnoredDirectory(item)) continue
            const subResult = await scanDirectory(fullPath, itemRelativePath, maxDepth, currentDepth + 1)
            entries.push(...subResult.entries)
            subdirs.push(subResult.structure)
          } else {
            entries.push(entry)
            files.push(item)
          }
        } catch {}
      }
    } catch (error) {
      log.warn("failed to scan directory", { path: dirPath, error })
    }

    return {
      entries,
      structure: {
        path: dirPath,
        name: path.basename(dirPath),
        files,
        subdirectories: subdirs,
      },
    }
  }

  function categorizeFiles(entries: FileEntry[], fingerprint: Fingerprint.Info): FileIndex {
    const byPath: Record<string, FileEntry> = {}
    const byExtension: Record<string, string[]> = {}
    const byName: Record<string, string[]> = {}

    const routes: string[] = []
    const components: string[] = []
    const tests: string[] = []
    const configs: string[] = []

    for (const entry of entries) {
      if (entry.directory) continue

      byPath[entry.relativePath] = entry

      if (entry.extension) {
        if (!byExtension[entry.extension]) byExtension[entry.extension] = []
        byExtension[entry.extension].push(entry.relativePath)
      }

      if (!byName[entry.name]) byName[entry.name] = []
      byName[entry.name].push(entry.relativePath)

      for (const testDir of fingerprint.directories.tests) {
        if (entry.relativePath.startsWith(testDir) || entry.relativePath.includes(`${testDir}/`)) {
          tests.push(entry.relativePath)
          break
        }
      }

      if (entry.name.includes(".test.") || entry.name.includes(".spec.") || entry.name.includes("_test.")) {
        if (!tests.includes(entry.relativePath)) tests.push(entry.relativePath)
      }

      if (entry.name.includes(".config.") || entry.name.startsWith(".") || entry.name === "tsconfig.json") {
        configs.push(entry.relativePath)
      }
    }

    const componentPatterns = [fingerprint.directories.components, "components", "Components", "src/components"].filter(
      Boolean,
    ) as string[]

    const routePatterns = [
      fingerprint.directories.routes,
      fingerprint.directories.pages,
      "routes",
      "pages",
      "src/routes",
      "src/pages",
      "app/routes",
      "app/pages",
    ].filter(Boolean) as string[]

    for (const entryPath in byPath) {
      const entry = byPath[entryPath]

      for (const pattern of componentPatterns) {
        if (entry.relativePath.startsWith(pattern + "/") || entry.relativePath.startsWith(pattern + "\\")) {
          if (!components.includes(entry.relativePath)) components.push(entry.relativePath)
          break
        }
      }

      for (const pattern of routePatterns) {
        if (entry.relativePath.startsWith(pattern + "/") || entry.relativePath.startsWith(pattern + "\\")) {
          if (!routes.includes(entry.relativePath)) routes.push(entry.relativePath)
          break
        }
      }
    }

    return {
      byPath,
      byExtension,
      byName,
      routes,
      components,
      tests,
      configs,
    }
  }

  async function extractDependencies(
    entries: FileEntry[],
    language: ProjectContext["fingerprint"]["language"],
  ): Promise<DependencyGraph> {
    const imports: Record<string, string[]> = {}
    const importedBy: Record<string, string[]> = {}

    const codeExtensions = {
      typescript: ["ts", "tsx"],
      javascript: ["js", "jsx", "mjs"],
      python: ["py"],
      go: ["go"],
      rust: ["rs"],
      csharp: ["cs"],
      java: ["java"],
      other: ["ts", "tsx", "js", "jsx"],
    }[language] || ["ts", "tsx", "js", "jsx"]

    for (const entry of entries) {
      if (entry.directory || !entry.extension) continue
      if (!codeExtensions.includes(entry.extension)) continue

      try {
        const content = await Bun.file(entry.path).text()
        const extractedImports = extractImportPaths(content, entry.extension, entry.relativePath)

        if (extractedImports.length > 0) {
          imports[entry.relativePath] = extractedImports
          for (const imp of extractedImports) {
            if (!importedBy[imp]) importedBy[imp] = []
            if (!importedBy[imp].includes(entry.relativePath)) {
              importedBy[imp].push(entry.relativePath)
            }
          }
        }
      } catch {}
    }

    return { imports, importedBy }
  }

  function extractImportPaths(content: string, extension: string, filePath: string): string[] {
    const imports: string[] = []
    const relativePath = path.dirname(filePath)

    if (extension === "ts" || extension === "tsx" || extension === "js" || extension === "jsx" || extension === "mjs") {
      const importPatterns = [
        /import\s+.*?\s+from\s+['"]([^'"]+)['"]/g,
        /import\(['"]([^'"]+)['"]\)/g,
        /require\(['"]([^'"]+)['"]\)/g,
        /export\s+.*?\s+from\s+['"]([^'"]+)['"]/g,
      ]

      for (const pattern of importPatterns) {
        let match
        while ((match = pattern.exec(content)) !== null) {
          const imp = match[1]
          if (imp.startsWith(".") || imp.startsWith("/")) {
            let resolved = path.join(relativePath, imp)
            if (!resolved.endsWith(extension)) {
              resolved = resolved + "." + extension
            }
            imports.push(resolved.replace(/\\/g, "/"))
          }
        }
      }
    }

    if (extension === "py") {
      const importPatterns = [/from\s+([^\s]+)\s+import/g, /import\s+([^\s]+)/g]

      for (const pattern of importPatterns) {
        let match
        while ((match = pattern.exec(content)) !== null) {
          const imp = match[1]
          if (!imp.startsWith(".")) continue
          imports.push(imp.replace(/\./g, "/") + ".py")
        }
      }
    }

    if (extension === "go") {
      const importPattern = /import\s+['"]([^'"]+)['"]/g
      let match
      while ((match = importPattern.exec(content)) !== null) {
        const imp = match[1]
        if (imp.startsWith(".") || imp.startsWith("./")) {
          imports.push(imp.replace("./", "") + ".go")
        }
      }
    }

    return imports
  }

  export async function analyze(options?: {
    maxDepth?: number
    includeDependencies?: boolean
  }): Promise<ProjectContext> {
    const worktree = Instance.worktree
    const projectID = Instance.project.id
    const now = Date.now()

    log.info("analyzing project", { projectID, worktree })

    const fingerprint = await Fingerprint.load()

    const { entries, structure } = await scanDirectory(worktree, "", options?.maxDepth ?? 10, 0)

    const index = categorizeFiles(entries, fingerprint)

    let dependencies: DependencyGraph | undefined
    if (
      (options?.includeDependencies !== false && fingerprint.hasTypeScript) ||
      fingerprint.language === "javascript"
    ) {
      try {
        dependencies = await extractDependencies(entries, fingerprint.language)
      } catch (error) {
        log.warn("failed to extract dependencies", { error })
      }
    }

    const result: ProjectContext = {
      projectID,
      fingerprint,
      structure,
      index,
      dependencies,
      time: {
        created: now,
        updated: now,
      },
    }

    log.info("project analysis complete", {
      projectID,
      filesCount: Object.keys(index.byPath).length,
      componentsCount: index.components.length,
      routesCount: index.routes.length,
      testsCount: index.tests.length,
      configsCount: index.configs.length,
    })

    return result
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

  export async function findRelatedFiles(filePath: string, context: ProjectContext): Promise<string[]> {
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

    if (context.dependencies) {
      const imports = context.dependencies.imports[filePath] || []
      const importedBy = context.dependencies.importedBy[filePath] || []
      related.push(...imports, ...importedBy)
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
