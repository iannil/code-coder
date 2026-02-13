import { Storage } from "../storage/storage"
import { Identifier } from "../id/id"
import { Instance } from "../project/instance"
import { DocumentSchema } from "./schema"
import { EOL } from "os"

// Re-export all new modules for convenient importing
export { Context } from "./context"
export { Summary } from "./summary"
export { Version } from "./version"
export { Editor } from "./editor"
export { Consistency } from "./consistency"
export { Proofreader } from "./proofreader"

// Import and re-export Entity and Volume namespaces
export { Entity } from "./entity"
export { Volume } from "./volume"

// Import and re-export Knowledge module (BookExpander)
export * as Knowledge from "./knowledge"

export namespace Document {
  const state = Instance.state(async () => {
    return {}
  })

  export async function create(input: {
    title: string
    description?: string
    targetWords: number
    styleGuide?: DocumentSchema.StyleGuide
  }): Promise<DocumentSchema.Metadata> {
    await state()
    const id = Identifier.create("document", false)

    const doc: DocumentSchema.Metadata = {
      id,
      projectID: Instance.project.id,
      title: input.title,
      description: input.description,
      status: "planning",
      targetWords: input.targetWords,
      currentWords: 0,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      outline: {
        title: input.title,
        description: input.description,
        chapters: [],
      },
      styleGuide: input.styleGuide,
      globalSummary: undefined,
      volumes: [],
    }

    await Storage.write(["document", Instance.project.id, id], doc)
    return doc
  }

  export async function get(id: string): Promise<DocumentSchema.Metadata | undefined> {
    await state()
    try {
      return await Storage.read<DocumentSchema.Metadata>(["document", Instance.project.id, id])
    } catch {
      return undefined
    }
  }

  export async function list(): Promise<DocumentSchema.Metadata[]> {
    await state()
    const keys = await Storage.list(["document", Instance.project.id])
    const docs: DocumentSchema.Metadata[] = []
    for (const key of keys) {
      const doc = await Storage.read<DocumentSchema.Metadata>(key).catch(() => undefined)
      if (doc) docs.push(doc)
    }
    return docs.sort((a, b) => b.updatedAt - a.updatedAt)
  }

  export async function updateOutline(input: {
    documentID: string
    outline: DocumentSchema.Outline
  }): Promise<void> {
    await state()
    const doc = await get(input.documentID)
    if (!doc) throw new Error("Document not found")

    // Use immutable update pattern
    const updated: DocumentSchema.Metadata = {
      ...doc,
      outline: input.outline,
      status: "writing",
      updatedAt: Date.now(),
    }

    await Storage.write(["document", Instance.project.id, input.documentID], updated)

    for (const chapter of input.outline.chapters) {
      await Chapter.create({
        documentID: input.documentID,
        outlineID: chapter.id,
        title: chapter.title,
      })
    }
  }

  export async function update(input: {
    documentID: string
    title?: string
    description?: string
    styleGuide?: DocumentSchema.StyleGuide
    status?: DocumentSchema.Status
    globalSummary?: DocumentSchema.GlobalSummary
    volumes?: string[]
  }): Promise<void> {
    await state()
    const doc = await get(input.documentID)
    if (!doc) throw new Error("Document not found")

    // Use immutable update pattern
    const updated: DocumentSchema.Metadata = {
      ...doc,
      ...(input.title && { title: input.title }),
      ...(input.description !== undefined && { description: input.description }),
      ...(input.styleGuide && { styleGuide: input.styleGuide }),
      ...(input.status && { status: input.status }),
      ...(input.globalSummary !== undefined && { globalSummary: input.globalSummary }),
      ...(input.volumes && { volumes: input.volumes }),
      updatedAt: Date.now(),
    }

    await Storage.write(["document", Instance.project.id, input.documentID], updated)
  }

  export async function remove(id: string): Promise<void> {
    await state()
    await Storage.remove(["document", Instance.project.id, id])
  }

  export namespace Chapter {
    export async function remove(documentID: string, chapterID: string): Promise<void> {
      await state()
      await Storage.remove(["document_chapter", documentID, chapterID])
    }
    export async function create(input: {
      documentID: string
      outlineID: string
      title: string
    }): Promise<DocumentSchema.Chapter> {
      await state()
      const id = Identifier.create("chapter", false)

      const chapter: DocumentSchema.Chapter = {
        id,
        documentID: input.documentID,
        outlineID: input.outlineID,
        title: input.title,
        status: "pending",
        content: "",
        wordCount: 0,
        createdAt: Date.now(),
        updatedAt: Date.now(),
        volumeID: undefined,
        mentionedEntityIDs: [],
      }

      await Storage.write(["document_chapter", input.documentID, id], chapter)
      return chapter
    }

    export async function get(documentID: string, chapterID: string): Promise<DocumentSchema.Chapter | undefined> {
      await state()
      try {
        return await Storage.read<DocumentSchema.Chapter>(["document_chapter", documentID, chapterID])
      } catch {
        return undefined
      }
    }

    export async function list(documentID: string): Promise<DocumentSchema.Chapter[]> {
      await state()
      const keys = await Storage.list(["document_chapter", documentID])
      const chapters: DocumentSchema.Chapter[] = []
      for (const key of keys) {
        const chapter = await Storage.read<DocumentSchema.Chapter>(key).catch(() => undefined)
        if (chapter) chapters.push(chapter)
      }
      const doc = await Document.get(documentID)
      if (!doc) return chapters
      const order = new Map(doc.outline.chapters.map((c, i) => [c.id, i]))
      return chapters.sort((a, b) => (order.get(a.outlineID) ?? 0) - (order.get(b.outlineID) ?? 0))
    }

