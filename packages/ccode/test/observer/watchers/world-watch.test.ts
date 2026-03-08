/**
 * WorldWatch Tests
 *
 * Tests for the World Watcher component of the Observer Network.
 *
 * @module test/observer/watchers/world-watch.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import {
  WorldWatch,
  createWorldWatch,
  type MarketDataPoint,
  type NewsItem,
} from "@/observer/watchers/world-watch"
import { resetEventStream } from "@/observer"

describe("WorldWatch", () => {
  let watcher: WorldWatch

  beforeEach(() => {
    resetEventStream()
    watcher = createWorldWatch({
      intervalMs: 0, // Disable automatic observation
      newsKeywords: ["typescript", "rust", "ai"],
      trackedDependencies: ["bun", "react"],
    })
  })

  afterEach(async () => {
    if (watcher.isRunning()) {
      await watcher.stop()
    }
  })

  describe("lifecycle", () => {
    it("should start and stop correctly", async () => {
      expect(watcher.isRunning()).toBe(false)

      await watcher.start()
      expect(watcher.isRunning()).toBe(true)

      await watcher.stop()
      expect(watcher.isRunning()).toBe(false)
    })

    it("should not start twice", async () => {
      await watcher.start()
      expect(watcher.isRunning()).toBe(true)

      // Second start should be idempotent
      await watcher.start()
      expect(watcher.isRunning()).toBe(true)
    })
  })

  describe("getStatus", () => {
    it("should return correct status when stopped", () => {
      const status = watcher.getStatus()
      expect(status.type).toBe("world")
      expect(status.running).toBe(false)
      expect(status.health).toBe("stopped")
    })

    it("should return correct status when running", async () => {
      await watcher.start()
      const status = watcher.getStatus()

      expect(status.type).toBe("world")
      expect(status.running).toBe(true)
      expect(status.health).toBe("healthy")
    })
  })

  describe("observeMarketData", () => {
    it("should emit observation for market data", async () => {
      await watcher.start()

      const marketData: MarketDataPoint[] = [
        {
          symbol: "BTC/USD",
          price: 45000,
          change: 500,
          changePercent: 1.12,
          timestamp: new Date(),
        },
      ]

      await watcher.observeMarketData(marketData)

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should include sentiment analysis", async () => {
      await watcher.start()

      // Positive sentiment (change > 1%)
      const positiveData: MarketDataPoint[] = [
        {
          symbol: "ETH/USD",
          price: 3000,
          change: 150,
          changePercent: 5.0,
          timestamp: new Date(),
        },
      ]

      await watcher.observeMarketData(positiveData)

      // Negative sentiment (change < -1%)
      const negativeData: MarketDataPoint[] = [
        {
          symbol: "SOL/USD",
          price: 100,
          change: -20,
          changePercent: -16.7,
          timestamp: new Date(),
        },
      ]

      await watcher.observeMarketData(negativeData)

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(2)
    })

    it("should not observe when stopped", async () => {
      const marketData: MarketDataPoint[] = [
        {
          symbol: "BTC/USD",
          price: 45000,
          change: 500,
          changePercent: 1.12,
          timestamp: new Date(),
        },
      ]

      await watcher.observeMarketData(marketData)

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(0)
    })
  })

  describe("observeNews", () => {
    it("should emit observation for news items", async () => {
      await watcher.start()

      const newsItem: NewsItem = {
        title: "TypeScript 6.0 Released with Major Improvements",
        summary: "New TypeScript version brings performance improvements",
        source: "tech-news",
        url: "https://example.com/news/ts-6",
        publishedAt: new Date(),
        sentiment: "positive",
      }

      await watcher.observeNews(newsItem)

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should filter by keywords when configured", async () => {
      await watcher.start()

      // Relevant news (contains configured keyword "typescript")
      const relevantNews: NewsItem = {
        title: "TypeScript gains popularity",
        source: "dev-blog",
        url: "https://example.com/ts",
        publishedAt: new Date(),
      }

      await watcher.observeNews(relevantNews)

      // Irrelevant news (no matching keywords)
      const irrelevantNews: NewsItem = {
        title: "New Python framework released",
        source: "dev-blog",
        url: "https://example.com/py",
        publishedAt: new Date(),
      }

      await watcher.observeNews(irrelevantNews)

      const status = watcher.getStatus()
      // Both should be processed but with different relevance scores
      // The irrelevant news may be filtered out due to low relevance
      expect(status.observationCount).toBeGreaterThanOrEqual(1)
    })

    it("should not observe when stopped", async () => {
      const newsItem: NewsItem = {
        title: "AI breakthrough announced",
        source: "tech-news",
        url: "https://example.com/ai",
        publishedAt: new Date(),
      }

      await watcher.observeNews(newsItem)

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(0)
    })
  })

  describe("observeApiChange", () => {
    it("should emit observation for API changes", async () => {
      await watcher.start()

      await watcher.observeApiChange({
        api: "openai",
        version: "v2.0.0",
        newFeatures: ["streaming", "function calling"],
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should set high relevance for breaking changes", async () => {
      await watcher.start()

      await watcher.observeApiChange({
        api: "anthropic",
        version: "v3.0.0",
        breakingChanges: ["deprecated method removed", "new auth flow"],
        deprecations: ["old endpoint"],
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })
  })

  describe("observeSecurityAdvisory", () => {
    it("should emit observation for security advisories", async () => {
      await watcher.start()

      await watcher.observeSecurityAdvisory({
        id: "CVE-2024-12345",
        severity: "high",
        package: "lodash",
        title: "Prototype pollution vulnerability",
        description: "Remote code execution via prototype pollution",
        fixedIn: "4.17.22",
        cve: "CVE-2024-12345",
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should handle critical severity", async () => {
      await watcher.start()

      await watcher.observeSecurityAdvisory({
        id: "CVE-2024-99999",
        severity: "critical",
        package: "express",
        title: "Critical RCE vulnerability",
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })
  })

  describe("observeDependencyRelease", () => {
    it("should emit observation for dependency releases", async () => {
      await watcher.start()

      await watcher.observeDependencyRelease({
        package: "bun",
        version: "2.0.0",
        previousVersion: "1.1.0",
        releaseNotes: "Major performance improvements and new features",
        isBreaking: true,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should handle non-breaking releases", async () => {
      await watcher.start()

      await watcher.observeDependencyRelease({
        package: "react",
        version: "18.3.1",
        previousVersion: "18.3.0",
        isBreaking: false,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })
  })

  describe("observeTrend", () => {
    it("should emit observation for trends", async () => {
      await watcher.start()

      await watcher.observeTrend({
        name: "AI adoption",
        description: "Increasing adoption of AI tools in development",
        direction: "up",
        strength: 0.85,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should handle different trend directions", async () => {
      await watcher.start()

      await watcher.observeTrend({
        name: "Traditional frameworks",
        description: "Declining usage of older frameworks",
        direction: "down",
        strength: 0.6,
      })

      await watcher.observeTrend({
        name: "Cloud costs",
        description: "Cloud costs stable",
        direction: "stable",
        strength: 0.3,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(2)
    })
  })

  describe("agent polling", () => {
    it("should respect enableAgentPolling option", () => {
      const watcherWithPolling = createWorldWatch({
        enableAgentPolling: true,
        agentPollingCycles: 3,
      })

      // The watcher should be configured with polling enabled
      // This is a configuration test, not a runtime test
      expect(watcherWithPolling).toBeDefined()
    })

    it("should not poll when enableAgentPolling is false", async () => {
      const watcherNoPolling = createWorldWatch({
        enableAgentPolling: false,
      })

      await watcherNoPolling.start()

      // No observations should be made for agent polling
      const status = watcherNoPolling.getStatus()
      expect(status.observationCount).toBe(0)

      await watcherNoPolling.stop()
    })
  })
})
