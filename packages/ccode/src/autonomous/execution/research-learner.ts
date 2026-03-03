/**
 * Research Learner
 *
 * Learns research patterns from successful research tasks:
 * - Effective keywords for topics
 * - Useful data sources
 * - Periodic task detection
 * - Automatic Hand creation suggestions
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "autonomous.research-learner" })

export interface ResearchRecord {
  topic: string
  keywords: string[]
  sources?: string[]
  timestamp?: string
  success?: boolean
}

export interface LearnedResearchPattern {
  id: string
  topic: string
  keywords: string[]
  sources: string[]
  analysisFramework?: string
  frequency?: "daily" | "weekly" | "monthly"
  confidence: number
  createdAt: string
  lastUsedAt: string
  usageCount: number
}

export interface HandSuggestion {
  topic: string
  frequency: "daily" | "weekly" | "monthly"
  keywords: string[]
  sources: string[]
  schedule: string
  confidence: number
}

interface ResearchLearnerState {
  patterns: Map<string, LearnedResearchPattern>
  history: ResearchRecord[]
}

/** Normalize topic for matching */
function normalizeTopic(topic: string): string {
  return topic.toLowerCase().trim()
}

/** Generate pattern ID */
function generatePatternId(topic: string): string {
  return `research-${normalizeTopic(topic).replace(/\s+/g, "-").slice(0, 30)}-${Date.now()}`
}

/** Detect frequency from timestamps */
function detectFrequency(timestamps: string[]): "daily" | "weekly" | "monthly" | undefined {
  if (timestamps.length < 3) return undefined

  const sorted = timestamps.map((t) => new Date(t).getTime()).sort((a, b) => a - b)
  const intervals = []

  for (let i = 1; i < sorted.length; i++) {
    intervals.push(sorted[i] - sorted[i - 1])
  }

  const avgInterval = intervals.reduce((a, b) => a + b, 0) / intervals.length
  const dayMs = 24 * 60 * 60 * 1000

  if (avgInterval < dayMs * 1.5) return "daily"
  if (avgInterval < dayMs * 10) return "weekly"
  if (avgInterval < dayMs * 45) return "monthly"

  return undefined
}

/** Create cron schedule from frequency */
function frequencyToSchedule(frequency: "daily" | "weekly" | "monthly"): string {
  switch (frequency) {
    case "daily":
      return "0 8 * * *"
    case "weekly":
      return "0 8 * * 1"
    case "monthly":
      return "0 8 1 * *"
  }
}

/** Create a Research Learner instance */
export function createResearchLearner() {
  const state: ResearchLearnerState = {
    patterns: new Map(),
    history: [],
  }

  return {
    /** Record a completed research session */
    recordResearch(record: ResearchRecord): void {
      const normalizedTopic = normalizeTopic(record.topic)
      const timestamp = record.timestamp ?? new Date().toISOString()

      state.history.push({ ...record, timestamp })

      // Update or create pattern
      const existing = state.patterns.get(normalizedTopic)

      if (existing) {
        // Merge keywords (dedupe)
        const allKeywords = [...new Set([...existing.keywords, ...record.keywords])]
        const allSources = [...new Set([...existing.sources, ...(record.sources ?? [])])]

        // Get all timestamps for this topic
        const topicHistory = state.history.filter(
          (h) => normalizeTopic(h.topic) === normalizedTopic,
        )
        const timestamps = topicHistory.map((h) => h.timestamp!).filter(Boolean)

        existing.keywords = allKeywords
        existing.sources = allSources
        existing.lastUsedAt = timestamp
        existing.usageCount++
        existing.frequency = detectFrequency(timestamps)
        existing.confidence = Math.min(0.95, existing.confidence + 0.1)

        log.debug("Updated research pattern", {
          topic: record.topic,
          usageCount: existing.usageCount,
          frequency: existing.frequency,
        })
      } else {
        const pattern: LearnedResearchPattern = {
          id: generatePatternId(record.topic),
          topic: record.topic,
          keywords: record.keywords,
          sources: record.sources ?? [],
          confidence: 0.5,
          createdAt: timestamp,
          lastUsedAt: timestamp,
          usageCount: 1,
        }
        state.patterns.set(normalizedTopic, pattern)

        log.debug("Created new research pattern", { topic: record.topic })
      }
    },

    /** Get all learned patterns */
    getPatterns(): LearnedResearchPattern[] {
      return Array.from(state.patterns.values())
    },

    /** Get a specific pattern by topic */
    getPattern(topic: string): LearnedResearchPattern | undefined {
      return state.patterns.get(normalizeTopic(topic))
    },

    /** Suggest Hand creation for periodic tasks */
    suggestHandCreation(topic: string): HandSuggestion | undefined {
      const pattern = state.patterns.get(normalizeTopic(topic))

      if (!pattern || !pattern.frequency || pattern.usageCount < 3) {
        return undefined
      }

      return {
        topic: pattern.topic,
        frequency: pattern.frequency,
        keywords: pattern.keywords,
        sources: pattern.sources,
        schedule: frequencyToSchedule(pattern.frequency),
        confidence: pattern.confidence,
      }
    },

    /** Export state for persistence */
    exportState(): ResearchLearnerState {
      return {
        patterns: new Map(state.patterns),
        history: [...state.history],
      }
    },

    /** Import state from persistence */
    importState(imported: ResearchLearnerState): void {
      state.patterns = new Map(imported.patterns)
      state.history = [...imported.history]
    },
  }
}

export type ResearchLearner = ReturnType<typeof createResearchLearner>
