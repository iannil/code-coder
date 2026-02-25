import { z } from "zod"

export const AutonomyLevel = z.enum(["lunatic", "insane", "crazy", "wild", "bold", "timid"])
export type AutonomyLevel = z.infer<typeof AutonomyLevel>

export { type AutonomyLevel as AutonomyLevelType }

export const ResourceBudgetSchema = z.object({
  maxTokens: z.number().min(0).default(1_000_000),
  maxCostUSD: z.number().min(0).default(10.0),
  maxDurationMinutes: z.number().min(0).default(30),
  maxFilesChanged: z.number().min(0).default(50),
  maxActions: z.number().min(0).default(100),
})
export type ResourceBudget = z.infer<typeof ResourceBudgetSchema>

export const CloseWeightsSchema = z.object({
  convergence: z.number().default(1.0),
  leverage: z.number().default(1.2),
  optionality: z.number().default(1.5),
  surplus: z.number().default(1.3),
  evolution: z.number().default(0.8),
})
export type CloseWeights = z.infer<typeof CloseWeightsSchema>

export const CheckpointConfigSchema = z.object({
  enabled: z.boolean().default(true),
  interval: z.number().min(1).default(5),
  autoRollback: z.boolean().default(true),
})
export type CheckpointConfig = z.infer<typeof CheckpointConfigSchema>

export const LoopDetectionConfigSchema = z.object({
  enabled: z.boolean().default(true),
  threshold: z.number().min(2).default(3),
  autoBreak: z.boolean().default(true),
})
export type LoopDetectionConfig = z.infer<typeof LoopDetectionConfigSchema>

export const GithubScoutConfigSchema = z.object({
  /** Enable GitHub Scout feature */
  enabled: z.boolean().default(true),
  /** Integration mode: autonomous(default) | recommend | ask */
  integrationMode: z.enum(["autonomous", "recommend", "ask"]).default("autonomous"),
  /** Operations that require asking user confirmation */
  askForPermissions: z
    .array(z.enum(["global_install", "sudo", "system_config", "system_deps"]))
    .default(["global_install", "sudo", "system_config"]),
  /** Maximum number of dependencies to auto-install */
  maxAutoInstallDeps: z.number().min(1).max(50).default(10),
  /** Whether to allow packages with security warnings */
  allowSecurityWarnings: z.boolean().default(false),
  /** Minimum trigger confidence to activate search (0-1) */
  triggerThreshold: z.number().min(0).max(1).default(0.6),
  /** Minimum CLOSE decision score for searching */
  decisionThreshold: z.number().min(0).max(10).default(6.0),
  /** Maximum repositories to evaluate */
  maxReposToEvaluate: z.number().min(1).max(20).default(5),
  /** Enable caching of search results */
  enableCache: z.boolean().default(true),
  /** Cache TTL in milliseconds (default: 1 hour) */
  cacheTTLMs: z.number().default(3600000),
})
export type GithubScoutConfig = z.infer<typeof GithubScoutConfigSchema>

export const AutonomousModeConfigSchema = z.object({
  enabled: z.boolean().default(false),

  autonomyLevel: AutonomyLevel.default("crazy"),

  unattended: z.boolean().default(false),

  resourceLimits: ResourceBudgetSchema,

  riskTolerance: z.enum(["low", "medium", "high"]).default("medium"),
  decisionThreshold: z.number().min(0).max(10).default(7.0),
  closeWeights: CloseWeightsSchema,

  checkpoints: CheckpointConfigSchema,

  loopDetection: LoopDetectionConfigSchema,

  /** GitHub Scout configuration for open-source solution discovery */
  githubScout: GithubScoutConfigSchema.optional(),

  phaseTimeouts: z
    .object({
      understand: z.number().default(60000),
      plan: z.number().default(120000),
      decide: z.number().default(30000),
      execute: z.number().default(300000),
      test: z.number().default(120000),
      verify: z.number().default(120000),
      evaluate: z.number().default(30000),
    })
    .optional(),
})
export type AutonomousModeConfig = z.infer<typeof AutonomousModeConfigSchema>

export const SessionConfigSchema = z.object({
  sessionId: z.string(),
  requestId: z.string(),
  request: z.string(),
  autonomousMode: AutonomousModeConfigSchema.optional(),
})
export type SessionConfig = z.infer<typeof SessionConfigSchema>

