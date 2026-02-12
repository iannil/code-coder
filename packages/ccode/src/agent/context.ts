import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Relevance } from "@/context/relevance"
import { Fingerprint } from "@/context/fingerprint"
import { Loader } from "@/context/loader"
import { Cache } from "@/context/cache"
import { Preferences, Style, Knowledge, Vector, EditHistory, Decision } from "@/memory"
import path from "path"

const log = Log.create({ service: "agent.context" })

export interface AgentContext {
  projectFingerprint: string
  codeStyle: string
  learnedPatterns: string[]
  projectKnowledge: {
    apiEndpoints: number
    components: number
    dataModels: number
  }
  relevantFiles: Array<{
    path: string
    reason: string
    summary?: string
  }>
  recentEdits: Array<{
    file: string
    timestamp: number
  }>
  decisions: Array<{
    title: string
    type: string
  }>
}

export async function getAgentContext(task: string, filePaths?: string[], maxTokens = 8000): Promise<AgentContext> {
  const context: AgentContext = {
    projectFingerprint: "",
    codeStyle: "",
    learnedPatterns: [],
    projectKnowledge: { apiEndpoints: 0, components: 0, dataModels: 0 },
    relevantFiles: [],
    recentEdits: [],
    decisions: [],
  }

  try {
    const [fingerprint, preferences, knowledge, recentEditsList, recentDecisions] = await Promise.all([
      Fingerprint.get(),
      Preferences.get(),
      Knowledge.get(),
      EditHistory.getRecentRecords(10),
      Decision.getRecent(5),
    ])

    if (fingerprint) {
      context.projectFingerprint = Fingerprint.describe(fingerprint)
    }

    if (preferences) {
      const codeStyle = await Preferences.getCodeStyle()
      context.codeStyle = Preferences.describe(preferences)
      context.learnedPatterns = preferences.learnedPatterns.slice(0, 5).map((p) => p.pattern)
    }

    if (knowledge) {
      context.projectKnowledge = {
        apiEndpoints: knowledge.apiEndpoints.length,
        components: knowledge.components.length,
        dataModels: knowledge.dataModels.length,
      }
    }

    if (recentEditsList.length > 0) {
      const fileEditMap = new Map<string, number>()
      for (const edit of recentEditsList) {
        for (const fileEdit of edit.edits) {
          fileEditMap.set(fileEdit.path, edit.timestamp)
        }
      }
      context.recentEdits = Array.from(fileEditMap.entries())
        .map(([file, timestamp]) => ({ file, timestamp }))
        .slice(0, 5)
    }

    if (recentDecisions.length > 0) {
      context.decisions = recentDecisions.map((d) => ({
        title: d.title,
        type: d.type,
      }))
    }

    if (task) {
      const relevantContext = await Relevance.getRelevantContext({
        task,
        filePaths,
        maxTokens,
        includeTests: true,
        includeConfigs: true,
        includeDependencies: true,
      })

      context.relevantFiles = relevantContext.files.slice(0, 10).map((f) => ({
        path: f.path,
        reason: f.reason,
        summary: f.content?.slice(0, 200) + ((f.content?.length ?? 0) > 200 ? "..." : ""),
      }))
    }
  } catch (error) {
    log.warn("failed to load full agent context", { error })
  }

  return context
}

export async function getContextForFiles(filePaths: string[]): Promise<{
  context: AgentContext
  promptAddition: string
}> {
  const task = `Edit files: ${filePaths.map((p) => path.basename(p)).join(", ")}`
  const context = await getAgentContext(task, filePaths)

  const promptAddition = formatContextAsPrompt(context)

  return { context, promptAddition }
}

export async function getContextForTask(task: string): Promise<{
  context: AgentContext
  promptAddition: string
}> {
  const context = await getAgentContext(task)
  const promptAddition = formatContextAsPrompt(context)

  return { context, promptAddition }
}

