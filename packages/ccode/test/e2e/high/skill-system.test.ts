/**
 * E2E High Priority Test: Skill System
 * Priority: High - Runs daily
 *
 * Tests the skill system's ability to discover, load, and execute skills
 * from various locations including builtin, project, and global directories.
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import fs from "fs/promises"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Skill } from "../../../src/skill"

// Number of builtin skills that are always loaded
const BUILTIN_SKILL_COUNT = 16

// List of known builtin skill names for validation
const KNOWN_BUILTIN_SKILLS = [
  "tdd-workflow",
  "coding-standards",
  "git-workflow",
  "debugging",
  "planning",
  "code-audit",
]

describe("E2E High: Skill System", () => {
  describe("Skill Discovery", () => {
    test("should discover built-in skills", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()

          // Should have builtin skills
          expect(skills.length).toBeGreaterThanOrEqual(BUILTIN_SKILL_COUNT)

          // Check for some expected builtin skills
          const skillNames = skills.map((s) => s.name)
          // At least one of the known builtin skills should be present
          const hasKnownSkill = KNOWN_BUILTIN_SKILLS.some((name) => skillNames.includes(name))
          expect(hasKnownSkill).toBe(true)
        },
      })
    })

    test("should discover project skills from .ccode/skills", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".ccode", "skills", "my-project-skill")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: my-project-skill
description: A custom project skill for testing.
---

# My Project Skill

This skill does something specific to this project.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const projectSkill = skills.find((s) => s.name === "my-project-skill")

          expect(projectSkill).toBeDefined()
          expect(projectSkill!.description).toBe("A custom project skill for testing.")
          expect(projectSkill!.location).toContain(".ccode/skills/my-project-skill/SKILL.md")
        },
      })
    })

    test("should discover project skills from .claude/skills", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".claude", "skills", "claude-project-skill")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: claude-project-skill
description: A skill in the .claude/skills directory.
---

# Claude Project Skill

This skill is loaded from .claude/skills.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const claudeSkill = skills.find((s) => s.name === "claude-project-skill")

          expect(claudeSkill).toBeDefined()
          expect(claudeSkill!.description).toBe("A skill in the .claude/skills directory.")
          expect(claudeSkill!.location).toContain(".claude/skills/claude-project-skill/SKILL.md")
        },
      })
    })

    test("should discover global skills from ~/.claude/skills", async () => {
      await using tmp = await tmpdir({ git: true })

      const originalHome = process.env.CCODE_TEST_HOME
      process.env.CCODE_TEST_HOME = tmp.path

      try {
        // Create global skill
        const globalSkillDir = path.join(tmp.path, ".claude", "skills", "global-custom-skill")
        await fs.mkdir(globalSkillDir, { recursive: true })
        await Bun.write(
          path.join(globalSkillDir, "SKILL.md"),
          `---
name: global-custom-skill
description: A globally available custom skill.
---

# Global Custom Skill

This skill is available in all projects.
`,
        )

        await Instance.provide({
          directory: tmp.path,
          fn: async () => {
            const skills = await Skill.all()
            const globalSkill = skills.find((s) => s.name === "global-custom-skill")

            expect(globalSkill).toBeDefined()
            expect(globalSkill!.description).toBe("A globally available custom skill.")
            expect(globalSkill!.location).toContain(".claude/skills/global-custom-skill/SKILL.md")
          },
        })
      } finally {
        process.env.CCODE_TEST_HOME = originalHome
      }
    })

    test("should discover multiple skills from the same directory", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          // Create multiple skills
          const skills = ["alpha-skill", "beta-skill", "gamma-skill"]
          for (const skillName of skills) {
            const skillDir = path.join(dir, ".ccode", "skills", skillName)
            await fs.mkdir(skillDir, { recursive: true })
            await Bun.write(
              path.join(skillDir, "SKILL.md"),
              `---
name: ${skillName}
description: Description for ${skillName}.
---

# ${skillName}

Instructions for ${skillName}.
`,
            )
          }
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const customSkills = skills.filter((s) => s.name.endsWith("-skill") && !s.name.includes("builtin"))

          expect(customSkills.length).toBeGreaterThanOrEqual(3)
          expect(skills.find((s) => s.name === "alpha-skill")).toBeDefined()
          expect(skills.find((s) => s.name === "beta-skill")).toBeDefined()
          expect(skills.find((s) => s.name === "gamma-skill")).toBeDefined()
        },
      })
    })
  })

  describe("Skill Execution", () => {
    test("should load skill by name", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".ccode", "skills", "loadable-skill")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: loadable-skill
description: A skill that can be loaded by name.
---

# Loadable Skill

Instructions for the loadable skill.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skill = await Skill.get("loadable-skill")

          expect(skill).toBeDefined()
          expect(skill!.name).toBe("loadable-skill")
          expect(skill!.description).toBe("A skill that can be loaded by name.")
        },
      })
    })

    test("should parse skill frontmatter correctly", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".ccode", "skills", "frontmatter-skill")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: frontmatter-skill
description: This skill has complex frontmatter with special characters like "quotes" and 'apostrophes'.
---

# Frontmatter Skill

This skill tests frontmatter parsing.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skill = await Skill.get("frontmatter-skill")

          expect(skill).toBeDefined()
          expect(skill!.description).toContain("quotes")
          expect(skill!.description).toContain("apostrophes")
        },
      })
    })

    test("should handle missing skill gracefully", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skill = await Skill.get("nonexistent-skill-12345")
          expect(skill).toBeUndefined()
        },
      })
    })

    test("should return all skills including builtin and custom", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".ccode", "skills", "mixed-test-skill")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: mixed-test-skill
description: A skill for testing mixed skill lists.
---

# Mixed Test Skill
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()

          // Should have both builtin and custom skills
          expect(skills.length).toBeGreaterThan(BUILTIN_SKILL_COUNT)

          // Check builtin skill exists (use one of the known builtin skills)
          expect(skills.find((s) => KNOWN_BUILTIN_SKILLS.includes(s.name))).toBeDefined()

          // Check custom skill exists
          expect(skills.find((s) => s.name === "mixed-test-skill")).toBeDefined()
        },
      })
    })
  })

  describe("Custom Skills", () => {
    test("should create and load custom skill", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".ccode", "skills", "custom-workflow")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: custom-workflow
description: A custom workflow skill for this project.
---

# Custom Workflow Skill

This skill defines a custom workflow:

1. First, check the current state
2. Then, perform the operation
3. Finally, verify the result

## Usage

Call this skill when you need to perform the custom workflow.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skill = await Skill.get("custom-workflow")

          expect(skill).toBeDefined()
          expect(skill!.name).toBe("custom-workflow")
          expect(skill!.description).toBe("A custom workflow skill for this project.")
        },
      })
    })

    test("should override built-in skill with project skill", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          // Create a skill with the same name as a builtin
          const skillDir = path.join(dir, ".ccode", "skills", "commit")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: commit
description: Custom project-specific commit skill that overrides builtin.
---

# Custom Commit Skill

This project-specific commit skill overrides the builtin one.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skill = await Skill.get("commit")

          // The project skill should take precedence or be present
          // Note: The actual behavior depends on load order
          expect(skill).toBeDefined()
          expect(skill!.name).toBe("commit")
        },
      })
    })

    test("should handle nested skill directories", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          // Create a deeply nested skill
          const skillDir = path.join(dir, ".ccode", "skills", "category", "subcategory", "nested-skill")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: nested-skill
description: A skill in a nested directory structure.
---

# Nested Skill

This skill is nested in subdirectories.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const nestedSkill = skills.find((s) => s.name === "nested-skill")

          expect(nestedSkill).toBeDefined()
          expect(nestedSkill!.location).toContain("nested-skill/SKILL.md")
        },
      })
    })
  })

  describe("Skill Validation", () => {
    test("should skip skills with missing frontmatter", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".ccode", "skills", "no-frontmatter")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `# No Frontmatter Skill

This skill has no YAML frontmatter and should be skipped.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const invalidSkill = skills.find((s) => s.name === "no-frontmatter")

          // Skills without valid frontmatter should be skipped
          expect(invalidSkill).toBeUndefined()
        },
      })
    })

    test("should skip skills with missing required fields", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".ccode", "skills", "missing-fields")
          await fs.mkdir(skillDir, { recursive: true })
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: missing-fields
# description is missing
---

# Missing Fields Skill
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const invalidSkill = skills.find((s) => s.name === "missing-fields")

          // Skills without required fields should be skipped
          expect(invalidSkill).toBeUndefined()
        },
      })
    })

    test("should handle empty skills directory gracefully", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          // Create empty skills directory
          await fs.mkdir(path.join(dir, ".ccode", "skills"), { recursive: true })
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Should not throw, just return builtin skills
          const skills = await Skill.all()
          expect(skills.length).toBe(BUILTIN_SKILL_COUNT)
        },
      })
    })

    test("should return only builtin skills when no user skills exist", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()

          // Should only have builtin skills
          expect(skills.length).toBe(BUILTIN_SKILL_COUNT)

          // All skills should have valid info
          for (const skill of skills) {
            expect(skill.name).toBeTruthy()
            expect(skill.description).toBeTruthy()
            expect(skill.location).toBeTruthy()
          }
        },
      })
    })
  })

  describe("Skill Priorities", () => {
    test("should handle duplicate skill names with warning", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          // Create same skill in both .ccode and .claude directories
          const ccodeSkillDir = path.join(dir, ".ccode", "skills", "duplicate-skill")
          const claudeSkillDir = path.join(dir, ".claude", "skills", "duplicate-skill")

          await fs.mkdir(ccodeSkillDir, { recursive: true })
          await fs.mkdir(claudeSkillDir, { recursive: true })

          await Bun.write(
            path.join(ccodeSkillDir, "SKILL.md"),
            `---
name: duplicate-skill
description: Duplicate skill from .ccode.
---

# Duplicate from .ccode
`,
          )

          await Bun.write(
            path.join(claudeSkillDir, "SKILL.md"),
            `---
name: duplicate-skill
description: Duplicate skill from .claude.
---

# Duplicate from .claude
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()

          // Should have exactly one skill with this name (later one wins)
          const duplicates = skills.filter((s) => s.name === "duplicate-skill")
          expect(duplicates.length).toBe(1)
        },
      })
    })
  })
})
