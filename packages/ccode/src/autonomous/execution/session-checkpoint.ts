/**
 * Session Checkpoint Manager
 *
 * Provides session-level checkpoint and recovery capabilities for autonomous mode.
 * Enables session resumption after interruption.
 *
 * @package autonomous/execution
 */

import { Log } from "@/util/log"
import { Global } from "@/global"
import { AutonomousState } from "../state/states"
import { CheckpointManager, type Checkpoint } from "./checkpoint"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import fs from "fs/promises"
import path from "path"
import z from "zod"

const log = Log.create({ service: "autonomous.session-checkpoint" })

// ============================================================================
// Types
// ============================================================================

/**
 * Resource usage at checkpoint time
 */
export interface ResourceUsage {
  iterations: number
  toolCalls: number
  inputTokens: number
  outputTokens: number
  apiCalls: number
  errors: number
}

/**
 * Pending task in checkpoint
 */
export interface PendingTask {
  id: string
  description: string
  status: "pending" | "in_progress"
  priority: number
  dependencies: string[]
}

/**
 * Session checkpoint data
 */
export interface SessionCheckpoint {
  /** Unique session identifier */
  sessionId: string

  /** Checkpoint timestamp */
  timestamp: number

  /** Autonomous state at checkpoint */
  state: AutonomousState

  /** Current iteration number */
  iteration: number

  /** Pending tasks */
  pendingTasks: PendingTask[]

  /** Completed requirements */
  completedRequirements: string[]

  /** Recent errors for context */
  recentErrors: string[]

  /** Resource usage */
  resourceUsage: ResourceUsage

  /** Working directory */
  workingDirectory: string

  /** Original request */
  originalRequest?: string

  /** Agent name */
  agent?: string

  /** Recovery metadata */
  metadata: {
    version: number
    createdAt: string
    lastModifiedAt: string
    interruptReason?: string
  }
}

/**
 * Zod schema for session checkpoint
 */
export const SessionCheckpointSchema = z.object({
  sessionId: z.string(),
  timestamp: z.number(),
  state: z.nativeEnum(AutonomousState),
  iteration: z.number(),
  pendingTasks: z.array(
    z.object({
      id: z.string(),
      description: z.string(),
      status: z.enum(["pending", "in_progress"]),
      priority: z.number(),
      dependencies: z.array(z.string()),
    })
  ),
  completedRequirements: z.array(z.string()),
  recentErrors: z.array(z.string()),
  resourceUsage: z.object({
    iterations: z.number(),
    toolCalls: z.number(),
    inputTokens: z.number(),
    outputTokens: z.number(),
    apiCalls: z.number(),
    errors: z.number(),
  }),
  workingDirectory: z.string(),
  originalRequest: z.string().optional(),
  agent: z.string().optional(),
  metadata: z.object({
    version: z.number(),
    createdAt: z.string(),
    lastModifiedAt: z.string(),
    interruptReason: z.string().optional(),
  }),
})

/**
 * Session checkpoint summary for listing
 */
export interface SessionCheckpointSummary {
  sessionId: string
  timestamp: number
  state: AutonomousState
  iteration: number
  taskCount: number
  workingDirectory: string
  agent?: string
  canResume: boolean
}

// ============================================================================
// Constants
// ============================================================================

const CHECKPOINT_VERSION = 1
const CHECKPOINT_DIR = "checkpoints"
const CHECKPOINT_FILE_EXTENSION = ".checkpoint.json"

// ============================================================================
// Session Checkpoint Manager
// ============================================================================

/**
 * Session Checkpoint Manager
 *
 * Manages session-level checkpoints for recovery after interruption.
 * Wraps the existing CheckpointManager for operation-level checkpoints.
 */
export class SessionCheckpointManager {
  private sessionId: string
  private checkpointDir: string
  private operationCheckpoints: CheckpointManager
  private currentCheckpoint?: SessionCheckpoint

  constructor(sessionId: string) {
    this.sessionId = sessionId
    this.checkpointDir = path.join(Global.Path.data, CHECKPOINT_DIR)
    this.operationCheckpoints = new CheckpointManager(sessionId)
  }

  /**
   * Initialize the checkpoint manager
   */
  async initialize(): Promise<void> {
    await fs.mkdir(this.checkpointDir, { recursive: true })
    await this.operationCheckpoints.load()

    // Try to load existing checkpoint for this session
    const existing = await this.loadCheckpoint(this.sessionId)
    if (existing) {
      this.currentCheckpoint = existing
      log.info("Loaded existing session checkpoint", {
        sessionId: this.sessionId,
        state: existing.state,
        iteration: existing.iteration,
      })
    }
  }

