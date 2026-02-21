import { Log } from "@/util/log"
import { Global } from "@/global"
import { Filesystem } from "@/util/filesystem"
import { Lock } from "@/util/lock"
import path from "path"
import fs from "fs/promises"
import z from "zod"

const log = Log.create({ service: "bootstrap.cost-tracker" })

/**
 * CostTracker monitors token consumption and tracks cost savings from skills.
 */
export namespace CostTracker {
  const BATCH_SIZE = 10

  // Dynamic path getters to respect CCODE_TEST_HOME
  function getStoreDir(): string {
    return path.join(Global.Path.config, "bootstrap")
  }

  function getStoreFile(): string {
    return path.join(getStoreDir(), "cost-metrics.json")
  }

  /**
   * Usage record schema
   */
  export const UsageRecord = z.object({
    sessionId: z.string(),
    skillId: z.string().optional(),
    operation: z.string(),
    inputTokens: z.number(),
    outputTokens: z.number(),
    reasoningSteps: z.number(),
    duration: z.number(),
    timestamp: z.number(),
  })
  export type UsageRecord = z.infer<typeof UsageRecord>

  /**
   * Cost savings calculation
   */
  export const CostSavings = z.object({
    skillId: z.string(),
    totalUsages: z.number(),
    averageTokensSaved: z.number(),
    averageStepsSaved: z.number(),
    totalTokensSaved: z.number(),
    costReductionPercent: z.number(),
  })
  export type CostSavings = z.infer<typeof CostSavings>

