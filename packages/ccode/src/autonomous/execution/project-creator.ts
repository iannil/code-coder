/**
 * Project Creator - Analyzes user intent and orchestrates project creation
 *
 * When users send messages like "帮我创建一个 React Todo App" via IM,
 * this component:
 * 1. Parses the intent (project type, name, technology stack)
 * 2. Searches for suitable templates via GithubScout
 * 3. Makes a CLOSE-based decision on creation strategy
 * 4. Orchestrates the actual project creation
 */

import { z } from "zod"
import { Log } from "@/util/log"
import { ProjectRegistry, type ProjectEntry } from "./project-scaffolder"
import { ProjectScaffolder, type ScaffoldResult, type ScaffoldOptions, type TemplateCloneOptions } from "./project-scaffolder"
import { GithubScout, type GithubRepo, type STAREvaluation } from "./github-scout"
import { createDecisionEngine, type DecisionResult, type DecisionContext } from "../decision/engine"
import { buildCriteria, type AutonomousDecisionCriteria } from "../decision/criteria"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"

const log = Log.create({ service: "autonomous.project-creator" })

// ============================================================================
// Types & Schemas
// ============================================================================

export interface ProjectCreationRequest {
  /** Session ID for tracking */
  sessionId: string
  /** User's original message */
  userMessage: string
  /** Source channel */
  channel: {
    type: "telegram" | "discord" | "slack" | "cli"
    chatId: string
  }
}

export interface ProjectCreationDecision {
  /** Recommended action */
  action: "clone_template" | "create_from_scratch" | "ask_user"
  /** Parsed project name */
  projectName: string
  /** URL-safe project slug */
  projectSlug: string
  /** Detected technology stack */
  technology: string[]
  /** Template recommendation (if action is clone_template) */
  template?: {
    repo: GithubRepo
    evaluation: STAREvaluation
  }
  /** CLOSE framework decision score */
  closeScore: number
  /** Human-readable reasoning */
  reasoning: string
  /** Confidence in the decision (0-1) */
  confidence: number
  /** Extracted description from user message */
  description?: string
}

export interface ProjectCreationResult {
  success: boolean
  decision: ProjectCreationDecision
  scaffoldResult?: ScaffoldResult
  project?: ProjectEntry
  error?: string
}

// ============================================================================
// Intent Detection Patterns
// ============================================================================

const PROJECT_CREATION_PATTERNS = {
  // Chinese patterns
  zh: [
    /帮我创建(?:一个)?(.+?)(?:项目|应用|App|程序)/i,
    /新建(?:一个)?(.+?)(?:项目|应用|App|程序)/i,
    /创建(?:一个)?(.+?)(?:项目|应用|App)/i,
    /搭建(?:一个)?(.+?)(?:项目|应用)/i,
    /开发(?:一个)?(.+?)(?:项目|应用)/i,
    /写(?:一个)?(.+?)(?:项目|应用)/i,
  ],
  // English patterns
  en: [
    /create (?:a |an )?(?:new )?(.+?) (?:project|app|application)/i,
    /build (?:a |an )?(?:new )?(.+?) (?:project|app|application)/i,
    /scaffold (?:a |an )?(?:new )?(.+?) (?:project|app)/i,
    /start (?:a |an )?(?:new )?(.+?) (?:project|app)/i,
    /make (?:a |an )?(?:new )?(.+?) (?:project|app)/i,
    /set up (?:a |an )?(?:new )?(.+?) (?:project|app)/i,
  ],
}

