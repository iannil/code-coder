import fs from "fs/promises"
import path from "path"
import os from "os"

// Helper to get home directory, respecting test override
const getHome = () => process.env.CCODE_TEST_HOME || os.homedir()

// All runtime paths are now unified under ~/.codecoder/
const configDir = () => path.join(getHome(), ".codecoder")

export namespace Global {
  export const Path = {
    // Allow override via CCODE_TEST_HOME for test isolation
    get home() {
      return getHome()
    },
    // All runtime data under ~/.codecoder/
    get config() {
      return configDir()
    },
    // Data directory for storage, memory, snapshots, reports
    get data() {
      return path.join(configDir(), "data")
    },
    // LSP and tool binaries
    get bin() {
      return path.join(configDir(), "bin")
    },
    // Unified log directory for TS and Rust services
    get logs() {
      return path.join(configDir(), "logs")
    },
    // Cache for models.json, package.json, node_modules
    get cache() {
      return path.join(configDir(), "cache")
    },
    // State files (kv.json, prompt-history.jsonl)
    get state() {
      return path.join(configDir(), "state")
    },
  }
}

await Promise.all([
  fs.mkdir(Global.Path.config, { recursive: true }),
  fs.mkdir(Global.Path.data, { recursive: true }),
  fs.mkdir(Global.Path.cache, { recursive: true }),
  fs.mkdir(Global.Path.state, { recursive: true }),
  fs.mkdir(Global.Path.logs, { recursive: true }),
  fs.mkdir(Global.Path.bin, { recursive: true }),
])

const CACHE_VERSION = "18"

const version = await Bun.file(path.join(Global.Path.cache, "version"))
  .text()
  .catch(() => "0")

if (version !== CACHE_VERSION) {
  try {
    const contents = await fs.readdir(Global.Path.cache)
    await Promise.all(
      contents.map((item) =>
        fs.rm(path.join(Global.Path.cache, item), {
          recursive: true,
          force: true,
        }),
      ),
    )
  } catch (e) {}
  await Bun.file(path.join(Global.Path.cache, "version")).write(CACHE_VERSION)
}
