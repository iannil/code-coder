import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useLspStore,
  useLspServers,
  useLspStatusLoading,
  useLspDiagnostics,
  useLspConfig,
  useHoverContent,
  useDefinitions,
  useReferences,
  useWorkspaceSymbols,
  useDocumentSymbols,
  useConnectedServersCount,
  useErrorServersCount,
  useTotalDiagnostics,
} from "@/stores/lsp"
import { api } from "@/lib/api"
import type { LspStatus, LspFileDiagnostics, LspConfig, LspSymbol, LspDocumentSymbol, LspLocation } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    getLspStatus: vi.fn(),
    getLspDiagnostics: vi.fn(),
    getLspConfig: vi.fn(),
    initLsp: vi.fn(),
    touchLspFile: vi.fn(),
    getLspHover: vi.fn(),
    getLspDefinition: vi.fn(),
    getLspReferences: vi.fn(),
    getLspWorkspaceSymbols: vi.fn(),
    getLspDocumentSymbols: vi.fn(),
    checkLspAvailable: vi.fn(),
  },
}))

const mockServerConnected: LspStatus = {
  name: "typescript",
  status: "connected",
  languageId: "typescript",
}

const mockServerError: LspStatus = {
  name: "python",
  status: "error",
  languageId: "python",
  error: "Server crashed",
}

const mockDiagnostics: LspFileDiagnostics = {
  filePath: "/src/index.ts",
  diagnostics: [
    { range: { start: { line: 1, character: 0 }, end: { line: 1, character: 10 } }, message: "Error", severity: 1 },
    { range: { start: { line: 5, character: 0 }, end: { line: 5, character: 10 } }, message: "Warning", severity: 2 },
  ],
}

const mockConfig: LspConfig = {
  servers: {
    typescript: { enabled: true },
    python: { enabled: false },
  },
}

const mockSymbol: LspSymbol = {
  name: "TestClass",
  kind: 5,
  location: {
    uri: "file:///src/test.ts",
    range: { start: { line: 0, character: 0 }, end: { line: 10, character: 0 } },
  },
}

const mockDocSymbol: LspDocumentSymbol = {
  name: "myFunction",
  kind: 12,
  range: { start: { line: 5, character: 0 }, end: { line: 15, character: 0 } },
  selectionRange: { start: { line: 5, character: 9 }, end: { line: 5, character: 19 } },
}

const mockLocation: LspLocation = {
  uri: "file:///src/index.ts",
  range: { start: { line: 10, character: 5 }, end: { line: 10, character: 15 } },
}

