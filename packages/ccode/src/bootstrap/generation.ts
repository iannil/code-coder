import { Log } from "@/util/log"
import { Global } from "@/global"
import { Instance } from "@/project/instance"
import { Skill } from "@/skill/skill"
import { Agent } from "@/agent/agent"
import { Provider } from "@/provider/provider"
import { generateText } from "ai"
import path from "path"
import fs from "fs/promises"
import { BootstrapTypes } from "./types"
import { CandidateStore } from "./candidate-store"

const log = Log.create({ service: "bootstrap.generation" })

/**
 * SkillGeneration module handles extracting and generating skills
 * from successful problem-solving sessions.
 */
export namespace SkillGeneration {
  /**
   * Extract a skill candidate from a session's tool calls and solution
   */
  export async function extractCandidate(input: {
    sessionId: string
    toolCalls: BootstrapTypes.ToolCallRecord[]
    problem: string
    solution: string
    triggerType?: BootstrapTypes.TriggerType
  }): Promise<BootstrapTypes.SkillCandidate> {
    const { sessionId, toolCalls, problem, solution, triggerType = "auto" } = input

    // Analyze the tool calls to determine skill type
    const skillType = analyzeSkillType(toolCalls)

    // Generate a name and description using LLM
    const { name, description } = await generateNameAndDescription(problem, solution, toolCalls)

    // Extract content based on skill type
    const content = extractContent(skillType, toolCalls, solution)

    const candidate = CandidateStore.create({
      type: skillType,
      name,
      description,
      trigger: {
        type: triggerType,
        context: problem.slice(0, 500),
      },
      content,
      source: {
        sessionId,
        toolCalls: toolCalls.map((tc) => tc.id),
        problem,
        solution,
      },
    })

    log.info("extracted skill candidate", {
      id: candidate.id,
      name: candidate.name,
      type: candidate.type,
      toolCallCount: toolCalls.length,
    })

    return candidate
  }

  /**
   * Analyze tool calls to determine the type of skill
   */
  function analyzeSkillType(toolCalls: BootstrapTypes.ToolCallRecord[]): BootstrapTypes.SkillType {
    const toolNames = toolCalls.map((tc) => tc.tool)
    const uniqueTools = [...new Set(toolNames)]

    // If primarily bash commands, likely a workflow
    if (toolNames.filter((t) => t === "bash").length > toolNames.length / 2) {
      return "workflow"
    }

    // If involves task delegation, could be an agent pattern
    if (uniqueTools.includes("task")) {
      return "agent"
    }

    // If involves multiple different tools, likely a workflow
    if (uniqueTools.length > 3) {
      return "workflow"
    }

    // Default to pattern
    return "pattern"
  }

  /**
   * Generate a name and description for the skill using LLM
   */
  async function generateNameAndDescription(
    problem: string,
    solution: string,
    toolCalls: BootstrapTypes.ToolCallRecord[],
  ): Promise<{ name: string; description: string }> {
    try {
      const model = await Provider.defaultModel()
      const languageModel = await Provider.getLanguage(
        await Provider.getModel(model.providerID, model.modelID),
      )

      const toolSummary = toolCalls
        .slice(0, 10)
        .map((tc) => `- ${tc.tool}: ${JSON.stringify(tc.input).slice(0, 100)}`)
        .join("\n")

      const prompt = `Analyze this problem-solving session and generate a concise skill name and description.

Problem:
${problem.slice(0, 500)}

Solution:
${solution.slice(0, 500)}

Tools Used:
${toolSummary}

Generate a response in this exact format:
NAME: <kebab-case-skill-name>
DESCRIPTION: <one-line description of what this skill does>

Requirements:
- NAME must be lowercase kebab-case, 2-4 words, descriptive
- DESCRIPTION should be actionable and specific
- Focus on the reusable pattern, not the specific instance`

      const result = await generateText({
        model: languageModel,
        prompt,
        maxOutputTokens: 200,
        temperature: 0.3,
      })

      const lines = result.text.split("\n")
      const nameLine = lines.find((l) => l.startsWith("NAME:"))
      const descLine = lines.find((l) => l.startsWith("DESCRIPTION:"))

      const name = nameLine?.replace("NAME:", "").trim() ?? `skill-${Date.now()}`
      const description =
        descLine?.replace("DESCRIPTION:", "").trim() ?? "Auto-generated skill from session"

      return { name, description }
    } catch (error) {
      log.warn("failed to generate name/description, using defaults", { error })
      return {
        name: `skill-${Date.now()}`,
        description: "Auto-generated skill from session",
      }
    }
  }

