/**
 * Hooks API Handlers
 *
 * Handles hooks system operations:
 * - List configured hooks
 * - Hook status
 */

import type { RouteHandler } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { Hook } from "@/hook/hook"
import { Global } from "@/global"
import { Config } from "@/config/config"
import path from "path"

// ============================================================================
// Hooks Handlers
// ============================================================================

/**
 * Get all configured hooks
 * GET /api/hooks
 */
export const listHooks: RouteHandler = async () => {
  try {
    const configs = await Hook.load()

    // Transform hook configs into a flat list
    const hooks: Array<{
      lifecycle: string
      name: string
      definition: Hook.HookDefinition
      source: string
    }> = []

    for (const config of configs) {
      for (const lifecycle of Object.keys(config.hooks) as Hook.Lifecycle[]) {
        const lifecycleHooks = config.hooks[lifecycle]
        if (!lifecycleHooks) continue

        for (const [name, definition] of Object.entries(lifecycleHooks)) {
          hooks.push({
            lifecycle,
            name,
            definition,
            source: "config",
          })
        }
      }
    }

    return jsonResponse({ success: true, data: hooks })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get hooks by lifecycle
 * GET /api/hooks/:lifecycle
 */
export const getHooksByLifecycle: RouteHandler = async (_req, params) => {
  try {
    const lifecycle = params.lifecycle as Hook.Lifecycle
    if (!lifecycle) {
      return errorResponse("Lifecycle required", 400)
    }

    const validLifecycles = ["PreToolUse", "PostToolUse", "PreResponse", "Stop"]
    if (!validLifecycles.includes(lifecycle)) {
      return errorResponse(`Invalid lifecycle. Must be one of: ${validLifecycles.join(", ")}`, 400)
    }

    const configs = await Hook.load()

    const hooks: Array<{
      name: string
      definition: Hook.HookDefinition
    }> = []

    for (const config of configs) {
      const lifecycleHooks = config.hooks[lifecycle]
      if (!lifecycleHooks) continue

      for (const [name, definition] of Object.entries(lifecycleHooks)) {
        hooks.push({ name, definition })
      }
    }

    return jsonResponse({ success: true, data: hooks })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get hooks settings
 * GET /api/hooks/settings
 */
export const getHooksSettings: RouteHandler = async () => {
  try {
    const configs = await Hook.load()

    // Merge settings from all configs, later configs override earlier
    let settings: Hook.HooksConfig["settings"] = {
      enabled: true,
      blocking_mode: "interactive",
      log_level: "info",
    }

    for (const config of configs) {
      if (config.settings) {
        settings = { ...settings, ...config.settings }
      }
    }

    return jsonResponse({ success: true, data: settings })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get hook config file locations
 * GET /api/hooks/locations
 */
export const getHookLocations: RouteHandler = async () => {
  try {
    const directories = await Config.directories()
    const globalConfigPath = Global.Path.config

    const locations = [
      {
        path: path.join(globalConfigPath, "hooks", "hooks.json"),
        scope: "global",
        description: "Global hooks configuration",
      },
      ...directories.map((dir) => ({
        path: path.join(dir, "hooks", "hooks.json"),
        scope: "project",
        description: `Project hooks: ${dir}`,
      })),
    ]

    // Check which locations actually exist
    const locationsWithStatus = await Promise.all(
      locations.map(async (loc) => {
        const exists = await Bun.file(loc.path).exists()
        return { ...loc, exists }
      }),
    )

    return jsonResponse({ success: true, data: locationsWithStatus })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get available action types
 * GET /api/hooks/action-types
 */
export const getActionTypes: RouteHandler = async () => {
  try {
    const actionTypes = [
      {
        type: "scan",
        description: "Scan input/output for patterns",
        params: ["patterns", "message", "block"],
      },
      {
        type: "scan_content",
        description: "Scan file content for patterns",
        params: ["patterns", "message", "block"],
      },
      {
        type: "check_env",
        description: "Check environment variable",
        params: ["variable", "message", "block", "command_pattern"],
      },
      {
        type: "check_style",
        description: "Style check reminder",
        params: ["message"],
      },
      {
        type: "notify_only",
        description: "Show notification",
        params: ["message", "block"],
      },
      {
        type: "run_command",
        description: "Execute shell command",
        params: ["command", "async", "block", "on_output"],
      },
      {
        type: "analyze_changes",
        description: "Analyze code changes",
        params: [],
      },
      {
        type: "scan_files",
        description: "Scan files",
        params: ["file_pattern"],
      },
    ]

    return jsonResponse({ success: true, data: actionTypes })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
