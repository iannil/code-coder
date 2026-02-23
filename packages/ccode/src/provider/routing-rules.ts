/**
 * LLM Routing Rules Configuration
 *
 * Defines task classification rules, role-based model permissions,
 * and routing strategies for intelligent LLM selection.
 *
 * Part of Phase 14: Intelligent LLM Router
 */

import { z } from "zod"

// ============================================================================
// Types & Schemas
// ============================================================================

/** Task types for classification */
export const TaskType = z.enum(["coding", "analysis", "chat", "sensitive"])
export type TaskType = z.infer<typeof TaskType>

/** User roles for RBAC */
export const UserRole = z.enum(["admin", "developer", "intern", "guest"])
export type UserRole = z.infer<typeof UserRole>

/** Model tier categories */
export const ModelTier = z.enum(["premium", "standard", "budget", "local"])
export type ModelTier = z.infer<typeof ModelTier>

/** Classification rule for detecting task types */
export const ClassificationRule = z.object({
  /** Unique rule identifier */
  id: z.string(),
  /** Task type this rule detects */
  taskType: TaskType,
  /** Priority (lower = higher priority) */
  priority: z.number().int().min(0).default(10),
  /** Regex patterns to match (any match triggers rule) */
  patterns: z.array(z.string()).default([]),
  /** Keywords to match (any match triggers rule) */
  keywords: z.array(z.string()).default([]),
  /** Agent names that trigger this task type */
  agents: z.array(z.string()).default([]),
  /** Whether rule is enabled */
  enabled: z.boolean().default(true),
})
export type ClassificationRule = z.infer<typeof ClassificationRule>

/** Model definition for routing */
export const RoutableModel = z.object({
  /** Model ID (e.g., "claude-3-opus", "gpt-4o-mini") */
  id: z.string(),
  /** Display name */
  name: z.string(),
  /** Provider (anthropic, openai, google, ollama) */
  provider: z.string(),
  /** Model tier for permission checks */
  tier: ModelTier,
  /** Task types this model is optimized for */
  optimizedFor: z.array(TaskType).default([]),
  /** Cost per 1M tokens (for cost optimization) */
  costPer1M: z.number().default(0),
  /** Whether model is available */
  available: z.boolean().default(true),
  /** Whether model runs locally (for sensitive data) */
  isLocal: z.boolean().default(false),
})
export type RoutableModel = z.infer<typeof RoutableModel>

/** Role permission configuration */
export const RolePermission = z.object({
  /** Role name */
  role: UserRole,
  /** Allowed model tiers */
  allowedTiers: z.array(ModelTier),
  /** Specific model IDs allowed (overrides tier restrictions) */
  allowedModels: z.array(z.string()).default([]),
  /** Specific model IDs denied (overrides tier allowances) */
  deniedModels: z.array(z.string()).default([]),
  /** Daily token limit */
  dailyTokenLimit: z.number().int().min(0).default(1_000_000),
  /** Monthly token limit */
  monthlyTokenLimit: z.number().int().min(0).default(10_000_000),
})
export type RolePermission = z.infer<typeof RolePermission>

/** Routing decision result */
export const RoutingDecision = z.object({
  /** Selected model ID */
  modelId: z.string(),
  /** Selected model display name */
  modelName: z.string(),
  /** Provider to use */
  provider: z.string(),
  /** Detected task type */
  taskType: TaskType,
  /** User's role */
  userRole: UserRole,
  /** Whether a fallback was used */
  isFallback: z.boolean().default(false),
  /** Reason for the decision */
  reason: z.string(),
  /** Warnings (e.g., quota warning) */
  warnings: z.array(z.string()).default([]),
})
export type RoutingDecision = z.infer<typeof RoutingDecision>

/** Routing configuration */
export const RoutingConfig = z.object({
  /** Whether routing is enabled */
  enabled: z.boolean().default(true),
  /** Default model for unclassified tasks */
  defaultModelId: z.string().default("claude-3-5-sonnet"),
  /** Default role for unknown users */
  defaultRole: UserRole.default("guest"),
  /** Classification rules */
  rules: z.array(ClassificationRule).default([]),
  /** Role permissions */
  rolePermissions: z.array(RolePermission).default([]),
  /** Available models */
  models: z.array(RoutableModel).default([]),
  /** Enable DLP integration for sensitive content detection */
  enableDlpIntegration: z.boolean().default(true),
  /** Force local model for DLP-detected sensitive content */
  forceLocalForSensitive: z.boolean().default(true),
})
export type RoutingConfig = z.infer<typeof RoutingConfig>

// ============================================================================
// Default Configuration
// ============================================================================

