/**
 * Memory Factory Tests
 */

import { describe, it, expect, afterEach } from "bun:test"
import { createMemory, getDefaultConfig } from "../src/factory"
import { SqliteMemory } from "../src/backends/sqlite"
import { MarkdownMemory } from "../src/backends/markdown"
import { CompositeMemory } from "../src/backends/composite"
import path from "path"
import os from "os"

describe("createMemory", () => {
  const tempDir = path.join(os.tmpdir(), `memory-factory-test-${Date.now()}`)
  const memories: Array<{ close: () => Promise<void> }> = []

  afterEach(async () => {
    for (const memory of memories) {
      await memory.close()
    }
    memories.length = 0
    try {
      Bun.spawnSync(["rm", "-rf", tempDir])
    } catch {
      // Ignore cleanup errors
    }
  })

  it("should create SQLite memory by default", () => {
    const memory = createMemory({
      backend: "sqlite",
      sqlite: { dbPath: path.join(tempDir, "default.db"), readOnly: false },
    })
    memories.push(memory)

    expect(memory).toBeInstanceOf(SqliteMemory)
    expect(memory.name).toBe("sqlite")
  })

  it("should create Markdown memory", () => {
    const memory = createMemory({
      backend: "markdown",
      markdown: { basePath: path.join(tempDir, "md") },
    })
    memories.push(memory)

    expect(memory).toBeInstanceOf(MarkdownMemory)
    expect(memory.name).toBe("markdown")
  })

  it("should create Composite memory", () => {
    const memory = createMemory({
      backend: "composite",
      sqlite: { dbPath: path.join(tempDir, "comp.db"), readOnly: false },
      markdown: { basePath: path.join(tempDir, "comp-md") },
    })
    memories.push(memory)

    expect(memory).toBeInstanceOf(CompositeMemory)
    expect(memory.name).toBe("composite")
  })

  it("should throw for invalid backend", () => {
    expect(() =>
      createMemory({
        // @ts-expect-error Testing invalid input
        backend: "invalid",
      }),
    ).toThrow()
  })
})

describe("getDefaultConfig", () => {
  it("should return default configuration", () => {
    const config = getDefaultConfig()

    expect(config.backend).toBe("sqlite")
    expect(config.sqlite.dbPath).toBe("~/.codecoder/workspace/memory/brain.db")
    expect(config.sqlite.vectorWeight).toBe(0.7)
    expect(config.sqlite.keywordWeight).toBe(0.3)
    expect(config.markdown.basePath).toBe("./memory")
    expect(config.markdown.longTermFile).toBe("MEMORY.md")
    expect(config.composite.primary).toBe("sqlite")
    expect(config.composite.writeToAll).toBe(true)
  })
})