const TECHNOLOGY_KEYWORDS: Record<string, string[]> = {
  react: ["react", "reactjs", "react.js"],
  vue: ["vue", "vuejs", "vue.js"],
  angular: ["angular", "angularjs"],
  svelte: ["svelte", "sveltekit"],
  nextjs: ["nextjs", "next.js", "next"],
  nuxt: ["nuxt", "nuxtjs", "nuxt.js"],
  astro: ["astro"],
  typescript: ["typescript", "ts"],
  javascript: ["javascript", "js"],
  node: ["node", "nodejs", "node.js", "express", "koa", "fastify"],
  python: ["python", "py", "django", "flask", "fastapi"],
  rust: ["rust", "cargo"],
  go: ["go", "golang"],
  bun: ["bun"],
  deno: ["deno"],
  cli: ["cli", "command-line", "command line", "命令行"],
  api: ["api", "rest", "restful", "graphql"],
  fullstack: ["fullstack", "full-stack", "full stack", "全栈"],
  mobile: ["mobile", "react-native", "flutter", "移动端"],
  desktop: ["desktop", "electron", "tauri", "桌面"],
}

const PROJECT_TYPE_KEYWORDS: Record<string, string[]> = {
  todo: ["todo", "待办", "任务管理"],
  blog: ["blog", "博客"],
  ecommerce: ["ecommerce", "e-commerce", "shop", "store", "商城", "电商"],
  dashboard: ["dashboard", "admin", "管理后台", "控制台"],
  chat: ["chat", "聊天", "即时通讯"],
  portfolio: ["portfolio", "个人网站", "作品集"],
  landing: ["landing", "落地页"],
  saas: ["saas", "软件即服务"],
}

// ============================================================================
// ProjectCreator Class
// ============================================================================

export class ProjectCreator {
  private scaffolder: ProjectScaffolder
  private githubScout: GithubScout
  private decisionEngine = createDecisionEngine()

  constructor() {
    this.scaffolder = new ProjectScaffolder()
    this.githubScout = new GithubScout({
      integrationMode: "recommend", // Don't auto-install, just recommend
      triggerThreshold: 0.5,
    })
  }

  /**
   * Check if a message indicates project creation intent
   */
  isProjectCreationRequest(message: string): boolean {
    const allPatterns = [...PROJECT_CREATION_PATTERNS.zh, ...PROJECT_CREATION_PATTERNS.en]
    return allPatterns.some((pattern) => pattern.test(message))
  }

  /**
   * Analyze a project creation request
   */
  async analyze(request: ProjectCreationRequest): Promise<ProjectCreationDecision> {
    const { userMessage, sessionId } = request

    log.info("Analyzing project creation request", { sessionId, message: userMessage })

    // Step 1: Parse the user's intent
    const parsedIntent = this.parseIntent(userMessage)

    // Step 2: Generate unique slug
    const projectSlug = await ProjectRegistry.generateUniqueSlug(parsedIntent.name)

    // Step 3: Search for templates (optional)
    let templateRecommendation: { repo: GithubRepo; evaluation: STAREvaluation } | undefined

    if (parsedIntent.technology.length > 0) {
      const scoutResult = await this.githubScout.scout({
        sessionId,
        description: `${parsedIntent.name} ${parsedIntent.technology.join(" ")}`,
        technology: parsedIntent.technology[0],
      })

      if (scoutResult.triggered && scoutResult.topRecommendation) {
        const topRec = scoutResult.topRecommendation
        if (topRec.recommendation === "adopt" || topRec.recommendation === "trial") {
          templateRecommendation = {
            repo: topRec.repo,
            evaluation: topRec,
          }
        }
      }
    }

    // Step 4: Make CLOSE-based decision
    const closeDecision = await this.makeDecision(sessionId, parsedIntent, templateRecommendation)

    // Step 5: Determine action
    let action: ProjectCreationDecision["action"]
    let reasoning: string

    if (templateRecommendation && closeDecision.score.total >= 7.0) {
      action = "clone_template"
      reasoning = `Found high-quality template ${templateRecommendation.repo.fullName} (STAR score: ${templateRecommendation.evaluation.totalScore.toFixed(1)}). Template provides proven structure and best practices.`
    } else if (closeDecision.score.total >= 5.0) {
      action = "create_from_scratch"
      reasoning = `Creating from scratch is recommended. ${templateRecommendation ? `Available template scored ${templateRecommendation.evaluation.totalScore.toFixed(1)} which is below adoption threshold.` : "No suitable templates found."}`
    } else {
      action = "ask_user"
      reasoning = `Low confidence in automatic decision (score: ${closeDecision.score.total.toFixed(1)}). User guidance needed for project requirements.`
    }

    log.info("Project creation decision", {
      sessionId,
      action,
      name: parsedIntent.name,
      technology: parsedIntent.technology,
      closeScore: closeDecision.score.total,
    })

    return {
      action,
      projectName: parsedIntent.name,
      projectSlug,
      technology: parsedIntent.technology,
      template: templateRecommendation,
      closeScore: closeDecision.score.total,
      reasoning,
      confidence: parsedIntent.confidence,
      description: parsedIntent.description,
    }
  }