  /**
   * Extract content from tool calls based on skill type
   */
  function extractContent(
    type: BootstrapTypes.SkillType,
    toolCalls: BootstrapTypes.ToolCallRecord[],
    solution: string,
  ): BootstrapTypes.SkillContent {
    switch (type) {
      case "workflow": {
        const steps = toolCalls.map((tc, i) => {
          const inputSummary = JSON.stringify(tc.input).slice(0, 200)
          return `${i + 1}. ${tc.tool}: ${inputSummary}`
        })
        return { steps }
      }

      case "tool": {
        const mainTool = toolCalls[0]
        return {
          toolDefinition: JSON.stringify(
            {
              name: mainTool?.tool,
              input: mainTool?.input,
            },
            null,
            2,
          ),
        }
      }

      case "agent": {
        return {
          agentPrompt: solution.slice(0, 2000),
        }
      }

      case "pattern":
      default: {
        // Extract code if present
        const codeBlocks = solution.match(/```[\s\S]*?```/g)
        const code = codeBlocks?.join("\n\n") ?? solution.slice(0, 1000)
        return { code }
      }
    }
  }

  /**
   * Generate a SKILL.md file from a candidate
   */
  export async function generateSkillMd(candidate: BootstrapTypes.SkillCandidate): Promise<string> {
    const lines = [
      "---",
      `name: ${candidate.name}`,
      `description: ${candidate.description}`,
      "---",
      "",
      `# ${candidate.name}`,
      "",
      candidate.description,
      "",
      "## When to Use",
      "",
      candidate.trigger.context,
      "",
      "## How It Works",
      "",
    ]

    switch (candidate.type) {
      case "workflow":
        lines.push("### Steps")
        lines.push("")
        for (const step of candidate.content.steps ?? []) {
          lines.push(`- ${step}`)
        }
        break

      case "pattern":
        lines.push("### Code Pattern")
        lines.push("")
        lines.push("```")
        lines.push(candidate.content.code ?? "")
        lines.push("```")
        break

      case "tool":
        lines.push("### Tool Definition")
        lines.push("")
        lines.push("```json")
        lines.push(candidate.content.toolDefinition ?? "{}")
        lines.push("```")
        break

      case "agent":
        lines.push("### Agent Prompt")
        lines.push("")
        lines.push(candidate.content.agentPrompt ?? "")
        break
    }

    lines.push("")
    lines.push("---")
    lines.push(`*Generated from session ${candidate.source.sessionId}*`)
    lines.push(`*Confidence: ${(candidate.verification.confidence * 100).toFixed(0)}%*`)

    return lines.join("\n")
  }

  /**
   * Persist a skill candidate as a SKILL.md file
   */
  export async function persist(candidate: BootstrapTypes.SkillCandidate): Promise<string> {
    const skillMd = await generateSkillMd(candidate)

    // Determine skill directory
    const skillDir = Instance.project.vcs
      ? path.join(Instance.worktree, ".codecoder", "skills", candidate.name)
      : path.join(Global.Path.config, "skills", candidate.name)

    await fs.mkdir(skillDir, { recursive: true })
    const skillPath = path.join(skillDir, "SKILL.md")

    await fs.writeFile(skillPath, skillMd, "utf-8")

    log.info("persisted skill", {
      name: candidate.name,
      path: skillPath,
    })

    return skillPath
  }

  /**
   * Check if a similar skill already exists
   */
  export async function isDuplicate(name: string): Promise<boolean> {
    const existingSkill = await Skill.get(name)
    return !!existingSkill
  }

  /**
   * Extract and store a candidate, checking for duplicates
   */
  export async function extractAndStore(input: {
    sessionId: string
    toolCalls: BootstrapTypes.ToolCallRecord[]
    problem: string
    solution: string
    triggerType?: BootstrapTypes.TriggerType
  }): Promise<BootstrapTypes.SkillCandidate | null> {
    const candidate = await extractCandidate(input)

    // Check for duplicate skill
    if (await isDuplicate(candidate.name)) {
      log.info("skipping duplicate skill", { name: candidate.name })
      return null
    }

    // Check for existing candidate
    const existing = await CandidateStore.getByName(candidate.name)
    if (existing) {
      log.info("updating existing candidate", { name: candidate.name })
      await CandidateStore.update(existing.id, (c) => {
        c.metadata.usageCount++
        c.source = candidate.source
      })
      return existing
    }

    await CandidateStore.add(candidate)
    return candidate
  }
}
