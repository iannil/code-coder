import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useDocumentStore,
  useDocuments,
  useSelectedDocument,
  useDocumentChapters,
  useSelectedChapter,
  useDocumentEntities,
  useDocumentVolumes,
  useDocumentStats,
  useDocumentsLoading,
} from "@/stores/document"
import { api } from "@/lib/api"
import type { DocumentMetadata, DocumentChapter, DocumentEntity, DocumentVolume, DocumentStats } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    listDocuments: vi.fn(),
    getDocument: vi.fn(),
    createDocument: vi.fn(),
    deleteDocument: vi.fn(),
    listChapters: vi.fn(),
    updateChapter: vi.fn(),
    listEntities: vi.fn(),
    listVolumes: vi.fn(),
    getDocumentStats: vi.fn(),
  },
}))

const mockDocument: DocumentMetadata = {
  id: "doc-1",
  title: "Test Document",
  description: "A test document",
  targetWords: 50000,
  createdAt: Date.now() - 86400000,
  updatedAt: Date.now(),
}

const mockDocument2: DocumentMetadata = {
  id: "doc-2",
  title: "Another Document",
  description: "Another test document",
  targetWords: 30000,
  createdAt: Date.now() - 172800000,
  updatedAt: Date.now() - 86400000,
}

const mockChapter: DocumentChapter = {
  id: "chapter-1",
  documentId: "doc-1",
  title: "Chapter 1",
  content: "Chapter content...",
  order: 1,
  wordCount: 1500,
}

const mockChapter2: DocumentChapter = {
  id: "chapter-2",
  documentId: "doc-1",
  title: "Chapter 2",
  content: "More content...",
  order: 2,
  wordCount: 2000,
}

const mockEntity: DocumentEntity = {
  id: "entity-1",
  documentId: "doc-1",
  name: "Main Character",
  type: "character",
  description: "The protagonist",
}

const mockVolume: DocumentVolume = {
  id: "volume-1",
  documentId: "doc-1",
  title: "Volume 1",
  order: 1,
  chapterIds: ["chapter-1", "chapter-2"],
}

const mockStats: DocumentStats = {
  totalWords: 3500,
  totalChapters: 2,
  totalEntities: 1,
  totalVolumes: 1,
  completionPercentage: 7,
}

