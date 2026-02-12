import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import path from "path"
import z from "zod"

const log = Log.create({ service: "memory.preferences" })

export namespace Preferences {
  export const CodeStyle = z.object({
    indentation: z.object({
      type: z.enum(["tabs", "spaces"]),
      spaces: z.number().int().min(2).max(8).optional(),
    }),
    naming: z.object({
      variables: z.enum(["camelCase", "snake_case", "PascalCase", "kebab-case"]),
      functions: z.enum(["camelCase", "snake_case"]),
      classes: z.enum(["PascalCase", "camelCase"]),
      constants: z.enum(["UPPER_SNAKE_CASE", "camelCase", "snake_case"]),
      components: z.enum(["PascalCase", "camelCase"]),
    }),
    quotes: z.enum(["single", "double", "backtick"]),
    semicolons: z.boolean(),
    trailingCommas: z.boolean(),
    lineWidth: z.number().int().min(80).max(200),
    bracketSpacing: z.boolean(),
    arrowParentheses: z.enum(["always", "avoid"]),
  })
  export type CodeStyle = z.infer<typeof CodeStyle>

  export const FrameworkPreferences = z.object({
    componentNaming: z.enum(["PascalCase", "kebab-case", "camelCase"]),
    componentStructure: z.enum(["colocated", "grouped", "flat"]),
    fileExtensions: z.array(z.string()),
    styleLocation: z.enum(["colocated", "shared", "external"]),
  })
  export type FrameworkPreferences = z.infer<typeof FrameworkPreferences>

  export const DirectoryPreferences = z.object({
    components: z.string().optional(),
    hooks: z.string().optional(),
    utils: z.string().optional(),
    types: z.string().optional(),
    tests: z.string().optional(),
    assets: z.string().optional(),
  })
  export type DirectoryPreferences = z.infer<typeof DirectoryPreferences>

  export const EditPreferences = z.object({
    autoImport: z.boolean(),
    sortImports: z.boolean(),
    removeUnused: z.boolean(),
    organizeOnSave: z.boolean(),
  })
  export type EditPreferences = z.infer<typeof EditPreferences>

