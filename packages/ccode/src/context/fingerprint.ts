import { Log } from "@/util/log"
import { Filesystem } from "@/util/filesystem"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import z from "zod"
import path from "path"

const log = Log.create({ service: "context.fingerprint" })

export namespace Fingerprint {
  export const FrameworkInfo = z.object({
    name: z.string(),
    version: z.string().optional(),
    type: z.enum(["frontend", "backend", "fullstack", "mobile", "desktop", "cli", "library"]),
  })
  export type FrameworkInfo = z.infer<typeof FrameworkInfo>

  export const BuildToolInfo = z.object({
    name: z.string(),
    config: z.string().optional(),
    version: z.string().optional(),
  })
  export type BuildToolInfo = z.infer<typeof BuildToolInfo>

  export const TestFrameworkInfo = z.object({
    name: z.string(),
    config: z.string().optional(),
    runner: z.string().optional(),
  })
  export type TestFrameworkInfo = z.infer<typeof TestFrameworkInfo>

  export const PackageInfo = z.object({
    name: z.string().optional(),
    version: z.string().optional(),
    manager: z.enum(["npm", "bun", "yarn", "pnpm", "unknown"]),
  })
  export type PackageInfo = z.infer<typeof PackageInfo>

  export const ConfigFile = z.object({
    path: z.string(),
    name: z.string(),
    content: z.string().optional(),
  })
  export type ConfigFile = z.infer<typeof ConfigFile>

