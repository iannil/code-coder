/**
 * Document Store
 *
 * Manages document/writing system state:
 * - Document list and selected document
 * - Chapters, entities, volumes
 * - Statistics
 */

import { create } from "zustand"
import type {
  DocumentMetadata,
  DocumentChapter,
  DocumentEntity,
  DocumentVolume,
  DocumentStats,
} from "@/lib/types"
import { api } from "@/lib/api"

// ============================================================================
// Types
// ============================================================================

interface DocumentState {
  // Documents
  documents: DocumentMetadata[]
  selectedDocumentId: string | null
  selectedDocument: DocumentMetadata | null
  documentsLoading: boolean
  documentsError: string | null

  // Chapters
  chapters: DocumentChapter[]
  selectedChapterId: string | null
  selectedChapter: DocumentChapter | null
  chaptersLoading: boolean

  // Entities
  entities: DocumentEntity[]
  entitiesLoading: boolean

  // Volumes
  volumes: DocumentVolume[]
  volumesLoading: boolean

  // Stats
  stats: DocumentStats | null
  statsLoading: boolean

  // Actions
  fetchDocuments: () => Promise<void>
  selectDocument: (id: string | null) => Promise<void>
  createDocument: (input: { title: string; description?: string; targetWords: number }) => Promise<DocumentMetadata>
  deleteDocument: (id: string) => Promise<void>
  fetchChapters: (documentId: string) => Promise<void>
  selectChapter: (id: string | null) => void
  updateChapter: (documentId: string, chapterId: string, input: Partial<DocumentChapter>) => Promise<void>
  fetchEntities: (documentId: string) => Promise<void>
  fetchVolumes: (documentId: string) => Promise<void>
  fetchStats: (documentId: string) => Promise<void>
  reset: () => void
}

// ============================================================================
// Initial State
// ============================================================================

const initialState = {
  documents: [],
  selectedDocumentId: null,
  selectedDocument: null,
  documentsLoading: false,
  documentsError: null,
  chapters: [],
  selectedChapterId: null,
  selectedChapter: null,
  chaptersLoading: false,
  entities: [],
  entitiesLoading: false,
  volumes: [],
  volumesLoading: false,
  stats: null,
  statsLoading: false,
}

// ============================================================================
// Store
// ============================================================================

export const useDocumentStore = create<DocumentState>((set, get) => ({
  ...initialState,

  fetchDocuments: async () => {
    set({ documentsLoading: true, documentsError: null })
    try {
      const documents = await api.listDocuments()
      set({ documents, documentsLoading: false })
    } catch (error) {
      set({
        documentsError: error instanceof Error ? error.message : "Failed to fetch documents",
        documentsLoading: false,
      })
    }
  },

  selectDocument: async (id) => {
    if (!id) {
      set({
        selectedDocumentId: null,
        selectedDocument: null,
        chapters: [],
        selectedChapterId: null,
        selectedChapter: null,
        entities: [],
        volumes: [],
        stats: null,
      })
      return
    }

    set({ selectedDocumentId: id })

    try {
      const document = await api.getDocument(id)
      set({ selectedDocument: document })

      // Load related data
      const { fetchChapters, fetchEntities, fetchVolumes, fetchStats } = get()
      await Promise.all([
        fetchChapters(id),
        fetchEntities(id),
        fetchVolumes(id),
        fetchStats(id),
      ])
    } catch (error) {
      set({ documentsError: error instanceof Error ? error.message : "Failed to fetch document" })
    }
  },

  createDocument: async (input) => {
    const document = await api.createDocument(input)
    set((state) => ({ documents: [document, ...state.documents] }))
    return document
  },

  deleteDocument: async (id) => {
    await api.deleteDocument(id)
    const { selectedDocumentId, selectDocument } = get()
    set((state) => ({
      documents: state.documents.filter((d) => d.id !== id),
    }))
    if (selectedDocumentId === id) {
      selectDocument(null)
    }
  },

  fetchChapters: async (documentId) => {
    set({ chaptersLoading: true })
    try {
      const chapters = await api.listChapters(documentId)
      set({ chapters, chaptersLoading: false })
    } catch {
      set({ chaptersLoading: false })
    }
  },

  selectChapter: (id) => {
    const { chapters } = get()
    const chapter = id ? chapters.find((c) => c.id === id) ?? null : null
    set({ selectedChapterId: id, selectedChapter: chapter })
  },

  updateChapter: async (documentId, chapterId, input) => {
    const updated = await api.updateChapter(documentId, chapterId, input)
    set((state) => ({
      chapters: state.chapters.map((c) => (c.id === chapterId ? updated : c)),
      selectedChapter: state.selectedChapterId === chapterId ? updated : state.selectedChapter,
    }))
  },

  fetchEntities: async (documentId) => {
    set({ entitiesLoading: true })
    try {
      const entities = await api.listEntities(documentId)
      set({ entities, entitiesLoading: false })
    } catch {
      set({ entitiesLoading: false })
    }
  },

  fetchVolumes: async (documentId) => {
    set({ volumesLoading: true })
    try {
      const volumes = await api.listVolumes(documentId)
      set({ volumes, volumesLoading: false })
    } catch {
      set({ volumesLoading: false })
    }
  },

  fetchStats: async (documentId) => {
    set({ statsLoading: true })
    try {
      const stats = await api.getDocumentStats(documentId)
      set({ stats, statsLoading: false })
    } catch {
      set({ statsLoading: false })
    }
  },

  reset: () => set(initialState),
}))

// ============================================================================
// Selectors
// ============================================================================

export const useDocuments = () => useDocumentStore((state) => state.documents)
export const useSelectedDocument = () => useDocumentStore((state) => state.selectedDocument)
export const useDocumentChapters = () => useDocumentStore((state) => state.chapters)
export const useSelectedChapter = () => useDocumentStore((state) => state.selectedChapter)
export const useDocumentEntities = () => useDocumentStore((state) => state.entities)
export const useDocumentVolumes = () => useDocumentStore((state) => state.volumes)
export const useDocumentStats = () => useDocumentStore((state) => state.stats)
export const useDocumentsLoading = () => useDocumentStore((state) => state.documentsLoading)
