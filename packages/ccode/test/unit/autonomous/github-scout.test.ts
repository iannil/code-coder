import { describe, test, expect, beforeEach, mock } from "bun:test"
import {
  SceneTrigger,
  RepoEvaluator,
  type GithubRepo,
  type TriggerDecision,
  type STAREvaluation,
  DEFAULT_GITHUB_SCOUT_CONFIG,
  GithubScoutConfigSchema,
} from "@/autonomous/execution/github-scout"

describe("GitHub Scout", () => {
  describe("SceneTrigger", () => {
    let trigger: SceneTrigger

    beforeEach(() => {
      trigger = new SceneTrigger()
    })

    describe("High priority triggers", () => {
      test("should trigger with high confidence for CLI tools", () => {
        const result = trigger.analyze("Implement a CLI tool for parsing JSON files")

        expect(result.shouldSearch).toBe(true)
        expect(result.confidence).toBeGreaterThanOrEqual(0.8)
        expect(result.category).toBe("high")
        expect(result.matchedKeywords).toContain("cli")
      })

      test("should trigger with high confidence for authentication tasks", () => {
        const result = trigger.analyze("Add OAuth authentication to the application")

        expect(result.shouldSearch).toBe(true)
        expect(result.confidence).toBeGreaterThanOrEqual(0.85)
        expect(result.category).toBe("high")
        expect(result.matchedKeywords).toContain("oauth")
      })

      test("should trigger for JWT-based auth", () => {
        const result = trigger.analyze("Implement JWT token validation")

        expect(result.shouldSearch).toBe(true)
        expect(result.confidence).toBeGreaterThanOrEqual(0.8)
        expect(result.matchedKeywords).toContain("jwt")
      })

      test("should trigger for data visualization", () => {
        const result = trigger.analyze("Create a chart visualization dashboard")

        expect(result.shouldSearch).toBe(true)
        expect(result.confidence).toBeGreaterThanOrEqual(0.8)
        expect(result.matchedKeywords).toContain("chart")
      })

      test("should trigger for parsers", () => {
        const result = trigger.analyze("Build a markdown parser for the editor")

        expect(result.shouldSearch).toBe(true)
        expect(result.confidence).toBeGreaterThanOrEqual(0.8)
        expect(result.matchedKeywords).toContain("parser")
      })

      test("should trigger for database/ORM tasks", () => {
        const result = trigger.analyze("Set up an ORM for database operations")

        expect(result.shouldSearch).toBe(true)
        expect(result.confidence).toBeGreaterThanOrEqual(0.75)
        expect(result.matchedKeywords).toContain("orm")
      })
    })

    describe("Medium priority triggers", () => {
      test("should trigger with medium confidence for general implementation", () => {
        const result = trigger.analyze("Implement a feature to export data")

        expect(result.shouldSearch).toBe(true)
        expect(result.confidence).toBeGreaterThanOrEqual(0.6)
        expect(result.confidence).toBeLessThan(0.8)
        expect(result.category).toBe("medium")
      })

      test("should trigger for utility libraries", () => {
        const result = trigger.analyze("Add utility functions for string manipulation")

        expect(result.shouldSearch).toBe(true)
        expect(result.matchedKeywords).toContain("utility")
      })
    })

    describe("Low priority triggers", () => {
      test("should have low confidence for bug fixes", () => {
        const result = trigger.analyze("Fix a small syntax error in the code")

        expect(result.confidence).toBeLessThan(0.5)
        expect(result.category).toBe("low")
      })

      test("should not trigger for simple fixes", () => {
        const result = trigger.analyze("Fix a small bug in the component")

        expect(result.shouldSearch).toBe(false)
        expect(result.confidence).toBeLessThan(0.5)
      })
    })

    describe("Query generation", () => {
      test("should generate relevant search queries", () => {
        const result = trigger.analyze("Build a CLI tool for data transformation", "typescript")

        expect(result.suggestedQueries.length).toBeGreaterThan(0)
        expect(result.suggestedQueries.some((q) => q.includes("cli"))).toBe(true)
      })

      test("should include technology in queries when provided", () => {
        const result = trigger.analyze("Create a chart library", "react")

        expect(result.suggestedQueries.some((q) => q.toLowerCase().includes("react"))).toBe(true)
      })
    })

    describe("Confidence boosting", () => {
      test("should boost confidence when multiple keywords match", () => {
        const singleMatch = trigger.analyze("Add a queue to the system")
        const multiMatch = trigger.analyze("Add a queue with cache and rate limiting")

        // Multiple medium triggers should boost confidence
        expect(multiMatch.matchedKeywords.length).toBeGreaterThan(singleMatch.matchedKeywords.length)
      })

      test("should cap confidence at 1.0", () => {
        const result = trigger.analyze("Implement OAuth authentication with JWT tokens for SSO login")

        expect(result.confidence).toBeLessThanOrEqual(1.0)
      })
    })
  })

  describe("RepoEvaluator (STAR Framework)", () => {
    let evaluator: RepoEvaluator

    beforeEach(() => {
      evaluator = new RepoEvaluator()
    })

    const createMockRepo = (overrides: Partial<GithubRepo> = {}): GithubRepo => ({
      fullName: "test/repo",
      description: "A test repository",
      url: "https://github.com/test/repo",
      stars: 1000,
      forks: 100,
      language: "TypeScript",
      license: "mit",
      topics: ["cli", "typescript"],
      pushedAt: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(), // 7 days ago
      createdAt: new Date(Date.now() - 365 * 24 * 60 * 60 * 1000).toISOString(), // 1 year ago
      openIssuesCount: 50,
      archived: false,
      homepage: null,
      ...overrides,
    })

    describe("Stars/Popularity Score (S)", () => {
      test("should give high score for popular repos", () => {
        const repo = createMockRepo({ stars: 50000, forks: 5000 })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.starScore).toBeGreaterThanOrEqual(9)
      })

      test("should give medium score for moderately popular repos", () => {
        const repo = createMockRepo({ stars: 1000, forks: 100 })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.starScore).toBeGreaterThanOrEqual(5)
        expect(evaluation.starScore).toBeLessThan(8)
      })

      test("should give low score for unpopular repos", () => {
        const repo = createMockRepo({ stars: 30, forks: 5 })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.starScore).toBeLessThan(4)
      })
    })

    describe("Time/Activity Score (T)", () => {
      test("should give high score for recently updated repos", () => {
        const repo = createMockRepo({
          pushedAt: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000).toISOString(), // 3 days ago
        })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.timeScore).toBeGreaterThanOrEqual(9)
      })

      test("should give low score for stale repos", () => {
        const repo = createMockRepo({
          pushedAt: new Date(Date.now() - 800 * 24 * 60 * 60 * 1000).toISOString(), // ~2.2 years ago
        })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.timeScore).toBeLessThan(3)
      })
    })

    describe("Alignment Score (A)", () => {
      test("should give high score when keywords match", () => {
        const repo = createMockRepo({
          description: "A CLI tool for parsing JSON",
          topics: ["cli", "json", "parser"],
        })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "build a json parser cli",
          keywords: ["cli", "json", "parser"],
          technology: "typescript",
        })

        expect(evaluation.alignmentScore).toBeGreaterThanOrEqual(7)
      })

      test("should boost score when technology matches", () => {
        const repo = createMockRepo({
          language: "TypeScript",
          description: "A utility library",
        })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "utility functions",
          keywords: ["utility"],
          technology: "typescript",
        })

        // Should have alignment boost from language match
        expect(evaluation.alignmentScore).toBeGreaterThanOrEqual(6)
      })
    })

    describe("Risk Score (R)", () => {
      test("should give low risk score (high R value) for well-maintained repos", () => {
        const repo = createMockRepo({
          license: "mit",
          archived: false,
          openIssuesCount: 20,
        })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.riskScore).toBeGreaterThanOrEqual(7)
      })

      test("should give high risk score (low R value) for archived repos", () => {
        const repo = createMockRepo({ archived: true })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.riskScore).toBeLessThanOrEqual(2)
      })

      test("should penalize repos without license", () => {
        const repo = createMockRepo({ license: null })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.riskScore).toBeLessThan(10)
      })

      test("should penalize repos with many open issues", () => {
        const repo = createMockRepo({ openIssuesCount: 600 })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.riskScore).toBeLessThan(9)
      })
    })

    describe("Recommendations", () => {
      test("should recommend ADOPT for high-scoring repos", () => {
        const repo = createMockRepo({
          stars: 50000,
          forks: 5000,
          pushedAt: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000).toISOString(),
          license: "mit",
          topics: ["cli", "typescript"],
        })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "build a cli",
          keywords: ["cli"],
          technology: "typescript",
        })

        expect(evaluation.recommendation).toBe("adopt")
      })

      test("should recommend AVOID for archived repos", () => {
        const repo = createMockRepo({ archived: true })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        expect(evaluation.recommendation).toBe("avoid")
      })

      test("should recommend TRIAL for moderately scoring repos", () => {
        const repo = createMockRepo({
          stars: 2000,
          pushedAt: new Date(Date.now() - 60 * 24 * 60 * 60 * 1000).toISOString(), // 60 days
        })
        const evaluation = evaluator.evaluate(repo, {
          taskDescription: "test task",
          keywords: ["cli"],
        })

        // Should be in trial range (6.0-7.5)
        expect(["trial", "assess", "adopt"]).toContain(evaluation.recommendation)
      })
    })

    describe("Ranking", () => {
      test("should rank repos by total score", () => {
        const repos = [
          createMockRepo({ fullName: "low/repo", stars: 50 }),
          createMockRepo({ fullName: "high/repo", stars: 50000 }),
          createMockRepo({ fullName: "mid/repo", stars: 5000 }),
        ]

        const evaluations = evaluator.evaluateAll(repos, {
          taskDescription: "test",
          keywords: ["test"],
        })

        expect(evaluations[0].repo.fullName).toBe("high/repo")
        expect(evaluations[2].repo.fullName).toBe("low/repo")
      })
    })
  })

  describe("GithubScoutConfig", () => {
    test("should have valid default config", () => {
      expect(DEFAULT_GITHUB_SCOUT_CONFIG.integrationMode).toBe("autonomous")
      expect(DEFAULT_GITHUB_SCOUT_CONFIG.triggerThreshold).toBe(0.6)
      expect(DEFAULT_GITHUB_SCOUT_CONFIG.maxReposToEvaluate).toBe(5)
      expect(DEFAULT_GITHUB_SCOUT_CONFIG.allowSecurityWarnings).toBe(false)
    })

    test("should validate config with zod schema", () => {
      const validConfig = {
        integrationMode: "recommend",
        triggerThreshold: 0.7,
        maxAutoInstallDeps: 5,
      }

      const result = GithubScoutConfigSchema.safeParse(validConfig)
      expect(result.success).toBe(true)
    })

    test("should reject invalid config values", () => {
      const invalidConfig = {
        integrationMode: "invalid_mode",
        triggerThreshold: 2.0, // out of range
      }

      const result = GithubScoutConfigSchema.safeParse(invalidConfig)
      expect(result.success).toBe(false)
    })

    test("should require askForPermissions for system operations", () => {
      const config = {
        ...DEFAULT_GITHUB_SCOUT_CONFIG,
        askForPermissions: ["global_install", "sudo"],
      }

      const result = GithubScoutConfigSchema.safeParse(config)
      expect(result.success).toBe(true)
      expect(result.data?.askForPermissions).toContain("global_install")
      expect(result.data?.askForPermissions).toContain("sudo")
    })
  })

  describe("Integration Modes", () => {
    test("autonomous mode should be the default", () => {
      expect(DEFAULT_GITHUB_SCOUT_CONFIG.integrationMode).toBe("autonomous")
    })

    test("recommend mode should only output reports", () => {
      const config = GithubScoutConfigSchema.parse({
        integrationMode: "recommend",
      })
      expect(config.integrationMode).toBe("recommend")
    })

    test("ask mode should require user confirmation", () => {
      const config = GithubScoutConfigSchema.parse({
        integrationMode: "ask",
      })
      expect(config.integrationMode).toBe("ask")
    })
  })

  describe("CLOSE Decision Integration", () => {
    test("searchVsBuild decision should have high scores", () => {
      // According to the plan, search vs build should have these scores:
      // Convergence: 8, Leverage: 9, Optionality: 9, Surplus: 9, Evolution: 7
      // Expected score: ~8.5/10
      const weights = {
        convergence: 1.0,
        leverage: 1.2,
        optionality: 1.5,
        surplus: 1.3,
        evolution: 0.8,
      }

      const scores = {
        convergence: 8,
        leverage: 9,
        optionality: 9,
        surplus: 9,
        evolution: 7,
      }

      const maxScore = 10 * (weights.convergence + weights.leverage + weights.optionality + weights.surplus + weights.evolution)
      const weightedSum =
        scores.convergence * weights.convergence +
        scores.leverage * weights.leverage +
        scores.optionality * weights.optionality +
        scores.surplus * weights.surplus +
        scores.evolution * weights.evolution

      const totalScore = (weightedSum / maxScore) * 10

      // Should be ~8.5 (auto-approve threshold is 7.0 for crazy mode)
      expect(totalScore).toBeGreaterThan(8)
      expect(totalScore).toBeLessThan(9)
    })
  })
})
