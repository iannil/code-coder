import { Storage } from "../storage/storage"
import { Identifier } from "../id/id"
import { Instance } from "../project/instance"
import { DocumentSchema } from "./schema"
import { Document } from "./index"

const state = Instance.state(async () => ({}))

const STORAGE_PREFIX = "document_volume"

export namespace Volume {
  export async function create(input: {
    documentID: string
    title: string
    description?: string
    startChapterID: string
    endChapterID: string
  }): Promise<DocumentSchema.Volume> {
    await state()

    // Validate chapters exist
    const chapters = await Document.Chapter.list(input.documentID)
    const startChapter = chapters.find((c) => c.id === input.startChapterID)
    const endChapter = chapters.find((c) => c.id === input.endChapterID)

    if (!startChapter) throw new Error("Start chapter not found")
    if (!endChapter) throw new Error("End chapter not found")

    // Determine order based on chapter position
    const startIndex = chapters.findIndex((c) => c.id === input.startChapterID)
    const endIndex = chapters.findIndex((c) => c.id === input.endChapterID)

    if (startIndex > endIndex) {
      throw new Error("Start chapter must come before end chapter")
    }

    // Get current max order for this document
    const existingVolumes = await list(input.documentID)
    const maxOrder = existingVolumes.length > 0
      ? Math.max(...existingVolumes.map((v) => v.order))
      : -1

    const id = Identifier.create("volume" as const, false)

    const volume: DocumentSchema.Volume = {
      id,
      documentID: input.documentID,
      title: input.title,
      description: input.description,
      startChapterID: input.startChapterID,
      endChapterID: input.endChapterID,
      order: maxOrder + 1,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    }

    await Storage.write([STORAGE_PREFIX, input.documentID, id], volume)

    // Update document's volume list
    const doc = await Document.get(input.documentID)
    if (doc) {
      const updatedVolumes = [...doc.volumes, id]
      await Document.update({ documentID: input.documentID, volumes: updatedVolumes })
    }

    return volume
  }

  export async function get(documentID: string, volumeID: string): Promise<DocumentSchema.Volume | undefined> {
    await state()
    try {
      return await Storage.read<DocumentSchema.Volume>([STORAGE_PREFIX, documentID, volumeID])
    } catch {
      return undefined
    }
  }

  export async function list(documentID: string): Promise<DocumentSchema.Volume[]> {
    await state()
    const keys = await Storage.list([STORAGE_PREFIX, documentID])
    const volumes: DocumentSchema.Volume[] = []

    for (const key of keys) {
      const volume = await Storage.read<DocumentSchema.Volume>(key).catch(() => undefined)
      if (volume) volumes.push(volume)
    }

    return volumes.sort((a, b) => a.order - b.order)
  }

  export async function update(input: {
    documentID: string
    volumeID: string
    title?: string
    description?: string
    summary?: string
  }): Promise<void> {
    await state()
    const volume = await get(input.documentID, input.volumeID)
    if (!volume) throw new Error("Volume not found")

    // Use immutable update pattern
    const updated: DocumentSchema.Volume = {
      ...volume,
      ...(input.title && { title: input.title }),
      ...(input.description !== undefined && { description: input.description }),
      ...(input.summary && { summary: input.summary }),
      updatedAt: Date.now(),
    }

    await Storage.write([STORAGE_PREFIX, input.documentID, input.volumeID], updated)
  }

  export async function remove(documentID: string, volumeID: string): Promise<void> {
    await state()

    // Update document's volume list
    const doc = await Document.get(documentID)
    if (doc && doc.volumes.includes(volumeID)) {
      const updatedVolumes = doc.volumes.filter((id) => id !== volumeID)
      await Document.update({ documentID, volumes: updatedVolumes })
    }

    await Storage.remove([STORAGE_PREFIX, documentID, volumeID])
  }

  /**
   * Get all chapters in a volume
   */
  export async function getChapters(
    documentID: string,
    volumeID: string,
  ): Promise<DocumentSchema.Chapter[]> {
    await state()
    const volume = await get(documentID, volumeID)
    if (!volume) throw new Error("Volume not found")

    const chapters = await Document.Chapter.list(documentID)
    const startIndex = chapters.findIndex((c) => c.id === volume.startChapterID)
    const endIndex = chapters.findIndex((c) => c.id === volume.endChapterID)

    if (startIndex < 0 || endIndex < 0) {
      return []
    }

    return chapters.slice(startIndex, endIndex + 1)
  }

  /**
   * Get volume progress statistics
   */
  export async function getProgress(
    documentID: string,
    volumeID: string,
  ): Promise<{
    totalChapters: number
    completedChapters: number
    totalWords: number
    completionPercentage: number
  }> {
    const chapters = await getChapters(documentID, volumeID)

    const completed = chapters.filter((c) => c.status === "completed")
    const totalWords = chapters.reduce((sum, c) => sum + c.wordCount, 0)

    return {
      totalChapters: chapters.length,
      completedChapters: completed.length,
      totalWords,
      completionPercentage: chapters.length > 0
        ? Math.round((completed.length / chapters.length) * 100)
        : 0,
    }
  }

