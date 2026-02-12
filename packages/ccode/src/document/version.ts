import { Storage } from "../storage/storage"
import { Identifier } from "../id/id"
import { Instance } from "../project/instance"
import { DocumentSchema } from "./schema"
import { Document } from "./index"
import { Entity } from "./entity"

const state = Instance.state(async () => ({}))

const STORAGE_PREFIX = "document_snapshot"

// Store full snapshots periodically (every N incremental snapshots)
const FULL_SNAPSHOT_INTERVAL = 10

export namespace Version {
  export async function createSnapshot(input: {
    documentID: string
    message: string
    createFull?: boolean
  }): Promise<DocumentSchema.Snapshot> {
    await state()

    const doc = await Document.get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Document.Chapter.list(input.documentID)
    const entities = await Entity.list(input.documentID)

    // Determine if we should create a full snapshot
    const existingSnapshots = await list(input.documentID)
    const shouldCreateFull = input.createFull ?? existingSnapshots.length % FULL_SNAPSHOT_INTERVAL === 0

    const id = Identifier.create("snapshot" as const, false)

    if (shouldCreateFull) {
      // Full snapshot - store all chapters and entities
      const chapterDeltas: DocumentSchema.ChapterDelta[] = chapters.map((ch) => ({
        chapterID: ch.id,
        action: "updated",
        content: ch.content,
        summary: ch.summary,
        wordCount: ch.wordCount,
      }))

      const entityDeltas: DocumentSchema.EntityDelta[] = entities.map((e: DocumentSchema.Entity) => ({
        entityID: e.id,
        action: "updated",
        data: e,
      }))

      const snapshot: DocumentSchema.Snapshot = {
        id,
        documentID: input.documentID,
        message: input.message,
        timestamp: Date.now(),
        chapterDeltas,
        globalSummary: doc.globalSummary,
        entityDeltas,
        chapterCount: chapters.length,
        totalWords: doc.currentWords,
      }

      await Storage.write([STORAGE_PREFIX, input.documentID, id], snapshot)
      return snapshot
    }

    // Incremental snapshot - find baseline and store only changes
    const baselineSnapshot = findBestBaseline(existingSnapshots)

    const chapterDeltas: DocumentSchema.ChapterDelta[] = []
    const entityDeltas: DocumentSchema.EntityDelta[] = []

    // If we have a baseline, compare to find changes
    if (baselineSnapshot) {
      const baselineChapterIDs = new Set(baselineSnapshot.chapterDeltas.map((d) => d.chapterID))
      const baselineEntityIDs = new Set(baselineSnapshot.entityDeltas?.map((d) => d.entityID) ?? [])

      // Find new, updated, or deleted chapters
      for (const chapter of chapters) {
        const baselineDelta = baselineSnapshot.chapterDeltas.find((d) => d.chapterID === chapter.id)

        if (!baselineDelta) {
          // New chapter
          chapterDeltas.push({
            chapterID: chapter.id,
            action: "created",
            content: chapter.content,
            summary: chapter.summary,
            wordCount: chapter.wordCount,
          })
          continue
        }

        // Check if updated
        const isUpdated =
          chapter.content !== baselineDelta.content ||
          chapter.summary !== baselineDelta.summary ||
          chapter.wordCount !== baselineDelta.wordCount

        if (isUpdated) {
          chapterDeltas.push({
            chapterID: chapter.id,
            action: "updated",
            content: chapter.content,
            summary: chapter.summary,
            wordCount: chapter.wordCount,
          })
        }
      }

      // Find deleted chapters
      for (const baselineDelta of baselineSnapshot.chapterDeltas) {
        const stillExists = chapters.find((c) => c.id === baselineDelta.chapterID)
        if (!stillExists) {
          chapterDeltas.push({
            chapterID: baselineDelta.chapterID,
            action: "deleted",
          })
        }
      }

      // Find new, updated, or deleted entities
      for (const entity of entities) {
        const baselineDelta = baselineSnapshot.entityDeltas?.find((d) => d.entityID === entity.id)

        if (!baselineDelta) {
          // New entity
          entityDeltas.push({
            entityID: entity.id,
            action: "created",
            data: entity,
          })
          continue
        }

        // Check if updated (simple comparison)
        const isUpdated = JSON.stringify(entity) !== JSON.stringify(baselineDelta.data)

        if (isUpdated) {
          entityDeltas.push({
            entityID: entity.id,
            action: "updated",
            data: entity,
          })
        }
      }

      // Find deleted entities
      for (const baselineDelta of baselineSnapshot.entityDeltas ?? []) {
        const stillExists = entities.find((e: DocumentSchema.Entity) => e.id === baselineDelta.entityID)
        if (!stillExists) {
          entityDeltas.push({
            entityID: baselineDelta.entityID,
            action: "deleted",
          })
        }
      }

      // Check if global summary changed
      const globalSummaryChanged =
        JSON.stringify(doc.globalSummary) !== JSON.stringify(baselineSnapshot.globalSummary)
    } else {
      // No baseline - store everything (fallback to full snapshot)
      return createSnapshot({
        documentID: input.documentID,
        message: input.message,
        createFull: true,
      })
    }

    const snapshot: DocumentSchema.Snapshot = {
      id,
      documentID: input.documentID,
      message: input.message,
      timestamp: Date.now(),
      baselineSnapshotID: baselineSnapshot?.id,
      chapterDeltas,
      entityDeltas,
      globalSummary: doc.globalSummary,
      chapterCount: chapters.length,
      totalWords: doc.currentWords,
    }

    await Storage.write([STORAGE_PREFIX, input.documentID, id], snapshot)
    return snapshot
  }

