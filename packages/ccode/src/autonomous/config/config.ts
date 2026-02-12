import { Log } from "@/util/log"
import { Config } from "@/config/config"
import type { AutonomousModeConfig } from "./schema"
import {
  mergeAutonomousModeConfig,
  DEFAULT_AUTONOMOUS_MODE_CONFIG,
  validateAutonomousModeConfig as validateConfig,
} from "./schema"

const log = Log.create({ service: "autonomous.config" })

export { DEFAULT_AUTONOMOUS_MODE_CONFIG, mergeAutonomousModeConfig }

export namespace AutonomousConfig {
  export async function get(): Promise<AutonomousModeConfig> {
    const cfg = await Config.get()

    const userConfig = cfg.autonomousMode as Partial<AutonomousModeConfig> | undefined

    return mergeAutonomousModeConfig(userConfig)
  }

  export async function isEnabled(): Promise<boolean> {
    const config = await get()
    return config.enabled
  }

  export async function getAutonomyLevel(): Promise<"lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"> {
    const config = await get()
    return config.autonomyLevel
  }

  export async function getResourceBudget() {
    const config = await get()
    return config.resourceLimits
  }

  export async function getCloseWeights() {
    const config = await get()
    return config.closeWeights
  }

  export async function getDecisionThreshold(): Promise<number> {
    const config = await get()
    return config.decisionThreshold
  }

  export async function getRiskTolerance(): Promise<"low" | "medium" | "high"> {
    const config = await get()
    return config.riskTolerance
  }

  export async function isUnattended(): Promise<boolean> {
    const config = await get()
    return config.unattended
  }

  export async function getCheckpointConfig() {
    const config = await get()
    return config.checkpoints
  }

  export async function getLoopDetectionConfig() {
    const config = await get()
    return config.loopDetection
  }

  export async function validate(): Promise<{
    valid: boolean
    errors: string[]
  }> {
    try {
      const config = await get()

      const errors: string[] = []

      if (config.resourceLimits.maxTokens <= 0) {
        errors.push("maxTokens must be positive")
      }

      if (config.resourceLimits.maxCostUSD <= 0) {
        errors.push("maxCostUSD must be positive")
      }

      if (config.decisionThreshold < 0 || config.decisionThreshold > 10) {
        errors.push("decisionThreshold must be between 0 and 10")
      }

      return {
        valid: errors.length === 0,
        errors,
      }
    } catch (error) {
      return {
        valid: false,
        errors: [error instanceof Error ? error.message : String(error)],
      }
    }
  }

  export const validateAutonomousModeConfig = validateConfig

  export function updateRuntime(updates: Partial<AutonomousModeConfig>): AutonomousModeConfig {
    return mergeAutonomousModeConfig(updates)
  }
}
