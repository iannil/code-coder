import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import path from "path"
import z from "zod"

const log = Log.create({ service: "memory.knowledge.code-index" })

export namespace CodeIndex {
  export const FunctionInfo = z.object({
    name: z.string(),
    file: z.string(),
    line: z.number(),
    type: z.enum(["function", "method", "arrow", "async"]),
    signature: z.string().optional(),
    exported: z.boolean().optional(),
    parameters: z
      .array(
        z.object({
          name: z.string(),
          type: z.string().optional(),
        }),
      )
      .optional(),
    returnType: z.string().optional(),
  })
  export type FunctionInfo = z.infer<typeof FunctionInfo>

  export const ClassInfo = z.object({
    name: z.string(),
    file: z.string(),
    line: z.number(),
    extends: z.string().optional(),
    implements: z.array(z.string()).optional(),
    methods: z.array(z.string()),
    properties: z.array(z.string()),
    exported: z.boolean().optional(),
  })
  export type ClassInfo = z.infer<typeof ClassInfo>

  export const InterfaceInfo = z.object({
    name: z.string(),
    file: z.string(),
    line: z.number(),
    extends: z.array(z.string()).optional(),
    properties: z.array(
      z.object({
        name: z.string(),
        type: z.string(),
        optional: z.boolean(),
      }),
    ),
    exported: z.boolean().optional(),
  })
  export type InterfaceInfo = z.infer<typeof InterfaceInfo>

  export const TypeInfo = z.object({
    name: z.string(),
    file: z.string(),
    line: z.number(),
    kind: z.enum(["type", "enum", "interface"]),
    definition: z.string().optional(),
    exported: z.boolean().optional(),
  })
  export type TypeInfo = z.infer<typeof TypeInfo>

  export const ImportInfo = z.object({
    source: z.string(),
    file: z.string(),
    line: z.number(),
    imports: z.array(z.string()),
    isExternal: z.boolean().optional(),
    isDynamic: z.boolean().optional(),
  })
  export type ImportInfo = z.infer<typeof ImportInfo>

  export const ExportInfo = z.object({
    name: z.string(),
    file: z.string(),
    line: z.number(),
    type: z.enum(["default", "named", "all"]),
  })
  export type ExportInfo = z.infer<typeof ExportInfo>

