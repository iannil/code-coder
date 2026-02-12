/**
 * Edge Case Test: Network Error Recovery
 *
 * Tests network error handling, retry logic, rate limiting,
 * and graceful degradation when providers are unreachable.
 */

import { describe, test, expect } from "bun:test"
import { SessionRetry } from "../../src/session/retry"
import { MessageV2 } from "../../src/session/message-v2"

describe("Network Error Recovery", () => {
  describe("Connection Timeout Retry", () => {
    test("should retry on connection timeout with exponential backoff", () => {
      const error = new MessageV2.APIError({
        message: "Connection timed out",
        isRetryable: true,
      }).toObject() as MessageV2.APIError

      // Verify delays increase exponentially
      const delays = Array.from({ length: 5 }, (_, i) => SessionRetry.delay(i + 1, error))
      expect(delays[0]).toBe(2000) // 2s
      expect(delays[1]).toBe(4000) // 4s
      expect(delays[2]).toBe(8000) // 8s
      expect(delays[3]).toBe(16000) // 16s
      expect(delays[4]).toBe(30000) // Capped at 30s
    })

    test("should identify timeout errors as retryable", () => {
      const timeoutError = new MessageV2.APIError({
        message: "Request timeout after 30000ms",
        isRetryable: true,
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(timeoutError)
      expect(retryable).toBeDefined()
    })
  })

  describe("Rate Limiting (429) Handling", () => {
    test("should handle 429 rate limiting with retry-after header", () => {
      const error = new MessageV2.APIError({
        message: "Rate limit exceeded",
        isRetryable: true,
        responseHeaders: { "retry-after": "60" },
      }).toObject() as MessageV2.APIError

      const delay = SessionRetry.delay(1, error)
      expect(delay).toBe(60000) // 60 seconds from header
    })

    test("should handle retry-after-ms header", () => {
      const error = new MessageV2.APIError({
        message: "Rate limit exceeded",
        isRetryable: true,
        responseHeaders: { "retry-after-ms": "5000" },
      }).toObject() as MessageV2.APIError

      const delay = SessionRetry.delay(1, error)
      expect(delay).toBe(5000) // 5000ms from header
    })

    test("should prefer retry-after-ms over retry-after seconds", () => {
      const error = new MessageV2.APIError({
        message: "Rate limit exceeded",
        isRetryable: true,
        responseHeaders: {
          "retry-after": "60",
          "retry-after-ms": "3000",
        },
      }).toObject() as MessageV2.APIError

      const delay = SessionRetry.delay(1, error)
      // Should use ms header since it's more precise
      expect(delay).toBe(3000)
    })

    test("should use exponential backoff when retry-after header is missing", () => {
      const error = new MessageV2.APIError({
        message: "Rate limit exceeded",
        isRetryable: true,
      }).toObject() as MessageV2.APIError

      const delays = Array.from({ length: 3 }, (_, i) => SessionRetry.delay(i + 1, error))
      expect(delays).toEqual([2000, 4000, 8000])
    })
  })

  describe("Temporary Network Failure Recovery", () => {
    test("should handle ECONNRESET as retryable", () => {
      const error = new MessageV2.APIError({
        message: "Connection reset by server",
        isRetryable: true,
        metadata: { code: "ECONNRESET" },
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeDefined()
      expect(retryable).toBe("Connection reset by server")
    })

    test("should handle ENOTFOUND as retryable", () => {
      const error = new MessageV2.APIError({
        message: "DNS resolution failed",
        isRetryable: true,
        metadata: { code: "ENOTFOUND" },
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeDefined()
    })

    test("should handle ETIMEDOUT as retryable", () => {
      const error = new MessageV2.APIError({
        message: "Connection timed out",
        isRetryable: true,
        metadata: { code: "ETIMEDOUT" },
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeDefined()
    })
  })

  describe("Auth Failure Error Messages", () => {
    test("should provide clear error message on auth failure (401)", () => {
      const error = new MessageV2.APIError({
        message: "Invalid API key provided",
        isRetryable: false,
        statusCode: 401,
      }).toObject() as MessageV2.APIError

      // Auth errors should not be retryable
      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeUndefined()

      // Error message should be clear
      expect(error.data.message).toContain("Invalid")
    })

    test("should not retry on authentication errors", () => {
      const error = new MessageV2.APIError({
        message: "Authentication failed",
        isRetryable: false,
        statusCode: 401,
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeUndefined()
    })

    test("should not retry on forbidden errors (403)", () => {
      const error = new MessageV2.APIError({
        message: "Access denied",
        isRetryable: false,
        statusCode: 403,
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeUndefined()
    })
  })

  describe("Provider Unreachable Handling", () => {
    test("should handle 502 bad gateway as retryable", () => {
      const error = new MessageV2.APIError({
        message: "Bad Gateway",
        isRetryable: true,
        statusCode: 502,
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeDefined()
    })

    test("should handle 503 service unavailable as retryable", () => {
      const error = new MessageV2.APIError({
        message: "Service temporarily unavailable",
        isRetryable: true,
        statusCode: 503,
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeDefined()
    })

    test("should handle 504 gateway timeout as retryable", () => {
      const error = new MessageV2.APIError({
        message: "Gateway timeout",
        isRetryable: true,
        statusCode: 504,
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeDefined()
    })
  })

  describe("Retry Delay Bounds", () => {
    test("should cap delay at 30 seconds without headers", () => {
      const error = new MessageV2.APIError({
        message: "Server error",
        isRetryable: true,
      }).toObject() as MessageV2.APIError

      // After many retries, delay should be capped
      const delays = Array.from({ length: 10 }, (_, i) => SessionRetry.delay(i + 1, error))
      expect(delays.every((d) => d <= 30000)).toBe(true)
    })

    test("should handle http-date retry-after format", () => {
      const futureDate = new Date(Date.now() + 10000).toUTCString()
      const error = new MessageV2.APIError({
        message: "Rate limit",
        isRetryable: true,
        responseHeaders: { "retry-after": futureDate },
      }).toObject() as MessageV2.APIError

      const delay = SessionRetry.delay(1, error)
      expect(delay).toBeGreaterThan(8000)
      expect(delay).toBeLessThanOrEqual(10000)
    })

    test("should ignore past date retry-after", () => {
      const pastDate = new Date(Date.now() - 5000).toUTCString()
      const error = new MessageV2.APIError({
        message: "Rate limit",
        isRetryable: true,
        responseHeaders: { "retry-after": pastDate },
      }).toObject() as MessageV2.APIError

      const delay = SessionRetry.delay(1, error)
      // Should fall back to exponential backoff
      expect(delay).toBe(2000)
    })

    test("should ignore invalid retry-after values", () => {
      const error = new MessageV2.APIError({
        message: "Rate limit",
        isRetryable: true,
        responseHeaders: { "retry-after": "not-a-number" },
      }).toObject() as MessageV2.APIError

      const delay = SessionRetry.delay(1, error)
      // Should fall back to exponential backoff
      expect(delay).toBe(2000)
    })
  })

  describe("Sleep Function", () => {
    test("should be abortable", async () => {
      const controller = new AbortController()
      const start = performance.now()

      // Abort after 50ms
      setTimeout(() => controller.abort(), 50)

      await SessionRetry.sleep(10000, controller.signal).catch(() => {})

      const elapsed = performance.now() - start
      expect(elapsed).toBeLessThan(200) // Should abort quickly
    })

    test("should handle large delay values without overflow warning", async () => {
      const controller = new AbortController()

      const warnings: string[] = []
      const originalWarn = process.emitWarning
      process.emitWarning = (warning: string | Error) => {
        warnings.push(typeof warning === "string" ? warning : warning.message)
      }

      // Large delay that would overflow 32-bit integer
      const promise = SessionRetry.sleep(2_560_914_000, controller.signal)
      controller.abort()

      try {
        await promise
      } catch {}

      process.emitWarning = originalWarn
      expect(warnings.some((w) => w.includes("TimeoutOverflowWarning"))).toBe(false)
    })
  })

  describe("Error Classification", () => {
    test("should classify APIError correctly", () => {
      const apiError = new MessageV2.APIError({
        message: "API Error",
        isRetryable: true,
      })

      expect(MessageV2.APIError.isInstance(apiError.toObject())).toBe(true)
    })

    test("should handle non-retryable errors", () => {
      const error = new MessageV2.APIError({
        message: "Bad request",
        isRetryable: false,
        statusCode: 400,
      }).toObject() as MessageV2.APIError

      const retryable = SessionRetry.retryable(error)
      expect(retryable).toBeUndefined()
    })

    test("should convert socket errors to retryable APIError", () => {
      const socketError = {
        code: "ECONNRESET",
        message: "socket connection was closed unexpectedly",
      }

      const result = MessageV2.fromError(socketError, { providerID: "anthropic" })

      expect(MessageV2.APIError.isInstance(result)).toBe(true)
      expect((result as MessageV2.APIError).data.isRetryable).toBe(true)
    })
  })
})
