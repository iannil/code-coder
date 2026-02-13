import { Log } from "@/util/log"
import { WriterService } from "./writer-service"

const log = Log.create({ service: "writer-timeout-monitor" })

export namespace WriterTimeoutMonitor {
  const MONITOR_INTERVAL_MS = 5000 // Check every 5 seconds
  const TIMEOUT_WARNING_MS = 45_000 // Warn after 45 seconds
  const TIMEOUT_CRITICAL_MS = 90_000 // Critical after 90 seconds

  interface MonitoredTask {
    sessionId: string
    startTime: number
    lastProgress: number
    chapterCount?: number
  }

  let activeTask: MonitoredTask | null = null
  let monitorInterval: ReturnType<typeof setInterval> | null = null

  /**
   * Start monitoring a writing task
   */
  export function startTask(sessionId: string, expectedChapters?: number) {
    stopTask() // Stop any existing task

    activeTask = {
      sessionId,
      startTime: Date.now(),
      lastProgress: Date.now(),
      chapterCount: expectedChapters,
    }

    log.info("Writer task started", {
      sessionId,
      expectedChapters,
    })

    // Start monitoring
    monitorInterval = setInterval(() => {
      checkTimeout()
    }, MONITOR_INTERVAL_MS)
  }

  /**
   * Update progress (call when chapter is completed)
   */
  export function updateProgress() {
    if (activeTask) {
      activeTask.lastProgress = Date.now()
    }
  }

  /**
   * Stop monitoring current task
   */
  export function stopTask() {
    if (monitorInterval) {
      clearInterval(monitorInterval)
      monitorInterval = null
    }
    activeTask = null
  }

  /**
   * Check if task has timed out
   */
  function checkTimeout() {
    if (!activeTask) return

    const now = Date.now()
    const elapsed = now - activeTask.startTime
    const sinceProgress = now - activeTask.lastProgress

    // Warn if no progress for 45 seconds
    if (sinceProgress > TIMEOUT_WARNING_MS && sinceProgress < TIMEOUT_CRITICAL_MS) {
      log.warn("Writer task stalling", {
        elapsed: Math.floor(elapsed / 1000),
        sinceProgress: Math.floor(sinceProgress / 1000),
      })

      WriterService.reportProgress({
        action: "error",
        message: `Generation taking longer than expected (${Math.floor(sinceProgress / 1000)}s since last progress). If this continues, consider breaking into smaller chunks.`,
      })

      // Reset lastProgress to avoid duplicate warnings
      activeTask.lastProgress = now - TIMEOUT_WARNING_MS + 5000
    }

    // Critical timeout after 90 seconds
    if (sinceProgress > TIMEOUT_CRITICAL_MS) {
      log.error("Writer task timeout", {
        elapsed: Math.floor(elapsed / 1000),
      })

      WriterService.reportProgress({
        action: "error",
        message: `Request timeout (${Math.floor(elapsed / 1000)}s). The task may be too large. Try: 1) Break into smaller chapters, 2) Use outline-first approach.`,
      })

      stopTask()
    }
  }

  /**
   * Get suggested chunk size based on model and content type
   */
  export function suggestChunkSize(
    totalWords: number,
    modelProvider: string,
  ): { chapters: number; wordsPerChapter: number; recommendation: string } {
    const baseWordsPerChapter = 2500 // Conservative default

    // Adjust based on provider
    let multiplier = 1
    if (modelProvider.includes("google") || modelProvider.includes("gemini")) {
      multiplier = 0.7 // Gemini has stricter limits
    } else if (modelProvider.includes("openai")) {
      multiplier = 1.2 // OpenAI can handle more
    } else if (modelProvider.includes("anthropic")) {
      multiplier = 1.5 // Claude handles long outputs well
    }

    const adjustedWordsPerChapter = Math.floor(baseWordsPerChapter * multiplier)
    const chapters = Math.max(3, Math.ceil(totalWords / adjustedWordsPerChapter))

    return {
      chapters,
      wordsPerChapter: adjustedWordsPerChapter,
      recommendation: `For ${totalWords} words with ${modelProvider}, split into ${chapters} chapters of ~${adjustedWordsPerChapter} words each.`,
    }
  }

  /**
   * Check if current task should be split
   */
  export function shouldSplitTask(elapsedMs: number, outputTokens: number): boolean {
    // If taking more than 2 minutes per 8k tokens, suggest splitting
    const tokensPerMs = outputTokens / elapsedMs
    const isSlow = tokensPerMs < 100 // Less than 100 tokens per second

    return elapsedMs > 120_000 || (elapsedMs > 60_000 && isSlow)
  }

  /**
   * Get active task info
   */
  export function getActiveTask(): MonitoredTask | null {
    return activeTask
  }
}
