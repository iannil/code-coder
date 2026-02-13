import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { TuiEvent } from "@/cli/cmd/tui/event"

const log = Log.create({ service: "writer-service" })

export namespace WriterService {
  export type Chapter = {
    title: string
    number: number
    outline: string
    content: string
    summary: string
    wordCount: number
  }

  export type Outline = {
    title: string
    description: string
    chapters: Array<{
      number: number
      title: string
      outline: string
      estimatedWords: number
    }>
    totalEstimatedWords: number
  }

  export type ProgressState = {
    outline?: Outline
    completedChapters: number
    totalChapters: number
    currentChapter?: number
    lastUpdate: number
  }

  const state: ProgressState = {
    completedChapters: 0,
    totalChapters: 0,
    lastUpdate: Date.now(),
  }

  /**
   * Report progress for long-form writing tasks
   */
  export function reportProgress(data: {
    action: "outline" | "chapter_start" | "chapter_complete" | "complete" | "error"
    chapter?: number
    total?: number
    message?: string
  }) {
    const timestamp = Date.now()
    state.lastUpdate = timestamp

    let message = ""
    let variant: "info" | "success" | "warning" | "error" = "info"

    switch (data.action) {
      case "outline":
        message = `📋 Outline generated: ${data.total || 0} chapters planned`
        state.totalChapters = data.total || 0
        variant = "info"
        break
      case "chapter_start":
        message = `✍️  Starting Chapter ${data.chapter}/${state.totalChapters}`
        state.currentChapter = data.chapter
        variant = "info"
        break
      case "chapter_complete":
        message = `✅ Chapter ${data.chapter}/${state.totalChapters} complete`
        state.completedChapters = data.chapter || 0
        variant = "success"
        break
      case "complete":
        message = `🎉 Writing complete! ${state.completedChapters} chapters written`
        variant = "success"
        break
      case "error":
        message = `⚠️ Error: ${data.message || "Unknown error"}`
        variant = "error"
        break
    }

    if (data.message && data.action !== "error") {
      message += ` - ${data.message}`
    }

    log.info("Writer progress", {
      action: data.action,
      chapter: data.chapter,
      total: data.total,
    })

    // Publish WriterProgress event for tracking (toast is handled by app.tsx)
    Bus.publish(TuiEvent.WriterProgress, {
      action: data.action,
      chapter: data.chapter,
      total: data.total,
      message: data.message,
    })
  }

  /**
   * Parse progress markers from AI output
   */
  export function parseProgressMarkers(content: string): {
    hasMarkers: boolean
    progress: number
    total: number
  } {
    const markerRegex = /<!--\s*PROGRESS:\s*(\d+)\/(\d+)\s+chapters\s*-->/gi
    const matches = Array.from(content.matchAll(markerRegex))

    if (matches.length === 0) {
      return { hasMarkers: false, progress: 0, total: 0 }
    }

    const lastMatch = matches[matches.length - 1]
    return {
      hasMarkers: true,
      progress: parseInt(lastMatch[1] || "0"),
      total: parseInt(lastMatch[2] || "0"),
    }
  }

  /**
   * Calculate estimated completion time based on current progress
   */
  export function estimateCompletion(
    startTime: number,
    progress: number,
    total: number,
  ): { elapsed: number; remaining: number; eta: Date } | null {
    if (progress === 0 || total === 0) {
      return null
    }

    const elapsed = Date.now() - startTime
    const msPerChapter = elapsed / progress
    const remaining = (total - progress) * msPerChapter
    const eta = new Date(Date.now() + remaining)

    return { elapsed, remaining, eta }
  }

  /**
   * Validate chapter content before saving
   */
  export function validateChapter(content: string, minWords: number = 500): {
    valid: boolean
    wordCount: number
    issues: string[]
  } {
    const issues: string[] = []
    const wordCount = content.split(/\s+/).filter((w) => w.length > 0).length

    if (wordCount < minWords) {
      issues.push(`Chapter too short: ${wordCount} words (minimum ${minWords})`)
    }

    if (content.length < 100) {
      issues.push("Chapter content appears incomplete")
    }

    // Check for incomplete sentences
    if (content.trim().endsWith("...") && wordCount < 100) {
      issues.push("Chapter appears to be cut off mid-sentence")
    }

    return {
      valid: issues.length === 0,
      wordCount,
      issues,
    }
  }

  /**
   * Suggest chunk size for content generation
   */
  export function suggestChunkSize(totalWords: number, maxTokens: number = 64_000): {
    chapters: number
    wordsPerChapter: number
    needsSplitting: boolean
  } {
    // Assume ~1.3 tokens per word
    const maxWordsPerChunk = Math.floor((maxTokens * 0.8) / 1.3)
    const optimalChapters = Math.max(3, Math.ceil(totalWords / 3000))
    const wordsPerChapter = Math.ceil(totalWords / optimalChapters)

    return {
      chapters: optimalChapters,
      wordsPerChapter: Math.min(wordsPerChapter, maxWordsPerChunk),
      needsSplitting: totalWords > maxWordsPerChunk,
    }
  }

  /**
   * Get current progress state
   */
  export function getState(): Readonly<ProgressState> {
    return { ...state }
  }

  /**
   * Reset progress state
   */
  export function resetState() {
    state.completedChapters = 0
    state.totalChapters = 0
    state.currentChapter = undefined
    state.lastUpdate = Date.now()
    log.info("Writer state reset")
  }
}