  /**
   * Create a project based on the decision
   */
  async create(decision: ProjectCreationDecision, channel: ProjectCreationRequest["channel"]): Promise<ProjectCreationResult> {
    log.info("Creating project", {
      action: decision.action,
      name: decision.projectName,
      slug: decision.projectSlug,
    })

    if (decision.action === "ask_user") {
      return {
        success: false,
        decision,
        error: "User guidance needed before proceeding",
      }
    }

    const baseOptions: ScaffoldOptions = {
      slug: decision.projectSlug,
      name: decision.projectName,
      description: decision.description,
      technology: decision.technology,
      sourceChannel: channel,
    }

    let scaffoldResult: ScaffoldResult

    if (decision.action === "clone_template" && decision.template) {
      const templateOptions: TemplateCloneOptions = {
        ...baseOptions,
        templateRepo: decision.template.repo.url,
      }
      scaffoldResult = await this.scaffolder.cloneTemplate(templateOptions)
    } else {
      scaffoldResult = await this.scaffolder.createEmpty(baseOptions)
    }

    // Publish creation event
    await Bus.publish(AutonomousEvent.ProjectCreated, {
      projectId: scaffoldResult.project?.id,
      name: decision.projectName,
      slug: decision.projectSlug,
      technology: decision.technology,
      action: decision.action,
      template: decision.template?.repo.fullName,
      success: scaffoldResult.success,
    })

    return {
      success: scaffoldResult.success,
      decision,
      scaffoldResult,
      project: scaffoldResult.project,
      error: scaffoldResult.error,
    }
  }

  /**
   * Full flow: analyze and create
   */
  async analyzeAndCreate(request: ProjectCreationRequest): Promise<ProjectCreationResult> {
    const decision = await this.analyze(request)

    if (decision.action === "ask_user") {
      return {
        success: false,
        decision,
        error: decision.reasoning,
      }
    }

    return this.create(decision, request.channel)
  }

  /**
   * Parse user message to extract project intent
   */
  private parseIntent(message: string): {
    name: string
    technology: string[]
    projectType: string | null
    confidence: number
    description?: string
  } {
    const normalizedMessage = message.toLowerCase()
    let name = ""
    let confidence = 0.5

    // Try to extract project name from patterns
    const allPatterns = [...PROJECT_CREATION_PATTERNS.zh, ...PROJECT_CREATION_PATTERNS.en]

    for (const pattern of allPatterns) {
      const match = message.match(pattern)
      if (match && match[1]) {
        name = match[1].trim()
        confidence = 0.8
        break
      }
    }

    // If no match, use a generic name
    if (!name) {
      name = "new-project"
      confidence = 0.3
    }

    // Detect technologies
    const technology: string[] = []
    for (const [tech, keywords] of Object.entries(TECHNOLOGY_KEYWORDS)) {
      if (keywords.some((kw) => normalizedMessage.includes(kw))) {
        technology.push(tech)
      }
    }

    // Detect project type
    let projectType: string | null = null
    for (const [type, keywords] of Object.entries(PROJECT_TYPE_KEYWORDS)) {
      if (keywords.some((kw) => normalizedMessage.includes(kw))) {
        projectType = type
        break
      }
    }

    // Extract description (everything after ":" or "，" or ",")
    let description: string | undefined
    const descMatch = message.match(/[：:，,]\s*(.+)$/)
    if (descMatch) {
      description = descMatch[1].trim()
    }

    // Boost confidence if technology or type detected
    if (technology.length > 0) confidence += 0.1
    if (projectType) confidence += 0.05
    confidence = Math.min(1.0, confidence)

    // Clean up the name
    const cleanedName = name
      .replace(new RegExp(technology.join("|"), "gi"), "")
      .replace(new RegExp(Object.keys(PROJECT_TYPE_KEYWORDS).join("|"), "gi"), "")
      .trim()
      .replace(/\s+/g, "-")

    // Generate a better name if cleanup left it empty
    const finalName = cleanedName || (projectType ? `${projectType}-app` : technology[0] ? `${technology[0]}-app` : "new-project")

    return {
      name: finalName,
      technology,
      projectType,
      confidence,
      description,
    }
  }

