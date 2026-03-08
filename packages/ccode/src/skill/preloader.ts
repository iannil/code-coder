/**
 * Skill preloading based on context prediction.
 *
 * This module provides intelligent skill preloading by analyzing
 * the current context (file patterns, recent commands, etc.) to
 * predict which skills will likely be needed.
 *
 * ## Design Principle
 *
 * Prediction is an **uncertain** task that benefits from LLM reasoning.
 * The actual loading is handled by the Rust SkillLoader.
 */

/**
 * A prediction for which skill to preload.
 */
export interface PreloadPrediction {
  /** Skill identifier */
  skillId: string
  /** Confidence score (0-1) */
  confidence: number
  /** Why this skill was predicted */
  reason: string
}

/**
 * Context signals used for prediction.
 */
export interface ContextSignals {
  /** Current working directory patterns */
  directoryPatterns: string[]
  /** Recent file extensions being worked on */
  recentFileTypes: string[]
  /** Recent tool names or commands */
  recentCommands: string[]
  /** Agent mode (build, writer, decision) */
  agentMode?: string
  /** Explicit skill hints from user input */
  explicitHints: string[]
}

/**
 * Minimal context interface for skill prediction.
 * Can be created from various sources.
 */
export interface PreloadContext {
  /** Current working directory */
  cwd?: string
  /** Recent files that were edited/viewed */
  recentFiles?: string[]
  /** Recent tool calls or commands */
  recentToolCalls?: Array<{ name: string }>
  /** Agent mode */
  mode?: string
  /** Last user message */
  lastUserMessage?: string
}

/**
 * Configuration for the skill preloader.
 */
export interface PreloaderConfig {
  /** Maximum number of skills to preload */
  maxPreload: number
  /** Minimum confidence threshold for preloading */
  minConfidence: number
  /** Whether to use LLM for predictions (vs heuristics only) */
  useLlmPrediction: boolean
}

const DEFAULT_CONFIG: PreloaderConfig = {
  maxPreload: 5,
  minConfidence: 0.6,
  useLlmPrediction: false,
}

/**
 * Pattern-based skill associations.
 */
const SKILL_PATTERNS: Record<string, { patterns: RegExp[]; confidence: number }> = {
  'tdd-guide': {
    patterns: [/test\.(ts|js|tsx|jsx)$/, /__tests__/, /\.spec\./],
    confidence: 0.8,
  },
  'code-reviewer': {
    patterns: [/\.pull_request/, /review/, /\.diff$/],
    confidence: 0.7,
  },
  'security-reviewer': {
    patterns: [/auth/, /security/, /crypto/, /\.env/],
    confidence: 0.75,
  },
  'architect': {
    patterns: [/architecture/, /design/, /ARCHITECTURE\.md/],
    confidence: 0.7,
  },
  'macro': {
    patterns: [/economic/, /market/, /trading/, /finance/],
    confidence: 0.7,
  },
  'trader': {
    patterns: [/trading/, /order/, /position/, /portfolio/],
    confidence: 0.75,
  },
  'writer': {
    patterns: [/\.md$/, /blog/, /article/, /content/],
    confidence: 0.6,
  },
}

/**
 * Agent mode to skill mappings.
 */
const MODE_SKILLS: Record<string, string[]> = {
  build: ['tdd-guide', 'code-reviewer', 'architect', 'security-reviewer'],
  writer: ['proofreader', 'expander'],
  decision: ['macro', 'trader', 'picker', 'decision'],
}

/**
 * Extract context signals from a preload context.
 */
