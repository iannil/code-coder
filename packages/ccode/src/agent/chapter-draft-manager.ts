import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { TuiEvent } from "@/cli/cmd/tui/event"
import * as fs from "fs/promises"
import * as path from "path"

const log = Log.create({ service: "chapter-draft-manager" })

/**
 * ChapterDraftManager - Manages incremental saving of chapter content during generation
 *
 * When generating long chapters (10000+ words), this manager periodically saves
 * the accumulated content to a draft file to prevent progress loss.
 */
export namespace ChapterDraftManager {
  const SAVE_INTERVAL_MS = 60_000 // Save every 60 seconds
  const SAVE_WORD_THRESHOLD = 2000 // Save when 2000 new words accumulated

  interface DraftSession {
    sessionID: string
    chapterPath: string
    lastSavedContent: string
    lastSavedWords: number
    lastSaveTime: number
    saveCount: number
  }

  const activeDrafts: Map<string, DraftSession> = new Map()
  const saveIntervals: Map<string, ReturnType<typeof setInterval>> = new Map()

  /**
   * Start tracking a chapter draft
   */
  export function start(input: {
    sessionID: string
    chapterPath: string
  }): void {
    const draft: DraftSession = {
      sessionID: input.sessionID,
      chapterPath: input.chapterPath,
      lastSavedContent: "",
      lastSavedWords: 0,
      lastSaveTime: Date.now(),
      saveCount: 0,
    }
    activeDrafts.set(input.sessionID, draft)

    log.info("Chapter draft manager started", {
      sessionID: input.sessionID,
      chapterPath: input.chapterPath,
    })

    // Start periodic save check
    const interval = setInterval(() => {
      checkAndSave(input.sessionID)
    }, SAVE_INTERVAL_MS)
    saveIntervals.set(input.sessionID, interval)
  }

  /**
   * Update draft content (call this when new content is generated)
   */
  export async function updateContent(
    sessionID: string,
    content: string,
    options?: { force?: boolean },
  ): Promise<boolean> {
    const draft = activeDrafts.get(sessionID)
    if (!draft) return false

    const currentWords = countWords(content)
    const wordsSinceLastSave = currentWords - draft.lastSavedWords
    const timeSinceLastSave = Date.now() - draft.lastSaveTime

    // Check if we should save
    const shouldSave =
      options?.force ||
      wordsSinceLastSave >= SAVE_WORD_THRESHOLD ||
      (timeSinceLastSave >= SAVE_INTERVAL_MS && wordsSinceLastSave > 500)

    if (shouldSave && content !== draft.lastSavedContent) {
      return await saveDraft(sessionID, content)
    }

    return false
  }

  /**
   * Save draft to file
   */
  async function saveDraft(sessionID: string, content: string): Promise<boolean> {
    const draft = activeDrafts.get(sessionID)
    if (!draft) return false

    try {
      // Ensure directory exists
      const dir = path.dirname(draft.chapterPath)
      await fs.mkdir(dir, { recursive: true })

      // Write draft with .draft suffix
      const draftPath = draft.chapterPath + ".draft"
      await fs.writeFile(draftPath, content, "utf-8")

      const currentWords = countWords(content)
      draft.lastSavedContent = content
      draft.lastSavedWords = currentWords
      draft.lastSaveTime = Date.now()
      draft.saveCount++

      log.info("Draft saved", {
        sessionID,
        words: currentWords,
        saveCount: draft.saveCount,
        path: draftPath,
      })

      // Publish save event
      Bus.publish(TuiEvent.ChapterDraftSaved, {
        sessionID,
        chapterPath: draft.chapterPath,
        wordsWritten: currentWords,
        saveCount: draft.saveCount,
      })

      return true
    } catch (error) {
      log.error("Failed to save draft", {
        sessionID,
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Check and save if needed (called periodically)
   */
  function checkAndSave(sessionID: string): void {
    const draft = activeDrafts.get(sessionID)
    if (!draft) return

    // This will be called by WriterStatsMonitor with the current content
    // For now, just log that we're checking
    log.debug("Checking draft save status", {
      sessionID,
      lastSaveTime: draft.lastSaveTime,
      saveCount: draft.saveCount,
    })
  }

  /**
   * Finalize draft - move from .draft to final file
   */
  export async function finalize(sessionID: string, finalContent: string): Promise<boolean> {
    const draft = activeDrafts.get(sessionID)
    if (!draft) return false

    try {
      // Ensure directory exists
      const dir = path.dirname(draft.chapterPath)
      await fs.mkdir(dir, { recursive: true })

      // Write final content
      await fs.writeFile(draft.chapterPath, finalContent, "utf-8")

      // Remove draft file if it exists
      const draftPath = draft.chapterPath + ".draft"
      await fs.unlink(draftPath).catch(() => {
        // Ignore if draft doesn't exist
      })

      const finalWords = countWords(finalContent)

      log.info("Draft finalized", {
        sessionID,
        words: finalWords,
        totalSaves: draft.saveCount,
        path: draft.chapterPath,
      })

      // Publish completion event
      Bus.publish(TuiEvent.ChapterDraftFinalized, {
        sessionID,
        chapterPath: draft.chapterPath,
        wordsWritten: finalWords,
        totalSaves: draft.saveCount,
      })

      return true
    } catch (error) {
      log.error("Failed to finalize draft", {
        sessionID,
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Stop tracking and clean up
   */
  export function stop(sessionID: string): void {
    const draft = activeDrafts.get(sessionID)
    if (draft) {
      log.info("Chapter draft manager stopped", {
        sessionID,
        totalSaves: draft.saveCount,
      })
    }

    // Clean up interval
    const interval = saveIntervals.get(sessionID)
    if (interval) {
      clearInterval(interval)
      saveIntervals.delete(sessionID)
    }

    activeDrafts.delete(sessionID)
  }

  /**
   * Get draft info
   */
  export function getDraft(sessionID: string): DraftSession | undefined {
    return activeDrafts.get(sessionID)
  }

  /**
   * Recover from draft file if exists
   */
  export async function recoverDraft(chapterPath: string): Promise<string | undefined> {
    const draftPath = chapterPath + ".draft"
    try {
      const content = await fs.readFile(draftPath, "utf-8")
      if (content && content.length > 0) {
        log.info("Draft recovered", {
          path: draftPath,
          words: countWords(content),
        })
        return content
      }
    } catch {
      // No draft file exists
    }
    return undefined
  }

  /**
   * Count words in text (Chinese chars + English words)
   */
  function countWords(text: string): number {
    const chineseChars = (text.match(/[\u4e00-\u9fa5]/g) || []).length
    const englishWords = (text.match(/[a-zA-Z]+/g) || []).length
    return chineseChars + englishWords
  }
}
