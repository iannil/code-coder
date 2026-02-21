/**
 * Skill API Handler
 * Handles /api/skills endpoints for skill management
 *
 * Provides:
 * - List installed skills
 * - Get skill details
 * - Install skill from URL or local path
 * - Uninstall skill
 * - Enable/disable skill
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { Skill } from "../../../skill/skill"
import { Global } from "../../../global"
import { ConfigMarkdown } from "../../../config/markdown"
import path from "path"
import fs from "fs/promises"

// ============================================================================
// Types
// ============================================================================

interface SkillMetadata {
  id: string
  name: string
  description: string
  version?: string
  author?: string
  category?: string
  location: string
  installed: boolean
  enabled: boolean
  lastUpdated?: string
  dependencies?: string[]
  readme?: string
}

interface InstallSkillRequest {
  /** URL or local path to skill package */
  source: string
  /** Whether to install globally (default: project-level) */
  global?: boolean
}

interface UpdateSkillRequest {
  /** Enable or disable the skill */
  enabled?: boolean
}

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

/** Parse extended metadata from SKILL.md frontmatter */
async function parseSkillMetadata(skillPath: string): Promise<Partial<SkillMetadata>> {
  const md = await ConfigMarkdown.parse(skillPath).catch(() => undefined)
  if (!md) return {}

  return {
    name: md.data.name as string | undefined,
    description: md.data.description as string | undefined,
    version: md.data.version as string | undefined,
    author: md.data.author as string | undefined,
    category: md.data.category as string | undefined,
    dependencies: md.data.dependencies as string[] | undefined,
    readme: md.content,
  }
}

/** Get skill enabled state from config */
async function getSkillEnabledState(): Promise<Record<string, boolean>> {
  const configPath = path.join(Global.Path.config, "skills.json")
  try {
    const data = await fs.readFile(configPath, "utf-8")
    return JSON.parse(data).enabled ?? {}
  } catch {
    return {}
  }
}

/** Save skill enabled state to config */
async function saveSkillEnabledState(state: Record<string, boolean>): Promise<void> {
  const configPath = path.join(Global.Path.config, "skills.json")
  await fs.mkdir(path.dirname(configPath), { recursive: true })
  await fs.writeFile(configPath, JSON.stringify({ enabled: state }, null, 2))
}

