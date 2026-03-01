import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useCredentialStore,
  useCredentials,
  useCredentialLoading,
  useCredentialError,
} from "@/stores/credential"
import { api } from "@/lib/api"
import type { CredentialSummary, CredentialEntry } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    listCredentials: vi.fn(),
    getCredential: vi.fn(),
    addCredential: vi.fn(),
    updateCredential: vi.fn(),
    deleteCredential: vi.fn(),
  },
}))

const mockCredential: CredentialSummary = {
  id: "cred-1",
  name: "Test Credential",
  type: "api_key",
  createdAt: Date.now() - 3600000,
  updatedAt: Date.now(),
}

const mockCredential2: CredentialSummary = {
  id: "cred-2",
  name: "Test Credential 2",
  type: "oauth",
  createdAt: Date.now() - 7200000,
  updatedAt: Date.now() - 3600000,
}

const mockCredentialEntry: CredentialEntry = {
  ...mockCredential,
  value: "secret-value",
  metadata: {},
}

describe("Credential Store", () => {
  beforeEach(() => {
    // Reset the store before each test
    useCredentialStore.setState({
      credentials: [],
      isLoading: false,
      isLoaded: false,
      isAdding: false,
      isUpdating: null,
      isDeleting: null,
      error: null,
    })
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty credentials array", () => {
      const state = useCredentialStore.getState()
      expect(state.credentials).toEqual([])
    })

    it("should have isLoading false", () => {
      const state = useCredentialStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isLoaded false", () => {
      const state = useCredentialStore.getState()
      expect(state.isLoaded).toBe(false)
    })

    it("should have isAdding false", () => {
      const state = useCredentialStore.getState()
      expect(state.isAdding).toBe(false)
    })

    it("should have isUpdating null", () => {
      const state = useCredentialStore.getState()
      expect(state.isUpdating).toBeNull()
    })

    it("should have isDeleting null", () => {
      const state = useCredentialStore.getState()
      expect(state.isDeleting).toBeNull()
    })

    it("should have null error", () => {
      const state = useCredentialStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("fetchCredentials", () => {
    it("should load credentials from API", async () => {
      vi.mocked(api.listCredentials).mockResolvedValueOnce([mockCredential, mockCredential2])

      await useCredentialStore.getState().fetchCredentials()

      const state = useCredentialStore.getState()
      expect(state.credentials).toHaveLength(2)
      expect(state.credentials[0].id).toBe("cred-1")
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(true)
    })

    it("should set isLoading during fetch", async () => {
      vi.mocked(api.listCredentials).mockImplementationOnce(async () => {
        expect(useCredentialStore.getState().isLoading).toBe(true)
        return [mockCredential]
      })

      await useCredentialStore.getState().fetchCredentials()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.listCredentials).mockRejectedValueOnce(new Error("Network error"))

      await useCredentialStore.getState().fetchCredentials()

      const state = useCredentialStore.getState()
      expect(state.error).toBe("Network error")
      expect(state.isLoading).toBe(false)
    })
  })

  describe("getCredential", () => {
    it("should return credential from API", async () => {
      vi.mocked(api.getCredential).mockResolvedValueOnce(mockCredentialEntry)

      const result = await useCredentialStore.getState().getCredential("cred-1")

      expect(result).toEqual(mockCredentialEntry)
      expect(api.getCredential).toHaveBeenCalledWith("cred-1")
    })

    it("should handle API errors", async () => {
      vi.mocked(api.getCredential).mockRejectedValueOnce(new Error("Not found"))

      const result = await useCredentialStore.getState().getCredential("cred-1")

      expect(result).toBeNull()
      expect(useCredentialStore.getState().error).toBe("Not found")
    })
  })

  describe("addCredential", () => {
    it("should add credential and refresh list", async () => {
      vi.mocked(api.addCredential).mockResolvedValueOnce({ id: "new-cred" })
      vi.mocked(api.listCredentials).mockResolvedValueOnce([mockCredential, { ...mockCredential2, id: "new-cred" }])

      const input = { name: "New Credential", type: "api_key" as const, value: "secret" }
      const id = await useCredentialStore.getState().addCredential(input)

      expect(id).toBe("new-cred")
      expect(api.addCredential).toHaveBeenCalledWith(input)
      expect(useCredentialStore.getState().credentials).toHaveLength(2)
      expect(useCredentialStore.getState().isAdding).toBe(false)
    })

    it("should set isAdding during add", async () => {
      vi.mocked(api.addCredential).mockImplementationOnce(async () => {
        expect(useCredentialStore.getState().isAdding).toBe(true)
        return { id: "new-cred" }
      })
      vi.mocked(api.listCredentials).mockResolvedValueOnce([])

      await useCredentialStore.getState().addCredential({ name: "Test", type: "api_key", value: "secret" })
    })

    it("should handle API errors", async () => {
      vi.mocked(api.addCredential).mockRejectedValueOnce(new Error("Invalid input"))

      await expect(
        useCredentialStore.getState().addCredential({ name: "Test", type: "api_key", value: "secret" })
      ).rejects.toThrow("Invalid input")

      expect(useCredentialStore.getState().error).toBe("Invalid input")
      expect(useCredentialStore.getState().isAdding).toBe(false)
    })
  })

  describe("updateCredential", () => {
    it("should update credential and refresh list", async () => {
      vi.mocked(api.updateCredential).mockResolvedValueOnce(undefined)
      vi.mocked(api.listCredentials).mockResolvedValueOnce([{ ...mockCredential, name: "Updated" }])

      await useCredentialStore.getState().updateCredential("cred-1", { name: "Updated" })

      expect(api.updateCredential).toHaveBeenCalledWith("cred-1", { name: "Updated" })
      expect(useCredentialStore.getState().credentials[0].name).toBe("Updated")
      expect(useCredentialStore.getState().isUpdating).toBeNull()
    })

    it("should set isUpdating during update", async () => {
      vi.mocked(api.updateCredential).mockImplementationOnce(async () => {
        expect(useCredentialStore.getState().isUpdating).toBe("cred-1")
      })
      vi.mocked(api.listCredentials).mockResolvedValueOnce([])

      await useCredentialStore.getState().updateCredential("cred-1", { name: "Updated" })
    })

    it("should handle API errors", async () => {
      vi.mocked(api.updateCredential).mockRejectedValueOnce(new Error("Update failed"))

      await expect(
        useCredentialStore.getState().updateCredential("cred-1", { name: "Updated" })
      ).rejects.toThrow("Update failed")

      expect(useCredentialStore.getState().error).toBe("Update failed")
      expect(useCredentialStore.getState().isUpdating).toBeNull()
    })
  })

  describe("deleteCredential", () => {
    it("should delete credential from list", async () => {
      useCredentialStore.setState({ credentials: [mockCredential, mockCredential2] })
      vi.mocked(api.deleteCredential).mockResolvedValueOnce(undefined)

      await useCredentialStore.getState().deleteCredential("cred-1")

      expect(api.deleteCredential).toHaveBeenCalledWith("cred-1")
      expect(useCredentialStore.getState().credentials).toHaveLength(1)
      expect(useCredentialStore.getState().credentials[0].id).toBe("cred-2")
      expect(useCredentialStore.getState().isDeleting).toBeNull()
    })

    it("should set isDeleting during delete", async () => {
      useCredentialStore.setState({ credentials: [mockCredential] })
      vi.mocked(api.deleteCredential).mockImplementationOnce(async () => {
        expect(useCredentialStore.getState().isDeleting).toBe("cred-1")
      })

      await useCredentialStore.getState().deleteCredential("cred-1")
    })

    it("should handle API errors", async () => {
      vi.mocked(api.deleteCredential).mockRejectedValueOnce(new Error("Delete failed"))

      await expect(
        useCredentialStore.getState().deleteCredential("cred-1")
      ).rejects.toThrow("Delete failed")

      expect(useCredentialStore.getState().error).toBe("Delete failed")
      expect(useCredentialStore.getState().isDeleting).toBeNull()
    })
  })

  describe("clearError", () => {
    it("should clear error", () => {
      useCredentialStore.setState({ error: "Some error" })

      useCredentialStore.getState().clearError()

      expect(useCredentialStore.getState().error).toBeNull()
    })
  })

  describe("hooks", () => {
    it("useCredentials should return credentials", () => {
      useCredentialStore.setState({ credentials: [mockCredential, mockCredential2] })

      const { result } = renderHook(() => useCredentials())
      expect(result.current).toHaveLength(2)
    })

    it("useCredentialLoading should return loading states", () => {
      useCredentialStore.setState({
        isLoading: true,
        isLoaded: true,
        isAdding: false,
        isUpdating: "cred-1",
        isDeleting: null,
      })

      const { result } = renderHook(() => useCredentialLoading())
      expect(result.current.isLoading).toBe(true)
      expect(result.current.isLoaded).toBe(true)
      expect(result.current.isAdding).toBe(false)
      expect(result.current.isUpdating).toBe("cred-1")
      expect(result.current.isDeleting).toBeNull()
    })

    it("useCredentialError should return error", () => {
      useCredentialStore.setState({ error: "Test error" })

      const { result } = renderHook(() => useCredentialError())
      expect(result.current).toBe("Test error")
    })
  })
})