  export const Info = z.object({
    projectID: z.string(),
    frameworks: z.array(FrameworkInfo),
    buildTools: z.array(BuildToolInfo),
    testFrameworks: z.array(TestFrameworkInfo),
    package: PackageInfo,
    configs: z.array(ConfigFile),
    language: z.enum(["typescript", "javascript", "python", "go", "rust", "java", "csharp", "other"]),
    hasTypeScript: z.boolean(),
    languageVersion: z.string().optional(),
    directories: z.object({
      src: z.string().optional(),
      components: z.string().optional(),
      pages: z.string().optional(),
      routes: z.string().optional(),
      tests: z.array(z.string()),
      lib: z.string().optional(),
      dist: z.string().optional(),
      build: z.string().optional(),
      public: z.string().optional(),
    }),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type Info = z.infer<typeof Info>

  const FRAMEWORK_PATTERNS = {
    React: {
      dependencies: ["react", "react-dom"],
      type: "frontend" as const,
      configFiles: ["vite.config.ts", "vite.config.js", "next.config.js", "next.config.mjs"],
    },
    Vue: {
      dependencies: ["vue"],
      type: "frontend" as const,
      configFiles: ["vite.config.ts", "vue.config.js", "nuxt.config.ts"],
    },
    Svelte: {
      dependencies: ["svelte"],
      type: "frontend" as const,
      configFiles: ["vite.config.ts", "svelte.config.js"],
    },
    Angular: {
      dependencies: ["@angular/core"],
      type: "frontend" as const,
      configFiles: ["angular.json"],
    },
    "Next.js": {
      dependencies: ["next"],
      type: "fullstack" as const,
      configFiles: ["next.config.js", "next.config.mjs", "next.config.ts"],
    },
    Nuxt: {
      dependencies: ["nuxt"],
      type: "fullstack" as const,
      configFiles: ["nuxt.config.ts"],
    },
    Remix: {
      dependencies: ["@remix-run/react"],
      type: "fullstack" as const,
      configFiles: ["remix.config.js"],
    },
    SvelteKit: {
      dependencies: ["@sveltejs/kit"],
      type: "fullstack" as const,
      configFiles: ["svelte.config.js"],
    },
    Astro: {
      dependencies: ["astro"],
      type: "frontend" as const,
      configFiles: ["astro.config.js", "astro.config.mjs", "astro.config.ts"],
    },
    NestJS: {
      dependencies: ["@nestjs/core"],
      type: "backend" as const,
      configFiles: ["nest-cli.json"],
    },
    Express: {
      dependencies: ["express"],
      type: "backend" as const,
      configFiles: [],
    },
    Fastify: {
      dependencies: ["fastify"],
      type: "backend" as const,
      configFiles: [],
    },
    Koa: {
      dependencies: ["koa"],
      type: "backend" as const,
      configFiles: [],
    },
    Hono: {
      dependencies: ["hono"],
      type: "backend" as const,
      configFiles: [],
    },
    Django: {
      dependencies: ["django"],
      type: "backend" as const,
      configFiles: ["settings.py", "manage.py"],
      language: "python",
    },
    Flask: {
      dependencies: ["flask"],
      type: "backend" as const,
      configFiles: [],
      language: "python",
    },
    FastAPI: {
      dependencies: ["fastapi"],
      type: "backend" as const,
      configFiles: [],
      language: "python",
    },
    Laravel: {
      dependencies: ["laravel/framework"],
      type: "backend" as const,
      configFiles: ["artisan"],
      language: "php",
    },
    Rails: {
      dependencies: ["rails"],
      type: "backend" as const,
      configFiles: ["config/application.rb"],
      language: "ruby",
    },
    Electron: {
      dependencies: ["electron"],
      type: "desktop" as const,
      configFiles: ["electron-builder.yml"],
    },
    Tauri: {
      dependencies: ["@tauri-apps/api"],
      type: "desktop" as const,
      configFiles: ["tauri.conf.json", "tauri.config.json"],
    },
    "React Native": {
      dependencies: ["react-native"],
      type: "mobile" as const,
      configFiles: ["app.json", "react-native.config.js"],
    },
    Expo: {
      dependencies: ["expo"],
      type: "mobile" as const,
      configFiles: ["app.config.js", "app.json"],
    },
  }

  const BUILD_TOOL_PATTERNS = {
    Vite: {
      configFiles: ["vite.config.ts", "vite.config.js", "vite.config.mjs"],
      packageKey: "vite",
    },
    Webpack: {
      configFiles: ["webpack.config.js", "webpack.config.ts"],
      packageKey: "webpack",
    },
    Rollup: {
      configFiles: ["rollup.config.js", "rollup.config.ts"],
      packageKey: "rollup",
    },
    esbuild: {
      configFiles: ["esbuild.config.js", "esbuild.js", "esbuild.ts", "esbuild.mjs"],
      packageKey: "esbuild",
    },
    Turbopack: {
      configFiles: [],
      packageKey: "turbo",
    },
    Turborepo: {
      configFiles: ["turbo.json"],
      packageKey: "turbo",
    },
    Nx: {
      configFiles: ["nx.json"],
      packageKey: "nx",
    },
    Rush: {
      configFiles: ["rush.json"],
      packageKey: "@microsoft/rush",
    },
    Babel: {
      configFiles: [".babelrc", ".babelrc.js", "babel.config.js", "babel.config.json"],
      packageKey: "@babel/core",
    },
    SWC: {
      configFiles: [".swcrc"],
      packageKey: "@swc/core",
    },
    PostCSS: {
      configFiles: ["postcss.config.js", "postcss.config.cjs", "postcss.config.mjs"],
      packageKey: "postcss",
    },
    TailwindCSS: {
      configFiles: ["tailwind.config.js", "tailwind.config.ts", "tailwind.config.cjs"],
      packageKey: "tailwindcss",
    },
  }

  const TEST_FRAMEWORK_PATTERNS = {
    Jest: {
      configFiles: ["jest.config.js", "jest.config.ts", "jest.config.json", ".jestrc"],
      packageKey: "jest",
      scripts: ["test", "test:unit"],
    },
    Vitest: {
      configFiles: ["vitest.config.ts", "vitest.config.js"],
      packageKey: "vitest",
      scripts: ["test"],
    },
    Mocha: {
      configFiles: [".mocharc.js", ".mocharc.json", "mocha.opts"],
      packageKey: "mocha",
      scripts: ["test"],
    },
    Jasmine: {
      configFiles: ["jasmine.json"],
      packageKey: "jasmine",
      scripts: ["test"],
    },
    Karma: {
      configFiles: ["karma.conf.js"],
      packageKey: "karma",
      scripts: ["test"],
    },
    Cypress: {
      configFiles: ["cypress.config.ts", "cypress.config.js"],
      packageKey: "cypress",
      scripts: ["cy:open", "cypress:open"],
    },
    Playwright: {
      configFiles: ["playwright.config.ts", "playwright.config.js"],
      packageKey: "@playwright/test",
      scripts: ["test:e2e", "playwright:test"],
    },
    Puppeteer: {
      configFiles: [],
      packageKey: "puppeteer",
      scripts: [],
    },
    "Testing Library": {
      configFiles: [],
      packageKey: "@testing-library",
      scripts: [],
    },
    Supertest: {
      configFiles: [],
      packageKey: "supertest",
      scripts: [],
    },
    Pytest: {
      configFiles: ["pytest.ini", "pyproject.toml"],
      packageKey: "pytest",
      language: "python",
    },
    Unittest: {
      configFiles: [],
      packageKey: "unittest",
      language: "python",
    },
  }

  const LANGUAGE_PATTERNS = {
    typescript: ["tsconfig.json", "*.ts", "*.tsx"],
    javascript: ["*.js", "*.jsx", "*.mjs"],
    python: ["*.py", "requirements.txt", "pyproject.toml", "Pipfile"],
    go: ["*.go", "go.mod"],
    rust: ["*.rs", "Cargo.toml"],
    java: ["*.java", "pom.xml", "build.gradle", "gradle"],
    csharp: ["*.cs", "*.csproj", "*.sln"],
  }

  const TEST_DIRECTORY_PATTERNS = ["test", "tests", "__tests__", "__test__", "spec", "specs", "e2e", "integration"]

  async function detectPackageManager(worktree: string): Promise<PackageInfo["manager"]> {
    const lockFiles = {
      "pnpm-lock.yaml": "pnpm" as const,
      "yarn.lock": "yarn" as const,
      "package-lock.json": "npm" as const,
      "bun.lock": "bun" as const,
      "bun.lockb": "bun" as const,
    }

    for (const [file, manager] of Object.entries(lockFiles)) {
      if (await Filesystem.exists(path.join(worktree, file))) return manager
    }

    return "unknown"
  }

  async function readPackageJson(worktree: string): Promise<Record<string, any> | undefined> {
    for (const name of ["package.json", "package.jsonc"]) {
      try {
        const content = await Bun.file(path.join(worktree, name)).text()
        return JSON.parse(content)
      } catch {}
    }
    return undefined
  }

  async function detectLanguage(
    worktree: string,
  ): Promise<{ language: Info["language"]; hasTypeScript: boolean; version?: string }> {
    const tsconfigPath = path.join(worktree, "tsconfig.json")
    const hasTypeScript = await Filesystem.exists(tsconfigPath)

    if (hasTypeScript) {
      let version = "unknown"
      try {
        const pkg = await readPackageJson(worktree)
        if (pkg?.devDependencies?.typescript) version = pkg.devDependencies.typescript
        else if (pkg?.dependencies?.typescript) version = pkg.dependencies.typescript
      } catch {}
      return { language: "typescript", hasTypeScript: true, version }
    }

    for (const [lang, patterns] of Object.entries(LANGUAGE_PATTERNS)) {
      for (const pattern of patterns) {
        if (pattern.startsWith("*")) {
          const matches = await Array.fromAsync(
            new Bun.Glob(pattern).scan({
              cwd: worktree,
              absolute: false,
            }),
          )
          if (matches.length > 5) return { language: lang as any, hasTypeScript: false }
        } else {
          const filePath = path.join(worktree, pattern)
          if (await Filesystem.exists(filePath)) return { language: lang as any, hasTypeScript: false }
        }
      }
    }

    return { language: "javascript", hasTypeScript: false }
  }

  async function detectDirectories(worktree: string): Promise<Info["directories"]> {
    const directories: Info["directories"] = {
      tests: [],
    }

    const commonDirs = ["src", "components", "pages", "routes", "lib", "dist", "build", "public", "app", "styles"]

    for (const dir of commonDirs) {
      const dirPath = path.join(worktree, dir)
      if (await Filesystem.isDir(dirPath)) {
        ;(directories as Record<string, string | string[]>)[dir] = dir
      }
    }

    for (const testDir of TEST_DIRECTORY_PATTERNS) {
      const dirPath = path.join(worktree, testDir)
      if (await Filesystem.isDir(dirPath)) {
        directories.tests.push(testDir)
      }
    }

    return directories
  }

  async function detectConfigFiles(worktree: string): Promise<ConfigFile[]> {
    const configs: ConfigFile[] = []
    const configPatterns = ["*.config.js", "*.config.ts", "*.config.mjs", "*.config.cjs", ".*rc*", ".*.json"]

    for (const pattern of configPatterns) {
      try {
        const matches = await Array.fromAsync(
          new Bun.Glob(pattern).scan({
            cwd: worktree,
            absolute: false,
          }),
        )

        for (const match of matches) {
          const fullPath = path.join(worktree, match)
          const stat = await Bun.file(fullPath).exists()
          if (stat) {
            configs.push({
              path: fullPath,
              name: match,
            })
          }
        }
      } catch {}
    }

    return configs
  }

  async function detectFrameworks(
    worktree: string,
    dependencies: Record<string, string> = {},
  ): Promise<FrameworkInfo[]> {
    const frameworks: FrameworkInfo[] = []
    const allDeps = new Set([...Object.keys(dependencies), ...Object.keys(dependencies)])

    for (const [name, pattern] of Object.entries(FRAMEWORK_PATTERNS)) {
      const language = (pattern as any).language
      if (language && language !== "javascript") {
        continue
      }

      for (const dep of pattern.dependencies) {
        if (allDeps.has(dep)) {
          frameworks.push({
            name,
            version: dependencies[dep],
            type: pattern.type,
          })
          break
        }
      }
    }

    return frameworks
  }

  async function detectBuildTools(
    worktree: string,
    dependencies: Record<string, string> = {},
  ): Promise<BuildToolInfo[]> {
    const tools: BuildToolInfo[] = []
    const allDeps = new Set([...Object.keys(dependencies), ...Object.keys(dependencies)])

    for (const [name, pattern] of Object.entries(BUILD_TOOL_PATTERNS)) {
      let found = false
      let configPath: string | undefined

      for (const configFile of pattern.configFiles) {
        const fullPath = path.join(worktree, configFile)
        if (await Filesystem.exists(fullPath)) {
          configPath = fullPath
          found = true
          break
        }
      }

      if (!found && allDeps.has(pattern.packageKey)) {
        found = true
      }

      if (found) {
        tools.push({
          name,
          config: configPath,
          version: dependencies[pattern.packageKey],
        })
      }
    }

    return tools
  }

  async function detectTestFrameworks(
    worktree: string,
    dependencies: Record<string, string> = {},
    scripts: Record<string, string> = {},
  ): Promise<TestFrameworkInfo[]> {
    const frameworks: TestFrameworkInfo[] = []
    const allDeps = new Set([...Object.keys(dependencies), ...Object.keys(dependencies)])

    for (const [name, pattern] of Object.entries(TEST_FRAMEWORK_PATTERNS)) {
      const language = (pattern as any).language
      if (language && language !== "javascript") {
        continue
      }

      let found = allDeps.has(pattern.packageKey)
      let configPath: string | undefined
      let runner: string | undefined

      for (const configFile of pattern.configFiles) {
        const fullPath = path.join(worktree, configFile)
        if (await Filesystem.exists(fullPath)) {
          configPath = fullPath
          found = true
          break
        }
      }

      const patternScripts = (pattern as any).scripts ?? []
      for (const script of patternScripts) {
        if (scripts[script]) {
          runner = scripts[script]
          found = true
          break
        }
      }

      if (found) {
        frameworks.push({
          name,
          config: configPath,
          runner,
        })
      }
    }

    return frameworks
  }

  export async function generate(worktree: string): Promise<Info> {
    const projectID = Instance.project.id
    const now = Date.now()

    const pkg = await readPackageJson(worktree)
    const dependencies = {
      ...(pkg?.dependencies ?? {}),
      ...(pkg?.devDependencies ?? {}),
      ...(pkg?.peerDependencies ?? {}),
    }

    const packageManager = await detectPackageManager(worktree)
    const { language, hasTypeScript, version: languageVersion } = await detectLanguage(worktree)
    const directories = await detectDirectories(worktree)
    const configs = await detectConfigFiles(worktree)

    const frameworks = await detectFrameworks(worktree, dependencies)
    const buildTools = await detectBuildTools(worktree, dependencies)
    const testFrameworks = await detectTestFrameworks(worktree, dependencies, pkg?.scripts ?? {})

    const result: Info = {
      projectID,
      frameworks,
      buildTools,
      testFrameworks,
      package: {
        name: pkg?.name,
        version: pkg?.version,
        manager: packageManager,
      },
      configs,
      language,
      hasTypeScript,
      languageVersion,
      directories,
      time: {
        created: now,
        updated: now,
      },
    }

    log.info("generated fingerprint", {
      projectID,
      language,
      frameworks: frameworks.map((f) => f.name),
      buildTools: buildTools.map((t) => t.name),
      testFrameworks: testFrameworks.map((t) => t.name),
    })

    return result
  }

  export async function get(): Promise<Info | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<Info>(["context", "fingerprint", projectID])
    } catch {
      return undefined
    }
  }

  export async function save(fingerprint: Info): Promise<void> {
    const projectID = Instance.project.id
    await Storage.write(["context", "fingerprint", projectID], fingerprint)
  }

  export async function load(): Promise<Info> {
    let existing = await get()
    if (!existing) {
      existing = await generate(Instance.worktree)
      await save(existing)
    }
    return existing
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["context", "fingerprint", projectID])
  }

  export function describe(fingerprint: Info): string {
    const parts: string[] = []

    if (fingerprint.language !== "other") {
      parts.push(fingerprint.language === "typescript" ? "TypeScript" : fingerprint.language)
    }

    if (fingerprint.frameworks.length > 0) {
      parts.push(fingerprint.frameworks.map((f) => f.name).join(", "))
    }

    if (fingerprint.buildTools.length > 0) {
      parts.push(fingerprint.buildTools.map((t) => t.name).join(", "))
    }

    if (fingerprint.testFrameworks.length > 0) {
      parts.push(fingerprint.testFrameworks.map((t) => t.name).join(", "))
    }

    return parts.join(" • ") || "Unknown project type"
  }
}
