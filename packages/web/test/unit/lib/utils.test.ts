import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { cn, formatTimestamp, formatDate, formatTime, formatRelativeTime } from "@/lib/utils"

describe("cn (className utility)", () => {
  it("should merge class names", () => {
    expect(cn("foo", "bar")).toBe("foo bar")
  })

  it("should handle conditional classes", () => {
    expect(cn("foo", false && "bar", "baz")).toBe("foo baz")
  })

  it("should handle arrays", () => {
    expect(cn(["foo", "bar"])).toBe("foo bar")
  })

  it("should handle objects", () => {
    expect(cn({ foo: true, bar: false, baz: true })).toBe("foo baz")
  })

  it("should merge Tailwind classes correctly", () => {
    expect(cn("px-2 py-1", "px-4")).toBe("py-1 px-4")
  })

  it("should handle undefined and null", () => {
    expect(cn("foo", undefined, null, "bar")).toBe("foo bar")
  })

  it("should handle empty string", () => {
    expect(cn("", "foo", "")).toBe("foo")
  })
})

describe("formatTimestamp", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("should return 'just now' for timestamps less than 1 minute ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatTimestamp(now - 30000)).toBe("just now")
  })

  it("should return minutes ago for timestamps less than 1 hour ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatTimestamp(now - 5 * 60000)).toBe("5m ago")
    expect(formatTimestamp(now - 30 * 60000)).toBe("30m ago")
  })

  it("should return hours ago for timestamps less than 24 hours ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatTimestamp(now - 2 * 3600000)).toBe("2h ago")
    expect(formatTimestamp(now - 12 * 3600000)).toBe("12h ago")
  })

  it("should return days ago for timestamps less than 7 days ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatTimestamp(now - 2 * 86400000)).toBe("2d ago")
    expect(formatTimestamp(now - 5 * 86400000)).toBe("5d ago")
  })

  it("should return formatted date for timestamps more than 7 days ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    const oldDate = now - 10 * 86400000
    const result = formatTimestamp(oldDate)
    // Result should be a date string (format varies by locale)
    expect(result).not.toBe("10d ago")
    expect(result).toBeTruthy()
  })
})

describe("formatDate", () => {
  it("should return a formatted date string", () => {
    const timestamp = new Date("2024-01-15T12:00:00Z").getTime()
    const result = formatDate(timestamp)
    // Result varies by locale, but should contain date parts
    expect(result).toBeTruthy()
    expect(typeof result).toBe("string")
  })

  it("should handle current date", () => {
    const now = Date.now()
    const result = formatDate(now)
    expect(result).toBeTruthy()
  })
})

describe("formatTime", () => {
  it("should return a formatted time string", () => {
    const timestamp = new Date("2024-01-15T14:30:00Z").getTime()
    const result = formatTime(timestamp)
    // Result varies by locale, but should be a time string
    expect(result).toBeTruthy()
    expect(typeof result).toBe("string")
  })

  it("should handle midnight", () => {
    const timestamp = new Date("2024-01-15T00:00:00Z").getTime()
    const result = formatTime(timestamp)
    expect(result).toBeTruthy()
  })
})

describe("formatRelativeTime", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("should return 'Just now' for timestamps less than 1 minute ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatRelativeTime(now - 30000)).toBe("Just now")
  })

  it("should return singular minute for exactly 1 minute ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatRelativeTime(now - 60000)).toBe("1 minute ago")
  })

  it("should return plural minutes for multiple minutes ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatRelativeTime(now - 5 * 60000)).toBe("5 minutes ago")
    expect(formatRelativeTime(now - 30 * 60000)).toBe("30 minutes ago")
  })

  it("should return singular hour for exactly 1 hour ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatRelativeTime(now - 3600000)).toBe("1 hour ago")
  })

  it("should return plural hours for multiple hours ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatRelativeTime(now - 2 * 3600000)).toBe("2 hours ago")
    expect(formatRelativeTime(now - 12 * 3600000)).toBe("12 hours ago")
  })

  it("should return singular day for exactly 1 day ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatRelativeTime(now - 86400000)).toBe("1 day ago")
  })

  it("should return plural days for multiple days ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    expect(formatRelativeTime(now - 2 * 86400000)).toBe("2 days ago")
    expect(formatRelativeTime(now - 5 * 86400000)).toBe("5 days ago")
  })

  it("should return 'on <date>' for timestamps more than 7 days ago", () => {
    const now = Date.now()
    vi.setSystemTime(now)
    const result = formatRelativeTime(now - 10 * 86400000)
    expect(result.startsWith("on ")).toBe(true)
  })
})