/** Get file stats for last modified time */
async function getLastModified(filePath: string): Promise<string | undefined> {
  try {
    const stats = await fs.stat(filePath)
    return stats.mtime.toISOString().split("T")[0]
  } catch {
    return undefined
  }
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/skills
 * List all installed skills with metadata
 */
export async function listSkills(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const skills = await Skill.all()
    const enabledState = await getSkillEnabledState()

    const skillList: SkillMetadata[] = await Promise.all(
      skills.map(async (skill) => {
        const metadata = await parseSkillMetadata(skill.location)
        const lastUpdated = await getLastModified(skill.location)

        return {
          id: skill.name,
          name: metadata.name ?? skill.name,
          description: metadata.description ?? skill.description,
          version: metadata.version ?? "1.0.0",
          author: metadata.author ?? "unknown",
          category: metadata.category ?? "General",
          location: skill.location,
          installed: true,
          enabled: enabledState[skill.name] !== false, // Default to enabled
          lastUpdated,
          dependencies: metadata.dependencies,
          readme: metadata.readme,
        }
      })
    )

    return jsonResponse({
      success: true,
      data: skillList,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/skills/:id
 * Get detailed information about a specific skill
 */
export async function getSkill(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Skill ID is required", 400)
    }

    const skill = await Skill.get(id)
    if (!skill) {
      return errorResponse(`Skill "${id}" not found`, 404)
    }

    const metadata = await parseSkillMetadata(skill.location)
    const enabledState = await getSkillEnabledState()
    const lastUpdated = await getLastModified(skill.location)

    const skillData: SkillMetadata = {
      id: skill.name,
      name: metadata.name ?? skill.name,
      description: metadata.description ?? skill.description,
      version: metadata.version ?? "1.0.0",
      author: metadata.author ?? "unknown",
      category: metadata.category ?? "General",
      location: skill.location,
      installed: true,
      enabled: enabledState[skill.name] !== false,
      lastUpdated,
      dependencies: metadata.dependencies,
      readme: metadata.readme,
    }

    return jsonResponse({
      success: true,
      data: skillData,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/skills/install
 * Install a skill from URL or local path
 */
export async function installSkill(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as InstallSkillRequest

    if (!input.source) {
      return errorResponse("source is required", 400)
    }

    // Determine target directory
    const targetBase = input.global
      ? path.join(Global.Path.home, ".codecoder", "skills")
      : path.join(process.cwd(), ".codecoder", "skills")

    let sourcePath: string
    let skillName: string

    // Handle URL vs local path
    if (input.source.startsWith("http://") || input.source.startsWith("https://")) {
      // For now, only support direct SKILL.md URLs or git repos
      return errorResponse("URL installation not yet implemented. Use local path.", 501)
    } else {
      // Local path
      sourcePath = path.resolve(input.source)

      // Check if source exists
      try {
        await fs.access(sourcePath)
      } catch {
        return errorResponse(`Source path does not exist: ${sourcePath}`, 400)
      }

      // Get skill name from SKILL.md
      const skillMdPath = sourcePath.endsWith("SKILL.md")
        ? sourcePath
        : path.join(sourcePath, "SKILL.md")

      const metadata = await parseSkillMetadata(skillMdPath)
      skillName = metadata.name ?? path.basename(path.dirname(skillMdPath))

      // Copy skill directory to target
      const targetDir = path.join(targetBase, skillName)
      const sourceDir = path.dirname(skillMdPath)

      await fs.mkdir(targetDir, { recursive: true })

      // Copy all files from source to target
      const files = await fs.readdir(sourceDir)
      for (const file of files) {
        const srcFile = path.join(sourceDir, file)
        const dstFile = path.join(targetDir, file)
        const stat = await fs.stat(srcFile)
        if (stat.isFile()) {
          await fs.copyFile(srcFile, dstFile)
        }
      }

      // Enable by default
      const enabledState = await getSkillEnabledState()
      enabledState[skillName] = true
      await saveSkillEnabledState(enabledState)

      return jsonResponse(
        {
          success: true,
          data: {
            name: skillName,
            location: targetDir,
            message: `Skill "${skillName}" installed successfully`,
          },
        },
        201
      )
    }
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/skills/:id
 * Uninstall a skill
 */
export async function uninstallSkill(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Skill ID is required", 400)
    }

    const skill = await Skill.get(id)
    if (!skill) {
      return errorResponse(`Skill "${id}" not found`, 404)
    }

    // Only allow uninstalling user-installed skills
    const skillDir = path.dirname(skill.location)
    const isUserSkill =
      skillDir.includes(".codecoder/skills") || skillDir.includes(".claude/skills")

    if (!isUserSkill) {
      return errorResponse("Cannot uninstall built-in skills", 403)
    }

    // Remove skill directory
    await fs.rm(skillDir, { recursive: true, force: true })

    // Remove from enabled state
    const enabledState = await getSkillEnabledState()
    delete enabledState[id]
    await saveSkillEnabledState(enabledState)

    return jsonResponse({
      success: true,
      data: {
        name: id,
        message: `Skill "${id}" uninstalled successfully`,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PATCH /api/skills/:id
 * Update skill settings (enable/disable)
 */
export async function updateSkill(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Skill ID is required", 400)
    }

    const skill = await Skill.get(id)
    if (!skill) {
      return errorResponse(`Skill "${id}" not found`, 404)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as UpdateSkillRequest

    // Update enabled state
    if (input.enabled !== undefined) {
      const enabledState = await getSkillEnabledState()
      enabledState[id] = input.enabled
      await saveSkillEnabledState(enabledState)
    }

    // Fetch updated skill data
    const metadata = await parseSkillMetadata(skill.location)
    const enabledState = await getSkillEnabledState()

    return jsonResponse({
      success: true,
      data: {
        id: skill.name,
        name: metadata.name ?? skill.name,
        enabled: enabledState[skill.name] !== false,
        message: `Skill "${id}" updated successfully`,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/skills/categories
 * List all skill categories
 */
export async function listSkillCategories(
  _req: HttpRequest,
  _params: RouteParams
): Promise<HttpResponse> {
  try {
    const skills = await Skill.all()

    const categories = new Set<string>()
    categories.add("General") // Default category

    for (const skill of skills) {
      const metadata = await parseSkillMetadata(skill.location)
      if (metadata.category) {
        categories.add(metadata.category)
      }
    }

    return jsonResponse({
      success: true,
      data: Array.from(categories).sort(),
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