/** Default classification rules */
export const DEFAULT_CLASSIFICATION_RULES: ClassificationRule[] = [
  // Coding tasks - highest priority for code-related content
  {
    id: "rule-coding-codeblock",
    taskType: "coding",
    priority: 1,
    patterns: ["```[a-z]*\\n", "\\bfunction\\s+\\w+", "\\bclass\\s+\\w+", "\\bconst\\s+\\w+\\s*=", "\\bimport\\s+[{\\w]"],
    keywords: [],
    agents: ["@dev", "@code", "@tdd", "@architect", "@code-reverse"],
    enabled: true,
  },
  {
    id: "rule-coding-keywords",
    taskType: "coding",
    priority: 2,
    patterns: [],
    keywords: [
      "code",
      "implement",
      "function",
      "class",
      "debug",
      "fix bug",
      "refactor",
      "typescript",
      "javascript",
      "python",
      "rust",
      "golang",
      "api",
      "endpoint",
      "compile",
      "build",
      "test",
      "unittest",
    ],
    agents: [],
    enabled: true,
  },

  // Analysis tasks - deep reasoning required
  {
    id: "rule-analysis-agents",
    taskType: "analysis",
    priority: 1,
    patterns: [],
    keywords: [],
    agents: ["@macro", "@decision", "@trader", "@picker", "@observer", "@ai-engineer"],
    enabled: true,
  },
  {
    id: "rule-analysis-keywords",
    taskType: "analysis",
    priority: 2,
    patterns: ["\\banalyze\\b", "\\bevaluate\\b", "\\bassess\\b", "\\bcompare\\b.*\\bvs\\b"],
    keywords: [
      "analyze",
      "analysis",
      "evaluate",
      "assessment",
      "decision",
      "strategy",
      "trade-off",
      "pros and cons",
      "deep dive",
      "architecture",
      "design review",
      "CLOSE framework",
      "PMI",
      "macroeconomic",
    ],
    agents: [],
    enabled: true,
  },

  // Sensitive content - local model required
  {
    id: "rule-sensitive-dlp",
    taskType: "sensitive",
    priority: 0, // Highest priority
    patterns: [
      "\\b\\d{3}-\\d{2}-\\d{4}\\b", // SSN
      "\\b(?:\\d{4}[- ]?){3}\\d{4}\\b", // Credit card
      "AKIA[0-9A-Z]{16}", // AWS key
      "(sk|pk|api|token)[-_][a-zA-Z0-9]{20,}", // API keys
    ],
    keywords: ["password", "secret", "credential", "private key", "api key", "access token", "ssn", "social security"],
    agents: [],
    enabled: true,
  },

  // Chat - default for simple conversations
  {
    id: "rule-chat-default",
    taskType: "chat",
    priority: 100, // Lowest priority (fallback)
    patterns: [],
    keywords: ["hello", "hi", "thanks", "help", "what is", "how do", "explain", "tell me about"],
    agents: ["@general"],
    enabled: true,
  },
]

/** Default available models */
export const DEFAULT_MODELS: RoutableModel[] = [
  // Anthropic models
  {
    id: "claude-3-5-sonnet",
    name: "Claude 3.5 Sonnet",
    provider: "anthropic",
    tier: "standard",
    optimizedFor: ["coding"],
    costPer1M: 3,
    available: true,
    isLocal: false,
  },
  {
    id: "claude-3-opus",
    name: "Claude 3 Opus",
    provider: "anthropic",
    tier: "premium",
    optimizedFor: ["analysis", "coding"],
    costPer1M: 15,
    available: true,
    isLocal: false,
  },
  {
    id: "claude-3-haiku",
    name: "Claude 3 Haiku",
    provider: "anthropic",
    tier: "budget",
    optimizedFor: ["chat"],
    costPer1M: 0.25,
    available: true,
    isLocal: false,
  },

  // OpenAI models
  {
    id: "gpt-4o",
    name: "GPT-4o",
    provider: "openai",
    tier: "standard",
    optimizedFor: ["coding", "analysis"],
    costPer1M: 5,
    available: true,
    isLocal: false,
  },
  {
    id: "gpt-4o-mini",
    name: "GPT-4o Mini",
    provider: "openai",
    tier: "budget",
    optimizedFor: ["chat"],
    costPer1M: 0.15,
    available: true,
    isLocal: false,
  },
  {
    id: "o1",
    name: "OpenAI O1",
    provider: "openai",
    tier: "premium",
    optimizedFor: ["analysis"],
    costPer1M: 15,
    available: true,
    isLocal: false,
  },

  // Local models (Ollama)
  {
    id: "ollama-llama3",
    name: "Llama 3 (Local)",
    provider: "ollama",
    tier: "local",
    optimizedFor: ["chat", "sensitive"],
    costPer1M: 0,
    available: true,
    isLocal: true,
  },
  {
    id: "ollama-codellama",
    name: "Code Llama (Local)",
    provider: "ollama",
    tier: "local",
    optimizedFor: ["coding", "sensitive"],
    costPer1M: 0,
    available: true,
    isLocal: true,
  },
  {
    id: "ollama-deepseek-coder",
    name: "DeepSeek Coder (Local)",
    provider: "ollama",
    tier: "local",
    optimizedFor: ["coding", "sensitive"],
    costPer1M: 0,
    available: true,
    isLocal: true,
  },
]