describe("Document Store", () => {
  beforeEach(() => {
    useDocumentStore.getState().reset()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty documents list", () => {
      const state = useDocumentStore.getState()
      expect(state.documents).toEqual([])
    })

    it("should have no selected document", () => {
      const state = useDocumentStore.getState()
      expect(state.selectedDocumentId).toBeNull()
      expect(state.selectedDocument).toBeNull()
    })

    it("should have documentsLoading false", () => {
      const state = useDocumentStore.getState()
      expect(state.documentsLoading).toBe(false)
    })
  })

  describe("fetchDocuments", () => {
    it("should fetch documents from API", async () => {
      vi.mocked(api.listDocuments).mockResolvedValueOnce([mockDocument, mockDocument2])

      await useDocumentStore.getState().fetchDocuments()

      const state = useDocumentStore.getState()
      expect(state.documents).toHaveLength(2)
      expect(state.documents[0]).toEqual(mockDocument)
      expect(state.documentsLoading).toBe(false)
    })

    it("should set loading state during fetch", async () => {
      vi.mocked(api.listDocuments).mockImplementationOnce(async () => {
        expect(useDocumentStore.getState().documentsLoading).toBe(true)
        return [mockDocument]
      })

      await useDocumentStore.getState().fetchDocuments()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.listDocuments).mockRejectedValueOnce(new Error("Network error"))

      await useDocumentStore.getState().fetchDocuments()

      const state = useDocumentStore.getState()
      expect(state.documentsError).toBe("Network error")
      expect(state.documentsLoading).toBe(false)
    })

    it("should handle non-Error exceptions", async () => {
      vi.mocked(api.listDocuments).mockRejectedValueOnce("Unknown error")

      await useDocumentStore.getState().fetchDocuments()

      expect(useDocumentStore.getState().documentsError).toBe("Failed to fetch documents")
    })
  })

  describe("selectDocument", () => {
    beforeEach(() => {
      vi.mocked(api.getDocument).mockResolvedValue(mockDocument)
      vi.mocked(api.listChapters).mockResolvedValue([mockChapter, mockChapter2])
      vi.mocked(api.listEntities).mockResolvedValue([mockEntity])
      vi.mocked(api.listVolumes).mockResolvedValue([mockVolume])
      vi.mocked(api.getDocumentStats).mockResolvedValue(mockStats)
    })

    it("should select document and load related data", async () => {
      await useDocumentStore.getState().selectDocument("doc-1")

      const state = useDocumentStore.getState()
      expect(state.selectedDocumentId).toBe("doc-1")
      expect(state.selectedDocument).toEqual(mockDocument)
      expect(state.chapters).toHaveLength(2)
      expect(state.entities).toHaveLength(1)
      expect(state.volumes).toHaveLength(1)
      expect(state.stats).toEqual(mockStats)
    })

    it("should clear selection when id is null", async () => {
      useDocumentStore.setState({
        selectedDocumentId: "doc-1",
        selectedDocument: mockDocument,
        chapters: [mockChapter],
        entities: [mockEntity],
        volumes: [mockVolume],
        stats: mockStats,
      })

      await useDocumentStore.getState().selectDocument(null)

      const state = useDocumentStore.getState()
      expect(state.selectedDocumentId).toBeNull()
      expect(state.selectedDocument).toBeNull()
      expect(state.chapters).toEqual([])
      expect(state.entities).toEqual([])
      expect(state.volumes).toEqual([])
      expect(state.stats).toBeNull()
    })

    it("should handle errors when selecting document", async () => {
      vi.mocked(api.getDocument).mockRejectedValueOnce(new Error("Not found"))

      await useDocumentStore.getState().selectDocument("doc-1")

      expect(useDocumentStore.getState().documentsError).toBe("Not found")
    })
  })

  describe("createDocument", () => {
    it("should create document and add to list", async () => {
      vi.mocked(api.createDocument).mockResolvedValueOnce(mockDocument)

      const result = await useDocumentStore.getState().createDocument({
        title: "Test Document",
        description: "A test document",
        targetWords: 50000,
      })

      expect(result).toEqual(mockDocument)
      expect(useDocumentStore.getState().documents[0]).toEqual(mockDocument)
    })

    it("should prepend new document to list", async () => {
      useDocumentStore.setState({ documents: [mockDocument2] })
      vi.mocked(api.createDocument).mockResolvedValueOnce(mockDocument)

      await useDocumentStore.getState().createDocument({
        title: "Test Document",
        targetWords: 50000,
      })

      const docs = useDocumentStore.getState().documents
      expect(docs[0]).toEqual(mockDocument)
      expect(docs[1]).toEqual(mockDocument2)
    })
  })

  describe("deleteDocument", () => {
    it("should delete document from list", async () => {
      useDocumentStore.setState({ documents: [mockDocument, mockDocument2] })
      vi.mocked(api.deleteDocument).mockResolvedValueOnce(undefined)

      await useDocumentStore.getState().deleteDocument("doc-1")

      const docs = useDocumentStore.getState().documents
      expect(docs).toHaveLength(1)
      expect(docs[0].id).toBe("doc-2")
    })

    it("should clear selection if deleted document was selected", async () => {
      useDocumentStore.setState({
        documents: [mockDocument, mockDocument2],
        selectedDocumentId: "doc-1",
        selectedDocument: mockDocument,
      })
      vi.mocked(api.deleteDocument).mockResolvedValueOnce(undefined)

      await useDocumentStore.getState().deleteDocument("doc-1")

      expect(useDocumentStore.getState().selectedDocumentId).toBeNull()
    })
  })

  describe("chapter management", () => {
    describe("fetchChapters", () => {
      it("should fetch chapters for document", async () => {
        vi.mocked(api.listChapters).mockResolvedValueOnce([mockChapter, mockChapter2])

        await useDocumentStore.getState().fetchChapters("doc-1")

        expect(useDocumentStore.getState().chapters).toHaveLength(2)
        expect(useDocumentStore.getState().chaptersLoading).toBe(false)
      })

      it("should handle errors", async () => {
        vi.mocked(api.listChapters).mockRejectedValueOnce(new Error("Failed"))

        await useDocumentStore.getState().fetchChapters("doc-1")

        expect(useDocumentStore.getState().chaptersLoading).toBe(false)
      })
    })

    describe("selectChapter", () => {
      it("should select chapter", () => {
        useDocumentStore.setState({ chapters: [mockChapter, mockChapter2] })

        useDocumentStore.getState().selectChapter("chapter-1")

        const state = useDocumentStore.getState()
        expect(state.selectedChapterId).toBe("chapter-1")
        expect(state.selectedChapter).toEqual(mockChapter)
      })

      it("should clear selection when id is null", () => {
        useDocumentStore.setState({
          chapters: [mockChapter],
          selectedChapterId: "chapter-1",
          selectedChapter: mockChapter,
        })

        useDocumentStore.getState().selectChapter(null)

        expect(useDocumentStore.getState().selectedChapterId).toBeNull()
        expect(useDocumentStore.getState().selectedChapter).toBeNull()
      })

      it("should handle non-existent chapter", () => {
        useDocumentStore.setState({ chapters: [mockChapter] })

        useDocumentStore.getState().selectChapter("non-existent")

        expect(useDocumentStore.getState().selectedChapter).toBeNull()
      })
    })

    describe("updateChapter", () => {
      it("should update chapter in list", async () => {
        useDocumentStore.setState({ chapters: [mockChapter] })
        const updatedChapter = { ...mockChapter, title: "Updated Title" }
        vi.mocked(api.updateChapter).mockResolvedValueOnce(updatedChapter)

        await useDocumentStore.getState().updateChapter("doc-1", "chapter-1", { title: "Updated Title" })

        expect(useDocumentStore.getState().chapters[0].title).toBe("Updated Title")
      })

      it("should update selected chapter if it was updated", async () => {
        useDocumentStore.setState({
          chapters: [mockChapter],
          selectedChapterId: "chapter-1",
          selectedChapter: mockChapter,
        })
        const updatedChapter = { ...mockChapter, title: "Updated Title" }
        vi.mocked(api.updateChapter).mockResolvedValueOnce(updatedChapter)

        await useDocumentStore.getState().updateChapter("doc-1", "chapter-1", { title: "Updated Title" })

        expect(useDocumentStore.getState().selectedChapter?.title).toBe("Updated Title")
      })
    })
  })

  describe("entity management", () => {
    it("should fetch entities", async () => {
      vi.mocked(api.listEntities).mockResolvedValueOnce([mockEntity])

      await useDocumentStore.getState().fetchEntities("doc-1")

      expect(useDocumentStore.getState().entities).toHaveLength(1)
      expect(useDocumentStore.getState().entitiesLoading).toBe(false)
    })

    it("should handle errors", async () => {
      vi.mocked(api.listEntities).mockRejectedValueOnce(new Error("Failed"))

      await useDocumentStore.getState().fetchEntities("doc-1")

      expect(useDocumentStore.getState().entitiesLoading).toBe(false)
    })
  })

  describe("volume management", () => {
    it("should fetch volumes", async () => {
      vi.mocked(api.listVolumes).mockResolvedValueOnce([mockVolume])

      await useDocumentStore.getState().fetchVolumes("doc-1")

      expect(useDocumentStore.getState().volumes).toHaveLength(1)
      expect(useDocumentStore.getState().volumesLoading).toBe(false)
    })

    it("should handle errors", async () => {
      vi.mocked(api.listVolumes).mockRejectedValueOnce(new Error("Failed"))

      await useDocumentStore.getState().fetchVolumes("doc-1")

      expect(useDocumentStore.getState().volumesLoading).toBe(false)
    })
  })

  describe("stats management", () => {
    it("should fetch stats", async () => {
      vi.mocked(api.getDocumentStats).mockResolvedValueOnce(mockStats)

      await useDocumentStore.getState().fetchStats("doc-1")

      expect(useDocumentStore.getState().stats).toEqual(mockStats)
      expect(useDocumentStore.getState().statsLoading).toBe(false)
    })

    it("should handle errors", async () => {
      vi.mocked(api.getDocumentStats).mockRejectedValueOnce(new Error("Failed"))

      await useDocumentStore.getState().fetchStats("doc-1")

      expect(useDocumentStore.getState().statsLoading).toBe(false)
    })
  })

  describe("reset", () => {
    it("should reset to initial state", () => {
      useDocumentStore.setState({
        documents: [mockDocument],
        selectedDocumentId: "doc-1",
        selectedDocument: mockDocument,
        chapters: [mockChapter],
        entities: [mockEntity],
        volumes: [mockVolume],
        stats: mockStats,
        documentsError: "Some error",
      })

      useDocumentStore.getState().reset()

      const state = useDocumentStore.getState()
      expect(state.documents).toEqual([])
      expect(state.selectedDocumentId).toBeNull()
      expect(state.selectedDocument).toBeNull()
      expect(state.chapters).toEqual([])
      expect(state.entities).toEqual([])
      expect(state.volumes).toEqual([])
      expect(state.stats).toBeNull()
      expect(state.documentsError).toBeNull()
    })
  })
})

