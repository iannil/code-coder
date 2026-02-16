/**
 * Document API Handlers
 *
 * Handles document/writing system operations:
 * - Document CRUD
 * - Chapter management
 * - Export functionality
 * - Statistics
 */

import type { RouteHandler } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { Document, Entity, Volume } from "@/document"

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    return "{}"
  }
  return await new Response(body).text()
}

// ============================================================================
// Document Handlers
// ============================================================================

/**
 * List all documents
 * GET /api/documents
 */
export const listDocuments: RouteHandler = async () => {
  try {
    const documents = await Document.list()
    return jsonResponse({ success: true, data: documents })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get a specific document
 * GET /api/documents/:id
 */
export const getDocument: RouteHandler = async (_req, params) => {
  try {
    const id = params.id
    if (!id) {
      return errorResponse("Document ID required", 400)
    }

    const doc = await Document.get(id)
    if (!doc) {
      return errorResponse("Document not found", 404)
    }

    return jsonResponse({ success: true, data: doc })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Create a new document
 * POST /api/documents
 */
export const createDocument: RouteHandler = async (req) => {
  try {
    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.title || !body.targetWords) {
      return errorResponse("Title and targetWords required", 400)
    }

    const doc = await Document.create({
      title: body.title,
      description: body.description,
      targetWords: body.targetWords,
      styleGuide: body.styleGuide,
    })

    return jsonResponse({ success: true, data: doc }, 201)
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Update a document
 * PUT /api/documents/:id
 */
export const updateDocument: RouteHandler = async (req, params) => {
  try {
    const id = params.id
    if (!id) {
      return errorResponse("Document ID required", 400)
    }

    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    await Document.update({
      documentID: id,
      ...body,
    })

    const updated = await Document.get(id)
    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Delete a document
 * DELETE /api/documents/:id
 */
export const deleteDocument: RouteHandler = async (_req, params) => {
  try {
    const id = params.id
    if (!id) {
      return errorResponse("Document ID required", 400)
    }

    await Document.remove(id)
    return jsonResponse({ success: true })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get document statistics
 * GET /api/documents/:id/stats
 */
export const getDocumentStats: RouteHandler = async (_req, params) => {
  try {
    const id = params.id
    if (!id) {
      return errorResponse("Document ID required", 400)
    }

    const stats = await Document.getStats(id)
    return jsonResponse({ success: true, data: stats })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Export document
 * GET /api/documents/:id/export
 */
export const exportDocument: RouteHandler = async (req, params) => {
  try {
    const id = params.id
    if (!id) {
      return errorResponse("Document ID required", 400)
    }

    const format = (req.url.searchParams.get("format") as "markdown" | "html") || "markdown"
    const content = await Document.exportDocument({ documentID: id, format })

    return {
      status: 200,
      headers: { "Content-Type": format === "html" ? "text/html" : "text/markdown" },
      body: content,
    }
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// Chapter Handlers
// ============================================================================

/**
 * List chapters for a document
 * GET /api/documents/:id/chapters
 */
export const listChapters: RouteHandler = async (_req, params) => {
  try {
    const documentID = params.id
    if (!documentID) {
      return errorResponse("Document ID required", 400)
    }

    const chapters = await Document.Chapter.list(documentID)
    return jsonResponse({ success: true, data: chapters })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get a specific chapter
 * GET /api/documents/:id/chapters/:chapterId
 */
export const getChapter: RouteHandler = async (_req, params) => {
  try {
    const { id: documentID, chapterId } = params
    if (!documentID || !chapterId) {
      return errorResponse("Document ID and Chapter ID required", 400)
    }

    const chapter = await Document.Chapter.get(documentID, chapterId)
    if (!chapter) {
      return errorResponse("Chapter not found", 404)
    }

    return jsonResponse({ success: true, data: chapter })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Update a chapter
 * PUT /api/documents/:id/chapters/:chapterId
 */
export const updateChapter: RouteHandler = async (req, params) => {
  try {
    const { id: documentID, chapterId } = params
    if (!documentID || !chapterId) {
      return errorResponse("Document ID and Chapter ID required", 400)
    }

    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    await Document.Chapter.update({
      documentID,
      chapterID: chapterId,
      content: body.content ?? "",
      summary: body.summary,
      status: body.status,
      volumeID: body.volumeID,
      mentionedEntityIDs: body.mentionedEntityIDs,
    })

    const updated = await Document.Chapter.get(documentID, chapterId)
    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// Entity Handlers
// ============================================================================

/**
 * List entities for a document
 * GET /api/documents/:id/entities
 */
export const listEntities: RouteHandler = async (_req, params) => {
  try {
    const documentID = params.id
    if (!documentID) {
      return errorResponse("Document ID required", 400)
    }

    const entities = await Entity.list(documentID)
    return jsonResponse({ success: true, data: entities })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Create an entity
 * POST /api/documents/:id/entities
 */
export const createEntity: RouteHandler = async (req, params) => {
  try {
    const documentID = params.id
    if (!documentID) {
      return errorResponse("Document ID required", 400)
    }

    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.name || !body.type || !body.description || !body.firstAppearedChapterID) {
      return errorResponse("Name, type, description, and firstAppearedChapterID required", 400)
    }

    const entity = await Entity.create({
      documentID,
      name: body.name,
      type: body.type,
      description: body.description,
      firstAppearedChapterID: body.firstAppearedChapterID,
      aliases: body.aliases,
      attributes: body.attributes,
    })

    return jsonResponse({ success: true, data: entity }, 201)
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Update an entity
 * PUT /api/documents/:id/entities/:entityId
 */
export const updateEntity: RouteHandler = async (req, params) => {
  try {
    const { id: documentID, entityId } = params
    if (!documentID || !entityId) {
      return errorResponse("Document ID and Entity ID required", 400)
    }

    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    await Entity.update({
      documentID,
      entityID: entityId,
      ...body,
    })

    const updated = await Entity.get(documentID, entityId)
    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Delete an entity
 * DELETE /api/documents/:id/entities/:entityId
 */
export const deleteEntity: RouteHandler = async (_req, params) => {
  try {
    const { id: documentID, entityId } = params
    if (!documentID || !entityId) {
      return errorResponse("Document ID and Entity ID required", 400)
    }

    await Entity.remove(documentID, entityId)
    return jsonResponse({ success: true })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// Volume Handlers
// ============================================================================

/**
 * List volumes for a document
 * GET /api/documents/:id/volumes
 */
export const listVolumes: RouteHandler = async (_req, params) => {
  try {
    const documentID = params.id
    if (!documentID) {
      return errorResponse("Document ID required", 400)
    }

    const volumes = await Volume.list(documentID)
    return jsonResponse({ success: true, data: volumes })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Create a volume
 * POST /api/documents/:id/volumes
 */
export const createVolume: RouteHandler = async (req, params) => {
  try {
    const documentID = params.id
    if (!documentID) {
      return errorResponse("Document ID required", 400)
    }

    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.title || !body.startChapterID || !body.endChapterID) {
      return errorResponse("Title, startChapterID, and endChapterID required", 400)
    }

    const volume = await Volume.create({
      documentID,
      title: body.title,
      description: body.description,
      startChapterID: body.startChapterID,
      endChapterID: body.endChapterID,
    })

    return jsonResponse({ success: true, data: volume }, 201)
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
