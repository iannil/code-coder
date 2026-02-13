import { Log } from "@/util/log"
import { getProjectIDForStorage, Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { GitOps } from "./git-ops"
import z from "zod"

const log = Log.create({ service: "autonomous.checkpoint" })

/**
 * Checkpoint types
 */
export type CheckpointType = "git" | "state" | "manual"

/**
 * Checkpoint data
 */
export interface Checkpoint {
  id: string
  sessionId: string
  type: CheckpointType
  createdAt: number
  state: Record<string, unknown>
  files: string[]
  gitCommit?: string
  metadata: Record<string, unknown>
}

/**
 * Stored checkpoint format
 */
const StoredCheckpoint = z.object({
  id: z.string(),
  sessionId: z.string(),
  type: z.enum(["git", "state", "manual"]),
  createdAt: z.number(),
  state: z.record(z.string(), z.any()),
  files: z.array(z.string()),
  gitCommit: z.string().optional(),
  metadata: z.record(z.string(), z.any()),
})

type StoredCheckpoint = z.infer<typeof StoredCheckpoint>

/**
 * Checkpoint manager
 *
 * Creates and restores checkpoints for rollback capability
 */
export class CheckpointManager {
  private sessionId: string
  private storageKey: string[]
  private checkpoints: Map<string, Checkpoint> = new Map()

  constructor(sessionId: string) {
    this.sessionId = sessionId
    const projectID = getProjectIDForStorage(sessionId)
    this.storageKey = ["autonomous", "checkpoints", projectID, sessionId]
  }

  /**
   * Create a checkpoint
   */
  async create(type: CheckpointType = "state", reason = ""): Promise<string> {
    const id = this.generateCheckpointId()

    const checkpoint: Checkpoint = {
      id,
      sessionId: this.sessionId,
      type,
      createdAt: Date.now(),
      state: await this.captureState(),
      files: await this.captureFiles(),
      gitCommit: type === "git" ? await this.createGitCommit(reason) : undefined,
      metadata: {
        reason,
        stateMachineState: (await this.getCurrentState()) ?? "unknown",
      },
    }

    this.checkpoints.set(id, checkpoint)
    await this.persist()

    log.info("Checkpoint created", { id, type, reason })

    await Bus.publish(AutonomousEvent.CheckpointCreated, {
      sessionId: this.sessionId,
      checkpointId: id,
      type,
      metadata: checkpoint.metadata,
    })

    return id
  }

  /**
   * Restore from a checkpoint
   */
  async restore(checkpointId: string, reason = ""): Promise<boolean> {
    const checkpoint = this.checkpoints.get(checkpointId)
    if (!checkpoint) {
      log.error("Checkpoint not found", { checkpointId })
      return false
    }

    log.info("Restoring checkpoint", { checkpointId, reason })

    try {
      // Restore state
      await this.restoreState(checkpoint.state)

      // Restore files if we have a git commit
      if (checkpoint.gitCommit) {
        await this.restoreGitCommit(checkpoint.gitCommit)
      }

      log.info("Checkpoint restored", { checkpointId })

      await Bus.publish(AutonomousEvent.RollbackPerformed, {
        sessionId: this.sessionId,
        checkpointId,
        reason,
        success: true,
      })

      return true
    } catch (error) {
      log.error("Failed to restore checkpoint", {
        checkpointId,
        error: error instanceof Error ? error.message : String(error),
      })

      await Bus.publish(AutonomousEvent.RollbackPerformed, {
        sessionId: this.sessionId,
        checkpointId,
        reason,
        success: false,
      })

      return false
    }
  }

  /**
   * Get a checkpoint
   */
  get(checkpointId: string): Checkpoint | undefined {
    return this.checkpoints.get(checkpointId)
  }

  /**
   * Get all checkpoints
   */
  getAll(): Checkpoint[] {
    return Array.from(this.checkpoints.values()).sort((a, b) => b.createdAt - a.createdAt)
  }

  /**
   * Get latest checkpoint
   */
  getLatest(): Checkpoint | undefined {
    const checkpoints = this.getAll()
    return checkpoints[0]
  }

  /**
   * Get checkpoints by type
   */
  getByType(type: CheckpointType): Checkpoint[] {
    return this.getAll().filter((c) => c.type === type)
  }

  /**
   * Delete a checkpoint
   */
  async delete(checkpointId: string): Promise<boolean> {
    const checkpoint = this.checkpoints.get(checkpointId)
    if (!checkpoint) {
      return false
    }

    this.checkpoints.delete(checkpointId)
    await this.persist()

    log.info("Checkpoint deleted", { checkpointId })

    return true
  }

  /**
   * Clear all checkpoints
   */
  async clear(): Promise<void> {
    this.checkpoints.clear()
    await this.persist()

    log.info("All checkpoints cleared", { sessionId: this.sessionId })
  }

  /**
   * Capture current state
   */
  private async captureState(): Promise<Record<string, unknown>> {
    // In a real implementation, this would capture:
    // - State machine state
    // - Task queue state
    // - Decision history
    // - Execution context

    try {
      const stateData = await Storage.read<Record<string, unknown>>([
        "autonomous",
        "context",
        getProjectIDForStorage(this.sessionId),
        this.sessionId,
      ])

      return stateData ?? {}
    } catch {
      return {}
    }
  }

  /**
   * Capture modified files
   */
  private async captureFiles(): Promise<string[]> {
    const status = await GitOps.getStatus()
    return [
      ...status.modified,
      ...status.added,
      ...Array.from(status.renamed.values()),
    ]
  }

  /**
   * Create a git commit
   */
  private async createGitCommit(reason: string): Promise<string | undefined> {
    const result = await GitOps.createCommit(`Checkpoint: ${reason || "Auto-save"}\n\nSession: ${this.sessionId}`, {
      addAll: true,
      allowEmpty: true,
    })

    if (result.success) {
      return result.commitHash
    }

    log.warn("Failed to create git commit", {
      error: result.error,
    })
    return undefined
  }

  /**
   * Restore state from checkpoint
   */
  private async restoreState(state: Record<string, unknown>): Promise<void> {
    try {
      await Storage.write(
        ["autonomous", "context", getProjectIDForStorage(this.sessionId), this.sessionId],
        state,
      )
    } catch (error) {
      log.error("Failed to restore state", {
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }

  /**
   * Restore git commit
   */
  private async restoreGitCommit(commitHash: string): Promise<void> {
    const result = await GitOps.resetToCommit(commitHash, true)

    if (!result.success) {
      log.error("Failed to restore git commit", {
        commitHash,
        error: result.error,
      })
      throw new Error(result.error ?? "Failed to restore git commit")
    }

    log.info("Git commit restored", { commitHash })
  }

  /**
   * Get current state machine state
   */
  private async getCurrentState(): Promise<string | undefined> {
    try {
      const context = await Storage.read<{ state: string }>([
        "autonomous",
        "context",
        Instance.project.id,
        this.sessionId,
      ])
      return context?.state
    } catch {
      return undefined
    }
  }

  /**
   * Persist checkpoints to storage
   */
  private async persist(): Promise<void> {
    try {
      const data = Array.from(this.checkpoints.values()).map((cp) => ({
        ...cp,
      }))

      await Storage.write(this.storageKey, { checkpoints: data })
    } catch (error) {
      log.error("Failed to persist checkpoints", {
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }

  /**
   * Load checkpoints from storage
   */
  async load(): Promise<boolean> {
    try {
      const data = await Storage.read<{ checkpoints: StoredCheckpoint[] }>(this.storageKey)

      if (!data?.checkpoints) {
        return false
      }

      this.checkpoints.clear()

      for (const cp of data.checkpoints) {
        this.checkpoints.set(cp.id, cp as Checkpoint)
      }

      log.info("Checkpoints loaded", { count: this.checkpoints.size })
      return true
    } catch {
      return false
    }
  }

  /**
   * Generate checkpoint ID
   */
  private generateCheckpointId(): string {
    return `checkpoint_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
  }

  /**
   * Serialize checkpoints
   */
  serialize(): { checkpoints: Checkpoint[] } {
    return {
      checkpoints: Array.from(this.checkpoints.values()),
    }
  }

  /**
   * Restore checkpoints from serialized data
   */
  static deserialize(data: { checkpoints: Checkpoint[] }, sessionId: string): CheckpointManager {
    const manager = new CheckpointManager(sessionId)

    for (const checkpoint of data.checkpoints) {
      manager.checkpoints.set(checkpoint.id, checkpoint)
    }

    return manager
  }
}

/**
 * Create a checkpoint manager
 */
export function createCheckpointManager(sessionId: string): CheckpointManager {
  const manager = new CheckpointManager(sessionId)

  // Auto-load existing checkpoints
  manager.load().catch(() => {
    // Ignore load errors
  })

  return manager
}