  /**
   * Save a session checkpoint
   *
   * @param state Current session state
   * @returns Checkpoint ID
   */
  async save(state: {
    state: AutonomousState
    iteration: number
    pendingTasks?: PendingTask[]
    completedRequirements?: string[]
    recentErrors?: string[]
    resourceUsage?: Partial<ResourceUsage>
    originalRequest?: string
    agent?: string
    interruptReason?: string
  }): Promise<string> {
    const now = Date.now()

    const checkpoint: SessionCheckpoint = {
      sessionId: this.sessionId,
      timestamp: now,
      state: state.state,
      iteration: state.iteration,
      pendingTasks: state.pendingTasks ?? [],
      completedRequirements: state.completedRequirements ?? [],
      recentErrors: state.recentErrors ?? [],
      resourceUsage: {
        iterations: state.resourceUsage?.iterations ?? state.iteration,
        toolCalls: state.resourceUsage?.toolCalls ?? 0,
        inputTokens: state.resourceUsage?.inputTokens ?? 0,
        outputTokens: state.resourceUsage?.outputTokens ?? 0,
        apiCalls: state.resourceUsage?.apiCalls ?? 0,
        errors: state.resourceUsage?.errors ?? 0,
      },
      workingDirectory: process.cwd(),
      originalRequest: state.originalRequest,
      agent: state.agent,
      metadata: {
        version: CHECKPOINT_VERSION,
        createdAt: this.currentCheckpoint?.metadata.createdAt ?? new Date(now).toISOString(),
        lastModifiedAt: new Date(now).toISOString(),
        interruptReason: state.interruptReason,
      },
    }

    await this.persistCheckpoint(checkpoint)
    this.currentCheckpoint = checkpoint

    log.info("Session checkpoint saved", {
      sessionId: this.sessionId,
      state: state.state,
      iteration: state.iteration,
    })

    await Bus.publish(AutonomousEvent.CheckpointCreated, {
      sessionId: this.sessionId,
      checkpointId: `session_${this.sessionId}`,
      type: "state",
      metadata: {
        state: state.state,
        iteration: state.iteration,
      },
    })

    return `session_${this.sessionId}`
  }

  /**
   * Restore a session checkpoint
   *
   * @param sessionId Session ID to restore
   * @returns Restored checkpoint or null if not found
   */
  async restore(sessionId: string): Promise<SessionCheckpoint | null> {
    const checkpoint = await this.loadCheckpoint(sessionId)

    if (!checkpoint) {
      log.warn("Session checkpoint not found", { sessionId })
      return null
    }

    // Verify the checkpoint is valid for recovery
    if (!this.isRecoverable(checkpoint)) {
      log.warn("Session checkpoint is not recoverable", {
        sessionId,
        state: checkpoint.state,
      })
      return null
    }

    this.currentCheckpoint = checkpoint

    log.info("Session checkpoint restored", {
      sessionId,
      state: checkpoint.state,
      iteration: checkpoint.iteration,
    })

    return checkpoint
  }

  /**
   * List all recoverable session checkpoints
   *
   * @returns Array of checkpoint summaries
   */
  async listRecoverable(): Promise<SessionCheckpointSummary[]> {
    const summaries: SessionCheckpointSummary[] = []

    try {
      const files = await fs.readdir(this.checkpointDir)
      const checkpointFiles = files.filter((f) => f.endsWith(CHECKPOINT_FILE_EXTENSION))

      for (const file of checkpointFiles) {
        try {
          const filePath = path.join(this.checkpointDir, file)
          const content = await fs.readFile(filePath, "utf-8")
          const checkpoint = SessionCheckpointSchema.parse(JSON.parse(content))

          summaries.push({
            sessionId: checkpoint.sessionId,
            timestamp: checkpoint.timestamp,
            state: checkpoint.state,
            iteration: checkpoint.iteration,
            taskCount: checkpoint.pendingTasks.length,
            workingDirectory: checkpoint.workingDirectory,
            agent: checkpoint.agent,
            canResume: this.isRecoverable(checkpoint),
          })
        } catch (error) {
          log.warn("Failed to read checkpoint file", {
            file,
            error: error instanceof Error ? error.message : String(error),
          })
        }
      }
    } catch (error) {
      log.error("Failed to list checkpoints", {
        error: error instanceof Error ? error.message : String(error),
      })
    }

    // Sort by timestamp descending (most recent first)
    return summaries.sort((a, b) => b.timestamp - a.timestamp)
  }

