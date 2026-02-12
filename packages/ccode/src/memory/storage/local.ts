import { Log } from "@/util/log"
import { Global } from "@/global"
import path from "path"
import { promises as fs } from "fs"
import { existsSync } from "fs"

const log = Log.create({ service: "memory.storage.local" })

export namespace LocalStorage {
  const MEMORY_DIR = "memory"

  export async function getPath(...segments: string[]): Promise<string> {
    const memoryDir = path.join(Global.Path.data, MEMORY_DIR)
    const fullPath = path.join(memoryDir, ...segments)

    const dir = path.dirname(fullPath)
    if (!existsSync(dir)) {
      await fs.mkdir(dir, { recursive: true })
    }

    return fullPath
  }

  export async function read<T>(key: string[]): Promise<T | undefined> {
    const filePath = await getPath(...key.map((k) => `${k}.json`))

    try {
      const content = await fs.readFile(filePath, "utf-8")
      return JSON.parse(content) as T
    } catch (error) {
      return undefined
    }
  }

  export async function write<T>(key: string[], value: T): Promise<void> {
    const filePath = await getPath(...key.map((k) => `${k}.json`))
    const content = JSON.stringify(value, null, 2)
    await fs.writeFile(filePath, content, "utf-8")
  }

  export async function remove(key: string[]): Promise<void> {
    const filePath = await getPath(...key.map((k) => `${k}.json`))

    try {
      await fs.unlink(filePath)
    } catch {}
  }

  export async function exists(key: string[]): Promise<boolean> {
    const filePath = await getPath(...key.map((k) => `${k}.json`))
    return existsSync(filePath)
  }

  export async function list(prefix: string[]): Promise<string[]> {
    const dirPath = await getPath(...prefix)

    try {
      const files = await fs.readdir(dirPath)
      return files.filter((f) => f.endsWith(".json")).map((f) => f.slice(0, -5))
    } catch {
      return []
    }
  }

  export async function clear(prefix?: string[]): Promise<void> {
    const dirPath = prefix ? await getPath(...prefix) : path.join(Global.Path.data, MEMORY_DIR)

    try {
      const files = await fs.readdir(dirPath)
      for (const file of files) {
        const filePath = path.join(dirPath, file)
        const stat = await fs.stat(filePath)
        if (stat.isDirectory()) {
          await fs.rm(filePath, { recursive: true, force: true })
        } else {
          await fs.unlink(filePath)
        }
      }
    } catch {}
  }

  export async function getSize(prefix?: string[]): Promise<number> {
    const dirPath = prefix ? await getPath(...prefix) : path.join(Global.Path.data, MEMORY_DIR)

    let size = 0

    async function calculateSize(dir: string): Promise<void> {
      try {
        const files = await fs.readdir(dir)
        for (const file of files) {
          const filePath = path.join(dir, file)
          const stat = await fs.stat(filePath)
          if (stat.isDirectory()) {
            await calculateSize(filePath)
          } else {
            size += stat.size
          }
        }
      } catch {}
    }

    await calculateSize(dirPath)
    return size
  }

  export async function exportData(prefix?: string[]): Promise<Record<string, any>> {
    const result: Record<string, any> = {}
    const dirPath = prefix ? await getPath(...prefix) : path.join(Global.Path.data, MEMORY_DIR)

    async function traverse(dir: string, currentKey: string[] = []): Promise<void> {
      try {
        const files = await fs.readdir(dir)
        for (const file of files) {
          const filePath = path.join(dir, file)
          const stat = await fs.stat(filePath)

          if (stat.isDirectory()) {
            await traverse(filePath, [...currentKey, file])
          } else if (file.endsWith(".json")) {
            try {
              const content = await fs.readFile(filePath, "utf-8")
              const key = file.slice(0, -5)
              const fullKey = [...currentKey, key].join("/")
              result[fullKey] = JSON.parse(content)
            } catch {}
          }
        }
      } catch {}
    }

    await traverse(dirPath)
    return result
  }

  export async function importData(data: Record<string, any>, prefix?: string[]): Promise<number> {
    let imported = 0

    for (const [key, value] of Object.entries(data)) {
      try {
        const keyParts = [...(prefix || []), ...key.split("/")]
        await write(keyParts, value)
        imported++
      } catch (error) {
        log.warn("failed to import key", { key, error })
      }
    }

    return imported
  }

  export async function cleanup(maxAge: number, prefix?: string[]): Promise<number> {
    const now = Date.now()
    let cleaned = 0
    const dirPath = prefix ? await getPath(...prefix) : path.join(Global.Path.data, MEMORY_DIR)

    async function traverse(dir: string): Promise<void> {
      try {
        const files = await fs.readdir(dir)
        for (const file of files) {
          const filePath = path.join(dir, file)
          const stat = await fs.stat(filePath)

          if (stat.isDirectory()) {
            await traverse(filePath)
          } else if (file.endsWith(".json")) {
            if (now - stat.mtimeMs > maxAge) {
              await fs.unlink(filePath)
              cleaned++
            }
          }
        }
      } catch {}
    }

    await traverse(dirPath)
    return cleaned
  }
}