  export async function get(documentID: string, snapshotID: string): Promise<DocumentSchema.Snapshot | undefined> {
    await state()
    try {
      return await Storage.read<DocumentSchema.Snapshot>([STORAGE_PREFIX, documentID, snapshotID])
    } catch {
      return undefined
    }
  }

  export async function list(documentID: string): Promise<DocumentSchema.Snapshot[]> {
    await state()
    const keys = await Storage.list([STORAGE_PREFIX, documentID])
    const snapshots: DocumentSchema.Snapshot[] = []

    for (const key of keys) {
      const snapshot = await Storage.read<DocumentSchema.Snapshot>(key).catch(() => undefined)
      if (snapshot) snapshots.push(snapshot)
    }

    return snapshots.sort((a, b) => b.timestamp - a.timestamp)
  }

  /**
   * Rebuild document state from a snapshot (lazily applies deltas)
   */
  export async function rebuildFromSnapshot(
    documentID: string,
    snapshotID: string,
  ): Promise<{
    chapters: Array<{ id: string; title: string; content: string; summary?: string }>
    entities: DocumentSchema.Entity[]
    globalSummary?: DocumentSchema.GlobalSummary
  }> {
    const snapshot = await get(documentID, snapshotID)
    if (!snapshot) throw new Error("Snapshot not found")

    // Collect all deltas from baseline chain
    const allChapterDeltas: DocumentSchema.ChapterDelta[] = []
    const allEntityDeltas: DocumentSchema.EntityDelta[] = []

    let currentSnapshot: DocumentSchema.Snapshot | undefined = snapshot

    while (currentSnapshot) {
      // Prepend deltas (earlier first)
      allChapterDeltas.unshift(...currentSnapshot.chapterDeltas)
      if (currentSnapshot.entityDeltas) {
        allEntityDeltas.unshift(...currentSnapshot.entityDeltas)
      }

      // Follow baseline chain
      if (currentSnapshot.baselineSnapshotID) {
        currentSnapshot = await get(documentID, currentSnapshot.baselineSnapshotID)
      } else {
        break
      }
    }

    // Apply chapter deltas
    const chaptersMap = new Map<string, DocumentSchema.ChapterDelta>()
    for (const delta of allChapterDeltas) {
      if (delta.action === "deleted") {
        chaptersMap.delete(delta.chapterID)
      } else {
        chaptersMap.set(delta.chapterID, delta)
      }
    }

    // Apply entity deltas
    const entitiesMap = new Map<string, DocumentSchema.Entity>()
    for (const delta of allEntityDeltas) {
      if (delta.action === "deleted") {
        entitiesMap.delete(delta.entityID)
      } else if (delta.data) {
        entitiesMap.set(delta.entityID, delta.data)
      }
    }

    // Fetch chapter titles from current state
    const currentChapters = await Document.Chapter.list(documentID)
    const chapterTitles = new Map(currentChapters.map((c) => [c.id, c.title]))

    const chapters = Array.from(chaptersMap.values()).map((delta) => ({
      id: delta.chapterID,
      title: chapterTitles.get(delta.chapterID) || "Unknown",
      content: delta.content || "",
      summary: delta.summary,
    }))

    return {
      chapters,
      entities: Array.from(entitiesMap.values()),
      globalSummary: snapshot.globalSummary,
    }
  }