  /**
   * Get progress for all volumes in a document
   */
  export async function getAllVolumeProgress(documentID: string): Promise<
    Array<{
      volume: DocumentSchema.Volume
      progress: {
        totalChapters: number
        completedChapters: number
        totalWords: number
        completionPercentage: number
      }
    }>
  > {
    const volumes = await list(documentID)
    const result = []

    for (const volume of volumes) {
      const progress = await getProgress(documentID, volume.id)
      result.push({ volume, progress })
    }

    return result
  }

  /**
   * Generate a prompt for AI to create a volume summary
   */
  export async function generateSummaryPrompt(documentID: string, volumeID: string): Promise<string> {
    const volume = await get(documentID, volumeID)
    if (!volume) throw new Error("Volume not found")

    const chapters = await getChapters(documentID, volumeID)
    const doc = await Document.get(documentID)

    const lines: string[] = []

    lines.push("# Volume Summary Generation")
    lines.push("")
    lines.push("Generate a comprehensive summary for this volume/part of the document.")
    lines.push("")
    lines.push("## Volume Information")
    lines.push("")
    lines.push(`**Title:** ${volume.title}`)
    if (volume.description) lines.push(`**Description:** ${volume.description}`)
    lines.push(`**Chapters:** ${chapters.length}`)
    lines.push("")
    lines.push("## Volume Contents")
    lines.push("")

    for (const chapter of chapters) {
      const outline = doc?.outline.chapters.find((c) => c.id === chapter.outlineID)
      lines.push(`### ${chapter.title}`)
      if (outline) lines.push(`${outline.description}`)
      if (chapter.summary) lines.push(`\nSummary: ${chapter.summary}`)
      lines.push("")
    }

    // Include full content of key chapters (first, middle, last)
    lines.push("---")
    lines.push("")
    lines.push("## Key Chapter Content")
    lines.push("")

    const keyChapters = [
      chapters[0],
      chapters[Math.floor(chapters.length / 2)],
      chapters[chapters.length - 1],
    ].filter((c): c is DocumentSchema.Chapter => c !== undefined)

    for (const chapter of keyChapters) {
      lines.push(`### ${chapter.title}`)
      lines.push("")
      // Include up to 3000 chars from key chapters
      const content = chapter.content.slice(0, 3000)
      lines.push(content)
      if (chapter.content.length > 3000) {
        lines.push("")
        lines.push("[Content truncated...]")
      }
      lines.push("")
    }

    lines.push("## Instructions")
    lines.push("")
    lines.push("Generate a 200-400 word summary that captures:")
    lines.push("")
    lines.push("- The main plot developments in this volume")
    lines.push("- Key character arcs or progressions")
    lines.push("- Important themes introduced or developed")
    lines.push("- How this volume connects to the overall story")
    lines.push("- Any cliffhangers or setup for the next volume")

    return lines.join("\n")
  }

  /**
   * Automatically create volumes based on chapter groupings
   */
  export async function autoCreate(input: {
    documentID: string
    chaptersPerVolume: number
    namingPattern?: "roman" | "number" | "custom"
    customPrefix?: string
  }): Promise<DocumentSchema.Volume[]> {
    const chapters = await Document.Chapter.list(input.documentID)
    if (chapters.length === 0) return []

    const volumes: DocumentSchema.Volume[] = []
    const totalVolumes = Math.ceil(chapters.length / input.chaptersPerVolume)

    for (let i = 0; i < totalVolumes; i++) {
      const startIndex = i * input.chaptersPerVolume
      const endIndex = Math.min((i + 1) * input.chaptersPerVolume - 1, chapters.length - 1)

      let title = ""
      if (input.namingPattern === "roman") {
        const romanNumerals = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"]
        title = `Volume ${romanNumerals[i] || i + 1}`
      } else if (input.namingPattern === "custom" && input.customPrefix) {
        title = `${input.customPrefix} ${i + 1}`
      } else {
        title = `Volume ${i + 1}`
      }

      try {
        const volume = await create({
          documentID: input.documentID,
          title,
          startChapterID: chapters[startIndex].id,
          endChapterID: chapters[endIndex].id,
        })
        volumes.push(volume)
      } catch (error) {
        // Log error but continue with other volumes
        // Error is thrown to caller for handling
        throw new Error(`Failed to create volume ${i + 1}: ${error}`)
      }
    }

    return volumes
  }

  /**
   * Reorder volumes
   */
  export async function reorder(input: {
    documentID: string
    volumeOrders: Array<{ volumeID: string; newOrder: number }>
  }): Promise<void> {
    await state()

    for (const { volumeID, newOrder } of input.volumeOrders) {
      const volume = await get(input.documentID, volumeID)
      if (!volume) continue

      volume.order = newOrder
      volume.updatedAt = Date.now()

      await Storage.write([STORAGE_PREFIX, input.documentID, volumeID], volume)
    }
  }
}