describe("Document Selector Hooks", () => {
  beforeEach(() => {
    useDocumentStore.setState({
      documents: [mockDocument, mockDocument2],
      selectedDocumentId: "doc-1",
      selectedDocument: mockDocument,
      chapters: [mockChapter, mockChapter2],
      selectedChapterId: "chapter-1",
      selectedChapter: mockChapter,
      entities: [mockEntity],
      volumes: [mockVolume],
      stats: mockStats,
      documentsLoading: false,
      documentsError: null,
    })
  })

  describe("useDocuments", () => {
    it("should return documents", () => {
      const { result } = renderHook(() => useDocuments())
      expect(result.current).toHaveLength(2)
    })
  })

  describe("useSelectedDocument", () => {
    it("should return selected document", () => {
      const { result } = renderHook(() => useSelectedDocument())
      expect(result.current).toEqual(mockDocument)
    })
  })

  describe("useDocumentChapters", () => {
    it("should return chapters", () => {
      const { result } = renderHook(() => useDocumentChapters())
      expect(result.current).toHaveLength(2)
    })
  })

  describe("useSelectedChapter", () => {
    it("should return selected chapter", () => {
      const { result } = renderHook(() => useSelectedChapter())
      expect(result.current).toEqual(mockChapter)
    })
  })

  describe("useDocumentEntities", () => {
    it("should return entities", () => {
      const { result } = renderHook(() => useDocumentEntities())
      expect(result.current).toHaveLength(1)
    })
  })

  describe("useDocumentVolumes", () => {
    it("should return volumes", () => {
      const { result } = renderHook(() => useDocumentVolumes())
      expect(result.current).toHaveLength(1)
    })
  })

  describe("useDocumentStats", () => {
    it("should return stats", () => {
      const { result } = renderHook(() => useDocumentStats())
      expect(result.current).toEqual(mockStats)
    })
  })

  describe("useDocumentsLoading", () => {
    it("should return loading state", () => {
      const { result } = renderHook(() => useDocumentsLoading())
      expect(result.current).toBe(false)
    })
  })
})