/** Default role permissions (RBAC) */
export const DEFAULT_ROLE_PERMISSIONS: RolePermission[] = [
  {
    role: "admin",
    allowedTiers: ["premium", "standard", "budget", "local"],
    allowedModels: [],
    deniedModels: [],
    dailyTokenLimit: 100_000_000, // 100M tokens/day
    monthlyTokenLimit: 1_000_000_000, // 1B tokens/month
  },
  {
    role: "developer",
    allowedTiers: ["standard", "budget", "local"],
    allowedModels: [],
    deniedModels: ["o1"], // Expensive reasoning model
    dailyTokenLimit: 10_000_000, // 10M tokens/day
    monthlyTokenLimit: 100_000_000, // 100M tokens/month
  },
  {
    role: "intern",
    allowedTiers: ["budget", "local"],
    allowedModels: [],
    deniedModels: ["claude-3-opus", "o1", "gpt-4o"],
    dailyTokenLimit: 1_000_000, // 1M tokens/day
    monthlyTokenLimit: 10_000_000, // 10M tokens/month
  },
  {
    role: "guest",
    allowedTiers: ["local"],
    allowedModels: ["gpt-4o-mini"], // Allow one cheap cloud model
    deniedModels: [],
    dailyTokenLimit: 100_000, // 100K tokens/day
    monthlyTokenLimit: 1_000_000, // 1M tokens/month
  },
]

/** Default routing configuration */
export const DEFAULT_ROUTING_CONFIG: RoutingConfig = {
  enabled: true,
  defaultModelId: "claude-3-5-sonnet",
  defaultRole: "guest",
  rules: DEFAULT_CLASSIFICATION_RULES,
  rolePermissions: DEFAULT_ROLE_PERMISSIONS,
  models: DEFAULT_MODELS,
  enableDlpIntegration: true,
  forceLocalForSensitive: true,
}

// ============================================================================
// Task-to-Model Mapping
// ============================================================================

/** Recommended model per task type (in order of preference) */
export const TASK_MODEL_PREFERENCES: Record<TaskType, string[]> = {
  coding: ["claude-3-5-sonnet", "gpt-4o", "ollama-codellama", "ollama-deepseek-coder"],
  analysis: ["claude-3-opus", "o1", "gpt-4o", "claude-3-5-sonnet"],
  chat: ["gpt-4o-mini", "claude-3-haiku", "ollama-llama3"],
  sensitive: ["ollama-llama3", "ollama-codellama", "ollama-deepseek-coder"],
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Get model tier cost rank (lower = cheaper)
 */
export function getTierCostRank(tier: ModelTier): number {
  const ranks: Record<ModelTier, number> = {
    local: 0,
    budget: 1,
    standard: 2,
    premium: 3,
  }
  return ranks[tier]
}

/**
 * Check if a role can access a model tier
 */
export function canRoleAccessTier(role: UserRole, tier: ModelTier, permissions: RolePermission[]): boolean {
  const perm = permissions.find((p) => p.role === role)
  if (!perm) return false
  return perm.allowedTiers.includes(tier)
}

/**
 * Check if a role can access a specific model
 */
export function canRoleAccessModel(
  role: UserRole,
  modelId: string,
  model: RoutableModel,
  permissions: RolePermission[],
): boolean {
  const perm = permissions.find((p) => p.role === role)
  if (!perm) return false

  // Check explicit denials first
  if (perm.deniedModels.includes(modelId)) {
    return false
  }

  // Check explicit allowances
  if (perm.allowedModels.includes(modelId)) {
    return true
  }

  // Check tier-based access
  return perm.allowedTiers.includes(model.tier)
}

/**
 * Find the best available model for a task type and role
 */
export function findBestModel(
  taskType: TaskType,
  role: UserRole,
  models: RoutableModel[],
  permissions: RolePermission[],
): RoutableModel | undefined {
  const preferences = TASK_MODEL_PREFERENCES[taskType]

  // Try each preferred model in order
  for (const modelId of preferences) {
    const model = models.find((m) => m.id === modelId && m.available)
    if (model && canRoleAccessModel(role, modelId, model, permissions)) {
      return model
    }
  }

  // Fallback to any available model the role can access
  const availableModels = models
    .filter((m) => m.available && canRoleAccessModel(role, m.id, m, permissions))
    .sort((a, b) => getTierCostRank(a.tier) - getTierCostRank(b.tier))

  return availableModels[0]
}
