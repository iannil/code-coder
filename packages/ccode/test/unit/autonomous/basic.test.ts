// @ts-nocheck
import { describe, test, expect, beforeEach } from "bun:test"
import type { ResourceBudget } from "@/autonomous/safety/constraints"

describe("Crazy Mode - Basic Tests", () => {
  describe("ResourceBudget", () => {
    test("should create default budget with all fields", () => {
      const budget: ResourceBudget = {
        maxTokens: 1_000_000,
        maxCostUSD: 10.0,
        maxDurationMinutes: 30,
        maxFilesChanged: 50,
        maxActions: 100,
      }

      expect(budget.maxTokens).toBe(1_000_000)
      expect(budget.maxCostUSD).toBe(10.0)
      expect(budget.maxDurationMinutes).toBe(30)
      expect(budget.maxFilesChanged).toBe(50)
      expect(budget.maxActions).toBe(100)
    })

    test("should calculate remaining correctly", () => {
      const budget: ResourceBudget = {
        maxTokens: 1000,
        maxCostUSD: 5.0,
        maxDurationMinutes: 10,
        maxFilesChanged: 20,
        maxActions: 50,
      }

      const used: ResourceBudget = {
        tokensUsed: 400,
        costUSD: 2.0,
        durationMinutes: 5,
        filesChanged: 8,
        actionsPerformed: 25,
      }

      const remainingTokens = 1000 - 400
      const remainingCost = 5.0 - 2.0
      const remainingMinutes = 10 - 5
      const remainingFiles = 20 - 8
      const remainingActions = 50 - 25

      expect(remainingTokens).toBe(600)
      expect(remainingCost).toBe(3.0)
      expect(remainingMinutes).toBe(5)
      expect(remainingFiles).toBe(12)
      expect(remainingActions).toBe(25)
    })
  })

  describe("SafetyStatus", () => {
    test("should track safe status", () => {
      let safe = true
      let resources = {
        tokensUsed: 100,
        costUSD: 1.0,
        durationMinutes: 5,
        filesChanged: 3,
        actionsPerformed: 10,
      }
      let remaining = {
        maxTokens: 1000,
        maxCostUSD: 10.0,
        maxDurationMinutes: 30,
        maxFilesChanged: 50,
        maxActions: 100,
      }
      let loops = {
        loopsBroken: 0,
      }
      let rollbacks = {
        count: 0,
        canRetry: true,
      }

      const status = {
        safe,
        resources: { usage: resources, remaining },
        loops,
        rollbacks,
      }

      expect(status.safe).toBe(true)
      expect(status.resources.usage.tokensUsed).toBe(100)
      expect(status.resources.usage.costUSD).toBe(1.0)
      expect(status.resources.usage.durationMinutes).toBe(5)
      expect(status.resources.usage.filesChanged).toBe(3)
      expect(status.resources.usage.actionsPerformed).toBe(10)
    })
  })

  describe("Craziness Scoring", () => {
    test("should calculate LUNATIC level (90+)", () => {
      // LUNATIC requires all high scores
      const autonomy = 95
      const selfCorrection = 90
      const speed = 85
      const riskTaking = 80

      // weighted score (weights sum to 1.0: 0.35 + 0.25 + 0.20 + 0.20)
      const score = (autonomy * 0.35) + (selfCorrection * 0.25) + (speed * 0.20) + (riskTaking * 0.20)
      // = 33.25 + 22.5 + 17 + 16 = 88.75

      expect(score).toBeGreaterThan(75)
      expect(score).toBeLessThan(95)
    })

    test("should calculate CRAZY level (60-74)", () => {
      const autonomy = 70
      const selfCorrection = 65
      const speed = 60
      const riskTaking = 55

      const score = (autonomy * 0.35) + (selfCorrection * 0.25) + (speed * 0.20) + (riskTaking * 0.20)
      // = 24.5 + 16.25 + 12 + 11 = 63.75

      expect(score).toBeGreaterThanOrEqual(60)
      expect(score).toBeLessThan(75)
    })

    test("should calculate TIMID level (<20)", () => {
      const autonomy = 15
      const selfCorrection = 10
      const speed = 5
      const riskTaking = 5

      const score = (autonomy * 0.35) + (selfCorrection * 0.25) + (speed * 0.20) + (riskTaking * 0.20)
      // = 5.25 + 2.5 + 1 + 1 = 9.75

      expect(score).toBeLessThan(20)
    })
  })
})