  /**
   * Rollback document to a previous snapshot state
   */
  export async function rollback(input: {
    documentID: string
    snapshotID: string
    options?: {
      chapters?: boolean
      entities?: boolean
      globalSummary?: boolean
    }
  }): Promise<void> {
    await state()

    const opts = {
      chapters: true,
      entities: true,
      globalSummary: true,
      ...input.options,
    }

    const snapshotState = await rebuildFromSnapshot(input.documentID, input.snapshotID)

    // Rollback chapters
    if (opts.chapters) {
      for (const chapter of snapshotState.chapters) {
        try {
          await Document.Chapter.update({
            documentID: input.documentID,
            chapterID: chapter.id,
            content: chapter.content,
            summary: chapter.summary,
          })
        } catch (error) {
          // Log failure but continue with other chapters
          // Error: Failed to rollback chapter ${chapter.id}
        }
      }
    }

    // Rollback entities (recreate all entities from snapshot)
    if (opts.entities) {
      // First delete all current entities
      const currentEntities = await Entity.list(input.documentID)
      for (const entity of currentEntities) {
        try {
          await Entity.remove(input.documentID, entity.id)
        } catch {
          // Ignore
        }
      }

      // Recreate entities from snapshot
      for (const entity of snapshotState.entities) {
        try {
          await Entity.create({
            documentID: input.documentID,
            type: entity.type,
            name: entity.name,
            description: entity.description,
            firstAppearedChapterID: entity.firstAppearedChapterID,
            aliases: entity.aliases,
            attributes: entity.attributes,
          })
        } catch (error) {
          // Log failure but continue with other entities
          // Error: Failed to recreate entity ${entity.id}
        }
      }
    }

    // Rollback global summary
    if (opts.globalSummary && snapshotState.globalSummary) {
      await Document.update({
        documentID: input.documentID,
        globalSummary: snapshotState.globalSummary,
      })
    }
  }

