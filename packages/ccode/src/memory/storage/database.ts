import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Global } from "@/global"
import path from "path"
import z from "zod"

const log = Log.create({ service: "memory.storage.database" })

let db: Database | undefined

export namespace DatabaseStorage {
  export const ConnectionConfig = z.object({
    path: z.string().optional(),
    inMemory: z.boolean().optional(),
  })
  export type ConnectionConfig = z.infer<typeof ConnectionConfig>

  export async function connect(config?: ConnectionConfig): Promise<Database> {
    if (db) return db

    const dbPath = config?.path || path.join(Global.Path.data, "memory", "codecoder.db")

    db = new Database(dbPath, config?.inMemory)
    await db.initialize()

    return db
  }

  export async function close(): Promise<void> {
    if (db) {
      await db.close()
      db = undefined
    }
  }

  export async function get<T>(table: string, key: string): Promise<T | undefined> {
    const database = await connect()
    return database.get<T>(table, key)
  }

  export async function set<T>(table: string, key: string, value: T): Promise<void> {
    const database = await connect()
    await database.set(table, key, value)
  }

  export async function remove(table: string, key: string): Promise<void> {
    const database = await connect()
    await database.remove(table, key)
  }

  export async function list(table: string, prefix?: string): Promise<string[]> {
    const database = await connect()
    return database.list(table, prefix)
  }

  export async function clear(table?: string): Promise<void> {
    const database = await connect()
    await database.clear(table)
  }

  export async function query<T>(
    table: string,
    predicate: (value: T, key: string) => boolean,
  ): Promise<Array<{ key: string; value: T }>> {
    const database = await connect()
    return database.query(table, predicate)
  }

  export async function count(table: string): Promise<number> {
    const database = await connect()
    return database.count(table)
  }

  export async function transaction<T>(fn: (tx: Transaction) => Promise<T>): Promise<T> {
    const database = await connect()
    return database.transaction(fn)
  }

  export interface Transaction {
    get<T>(table: string, key: string): Promise<T | undefined>
    set<T>(table: string, key: string, value: T): Promise<void>
    remove(table: string, key: string): Promise<void>
  }
}

class Database {
  private tables = new Map<string, Map<string, any>>()
  private filePath: string
  private inMemory: boolean
  private initialized = false

  constructor(filePath: string, inMemory = false) {
    this.filePath = filePath
    this.inMemory = inMemory
  }

  async initialize(): Promise<void> {
    if (this.initialized) return

    if (!this.inMemory) {
      await this.loadFromDisk()
    }

    this.initialized = true
  }

  private async loadFromDisk(): Promise<void> {
    const { LocalStorage } = await import("./local")

    const tablesList = await LocalStorage.list(["memory", "db"])

    for (const tableName of tablesList) {
      const tableData = await LocalStorage.read<Record<string, any>>(["memory", "db", tableName])
      if (tableData) {
        const table = new Map<string, any>(Object.entries(tableData))
        this.tables.set(tableName, table)
      }
    }
  }

  private async saveToDisk(): Promise<void> {
    const { LocalStorage } = await import("./local")

    for (const [tableName, table] of this.tables.entries()) {
      const obj = Object.fromEntries(table.entries())
      await LocalStorage.write(["memory", "db", tableName], obj)
    }
  }

  async get<T>(table: string, key: string): Promise<T | undefined> {
    await this.initialize()
    return this.tables.get(table)?.get(key) as T | undefined
  }

  async set<T>(table: string, key: string, value: T): Promise<void> {
    await this.initialize()

    if (!this.tables.has(table)) {
      this.tables.set(table, new Map())
    }

    this.tables.get(table)!.set(key, value)

    if (!this.inMemory) {
      await this.saveToDisk()
    }
  }

  async remove(table: string, key: string): Promise<void> {
    await this.initialize()

    const tableMap = this.tables.get(table)
    if (tableMap) {
      tableMap.delete(key)

      if (!this.inMemory) {
        await this.saveToDisk()
      }
    }
  }

  async list(table: string, prefix?: string): Promise<string[]> {
    await this.initialize()

    const tableMap = this.tables.get(table)
    if (!tableMap) return []

    let keys = Array.from(tableMap.keys())

    if (prefix) {
      keys = keys.filter((k) => k.startsWith(prefix))
    }

    return keys
  }

  async clear(table?: string): Promise<void> {
    await this.initialize()

    if (table) {
      this.tables.delete(table)
    } else {
      this.tables.clear()
    }

    if (!this.inMemory) {
      await this.saveToDisk()
    }
  }

  async query<T>(
    table: string,
    predicate: (value: T, key: string) => boolean,
  ): Promise<Array<{ key: string; value: T }>> {
    await this.initialize()

    const tableMap = this.tables.get(table)
    if (!tableMap) return []

    const results: Array<{ key: string; value: T }> = []

    for (const [key, value] of tableMap.entries()) {
      if (predicate(value as T, key)) {
        results.push({ key, value: value as T })
      }
    }

    return results
  }

  async count(table: string): Promise<number> {
    await this.initialize()
    return this.tables.get(table)?.size || 0
  }

  async transaction<T>(fn: (tx: DatabaseStorage.Transaction) => Promise<T>): Promise<T> {
    const tx: DatabaseStorage.Transaction = {
      get: async <T>(table: string, key: string) => this.get<T>(table, key),
      set: async <T>(table: string, key: string, value: T) => this.set(table, key, value),
      remove: async (table: string, key: string) => this.remove(table, key),
    }

    const result = await fn(tx)

    if (!this.inMemory) {
      await this.saveToDisk()
    }

    return result
  }

  async close(): Promise<void> {
    if (!this.inMemory) {
      await this.saveToDisk()
    }
    this.tables.clear()
    this.initialized = false
  }

  getTableNames(): string[] {
    return Array.from(this.tables.keys())
  }

  getTableSize(table: string): number {
    return this.tables.get(table)?.size || 0
  }
}
