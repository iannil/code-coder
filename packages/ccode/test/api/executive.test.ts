/**
 * Executive Dashboard API Tests
 */

import { describe, it, expect } from "bun:test"
import { getTrends, getTeams, getActivity, getSummary, executiveHealth } from "../../src/api/server/handlers/executive"

// Mock request helper
function createMockRequest(url: string): Request {
  return new Request(`http://localhost:4400${url}`)
}

describe("Executive API", () => {
  describe("GET /api/v1/executive/health", () => {
    it("should return healthy status", async () => {
      const request = createMockRequest("/api/v1/executive/health")
      const response = await executiveHealth(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(true)
      expect(body.data.status).toBe("healthy")
      expect(body.data.service).toBe("executive-dashboard")
    })
  })

  describe("GET /api/v1/executive/trends", () => {
    it("should return weekly trends by default", async () => {
      const request = createMockRequest("/api/v1/executive/trends")
      const response = await getTrends(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(true)
      expect(body.data.period).toBe("weekly")
      expect(body.data.days).toBe(7)
      expect(body.data.trends).toHaveLength(7)
      expect(body.data.totals).toBeDefined()
    })

    it("should return daily trends when specified", async () => {
      const request = createMockRequest("/api/v1/executive/trends?period=daily&days=1")
      const response = await getTrends(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(true)
      expect(body.data.period).toBe("daily")
      expect(body.data.days).toBe(1)
    })

    it("should return monthly trends when specified", async () => {
      const request = createMockRequest("/api/v1/executive/trends?period=monthly")
      const response = await getTrends(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(true)
      expect(body.data.period).toBe("monthly")
      expect(body.data.days).toBe(30)
    })

    it("should clamp days to maximum 90", async () => {
      const request = createMockRequest("/api/v1/executive/trends?days=200")
      const response = await getTrends(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.data.days).toBe(90)
    })

    it("should include cost_usd in trend data points", async () => {
      const request = createMockRequest("/api/v1/executive/trends")
      const response = await getTrends(request, {})

      const body = JSON.parse(response.body as string)
      const firstPoint = body.data.trends[0]
      expect(firstPoint.date).toBeDefined()
      expect(firstPoint.input_tokens).toBeGreaterThanOrEqual(0)
      expect(firstPoint.output_tokens).toBeGreaterThanOrEqual(0)
      expect(firstPoint.total_tokens).toBeGreaterThanOrEqual(0)
      expect(firstPoint.cost_usd).toBeGreaterThanOrEqual(0)
    })
  })

  describe("GET /api/v1/executive/teams", () => {
    it("should return team usage breakdown", async () => {
      const request = createMockRequest("/api/v1/executive/teams")
      const response = await getTeams(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(true)
      expect(body.data.teams).toBeDefined()
      expect(body.data.totals).toBeDefined()
      expect(body.data.team_count).toBeGreaterThan(0)
    })

    it("should include top_users in team data", async () => {
      const request = createMockRequest("/api/v1/executive/teams")
      const response = await getTeams(request, {})

      const body = JSON.parse(response.body as string)
      const firstTeam = body.data.teams[0]
      expect(firstTeam.team_id).toBeDefined()
      expect(firstTeam.team_name).toBeDefined()
      expect(firstTeam.member_count).toBeGreaterThan(0)
      expect(firstTeam.top_users).toBeDefined()
    })
  })

  describe("GET /api/v1/executive/activity", () => {
    it("should return project activity summary", async () => {
      const request = createMockRequest("/api/v1/executive/activity")
      const response = await getActivity(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(true)
      expect(body.data.projects).toBeDefined()
      expect(body.data.totals).toBeDefined()
      expect(body.data.project_count).toBeGreaterThan(0)
    })

    it("should include commit counts in project data", async () => {
      const request = createMockRequest("/api/v1/executive/activity")
      const response = await getActivity(request, {})

      const body = JSON.parse(response.body as string)
      const firstProject = body.data.projects[0]
      expect(firstProject.project_id).toBeDefined()
      expect(firstProject.project_name).toBeDefined()
      expect(firstProject.commits_today).toBeGreaterThanOrEqual(0)
      expect(firstProject.commits_week).toBeGreaterThanOrEqual(0)
      expect(firstProject.ai_sessions).toBeGreaterThanOrEqual(0)
    })
  })

  describe("GET /api/v1/executive/summary", () => {
    it("should return executive summary for weekly period", async () => {
      const request = createMockRequest("/api/v1/executive/summary")
      const response = await getSummary(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(true)
      expect(body.data.period).toBe("weekly")
      expect(body.data.total_cost_usd).toBeGreaterThan(0)
      expect(body.data.total_tokens).toBeGreaterThan(0)
      expect(body.data.active_users).toBeGreaterThan(0)
    })

    it("should include top_models in summary", async () => {
      const request = createMockRequest("/api/v1/executive/summary")
      const response = await getSummary(request, {})

      const body = JSON.parse(response.body as string)
      expect(body.data.top_models).toBeDefined()
      expect(body.data.top_models.length).toBeGreaterThan(0)
      const firstModel = body.data.top_models[0]
      expect(firstModel.model).toBeDefined()
      expect(firstModel.usage_percent).toBeGreaterThan(0)
    })

    it("should return summary for different periods", async () => {
      const periods = ["daily", "weekly", "monthly"]
      for (const period of periods) {
        const request = createMockRequest(`/api/v1/executive/summary?period=${period}`)
        const response = await getSummary(request, {})

        const body = JSON.parse(response.body as string)
        expect(body.success).toBe(true)
        expect(body.data.period).toBe(period)
      }
    })
  })
})
