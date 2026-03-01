/**
 * Concept Registration
 *
 * Registers validated concepts to their appropriate storage locations.
 * Handles file writing, registry updates, and backup creation.
 *
 * @package autonomous/builder
 */

import { Log } from "@/util/log"
import { Filesystem } from "@/util/filesystem"
import { DynamicToolRegistry, type ToolTypes } from "@/memory/tools"
import { mkdir } from "node:fs/promises"
import path from "path"

import {
  type ConceptType,
  type GeneratedConcept,
  type RegistrationResult,
  type ConceptRegistrar,
} from "./types"
import { getConceptInventory } from "./concept-inventory"

const log = Log.create({ service: "autonomous.builder.registration" })

// ============================================================================
// Base Registrar
// ============================================================================

class BaseRegistrar implements ConceptRegistrar {
  async register(concept: GeneratedConcept): Promise<RegistrationResult> {
    log.info("Registering concept", {
      type: concept.type,
      identifier: concept.identifier,
      targetPath: concept.targetPath,
    })

    try {
      // Ensure target directory exists
      const targetDir = path.dirname(concept.targetPath)
      await mkdir(targetDir, { recursive: true })

      // Check if file exists and create backup if needed
      let backupPath: string | undefined
      const exists = await Filesystem.exists(concept.targetPath)
      if (exists) {
        backupPath = await this.createBackup(concept.targetPath)
      }

      // Write main content
      await Bun.write(concept.targetPath, concept.content)

      // Write additional files
      if (concept.additionalFiles) {
        for (const file of concept.additionalFiles) {
          const fileDir = path.dirname(file.path)
          await mkdir(fileDir, { recursive: true })
          await Bun.write(file.path, file.content)
        }
      }

      // Invalidate concept inventory cache
      getConceptInventory().invalidateCache()

      log.info("Concept registered successfully", {
        conceptId: concept.identifier,
        storagePath: concept.targetPath,
      })

      return {
        success: true,
        conceptId: concept.identifier,
        storagePath: concept.targetPath,
        backupCreated: Boolean(backupPath),
        backupPath,
      }
    } catch (error) {
      log.error("Failed to register concept", { error })
      return {
        success: false,
        error: String(error),
      }
    }
  }

  async unregister(conceptId: string): Promise<boolean> {
    log.warn("Unregister not implemented for base registrar", { conceptId })
    return false
  }

  protected async createBackup(filePath: string): Promise<string> {
    const backupDir = path.join(path.dirname(filePath), ".backup")
    await mkdir(backupDir, { recursive: true })

    const timestamp = new Date().toISOString().replace(/[:.]/g, "-")
    const fileName = path.basename(filePath)
    const backupPath = path.join(backupDir, `${fileName}.${timestamp}.bak`)

    const content = await Bun.file(filePath).text()
    await Bun.write(backupPath, content)

    log.info("Created backup", { original: filePath, backup: backupPath })
    return backupPath
  }
}

// ============================================================================
// Type-Specific Registrars
// ============================================================================

export class ToolRegistrar extends BaseRegistrar {
  override async register(concept: GeneratedConcept): Promise<RegistrationResult> {
    log.info("Registering TOOL concept", { identifier: concept.identifier })

    try {
      // Parse metadata if available
      const metadataFile = concept.additionalFiles?.find((f) => f.path.endsWith(".meta.json"))
      let metadata: ToolTypes.DynamicTool["metadata"] | undefined

      if (metadataFile) {
        try {
          const parsed = JSON.parse(metadataFile.content)
          metadata = parsed.metadata
        } catch {
          // Use defaults
        }
      }

      // Determine language from file extension
      const ext = concept.targetPath.split(".").pop()
      const language: "python" | "nodejs" | "bash" =
        ext === "py" ? "python" : ext === "js" ? "nodejs" : "bash"

      // Register with DynamicToolRegistry
      const toolInput: ToolTypes.CreateToolInput = {
        name: concept.displayName,
        description: concept.description,
        tags: concept.metadata.tags ?? [],
        code: concept.content,
        language,
        parameters: [],
        examples: [],
        createdBy: "agent",
        sourceTask: concept.metadata.generatedBy,
      }

      const tool = await DynamicToolRegistry.register(toolInput)

      // Also write to file system for persistence
      await super.register(concept)

      return {
        success: true,
        conceptId: tool.id,
        storagePath: concept.targetPath,
      }
    } catch (error) {
      log.error("Failed to register tool", { error })
      return {
        success: false,
        error: String(error),
      }
    }
  }

  override async unregister(conceptId: string): Promise<boolean> {
    try {
      await DynamicToolRegistry.remove(conceptId)
      return true
    } catch (error) {
      log.error("Failed to unregister tool", { error })
      return false
    }
  }
}

export class PromptRegistrar extends BaseRegistrar {
  // Uses base implementation - just writes files
}

export class SkillRegistrar extends BaseRegistrar {
  // Uses base implementation - writes SKILL.md files
  // Skills are discovered dynamically from file system
}

export class AgentRegistrar extends BaseRegistrar {
  override async register(concept: GeneratedConcept): Promise<RegistrationResult> {
    // Write agent JSON config
    const result = await super.register(concept)

    if (result.success) {
      // Note: Agent system will pick up new config on next Agent.list() call
      // via Instance.state() invalidation
      log.info("Agent registered - restart may be needed for full effect", {
        identifier: concept.identifier,
      })
    }

    return result
  }
}

export class MemoryRegistrar extends BaseRegistrar {
  // Uses base implementation - writes JSON Schema files
}

export class HandRegistrar extends BaseRegistrar {
  override async register(concept: GeneratedConcept): Promise<RegistrationResult> {
    // Write HAND.md file
    const result = await super.register(concept)

    if (result.success) {
      log.info("Hand registered (disabled by default) - enable manually when ready", {
        identifier: concept.identifier,
        path: concept.targetPath,
      })
    }

    return result
  }
}

export class WorkflowRegistrar extends BaseRegistrar {
  override async register(concept: GeneratedConcept): Promise<RegistrationResult> {
    // Write WORKFLOW.md file
    const result = await super.register(concept)

    if (result.success) {
      log.info("Workflow registered (disabled by default) - enable manually when ready", {
        identifier: concept.identifier,
        path: concept.targetPath,
      })
    }

    return result
  }
}

// ============================================================================
// Registrar Factory
// ============================================================================

const registrars: Map<ConceptType, ConceptRegistrar> = new Map([
  ["TOOL", new ToolRegistrar()],
  ["PROMPT", new PromptRegistrar()],
  ["SKILL", new SkillRegistrar()],
  ["AGENT", new AgentRegistrar()],
  ["MEMORY", new MemoryRegistrar()],
  ["HAND", new HandRegistrar()],
  ["WORKFLOW", new WorkflowRegistrar()],
])

/**
 * Get the registrar for a concept type
 */
export function getRegistrar(type: ConceptType): ConceptRegistrar {
  const registrar = registrars.get(type)
  if (!registrar) {
    throw new Error(`No registrar registered for concept type: ${type}`)
  }
  return registrar
}

/**
 * Register a concept
 */
export async function registerConcept(concept: GeneratedConcept): Promise<RegistrationResult> {
  const registrar = getRegistrar(concept.type)
  return registrar.register(concept)
}

/**
 * Unregister a concept
 */
export async function unregisterConcept(type: ConceptType, conceptId: string): Promise<boolean> {
  const registrar = getRegistrar(type)
  return registrar.unregister(conceptId)
}
