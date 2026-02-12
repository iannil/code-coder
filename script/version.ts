// Script utilities for build and publish
import { readFileSync } from "fs"
import { join, dirname } from "path"
import { fileURLToPath } from "url"

function getPackageVersion(): string {
  // Try environment variable first
  if (process.env.VERSION) return process.env.VERSION

  // Try to read from root package.json
  try {
    const __dirname = dirname(fileURLToPath(import.meta.url))
    const rootPkg = JSON.parse(readFileSync(join(__dirname, "../packages/ccode/package.json"), "utf-8"))
    return rootPkg.version || "0.0.0"
  } catch {
    return "0.0.0"
  }
}

export const Script = {
  get version(): string {
    return process.env.VERSION || getPackageVersion()
  },
  get channel(): string {
    return process.env.CHANNEL || "latest"
  },
  get preview(): boolean {
    return process.env.PREVIEW === "true" || Script.version.includes("-")
  },
}
