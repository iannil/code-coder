import { realpathSync } from "fs"
import fs from "fs/promises"
import { dirname, join, relative } from "path"

export namespace Filesystem {
  export const exists = (p: string) =>
    Bun.file(p)
      .stat()
      .then(() => true)
      .catch(() => false)

  export const isDir = (p: string) =>
    Bun.file(p)
      .stat()
      .then((s) => s.isDirectory())
      .catch(() => false)
  /**
   * On Windows, normalize a path to its canonical casing using the filesystem.
   * This is needed because Windows paths are case-insensitive but LSP servers
   * may return paths with different casing than what we send them.
   */
  export function normalizePath(p: string): string {
    if (process.platform !== "win32") return p
    try {
      return realpathSync.native(p)
    } catch {
      return p
    }
  }
  export function overlaps(a: string, b: string) {
    const relA = relative(a, b)
    const relB = relative(b, a)
    return !relA || !relA.startsWith("..") || !relB || !relB.startsWith("..")
  }

  export function contains(parent: string, child: string) {
    return !relative(parent, child).startsWith("..")
  }

  /**
   * Safely check if a child path is contained within a parent directory.
   * Unlike `contains`, this resolves symlinks to prevent escape attacks.
   *
   * Security: Resolves both paths to their canonical form before comparison.
   * - Prevents symlink escape (project symlink pointing outside)
   * - Handles Windows cross-drive paths correctly
   *
   * @param parent - The parent directory (container)
   * @param child - The child path to check
   * @returns true if child is safely contained within parent
   */
  export async function containsSafe(parent: string, child: string): Promise<boolean> {
    try {
      // Resolve both paths to their canonical form
      // This follows symlinks and normalizes the path
      const [realParent, realChild] = await Promise.all([
        fs.realpath(parent).catch(() => parent),
        fs.realpath(child).catch(() => child),
      ])

      // On Windows, check for cross-drive access
      if (process.platform === "win32") {
        const parentDrive = realParent.match(/^([a-zA-Z]:)/)?.[1]?.toLowerCase()
        const childDrive = realChild.match(/^([a-zA-Z]:)/)?.[1]?.toLowerCase()
        if (parentDrive && childDrive && parentDrive !== childDrive) {
          return false
        }
      }

      // Check containment using the resolved paths
      const rel = relative(realParent, realChild)
      return !rel.startsWith("..") && !rel.startsWith("/")
    } catch {
      // If we can't resolve paths, fall back to lexical check
      // This can happen if the file doesn't exist yet (write operations)
      return contains(parent, child)
    }
  }

  export async function findUp(target: string, start: string, stop?: string) {
    let current = start
    const result = []
    while (true) {
      const search = join(current, target)
      if (await exists(search)) result.push(search)
      if (stop === current) break
      const parent = dirname(current)
      if (parent === current) break
      current = parent
    }
    return result
  }

  export async function* up(options: { targets: string[]; start: string; stop?: string }) {
    const { targets, start, stop } = options
    let current = start
    while (true) {
      for (const target of targets) {
        const search = join(current, target)
        if (await exists(search)) yield search
      }
      if (stop === current) break
      const parent = dirname(current)
      if (parent === current) break
      current = parent
    }
  }

  export async function globUp(pattern: string, start: string, stop?: string) {
    let current = start
    const result = []
    while (true) {
      try {
        const glob = new Bun.Glob(pattern)
        for await (const match of glob.scan({
          cwd: current,
          absolute: true,
          onlyFiles: true,
          followSymlinks: true,
          dot: true,
        })) {
          result.push(match)
        }
      } catch {
        // Skip invalid glob patterns
      }
      if (stop === current) break
      const parent = dirname(current)
      if (parent === current) break
      current = parent
    }
    return result
  }

  /**
   * Atomically write content to a file using temp file + rename.
   * This prevents data corruption if the process is interrupted during write.
   */
  export async function atomicWrite(filepath: string, content: string): Promise<void> {
    const dir = dirname(filepath)
    await fs.mkdir(dir, { recursive: true })
    const tempPath = `${filepath}.${Date.now()}.tmp`
    try {
      await Bun.write(tempPath, content)
      await fs.rename(tempPath, filepath)
    } catch (e) {
      await fs.unlink(tempPath).catch(() => {})
      throw e
    }
  }
}
