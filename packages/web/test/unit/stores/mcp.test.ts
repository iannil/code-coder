import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useMcpStore,
  useMcpStatus,
  useMcpTools,
  useMcpResources,
  useMcpLoading,
} from "@/stores/mcp"
import { api } from "@/lib/api"
import type { McpStatus, McpTool, McpResource } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    getMcpStatus: vi.fn(),
    getMcpTools: vi.fn(),
    getMcpResources: vi.fn(),
    toggleMcp: vi.fn(),
    connectMcp: vi.fn(),
    disconnectMcp: vi.fn(),
    getMcpAuthStatus: vi.fn(),
    startMcpAuth: vi.fn(),
    finishMcpAuth: vi.fn(),
  },
}))

const mockStatus: McpStatus = {
  name: "test-server",
  status: "connected",
  error: null,
}

const mockStatusDisabled: McpStatus = {
  name: "disabled-server",
  status: "disabled",
  error: null,
}

const mockStatusFailed: McpStatus = {
  name: "failed-server",
  status: "failed",
  error: "Connection refused",
}

const mockTool: McpTool = {
  name: "test-tool",
  description: "A test tool",
  server: "test-server",
  inputSchema: {},
}

const mockResource: McpResource = {
  uri: "test://resource",
  name: "Test Resource",
  mimeType: "text/plain",
}

