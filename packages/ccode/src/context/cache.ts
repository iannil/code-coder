import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { Fingerprint } from "./fingerprint"
import path from "path"
import z from "zod"

const log = Log.create({ service: "context.cache" })

export namespace Cache {
  export const CacheEntry = z.object({
    path: z.string(),
    type: z.enum(["file", "directory", "config", "route", "component", "test"]),
    lastModified: z.number(),
    size: z.number(),
    hash: z.string().optional(),
  })
  export type CacheEntry = z.infer<typeof CacheEntry>

  export const RouteCache = z.object({
    path: z.string(),
    type: z.enum(["file", "directory", "app", "pages", "api"]),
    framework: z.string().optional(),
    methods: z.array(z.string()).optional(),
    middleware: z.string().optional(),
  })
  export type RouteCache = z.infer<typeof RouteCache>

  export const ComponentCache = z.object({
    path: z.string(),
    name: z.string(),
    type: z.enum(["component", "hook", "util", "layout", "page"]),
    props: z.array(z.string()).optional(),
    imports: z.array(z.string()).optional(),
  })
  export type ComponentCache = z.infer<typeof ComponentCache>

  export const ConfigCache = z.object({
    path: z.string(),
    name: z.string(),
    type: z.string(),
    content: z.string().optional(),
    parsed: z.record(z.string(), z.any()).optional(),
  })
  export type ConfigCache = z.infer<typeof ConfigCache>