describe("LSP Store", () => {
  beforeEach(() => {
    useLspStore.getState().reset()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty servers array", () => {
      const state = useLspStore.getState()
      expect(state.servers).toEqual([])
    })

    it("should have statusLoading false", () => {
      const state = useLspStore.getState()
      expect(state.statusLoading).toBe(false)
    })

    it("should have null statusError", () => {
      const state = useLspStore.getState()
      expect(state.statusError).toBeNull()
    })

    it("should have empty diagnostics", () => {
      const state = useLspStore.getState()
      expect(state.diagnostics).toEqual([])
    })

    it("should have null config", () => {
      const state = useLspStore.getState()
      expect(state.config).toBeNull()
    })

    it("should have null hoverContent", () => {
      const state = useLspStore.getState()
      expect(state.hoverContent).toBeNull()
    })

    it("should have empty definitions", () => {
      const state = useLspStore.getState()
      expect(state.definitions).toEqual([])
    })
  })

  describe("fetchStatus", () => {
    it("should load servers from API", async () => {
      vi.mocked(api.getLspStatus).mockResolvedValueOnce([mockServerConnected])

      await useLspStore.getState().fetchStatus()

      const state = useLspStore.getState()
      expect(state.servers).toEqual([mockServerConnected])
      expect(state.statusLoading).toBe(false)
    })

    it("should set statusLoading during fetch", async () => {
      vi.mocked(api.getLspStatus).mockImplementationOnce(async () => {
        expect(useLspStore.getState().statusLoading).toBe(true)
        return [mockServerConnected]
      })

      await useLspStore.getState().fetchStatus()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.getLspStatus).mockRejectedValueOnce(new Error("Network error"))

      await useLspStore.getState().fetchStatus()

      const state = useLspStore.getState()
      expect(state.statusError).toBe("Network error")
      expect(state.statusLoading).toBe(false)
    })
  })

  describe("fetchDiagnostics", () => {
    it("should load diagnostics from API", async () => {
      vi.mocked(api.getLspDiagnostics).mockResolvedValueOnce([mockDiagnostics])

      await useLspStore.getState().fetchDiagnostics()

      const state = useLspStore.getState()
      expect(state.diagnostics).toEqual([mockDiagnostics])
      expect(state.diagnosticsLoading).toBe(false)
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.getLspDiagnostics).mockRejectedValueOnce(new Error("Failed"))

      await useLspStore.getState().fetchDiagnostics()

      expect(useLspStore.getState().diagnosticsLoading).toBe(false)
    })
  })

  describe("fetchConfig", () => {
    it("should load config from API", async () => {
      vi.mocked(api.getLspConfig).mockResolvedValueOnce(mockConfig)

      await useLspStore.getState().fetchConfig()

      expect(useLspStore.getState().config).toEqual(mockConfig)
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.getLspConfig).mockRejectedValueOnce(new Error("Failed"))

      await useLspStore.getState().fetchConfig()

      expect(useLspStore.getState().configLoading).toBe(false)
    })
  })

  describe("initLsp", () => {
    it("should initialize LSP and fetch status", async () => {
      vi.mocked(api.initLsp).mockResolvedValueOnce(undefined)
      vi.mocked(api.getLspStatus).mockResolvedValueOnce([mockServerConnected])

      await useLspStore.getState().initLsp()

      expect(api.initLsp).toHaveBeenCalled()
      expect(useLspStore.getState().servers).toEqual([mockServerConnected])
    })

    it("should handle errors", async () => {
      vi.mocked(api.initLsp).mockRejectedValueOnce(new Error("Init failed"))

      await useLspStore.getState().initLsp()

      expect(useLspStore.getState().statusError).toBe("Init failed")
    })
  })

  describe("touchFile", () => {
    it("should call API", async () => {
      vi.mocked(api.touchLspFile).mockResolvedValueOnce(undefined)

      await useLspStore.getState().touchFile("/src/test.ts")

      expect(api.touchLspFile).toHaveBeenCalledWith("/src/test.ts")
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.touchLspFile).mockRejectedValueOnce(new Error("Failed"))

      // Should not throw
      await useLspStore.getState().touchFile("/src/test.ts")
    })
  })

  describe("getHover", () => {
    it("should return hover content", async () => {
      const hoverContent = [{ contents: "Test content" }]
      vi.mocked(api.getLspHover).mockResolvedValueOnce(hoverContent)

      const result = await useLspStore.getState().getHover("/src/test.ts", 10, 5)

      expect(result).toEqual(hoverContent)
      expect(useLspStore.getState().hoverContent).toEqual(hoverContent)
    })

    it("should handle errors", async () => {
      vi.mocked(api.getLspHover).mockRejectedValueOnce(new Error("Failed"))

      const result = await useLspStore.getState().getHover("/src/test.ts", 10, 5)

      expect(result).toBeNull()
    })
  })

  describe("getDefinition", () => {
    it("should return definitions", async () => {
      vi.mocked(api.getLspDefinition).mockResolvedValueOnce([mockLocation])

      const result = await useLspStore.getState().getDefinition("/src/test.ts", 10, 5)

      expect(result).toEqual([mockLocation])
      expect(useLspStore.getState().definitions).toEqual([mockLocation])
    })

    it("should handle errors", async () => {
      vi.mocked(api.getLspDefinition).mockRejectedValueOnce(new Error("Failed"))

      const result = await useLspStore.getState().getDefinition("/src/test.ts", 10, 5)

      expect(result).toEqual([])
    })
  })

  describe("getReferences", () => {
    it("should return references", async () => {
      vi.mocked(api.getLspReferences).mockResolvedValueOnce([mockLocation])

      const result = await useLspStore.getState().getReferences("/src/test.ts", 10, 5)

      expect(result).toEqual([mockLocation])
      expect(useLspStore.getState().references).toEqual([mockLocation])
    })

    it("should handle errors", async () => {
      vi.mocked(api.getLspReferences).mockRejectedValueOnce(new Error("Failed"))

      const result = await useLspStore.getState().getReferences("/src/test.ts", 10, 5)

      expect(result).toEqual([])
    })
  })

  describe("searchWorkspaceSymbols", () => {
    it("should return symbols", async () => {
      vi.mocked(api.getLspWorkspaceSymbols).mockResolvedValueOnce([mockSymbol])

      const result = await useLspStore.getState().searchWorkspaceSymbols("Test")

      expect(result).toEqual([mockSymbol])
      expect(useLspStore.getState().workspaceSymbols).toEqual([mockSymbol])
    })

    it("should handle errors", async () => {
      vi.mocked(api.getLspWorkspaceSymbols).mockRejectedValueOnce(new Error("Failed"))

      const result = await useLspStore.getState().searchWorkspaceSymbols("Test")

      expect(result).toEqual([])
    })
  })

  describe("getDocumentSymbols", () => {
    it("should return document symbols", async () => {
      vi.mocked(api.getLspDocumentSymbols).mockResolvedValueOnce([mockDocSymbol])

      const result = await useLspStore.getState().getDocumentSymbols("/src/test.ts")

      expect(result).toEqual([mockDocSymbol])
      expect(useLspStore.getState().documentSymbols).toEqual([mockDocSymbol])
    })

    it("should handle errors", async () => {
      vi.mocked(api.getLspDocumentSymbols).mockRejectedValueOnce(new Error("Failed"))

      const result = await useLspStore.getState().getDocumentSymbols("/src/test.ts")

      expect(result).toEqual([])
    })
  })

  describe("checkAvailable", () => {
    it("should return true when LSP is available", async () => {
      vi.mocked(api.checkLspAvailable).mockResolvedValueOnce({ available: true })

      const result = await useLspStore.getState().checkAvailable("/src/test.ts")

      expect(result).toBe(true)
    })

    it("should return false when LSP is not available", async () => {
      vi.mocked(api.checkLspAvailable).mockResolvedValueOnce({ available: false })

      const result = await useLspStore.getState().checkAvailable("/src/test.ts")

      expect(result).toBe(false)
    })

    it("should return false on error", async () => {
      vi.mocked(api.checkLspAvailable).mockRejectedValueOnce(new Error("Failed"))

      const result = await useLspStore.getState().checkAvailable("/src/test.ts")

      expect(result).toBe(false)
    })
  })

  describe("reset", () => {
    it("should reset to initial state", () => {
      useLspStore.setState({
        servers: [mockServerConnected],
        diagnostics: [mockDiagnostics],
        config: mockConfig,
        hoverContent: [{}],
        definitions: [mockLocation],
      })

      useLspStore.getState().reset()

      const state = useLspStore.getState()
      expect(state.servers).toEqual([])
      expect(state.diagnostics).toEqual([])
      expect(state.config).toBeNull()
      expect(state.hoverContent).toBeNull()
      expect(state.definitions).toEqual([])
    })
  })

  describe("hooks", () => {
    it("useLspServers should return servers", () => {
      useLspStore.setState({ servers: [mockServerConnected, mockServerError] })

      const { result } = renderHook(() => useLspServers())
      expect(result.current).toHaveLength(2)
    })

    it("useLspStatusLoading should return loading state", () => {
      useLspStore.setState({ statusLoading: true })

      const { result } = renderHook(() => useLspStatusLoading())
      expect(result.current).toBe(true)
    })

    it("useLspDiagnostics should return diagnostics", () => {
      useLspStore.setState({ diagnostics: [mockDiagnostics] })

      const { result } = renderHook(() => useLspDiagnostics())
      expect(result.current).toEqual([mockDiagnostics])
    })

    it("useLspConfig should return config", () => {
      useLspStore.setState({ config: mockConfig })

      const { result } = renderHook(() => useLspConfig())
      expect(result.current).toEqual(mockConfig)
    })

    it("useHoverContent should return hover content", () => {
      useLspStore.setState({ hoverContent: [{ contents: "Test" }] })

      const { result } = renderHook(() => useHoverContent())
      expect(result.current).toEqual([{ contents: "Test" }])
    })

    it("useDefinitions should return definitions", () => {
      useLspStore.setState({ definitions: [mockLocation] })

      const { result } = renderHook(() => useDefinitions())
      expect(result.current).toEqual([mockLocation])
    })

    it("useReferences should return references", () => {
      useLspStore.setState({ references: [mockLocation] })

      const { result } = renderHook(() => useReferences())
      expect(result.current).toEqual([mockLocation])
    })

    it("useWorkspaceSymbols should return symbols", () => {
      useLspStore.setState({ workspaceSymbols: [mockSymbol] })

      const { result } = renderHook(() => useWorkspaceSymbols())
      expect(result.current).toEqual([mockSymbol])
    })

    it("useDocumentSymbols should return document symbols", () => {
      useLspStore.setState({ documentSymbols: [mockDocSymbol] })

      const { result } = renderHook(() => useDocumentSymbols())
      expect(result.current).toEqual([mockDocSymbol])
    })

    it("useConnectedServersCount should return count", () => {
      useLspStore.setState({ servers: [mockServerConnected, mockServerError] })

      const { result } = renderHook(() => useConnectedServersCount())
      expect(result.current).toBe(1)
    })

    it("useErrorServersCount should return count", () => {
      useLspStore.setState({ servers: [mockServerConnected, mockServerError] })

      const { result } = renderHook(() => useErrorServersCount())
      expect(result.current).toBe(1)
    })

    it("useTotalDiagnostics should return total count", () => {
      useLspStore.setState({ diagnostics: [mockDiagnostics] })

      const { result } = renderHook(() => useTotalDiagnostics())
      expect(result.current).toBe(2)
    })
  })
})