  /**
   * Metrics store schema
   */
  const MetricsStore = z.object({
    version: z.number(),
    records: z.array(UsageRecord),
    skillStats: z.record(
      z.string(),
      z.object({
        usageCount: z.number(),
        totalInputTokens: z.number(),
        totalOutputTokens: z.number(),
        totalSteps: z.number(),
        totalDuration: z.number(),
        baseline: z
          .object({
            avgInputTokens: z.number(),
            avgOutputTokens: z.number(),
            avgSteps: z.number(),
          })
          .optional(),
      }),
    ),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  type MetricsStore = z.infer<typeof MetricsStore>

  // In-memory buffer for batching writes
  let pendingRecords: UsageRecord[] = []
  let writeScheduled = false

  /**
   * Initialize store directory
   */
  async function ensureDir(): Promise<void> {
    await fs.mkdir(getStoreDir(), { recursive: true })
  }

  /**
   * Create empty store
   */
  function createEmpty(): MetricsStore {
    const now = Date.now()
    return {
      version: 1,
      records: [],
      skillStats: {},
      time: { created: now, updated: now },
    }
  }

  /**
   * Read the metrics store
   */
  async function read(): Promise<MetricsStore> {
    await ensureDir()
    const storeFile = getStoreFile()

    try {
      using _ = await Lock.read(storeFile)
      const file = Bun.file(storeFile)
      if (!(await file.exists())) {
        return createEmpty()
      }

      const text = await file.text()
      const data = JSON.parse(text)
      const parsed = MetricsStore.safeParse(data)

      if (!parsed.success) {
        log.warn("invalid metrics store, creating new")
        return createEmpty()
      }

      return parsed.data
    } catch {
      return createEmpty()
    }
  }

  /**
   * Write the metrics store
   */
  async function write(store: MetricsStore): Promise<void> {
    await ensureDir()
    const storeFile = getStoreFile()

    store.time.updated = Date.now()

    using _ = await Lock.write(storeFile)
    await Filesystem.atomicWrite(storeFile, JSON.stringify(store, null, 2))
  }

  /**
   * Schedule a batched write
   */
  function scheduleBatchWrite(): void {
    if (writeScheduled) return

    writeScheduled = true
    setTimeout(async () => {
      writeScheduled = false
      await flushPendingRecords()
    }, 5000) // 5 second debounce
  }

  /**
   * Flush pending records to disk
   */
  async function flushPendingRecords(): Promise<void> {
    if (pendingRecords.length === 0) return

    const toWrite = [...pendingRecords]
    pendingRecords = []

    const store = await read()

    for (const record of toWrite) {
      // Keep only recent records (last 7 days)
      const cutoff = Date.now() - 7 * 24 * 60 * 60 * 1000
      store.records = store.records.filter((r) => r.timestamp > cutoff)
      store.records.push(record)

      // Update skill stats if skill was used
      if (record.skillId) {
        const stats = store.skillStats[record.skillId] ?? {
          usageCount: 0,
          totalInputTokens: 0,
          totalOutputTokens: 0,
          totalSteps: 0,
          totalDuration: 0,
        }

        stats.usageCount++
        stats.totalInputTokens += record.inputTokens
        stats.totalOutputTokens += record.outputTokens
        stats.totalSteps += record.reasoningSteps
        stats.totalDuration += record.duration

        store.skillStats[record.skillId] = stats
      }
    }

    await write(store)
    log.info("flushed usage records", { count: toWrite.length })
  }

  /**
   * Record a usage event
   */
  export async function record(usage: UsageRecord): Promise<void> {
    pendingRecords.push(usage)

    // Flush immediately if batch size reached
    if (pendingRecords.length >= BATCH_SIZE) {
      await flushPendingRecords()
    } else {
      scheduleBatchWrite()
    }
  }

  /**
   * Set baseline metrics for a task type (without skill)
   */
  export async function setBaseline(
    skillId: string,
    baseline: { avgInputTokens: number; avgOutputTokens: number; avgSteps: number },
  ): Promise<void> {
    const store = await read()

    const stats = store.skillStats[skillId] ?? {
      usageCount: 0,
      totalInputTokens: 0,
      totalOutputTokens: 0,
      totalSteps: 0,
      totalDuration: 0,
    }

    stats.baseline = baseline
    store.skillStats[skillId] = stats

    await write(store)
  }

  /**
   * Get cost savings for a skill
   */
  export async function getSavings(skillId: string): Promise<CostSavings | null> {
    const store = await read()
    const stats = store.skillStats[skillId]

    if (!stats || stats.usageCount === 0) {
      return null
    }

    const avgWithSkill = {
      inputTokens: stats.totalInputTokens / stats.usageCount,
      outputTokens: stats.totalOutputTokens / stats.usageCount,
      steps: stats.totalSteps / stats.usageCount,
    }

    // Use baseline if available, otherwise estimate
    const baseline = stats.baseline ?? {
      avgInputTokens: avgWithSkill.inputTokens * 1.5,
      avgOutputTokens: avgWithSkill.outputTokens * 1.3,
      avgSteps: avgWithSkill.steps * 1.2,
    }

    const tokensSaved = baseline.avgInputTokens + baseline.avgOutputTokens -
      (avgWithSkill.inputTokens + avgWithSkill.outputTokens)
    const stepsSaved = baseline.avgSteps - avgWithSkill.steps

    const totalBaseline = baseline.avgInputTokens + baseline.avgOutputTokens
    const totalWithSkill = avgWithSkill.inputTokens + avgWithSkill.outputTokens
    const costReduction = totalBaseline > 0
      ? ((totalBaseline - totalWithSkill) / totalBaseline) * 100
      : 0

    return {
      skillId,
      totalUsages: stats.usageCount,
      averageTokensSaved: Math.max(0, tokensSaved),
      averageStepsSaved: Math.max(0, stepsSaved),
      totalTokensSaved: Math.max(0, tokensSaved * stats.usageCount),
      costReductionPercent: Math.max(0, costReduction),
    }
  }

  /**
   * Compare metrics with and without skill
   */
  export async function compare(taskType: string): Promise<{
    withSkill: { avgTokens: number; avgSteps: number }
    withoutSkill: { avgTokens: number; avgSteps: number }
    savings: { tokenPercent: number; stepPercent: number }
  }> {
    const store = await read()

    // Find records with and without skills for similar operations
    const withSkill = store.records.filter(
      (r) => r.skillId && r.operation.includes(taskType),
    )
    const withoutSkill = store.records.filter(
      (r) => !r.skillId && r.operation.includes(taskType),
    )

    const avgWithSkill =
      withSkill.length > 0
        ? {
            avgTokens:
              withSkill.reduce((sum, r) => sum + r.inputTokens + r.outputTokens, 0) /
              withSkill.length,
            avgSteps:
              withSkill.reduce((sum, r) => sum + r.reasoningSteps, 0) / withSkill.length,
          }
        : { avgTokens: 0, avgSteps: 0 }

    const avgWithoutSkill =
      withoutSkill.length > 0
        ? {
            avgTokens:
              withoutSkill.reduce((sum, r) => sum + r.inputTokens + r.outputTokens, 0) /
              withoutSkill.length,
            avgSteps:
              withoutSkill.reduce((sum, r) => sum + r.reasoningSteps, 0) / withoutSkill.length,
          }
        : { avgTokens: avgWithSkill.avgTokens * 1.3, avgSteps: avgWithSkill.avgSteps * 1.2 }

    const tokenSavings =
      avgWithoutSkill.avgTokens > 0
        ? ((avgWithoutSkill.avgTokens - avgWithSkill.avgTokens) / avgWithoutSkill.avgTokens) * 100
        : 0

    const stepSavings =
      avgWithoutSkill.avgSteps > 0
        ? ((avgWithoutSkill.avgSteps - avgWithSkill.avgSteps) / avgWithoutSkill.avgSteps) * 100
        : 0

    return {
      withSkill: avgWithSkill,
      withoutSkill: avgWithoutSkill,
      savings: {
        tokenPercent: Math.max(0, tokenSavings),
        stepPercent: Math.max(0, stepSavings),
      },
    }
  }

  /**
   * Get summary of all skill savings
   */
  export async function getSummary(): Promise<{
    totalSkillUsages: number
    totalTokensSaved: number
    averageCostReduction: number
    topSkills: Array<{ skillId: string; savings: CostSavings }>
  }> {
    const store = await read()
    const skillIds = Object.keys(store.skillStats)

    let totalUsages = 0
    let totalTokens = 0
    let totalReduction = 0
    const allSavings: Array<{ skillId: string; savings: CostSavings }> = []

    for (const skillId of skillIds) {
      const savings = await getSavings(skillId)
      if (savings) {
        totalUsages += savings.totalUsages
        totalTokens += savings.totalTokensSaved
        totalReduction += savings.costReductionPercent
        allSavings.push({ skillId, savings })
      }
    }

    // Sort by total tokens saved
    allSavings.sort((a, b) => b.savings.totalTokensSaved - a.savings.totalTokensSaved)

    return {
      totalSkillUsages: totalUsages,
      totalTokensSaved: totalTokens,
      averageCostReduction: skillIds.length > 0 ? totalReduction / skillIds.length : 0,
      topSkills: allSavings.slice(0, 10),
    }
  }

  /**
   * Force flush any pending records (for shutdown)
   */
  export async function flush(): Promise<void> {
    await flushPendingRecords()
  }
}