    export async function update(input: {
      documentID: string
      chapterID: string
      content: string
      summary?: string
      status?: DocumentSchema.ChapterStatus
      volumeID?: string
      mentionedEntityIDs?: string[]
    }): Promise<void> {
      await state()
      const chapter = await get(input.documentID, input.chapterID)
      if (!chapter) throw new Error("Chapter not found")

      // Use immutable update pattern
      const updated: DocumentSchema.Chapter = {
        ...chapter,
        content: input.content,
        summary: input.summary ?? chapter.summary,
        status: input.status ?? chapter.status,
        wordCount: countWords(input.content),
        updatedAt: Date.now(),
        ...(input.volumeID !== undefined && { volumeID: input.volumeID }),
        ...(input.mentionedEntityIDs && { mentionedEntityIDs: input.mentionedEntityIDs }),
      }

      await Storage.write(["document_chapter", input.documentID, input.chapterID], updated)

      const doc = await Document.get(input.documentID)
      if (doc) {
        const chapters = await list(input.documentID)
        // Use immutable update pattern
        const completed = chapters.every((c) => c.status === "completed")
        const updatedDoc: DocumentSchema.Metadata = {
          ...doc,
          currentWords: chapters.reduce((sum, c) => sum + c.wordCount, 0),
          updatedAt: Date.now(),
          ...(completed && { status: "completed" }),
        }

        await Storage.write(["document", Instance.project.id, input.documentID], updatedDoc)
      }
    }

    export async function getPendingContext(documentID: string, currentOutlineID: string): Promise<{
      summaries: string
      recentContent: string
    }> {
      await state()
      const chapters = await list(documentID)
      const summaries: string[] = []
      const recentContent: string[] = []
      let foundCurrent = false
      let recentCount = 0

      for (const chapter of chapters) {
        if (chapter.outlineID === currentOutlineID) {
          foundCurrent = true
          break
        }
        if (chapter.summary) {
          summaries.push(`## ${chapter.title}\n${chapter.summary}`)
        }
        if (chapter.status === "completed" && recentCount < 2) {
          recentContent.push(`## ${chapter.title}\n${chapter.content}`)
          recentCount++
        }
      }

      return {
        summaries: summaries.join("\n\n"),
        recentContent: recentContent.join("\n\n---\n\n"),
      }
    }
  }

  export async function exportDocument(input: {
    documentID: string
    format: "markdown" | "html"
  }): Promise<string> {
    await state()
    const doc = await get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Chapter.list(input.documentID)

    if (input.format === "markdown") {
      const lines: string[] = []
      lines.push(`# ${doc.title}`)
      if (doc.description) lines.push("", doc.description)
      lines.push("", "---", "")

      lines.push("## 目录", "")
      for (let i = 0; i < chapters.length; i++) {
        const chapter = chapters[i]
        lines.push(`${i + 1}. [${chapter.title}](#chapter-${i + 1})`)
      }
      lines.push("", "---", "")

      for (let i = 0; i < chapters.length; i++) {
        const chapter = chapters[i]
        lines.push(`## 第 ${i + 1} 章 ${chapter.title}`, "")
        lines.push(chapter.content, "")
      }

      return lines.join("\n")
    }

    throw new Error("HTML export not implemented yet")
  }

  function countWords(text: string): number {
    const chineseChars = (text.match(/[\u4e00-\u9fa5]/g) || []).length
    const englishWords = (text.match(/[a-zA-Z]+/g) || []).length
    return chineseChars + englishWords
  }

  export async function getStats(documentID: string): Promise<{
    totalChapters: number
    completedChapters: number
    pendingChapters: number
    totalWords: number
    targetWords: number
    progress: number
    estimatedRemaining: number
  }> {
    await state()
    const doc = await get(documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Chapter.list(documentID)
    const completed = chapters.filter((c) => c.status === "completed").length
    const pending = chapters.filter((c) => c.status === "pending" || c.status === "drafting").length

    const outlineWordCounts = doc.outline.chapters.map((c) => {
      const chapter = chapters.find((ch) => ch.outlineID === c.id)
      return chapter?.wordCount ?? 0
    })

    const completedOutlineWords = doc.outline.chapters
      .filter((c) => chapters.find((ch) => ch.outlineID === c.id && ch.status === "completed"))
      .reduce((sum, c) => sum + c.estimatedWords, 0)

    const remainingOutlineWords = doc.targetWords - completedOutlineWords

    return {
      totalChapters: chapters.length,
      completedChapters: completed,
      pendingChapters: pending,
      totalWords: doc.currentWords,
      targetWords: doc.targetWords,
      progress: doc.targetWords > 0 ? Math.round((doc.currentWords / doc.targetWords) * 100) : 0,
      estimatedRemaining: Math.max(0, remainingOutlineWords),
    }
  }

  export async function regenerateSummary(input: {
    documentID: string
    chapterID: string
  }): Promise<string> {
    await state()
    const chapter = await Chapter.get(input.documentID, input.chapterID)
    if (!chapter) throw new Error("Chapter not found")
    if (!chapter.content) throw new Error("Chapter has no content")

    const doc = await get(input.documentID)
    const outline = doc?.outline.chapters.find((c) => c.id === chapter.outlineID)

    const prompt = `Generate a concise 200-300 word summary in Chinese for the following chapter.

Chapter: ${chapter.title}
${outline ? `Description: ${outline.description}` : ""}

Content:
${chapter.content.slice(0, 5000)}

Please provide:
1. Main topics covered
2. Key points and arguments
3. Characters or concepts introduced
4. Any foreshadowing or setup for future chapters

Summary (in Chinese):`

    return prompt
  }
}
