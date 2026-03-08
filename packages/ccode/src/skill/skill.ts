import z from "zod"
import path from "path"
import { Config } from "../config/config"
import { Instance } from "../project/instance"
import { NamedError } from "@codecoder-ai/core/util/error"
import { Log } from "@/util/log"
import { Global } from "@/global"
import { Filesystem } from "@/util/filesystem"
import { Flag } from "@/flag/flag"
import { Bus } from "@/bus"
import { Session } from "@/session"
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

  const CCODE_SKILL_GLOB = new Bun.Glob("{skill,skills}/**/SKILL.md")
  const CLAUDE_SKILL_GLOB = new Bun.Glob("skills/**/SKILL.md")
  const BUILTIN_SKILL_GLOB = new Bun.Glob("*/SKILL.md")

  /**
   * Parse skill metadata using native Rust parser.
   */
  const parseSkill = async (filePath: string): Promise<{ name: string; description: string } | undefined> => {
    if (!parseSkillFromFile) {
      throw new Error("Native parseSkillFromFile binding is unavailable")
    }

    try {
      const parsed = parseSkillFromFile(filePath)
      return {
        name: parsed.metadata.name,
        description: parsed.metadata.description,
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : `Failed to parse skill ${filePath}`
      Bus.publish(Session.Event.Error, { error: new NamedError.Unknown({ message }).toObject() })
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
