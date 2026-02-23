/**
 * LLM Router Tests
 *
 * Tests for the Intelligent LLM Router module that routes requests
 * to optimal models based on task classification and RBAC.
 *
 * Part of Phase 14: Intelligent LLM Router
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import {
  TaskType,
  UserRole,
  ModelTier,
  ClassificationRule,
  RoutableModel,
  RolePermission,
  RoutingConfig,
  DEFAULT_CLASSIFICATION_RULES,
  DEFAULT_MODELS,
  DEFAULT_ROLE_PERMISSIONS,
  DEFAULT_ROUTING_CONFIG,
  getTierCostRank,
  canRoleAccessTier,
  canRoleAccessModel,
  findBestModel,
  TASK_MODEL_PREFERENCES,
} from "../../../src/provider/routing-rules"
import { classifyTask } from "../../../src/api/server/handlers/llm-router"

// ============================================================================
// Schema Validation Tests
// ============================================================================

describe("Routing Rules Schemas", () => {
  describe("TaskType schema", () => {
    test("should validate all task types", () => {
      const validTypes = ["coding", "analysis", "chat", "sensitive"]
      for (const type of validTypes) {
        const result = TaskType.safeParse(type)
        expect(result.success).toBe(true)
      }
    })

    test("should reject invalid task type", () => {
      const result = TaskType.safeParse("invalid")
      expect(result.success).toBe(false)
    })
  })

  describe("UserRole schema", () => {
    test("should validate all user roles", () => {
      const validRoles = ["admin", "developer", "intern", "guest"]
      for (const role of validRoles) {
        const result = UserRole.safeParse(role)
        expect(result.success).toBe(true)
      }
    })

    test("should reject invalid user role", () => {
      const result = UserRole.safeParse("superuser")
      expect(result.success).toBe(false)
    })
  })

  describe("ModelTier schema", () => {
    test("should validate all model tiers", () => {
      const validTiers = ["premium", "standard", "budget", "local"]
      for (const tier of validTiers) {
        const result = ModelTier.safeParse(tier)
        expect(result.success).toBe(true)
      }
    })
  })

  describe("ClassificationRule schema", () => {
    test("should validate valid classification rule", () => {
      const rule = {
        id: "test-rule",
        taskType: "coding",
        priority: 5,
        patterns: ["\\bfunction\\b"],
        keywords: ["code", "implement"],
        agents: ["@dev"],
        enabled: true,
      }

      const result = ClassificationRule.safeParse(rule)
      expect(result.success).toBe(true)
    })

    test("should apply defaults for optional fields", () => {
      const rule = {
        id: "minimal-rule",
        taskType: "chat",
      }

      const result = ClassificationRule.safeParse(rule)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.priority).toBe(10)
        expect(result.data.patterns).toEqual([])
        expect(result.data.keywords).toEqual([])
        expect(result.data.enabled).toBe(true)
      }
    })
  })

  describe("RoutableModel schema", () => {
    test("should validate valid routable model", () => {
      const model = {
        id: "test-model",
        name: "Test Model",
        provider: "test-provider",
        tier: "standard",
        optimizedFor: ["coding", "chat"],
        costPer1M: 5.0,
        available: true,
        isLocal: false,
      }

      const result = RoutableModel.safeParse(model)
      expect(result.success).toBe(true)
    })

    test("should validate local model", () => {
      const model = {
        id: "local-llama",
        name: "Llama Local",
        provider: "ollama",
        tier: "local",
        optimizedFor: ["sensitive"],
        costPer1M: 0,
        available: true,
        isLocal: true,
      }

      const result = RoutableModel.safeParse(model)
      expect(result.success).toBe(true)
    })
  })

  describe("RolePermission schema", () => {
    test("should validate valid role permission", () => {
      const perm = {
        role: "developer",
        allowedTiers: ["standard", "budget"],
        allowedModels: [],
        deniedModels: ["o1"],
        dailyTokenLimit: 10_000_000,
        monthlyTokenLimit: 100_000_000,
      }

      const result = RolePermission.safeParse(perm)
      expect(result.success).toBe(true)
    })
  })
})

// ============================================================================
// Default Configuration Tests
// ============================================================================

describe("Default Configuration", () => {
  test("should have valid default classification rules", () => {
    expect(DEFAULT_CLASSIFICATION_RULES.length).toBeGreaterThan(0)

    for (const rule of DEFAULT_CLASSIFICATION_RULES) {
      const result = ClassificationRule.safeParse(rule)
      expect(result.success).toBe(true)
    }
  })

  test("should have coding rules with lower priority than chat", () => {
    const codingRules = DEFAULT_CLASSIFICATION_RULES.filter((r) => r.taskType === "coding")
    const chatRules = DEFAULT_CLASSIFICATION_RULES.filter((r) => r.taskType === "chat")

    const minCodingPriority = Math.min(...codingRules.map((r) => r.priority))
    const minChatPriority = Math.min(...chatRules.map((r) => r.priority))

    expect(minCodingPriority).toBeLessThan(minChatPriority)
  })

  test("should have sensitive rules with highest priority", () => {
    const sensitiveRules = DEFAULT_CLASSIFICATION_RULES.filter((r) => r.taskType === "sensitive")
    const otherRules = DEFAULT_CLASSIFICATION_RULES.filter((r) => r.taskType !== "sensitive")

    const maxSensitivePriority = Math.max(...sensitiveRules.map((r) => r.priority))
    const minOtherPriority = Math.min(...otherRules.map((r) => r.priority))

    expect(maxSensitivePriority).toBeLessThanOrEqual(minOtherPriority)
  })

  test("should have valid default models", () => {
    expect(DEFAULT_MODELS.length).toBeGreaterThan(0)

    for (const model of DEFAULT_MODELS) {
      const result = RoutableModel.safeParse(model)
      expect(result.success).toBe(true)
    }
  })

  test("should have models from multiple providers", () => {
    const providers = new Set(DEFAULT_MODELS.map((m) => m.provider))
    expect(providers.size).toBeGreaterThanOrEqual(2)
    expect(providers.has("anthropic")).toBe(true)
    expect(providers.has("openai")).toBe(true)
  })

  test("should have local models available", () => {
    const localModels = DEFAULT_MODELS.filter((m) => m.isLocal)
    expect(localModels.length).toBeGreaterThan(0)
  })

  test("should have valid default role permissions", () => {
    expect(DEFAULT_ROLE_PERMISSIONS.length).toBe(4) // admin, developer, intern, guest

    for (const perm of DEFAULT_ROLE_PERMISSIONS) {
      const result = RolePermission.safeParse(perm)
      expect(result.success).toBe(true)
    }
  })

  test("should have admin with access to all tiers", () => {
    const adminPerm = DEFAULT_ROLE_PERMISSIONS.find((p) => p.role === "admin")
    expect(adminPerm).toBeDefined()
    expect(adminPerm!.allowedTiers).toContain("premium")
    expect(adminPerm!.allowedTiers).toContain("standard")
    expect(adminPerm!.allowedTiers).toContain("budget")
    expect(adminPerm!.allowedTiers).toContain("local")
  })

  test("should have intern restricted from premium tier", () => {
    const internPerm = DEFAULT_ROLE_PERMISSIONS.find((p) => p.role === "intern")
    expect(internPerm).toBeDefined()
    expect(internPerm!.allowedTiers).not.toContain("premium")
  })

  test("should have guest restricted to local tier", () => {
    const guestPerm = DEFAULT_ROLE_PERMISSIONS.find((p) => p.role === "guest")
    expect(guestPerm).toBeDefined()
    expect(guestPerm!.allowedTiers).toEqual(["local"])
  })

  test("should have valid default routing config", () => {
    const result = RoutingConfig.safeParse(DEFAULT_ROUTING_CONFIG)
    expect(result.success).toBe(true)
  })
})

// ============================================================================
// Helper Function Tests
// ============================================================================

describe("Helper Functions", () => {
  describe("getTierCostRank", () => {
    test("should return correct cost ranks", () => {
      expect(getTierCostRank("local")).toBe(0)
      expect(getTierCostRank("budget")).toBe(1)
      expect(getTierCostRank("standard")).toBe(2)
      expect(getTierCostRank("premium")).toBe(3)
    })

    test("should rank local as cheapest", () => {
      expect(getTierCostRank("local")).toBeLessThan(getTierCostRank("budget"))
    })

    test("should rank premium as most expensive", () => {
      expect(getTierCostRank("premium")).toBeGreaterThan(getTierCostRank("standard"))
    })
  })

  describe("canRoleAccessTier", () => {
    test("admin can access all tiers", () => {
      expect(canRoleAccessTier("admin", "premium", DEFAULT_ROLE_PERMISSIONS)).toBe(true)
      expect(canRoleAccessTier("admin", "standard", DEFAULT_ROLE_PERMISSIONS)).toBe(true)
      expect(canRoleAccessTier("admin", "budget", DEFAULT_ROLE_PERMISSIONS)).toBe(true)
      expect(canRoleAccessTier("admin", "local", DEFAULT_ROLE_PERMISSIONS)).toBe(true)
    })

    test("developer cannot access premium tier", () => {
      expect(canRoleAccessTier("developer", "premium", DEFAULT_ROLE_PERMISSIONS)).toBe(false)
      expect(canRoleAccessTier("developer", "standard", DEFAULT_ROLE_PERMISSIONS)).toBe(true)
    })

    test("intern can only access budget and local tiers", () => {
      expect(canRoleAccessTier("intern", "premium", DEFAULT_ROLE_PERMISSIONS)).toBe(false)
      expect(canRoleAccessTier("intern", "standard", DEFAULT_ROLE_PERMISSIONS)).toBe(false)
      expect(canRoleAccessTier("intern", "budget", DEFAULT_ROLE_PERMISSIONS)).toBe(true)
      expect(canRoleAccessTier("intern", "local", DEFAULT_ROLE_PERMISSIONS)).toBe(true)
    })

    test("guest can only access local tier", () => {
      expect(canRoleAccessTier("guest", "premium", DEFAULT_ROLE_PERMISSIONS)).toBe(false)
      expect(canRoleAccessTier("guest", "standard", DEFAULT_ROLE_PERMISSIONS)).toBe(false)
      expect(canRoleAccessTier("guest", "budget", DEFAULT_ROLE_PERMISSIONS)).toBe(false)
      expect(canRoleAccessTier("guest", "local", DEFAULT_ROLE_PERMISSIONS)).toBe(true)
    })

    test("returns false for unknown role", () => {
      expect(canRoleAccessTier("unknown" as UserRole, "standard", DEFAULT_ROLE_PERMISSIONS)).toBe(false)
    })
  })

  describe("canRoleAccessModel", () => {
    const testModel: RoutableModel = {
      id: "test-standard",
      name: "Test Standard",
      provider: "test",
      tier: "standard",
      optimizedFor: ["coding"],
      costPer1M: 5,
      available: true,
      isLocal: false,
    }

    test("admin can access any model", () => {
      expect(canRoleAccessModel("admin", "test-standard", testModel, DEFAULT_ROLE_PERMISSIONS)).toBe(true)
    })

    test("developer can access standard tier models", () => {
      expect(canRoleAccessModel("developer", "test-standard", testModel, DEFAULT_ROLE_PERMISSIONS)).toBe(true)
    })

    test("intern cannot access standard tier models", () => {
      expect(canRoleAccessModel("intern", "test-standard", testModel, DEFAULT_ROLE_PERMISSIONS)).toBe(false)
    })

    test("explicit deny overrides tier allowance", () => {
      const o1Model: RoutableModel = {
        id: "o1",
        name: "OpenAI O1",
        provider: "openai",
        tier: "premium",
        optimizedFor: ["analysis"],
        costPer1M: 15,
        available: true,
        isLocal: false,
      }

      // Developer has o1 in deniedModels
      expect(canRoleAccessModel("developer", "o1", o1Model, DEFAULT_ROLE_PERMISSIONS)).toBe(false)
    })

    test("explicit allow overrides tier restriction", () => {
      // Guest has gpt-4o-mini in allowedModels
      const miniModel: RoutableModel = {
        id: "gpt-4o-mini",
        name: "GPT-4o Mini",
        provider: "openai",
        tier: "budget",
        optimizedFor: ["chat"],
        costPer1M: 0.15,
        available: true,
        isLocal: false,
      }

      expect(canRoleAccessModel("guest", "gpt-4o-mini", miniModel, DEFAULT_ROLE_PERMISSIONS)).toBe(true)
    })
  })

  describe("findBestModel", () => {
    test("should find coding model for coding task", () => {
      const model = findBestModel("coding", "developer", DEFAULT_MODELS, DEFAULT_ROLE_PERMISSIONS)
      expect(model).toBeDefined()
      expect(model!.id).toBe("claude-3-5-sonnet")
    })

    test("should find analysis model for analysis task", () => {
      const model = findBestModel("analysis", "admin", DEFAULT_MODELS, DEFAULT_ROLE_PERMISSIONS)
      expect(model).toBeDefined()
      // Admin can access opus
      expect(model!.id).toBe("claude-3-opus")
    })

    test("should find chat model for chat task", () => {
      const model = findBestModel("chat", "developer", DEFAULT_MODELS, DEFAULT_ROLE_PERMISSIONS)
      expect(model).toBeDefined()
      expect(["gpt-4o-mini", "claude-3-haiku"]).toContain(model!.id)
    })

    test("should find local model for sensitive task", () => {
      const model = findBestModel("sensitive", "developer", DEFAULT_MODELS, DEFAULT_ROLE_PERMISSIONS)
      expect(model).toBeDefined()
      expect(model!.isLocal).toBe(true)
    })

    test("should respect role permissions when finding model", () => {
      const internModel = findBestModel("coding", "intern", DEFAULT_MODELS, DEFAULT_ROLE_PERMISSIONS)
      expect(internModel).toBeDefined()
      // Intern can only use budget and local tiers
      expect(["budget", "local"]).toContain(internModel!.tier)
    })

    test("should fallback to any available model for restricted role", () => {
      const guestModel = findBestModel("coding", "guest", DEFAULT_MODELS, DEFAULT_ROLE_PERMISSIONS)
      expect(guestModel).toBeDefined()
      // Guest should get local model or explicitly allowed model
      expect(guestModel!.tier === "local" || guestModel!.id === "gpt-4o-mini").toBe(true)
    })
  })
})

// ============================================================================
// Task Classification Tests
// ============================================================================

describe("Task Classification", () => {
  describe("classifyTask", () => {
    test("should classify code blocks as coding", () => {
      const content = `Here's how to implement it:
\`\`\`typescript
function hello() {
  console.log("Hello World");
}
\`\`\``

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("coding")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("should classify function definitions as coding", () => {
      const content = "I need to implement a function calculateTotal that sums an array"

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("coding")
    })

    test("should classify @dev agent as coding", () => {
      const content = "Help me build this feature"

      const result = classifyTask(content, "@dev", DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("coding")
      expect(result.confidence).toBe(1.0)
    })

    test("should classify @macro agent as analysis", () => {
      const content = "What are your thoughts?"

      const result = classifyTask(content, "@macro", DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("analysis")
      expect(result.confidence).toBe(1.0)
    })

    test("should classify @decision agent as analysis", () => {
      const content = "Help me decide"

      const result = classifyTask(content, "@decision", DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("analysis")
    })

    test("should classify analysis keywords as analysis", () => {
      const content = "Please analyze the trade-offs between these two approaches and evaluate which is better"

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("analysis")
    })

    test("should classify SSN patterns as sensitive", () => {
      const content = "My SSN is 123-45-6789"

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("sensitive")
    })

    test("should classify credit card patterns as sensitive", () => {
      const content = "My card number is 4111 1111 1111 1111"

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("sensitive")
    })

    test("should classify AWS keys as sensitive", () => {
      const content = "Use this key: AKIAIOSFODNN7EXAMPLE"

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("sensitive")
    })

    test("should classify API key patterns as sensitive", () => {
      const content = "Here's my API key: sk-abcdefghijklmnopqrstuvwxyz1234567890"

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("sensitive")
    })

    test("should classify greetings as chat", () => {
      const content = "Hello! How are you?"

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("chat")
    })

    test("should classify simple questions as chat", () => {
      const content = "What is the weather like today?"

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("chat")
    })

    test("should default to chat for unclassifiable content", () => {
      const content = "abcdefg12345"

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      expect(result.taskType).toBe("chat")
      expect(result.matchedRule).toBe("default")
    })

    test("should prioritize sensitive over other classifications", () => {
      // Content that matches both coding AND sensitive patterns
      const content = `Here's the code:
\`\`\`
const apiKey = "sk-abcdefghijklmnopqrstuvwxyz123"
\`\`\``

      const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
      // Sensitive has priority 0, coding has priority 1
      expect(result.taskType).toBe("sensitive")
    })

    test("should handle agent name with @ prefix", () => {
      const result1 = classifyTask("test", "@dev", DEFAULT_CLASSIFICATION_RULES)
      const result2 = classifyTask("test", "dev", DEFAULT_CLASSIFICATION_RULES)

      expect(result1.taskType).toBe(result2.taskType)
    })

    test("should be case-insensitive for keywords", () => {
      const result1 = classifyTask("Please ANALYZE this", undefined, DEFAULT_CLASSIFICATION_RULES)
      const result2 = classifyTask("please analyze this", undefined, DEFAULT_CLASSIFICATION_RULES)

      expect(result1.taskType).toBe(result2.taskType)
    })
  })
})

// ============================================================================
// Task-Model Preference Tests
// ============================================================================

describe("Task-Model Preferences", () => {
  test("should have preferences for all task types", () => {
    const taskTypes: TaskType[] = ["coding", "analysis", "chat", "sensitive"]

    for (const taskType of taskTypes) {
      expect(TASK_MODEL_PREFERENCES[taskType]).toBeDefined()
      expect(TASK_MODEL_PREFERENCES[taskType].length).toBeGreaterThan(0)
    }
  })

  test("coding task should prefer Claude Sonnet first", () => {
    expect(TASK_MODEL_PREFERENCES.coding[0]).toBe("claude-3-5-sonnet")
  })

  test("analysis task should prefer Opus first", () => {
    expect(TASK_MODEL_PREFERENCES.analysis[0]).toBe("claude-3-opus")
  })

  test("chat task should prefer budget models", () => {
    const chatPrefs = TASK_MODEL_PREFERENCES.chat
    // Budget models should be preferred for chat
    expect(["gpt-4o-mini", "claude-3-haiku"]).toContain(chatPrefs[0])
  })

  test("sensitive task should prefer local models", () => {
    const sensitivePrefs = TASK_MODEL_PREFERENCES.sensitive
    // All preferences should be local models
    expect(sensitivePrefs.every((id) => id.startsWith("ollama-"))).toBe(true)
  })
})

// ============================================================================
// Edge Cases Tests
// ============================================================================

describe("Edge Cases", () => {
  test("should handle empty content", () => {
    const result = classifyTask("", undefined, DEFAULT_CLASSIFICATION_RULES)
    expect(result.taskType).toBe("chat")
  })

  test("should handle very long content", () => {
    const content = "Please implement a function ".repeat(1000)
    const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
    expect(result.taskType).toBe("coding")
  })

  test("should handle content with special characters", () => {
    const content = "Help me with this regex: /^[a-z]+$/gi"
    const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
    expect(result).toBeDefined()
    // Should not crash on special characters
  })

  test("should handle content with unicode", () => {
    const content = "帮我写一个函数 function 来处理数据"
    const result = classifyTask(content, undefined, DEFAULT_CLASSIFICATION_RULES)
    expect(result).toBeDefined()
  })

  test("should handle empty rules array", () => {
    const result = classifyTask("test content", undefined, [])
    expect(result.taskType).toBe("chat")
    expect(result.matchedRule).toBe("default")
  })

  test("should handle invalid regex in rules gracefully", () => {
    const badRules: ClassificationRule[] = [
      {
        id: "bad-rule",
        taskType: "coding",
        priority: 1,
        patterns: ["[invalid(regex"], // Invalid regex
        keywords: [],
        agents: [],
        enabled: true,
      },
    ]

    // Should not throw, just skip the bad pattern
    const result = classifyTask("test content", undefined, badRules)
    expect(result).toBeDefined()
  })

  test("should handle disabled rules", () => {
    const disabledRules: ClassificationRule[] = [
      {
        id: "disabled-rule",
        taskType: "coding",
        priority: 1,
        patterns: [],
        keywords: ["test"],
        agents: [],
        enabled: false,
      },
    ]

    const result = classifyTask("test content", undefined, disabledRules)
    // Should fall through to default since rule is disabled
    expect(result.matchedRule).toBe("default")
  })
})

// ============================================================================
// RBAC Integration Tests
// ============================================================================

describe("RBAC Integration", () => {
  test("should prevent intern from using expensive models", () => {
    const expensiveModels = DEFAULT_MODELS.filter((m) => m.tier === "premium")

    for (const model of expensiveModels) {
      expect(canRoleAccessModel("intern", model.id, model, DEFAULT_ROLE_PERMISSIONS)).toBe(false)
    }
  })

  test("should prevent developer from using O1 model", () => {
    const o1Model = DEFAULT_MODELS.find((m) => m.id === "o1")
    expect(o1Model).toBeDefined()
    expect(canRoleAccessModel("developer", "o1", o1Model!, DEFAULT_ROLE_PERMISSIONS)).toBe(false)
  })

  test("should allow all roles to use local models", () => {
    const localModels = DEFAULT_MODELS.filter((m) => m.isLocal)
    const roles: UserRole[] = ["admin", "developer", "intern", "guest"]

    for (const model of localModels) {
      for (const role of roles) {
        expect(canRoleAccessModel(role, model.id, model, DEFAULT_ROLE_PERMISSIONS)).toBe(true)
      }
    }
  })

  test("token limits should decrease with role privilege", () => {
    const adminPerm = DEFAULT_ROLE_PERMISSIONS.find((p) => p.role === "admin")!
    const devPerm = DEFAULT_ROLE_PERMISSIONS.find((p) => p.role === "developer")!
    const internPerm = DEFAULT_ROLE_PERMISSIONS.find((p) => p.role === "intern")!
    const guestPerm = DEFAULT_ROLE_PERMISSIONS.find((p) => p.role === "guest")!

    expect(adminPerm.dailyTokenLimit).toBeGreaterThan(devPerm.dailyTokenLimit)
    expect(devPerm.dailyTokenLimit).toBeGreaterThan(internPerm.dailyTokenLimit)
    expect(internPerm.dailyTokenLimit).toBeGreaterThan(guestPerm.dailyTokenLimit)
  })
})
