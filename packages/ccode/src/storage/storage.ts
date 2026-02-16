import { Log } from "@/util/log"
import path from "path"
import fs from "fs/promises"
import { Global } from "../global"
import { Filesystem } from "@/util/filesystem"
import { lazy } from "@/util/lazy"
import { Lock } from "@/util/lock"
import { $ } from "bun"
import { NamedError } from "@codecoder-ai/util/error"
import z from "zod"

export namespace Storage {
  const log = Log.create({ service: "storage" })

  type Migration = (dir: string) => Promise<void>

  export const NotFoundError = NamedError.create(
    "NotFoundError",
    z.object({
      message: z.string(),
    }),
  )

  export const CorruptedError = NamedError.create(
    "CorruptedError",
    z.object({
      path: z.string(),
      message: z.string(),
      originalError: z.string().optional(),
      recovered: z.boolean().optional(),
    }),
  )

  const MIGRATIONS: Migration[] = [
    async (dir) => {
      const project = path.resolve(dir, "../project")
      if (!(await Filesystem.isDir(project))) return
      for await (const projectDir of new Bun.Glob("*").scan({
        cwd: project,
        onlyFiles: false,
      })) {
        log.info(`migrating project ${projectDir}`)
        let projectID = projectDir
        const fullProjectDir = path.join(project, projectDir)
        let worktree = "/"

        if (projectID !== "global") {
          for await (const msgFile of new Bun.Glob("storage/session/message/*/*.json").scan({
            cwd: path.join(project, projectDir),
            absolute: true,
          })) {
            const json = await Bun.file(msgFile).json()
            worktree = json.path?.root
            if (worktree) break
          }
          if (!worktree) continue
          if (!(await Filesystem.isDir(worktree))) continue
          const [id] = await $`git rev-list --max-parents=0 --all`
            .quiet()
            .nothrow()
            .cwd(worktree)
            .text()
            .then((x) =>
              x
                .split("\n")
                .filter(Boolean)
                .map((x) => x.trim())
                .toSorted(),
            )
          if (!id) continue
          projectID = id

          await Bun.write(
            path.join(dir, "project", projectID + ".json"),
            JSON.stringify({
              id,
              vcs: "git",
              worktree,
              time: {
                created: Date.now(),
                initialized: Date.now(),
              },
            }),
          )

          log.info(`migrating sessions for project ${projectID}`)
          for await (const sessionFile of new Bun.Glob("storage/session/info/*.json").scan({
            cwd: fullProjectDir,
            absolute: true,
          })) {
            const dest = path.join(dir, "session", projectID, path.basename(sessionFile))
            log.info("copying", {
              sessionFile,
              dest,
            })
            const session = await Bun.file(sessionFile).json()
            await Bun.write(dest, JSON.stringify(session))
            log.info(`migrating messages for session ${session.id}`)
            for await (const msgFile of new Bun.Glob(`storage/session/message/${session.id}/*.json`).scan({
              cwd: fullProjectDir,
              absolute: true,
            })) {
              const dest = path.join(dir, "message", session.id, path.basename(msgFile))
              log.info("copying", {
                msgFile,
                dest,
              })
              const message = await Bun.file(msgFile).json()
              await Bun.write(dest, JSON.stringify(message))

              log.info(`migrating parts for message ${message.id}`)
              for await (const partFile of new Bun.Glob(`storage/session/part/${session.id}/${message.id}/*.json`).scan(
                {
                  cwd: fullProjectDir,
                  absolute: true,
                },
              )) {
                const dest = path.join(dir, "part", message.id, path.basename(partFile))
                const part = await Bun.file(partFile).json()
                log.info("copying", {
                  partFile,
                  dest,
                })
                await Bun.write(dest, JSON.stringify(part))
              }
            }
          }
        }
      }
    },
    async (dir) => {
      for await (const item of new Bun.Glob("session/*/*.json").scan({
        cwd: dir,
        absolute: true,
      })) {
        const session = await Bun.file(item).json()
        if (!session.projectID) continue
        if (!session.summary?.diffs) continue
        const { diffs } = session.summary
        await Bun.file(path.join(dir, "session_diff", session.id + ".json")).write(JSON.stringify(diffs))
        await Bun.file(path.join(dir, "session", session.projectID, session.id + ".json")).write(
          JSON.stringify({
            ...session,
            summary: {
              additions: diffs.reduce((sum: any, x: any) => sum + x.additions, 0),
              deletions: diffs.reduce((sum: any, x: any) => sum + x.deletions, 0),
            },
          }),
        )
      }
    },
  ]

