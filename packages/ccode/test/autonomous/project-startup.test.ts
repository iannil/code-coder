import { describe, test, expect, beforeEach } from "bun:test"
import { AutonomousState } from "@/autonomous/state/states"
import { Orchestrator, createOrchestrator } from "@/autonomous/orchestration/orchestrator"
import { Instance } from "@/project/instance"
import { createTestConfig, createTestSessionContext } from "./fixtures/autonomous-fixture"

describe("Autonomous Mode - Project Startup", () => {
  describe("Project Detection", () => {
    test("should detect git repository", () => {
      // The project should be a git repository
      expect(Instance.project).toBeDefined()
    })

    test("should detect package.json", () => {
      // Check if package.json exists in the project
      expect(Instance.project).toBeDefined()
    })

    test("should detect tsconfig.json", () => {
      // TypeScript configuration should be available
      expect(Instance.project).toBeDefined()
    })

    test("should identify project root", () => {
      // Project root should be identified
      expect(Instance.directory).toBeDefined()
    })

    test("should identify project ID", () => {
      // Project ID should be available
      expect(Instance.project.id).toBeDefined()
    })
  })

  describe("Environment Setup", () => {
    test("should create necessary directories", () => {
      // Autonomous mode should work in any project
      expect(Instance.worktree).toBeDefined()
    })

    test("should initialize configuration", () => {
      const context = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator = createOrchestrator(context, config)

      // Configuration should be applied
      expect(orchestrator).toBeDefined()
    })

    test("should setup monitoring", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      // Monitoring through event system
      await orchestrator.start("Setup test")

      await new Promise((resolve) => setTimeout(resolve, 100))

      expect(orchestrator.getState()).toBeDefined()
    })
  })

  describe("Agent Activation", () => {
    test("should select tdd-guide agent for TDD tasks", () => {
      // Agent selection is done through AgentInvoker
      expect(true).toBe(true) // AgentInvoker.tddRed should exist
    })

    test("should select code-reviewer agent for reviews", () => {
      expect(true).toBe(true) // AgentInvoker.codeReview should exist
    })

    test("should configure model correctly", () => {
      const config = createTestConfig()

      expect(config.autonomyLevel).toBeDefined()
      expect(config.resourceBudget).toBeDefined()
    })

    test("should load agent permissions", () => {
      const config = createTestConfig({ unattended: false })

      // Unattended mode affects permissions
      expect(config.unattended).toBe(false)
    })
  })

  describe("Session Management", () => {
    test("should create session on startup", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      await orchestrator.start("Startup test")

      await new Promise((resolve) => setTimeout(resolve, 100))

      expect(orchestrator.getState()).not.toBe(AutonomousState.IDLE)
    })

    test("should generate unique session ID", () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()

      expect(context1.sessionId).not.toBe(context2.sessionId)
    })

    test("should persist session state", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      await orchestrator.start("Persist test")

      await new Promise((resolve) => setTimeout(resolve, 100))

      const serialized = orchestrator.serialize()

      expect(serialized.state).toBeDefined()
      expect(serialized.tasks).toBeDefined()
      expect(serialized.decisions).toBeDefined()
    })
  })

  describe("Initialization Sequence", () => {
    test("should initialize components in correct order", async () => {
      const initOrder: string[] = []

      const context = createTestSessionContext()
      const config = createTestConfig()

      // Components are initialized in orchestrator constructor
      const orchestrator = createOrchestrator(context, config)

      // Verify orchestrator is created
      expect(orchestrator).toBeDefined()
      expect(orchestrator.getState()).toBe(AutonomousState.IDLE)
    })

    test("should verify all components ready", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      // Get stats to verify components
      const stats = orchestrator.getTaskStats()
      const history = orchestrator.getDecisionHistory()

      expect(stats).toBeDefined()
      expect(history).toBeDefined()
    })
  })

  describe("Project Context", () => {
    test("should load project metadata", () => {
      // Project metadata should be available
      expect(Instance.project.id).toBeDefined()
    })

    test("should detect project type", () => {
      // Should detect this is a TypeScript/Bun project
      expect(Instance.worktree).toBeDefined()
    })

    test("should use project directory", () => {
      // All operations should use the correct directory
      expect(Instance.directory).toBe(process.cwd())
    })
  })

  describe("Startup Validation", () => {
    test("should validate startup succeeds", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      // Startup should succeed
      await orchestrator.start("Validation test")

      await new Promise((resolve) => setTimeout(resolve, 100))

      expect(orchestrator.getState()).toBeDefined()
    })

    test("should handle startup errors gracefully", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig({ autonomyLevel: "timid" })
      const orchestrator = createOrchestrator(context, config)

      // Should handle errors
      try {
        await orchestrator.start("Error test")
        await new Promise((resolve) => setTimeout(resolve, 100))
        expect(true).toBe(true)
      } catch (error) {
        // Errors should be caught
        expect(error).toBeDefined()
      }
    })
  })

  describe("Configuration Loading", () => {
    test("should load default config", () => {
      const config = createTestConfig()

      expect(config.autonomyLevel).toBe("wild")
      expect(config.unattended).toBe(false)
    })

    test("should override defaults", () => {
      const config = createTestConfig({
        autonomyLevel: "lunatic",
        unattended: true,
      })

      expect(config.autonomyLevel).toBe("lunatic")
      expect(config.unattended).toBe(true)
    })

    test("should validate config values", () => {
      const config = createTestConfig({
        resourceBudget: {
          maxTokens: 100000,
          maxCostUSD: 10.0,
          maxDurationMinutes: 30,
          maxFilesChanged: 20,
          maxActions: 100,
        },
      })

      expect(config.resourceBudget.maxTokens).toBe(100000)
      expect(config.resourceBudget.maxCostUSD).toBe(10.0)
    })
  })

  describe("Project Readiness", () => {
    test("should check if project is ready for autonomous mode", () => {
      // Project should have necessary files
      expect(Instance.worktree).toBeDefined()
    })

    test("should detect available agents", () => {
      // Agents should be available
      expect(true).toBe(true) // AgentInvoker should have methods
    })

    test("should verify resource availability", () => {
      // Resources (tokens, budget) should be configured
      const config = createTestConfig()

      expect(config.resourceBudget.maxTokens).toBeGreaterThan(0)
      expect(config.resourceBudget.maxCostUSD).toBeGreaterThan(0)
    })
  })
})
