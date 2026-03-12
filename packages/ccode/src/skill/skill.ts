import z from "zod"
import path from "path"
import fs from "fs/promises"
import { Config } from "@/config/config"
import { Instance } from "@/project/instance"
import { NamedError } from "@codecoder-ai/core/util/error"
import { Log } from "@/util/log"
import { Global } from "@/util/global"
import { Filesystem } from "@/util/filesystem"
import { Flag } from "@/util/flag/flag"
import { Bus } from "@/bus"
import { BusEvent } from "@/bus/bus-event"
import { parseSkillFromFile } from "@codecoder-ai/core"

export namespace Skill {
  const log = Log.create({ service: "skill" })
  export const Info = z.object({
    name: z.string(),
    description: z.string(),
    location: z.string(),
  })
  export type Info = z.infer<typeof Info>

  export const InvalidError = NamedError.create(
    "SkillInvalidError",
    z.object({
      path: z.string(),
      message: z.string().optional(),
      issues: z.custom<z.core.$ZodIssue[]>().optional(),
    }),
  )

  export const NameMismatchError = NamedError.create(
    "SkillNameMismatchError",
    z.object({
      path: z.string(),
      expected: z.string(),
      actual: z.string(),
    }),
  )

  /**
   * Error event for skill loading failures.
   * Mirrors Session.Event.Error for compatibility with existing Bus subscribers.
   */
  export const Event = {
    Error: BusEvent.define(
      "skill.error",
      z.object({
        error: z.object({
          name: z.string(),
          message: z.string().optional(),
        }).passthrough(),
      }),
    ),
  }

  const CCODE_SKILL_GLOB = new Bun.Glob("{skill,skills}/**/SKILL.md")
  const CLAUDE_SKILL_GLOB = new Bun.Glob("skills/**/SKILL.md")
  const BUILTIN_SKILL_GLOB = new Bun.Glob("*/SKILL.md")

  /**
   * JavaScript fallback for parsing skill YAML frontmatter.
   * Used when native parseSkillFromFile binding is unavailable.
   */
  const parseSkillFallback = async (
    filePath: string,
  ): Promise<{ name: string; description: string } | undefined> => {
    const content = await fs.readFile(filePath, "utf-8")

    // Extract YAML frontmatter (between --- markers)
    const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---/)
    if (!frontmatterMatch) {
      log.warn("no frontmatter found in skill file", { filePath })
      return undefined
    }

    const frontmatter = frontmatterMatch[1]

    // Parse simple YAML key-value pairs
    const getName = (yaml: string): string | undefined => {
      const match = yaml.match(/^name:\s*(.+)$/m)
      return match ? match[1].trim().replace(/^["']|["']$/g, "") : undefined
    }

    const getDescription = (yaml: string): string | undefined => {
      const match = yaml.match(/^description:\s*(.+)$/m)
      return match ? match[1].trim().replace(/^["']|["']$/g, "") : undefined
    }

    const name = getName(frontmatter)
    const description = getDescription(frontmatter) ?? ""

    if (!name) {
      log.warn("skill file missing name in frontmatter", { filePath })
      return undefined
    }

    return { name, description }
  }

  /**
   * Parse skill metadata using native Rust parser with JavaScript fallback.
   */
  const parseSkill = async (filePath: string): Promise<{ name: string; description: string } | undefined> => {
    // Use native parser if available, otherwise fallback to JavaScript
    if (parseSkillFromFile) {
      try {
        const parsed = parseSkillFromFile(filePath)
        return {
          name: parsed.metadata.name,
          description: parsed.metadata.description,
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : `Failed to parse skill ${filePath}`
        Bus.publish(Event.Error, { error: new NamedError.Unknown({ message }).toObject() })
        log.error("failed to load skill with native parser", { skill: filePath, err })
        return undefined
      }
    }

    // Fallback to JavaScript parser
    try {
      return await parseSkillFallback(filePath)
    } catch (err) {
      const message = err instanceof Error ? err.message : `Failed to parse skill ${filePath}`
      Bus.publish(Event.Error, { error: new NamedError.Unknown({ message }).toObject() })
      log.error("failed to load skill", { skill: filePath, err })
      return undefined
    }
  }

  export const state = Instance.state(async () => {
    const skills: Record<string, Info> = {}

    const addSkill = async (match: string) => {
      const parsed = await parseSkill(match)
      if (!parsed) return

      // Warn on duplicate skill names
      if (skills[parsed.name]) {
        log.warn("duplicate skill name", {
          name: parsed.name,
          existing: skills[parsed.name].location,
          duplicate: match,
        })
      }

      skills[parsed.name] = {
        name: parsed.name,
        description: parsed.description,
        location: match,
      }
    }

    // Scan builtin skills directory first
    const builtinDir = path.join(import.meta.dirname, "builtin")
    for await (const match of BUILTIN_SKILL_GLOB.scan({
      cwd: builtinDir,
      absolute: true,
      onlyFiles: true,
      followSymlinks: true,
    })) {
      await addSkill(match)
    }

    // Scan .claude/skills/ and .codecoder/skills/ directories (project-level)
    const claudeDirs = await Array.fromAsync(
      Filesystem.up({
        targets: [".claude", ".codecoder"],
        start: Instance.directory,
        stop: Instance.worktree,
      }),
    )
    // Also include global ~/.claude/skills/
    const globalClaude = `${Global.Path.home}/.claude`
    if (await Filesystem.isDir(globalClaude)) {
      claudeDirs.push(globalClaude)
    }

    if (!Flag.CCODE_DISABLE_CLAUDE_CODE_SKILLS) {
      for (const dir of claudeDirs) {
        const matches = await Array.fromAsync(
          CLAUDE_SKILL_GLOB.scan({
            cwd: dir,
            absolute: true,
            onlyFiles: true,
            followSymlinks: true,
            dot: true,
          }),
        ).catch((error) => {
          log.error("failed .claude directory scan for skills", { dir, error })
          return []
        })

        for (const match of matches) {
          await addSkill(match)
        }
      }
    }

    // Scan .codecoder/skills/ directories
    for (const dir of await Config.directories()) {
      for await (const match of CCODE_SKILL_GLOB.scan({
        cwd: dir,
        absolute: true,
        onlyFiles: true,
        followSymlinks: true,
      })) {
        await addSkill(match)
      }
    }

    return skills
  })

  export async function get(name: string) {
    return state().then((x) => x[name])
  }

  export async function all() {
    return state().then((x) => Object.values(x))
  }
}