  export const Info = z.object({
    projectID: z.string(),
    codeStyle: CodeStyle.partial(),
    framework: FrameworkPreferences.partial(),
    directories: DirectoryPreferences.partial(),
    edit: EditPreferences.partial(),
    learnedPatterns: z.array(
      z.object({
        pattern: z.string(),
        frequency: z.number(),
        lastSeen: z.number(),
      }),
    ),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type Info = z.infer<typeof Info>

  const DEFAULT_CODE_STYLE: CodeStyle = {
    indentation: { type: "spaces", spaces: 2 },
    naming: {
      variables: "camelCase",
      functions: "camelCase",
      classes: "PascalCase",
      constants: "UPPER_SNAKE_CASE",
      components: "PascalCase",
    },
    quotes: "single",
    semicolons: false,
    trailingCommas: true,
    lineWidth: 120,
    bracketSpacing: true,
    arrowParentheses: "avoid",
  }

  const DEFAULT_FRAMEWORK: FrameworkPreferences = {
    componentNaming: "PascalCase",
    componentStructure: "colocated",
    fileExtensions: [".tsx", ".ts"],
    styleLocation: "colocated",
  }

  const DEFAULT_EDIT: EditPreferences = {
    autoImport: true,
    sortImports: true,
    removeUnused: true,
    organizeOnSave: false,
  }

  export async function get(): Promise<Info> {
    const projectID = Instance.project.id
    try {
      const stored = await Storage.read<Info>(["memory", "preferences", projectID])
      return stored
    } catch {
      return create()
    }
  }

  export async function create(): Promise<Info> {
    const projectID = Instance.project.id
    const now = Date.now()

    const detected = await detectFromProject()

    const result: Info = {
      projectID,
      codeStyle: detected.codeStyle,
      framework: detected.framework,
      directories: detected.directories,
      edit: { ...DEFAULT_EDIT },
      learnedPatterns: [],
      time: {
        created: now,
        updated: now,
      },
    }

    await save(result)
    return result
  }

  async function detectFromProject(): Promise<{
    codeStyle: Partial<CodeStyle>
    framework: Partial<FrameworkPreferences>
    directories: Partial<DirectoryPreferences>
  }> {
    const worktree = Instance.worktree
    const codeStyle: Partial<CodeStyle> = {}
    const framework: Partial<FrameworkPreferences> = {}
    const directories: Partial<DirectoryPreferences> = {}

    try {
      const prettierConfig = await findAndParseConfig(worktree, ["prettier.config.*", ".prettierrc*"])
      if (prettierConfig) {
        if (prettierConfig.tabWidth) {
          codeStyle.indentation = { type: prettierConfig.useTabs ? "tabs" : "spaces", spaces: prettierConfig.tabWidth }
        }
        if (prettierConfig.singleQuote !== undefined)
          codeStyle.quotes = prettierConfig.singleQuote ? "single" : "double"
        if (prettierConfig.semi !== undefined) codeStyle.semicolons = prettierConfig.semi
        if (prettierConfig.trailingComma !== undefined) {
          codeStyle.trailingCommas = prettierConfig.trailingComma !== "none"
        }
        if (prettierConfig.printWidth) codeStyle.lineWidth = prettierConfig.printWidth
        if (prettierConfig.bracketSpacing !== undefined) codeStyle.bracketSpacing = prettierConfig.bracketSpacing
        if (prettierConfig.arrowParens)
          codeStyle.arrowParentheses = prettierConfig.arrowParens === "always" ? "always" : "avoid"
      }

      const eslintConfig = await findAndParseConfig(worktree, [".eslintrc.*", "eslint.config.*"])
      if (eslintConfig) {
        if (eslintConfig.rules) {
          const quotesRule = eslintConfig.rules.quotes || eslintConfig.rules["@typescript-eslint/quotes"]
          if (quotesRule && typeof quotesRule === "object" && quotesRule[0]) {
            codeStyle.quotes = quotesRule[1] === "single" ? "single" : "double"
          }
          const semiRule = eslintConfig.rules.semi || eslintConfig.rules["@typescript-eslint/semi"]
          if (semiRule && typeof semiRule === "object") {
            codeStyle.semicolons = semiRule[0] !== "never"
          }
        }
      }

      const tsconfig = await findAndParseConfig(worktree, ["tsconfig.json"])
      if (tsconfig) {
        if (tsconfig.compilerOptions) {
          codeStyle.indentation = codeStyle.indentation || { type: "spaces" }
          if (tsconfig.compilerOptions.useDefineForClassFields) {
          }
        }
      }

      const editorconfig = await findAndParseConfig(worktree, [".editorconfig"])
      if (editorconfig) {
        const defaultSection = editorconfig["*"] || {}
        if (defaultSection.indent_style) {
          codeStyle.indentation = codeStyle.indentation || { type: "spaces" }
          codeStyle.indentation.type = defaultSection.indent_style === "tab" ? "tabs" : "spaces"
          if (defaultSection.indent_size) {
            codeStyle.indentation.spaces = defaultSection.indent_size
          }
        }
        if (defaultSection.max_line_length) {
          codeStyle.lineWidth = defaultSection.max_line_length
        }
      }

      const packageJson = await findAndParseConfig(worktree, ["package.json"])
      if (packageJson) {
        if (packageJson.dependencies?.react || packageJson.dependencies?.next) {
          framework.componentNaming = "PascalCase"
          framework.fileExtensions = [".tsx", ".ts"]
        }
        if (packageJson.dependencies?.vue) {
          framework.componentNaming = "PascalCase"
          framework.fileExtensions = [".vue", ".ts"]
        }
      }
    } catch (error) {
      log.warn("error detecting project preferences", { error })
    }

    return { codeStyle, framework, directories }
  }

  async function findAndParseConfig(worktree: string, patterns: string[]): Promise<any> {
    for (const pattern of patterns) {
      try {
        const matches = await Array.fromAsync(
          new Bun.Glob(pattern).scan({
            cwd: worktree,
            absolute: true,
          }),
        )

        for (const match of matches) {
          try {
            const content = await Bun.file(match).text()
            if (match.endsWith(".json") || match.endsWith(".jsonc")) {
              return JSON.parse(content)
            }
            if (match.endsWith(".js") || match.endsWith(".mjs") || match.endsWith(".cjs")) {
              return eval(`(${content})`)
            }
            if (match.endsWith(".ts")) {
              return eval(`(${content})`)
            }
            if (match.includes(".editorconfig")) {
              const result: any = {}
              const lines = content.split("\n")
              let currentSection: any = result
              for (const line of lines) {
                if (line.startsWith("[")) {
                  const sectionName = line.match(/\[([^\]]+)\]/)?.[1]
                  if (sectionName) {
                    result[sectionName] = result[sectionName] || {}
                    currentSection = result[sectionName]
                  }
                } else if (line.includes("=")) {
                  const [key, value] = line.split("=").map((s) => s.trim())
                  if (key && value) {
                    currentSection[key] = value
                  }
                }
              }
              return result
            }
          } catch {}
        }
      } catch {}
    }
    return null
  }

  export async function save(preferences: Info): Promise<void> {
    const projectID = Instance.project.id
    preferences.time.updated = Date.now()
    await Storage.write(["memory", "preferences", projectID], preferences)
  }

  export async function update(updates: Partial<Info>): Promise<Info> {
    const preferences = await get()
    Object.assign(preferences, updates)
    preferences.time.updated = Date.now()
    await save(preferences)
    return preferences
  }

  export async function learnPattern(pattern: string): Promise<void> {
    const preferences = await get()
    const existing = preferences.learnedPatterns.find((p) => p.pattern === pattern)

    if (existing) {
      existing.frequency++
      existing.lastSeen = Date.now()
    } else {
      preferences.learnedPatterns.push({
        pattern,
        frequency: 1,
        lastSeen: Date.now(),
      })
    }

    preferences.learnedPatterns.sort((a, b) => b.frequency - a.frequency)
    preferences.learnedPatterns = preferences.learnedPatterns.slice(0, 100)

    await save(preferences)
  }

  export async function getCodeStyle(): Promise<CodeStyle> {
    const preferences = await get()
    return {
      ...DEFAULT_CODE_STYLE,
      ...(preferences.codeStyle || {}),
    }
  }

  export async function getFrameworkPreferences(): Promise<FrameworkPreferences> {
    const preferences = await get()
    return {
      ...DEFAULT_FRAMEWORK,
      ...(preferences.framework || {}),
    }
  }

  export async function getEditPreferences(): Promise<EditPreferences> {
    const preferences = await get()
    return {
      ...DEFAULT_EDIT,
      ...(preferences.edit || {}),
    }
  }

  export function describe(preferences: Info): string {
    const parts: string[] = []

    if (preferences.codeStyle) {
      const style = preferences.codeStyle
      const indent = style.indentation?.type === "tabs" ? "tabs" : `${style.indentation?.spaces || 2} spaces`
      parts.push(`${indent}, ${style.quotes || "single"} quotes, ${style.semicolons ? "with" : "without"} semicolons`)
    }

    if (preferences.framework?.componentNaming) {
      parts.push(`Component naming: ${preferences.framework.componentNaming}`)
    }

    if (preferences.learnedPatterns.length > 0) {
      parts.push(
        `\nLearned patterns: ${preferences.learnedPatterns
          .slice(0, 5)
          .map((p) => p.pattern)
          .join(", ")}`,
      )
    }

    return parts.join("\n") || "Default preferences"
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "preferences", projectID])
  }
}