  /**
   * Make a CLOSE-based decision on project creation strategy
   */
  private async makeDecision(
    sessionId: string,
    parsedIntent: ReturnType<ProjectCreator["parseIntent"]>,
    templateRecommendation?: { repo: GithubRepo; evaluation: STAREvaluation },
  ): Promise<DecisionResult> {
    // Build CLOSE criteria based on whether template is available
    let criteria: AutonomousDecisionCriteria

    if (templateRecommendation) {
      // Template available - evaluate template usage
      criteria = buildCriteria({
        type: "resource_acquisition",
        description: `Use template ${templateRecommendation.repo.fullName} for ${parsedIntent.name}`,
        riskLevel: "low",
        // Template somewhat fixes structure (moderate convergence)
        convergence: 4,
        // High leverage - template saves significant effort
        leverage: 8,
        // Can always restructure later
        optionality: 7,
        // One-time setup cost
        surplus: 7,
        // Learn from template best practices
        evolution: 8,
      })
    } else {
      // No template - evaluate from-scratch creation
      criteria = buildCriteria({
        type: "implementation",
        description: `Create ${parsedIntent.name} from scratch`,
        riskLevel: "low",
        // Full control over structure
        convergence: 3,
        // Less leverage without template
        leverage: 5,
        // Full flexibility
        optionality: 9,
        // More effort required
        surplus: 5,
        // Learn by building
        evolution: 6,
      })
    }

    const context: DecisionContext = {
      sessionId,
      currentState: "planning",
      errorCount: 0,
      recentDecisions: [],
    }

    return this.decisionEngine.evaluate(criteria, context)
  }

  /**
   * Get suggested technology from project description
   */
  suggestTechnology(description: string): string[] {
    const suggestions: string[] = []
    const normalized = description.toLowerCase()

    // Check for framework mentions
    if (normalized.includes("react") || normalized.includes("frontend")) {
      suggestions.push("react", "typescript")
    }
    if (normalized.includes("api") || normalized.includes("backend")) {
      suggestions.push("node", "typescript")
    }
    if (normalized.includes("full") && normalized.includes("stack")) {
      suggestions.push("nextjs", "typescript")
    }
    if (normalized.includes("cli") || normalized.includes("command")) {
      suggestions.push("typescript", "node")
    }

    // Default to TypeScript if nothing detected
    if (suggestions.length === 0) {
      suggestions.push("typescript")
    }

    return [...new Set(suggestions)]
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a new ProjectCreator instance
 */
export function createProjectCreator(): ProjectCreator {
  return new ProjectCreator()
}

/**
 * Check if a message is a project creation request
 */
export function isProjectCreationRequest(message: string): boolean {
  const creator = createProjectCreator()
  return creator.isProjectCreationRequest(message)
}

/**
 * Analyze and create a project from an IM message
 */
export async function createProjectFromMessage(request: ProjectCreationRequest): Promise<ProjectCreationResult> {
  const creator = createProjectCreator()
  return creator.analyzeAndCreate(request)
}
