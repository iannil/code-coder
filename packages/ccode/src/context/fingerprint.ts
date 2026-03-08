/**
 * Project Fingerprinting
 *
 * Thin wrapper around @codecoder-ai/core native Rust implementation.
 * Detects project type, frameworks, build tools, test frameworks,
 * and other characteristics for contextual understanding.
 *
 * @package context
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/infrastructure/storage/storage"
import z from "zod"
import type {
  NapiFingerprintInfo,
  NapiFrameworkInfo,
  NapiBuildToolInfo,
  NapiTestFrameworkInfo,
  NapiPackageInfo,
  NapiConfigFile,
  NapiDirectoryInfo,
  NapiFrameworkType,
  NapiPackageManager,
  NapiProjectLanguage,
} from "@codecoder-ai/core"

const log = Log.create({ service: "context.fingerprint" })

// ============================================================================
// Native Bindings (lazy loaded)
// ============================================================================

interface NativeBindings {
  generateFingerprint: (path: string) => NapiFingerprintInfo
  fingerprintSimilarity: (a: NapiFingerprintInfo, b: NapiFingerprintInfo) => number
  describeFingerprint: (fp: NapiFingerprintInfo) => string
}

let nativeBindings: NativeBindings | null = null
let nativeBindingsLoaded = false

async function loadNativeBindings(): Promise<NativeBindings> {
  if (nativeBindingsLoaded && nativeBindings) return nativeBindings

  try {
    const bindings = await import("@codecoder-ai/core")
    if (typeof bindings.generateFingerprint === "function") {
      nativeBindings = bindings as NativeBindings
      log.debug("Using native fingerprint implementation")
      nativeBindingsLoaded = true
      return nativeBindings
    }
  } catch {
    // Native bindings not available
  }

  nativeBindingsLoaded = true
  throw new Error("Native bindings required: @codecoder-ai/core fingerprint functions not available")
}

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

  /**
   * Convert native fingerprint result to Info type
   */
  function convertNativeToInfo(native: NapiFingerprintInfo, projectID: string, now: number): Info {
    const languageMap: Record<string, Info["language"]> = {
      TypeScript: "typescript",
      JavaScript: "javascript",
      Python: "python",
      Go: "go",
      Rust: "rust",
      Java: "java",
      CSharp: "csharp",
      Other: "other",
    }

    const managerMap: Record<string, PackageInfo["manager"]> = {
      Npm: "npm",
      Bun: "bun",
      Yarn: "yarn",
      Pnpm: "pnpm",
      Unknown: "unknown",
    }

    const typeMap: Record<string, FrameworkInfo["type"]> = {
      Frontend: "frontend",
      Backend: "backend",
      Fullstack: "fullstack",
      Mobile: "mobile",
      Desktop: "desktop",
      Cli: "cli",
      Library: "library",
    }

    return {
      projectID,
      frameworks: native.frameworks.map((f) => ({
        name: f.name,
        version: f.version,
        type: typeMap[f.frameworkType] ?? "library",
      })),
      buildTools: native.buildTools.map((t) => ({
        name: t.name,
        config: t.config,
        version: t.version,
      })),
      testFrameworks: native.testFrameworks.map((t) => ({
        name: t.name,
        config: t.config,
        runner: t.runner,
      })),
      package: {
        name: native.package.name,
        version: native.package.version,
        manager: managerMap[native.package.manager] ?? "unknown",
      },
      configs: native.configs.map((c) => ({
        path: c.path,
        name: c.name,
      })),
      language: languageMap[native.language] ?? "other",
      hasTypeScript: native.hasTypescript,
      languageVersion: native.languageVersion,
      directories: {
        src: native.directories.src,
        components: native.directories.components,
        pages: native.directories.pages,
        routes: native.directories.routes,
        tests: native.directories.tests,
        lib: native.directories.lib,
        dist: native.directories.dist,
        build: native.directories.build,
        public: native.directories.public,
      },
      time: {
        created: now,
        updated: now,
      },
    }
  }

  /**
   * Convert Info type to native format for similarity comparison
   */
  function convertInfoToNative(info: Info): NapiFingerprintInfo {
    const languageMap: Record<Info["language"], NapiProjectLanguage> = {
      typescript: "TypeScript" as NapiProjectLanguage,
      javascript: "JavaScript" as NapiProjectLanguage,
      python: "Python" as NapiProjectLanguage,
      go: "Go" as NapiProjectLanguage,
      rust: "Rust" as NapiProjectLanguage,
      java: "Java" as NapiProjectLanguage,
      csharp: "CSharp" as NapiProjectLanguage,
      other: "Other" as NapiProjectLanguage,
    }

    const managerMap: Record<PackageInfo["manager"], NapiPackageManager> = {
      npm: "Npm" as NapiPackageManager,
      bun: "Bun" as NapiPackageManager,
      yarn: "Yarn" as NapiPackageManager,
      pnpm: "Pnpm" as NapiPackageManager,
      unknown: "Unknown" as NapiPackageManager,
    }

    const typeMap: Record<FrameworkInfo["type"], NapiFrameworkType> = {
      frontend: "Frontend" as NapiFrameworkType,
      backend: "Backend" as NapiFrameworkType,
      fullstack: "Fullstack" as NapiFrameworkType,
      mobile: "Mobile" as NapiFrameworkType,
      desktop: "Desktop" as NapiFrameworkType,
      cli: "Cli" as NapiFrameworkType,
      library: "Library" as NapiFrameworkType,
    }

    return {
      projectId: info.projectID,
      frameworks: info.frameworks.map((f) => ({
        name: f.name,
        version: f.version,
        frameworkType: typeMap[f.type],
      })),
      buildTools: info.buildTools.map((t) => ({
        name: t.name,
        config: t.config,
        version: t.version,
      })),
      testFrameworks: info.testFrameworks.map((t) => ({
        name: t.name,
        config: t.config,
        runner: t.runner,
      })),
      package: {
        name: info.package.name,
        version: info.package.version,
        manager: managerMap[info.package.manager],
      },
      configs: info.configs.map((c) => ({
        path: c.path,
        name: c.name,
      })),
      language: languageMap[info.language],
      hasTypescript: info.hasTypeScript,
      languageVersion: info.languageVersion,
      directories: {
        src: info.directories.src,
        components: info.directories.components,
        pages: info.directories.pages,
        routes: info.directories.routes,
        tests: info.directories.tests,
        lib: info.directories.lib,
        dist: info.directories.dist,
        build: info.directories.build,
        public: info.directories.public,
      },
      hash: info.projectID,
    }
  }

  /**
   * Generate fingerprint for a project directory.
   * Uses native Rust implementation for fast, accurate detection.
   * @throws Error if native bindings unavailable
   */
  export async function generate(worktree: string): Promise<Info> {
    const projectID = Instance.project.id
    const now = Date.now()

    const native = await loadNativeBindings()
    const nativeResult = native.generateFingerprint(worktree)
    const result = convertNativeToInfo(nativeResult, projectID, now)

    log.info("generated fingerprint (native)", {
      projectID,
      language: result.language,
      frameworks: result.frameworks.map((f) => f.name),
      buildTools: result.buildTools.map((t) => t.name),
      testFrameworks: result.testFrameworks.map((t) => t.name),
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

  /**
   * Check if native fingerprint implementation is being used
   */
  export function isUsingNative(): boolean {
    return nativeBindings !== null
  }

  /**
   * Compute similarity between two fingerprints.
   * Uses native Rust implementation for accurate comparison.
   * @throws Error if native bindings unavailable
   */
  export async function similarity(a: Info, b: Info): Promise<number> {
    const native = await loadNativeBindings()
    const nativeA = convertInfoToNative(a)
    const nativeB = convertInfoToNative(b)
    return native.fingerprintSimilarity(nativeA, nativeB)
  }
}
