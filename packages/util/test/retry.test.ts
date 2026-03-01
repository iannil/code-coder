import { describe, expect, it } from "bun:test"
import { retry } from "../src/retry"

describe("retry", () => {
  it("should return result on first success", async () => {
    let attempts = 0
    const result = await retry(async () => {
      attempts++
      return "success"
    })
    expect(result).toBe("success")
    expect(attempts).toBe(1)
  })

  it("should retry on transient errors", async () => {
    let attempts = 0
    const result = await retry(
      async () => {
        attempts++
        if (attempts < 3) {
          throw new Error("ECONNRESET")
        }
        return "success"
      },
      { delay: 10 }
    )
    expect(result).toBe("success")
    expect(attempts).toBe(3)
  })

  it("should throw after max attempts", async () => {
    let attempts = 0
    await expect(
      retry(
        async () => {
          attempts++
          throw new Error("ECONNRESET")
        },
        { attempts: 3, delay: 10 }
      )
    ).rejects.toThrow("ECONNRESET")
    expect(attempts).toBe(3)
  })

  it("should throw immediately for non-transient errors", async () => {
    let attempts = 0
    await expect(
      retry(
        async () => {
          attempts++
          throw new Error("Invalid input")
        },
        { attempts: 3, delay: 10 }
      )
    ).rejects.toThrow("Invalid input")
    expect(attempts).toBe(1)
  })

  it("should use custom retryIf predicate", async () => {
    let attempts = 0
    const result = await retry(
      async () => {
        attempts++
        if (attempts < 2) {
          throw new Error("Custom error")
        }
        return "success"
      },
      {
        delay: 10,
        retryIf: (error) => error instanceof Error && error.message === "Custom error",
      }
    )
    expect(result).toBe("success")
    expect(attempts).toBe(2)
  })

  it("should apply exponential backoff", async () => {
    const delays: number[] = []
    const startTime = Date.now()
    let lastTime = startTime

    await retry(
      async () => {
        const now = Date.now()
        if (delays.length < 3) {
          delays.push(now - lastTime)
          lastTime = now
          throw new Error("network request failed")
        }
        return "success"
      },
      { delay: 50, factor: 2, attempts: 5 }
    )

    // First delay should be ~50ms, second ~100ms
    expect(delays[1]!).toBeGreaterThan(40)
    expect(delays[2]!).toBeGreaterThan(80)
  })

  it("should respect maxDelay", async () => {
    let attempts = 0
    const startTime = Date.now()

    await retry(
      async () => {
        attempts++
        if (attempts < 3) {
          throw new Error("network request failed")
        }
        return "success"
      },
      { delay: 1000, factor: 10, maxDelay: 100, attempts: 5 }
    )

    const elapsed = Date.now() - startTime
    // With maxDelay=100, total should be around 200ms (2 retries x 100ms)
    expect(elapsed).toBeLessThan(500)
  })
})
