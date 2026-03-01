import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useProjectStore,
  useProjects,
  useProject,
  useSelectedProject,
  useSelectedProjectId,
  useProjectsLoading,
  useProjectError,
  useProjectDeleting,
} from "@/stores/project"
import { api } from "@/lib/api"
import type { ProjectInfo } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    listProjects: vi.fn(),
    createProject: vi.fn(),
    deleteProject: vi.fn(),
    updateProject: vi.fn(),
  },
}))

const mockProject1: ProjectInfo = {
  id: "proj-1",
  name: "Project One",
  path: "/path/to/project1",
  createdAt: Date.now() - 3600000,
  updatedAt: Date.now(),
}

const mockProject2: ProjectInfo = {
  id: "proj-2",
  name: "Project Two",
  path: "/path/to/project2",
  createdAt: Date.now() - 7200000,
  updatedAt: Date.now() - 3600000,
  icon: { url: "https://example.com/icon.png" },
}

describe("Project Store", () => {
  beforeEach(() => {
    useProjectStore.getState().reset()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty projects", () => {
      const state = useProjectStore.getState()
      expect(state.projects.size).toBe(0)
    })

    it("should have null selectedProjectId", () => {
      const state = useProjectStore.getState()
      expect(state.selectedProjectId).toBeNull()
    })

    it("should have isLoading false", () => {
      const state = useProjectStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isLoaded false", () => {
      const state = useProjectStore.getState()
      expect(state.isLoaded).toBe(false)
    })

    it("should have isCreating false", () => {
      const state = useProjectStore.getState()
      expect(state.isCreating).toBe(false)
    })

    it("should have null error", () => {
      const state = useProjectStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("loadProjects", () => {
    it("should load projects from API", async () => {
      vi.mocked(api.listProjects).mockResolvedValueOnce([mockProject1, mockProject2])

      await useProjectStore.getState().loadProjects()

      const state = useProjectStore.getState()
      expect(state.projects.size).toBe(2)
      expect(state.projects.get("proj-1")).toEqual(mockProject1)
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(true)
    })

    it("should set isLoading during fetch", async () => {
      vi.mocked(api.listProjects).mockImplementationOnce(async () => {
        expect(useProjectStore.getState().isLoading).toBe(true)
        return [mockProject1]
      })

      await useProjectStore.getState().loadProjects()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.listProjects).mockRejectedValueOnce(new Error("Network error"))

      await useProjectStore.getState().loadProjects()

      const state = useProjectStore.getState()
      expect(state.error).toBe("Network error")
      expect(state.isLoading).toBe(false)
    })
  })

  describe("selectProject", () => {
    it("should select a project", () => {
      useProjectStore.getState().selectProject("proj-1")

      expect(useProjectStore.getState().selectedProjectId).toBe("proj-1")
    })

    it("should clear selection with null", () => {
      useProjectStore.getState().selectProject("proj-1")
      useProjectStore.getState().selectProject(null)

      expect(useProjectStore.getState().selectedProjectId).toBeNull()
    })
  })

  describe("createProject", () => {
    it("should create a project and select it", async () => {
      const newProject = { ...mockProject1, id: "new-proj" }
      vi.mocked(api.createProject).mockResolvedValueOnce(newProject)

      const input = { name: "New Project", path: "/path/to/new" }
      const result = await useProjectStore.getState().createProject(input)

      expect(result).toEqual(newProject)
      expect(useProjectStore.getState().projects.get("new-proj")).toEqual(newProject)
      expect(useProjectStore.getState().selectedProjectId).toBe("new-proj")
      expect(useProjectStore.getState().isCreating).toBe(false)
    })

    it("should set isCreating during creation", async () => {
      vi.mocked(api.createProject).mockImplementationOnce(async () => {
        expect(useProjectStore.getState().isCreating).toBe(true)
        return mockProject1
      })

      await useProjectStore.getState().createProject({ name: "Test", path: "/test" })
    })

    it("should handle errors and throw", async () => {
      vi.mocked(api.createProject).mockRejectedValueOnce(new Error("Create failed"))

      await expect(
        useProjectStore.getState().createProject({ name: "Test", path: "/test" })
      ).rejects.toThrow("Create failed")

      expect(useProjectStore.getState().error).toBe("Create failed")
      expect(useProjectStore.getState().isCreating).toBe(false)
    })
  })

  describe("deleteProject", () => {
    beforeEach(async () => {
      // Set up projects
      vi.mocked(api.listProjects).mockResolvedValueOnce([mockProject1, mockProject2])
      await useProjectStore.getState().loadProjects()
    })

    it("should delete a project", async () => {
      vi.mocked(api.deleteProject).mockResolvedValueOnce(undefined)

      await useProjectStore.getState().deleteProject("proj-1")

      expect(useProjectStore.getState().projects.has("proj-1")).toBe(false)
      expect(useProjectStore.getState().projects.size).toBe(1)
    })

    it("should clear selection if deleted project was selected", async () => {
      useProjectStore.getState().selectProject("proj-1")
      vi.mocked(api.deleteProject).mockResolvedValueOnce(undefined)

      await useProjectStore.getState().deleteProject("proj-1")

      expect(useProjectStore.getState().selectedProjectId).toBeNull()
    })

    it("should set isDeleting during deletion", async () => {
      vi.mocked(api.deleteProject).mockImplementationOnce(async () => {
        expect(useProjectStore.getState().isDeleting.has("proj-1")).toBe(true)
      })

      await useProjectStore.getState().deleteProject("proj-1")
    })

    it("should handle errors and throw", async () => {
      vi.mocked(api.deleteProject).mockRejectedValueOnce(new Error("Delete failed"))

      await expect(useProjectStore.getState().deleteProject("proj-1")).rejects.toThrow("Delete failed")

      expect(useProjectStore.getState().error).toBe("Delete failed")
      expect(useProjectStore.getState().isDeleting.has("proj-1")).toBe(false)
    })
  })

  describe("updateProject", () => {
    beforeEach(async () => {
      vi.mocked(api.listProjects).mockResolvedValueOnce([mockProject1])
      await useProjectStore.getState().loadProjects()
    })

    it("should update project name", async () => {
      vi.mocked(api.updateProject).mockResolvedValueOnce(undefined)

      await useProjectStore.getState().updateProject("proj-1", { name: "Updated Name" })

      expect(useProjectStore.getState().projects.get("proj-1")?.name).toBe("Updated Name")
    })

    it("should update project icon", async () => {
      vi.mocked(api.updateProject).mockResolvedValueOnce(undefined)

      const newIcon = { url: "https://new-icon.png" }
      await useProjectStore.getState().updateProject("proj-1", { icon: newIcon })

      expect(useProjectStore.getState().projects.get("proj-1")?.icon).toEqual(newIcon)
    })

    it("should handle errors and throw", async () => {
      vi.mocked(api.updateProject).mockRejectedValueOnce(new Error("Update failed"))

      await expect(
        useProjectStore.getState().updateProject("proj-1", { name: "Updated" })
      ).rejects.toThrow("Update failed")

      expect(useProjectStore.getState().error).toBe("Update failed")
    })
  })

  describe("state management", () => {
    it("setError should set error", () => {
      useProjectStore.getState().setError("Test error")

      expect(useProjectStore.getState().error).toBe("Test error")
    })

    it("clearError should clear error", () => {
      useProjectStore.getState().setError("Test error")
      useProjectStore.getState().clearError()

      expect(useProjectStore.getState().error).toBeNull()
    })

    it("reset should clear all state", async () => {
      vi.mocked(api.listProjects).mockResolvedValueOnce([mockProject1])
      await useProjectStore.getState().loadProjects()
      useProjectStore.getState().selectProject("proj-1")
      useProjectStore.getState().setError("Test error")

      useProjectStore.getState().reset()

      const state = useProjectStore.getState()
      expect(state.projects.size).toBe(0)
      expect(state.selectedProjectId).toBeNull()
      expect(state.error).toBeNull()
      expect(state.isLoaded).toBe(false)
    })
  })

  describe("hooks", () => {
    beforeEach(async () => {
      vi.mocked(api.listProjects).mockResolvedValueOnce([mockProject1, mockProject2])
      await useProjectStore.getState().loadProjects()
    })

    it("useProjects should return projects array", () => {
      const { result } = renderHook(() => useProjects())
      expect(result.current).toHaveLength(2)
    })

    it("useProject should return specific project", () => {
      const { result } = renderHook(() => useProject("proj-1"))
      expect(result.current).toEqual(mockProject1)
    })

    it("useSelectedProject should return selected project", () => {
      useProjectStore.getState().selectProject("proj-1")

      const { result } = renderHook(() => useSelectedProject())
      expect(result.current).toEqual(mockProject1)
    })

    it("useSelectedProject should return null when nothing selected", () => {
      const { result } = renderHook(() => useSelectedProject())
      expect(result.current).toBeNull()
    })

    it("useSelectedProjectId should return selected ID", () => {
      useProjectStore.getState().selectProject("proj-2")

      const { result } = renderHook(() => useSelectedProjectId())
      expect(result.current).toBe("proj-2")
    })

    it("useProjectsLoading should return loading states", () => {
      const { result } = renderHook(() => useProjectsLoading())
      expect(result.current.isLoading).toBe(false)
      expect(result.current.isLoaded).toBe(true)
      expect(result.current.isCreating).toBe(false)
    })

    it("useProjectError should return error", () => {
      useProjectStore.getState().setError("Test error")

      const { result } = renderHook(() => useProjectError())
      expect(result.current).toBe("Test error")
    })

    it("useProjectDeleting should check if project is being deleted", async () => {
      vi.mocked(api.deleteProject).mockImplementationOnce(async () => {
        const { result } = renderHook(() => useProjectDeleting("proj-1"))
        expect(result.current).toBe(true)
      })

      await useProjectStore.getState().deleteProject("proj-1")
    })
  })
})