  const state = lazy(async () => {
    const dir = path.join(Global.Path.data, "storage")
    const migration = await Bun.file(path.join(dir, "migration"))
      .json()
      .then((x) => parseInt(x))
      .catch(() => 0)
    for (let index = migration; index < MIGRATIONS.length; index++) {
      log.info("running migration", { index })
      const migration = MIGRATIONS[index]
      await migration(dir).catch(() => log.error("failed to run migration", { index }))
      await Bun.write(path.join(dir, "migration"), (index + 1).toString())
    }
    return {
      dir,
    }
  })

  const BACKUP_RETENTION_DAYS = 7
  const BACKUP_MAX_COUNT = 3

  async function isolateCorrupted(filepath: string, content: string) {
    const corrupted = path.join(Global.Path.data, "storage", "_corrupted")
    const timestamp = Date.now()
    const dest = path.join(corrupted, `${path.basename(filepath)}.${timestamp}`)
    await fs.mkdir(corrupted, { recursive: true })
    await fs.writeFile(dest, content)
    log.warn("Corrupted file isolated", { original: filepath, isolated: dest })
  }

  export async function remove(key: string[]) {
    const dir = await state().then((x) => x.dir)
    const target = path.join(dir, ...key) + ".json"
    return withErrorHandling(async () => {
      await fs.unlink(target).catch(() => {})
    })
  }

  export async function read<T>(key: string[]) {
    const dir = await state().then((x) => x.dir)
    const target = path.join(dir, ...key) + ".json"
    return withErrorHandling(async () => {
      using _ = await Lock.read(target)
      const file = Bun.file(target)
      const text = await file.text()
      try {
        return JSON.parse(text) as T
      } catch (parseError) {
        log.error("JSON parse error, attempting recovery", { path: target, error: parseError })

        // Isolate corrupted file
        await isolateCorrupted(target, text)

        // Attempt auto-recovery from backup
        const restored = await restore(key)
        if (restored) {
          log.info("Auto-recovered from backup", { key })
          const recoveredText = await Bun.file(target).text()
          return JSON.parse(recoveredText) as T
        }

        // Recovery failed
        throw new CorruptedError({
          path: target,
          message: "Failed to parse JSON and no valid backup available",
          originalError: parseError instanceof Error ? parseError.message : String(parseError),
          recovered: false,
        })
      }
    })
  }

  export async function update<T>(key: string[], fn: (draft: T) => void) {
    const dir = await state().then((x) => x.dir)
    const target = path.join(dir, ...key) + ".json"
    return withErrorHandling(async () => {
      using _ = await Lock.write(target)
      const file = Bun.file(target)
      const text = await file.text()
      let content: T
      try {
        content = JSON.parse(text)
      } catch (parseError) {
        // Attempt auto-recovery
        await isolateCorrupted(target, text)
        const restored = await restore(key)
        if (restored) {
          log.info("Auto-recovered from backup during update", { key })
          const recoveredText = await Bun.file(target).text()
          content = JSON.parse(recoveredText)
        } else {
          throw new CorruptedError({
            path: target,
            message: "Failed to parse JSON during update and no valid backup",
            originalError: parseError instanceof Error ? parseError.message : String(parseError),
            recovered: false,
          })
        }
      }

      // Backup before update
      await backup(key)

      fn(content)
      await Filesystem.atomicWrite(target, JSON.stringify(content, null, 2))
      return content as T
    })
  }

  export async function write<T>(key: string[], content: T) {
    const dir = await state().then((x) => x.dir)
    const target = path.join(dir, ...key) + ".json"
    return withErrorHandling(async () => {
      using _ = await Lock.write(target)

      // Backup existing file before overwrite
      if (await Bun.file(target).exists()) {
        await backup(key)
      }

      await Filesystem.atomicWrite(target, JSON.stringify(content, null, 2))
    })
  }

  async function withErrorHandling<T>(body: () => Promise<T>) {
    return body().catch((e) => {
      if (!(e instanceof Error)) throw e
      const errnoException = e as NodeJS.ErrnoException
      if (errnoException.code === "ENOENT") {
        throw new NotFoundError({ message: `Resource not found: ${errnoException.path}` })
      }
      throw e
    })
  }