export function formatContextAsPrompt(context: AgentContext): string {
  const parts: string[] = []

  parts.push("## Project Context")

  if (context.projectFingerprint) {
    parts.push(`\n**Project Type:** ${context.projectFingerprint}`)
  }

  if (context.codeStyle) {
    parts.push(`\n**Code Style:**\n${context.codeStyle}`)
  }

  if (context.learnedPatterns.length > 0) {
    parts.push(`\n**Learned Patterns:**\n${context.learnedPatterns.join(", ")}`)
  }

  if (context.projectKnowledge.apiEndpoints > 0 || context.projectKnowledge.components > 0) {
    parts.push(`\n**Project Knowledge:**`)
    parts.push(`- API Endpoints: ${context.projectKnowledge.apiEndpoints}`)
    parts.push(`- Components: ${context.projectKnowledge.components}`)
    parts.push(`- Data Models: ${context.projectKnowledge.dataModels}`)
  }

  if (context.relevantFiles.length > 0) {
    parts.push(`\n**Relevant Files:**`)
    for (const file of context.relevantFiles.slice(0, 10)) {
      parts.push(`- \`${file.path}\` (${file.reason})`)
      if (file.summary) {
        parts.push(`  ${file.summary}`)
      }
    }
  }

  if (context.recentEdits.length > 0) {
    parts.push(`\n**Recently Edited Files:**`)
    for (const edit of context.recentEdits) {
      const timeAgo = Math.round((Date.now() - edit.timestamp) / 60000)
      parts.push(`- \`${edit.file}\` (${timeAgo}m ago)`)
    }
  }

  if (context.decisions.length > 0) {
    parts.push(`\n**Recent Decisions:**`)
    for (const decision of context.decisions) {
      parts.push(`- ${decision.title} (${decision.type})`)
    }
  }

  return parts.join("\n")
}

export async function recordAgentEdit(
  sessionID: string,
  edits: Array<{
    path: string
    type: "create" | "update" | "delete"
    additions: number
    deletions: number
  }>,
  agent: string,
  model: string,
): Promise<void> {
  try {
    await EditHistory.createRecord({
      sessionID,
      description: `Agent ${agent} edits`,
      edits: edits.map((e) => ({
        path: e.path,
        type: e.type,
        additions: e.additions,
        deletions: e.deletions,
      })),
      agent,
      model,
    })

    for (const edit of edits) {
      if (edit.type === "update" || edit.type === "create") {
        try {
          const content = await Bun.file(path.join(Instance.worktree, edit.path)).text()
          await Style.recordEditChoice({
            type: "accept",
            fileType: path.extname(edit.path),
            finalCode: content,
          })
        } catch {}
      }
    }
  } catch (error) {
    log.warn("failed to record agent edit", { error })
  }
}

export async function recordAgentDecision(
  sessionID: string,
  title: string,
  type: Decision.DecisionRecord["type"],
  description: string,
  rationale?: string,
): Promise<void> {
  try {
    await Decision.create({
      sessionID,
      title,
      type,
      description,
      rationale,
    })
  } catch (error) {
    log.warn("failed to record agent decision", { error })
  }
}

export async function learnFromUserEdit(originalCode: string, userCode: string, filePath: string): Promise<void> {
  try {
    const fileType = path.extname(filePath)

    if (originalCode !== userCode) {
      await Style.recordEditChoice({
        type: "modify",
        fileType,
        originalSuggestion: originalCode,
        finalCode: userCode,
        reason: "User modified AI suggestion",
      })
    } else {
      await Style.recordEditChoice({
        type: "accept",
        fileType,
        finalCode: userCode,
        reason: "User accepted AI suggestion",
      })
    }
  } catch (error) {
    log.warn("failed to learn from user edit", { error })
  }
}

export async function getContextualPrompt(basePrompt: string, task: string, filePaths?: string[]): Promise<string> {
  const { promptAddition } = filePaths ? await getContextForFiles(filePaths) : await getContextForTask(task)

  return `${basePrompt}\n\n${promptAddition}`
}