  /**
   * Delete a session checkpoint
   *
   * @param sessionId Session ID to delete
   */
  async delete(sessionId: string): Promise<boolean> {
    const filePath = this.getCheckpointPath(sessionId)

    try {
      await fs.unlink(filePath)
      log.info("Session checkpoint deleted", { sessionId })

      if (this.currentCheckpoint?.sessionId === sessionId) {
        this.currentCheckpoint = undefined
      }

      return true
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
        log.error("Failed to delete checkpoint", {
          sessionId,
          error: error instanceof Error ? error.message : String(error),
        })
      }
      return false
    }
  }

  /**
   * Clean up old checkpoints
   *
   * @param maxAge Maximum age in milliseconds (default: 7 days)
   * @param maxCount Maximum number of checkpoints to keep (default: 50)
   */
  async cleanup(maxAge = 7 * 24 * 60 * 60 * 1000, maxCount = 50): Promise<number> {
    const checkpoints = await this.listRecoverable()
    const now = Date.now()
    let deleted = 0

    // Delete old checkpoints
    for (const cp of checkpoints) {
      if (now - cp.timestamp > maxAge) {
        if (await this.delete(cp.sessionId)) {
          deleted++
        }
      }
    }

    // Delete excess checkpoints (keep most recent)
    const remaining = await this.listRecoverable()
    if (remaining.length > maxCount) {
      const toDelete = remaining.slice(maxCount)
      for (const cp of toDelete) {
        if (await this.delete(cp.sessionId)) {
          deleted++
        }
      }
    }

    log.info("Checkpoint cleanup completed", { deleted })
    return deleted
  }

  /**
   * Get the current checkpoint
   */
  getCurrentCheckpoint(): SessionCheckpoint | undefined {
    return this.currentCheckpoint
  }

  /**
   * Get operation-level checkpoint manager
   */
  getOperationCheckpoints(): CheckpointManager {
    return this.operationCheckpoints
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private getCheckpointPath(sessionId: string): string {
    return path.join(this.checkpointDir, `${sessionId}${CHECKPOINT_FILE_EXTENSION}`)
  }

  private async persistCheckpoint(checkpoint: SessionCheckpoint): Promise<void> {
    const filePath = this.getCheckpointPath(checkpoint.sessionId)
    await fs.writeFile(filePath, JSON.stringify(checkpoint, null, 2), "utf-8")
  }

  private async loadCheckpoint(sessionId: string): Promise<SessionCheckpoint | null> {
    const filePath = this.getCheckpointPath(sessionId)

    try {
      const content = await fs.readFile(filePath, "utf-8")
      return SessionCheckpointSchema.parse(JSON.parse(content))
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
        log.warn("Failed to load checkpoint", {
          sessionId,
          error: error instanceof Error ? error.message : String(error),
        })
      }
      return null
    }
  }

  /**
   * Check if a checkpoint is recoverable
   */
  private isRecoverable(checkpoint: SessionCheckpoint): boolean {
    // Cannot recover from terminal states
    if (
      checkpoint.state === AutonomousState.COMPLETED ||
      checkpoint.state === AutonomousState.FAILED ||
      checkpoint.state === AutonomousState.TERMINATED
    ) {
      return false
    }

    // Verify working directory still exists
    try {
      const stats = Bun.file(checkpoint.workingDirectory)
      if (!stats) return false
    } catch {
      return false
    }

    // Checkpoint must be relatively recent (within 7 days)
    const maxAge = 7 * 24 * 60 * 60 * 1000
    if (Date.now() - checkpoint.timestamp > maxAge) {
      return false
    }

    return true
  }
}

// ============================================================================
// Factory
// ============================================================================

/**
 * Create a session checkpoint manager
 *
 * @param sessionId Session ID
 * @returns Initialized SessionCheckpointManager
 */
export async function createSessionCheckpointManager(
  sessionId: string
): Promise<SessionCheckpointManager> {
  const manager = new SessionCheckpointManager(sessionId)
  await manager.initialize()
  return manager
}

/**
 * List all recoverable sessions (convenience function)
 *
 * @returns Array of recoverable session summaries
 */
export async function listRecoverableSessions(): Promise<SessionCheckpointSummary[]> {
  const manager = new SessionCheckpointManager("__list__")
  await manager.initialize()
  return manager.listRecoverable()
}