  const glob = new Bun.Glob("**/*")
  export async function list(prefix: string[]) {
    const dir = await state().then((x) => x.dir)
    try {
      const result = await Array.fromAsync(
        glob.scan({
          cwd: path.join(dir, ...prefix),
          onlyFiles: true,
        }),
      ).then((results) => results.map((x) => [...prefix, ...x.slice(0, -5).split(path.sep)]))
      result.sort()
      return result
    } catch {
      return []
    }
  }

  export async function backup(key: string[]): Promise<string | undefined> {
    const dir = await state().then((x) => x.dir)
    const target = path.join(dir, ...key) + ".json"
    const file = Bun.file(target)

    if (!(await file.exists())) return undefined

    const backupDir = path.join(Global.Path.data, "storage", "_backup", ...key.slice(0, -1))
    const timestamp = Date.now()
    const backupPath = path.join(backupDir, `${key.at(-1)}.${timestamp}.json`)

    await fs.mkdir(backupDir, { recursive: true })
    await fs.copyFile(target, backupPath)

    await cleanupBackups(backupDir, key.at(-1)!)

    log.info("Backup created", { original: target, backup: backupPath })
    return backupPath
  }

  async function cleanupBackups(backupDir: string, basename: string) {
    const pattern = new Bun.Glob(`${basename}.*.json`)
    const files: { path: string; time: number }[] = []

    for await (const file of pattern.scan({ cwd: backupDir, absolute: true })) {
      const match = file.match(/\.(\d+)\.json$/)
      if (match) {
        files.push({ path: file, time: parseInt(match[1]) })
      }
    }

    files.sort((a, b) => b.time - a.time)

    const cutoff = Date.now() - BACKUP_RETENTION_DAYS * 24 * 60 * 60 * 1000

    for (let i = 0; i < files.length; i++) {
      if (i >= BACKUP_MAX_COUNT || files[i].time < cutoff) {
        await fs.unlink(files[i].path).catch(() => {})
      }
    }
  }

  export async function restore(key: string[]): Promise<boolean> {
    const dir = await state().then((x) => x.dir)
    const target = path.join(dir, ...key) + ".json"
    const backupDir = path.join(Global.Path.data, "storage", "_backup", ...key.slice(0, -1))
    const basename = key.at(-1)!

    const pattern = new Bun.Glob(`${basename}.*.json`)
    let latest: { path: string; time: number } | undefined

    try {
      for await (const file of pattern.scan({ cwd: backupDir, absolute: true })) {
        const match = file.match(/\.(\d+)\.json$/)
        if (match) {
          const time = parseInt(match[1])
          if (!latest || time > latest.time) {
            latest = { path: file, time }
          }
        }
      }
    } catch {
      log.warn("No backup directory found", { key })
      return false
    }

    if (!latest) {
      log.warn("No backup found", { key })
      return false
    }

    // Validate backup file is valid JSON
    try {
      const content = await Bun.file(latest.path).text()
      JSON.parse(content)
    } catch {
      log.error("Backup file is also corrupted", { backup: latest.path })
      return false
    }

    await fs.mkdir(path.dirname(target), { recursive: true })
    await fs.copyFile(latest.path, target)
    log.info("Restored from backup", { backup: latest.path, target })
    return true
  }

  export interface HealthReport {
    total: number
    healthy: number
    corrupted: { key: string[]; error: string }[]
    orphaned: string[]
  }

  export async function healthCheck(prefix: string[]): Promise<HealthReport> {
    const dir = await state().then((x) => x.dir)
    const report: HealthReport = {
      total: 0,
      healthy: 0,
      corrupted: [],
      orphaned: [],
    }

    for (const key of await list(prefix)) {
      report.total++
      const target = path.join(dir, ...key) + ".json"
      try {
        const content = await Bun.file(target).text()
        JSON.parse(content)
        report.healthy++
      } catch (e) {
        report.corrupted.push({
          key,
          error: e instanceof Error ? e.message : String(e),
        })
      }
    }

    return report
  }

  export async function listCorrupted(): Promise<string[]> {
    const corrupted = path.join(Global.Path.data, "storage", "_corrupted")
    try {
      const files = await fs.readdir(corrupted)
      return files.map((f) => path.join(corrupted, f))
    } catch {
      return []
    }
  }

  export async function clearCorrupted(): Promise<number> {
    const files = await listCorrupted()
    for (const file of files) {
      await fs.unlink(file).catch(() => {})
    }
    return files.length
  }
}