  export const Info = z.object({
    projectID: z.string(),
    routes: z.array(RouteCache),
    components: z.array(ComponentCache),
    configs: z.array(ConfigCache),
    testFiles: z.array(z.string()),
    entries: z.record(z.string(), CacheEntry),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type Info = z.infer<typeof Info>

  const ROUTE_PATTERNS = {
    "Next.js App Router": {
      patterns: ["app/**/page.tsx", "app/**/page.ts", "app/**/route.ts"],
      type: "app" as const,
    },
    "Next.js Pages Router": {
      patterns: ["pages/**/*.tsx", "pages/**/*.ts"],
      type: "pages" as const,
    },
    Remix: {
      patterns: ["app/routes/**/*.tsx"],
      type: "file" as const,
    },
    SvelteKit: {
      patterns: ["src/routes/**/*.svelte"],
      type: "file" as const,
    },
    Nuxt: {
      patterns: ["pages/**/*.vue"],
      type: "file" as const,
    },
    Astro: {
      patterns: ["src/pages/**/*.astro"],
      type: "file" as const,
    },
    Express: {
      patterns: ["**/*.routes.ts", "**/routes/**/*.ts"],
      type: "api" as const,
    },
    NestJS: {
      patterns: ["**/*.controller.ts", "**/*.resolver.ts"],
      type: "api" as const,
    },
    Hono: {
      patterns: ["**/routes/**/*.ts", "**/*routes.ts"],
      type: "api" as const,
    },
  }

  async function hashFile(filePath: string): Promise<string> {
    try {
      const content = await Bun.file(filePath).text()
      let hash = 0
      for (let i = 0; i < content.length; i++) {
        const char = content.charCodeAt(i)
        hash = (hash << 5) - hash + char
        hash = hash & hash
      }
      return hash.toString(36)
    } catch {
      return ""
    }
  }

  async function detectRoutes(fingerprint: Fingerprint.Info): Promise<RouteCache[]> {
    const routes: RouteCache[] = []
    const worktree = Instance.worktree

    const frameworkName = fingerprint.frameworks.find((f) =>
      ["Next.js", "Remix", "SvelteKit", "Nuxt", "Astro"].includes(f.name),
    )?.name

    const routeConfig = frameworkName
      ? ROUTE_PATTERNS[frameworkName as keyof typeof ROUTE_PATTERNS]
      : Object.values(ROUTE_PATTERNS).find((config) => fingerprint.frameworks.some((f) => f.name.includes(config.type)))

    if (!routeConfig) return routes

    for (const pattern of routeConfig.patterns) {
      try {
        const matches = await Array.fromAsync(
          new Bun.Glob(pattern).scan({
            cwd: worktree,
            absolute: true,
          }),
        )

        for (const fullPath of matches) {
          const relativePath = path.relative(worktree, fullPath).replace(/\\/g, "/")

          let methods: string[] | undefined
          if (pattern.includes("route.ts") || pattern.includes("controller") || pattern.includes("api")) {
            try {
              const content = await Bun.file(fullPath).text()
              const foundMethods = new Set<string>()
              for (const method of ["GET", "POST", "PUT", "DELETE", "PATCH"]) {
                if (content.includes(method.toLowerCase())) {
                  foundMethods.add(method)
                }
              }
              if (foundMethods.size > 0) methods = Array.from(foundMethods)
            } catch {}
          }

          routes.push({
            path: relativePath,
            type: routeConfig.type,
            framework: frameworkName,
            methods,
          })
        }
      } catch {}
    }

    return routes
  }

  async function detectComponents(fingerprint: Fingerprint.Info): Promise<ComponentCache[]> {
    const components: ComponentCache[] = []
    const worktree = Instance.worktree

    const componentPatterns = [
      "src/components/**/*.{ts,tsx,js,jsx,vue,svelte}",
      "components/**/*.{ts,tsx,js,jsx,vue,svelte}",
      "app/components/**/*.{ts,tsx,js,jsx}",
    ]

    for (const pattern of componentPatterns) {
      try {
        const matches = await Array.fromAsync(
          new Bun.Glob(pattern).scan({
            cwd: worktree,
            absolute: true,
          }),
        )

        for (const fullPath of matches) {
          const relativePath = path.relative(worktree, fullPath).replace(/\\/g, "/")
          const name = path.basename(relativePath).replace(/\.(tsx?|jsx?|vue|svelte)$/, "")

          let type: ComponentCache["type"] = "component"
          if (name.endsWith(".hook") || name.startsWith("use")) type = "hook"
          else if (name.endsWith(".util") || name.endsWith(".helper")) type = "util"
          else if (name.includes("layout") || name.includes("Layout")) type = "layout"
          else if (name.endsWith("page") || name.includes("Page")) type = "page"

          const imports: string[] = []
          try {
            const content = await Bun.file(fullPath).text()
            const importMatches = content.match(/import\s+.*?\s+from\s+['"]([^'"]+)['"]/g) || []
            for (const match of importMatches) {
              const imp = match.match(/from\s+['"]([^'"]+)['"]/)?.[1]
              if (imp && imp.startsWith(".")) imports.push(imp)
            }
          } catch {}

          components.push({
            path: relativePath,
            name,
            type,
            imports: imports.length > 0 ? imports : undefined,
          })
        }
      } catch {}
    }

    return components
  }

  async function detectConfigFiles(fingerprint: Fingerprint.Info): Promise<ConfigCache[]> {
    const configs: ConfigCache[] = []
    const worktree = Instance.worktree

    const configPatterns = ["*.{json,js,ts,mjs,cjs}", ".*rc*", ".*.{yml,yaml}", "tsconfig.json", "package.json"]

    const seen = new Set<string>()

    for (const pattern of configPatterns) {
      try {
        const matches = await Array.fromAsync(
          new Bun.Glob(pattern).scan({
            cwd: worktree,
            absolute: true,
          }),
        )

        for (const fullPath of matches) {
          const relativePath = path.relative(worktree, fullPath).replace(/\\/g, "/")
          if (seen.has(relativePath)) continue
          seen.add(relativePath)

          const name = path.basename(relativePath)
          let type = "unknown"

          if (name.includes("config")) type = "config"
          else if (name.startsWith(".")) type = "rc"
          else if (name === "package.json") type = "package"
          else if (name === "tsconfig.json") type = "typescript"

          let content: string | undefined
          let parsed: Record<string, any> | undefined
          try {
            content = await Bun.file(fullPath).text()
            if (name.endsWith(".json")) {
              parsed = JSON.parse(content)
            }
          } catch {}

          configs.push({
            path: relativePath,
            name,
            type,
            content: content?.slice(0, 1000),
            parsed,
          })
        }
      } catch {}
    }

    return configs
  }

  async function detectTestFiles(fingerprint: Fingerprint.Info): Promise<string[]> {
    const tests: string[] = []
    const worktree = Instance.worktree

    const testPatterns = [
      "**/*.test.{ts,tsx,js,jsx}",
      "**/*.spec.{ts,tsx,js,jsx}",
      "**/__tests__/**/*.{ts,tsx,js,jsx}",
      "**/{test,tests}/**/*.{ts,tsx,js,jsx}",
    ]

    for (const pattern of testPatterns) {
      try {
        const matches = await Array.fromAsync(
          new Bun.Glob(pattern).scan({
            cwd: worktree,
            absolute: false,
          }),
        )
        tests.push(...matches)
      } catch {}
    }

    return [...new Set(tests)]
  }

  export async function build(): Promise<Info> {
    const projectID = Instance.project.id
    const now = Date.now()

    log.info("building cache", { projectID })

    const fingerprint = await Fingerprint.load()

    const [routes, components, configs, testFiles] = await Promise.all([
      detectRoutes(fingerprint),
      detectComponents(fingerprint),
      detectConfigFiles(fingerprint),
      detectTestFiles(fingerprint),
    ])

    const entries: Record<string, CacheEntry> = {}

    for (const route of routes) {
      entries[route.path] = {
        path: route.path,
        type: "route",
        lastModified: Date.now(),
        size: 0,
      }
    }

    for (const component of components) {
      entries[component.path] = {
        path: component.path,
        type: "component",
        lastModified: Date.now(),
        size: 0,
      }
    }

    for (const config of configs) {
      entries[config.path] = {
        path: config.path,
        type: "config",
        lastModified: Date.now(),
        size: config.content?.length || 0,
      }
    }

    for (const test of testFiles) {
      entries[test] = {
        path: test,
        type: "test",
        lastModified: Date.now(),
        size: 0,
      }
    }

    const result: Info = {
      projectID,
      routes,
      components,
      configs,
      testFiles,
      entries,
      time: {
        created: now,
        updated: now,
      },
    }

    log.info("cache built", {
      projectID,
      routesCount: routes.length,
      componentsCount: components.length,
      configsCount: configs.length,
      testFilesCount: testFiles.length,
    })

    return result
  }

  export async function get(): Promise<Info | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<Info>(["context", "cache", projectID])
    } catch {
      return undefined
    }
  }

