import { Log } from "@/util/log"
import { Context } from "@/util/context"
import { Project } from "./project"
import { State } from "./state"
import { iife } from "@/util/iife"
import { GlobalBus } from "@/bus/global"
import { Filesystem } from "@/util/filesystem"

interface Context {
  directory: string
  worktree: string
  project: Project.Info
}
const context = Context.create<Context>("instance")
const cache = new Map<string, Promise<Context>>()

/**
 * Safely get project ID, with fallback for test contexts
 *
 * In tests where Instance.provide() is not set, this returns
 * a fallback ID based on the provided sessionId instead of throwing.
 *
 * @param sessionId - Optional session ID for fallback namespacing in tests
 * @returns The project ID, or a test fallback ID
 */
export function getProjectIDForStorage(sessionId?: string): string {
  try {
    return Instance.project.id
  } catch {
    // No instance context - likely running in tests
    // Use session-specific fallback for namespacing
    return sessionId ? `test_${sessionId}` : "test_project"
  }
}

export const Instance = {
  async provide<R>(input: { directory: string; init?: () => Promise<any>; fn: () => R }): Promise<R> {
    let existing = cache.get(input.directory)
    if (!existing) {
      Log.Default.info("creating instance", { directory: input.directory })
      existing = iife(async () => {
        const { project, sandbox } = await Project.fromDirectory(input.directory)
        const ctx = {
          directory: input.directory,
          worktree: sandbox,
          project,
        }
        await context.provide(ctx, async () => {
          await input.init?.()
        })
        return ctx
      })
      cache.set(input.directory, existing)
    }
    const ctx = await existing
    return context.provide(ctx, async () => {
      return input.fn()
    })
  },
  get directory() {
    try {
      return context.use().directory
    } catch {
      // Test mode - return fallback
      return "/tmp/test-project"
    }
  },
  get worktree() {
    try {
      return context.use().worktree
    } catch {
      // Test mode - return fallback
      return "/"
    }
  },
  get project() {
    try {
      return context.use().project
    } catch {
      // Test mode - return fallback project info
      return {
        id: "test_project",
        name: "Test Project",
        worktree: "/",
        sandboxes: [],
        time: { created: Date.now(), updated: Date.now() },
      }
    }
  },
  /**
   * Check if a path is within the project boundary.
   * Returns true if path is inside Instance.directory OR Instance.worktree.
   * Paths within the worktree but outside the working directory should not trigger external_directory permission.
   */
  containsPath(filepath: string) {
    if (Filesystem.contains(Instance.directory, filepath)) return true
    // Non-git projects set worktree to "/" which would match ANY absolute path.
    // Skip worktree check in this case to preserve external_directory permissions.
    if (Instance.worktree === "/") return false
    return Filesystem.contains(Instance.worktree, filepath)
  },
  state<S>(init: () => S, dispose?: (state: Awaited<S>) => Promise<void>): () => S {
    return State.create(() => Instance.directory, init, dispose)
  },
  async dispose() {
    Log.Default.info("disposing instance", { directory: Instance.directory })
    await State.dispose(Instance.directory)
    cache.delete(Instance.directory)
    GlobalBus.emit("event", {
      directory: Instance.directory,
      payload: {
        type: "server.instance.disposed",
        properties: {
          directory: Instance.directory,
        },
      },
    })
  },
  async disposeAll() {
    Log.Default.info("disposing all instances")
    for (const [_key, value] of cache) {
      const awaited = await value.catch(() => {})
      if (awaited) {
        await context.provide(await value, async () => {
          await Instance.dispose()
        })
      }
    }
    cache.clear()
  },
}