  export const Index = z.object({
    projectID: z.string(),
    functions: z.array(FunctionInfo),
    classes: z.array(ClassInfo),
    interfaces: z.array(InterfaceInfo),
    types: z.array(TypeInfo),
    imports: z.array(ImportInfo),
    exports: z.array(ExportInfo),
    fileIndex: z.record(z.string(), z.array(z.string())),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type Index = z.infer<typeof Index>

  const FUNCTION_PATTERNS = {
    named: /(?:function\s+(\w+)|(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?(?:function|\([^)]*\)\s*=>))/g,
    method: /(\w+)\s*\([^)]*\)\s*{/g,
    arrow: /(?:const|let|var)\s+(\w+)\s*=\s*(?:\([^)]*\)|\w+)\s*=>/g,
  }

  const CLASS_PATTERN = /(?:class\s+(\w+)|(?:interface|type)\s+(\w+))/g
  const INTERFACE_PATTERN = /interface\s+(\w+)(?:\s+extends\s+([\w\s,]+))?\s*{/g
  const TYPE_PATTERN = /type\s+(\w+)\s*=\s*([^;]+);/g
  const ENUM_PATTERN = /enum\s+(\w+)\s*{/g

  const IMPORT_PATTERN = /import\s+(?:(\{[^}]+\})|(\w+)|(?:\*\s+as\s+(\w+)))\s+from\s+['"]([^'"]+)['"]/g
  const DYNAMIC_IMPORT_PATTERN = /import\s*\(\s*['"]([^'"]+)['"]\s*\)/g
  const EXPORT_PATTERN =
    /export\s+(?:(default\s+)?(?:class|function|const|let|var)\s+(\w+)|(?:\{([^}]+)\})\s+from\s+['"]([^'"]+)['"])/g

  export async function get(): Promise<Index | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<Index>(["memory", "knowledge", "code-index", projectID])
    } catch {
      return undefined
    }
  }

  export async function build(): Promise<Index> {
    const projectID = Instance.project.id
    const now = Date.now()

    log.info("building code index", { projectID })

    const functions: FunctionInfo[] = []
    const classes: ClassInfo[] = []
    const interfaces: InterfaceInfo[] = []
    const types: TypeInfo[] = []
    const imports: ImportInfo[] = []
    const exports: ExportInfo[] = []
    const fileIndex: Record<string, string[]> = {}

    const codeFiles = await findCodeFiles()

    for (const filePath of codeFiles) {
      const relativePath = path.relative(Instance.worktree, filePath).replace(/\\/g, "/")
      fileIndex[relativePath] = []

      try {
        const content = await Bun.file(filePath).text()
        const lines = content.split("\n")

        await indexFile(relativePath, content, lines, {
          functions,
          classes,
          interfaces,
          types,
          imports,
          exports,
          fileIndex,
        })
      } catch (error) {
        log.warn("error indexing file", { path: relativePath, error })
      }
    }

    const result: Index = {
      projectID,
      functions,
      classes,
      interfaces,
      types,
      imports,
      exports,
      fileIndex,
      time: {
        created: now,
        updated: now,
      },
    }

    log.info("code index built", {
      functionsCount: functions.length,
      classesCount: classes.length,
      interfacesCount: interfaces.length,
      typesCount: types.length,
      importsCount: imports.length,
      exportsCount: exports.length,
      filesCount: Object.keys(fileIndex).length,
    })

    await save(result)
    return result
  }

  async function findCodeFiles(): Promise<string[]> {
    const worktree = Instance.worktree
    const files: string[] = []

    const patterns = ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"]

    for (const pattern of patterns) {
      try {
        const matches = await Array.fromAsync(
          new Bun.Glob(pattern).scan({
            cwd: worktree,
            absolute: true,
          }),
        )
        files.push(...matches)
      } catch {}
    }

    return files.filter((file) => {
      return (
        !file.includes("node_modules") &&
        !file.includes(".next") &&
        !file.includes("dist") &&
        !file.includes("build") &&
        !file.includes(".git")
      )
    })
  }

  async function indexFile(
    filePath: string,
    content: string,
    lines: string[],
    output: {
      functions: FunctionInfo[]
      classes: ClassInfo[]
      interfaces: InterfaceInfo[]
      types: TypeInfo[]
      imports: ImportInfo[]
      exports: ExportInfo[]
      fileIndex: Record<string, string[]>
    },
  ): Promise<void> {
    let match

    while ((match = IMPORT_PATTERN.exec(content)) !== null) {
      const [, namedImports, defaultImport, namespaceImport, source] = match
      const imports: string[] = []
      let isExternal = !source.startsWith(".")

      if (namedImports) {
        imports.push(
          ...namedImports
            .replace(/[{}]/g, "")
            .split(",")
            .map((s) => s.trim().split(" as ")[0]),
        )
      } else if (defaultImport) {
        imports.push(defaultImport)
      } else if (namespaceImport) {
        imports.push(namespaceImport)
      }

      output.imports.push({
        source,
        file: filePath,
        line: content.substring(0, match.index).split("\n").length,
        imports,
        isExternal,
        isDynamic: false,
      })

      output.fileIndex[filePath].push(...imports)
    }

    let dynamicMatch
    while ((dynamicMatch = DYNAMIC_IMPORT_PATTERN.exec(content)) !== null) {
      const [, source] = dynamicMatch
      output.imports.push({
        source,
        file: filePath,
        line: content.substring(0, dynamicMatch.index).split("\n").length,
        imports: [],
        isExternal: !source.startsWith("."),
        isDynamic: true,
      })
    }

    while ((match = INTERFACE_PATTERN.exec(content)) !== null) {
      const [, name, extendsClause] = match
      const line = content.substring(0, match.index).split("\n").length

      const properties = extractProperties(content, match.index)

      output.interfaces.push({
        name,
        file: filePath,
        line,
        extends: extendsClause ? extendsClause.split(",").map((s) => s.trim()) : undefined,
        properties,
        exported: content.substring(Math.max(0, match.index - 50), match.index).includes("export"),
      })

      output.fileIndex[filePath].push(name)
    }

    while ((match = TYPE_PATTERN.exec(content)) !== null) {
      const [, name, definition] = match
      const line = content.substring(0, match.index).split("\n").length

      output.types.push({
        name,
        file: filePath,
        line,
        kind: "type",
        definition: definition.trim(),
        exported: content.substring(Math.max(0, match.index - 50), match.index).includes("export"),
      })

      output.fileIndex[filePath].push(name)
    }

    while ((match = ENUM_PATTERN.exec(content)) !== null) {
      const [, name] = match
      const line = content.substring(0, match.index).split("\n").length

      output.types.push({
        name,
        file: filePath,
        line,
        kind: "enum",
        exported: content.substring(Math.max(0, match.index - 50), match.index).includes("export"),
      })

      output.fileIndex[filePath].push(name)
    }
  }

  function extractProperties(
    content: string,
    startIndex: number,
  ): Array<{ name: string; type: string; optional: boolean }> {
    const properties: Array<{ name: string; type: string; optional: boolean }> = []

    let depth = 1
    let current = ""
    let i = startIndex + content.substring(startIndex).indexOf("{")

    while (i < content.length && depth > 0) {
      const char = content[i]
      current += char

      if (char === "{") depth++
      else if (char === "}") depth--
      else if (char === ";" && depth === 1) {
        const prop = parseProperty(current.trim().slice(0, -1))
        if (prop) properties.push(prop)
        current = ""
      }

      i++
    }

    if (current.trim().endsWith("}")) {
      const prop = parseProperty(current.trim().slice(0, -1))
      if (prop) properties.push(prop)
    }

    return properties
  }

  function parseProperty(str: string): { name: string; type: string; optional: boolean } | null {
    const match = str.match(/^(\w+)(\?)?(?:\s*:\s*([^=]+))?/)
    if (!match) return null

    const [, name, optional, type] = match
    return {
      name,
      type: type?.trim() || "unknown",
      optional: !!optional,
    }
  }

  export async function save(index: Index): Promise<void> {
    const projectID = Instance.project.id
    index.time.updated = Date.now()
    await Storage.write(["memory", "knowledge", "code-index", projectID], index)
  }

  export async function load(): Promise<Index> {
    let existing = await get()
    if (!existing) {
      existing = await build()
    }
    return existing
  }

  export async function findFunctions(name: string): Promise<FunctionInfo[]> {
    const index = await load()
    const lowerName = name.toLowerCase()
    return index.functions.filter(
      (f) => f.name.toLowerCase().includes(lowerName) || f.file.toLowerCase().includes(lowerName),
    )
  }

  export async function findClasses(name: string): Promise<ClassInfo[]> {
    const index = await load()
    const lowerName = name.toLowerCase()
    return index.classes.filter(
      (c) => c.name.toLowerCase().includes(lowerName) || c.file.toLowerCase().includes(lowerName),
    )
  }

  export async function findTypes(name: string): Promise<TypeInfo[]> {
    const index = await load()
    const lowerName = name.toLowerCase()
    return index.types.filter(
      (t) => t.name.toLowerCase().includes(lowerName) || t.file.toLowerCase().includes(lowerName),
    )
  }

  export async function findImports(source: string): Promise<ImportInfo[]> {
    const index = await load()
    return index.imports.filter((i) => i.source === source)
  }

  export async function findExportsFromFile(filePath: string): Promise<ExportInfo[]> {
    const index = await load()
    return index.exports.filter((e) => e.file === filePath)
  }

  export async function getSymbolsInFile(filePath: string): Promise<{
    functions: FunctionInfo[]
    classes: ClassInfo[]
    interfaces: InterfaceInfo[]
    types: TypeInfo[]
  }> {
    const index = await load()
    const relativePath = path.relative(Instance.worktree, filePath).replace(/\\/g, "/")

    return {
      functions: index.functions.filter((f) => f.file === relativePath),
      classes: index.classes.filter((c) => c.file === relativePath),
      interfaces: index.interfaces.filter((i) => i.file === relativePath),
      types: index.types.filter((t) => t.file === relativePath),
    }
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "knowledge", "code-index", projectID])
  }
}