export function validateAutonomousModeConfig(config: unknown): {
  success: boolean
  data?: AutonomousModeConfig
  errors?: string[]
} {
  const result = AutonomousModeConfigSchema.safeParse(config)

  if (result.success) {
    return {
      success: true,
      data: result.data,
    }
  }

  const zodError = result.error
  return {
    success: false,
    errors: zodError.issues.map((e) => `${e.path.join(".")}: ${e.message}`),
  }
}

export const DEFAULT_AUTONOMOUS_MODE_CONFIG: AutonomousModeConfig = {
  enabled: false,
  autonomyLevel: "crazy",
  unattended: false,
  resourceLimits: {
    maxTokens: 1_000_000,
    maxCostUSD: 10.0,
    maxDurationMinutes: 30,
    maxFilesChanged: 50,
    maxActions: 100,
  },
  riskTolerance: "medium",
  decisionThreshold: 7.0,
  closeWeights: {
    convergence: 1.0,
    leverage: 1.2,
    optionality: 1.5,
    surplus: 1.3,
    evolution: 0.8,
  },
  checkpoints: {
    enabled: true,
    interval: 5,
    autoRollback: true,
  },
  loopDetection: {
    enabled: true,
    threshold: 3,
    autoBreak: true,
  },
  githubScout: {
    enabled: true,
    integrationMode: "autonomous",
    askForPermissions: ["global_install", "sudo", "system_config"],
    maxAutoInstallDeps: 10,
    allowSecurityWarnings: false,
    triggerThreshold: 0.6,
    decisionThreshold: 6.0,
    maxReposToEvaluate: 5,
    enableCache: true,
    cacheTTLMs: 3600000,
  },
}

export function mergeAutonomousModeConfig(userConfig?: Partial<AutonomousModeConfig>): AutonomousModeConfig {
  if (!userConfig) {
    return DEFAULT_AUTONOMOUS_MODE_CONFIG
  }

  const defaultGithubScout = DEFAULT_AUTONOMOUS_MODE_CONFIG.githubScout!

  return {
    ...DEFAULT_AUTONOMOUS_MODE_CONFIG,
    ...userConfig,
    resourceLimits: {
      ...DEFAULT_AUTONOMOUS_MODE_CONFIG.resourceLimits,
      ...userConfig.resourceLimits,
    },
    closeWeights: {
      ...DEFAULT_AUTONOMOUS_MODE_CONFIG.closeWeights,
      ...userConfig.closeWeights,
    },
    checkpoints: {
      ...DEFAULT_AUTONOMOUS_MODE_CONFIG.checkpoints,
      ...userConfig.checkpoints,
    },
    loopDetection: {
      ...DEFAULT_AUTONOMOUS_MODE_CONFIG.loopDetection,
      ...userConfig.loopDetection,
    },
    githubScout: userConfig.githubScout
      ? {
          enabled: userConfig.githubScout.enabled ?? defaultGithubScout.enabled,
          integrationMode: userConfig.githubScout.integrationMode ?? defaultGithubScout.integrationMode,
          askForPermissions: userConfig.githubScout.askForPermissions ?? defaultGithubScout.askForPermissions,
          maxAutoInstallDeps: userConfig.githubScout.maxAutoInstallDeps ?? defaultGithubScout.maxAutoInstallDeps,
          allowSecurityWarnings: userConfig.githubScout.allowSecurityWarnings ?? defaultGithubScout.allowSecurityWarnings,
          triggerThreshold: userConfig.githubScout.triggerThreshold ?? defaultGithubScout.triggerThreshold,
          decisionThreshold: userConfig.githubScout.decisionThreshold ?? defaultGithubScout.decisionThreshold,
          maxReposToEvaluate: userConfig.githubScout.maxReposToEvaluate ?? defaultGithubScout.maxReposToEvaluate,
          enableCache: userConfig.githubScout.enableCache ?? defaultGithubScout.enableCache,
          cacheTTLMs: userConfig.githubScout.cacheTTLMs ?? defaultGithubScout.cacheTTLMs,
        }
      : defaultGithubScout,
  }
}