  /**
   * Compare two snapshots and generate a diff
   */
  export async function diff(input: {
    documentID: string
    fromSnapshotID: string
    toSnapshotID?: string
  }): Promise<{
    chaptersChanged: Array<{
      chapterID: string
      title: string
      action: "created" | "updated" | "deleted"
      wordCountDiff?: number
    }>
    entitiesChanged: Array<{
      entityID: string
      name: string
      action: "created" | "updated" | "deleted"
    }>
    globalSummaryChanged: boolean
    timeDifference: number
  }> {
    const fromSnapshot = await get(input.documentID, input.fromSnapshotID)
    if (!fromSnapshot) throw new Error("From snapshot not found")

    let toSnapshot: DocumentSchema.Snapshot | undefined
    if (input.toSnapshotID) {
      toSnapshot = await get(input.documentID, input.toSnapshotID)
    }

    if (!toSnapshot) {
      // Compare with current state
      const chapters = await Document.Chapter.list(input.documentID)
      const entities = await Entity.list(input.documentID)
      const doc = await Document.get(input.documentID)

      // Create a virtual snapshot from current state
      toSnapshot = {
        id: "current",
        documentID: input.documentID,
        message: "Current state",
        timestamp: Date.now(),
        chapterDeltas: chapters.map((ch) => ({
          chapterID: ch.id,
          action: "updated" as const,
          content: ch.content,
          summary: ch.summary,
          wordCount: ch.wordCount,
        })),
        entityDeltas: entities.map((e) => ({
          entityID: e.id,
          action: "updated" as const,
          data: e,
        })),
        globalSummary: doc?.globalSummary,
        chapterCount: chapters.length,
        totalWords: doc?.currentWords ?? 0,
      }
    }

    const fromState = await rebuildFromSnapshot(input.documentID, fromSnapshot.id)
    const toState = await rebuildFromSnapshot(input.documentID, toSnapshot.id)

    // Get chapter titles
    const currentChapters = await Document.Chapter.list(input.documentID)
    const chapterTitles = new Map(currentChapters.map((c) => [c.id, c.title]))

    // Compare chapters
    const fromChapterMap = new Map(fromState.chapters.map((c) => [c.id, c]))
    const toChapterMap = new Map(toState.chapters.map((c) => [c.id, c]))

    const chaptersChanged: Array<{
      chapterID: string
      title: string
      action: "created" | "updated" | "deleted"
      wordCountDiff?: number
    }> = []

    // Find created and updated chapters
    for (const [id, chapter] of toChapterMap) {
      const fromChapter = fromChapterMap.get(id)

      if (!fromChapter) {
        chaptersChanged.push({
          chapterID: id,
          title: chapter.title,
          action: "created",
        })
      } else if (chapter.content !== fromChapter.content) {
        chaptersChanged.push({
          chapterID: id,
          title: chapter.title,
          action: "updated",
          wordCountDiff: (chapter.content.length - fromChapter.content.length),
        })
      }
    }

    // Find deleted chapters
    for (const [id, chapter] of fromChapterMap) {
      if (!toChapterMap.has(id)) {
        chaptersChanged.push({
          chapterID: id,
          title: chapter.title,
          action: "deleted",
        })
      }
    }

    // Compare entities
    const fromEntityMap = new Map(fromState.entities.map((e) => [e.id, e]))
    const toEntityMap = new Map(toState.entities.map((e) => [e.id, e]))

    const entitiesChanged: Array<{
      entityID: string
      name: string
      action: "created" | "updated" | "deleted"
    }> = []

    // Find created and updated entities
    for (const [id, entity] of toEntityMap) {
      const fromEntity = fromEntityMap.get(id)

      if (!fromEntity) {
        entitiesChanged.push({
          entityID: id,
          name: entity.name,
          action: "created",
        })
      } else if (JSON.stringify(entity) !== JSON.stringify(fromEntity)) {
        entitiesChanged.push({
          entityID: id,
          name: entity.name,
          action: "updated",
        })
      }
    }

    // Find deleted entities
    for (const [id, entity] of fromEntityMap) {
      if (!toEntityMap.has(id)) {
        entitiesChanged.push({
          entityID: id,
          name: entity.name,
          action: "deleted",
        })
      }
    }

    const globalSummaryChanged =
      JSON.stringify(fromState.globalSummary) !== JSON.stringify(toState.globalSummary)

    return {
      chaptersChanged,
      entitiesChanged,
      globalSummaryChanged,
      timeDifference: toSnapshot.timestamp - fromSnapshot.timestamp,
    }
  }

  /**
   * Delete old snapshots to save space
   */
  export async function prune(input: {
    documentID: string
    keepLast: number
    keepFullSnapshots?: boolean
  }): Promise<number> {
    const snapshots = await list(input.documentID)

    if (snapshots.length <= input.keepLast) return 0

    const toDelete = snapshots.slice(input.keepLast)

    // Keep at least one full snapshot
    const fullSnapshots = toDelete.filter((s) => !s.baselineSnapshotID)
    const fullSnapshotToKeep = fullSnapshots[fullSnapshots.length - 1]

    let deleted = 0
    for (const snapshot of toDelete) {
      // Don't delete if it's the last full snapshot and option is set
      if (input.keepFullSnapshots && snapshot.id === fullSnapshotToKeep?.id) {
        continue
      }

      // Don't delete if another snapshot depends on this one
      const dependents = snapshots.filter((s) => s.baselineSnapshotID === snapshot.id)
      if (dependents.length > 0) {
        continue
      }

      await Storage.remove([STORAGE_PREFIX, input.documentID, snapshot.id])
      deleted++
    }

    return deleted
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  function findBestBaseline(snapshots: DocumentSchema.Snapshot[]): DocumentSchema.Snapshot | undefined {
    // Find the most recent full snapshot
    const fullSnapshots = snapshots.filter((s) => !s.baselineSnapshotID)
    if (fullSnapshots.length > 0) {
      return fullSnapshots[0] // Most recent full snapshot
    }

    // Fallback to most recent snapshot
    return snapshots[0]
  }
}
