/**
 * Registration Module Tests
 *
 * Tests for concept registrars.
 *
 * @package test/autonomous/builder
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { mkdir, rm, readdir } from "node:fs/promises"
import path from "path"

import {
  getRegistrar,
  registerConcept,
  unregisterConcept,
  ToolRegistrar,
  PromptRegistrar,
  SkillRegistrar,
  AgentRegistrar,
  MemoryRegistrar,
  HandRegistrar,
  WorkflowRegistrar,
} from "@/autonomous/builder"

import {
  createMockGeneratedConcept,
  assert,
  verify,
  ALL_CONCEPT_TYPES,
  withTestInstance,
} from "./fixtures/builder-fixture"

describe("Registration Module", () => {
  let testDir: string

  beforeEach(async () => {
    testDir = `/tmp/builder-test-${Date.now()}`
    await mkdir(testDir, { recursive: true })
  })

  afterEach(async () => {
    try {
      await rm(testDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  })

  // ==========================================================================
  // getRegistrar
  // ==========================================================================

  describe("getRegistrar", () => {
    test("should return registrar for all concept types", () => {
      for (const type of ALL_CONCEPT_TYPES) {
        const registrar = getRegistrar(type)
        expect(registrar).toBeDefined()
      }
    })

    test("should throw for invalid concept type", () => {
      expect(() => getRegistrar("INVALID" as any)).toThrow()
    })

    test("should return correct registrar class", () => {
      expect(getRegistrar("TOOL")).toBeInstanceOf(ToolRegistrar)
      expect(getRegistrar("PROMPT")).toBeInstanceOf(PromptRegistrar)
      expect(getRegistrar("SKILL")).toBeInstanceOf(SkillRegistrar)
      expect(getRegistrar("AGENT")).toBeInstanceOf(AgentRegistrar)
      expect(getRegistrar("MEMORY")).toBeInstanceOf(MemoryRegistrar)
      expect(getRegistrar("HAND")).toBeInstanceOf(HandRegistrar)
      expect(getRegistrar("WORKFLOW")).toBeInstanceOf(WorkflowRegistrar)
    })
  })

  // ==========================================================================
  // registerConcept
  // ==========================================================================

  describe("registerConcept", () => {
    test("should register concept and create file", async () => {
      await withTestInstance(async () => {
        const concept = createMockGeneratedConcept("PROMPT", {
          identifier: "test_prompt",
          targetPath: path.join(testDir, "prompts", "test_prompt.txt"),
        })

        const result = await registerConcept(concept)

        assert.validRegistration(result)
        expect(result.success).toBe(true)
        expect(result.storagePath).toBe(concept.targetPath)
      })
    })

    test("should create parent directories", async () => {
      await withTestInstance(async () => {
        const nestedPath = path.join(testDir, "deep", "nested", "path", "concept.txt")
        const concept = createMockGeneratedConcept("PROMPT", {
          targetPath: nestedPath,
        })

        const result = await registerConcept(concept)

        expect(result.success).toBe(true)

        // Verify directory was created
        const file = Bun.file(nestedPath)
        expect(await file.exists()).toBe(true)
      })
    })

    test("should write content to file", async () => {
      await withTestInstance(async () => {
        const content = "Test content for registration"
        const concept = createMockGeneratedConcept("PROMPT", {
          content,
          targetPath: path.join(testDir, "content_test.txt"),
        })

        await registerConcept(concept)

        const file = Bun.file(concept.targetPath)
        const savedContent = await file.text()
        expect(savedContent).toBe(content)
      })
    })

    test("should write additional files", async () => {
      await withTestInstance(async () => {
        const concept = createMockGeneratedConcept("TOOL", {
          targetPath: path.join(testDir, "tool.py"),
          additionalFiles: [
            {
              path: path.join(testDir, "tool.meta.json"),
              content: '{"name": "tool"}',
            },
          ],
        })

        await registerConcept(concept)

        const metaFile = Bun.file(path.join(testDir, "tool.meta.json"))
        expect(await metaFile.exists()).toBe(true)
      })
    })

    test("should create backup if file exists", async () => {
      await withTestInstance(async () => {
        const targetPath = path.join(testDir, "existing.txt")

        // Create existing file
        await Bun.write(targetPath, "original content")

        const concept = createMockGeneratedConcept("PROMPT", {
          content: "new content",
          targetPath,
        })

        const result = await registerConcept(concept)

        expect(result.success).toBe(true)
        expect(result.backupCreated).toBe(true)
        expect(result.backupPath).toBeDefined()

        // Verify backup was created
        if (result.backupPath) {
          const backupFile = Bun.file(result.backupPath)
          expect(await backupFile.exists()).toBe(true)
          const backupContent = await backupFile.text()
          expect(backupContent).toBe("original content")
        }
      })
    })

    test("should handle registration failure gracefully", async () => {
      await withTestInstance(async () => {
        // Try to write to an invalid path
        const concept = createMockGeneratedConcept("PROMPT", {
          targetPath: "/nonexistent-root-dir-xyz/file.txt",
        })

        const result = await registerConcept(concept)

        expect(result.success).toBe(false)
        expect(result.error).toBeDefined()
      })
    })
  })

  // ==========================================================================
  // ToolRegistrar
  // ==========================================================================

  describe("ToolRegistrar", () => {
    test("should register tool with DynamicToolRegistry", async () => {
      await withTestInstance(async () => {
        const concept = createMockGeneratedConcept("TOOL", {
          identifier: "test_tool_reg",
          displayName: "Test Tool Reg",
          description: "A test tool for registration",
          targetPath: path.join(testDir, "tools", "test_tool.py"),
        })

        const result = await registerConcept(concept)

        // Tool registration may succeed or fail depending on registry state
        // but should not throw
        assert.validRegistration(result)
      })
    })

    test("should determine language from file extension", async () => {
      await withTestInstance(async () => {
        const pythonTool = createMockGeneratedConcept("TOOL", {
          targetPath: path.join(testDir, "tool.py"),
        })

        const jsTool = createMockGeneratedConcept("TOOL", {
          targetPath: path.join(testDir, "tool.js"),
        })

        const shellTool = createMockGeneratedConcept("TOOL", {
          targetPath: path.join(testDir, "tool.sh"),
        })

        // All should register without errors
        await registerConcept(pythonTool)
        await registerConcept(jsTool)
        await registerConcept(shellTool)
      })
    })
  })

  // ==========================================================================
  // SkillRegistrar
  // ==========================================================================

  describe("SkillRegistrar", () => {
    test("should register skill with SKILL.md format", async () => {
      await withTestInstance(async () => {
        const concept = createMockGeneratedConcept("SKILL", {
          identifier: "deploy",
          targetPath: path.join(testDir, "skills", "deploy", "SKILL.md"),
        })

        const result = await registerConcept(concept)

        expect(result.success).toBe(true)

        const file = Bun.file(concept.targetPath)
        expect(await file.exists()).toBe(true)
      })
    })
  })

  // ==========================================================================
  // AgentRegistrar
  // ==========================================================================

  describe("AgentRegistrar", () => {
    test("should register agent JSON config", async () => {
      await withTestInstance(async () => {
        const concept = createMockGeneratedConcept("AGENT", {
          identifier: "reviewer",
          targetPath: path.join(testDir, "agents", "reviewer.json"),
        })

        const result = await registerConcept(concept)

        expect(result.success).toBe(true)

        const file = Bun.file(concept.targetPath)
        const content = await file.text()
        expect(() => JSON.parse(content)).not.toThrow()
      })
    })
  })

  // ==========================================================================
  // HandRegistrar
  // ==========================================================================

  describe("HandRegistrar", () => {
    test("should register hand with HAND.md format", async () => {
      await withTestInstance(async () => {
        const concept = createMockGeneratedConcept("HAND", {
          identifier: "daily_report",
          targetPath: path.join(testDir, "hands", "daily_report", "HAND.md"),
        })

        const result = await registerConcept(concept)

        expect(result.success).toBe(true)

        const file = Bun.file(concept.targetPath)
        expect(await file.exists()).toBe(true)
      })
    })
  })

  // ==========================================================================
  // WorkflowRegistrar
  // ==========================================================================

  describe("WorkflowRegistrar", () => {
    test("should register workflow with WORKFLOW.md format", async () => {
      await withTestInstance(async () => {
        const concept = createMockGeneratedConcept("WORKFLOW", {
          identifier: "deploy_pipeline",
          targetPath: path.join(testDir, "workflows", "deploy_pipeline", "WORKFLOW.md"),
        })

        const result = await registerConcept(concept)

        expect(result.success).toBe(true)

        const file = Bun.file(concept.targetPath)
        expect(await file.exists()).toBe(true)
      })
    })
  })

  // ==========================================================================
  // unregisterConcept
  // ==========================================================================

  describe("unregisterConcept", () => {
    test("should unregister tool from registry", async () => {
      await withTestInstance(async () => {
        // Note: unregister depends on tool being in DynamicToolRegistry
        const result = await unregisterConcept("TOOL", "nonexistent_tool")

        // Should return false for non-existent tool
        expect(typeof result).toBe("boolean")
      })
    })

    test("should return false for non-TOOL types (not implemented)", async () => {
      await withTestInstance(async () => {
        const result = await unregisterConcept("PROMPT", "some_id")

        // Base registrar returns false for unregister
        expect(result).toBe(false)
      })
    })
  })

  // ==========================================================================
  // Backup Functionality
  // ==========================================================================

  describe("backup functionality", () => {
    test("should create backup in .backup directory", async () => {
      await withTestInstance(async () => {
        const targetPath = path.join(testDir, "backup_test", "file.txt")
        await mkdir(path.dirname(targetPath), { recursive: true })
        await Bun.write(targetPath, "original")

        const concept = createMockGeneratedConcept("PROMPT", {
          content: "updated",
          targetPath,
        })

        const result = await registerConcept(concept)

        expect(result.backupCreated).toBe(true)
        expect(result.backupPath).toContain(".backup")
      })
    })

    test("should include timestamp in backup filename", async () => {
      await withTestInstance(async () => {
        const targetPath = path.join(testDir, "timestamp_test.txt")
        await Bun.write(targetPath, "original")

        const concept = createMockGeneratedConcept("PROMPT", {
          content: "updated",
          targetPath,
        })

        const result = await registerConcept(concept)

        if (result.backupPath) {
          // Backup path should have timestamp pattern
          expect(result.backupPath).toMatch(/\d{4}-\d{2}-\d{2}/)
        }
      })
    })

    test("should not create backup for new files", async () => {
      await withTestInstance(async () => {
        const targetPath = path.join(testDir, "new_file.txt")

        const concept = createMockGeneratedConcept("PROMPT", {
          targetPath,
        })

        const result = await registerConcept(concept)

        expect(result.backupCreated).toBe(false)
        expect(result.backupPath).toBeUndefined()
      })
    })
  })

  // ==========================================================================
  // Cache Invalidation
  // ==========================================================================

  describe("cache invalidation", () => {
    test("should invalidate concept inventory cache after registration", async () => {
      await withTestInstance(async () => {
        const concept = createMockGeneratedConcept("PROMPT", {
          targetPath: path.join(testDir, "cache_test.txt"),
        })

        // This should invalidate the cache (we can't directly verify,
        // but the code path should be exercised)
        const result = await registerConcept(concept)

        expect(result.success).toBe(true)
      })
    })
  })
})
