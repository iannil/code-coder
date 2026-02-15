import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { MessageV2 } from "@/session/message-v2"
import { TuiEvent } from "@/cli/cmd/tui/event"
import { ChapterDraftManager } from "./chapter-draft-manager"

const log = Log.create({ service: "writer-stats-monitor" })

/**
 * WriterStatsMonitor - Periodic execution statistics for expander agents
 *
 * When writer agent calls expander/expander-fiction/expander-nonfiction to generate
 * long-form content (tens of thousands of words), execution can take several minutes.
 * This monitor provides periodic status updates every 30 seconds to inform the user
 * about the current execution state.
 *
 * It also integrates with ChapterDraftManager to periodically save generated content
 * to prevent progress loss during long generation sessions.
 */
export namespace WriterStatsMonitor {
  const STATS_INTERVAL_MS = 30_000 // Report every 30 seconds
  const DRAFT_SAVE_INTERVAL_MS = 60_000 // Check for draft save every 60 seconds
  const DRAFT_SAVE_WORD_THRESHOLD = 2000 // Save when 2000 new words accumulated

  interface MonitoredSession {
    sessionID: string
    parentSessionID: string
    agentType: string
    startTime: number
    lastUpdate: number
    wordsGenerated: number
    filesWritten: number
    writesPending: number
    textParts: Map<string, string> // Track full text content by part ID
    // Draft management
    chapterPath?: string
    lastDraftSaveWords: number
    lastDraftSaveTime: number
    draftSaveCount: number
  }

  const activeSessions: Map<string, MonitoredSession> = new Map()
  const intervals: Map<string, ReturnType<typeof setInterval>> = new Map()
  const draftIntervals: Map<string, ReturnType<typeof setInterval>> = new Map()
  const unsubscribers: Map<string, () => void> = new Map()

  /**
   * Count words in text (Chinese chars + English words)
   * Matches the countWords function in document/index.ts
   */
  function countWords(text: string): number {
    const chineseChars = (text.match(/[\u4e00-\u9fa5]/g) || []).length
    const englishWords = (text.match(/[a-zA-Z]+/g) || []).length
    return chineseChars + englishWords
  }

  /**
   * Check if a file path looks like a chapter file
   */
  function isChapterFile(filePath: string): boolean {
    const lower = filePath.toLowerCase()
    // Match common chapter file patterns
    return (
      lower.includes("chapter") ||
      lower.includes("章") ||
      /chapter[-_]?\d+/i.test(filePath) ||
      /ch[-_]?\d+/i.test(filePath) ||
      /第.+章/.test(filePath) ||
      // Match book structure patterns
      (lower.includes("part") && lower.endsWith(".md")) ||
      (lower.includes("volume") && lower.endsWith(".md"))
    )
  }

  /**
   * Start monitoring an expander session
   */
  export function start(input: {
    sessionID: string
    parentSessionID: string
    agentType: string
    chapterPath?: string
  }) {
    const session: MonitoredSession = {
      ...input,
      startTime: Date.now(),
      lastUpdate: Date.now(),
      wordsGenerated: 0,
      filesWritten: 0,
      writesPending: 0,
      textParts: new Map(),
      lastDraftSaveWords: 0,
      lastDraftSaveTime: Date.now(),
      draftSaveCount: 0,
    }
    activeSessions.set(input.sessionID, session)

    log.info("Writer stats monitor started", {
      sessionID: input.sessionID,
      agentType: input.agentType,
      chapterPath: input.chapterPath,
    })

    // Initialize draft manager if chapter path is provided
    if (input.chapterPath) {
      ChapterDraftManager.start({
        sessionID: input.sessionID,
        chapterPath: input.chapterPath,
      })
    }

    // Subscribe to message updates to track progress
    const unsub = Bus.subscribe(MessageV2.Event.PartUpdated, (evt) => {
      if (evt.properties.part.sessionID !== input.sessionID) return
      session.lastUpdate = Date.now()

      const part = evt.properties.part

      // Track text content by accumulating the full text (not just delta)
      if (part.type === "text") {
        session.textParts.set(part.id, part.text)
        // Recalculate total words from all text parts
        session.wordsGenerated = Array.from(session.textParts.values()).reduce((sum, text) => sum + countWords(text), 0)

        // Check if we should save draft
        if (session.chapterPath) {
          checkAndSaveDraft(session)
        }
      }

      // Track Write tool operations specifically
      if (part.type === "tool" && part.tool === "write") {
        // Auto-detect chapter path from Write tool input
        if (!session.chapterPath && part.state.status === "running") {
          const input = part.state.input as { file_path?: string } | undefined
          if (input?.file_path && isChapterFile(input.file_path)) {
            session.chapterPath = input.file_path
            log.info("Auto-detected chapter path from Write tool", {
              sessionID: session.sessionID,
              chapterPath: session.chapterPath,
            })

            // Initialize draft manager
            ChapterDraftManager.start({
              sessionID: session.sessionID,
              chapterPath: session.chapterPath,
            })

            // Start draft save interval if not already running
            if (!draftIntervals.has(session.sessionID)) {
              const draftInterval = setInterval(() => {
                checkAndSaveDraft(session, true)
              }, DRAFT_SAVE_INTERVAL_MS)
              draftIntervals.set(session.sessionID, draftInterval)
            }
          }
        }

        if (part.state.status === "running") {
          session.writesPending++
        } else if (part.state.status === "completed") {
          session.writesPending = Math.max(0, session.writesPending - 1)
          session.filesWritten++
          log.info("File write completed", {
            sessionID: input.sessionID,
            filesWritten: session.filesWritten,
          })
        } else if (part.state.status === "error") {
          session.writesPending = Math.max(0, session.writesPending - 1)
          log.warn("File write failed", {
            sessionID: input.sessionID,
            error: part.state.error,
          })
        }
      }
    })
    unsubscribers.set(input.sessionID, unsub)

    // Start periodic reporting
    const interval = setInterval(() => {
      reportStats(session)
    }, STATS_INTERVAL_MS)
    intervals.set(input.sessionID, interval)

    // Start periodic draft save check
    if (input.chapterPath) {
      const draftInterval = setInterval(() => {
        checkAndSaveDraft(session, true) // force check on interval
      }, DRAFT_SAVE_INTERVAL_MS)
      draftIntervals.set(input.sessionID, draftInterval)
    }

    // Report immediately on start
    reportStats(session, "started")
  }

