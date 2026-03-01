/**
 * Benchmark Fixture Generator
 *
 * Creates test files of various sizes for tool performance benchmarks.
 * Files are generated on-demand and cached in the fixture directory.
 *
 * Usage:
 *   import { getFixture, ensureFixtures } from "./fixture"
 *   await ensureFixtures() // Generate all fixtures
 *   const path = getFixture("1KB.txt")
 */

import fs from "fs/promises"
import path from "path"

// ============================================================================
// Configuration
// ============================================================================

const FIXTURE_DIR = path.resolve(import.meta.dir)

export interface FixtureSpec {
  name: string
  sizeBytes: number
  type: "text" | "typescript" | "json"
}

const FIXTURES: FixtureSpec[] = [
  // Text files of various sizes
  { name: "1KB.txt", sizeBytes: 1024, type: "text" },
  { name: "100KB.txt", sizeBytes: 100 * 1024, type: "text" },
  { name: "1MB.txt", sizeBytes: 1024 * 1024, type: "text" },
  { name: "10MB.txt", sizeBytes: 10 * 1024 * 1024, type: "text" },

  // TypeScript files for Glob/Grep benchmarks
  { name: "small.ts", sizeBytes: 2 * 1024, type: "typescript" },
  { name: "medium.ts", sizeBytes: 50 * 1024, type: "typescript" },
  { name: "large.ts", sizeBytes: 200 * 1024, type: "typescript" },

  // JSON files
  { name: "small.json", sizeBytes: 1024, type: "json" },
  { name: "large.json", sizeBytes: 100 * 1024, type: "json" },
]

// ============================================================================
// Content Generators
// ============================================================================

function generateTextContent(sizeBytes: number): string {
  const line = "This is a benchmark test line with some content for performance testing purposes.\n"
  const linesNeeded = Math.ceil(sizeBytes / line.length)
  return Array(linesNeeded).fill(line).join("").slice(0, sizeBytes)
}

function generateTypeScriptContent(sizeBytes: number): string {
  const header = `/**
 * Generated TypeScript file for benchmark testing
 * Size target: ${sizeBytes} bytes
 */

import { EventEmitter } from "events"

export interface Config {
  enabled: boolean
  timeout: number
  retries: number
  options: Record<string, unknown>
}

`

  const classTemplate = (index: number) => `
export class Service${index} extends EventEmitter {
  private cache = new Map<string, unknown>()
  private config: Config

  constructor(config: Config) {
    super()
    this.config = config
  }

  async process(input: string): Promise<string> {
    const cacheKey = \`cache-\${input}\`
    if (this.cache.has(cacheKey)) {
      return this.cache.get(cacheKey) as string
    }

    const result = await this.transform(input)
    this.cache.set(cacheKey, result)
    this.emit("processed", { input, result })
    return result
  }

  private async transform(input: string): Promise<string> {
    // TODO: Implement transformation logic
    return input.toUpperCase()
  }

  getConfig(): Config {
    return { ...this.config }
  }

  setConfig(updates: Partial<Config>): void {
    this.config = { ...this.config, ...updates }
  }
}
`

  const lines = [header]
  let currentSize = header.length
  let classIndex = 0

  while (currentSize < sizeBytes) {
    const classContent = classTemplate(classIndex++)
    lines.push(classContent)
    currentSize += classContent.length
  }

  return lines.join("\n").slice(0, sizeBytes)
}

function generateJsonContent(sizeBytes: number): string {
  const items: Array<{ id: number; name: string; data: string }> = []
  const itemTemplate = { id: 0, name: "item-", data: "benchmark data for testing " }
  const itemSize = JSON.stringify(itemTemplate).length + 10 // Extra for larger numbers

  const itemsNeeded = Math.ceil(sizeBytes / itemSize)

  for (let i = 0; i < itemsNeeded; i++) {
    items.push({
      id: i,
      name: `item-${i}`,
      data: `benchmark data for testing performance ${i}`,
    })
  }

  const content = JSON.stringify({ items }, null, 2)
  return content.slice(0, sizeBytes)
}

function generateContent(spec: FixtureSpec): string {
  switch (spec.type) {
    case "text":
      return generateTextContent(spec.sizeBytes)
    case "typescript":
      return generateTypeScriptContent(spec.sizeBytes)
    case "json":
      return generateJsonContent(spec.sizeBytes)
    default:
      return generateTextContent(spec.sizeBytes)
  }
}

// ============================================================================
// Fixture Management
// ============================================================================

export function getFixturePath(name: string): string {
  return path.join(FIXTURE_DIR, name)
}

export async function fixtureExists(name: string): Promise<boolean> {
  try {
    await fs.access(getFixturePath(name))
    return true
  } catch {
    return false
  }
}

export async function generateFixture(spec: FixtureSpec): Promise<void> {
  const filePath = getFixturePath(spec.name)
  const content = generateContent(spec)
  await fs.writeFile(filePath, content, "utf-8")
}

export async function ensureFixture(spec: FixtureSpec): Promise<string> {
  const filePath = getFixturePath(spec.name)

  if (!(await fixtureExists(spec.name))) {
    await generateFixture(spec)
  }

  return filePath
}

export async function ensureFixtures(): Promise<void> {
  // Create subdirectories for organized fixtures
  const subdirs = ["nested/deep", "src/components", "src/utils"]
  for (const subdir of subdirs) {
    await fs.mkdir(path.join(FIXTURE_DIR, subdir), { recursive: true })
  }

  // Generate main fixtures
  for (const spec of FIXTURES) {
    await ensureFixture(spec)
  }

  // Generate nested TypeScript files for Glob pattern testing
  const nestedTsFiles = [
    "nested/service.ts",
    "nested/deep/handler.ts",
    "src/components/Button.tsx",
    "src/components/Input.tsx",
    "src/utils/helpers.ts",
  ]

  for (const relativePath of nestedTsFiles) {
    const fullPath = path.join(FIXTURE_DIR, relativePath)
    const exists = await fixtureExists(relativePath)

    if (!exists) {
      await fs.writeFile(
        fullPath,
        generateTypeScriptContent(2 * 1024),
        "utf-8",
      )
    }
  }
}

export async function cleanupFixtures(): Promise<void> {
  const entries = await fs.readdir(FIXTURE_DIR)

  for (const entry of entries) {
    // Keep index.ts and .gitkeep
    if (entry === "index.ts" || entry === ".gitkeep") continue

    const entryPath = path.join(FIXTURE_DIR, entry)
    const stat = await fs.stat(entryPath)

    if (stat.isDirectory()) {
      await fs.rm(entryPath, { recursive: true })
    } else {
      await fs.unlink(entryPath)
    }
  }
}

// ============================================================================
// Exports
// ============================================================================

export { FIXTURES, FIXTURE_DIR }

// Entry point for manual fixture generation
if (import.meta.main) {
  console.log("Generating benchmark fixtures...")
  ensureFixtures()
    .then(() => console.log("Fixtures generated successfully"))
    .catch((err) => console.error("Failed to generate fixtures:", err))
}
