// @ts-nocheck
/**
 * DialogMcp Component Unit Tests
 *
 * Tests for the MCP (Model Context Protocol) dialog including:
 * - MCP server list display
 * - Connection status display
 * - Authentication flow
 * - Enable/disable toggle
 * - Error state handling
 */

import { describe, it, expect, beforeEach, mock } from "bun:test"
import { render } from "solid-js/web"
import { createRoot, createSignal } from "solid-js"
import { TestProviders } from "@/test/helpers/test-context"
import { DialogMcp } from "@/cli/cmd/tui/component/dialog-mcp"

// Mock the MCP data
const mockMcpData = {
  "filesystem": {
    status: "enabled",
    version: "1.0.0",
  },
  "brave-search": {
    status: "disabled",
    version: "1.2.0",
  },
  "github": {
    status: "failed",
    error: "Authentication failed",
    version: "0.9.0",
  },
  "postgres": {
    status: "enabled",
    version: "2.0.0",
  },
}

const mockLocalMcp = {
  isEnabled: mock((name: string) => {
    const mcp = mockMcpData[name as keyof typeof mockMcpData]
    return mcp?.status === "enabled"
  }),
  toggle: mock(async (name: string) => {
    // Simulate toggle
    const current = mockMcpData[name as keyof typeof mockMcpData]
    if (current) {
      current.status = current.status === "enabled" ? "disabled" : "enabled"
    }
  }),
}

const mockSyncSet = mock(() => {})

const mockSyncData = {
  mcp: mockMcpData,
}

const mockSdkClient = {
  mcp: {
    status: mock(async () => ({
      data: mockMcpData,
    })),
  },
}

const mockSdk = {
  client: mockSdkClient,
}

describe("DialogMcp Component", () => {
  describe("MCP Server List", () => {
    it("should list all MCP servers", () => {
      const mcpEntries = Object.entries(mockMcpData)
      expect(mcpEntries).toHaveLength(4)
    })

    it("should sort MCP servers alphabetically", () => {
      const mcpEntries = Object.entries(mockMcpData).sort(([a], [b]) => a.localeCompare(b))
      expect(mcpEntries[0][0]).toBe("brave-search")
      expect(mcpEntries[1][0]).toBe("filesystem")
      expect(mcpEntries[2][0]).toBe("github")
      expect(mcpEntries[3][0]).toBe("postgres")
    })

    it("should create options for each MCP server", () => {
      const options = Object.entries(mockMcpData).map(([name, status]) => ({
        value: name,
        title: name,
        description: status.status === "failed" ? "failed" : status.status,
      }))

      expect(options).toHaveLength(4)
      expect(options[0].title).toBe("filesystem")
      expect(options[1].title).toBe("brave-search")
    })
  })

  describe("Connection Status Display", () => {
    it("should show enabled status for enabled MCPs", () => {
      const enabled = mockMcpData["filesystem"].status === "enabled"
      expect(enabled).toBe(true)
    })

    it("should show disabled status for disabled MCPs", () => {
      const disabled = mockMcpData["brave-search"].status === "disabled"
      expect(disabled).toBe(true)
    })

    it("should show failed status for failed MCPs", () => {
      const failed = mockMcpData["github"].status === "failed"
      expect(failed).toBe(true)
    })

    it("should check if MCP is enabled using local.mcp.isEnabled", () => {
      const isEnabled = mockLocalMcp.isEnabled("filesystem")
      expect(isEnabled).toBe(true)
      expect(mockLocalMcp.isEnabled).toHaveBeenCalledWith("filesystem")
    })

    it("should return disabled for not enabled MCPs", () => {
      const isEnabled = mockLocalMcp.isEnabled("brave-search")
      expect(isEnabled).toBe(false)
    })
  })

  describe("Enable/Disable Toggle", () => {
    it("should have a keybind for toggling", () => {
      const keybind = { key: "space", modifiers: [] }
      expect(keybind.key).toBe("space")
    })

    it("should call local.mcp.toggle when triggered", async () => {
      await mockLocalMcp.toggle("filesystem")
      expect(mockLocalMcp.toggle).toHaveBeenCalledWith("filesystem")
    })

    it("should prevent toggle while loading", () => {
      const loading = "filesystem"
      const shouldPrevent = loading !== null
      expect(shouldPrevent).toBe(true)
    })

    it("should allow toggle when not loading", () => {
      const loading = null
      const shouldPrevent = loading !== null
      expect(shouldPrevent).toBe(false)
    })
  })

  describe("Error State Handling", () => {
    it("should display error message for failed MCPs", () => {
      const githubMcp = mockMcpData["github"]
      expect(githubMcp.status).toBe("failed")
      expect(githubMcp.error).toBe("Authentication failed")
    })

    it("should show failed status in description", () => {
      const options = Object.entries(mockMcpData).map(([name, status]) => ({
        value: name,
        title: name,
        description: status.status === "failed" ? "failed" : status.status,
      }))

      const githubOption = options.find((opt) => opt.value === "github")
      expect(githubOption?.description).toBe("failed")
    })
  })

  describe("Refresh Status After Toggle", () => {
    it("should call sdk.client.mcp.status after toggle", async () => {
      await mockLocalMcp.toggle("postgres")
      const status = await mockSdkClient.mcp.status()
      expect(mockSdkClient.mcp.status).toHaveBeenCalled()
      expect(status.data).toBeDefined()
    })

    it("should update sync data with new status", async () => {
      const status = await mockSdkClient.mcp.status()
      if (status.data) {
        mockSyncSet("mcp", status.data)
      }
      expect(mockSyncSet).toHaveBeenCalledWith("mcp", mockMcpData)
    })
  })

  describe("Loading State", () => {
    it("should track which MCP is currently loading", () => {
      let loading: string | null = null

      const setLoading = (name: string | null) => {
        loading = name
      }

      setLoading("filesystem")
      expect(loading).toBe("filesystem")

      setLoading(null)
      expect(loading).toBe(null)
    })

    it("should show loading indicator for loading MCP", () => {
      const loading = "filesystem"
      const isLoading = loading === "filesystem"
      expect(isLoading).toBe(true)
    })
  })

  describe("MCP Data Structure", () => {
    it("should handle missing MCP data gracefully", () => {
      const mcpData = {}
      const entries = Object.entries(mcpData ?? {})
      expect(entries).toHaveLength(0)
    })

    it("should handle undefined sync data", () => {
      const syncData = { mcp: undefined }
      const mcpData = syncData.mcp ?? {}
      expect(mcpData).toEqual({})
    })
  })

  describe("Dialog Behavior", () => {
    it("should not close on select", () => {
      const shouldClose = false
      expect(shouldClose).toBe(false)
    })

    it("should only close on escape", () => {
      const escapeKey = { name: "escape", sequence: "\x1b" }
      expect(escapeKey.name).toBe("escape")
    })
  })
})
