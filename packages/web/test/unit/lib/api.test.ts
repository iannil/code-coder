import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { ApiClient, ApiError, NetworkError, TimeoutError, api, getClient, setDefaultClient } from "@/lib/api"

describe("ApiClient", () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("constructor", () => {
    it("should create client with default config", () => {
      const client = new ApiClient()
      expect(client).toBeDefined()
    })

    it("should create client with custom config", () => {
      const client = new ApiClient({
        baseUrl: "http://localhost:8080",
        apiKey: "test-key",
        timeout: 5000,
      })
      expect(client).toBeDefined()
    })
  })

  describe("health", () => {
    it("should call health endpoint", async () => {
      const mockResponse = { status: "ok", timestamp: Date.now() }
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ({ data: mockResponse }),
      } as Response)

      const client = new ApiClient({ baseUrl: "/api" })
      const result = await client.health()

      expect(fetch).toHaveBeenCalledWith(
        "/api/health",
        expect.objectContaining({
          method: "GET",
          headers: expect.objectContaining({
            "Content-Type": "application/json",
          }),
        })
      )
      expect(result).toEqual(mockResponse)
    })
  })

  describe("listSessions", () => {
    it("should list sessions without query params", async () => {
      const mockSessions = [{ id: "1", title: "Test Session" }]
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ({ data: mockSessions }),
      } as Response)

      const client = new ApiClient({ baseUrl: "/api" })
      const result = await client.listSessions()

      expect(fetch).toHaveBeenCalledWith(
        "/api/sessions",
        expect.objectContaining({ method: "GET" })
      )
      expect(result).toEqual(mockSessions)
    })

    it("should list sessions with query params", async () => {
      const mockSessions = [{ id: "1", title: "Test Session" }]
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ({ data: mockSessions }),
      } as Response)

      const client = new ApiClient({ baseUrl: "/api" })
      const result = await client.listSessions({ limit: 10, search: "test" })

      expect(fetch).toHaveBeenCalledWith(
        "/api/sessions?limit=10&search=test",
        expect.objectContaining({ method: "GET" })
      )
      expect(result).toEqual(mockSessions)
    })
  })

  describe("createSession", () => {
    it("should create a session", async () => {
      const mockSession = { id: "new-id", title: "New Session" }
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: true,
        status: 201,
        json: async () => ({ data: mockSession }),
      } as Response)

      const client = new ApiClient({ baseUrl: "/api" })
      const result = await client.createSession({ title: "New Session" })

      expect(fetch).toHaveBeenCalledWith(
        "/api/sessions",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ title: "New Session" }),
        })
      )
      expect(result).toEqual(mockSession)
    })
  })

  describe("error handling", () => {
    it("should throw ApiError for HTTP errors", async () => {
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: false,
        status: 404,
        statusText: "Not Found",
        json: async () => ({ error: "Session not found" }),
      } as Response)

      const client = new ApiClient({ baseUrl: "/api" })
      await expect(client.getSession("non-existent")).rejects.toThrow(ApiError)
    })

    it("should throw TimeoutError when request times out", async () => {
      vi.mocked(fetch).mockImplementationOnce(() => {
        return new Promise((_, reject) => {
          const error = new Error("Aborted")
          error.name = "AbortError"
          reject(error)
        })
      })

      const client = new ApiClient({ baseUrl: "/api", timeout: 100 })
      await expect(client.health()).rejects.toThrow(TimeoutError)
    })

    it("should throw NetworkError for network failures", async () => {
      vi.mocked(fetch).mockRejectedValueOnce(new Error("Network error"))

      const client = new ApiClient({ baseUrl: "/api" })
      await expect(client.health()).rejects.toThrow(NetworkError)
    })

    it("should handle 204 No Content response", async () => {
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: true,
        status: 204,
      } as Response)

      const client = new ApiClient({ baseUrl: "/api" })
      const result = await client.deleteSession("test-id")

      expect(result).toBeUndefined()
    })
  })

  describe("API key handling", () => {
    it("should include Authorization header when API key is set", async () => {
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ({ data: {} }),
      } as Response)

      const client = new ApiClient({ baseUrl: "/api", apiKey: "test-key" })
      await client.health()

      expect(fetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({
            Authorization: "Bearer test-key",
            "X-API-Key": "test-key",
          }),
        })
      )
    })
  })
})

describe("Error Classes", () => {
  describe("ApiError", () => {
    it("should create ApiError with all properties", () => {
      const error = new ApiError(404, "NOT_FOUND", "Resource not found", { id: "123" })
      expect(error.statusCode).toBe(404)
      expect(error.code).toBe("NOT_FOUND")
      expect(error.message).toBe("Resource not found")
      expect(error.details).toEqual({ id: "123" })
      expect(error.name).toBe("ApiError")
    })
  })

  describe("NetworkError", () => {
    it("should create NetworkError with cause", () => {
      const cause = new Error("Connection refused")
      const error = new NetworkError("Failed to connect", cause)
      expect(error.message).toBe("Failed to connect")
      expect(error.cause).toBe(cause)
      expect(error.name).toBe("NetworkError")
    })
  })

  describe("TimeoutError", () => {
    it("should create TimeoutError with timeout value", () => {
      const error = new TimeoutError(5000)
      expect(error.message).toBe("Request timed out after 5000ms")
      expect(error.name).toBe("TimeoutError")
    })
  })
})

describe("Default Client", () => {
  it("should create default client on first call", () => {
    const client = getClient()
    expect(client).toBeDefined()
  })

  it("should set custom default client", () => {
    setDefaultClient({ baseUrl: "http://custom.api" })
    const client = getClient()
    expect(client).toBeDefined()
  })
})

describe("api convenience object", () => {
  it("should have all expected methods", () => {
    expect(api.health).toBeDefined()
    expect(api.discover).toBeDefined()
    expect(api.listSessions).toBeDefined()
    expect(api.getSession).toBeDefined()
    expect(api.createSession).toBeDefined()
    expect(api.deleteSession).toBeDefined()
    expect(api.getSessionMessages).toBeDefined()
    expect(api.sendMessage).toBeDefined()
    expect(api.getConfig).toBeDefined()
    expect(api.updateConfig).toBeDefined()
    expect(api.listProviders).toBeDefined()
    expect(api.getMcpStatus).toBeDefined()
  })
})