  /**
   * Check and save draft if enough new content has been generated
   */
  async function checkAndSaveDraft(session: MonitoredSession, forceTimeCheck = false): Promise<void> {
    if (!session.chapterPath) return

    const wordsSinceLastSave = session.wordsGenerated - session.lastDraftSaveWords
    const timeSinceLastSave = Date.now() - session.lastDraftSaveTime

    // Check if we should save
    const shouldSave =
      wordsSinceLastSave >= DRAFT_SAVE_WORD_THRESHOLD ||
      (forceTimeCheck && timeSinceLastSave >= DRAFT_SAVE_INTERVAL_MS && wordsSinceLastSave > 500)

    if (shouldSave) {
      // Get all accumulated text
      const content = Array.from(session.textParts.values()).join("\n\n")

      const saved = await ChapterDraftManager.updateContent(session.sessionID, content, { force: true })

      if (saved) {
        session.lastDraftSaveWords = session.wordsGenerated
        session.lastDraftSaveTime = Date.now()
        session.draftSaveCount++

        log.info("Draft auto-saved", {
          sessionID: session.sessionID,
          words: session.wordsGenerated,
          saveCount: session.draftSaveCount,
        })
      }
    }
  }

  /**
   * Stop monitoring a session
   */
  export async function stop(sessionID: string) {
    const session = activeSessions.get(sessionID)
    if (session) {
      // Save final draft if we have content
      if (session.chapterPath && session.wordsGenerated > 0) {
        const content = Array.from(session.textParts.values()).join("\n\n")
        await ChapterDraftManager.finalize(sessionID, content)
      }

      reportStats(session, "completed")
      log.info("Writer stats monitor stopped", {
        sessionID,
        elapsedSeconds: Math.floor((Date.now() - session.startTime) / 1000),
        wordsGenerated: session.wordsGenerated,
        filesWritten: session.filesWritten,
        draftSaveCount: session.draftSaveCount,
      })
    }

    // Clean up resources
    const unsub = unsubscribers.get(sessionID)
    if (unsub) unsub()
    unsubscribers.delete(sessionID)

    const interval = intervals.get(sessionID)
    if (interval) clearInterval(interval)
    intervals.delete(sessionID)

    const draftInterval = draftIntervals.get(sessionID)
    if (draftInterval) clearInterval(draftInterval)
    draftIntervals.delete(sessionID)

    // Stop draft manager
    ChapterDraftManager.stop(sessionID)

    activeSessions.delete(sessionID)
  }

  /**
   * Report statistics to TUI
   */
  function reportStats(session: MonitoredSession, status: "started" | "running" | "completed" = "running") {
    const elapsed = Date.now() - session.startTime
    const sinceUpdate = Date.now() - session.lastUpdate

    Bus.publish(TuiEvent.WriterStats, {
      status,
      agentType: session.agentType,
      elapsedSeconds: Math.floor(elapsed / 1000),
      wordsGenerated: session.wordsGenerated,
      filesWritten: session.filesWritten,
      writesPending: session.writesPending,
      isStalled: sinceUpdate > 45_000, // 45 seconds without update is considered stalled
    })
  }

  /**
   * Set chapter path for an active session (for late binding)
   */
  export function setChapterPath(sessionID: string, chapterPath: string): void {
    const session = activeSessions.get(sessionID)
    if (session) {
      session.chapterPath = chapterPath

      // Initialize draft manager
      ChapterDraftManager.start({
        sessionID,
        chapterPath,
      })

      // Start draft save interval if not already running
      if (!draftIntervals.has(sessionID)) {
        const draftInterval = setInterval(() => {
          checkAndSaveDraft(session, true)
        }, DRAFT_SAVE_INTERVAL_MS)
        draftIntervals.set(sessionID, draftInterval)
      }

      log.info("Chapter path set for session", {
        sessionID,
        chapterPath,
      })
    }
  }

  /**
   * Check if a session is being monitored
   */
  export function isMonitoring(sessionID: string): boolean {
    return activeSessions.has(sessionID)
  }

  /**
   * Get info about a monitored session
   */
  export function getSession(sessionID: string): MonitoredSession | undefined {
    return activeSessions.get(sessionID)
  }

  /**
   * Get all active sessions (for debugging)
   */
  export function getActiveSessions(): Map<string, MonitoredSession> {
    return new Map(activeSessions)
  }

  /**
   * Get accumulated content for a session
   */
  export function getAccumulatedContent(sessionID: string): string | undefined {
    const session = activeSessions.get(sessionID)
    if (!session) return undefined
    return Array.from(session.textParts.values()).join("\n\n")
  }
}