describe("MCP Store", () => {
  beforeEach(() => {
    // Reset the store before each test
    useMcpStore.setState({
      status: {},
      tools: [],
      resources: {},
      isLoading: false,
      isToggling: null,
      error: null,
    })
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty status", () => {
      const state = useMcpStore.getState()
      expect(state.status).toEqual({})
    })

    it("should have empty tools array", () => {
      const state = useMcpStore.getState()
      expect(state.tools).toEqual([])
    })

    it("should have empty resources", () => {
      const state = useMcpStore.getState()
      expect(state.resources).toEqual({})
    })

    it("should have isLoading false", () => {
      const state = useMcpStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isToggling null", () => {
      const state = useMcpStore.getState()
      expect(state.isToggling).toBeNull()
    })

    it("should have null error", () => {
      const state = useMcpStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("fetchStatus", () => {
    it("should load status from API", async () => {
      vi.mocked(api.getMcpStatus).mockResolvedValueOnce({ "test-server": mockStatus })

      await useMcpStore.getState().fetchStatus()

      const state = useMcpStore.getState()
      expect(state.status["test-server"]).toEqual(mockStatus)
    })

    it("should handle API errors", async () => {
      vi.mocked(api.getMcpStatus).mockRejectedValueOnce(new Error("Network error"))

      await useMcpStore.getState().fetchStatus()

      expect(useMcpStore.getState().error).toBe("Network error")
    })
  })

  describe("fetchTools", () => {
    it("should load tools from API", async () => {
      vi.mocked(api.getMcpTools).mockResolvedValueOnce([mockTool])

      await useMcpStore.getState().fetchTools()

      expect(useMcpStore.getState().tools).toEqual([mockTool])
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.getMcpTools).mockRejectedValueOnce(new Error("Failed"))

      await useMcpStore.getState().fetchTools()

      // Should not set error, just console.error
      expect(useMcpStore.getState().error).toBeNull()
    })
  })

  describe("fetchResources", () => {
    it("should load resources from API", async () => {
      vi.mocked(api.getMcpResources).mockResolvedValueOnce({ "test://resource": mockResource })

      await useMcpStore.getState().fetchResources()

      expect(useMcpStore.getState().resources["test://resource"]).toEqual(mockResource)
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.getMcpResources).mockRejectedValueOnce(new Error("Failed"))

      await useMcpStore.getState().fetchResources()

      // Should not set error
      expect(useMcpStore.getState().error).toBeNull()
    })
  })

  describe("fetchAll", () => {
    it("should set isLoading during fetch", async () => {
      vi.mocked(api.getMcpStatus).mockImplementationOnce(async () => {
        expect(useMcpStore.getState().isLoading).toBe(true)
        return {}
      })
      vi.mocked(api.getMcpTools).mockResolvedValueOnce([])
      vi.mocked(api.getMcpResources).mockResolvedValueOnce({})

      await useMcpStore.getState().fetchAll()

      expect(useMcpStore.getState().isLoading).toBe(false)
    })

    it("should handle errors", async () => {
      vi.mocked(api.getMcpStatus).mockRejectedValueOnce(new Error("Failed"))
      vi.mocked(api.getMcpTools).mockResolvedValueOnce([])
      vi.mocked(api.getMcpResources).mockResolvedValueOnce({})

      await useMcpStore.getState().fetchAll()

      expect(useMcpStore.getState().isLoading).toBe(false)
    })
  })

  describe("toggle", () => {
    it("should toggle MCP server", async () => {
      vi.mocked(api.toggleMcp).mockResolvedValueOnce({ status: mockStatusDisabled })
      vi.mocked(api.getMcpTools).mockResolvedValueOnce([])

      await useMcpStore.getState().toggle("disabled-server")

      expect(useMcpStore.getState().status["disabled-server"]).toEqual(mockStatusDisabled)
      expect(useMcpStore.getState().isToggling).toBeNull()
    })

    it("should set isToggling during toggle", async () => {
      vi.mocked(api.toggleMcp).mockImplementationOnce(async () => {
        expect(useMcpStore.getState().isToggling).toBe("test-server")
        return { status: mockStatus }
      })
      vi.mocked(api.getMcpTools).mockResolvedValueOnce([])

      await useMcpStore.getState().toggle("test-server")
    })

    it("should refresh tools when connecting", async () => {
      vi.mocked(api.toggleMcp).mockResolvedValueOnce({ status: mockStatus })
      vi.mocked(api.getMcpTools).mockResolvedValueOnce([mockTool])

      await useMcpStore.getState().toggle("test-server")

      expect(api.getMcpTools).toHaveBeenCalled()
    })

    it("should handle errors", async () => {
      vi.mocked(api.toggleMcp).mockRejectedValueOnce(new Error("Toggle failed"))

      await useMcpStore.getState().toggle("test-server")

      expect(useMcpStore.getState().error).toBe("Toggle failed")
      expect(useMcpStore.getState().isToggling).toBeNull()
    })
  })

  describe("connect", () => {
    it("should connect to MCP server", async () => {
      vi.mocked(api.connectMcp).mockResolvedValueOnce({ status: mockStatus })
      vi.mocked(api.getMcpTools).mockResolvedValueOnce([mockTool])

      await useMcpStore.getState().connect("test-server")

      expect(useMcpStore.getState().status["test-server"]).toEqual(mockStatus)
      expect(api.getMcpTools).toHaveBeenCalled()
    })

    it("should handle errors", async () => {
      vi.mocked(api.connectMcp).mockRejectedValueOnce(new Error("Connect failed"))

      await useMcpStore.getState().connect("test-server")

      expect(useMcpStore.getState().error).toBe("Connect failed")
    })
  })

  describe("disconnect", () => {
    it("should disconnect from MCP server", async () => {
      vi.mocked(api.disconnectMcp).mockResolvedValueOnce({ status: mockStatusDisabled })

      await useMcpStore.getState().disconnect("disabled-server")

      expect(useMcpStore.getState().status["disabled-server"]).toEqual(mockStatusDisabled)
    })

    it("should handle errors", async () => {
      vi.mocked(api.disconnectMcp).mockRejectedValueOnce(new Error("Disconnect failed"))

      await useMcpStore.getState().disconnect("test-server")

      expect(useMcpStore.getState().error).toBe("Disconnect failed")
    })
  })

  describe("auth operations", () => {
    it("getAuthStatus should call API", async () => {
      const authStatus = { authenticated: true, needsAuth: false }
      vi.mocked(api.getMcpAuthStatus).mockResolvedValueOnce(authStatus)

      const result = await useMcpStore.getState().getAuthStatus("test-server")

      expect(result).toEqual(authStatus)
      expect(api.getMcpAuthStatus).toHaveBeenCalledWith("test-server")
    })

    it("startAuth should call API", async () => {
      const authResult = { authorizationUrl: "https://auth.example.com" }
      vi.mocked(api.startMcpAuth).mockResolvedValueOnce(authResult)

      const result = await useMcpStore.getState().startAuth("test-server")

      expect(result).toEqual(authResult)
      expect(api.startMcpAuth).toHaveBeenCalledWith("test-server")
    })

    it("finishAuth should update status", async () => {
      vi.mocked(api.finishMcpAuth).mockResolvedValueOnce({ status: mockStatus })

      await useMcpStore.getState().finishAuth("test-server", "auth-code")

      expect(api.finishMcpAuth).toHaveBeenCalledWith("test-server", "auth-code")
      expect(useMcpStore.getState().status["test-server"]).toEqual(mockStatus)
    })
  })

  describe("helpers", () => {
    it("isEnabled should return true for connected servers", () => {
      useMcpStore.setState({ status: { "test-server": mockStatus } })

      expect(useMcpStore.getState().isEnabled("test-server")).toBe(true)
    })

    it("isEnabled should return false for disabled servers", () => {
      useMcpStore.setState({ status: { "disabled-server": mockStatusDisabled } })

      expect(useMcpStore.getState().isEnabled("disabled-server")).toBe(false)
    })

    it("getStatusLabel should return correct label for connected", () => {
      useMcpStore.setState({ status: { "test-server": mockStatus } })

      expect(useMcpStore.getState().getStatusLabel("test-server")).toBe("Connected")
    })

    it("getStatusLabel should return correct label for disabled", () => {
      useMcpStore.setState({ status: { "disabled-server": mockStatusDisabled } })

      expect(useMcpStore.getState().getStatusLabel("disabled-server")).toBe("Disabled")
    })

    it("getStatusLabel should return correct label for failed", () => {
      useMcpStore.setState({ status: { "failed-server": mockStatusFailed } })

      expect(useMcpStore.getState().getStatusLabel("failed-server")).toBe("Failed: Connection refused")
    })

    it("getStatusLabel should return Unknown for unknown server", () => {
      expect(useMcpStore.getState().getStatusLabel("unknown")).toBe("Unknown")
    })

    it("getStatusLabel should return correct label for needs_auth", () => {
      useMcpStore.setState({
        status: { "auth-server": { name: "auth-server", status: "needs_auth", error: null } },
      })

      expect(useMcpStore.getState().getStatusLabel("auth-server")).toBe("Needs Authentication")
    })

    it("getStatusLabel should return correct label for needs_client_registration", () => {
      useMcpStore.setState({
        status: { "reg-server": { name: "reg-server", status: "needs_client_registration", error: null } },
      })

      expect(useMcpStore.getState().getStatusLabel("reg-server")).toBe("Needs Client Registration")
    })
  })

  describe("hooks", () => {
    it("useMcpStatus should return status", () => {
      useMcpStore.setState({ status: { "test-server": mockStatus } })

      const { result } = renderHook(() => useMcpStatus())
      expect(result.current["test-server"]).toEqual(mockStatus)
    })

    it("useMcpTools should return tools", () => {
      useMcpStore.setState({ tools: [mockTool] })

      const { result } = renderHook(() => useMcpTools())
      expect(result.current).toEqual([mockTool])
    })

    it("useMcpResources should return resources", () => {
      useMcpStore.setState({ resources: { "test://resource": mockResource } })

      const { result } = renderHook(() => useMcpResources())
      expect(result.current["test://resource"]).toEqual(mockResource)
    })

    it("useMcpLoading should return loading states", () => {
      useMcpStore.setState({
        isLoading: true,
        isToggling: "test-server",
        error: "Test error",
      })

      const { result } = renderHook(() => useMcpLoading())
      expect(result.current.isLoading).toBe(true)
      expect(result.current.isToggling).toBe("test-server")
      expect(result.current.error).toBe("Test error")
    })
  })
})