  export async function load(options?: { force?: boolean }): Promise<Info> {
    let existing = await get()

    if (!existing || options?.force) {
      existing = await build()
      await save(existing)
    }

    return existing
  }

  export async function save(cache: Info): Promise<void> {
    const projectID = Instance.project.id
    cache.time.updated = Date.now()
    await Storage.write(["context", "cache", projectID], cache)
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["context", "cache", projectID])
  }

  export async function updateEntry(entry: CacheEntry): Promise<void> {
    const cache = await load()
    cache.entries[entry.path] = entry
    cache.time.updated = Date.now()
    await save(cache)
  }

  export async function removeEntry(path: string): Promise<void> {
    const cache = await get()
    if (!cache) return

    delete cache.entries[path]
    cache.routes = cache.routes.filter((r) => r.path !== path)
    cache.components = cache.components.filter((c) => c.path !== path)
    cache.configs = cache.configs.filter((c) => c.path !== path)
    cache.testFiles = cache.testFiles.filter((t) => t !== path)

    await save(cache)
  }

  export async function getRoute(path: string): Promise<RouteCache | undefined> {
    const cache = await get()
    return cache?.routes.find((r) => r.path === path)
  }

  export async function getRoutesByPattern(pattern: string): Promise<RouteCache[]> {
    const cache = await get()
    if (!cache) return []

    const regex = new RegExp(pattern.replace(/\*/g, ".*"))
    return cache.routes.filter((r) => regex.test(r.path))
  }

  export async function getComponent(name: string): Promise<ComponentCache | undefined> {
    const cache = await get()
    return cache?.components.find((c) => c.name === name)
  }

  export async function getComponentsByType(type: ComponentCache["type"]): Promise<ComponentCache[]> {
    const cache = await get()
    if (!cache) return []
    return cache.components.filter((c) => c.type === type)
  }

  export async function getConfig(name: string): Promise<ConfigCache | undefined> {
    const cache = await get()
    return cache?.configs.find((c) => c.name === name)
  }

  export async function getTestForFile(filePath: string): Promise<string | undefined> {
    const cache = await get()
    if (!cache) return undefined

    const baseName = path.basename(filePath).replace(/\.(ts|tsx|js|jsx)$/, "")

    for (const testFile of cache.testFiles) {
      const testBaseName = path.basename(testFile).replace(/\.(test|spec)\.(ts|tsx|js|jsx)$/, "$1")
      if (testBaseName === baseName) {
        return testFile
      }
    }

    return undefined
  }
}
