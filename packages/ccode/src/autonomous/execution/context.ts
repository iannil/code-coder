import { Log } from "@/util/log"
import { getProjectIDForStorage } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { AutonomousState } from "../state/states"
import z from "zod"

const log = Log.create({ service: "autonomous.execution.context" })

/**
 * Execution context schema
 */
const ExecutionContextSchema = z.object({
  sessionId: z.string(),
  requestId: z.string(),
  state: z.nativeEnum(AutonomousState),
  createdAt: z.number(),
  updatedAt: z.number(),
  metadata: z.record(z.string(), z.any()),
  currentPhase: z.string().optional(),
  currentTask: z.string().optional(),
  filesModified: z.array(z.string()),
  testsRun: z.number(),
  testsPassed: z.number(),
  testsFailed: z.number(),
  // Additional metadata for Autonomous Mode execution
  requirement: z.string().optional(),
  lastTestFile: z.string().optional(),
  lastImplFile: z.string().optional(),
})

export type ExecutionContext = z.infer<typeof ExecutionContextSchema>

/**
 * Context options
 */
export interface ContextOptions {
  sessionId: string
  requestId: string
  initialMetadata?: Record<string, unknown>
}

/**
 * Execution context manager
 *
 * Manages the mutable context during Autonomous Mode execution
 */
export class ContextManager {
  private context: ExecutionContext
  private storageKey: string[]

  constructor(options: ContextOptions) {
    const now = Date.now()
    const projectID = getProjectIDForStorage(options.sessionId)

    this.context = {
      sessionId: options.sessionId,
      requestId: options.requestId,
      state: AutonomousState.IDLE,
      createdAt: now,
      updatedAt: now,
      metadata: options.initialMetadata ?? {},
      filesModified: [],
      testsRun: 0,
      testsPassed: 0,
      testsFailed: 0,
    }

    this.storageKey = ["autonomous", "context", projectID, options.sessionId]
  }

  /**
   * Get current context
   */
  get(): ExecutionContext {
    return { ...this.context }
  }

  /**
   * Update context
   */
  async update(updates: Partial<Omit<ExecutionContext, "sessionId" | "requestId" | "createdAt">>): Promise<void> {
    this.context = {
      ...this.context,
      ...updates,
      updatedAt: Date.now(),
    }

    await this.persist()
  }

  /**
   * Set state
   */
  async setState(state: AutonomousState): Promise<void> {
    await this.update({ state })
  }

  /**
   * Set current phase
   */
  async setPhase(phase: string): Promise<void> {
    await this.update({ currentPhase: phase })
  }

  /**
   * Set current task
   */
  async setTask(taskId: string): Promise<void> {
    await this.update({ currentTask: taskId })
  }

  /**
   * Add modified file
   */
  async addFile(filePath: string): Promise<void> {
    if (!this.context.filesModified.includes(filePath)) {
      await this.update({
        filesModified: [...this.context.filesModified, filePath],
      })
    }
  }

  /**
   * Record test results
   */
  async recordTestResults(run: number, passed: number, failed: number): Promise<void> {
    await this.update({
      testsRun: this.context.testsRun + run,
      testsPassed: this.context.testsPassed + passed,
      testsFailed: this.context.testsFailed + failed,
    })
  }

  /**
   * Get test statistics
   */
  getTestStats(): { run: number; passed: number; failed: number; passRate: number } {
    const { testsRun, testsPassed, testsFailed } = this.context
    return {
      run: testsRun,
      passed: testsPassed,
      failed: testsFailed,
      passRate: testsRun > 0 ? testsPassed / testsRun : 0,
    }
  }

  /**
   * Set metadata value
   */
  async setMetadata(key: string, value: unknown): Promise<void> {
    await this.update({
      metadata: { ...this.context.metadata, [key]: value },
    })
  }

  /**
   * Get metadata value
   */
  getMetadata<T = unknown>(key: string): T | undefined {
    return this.context.metadata[key] as T
  }

  /**
   * Get all metadata
   */
  getAllMetadata(): Record<string, unknown> {
    return { ...this.context.metadata }
  }

  /**
   * Persist context to storage
   */
  async persist(): Promise<void> {
    try {
      await Storage.write(this.storageKey, this.context)
    } catch (error) {
      log.error("Failed to persist context", { error })
    }
  }

  /**
   * Load context from storage
   */
  async load(): Promise<boolean> {
    try {
      const data = await Storage.read<ExecutionContext>(this.storageKey)
      this.context = data
      return true
    } catch {
      return false
    }
  }

  /**
   * Clear context from storage
   */
  async clear(): Promise<void> {
    try {
      await Storage.remove(this.storageKey)
    } catch (error) {
      log.error("Failed to clear context", { error })
    }
  }

  /**
   * Serialize context
   */
  serialize(): string {
    return JSON.stringify(this.context)
  }

  /**
   * Deserialize context
   */
  static deserialize(data: string): ExecutionContext {
    return ExecutionContextSchema.parse(JSON.parse(data))
  }

  /**
   * Create context from storage
   */
  static async create(sessionId: string, requestId: string): Promise<ContextManager> {
    const manager = new ContextManager({ sessionId, requestId })
    const loaded = await manager.load()

    if (!loaded) {
      // Initialize new context
      await manager.persist()
    }

    return manager
  }
}

/**
 * Create a new context manager
 */
export function createExecutionContext(options: ContextOptions): ContextManager {
  return new ContextManager(options)
}