export function extractSignals(context: PreloadContext): ContextSignals {
  const signals: ContextSignals = {
    directoryPatterns: [],
    recentFileTypes: [],
    recentCommands: [],
    agentMode: context.mode,
    explicitHints: [],
  }

  // Extract from current directory
  if (context.cwd) {
    const parts = context.cwd.split('/')
    signals.directoryPatterns = parts.slice(-3) // Last 3 path segments
  }

  // Extract from recent files
  if (context.recentFiles) {
    const extensions = context.recentFiles
      .map((f) => f.split('.').pop() || '')
      .filter(Boolean)
    signals.recentFileTypes = Array.from(new Set(extensions))
  }

  // Extract from recent tool calls
  if (context.recentToolCalls) {
    signals.recentCommands = context.recentToolCalls.map((t) => t.name).slice(-10)
  }

  // Extract explicit hints from user message
  if (context.lastUserMessage) {
    const skillMentions = context.lastUserMessage.match(/@(\w+[-\w]*)/g)
    if (skillMentions) {
      signals.explicitHints = skillMentions.map((m) => m.slice(1))
    }
  }

  return signals
}

/**
 * Predict skills needed based on context signals using heuristics.
 */
export function predictFromHeuristics(signals: ContextSignals): PreloadPrediction[] {
  const predictions: PreloadPrediction[] = []

  // Add explicit hints with high confidence
  for (const hint of signals.explicitHints) {
    predictions.push({
      skillId: hint,
      confidence: 0.95,
      reason: 'Explicitly mentioned in user message',
    })
  }

  // Add mode-based predictions
  if (signals.agentMode && MODE_SKILLS[signals.agentMode]) {
    const modeSkills = MODE_SKILLS[signals.agentMode]
    for (const skillId of modeSkills.slice(0, 2)) {
      predictions.push({
        skillId,
        confidence: 0.65,
        reason: `Common skill for ${signals.agentMode} mode`,
      })
    }
  }

  // Add pattern-based predictions
  const allPatterns = [
    ...signals.directoryPatterns,
    ...signals.recentFileTypes,
    ...signals.recentCommands,
  ]

  for (const [skillId, config] of Object.entries(SKILL_PATTERNS)) {
    for (const pattern of config.patterns) {
      const match = allPatterns.some((p) => pattern.test(p))
      if (match) {
        predictions.push({
          skillId,
          confidence: config.confidence,
          reason: `Pattern match: ${pattern.source}`,
        })
        break // Only add once per skill
      }
    }
  }

  return predictions
}

/**
 * Deduplicate and sort predictions by confidence.
 */
function processPreductions(
  predictions: PreloadPrediction[],
  config: PreloaderConfig
): PreloadPrediction[] {
  // Deduplicate, keeping highest confidence for each skill
  const bySkill = new Map<string, PreloadPrediction>()
  for (const pred of predictions) {
    const existing = bySkill.get(pred.skillId)
    if (!existing || pred.confidence > existing.confidence) {
      bySkill.set(pred.skillId, pred)
    }
  }

  // Filter by minimum confidence and sort by confidence descending
  return Array.from(bySkill.values())
    .filter((p) => p.confidence >= config.minConfidence)
    .sort((a, b) => b.confidence - a.confidence)
    .slice(0, config.maxPreload)
}

/**
 * Skill preloader that predicts and warms up skills.
 */
export class SkillPreloader {
  private config: PreloaderConfig

  constructor(config: Partial<PreloaderConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Predict which skills are needed based on context.
   */
  async predictNeeded(context: PreloadContext): Promise<PreloadPrediction[]> {
    const signals = extractSignals(context)
    const predictions = predictFromHeuristics(signals)
    return processPreductions(predictions, this.config)
  }

  /**
   * Warm up skills by sending preload requests.
   *
   * This would typically call the Rust SkillLoader via IPC.
   */
  async warmup(skillIds: string[]): Promise<void> {
    // In a real implementation, this would call the Rust loader
    // via IPC: await ipcClient.send('skills/preload', { skillIds })
    console.log(`[SkillPreloader] Warming up skills: ${skillIds.join(', ')}`)
  }

  /**
   * Predict and preload skills in one call.
   */
  async predictAndPreload(context: PreloadContext): Promise<PreloadPrediction[]> {
    const predictions = await this.predictNeeded(context)
    const skillIds = predictions.map((p) => p.skillId)
    await this.warmup(skillIds)
    return predictions
  }
}

/**
 * Create a skill preloader with default configuration.
 */
export function createPreloader(config?: Partial<PreloaderConfig>): SkillPreloader {
  return new SkillPreloader(config)
}
