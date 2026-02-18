/**
 * LSP Store
 *
 * Manages Language Server Protocol state:
 * - LSP server status
 * - Diagnostics
 * - Configuration
 */

import { create } from "zustand"
import { useShallow } from "zustand/react/shallow"
import type {
  LspStatus,
  LspFileDiagnostics,
  LspConfig,
  LspSymbol,
  LspDocumentSymbol,
  LspLocation,
} from "@/lib/types"
import { api } from "@/lib/api"

// ============================================================================
// Types
// ============================================================================

interface LspState {
  // Status
  servers: LspStatus[]
  statusLoading: boolean
  statusError: string | null

  // Diagnostics
  diagnostics: LspFileDiagnostics[]
  diagnosticsLoading: boolean

  // Config
  config: LspConfig | null
  configLoading: boolean

  // Code intelligence results
  hoverContent: unknown[] | null
  definitions: LspLocation[]
  references: LspLocation[]
  workspaceSymbols: LspSymbol[]
  documentSymbols: LspDocumentSymbol[]
  codeIntelLoading: boolean

  // Actions
  fetchStatus: () => Promise<void>
  fetchDiagnostics: () => Promise<void>
  fetchConfig: () => Promise<void>
  initLsp: () => Promise<void>
  touchFile: (filePath: string) => Promise<void>
  getHover: (filePath: string, line: number, character: number) => Promise<unknown[] | null>
  getDefinition: (filePath: string, line: number, character: number) => Promise<LspLocation[]>
  getReferences: (filePath: string, line: number, character: number) => Promise<LspLocation[]>
  searchWorkspaceSymbols: (query: string) => Promise<LspSymbol[]>
  getDocumentSymbols: (filePath: string) => Promise<LspDocumentSymbol[]>
  checkAvailable: (filePath: string) => Promise<boolean>
  reset: () => void
}

// ============================================================================
// Initial State
// ============================================================================

const initialState = {
  servers: [],
  statusLoading: false,
  statusError: null,
  diagnostics: [],
  diagnosticsLoading: false,
  config: null,
  configLoading: false,
  hoverContent: null,
  definitions: [],
  references: [],
  workspaceSymbols: [],
  documentSymbols: [],
  codeIntelLoading: false,
}

// ============================================================================
// Store
// ============================================================================

export const useLspStore = create<LspState>((set) => ({
  ...initialState,

  fetchStatus: async () => {
    set({ statusLoading: true, statusError: null })
    try {
      const servers = await api.getLspStatus()
      set({ servers, statusLoading: false })
    } catch (error) {
      set({
        statusError: error instanceof Error ? error.message : "Failed to fetch LSP status",
        statusLoading: false,
      })
    }
  },

  fetchDiagnostics: async () => {
    set({ diagnosticsLoading: true })
    try {
      const diagnostics = await api.getLspDiagnostics()
      set({ diagnostics, diagnosticsLoading: false })
    } catch {
      set({ diagnosticsLoading: false })
    }
  },

  fetchConfig: async () => {
    set({ configLoading: true })
    try {
      const config = await api.getLspConfig()
      set({ config, configLoading: false })
    } catch {
      set({ configLoading: false })
    }
  },

  initLsp: async () => {
    set({ statusLoading: true })
    try {
      await api.initLsp()
      const servers = await api.getLspStatus()
      set({ servers, statusLoading: false })
    } catch (error) {
      set({
        statusError: error instanceof Error ? error.message : "Failed to initialize LSP",
        statusLoading: false,
      })
    }
  },

  touchFile: async (filePath) => {
    try {
      await api.touchLspFile(filePath)
    } catch {
      // Ignore errors
    }
  },

  getHover: async (filePath, line, character) => {
    set({ codeIntelLoading: true })
    try {
      const result = await api.getLspHover(filePath, line, character)
      set({ hoverContent: result, codeIntelLoading: false })
      return result
    } catch {
      set({ codeIntelLoading: false })
      return null
    }
  },

  getDefinition: async (filePath, line, character) => {
    set({ codeIntelLoading: true })
    try {
      const definitions = await api.getLspDefinition(filePath, line, character)
      set({ definitions, codeIntelLoading: false })
      return definitions
    } catch {
      set({ codeIntelLoading: false })
      return []
    }
  },

  getReferences: async (filePath, line, character) => {
    set({ codeIntelLoading: true })
    try {
      const references = await api.getLspReferences(filePath, line, character)
      set({ references, codeIntelLoading: false })
      return references
    } catch {
      set({ codeIntelLoading: false })
      return []
    }
  },

  searchWorkspaceSymbols: async (query) => {
    set({ codeIntelLoading: true })
    try {
      const symbols = await api.getLspWorkspaceSymbols(query)
      set({ workspaceSymbols: symbols, codeIntelLoading: false })
      return symbols
    } catch {
      set({ codeIntelLoading: false })
      return []
    }
  },

  getDocumentSymbols: async (filePath) => {
    set({ codeIntelLoading: true })
    try {
      const symbols = await api.getLspDocumentSymbols(filePath)
      set({ documentSymbols: symbols, codeIntelLoading: false })
      return symbols
    } catch {
      set({ codeIntelLoading: false })
      return []
    }
  },

  checkAvailable: async (filePath) => {
    try {
      const result = await api.checkLspAvailable(filePath)
      return result.available
    } catch {
      return false
    }
  },

  reset: () => set(initialState),
}))

// ============================================================================
// Selectors
// ============================================================================

export const useLspServers = () => useLspStore((state) => state.servers)
export const useLspStatusLoading = () => useLspStore((state) => state.statusLoading)
export const useLspDiagnostics = () => useLspStore((state) => state.diagnostics)
export const useLspConfig = () => useLspStore((state) => state.config)
export const useHoverContent = () => useLspStore((state) => state.hoverContent)
export const useDefinitions = () => useLspStore((state) => state.definitions)
export const useReferences = () => useLspStore((state) => state.references)
export const useWorkspaceSymbols = () => useLspStore((state) => state.workspaceSymbols)
export const useDocumentSymbols = () => useLspStore((state) => state.documentSymbols)

// Computed selectors with stable references
export const useConnectedServers = () =>
  useLspStore(
    useShallow((state) => state.servers.filter((s) => s.status === "connected"))
  )

export const useErrorServers = () =>
  useLspStore(
    useShallow((state) => state.servers.filter((s) => s.status === "error"))
  )

// Count selectors (primitive values, no need for useShallow)
export const useConnectedServersCount = () =>
  useLspStore((state) => state.servers.filter((s) => s.status === "connected").length)

export const useErrorServersCount = () =>
  useLspStore((state) => state.servers.filter((s) => s.status === "error").length)

export const useTotalDiagnostics = () =>
  useLspStore((state) =>
    state.diagnostics.reduce((sum, file) => sum + file.diagnostics.length, 0)
  )

export const useErrorDiagnostics = () =>
  useLspStore(
    useShallow((state) =>
      state.diagnostics.flatMap((file) =>
        file.diagnostics
          .filter((d) => d.severity === 1)
          .map((d) => ({ ...d, filePath: file.filePath }))
      )
    )
  )
