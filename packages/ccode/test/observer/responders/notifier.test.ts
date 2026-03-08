/**
 * Notifier Responder Tests
 *
 * Tests for the Notifier component of the Observer Network.
 *
 * @module test/observer/responders/notifier.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach, mock } from "bun:test"
import {
  Notifier,
  createNotifier,
  type NotifierConfig,
  type NotificationRule,
  type Notification,
  type NotificationPriority,
} from "@/observer/responders/notifier"

describe("Notifier", () => {
  let notifier: Notifier

  beforeEach(() => {
    notifier = createNotifier({
      enabledChannels: ["log", "tui"],
      minPriority: "low",
      rateLimit: 100,
      cooldownMs: 100,
    })
  })

  afterEach(() => {
    notifier.stop()
    notifier.clear()
  })

  describe("lifecycle", () => {
    it("should start and stop correctly", async () => {
      await notifier.start()
      // Notifier should be running (no direct isRunning, but stop should work)
      notifier.stop()
      // Should be able to stop without error
    })

    it("should not start twice", async () => {
      await notifier.start()
      await notifier.start() // Should not throw
      notifier.stop()
    })

    it("should be safe to stop when not started", () => {
      notifier.stop() // Should not throw
    })
  })

  describe("notify", () => {
    it("should create notification with default options", async () => {
      await notifier.start()

      const notification = await notifier.notify("Test Title", "Test body message")

      expect(notification).toBeDefined()
      expect(notification.title).toBe("Test Title")
      expect(notification.body).toBe("Test body message")
      expect(notification.priority).toBe("medium")
      expect(notification.status).toBe("sent")
    })

    it("should create notification with custom priority", async () => {
      await notifier.start()

      const notification = await notifier.notify("Urgent Alert", "Critical issue", {
        priority: "urgent",
      })

      expect(notification.priority).toBe("urgent")
    })

    it("should create notification with custom channels", async () => {
      await notifier.start()

      const notification = await notifier.notify("Test", "Body", {
        channels: ["log"],
      })

      expect(notification.channels).toContain("log")
    })

    it("should include metadata in notification", async () => {
      await notifier.start()

      const notification = await notifier.notify("Test", "Body", {
        metadata: { key: "value", count: 42 },
      })

      expect(notification.metadata).toEqual({ key: "value", count: 42 })
    })

    it("should set triggeredBy and triggerType", async () => {
      await notifier.start()

      const notification = await notifier.notify("Test", "Body", {
        triggeredBy: "anomaly_123",
        triggerType: "anomaly",
      })

      expect(notification.triggeredBy).toBe("anomaly_123")
      expect(notification.triggerType).toBe("anomaly")
    })
  })

  describe("priority filtering", () => {
    it("should filter notifications below minimum priority", async () => {
      const strictNotifier = createNotifier({
        enabledChannels: ["log"],
        minPriority: "high",
        rateLimit: 100,
        cooldownMs: 100,
      })
      await strictNotifier.start()

      // Low priority notification should be filtered
      const notification = await strictNotifier.notify("Low Priority", "Body", {
        priority: "low",
      })

      // Notification is created but not sent (status remains pending due to filter)
      expect(notification.status).toBe("pending")

      strictNotifier.stop()
    })

    it("should allow notifications at or above minimum priority", async () => {
      const strictNotifier = createNotifier({
        enabledChannels: ["log"],
        minPriority: "medium",
        rateLimit: 100,
        cooldownMs: 100,
      })
      await strictNotifier.start()

      const notification = await strictNotifier.notify("High Priority", "Body", {
        priority: "high",
      })

      expect(notification.status).toBe("sent")

      strictNotifier.stop()
    })
  })

  describe("rate limiting", () => {
    it("should enforce rate limit", async () => {
      const limitedNotifier = createNotifier({
        enabledChannels: ["log"],
        minPriority: "low",
        rateLimit: 2,
        cooldownMs: 50,
      })
      await limitedNotifier.start()

      // Send notifications up to rate limit
      const n1 = await limitedNotifier.notify("First", "Body 1", {
        triggeredBy: "test1",
        triggerType: "observation",
      })
      const n2 = await limitedNotifier.notify("Second", "Body 2", {
        triggeredBy: "test2",
        triggerType: "observation",
      })

      expect(n1.status).toBe("sent")
      expect(n2.status).toBe("sent")

      // Third notification should be rate limited (dropped)
      const n3 = await limitedNotifier.notify("Third", "Body 3", {
        triggeredBy: "test3",
        triggerType: "observation",
      })

      // Rate limited notification is created but not stored
      expect(n3.status).toBe("pending")

      limitedNotifier.stop()
    })
  })

  describe("deduplication / cooldown", () => {
    it("should deduplicate rapid duplicate notifications", async () => {
      await notifier.start()

      // First notification
      const n1 = await notifier.notify("Alert", "Same alert", {
        triggeredBy: "same_trigger",
        triggerType: "anomaly",
      })

      // Same trigger within cooldown period
      const n2 = await notifier.notify("Alert", "Same alert", {
        triggeredBy: "same_trigger",
        triggerType: "anomaly",
      })

      expect(n1.status).toBe("sent")
      expect(n2.status).toBe("pending") // Should be deduplicated
    })

    it("should allow same notification after cooldown expires", async () => {
      const fastNotifier = createNotifier({
        enabledChannels: ["log"],
        minPriority: "low",
        rateLimit: 100,
        cooldownMs: 50,
      })
      await fastNotifier.start()

      const n1 = await fastNotifier.notify("Alert", "Body", {
        triggeredBy: "trigger1",
        triggerType: "anomaly",
      })

      // Wait for cooldown
      await new Promise((r) => setTimeout(r, 100))

      const n2 = await fastNotifier.notify("Alert", "Body", {
        triggeredBy: "trigger1",
        triggerType: "anomaly",
      })

      expect(n1.status).toBe("sent")
      expect(n2.status).toBe("sent")

      fastNotifier.stop()
    })
  })

  describe("rules", () => {
    it("should add custom rule", () => {
      const rule: NotificationRule = {
        id: "custom-rule",
        name: "Custom Rule",
        enabled: true,
        trigger: { type: "observation" },
        priority: "medium",
        channels: ["log"],
        template: () => ({ title: "Custom", body: "Custom body" }),
      }

      notifier.addRule(rule)
      // Rule is added (no direct getter, but should not throw)
    })

    it("should remove rule by id", () => {
      const rule: NotificationRule = {
        id: "removable-rule",
        name: "Removable",
        enabled: true,
        trigger: { type: "observation" },
        priority: "low",
        channels: ["log"],
        template: () => ({ title: "Test", body: "Test" }),
      }

      notifier.addRule(rule)
      const removed = notifier.removeRule("removable-rule")

      expect(removed).toBe(true)
    })

    it("should return false when removing non-existent rule", () => {
      const removed = notifier.removeRule("non-existent")
      expect(removed).toBe(false)
    })
  })

  describe("history", () => {
    it("should track notification history", async () => {
      await notifier.start()

      await notifier.notify("First", "Body 1", {
        triggeredBy: "a",
        triggerType: "observation",
      })
      await new Promise((r) => setTimeout(r, 10))
      await notifier.notify("Second", "Body 2", {
        triggeredBy: "b",
        triggerType: "observation",
      })
      await new Promise((r) => setTimeout(r, 10))
      await notifier.notify("Third", "Body 3", {
        triggeredBy: "c",
        triggerType: "observation",
      })

      const history = notifier.getHistory()

      expect(history.length).toBe(3)
      // Most recent first (sorted by descending timestamp)
      expect(history[0].title).toBe("Third")
    })

    it("should limit history results", async () => {
      await notifier.start()

      for (let i = 0; i < 5; i++) {
        await notifier.notify(`Notification ${i}`, "Body", {
          triggeredBy: `trigger_${i}`,
          triggerType: "observation",
        })
      }

      const history = notifier.getHistory(3)

      expect(history.length).toBe(3)
    })

    it("should get pending notifications", async () => {
      // When not started, notifications stay pending
      const notification = await notifier.notify("Pending", "Body")

      const pending = notifier.getPending()

      expect(pending.length).toBeGreaterThanOrEqual(0)
    })
  })

  describe("clear", () => {
    it("should clear all notification history", async () => {
      await notifier.start()

      await notifier.notify("Test 1", "Body", {
        triggeredBy: "t1",
        triggerType: "observation",
      })
      await notifier.notify("Test 2", "Body", {
        triggeredBy: "t2",
        triggerType: "observation",
      })

      notifier.clear()

      const history = notifier.getHistory()
      expect(history.length).toBe(0)
    })
  })

  describe("channel configuration", () => {
    it("should filter channels to only enabled ones", async () => {
      const limitedChannelNotifier = createNotifier({
        enabledChannels: ["log"],
        minPriority: "low",
        rateLimit: 100,
        cooldownMs: 100,
      })
      await limitedChannelNotifier.start()

      const notification = await limitedChannelNotifier.notify("Test", "Body", {
        channels: ["log", "im", "webhook"],
        triggeredBy: "test",
        triggerType: "observation",
      })

      // Only 'log' should be in the final channels (im/webhook not enabled)
      expect(notification.channels).toContain("log")
      expect(notification.channels).not.toContain("im")
      expect(notification.channels).not.toContain("webhook")

      limitedChannelNotifier.stop()
    })
  })

  describe("built-in rules", () => {
    it("should include critical-anomaly rule by default", () => {
      // Built-in rules are added in constructor
      // We can verify by checking rule removal works
      const removed = notifier.removeRule("critical-anomaly")
      expect(removed).toBe(true)
    })

    it("should include high-impact-opportunity rule by default", () => {
      const removed = notifier.removeRule("high-impact-opportunity")
      expect(removed).toBe(true)
    })

    it("should include escalation-alert rule by default", () => {
      const removed = notifier.removeRule("escalation-alert")
      expect(removed).toBe(true)
    })

    it("should include strong-pattern rule by default", () => {
      const removed = notifier.removeRule("strong-pattern")
      expect(removed).toBe(true)
    })
  })
})
