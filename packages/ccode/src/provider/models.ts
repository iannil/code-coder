import { Global } from "../global"
import { Log } from "../util/log"
import path from "path"
import z from "zod"
import { USER_AGENT } from "../version"
import { Flag } from "../flag/flag"
import { lazy } from "@/util/lazy"

// Try to import bundled snapshot (generated at build time)
// Falls back to undefined in dev mode when snapshot doesn't exist
/* @ts-ignore */

export namespace ModelsDev {
  const log = Log.create({ service: "models.dev" })
  const filepath = path.join(Global.Path.cache, "models.json")

  export const Model = z.object({
    id: z.string(),
    name: z.string(),
    family: z.string().optional(),
    release_date: z.string(),
    attachment: z.boolean(),
    reasoning: z.boolean(),
    temperature: z.boolean(),
    tool_call: z.boolean(),
    interleaved: z
      .union([
        z.literal(true),
        z
          .object({
            field: z.enum(["reasoning_content", "reasoning_details"]),
          })
          .strict(),
      ])
      .optional(),
    cost: z
      .object({
        input: z.number(),
        output: z.number(),
        cache_read: z.number().optional(),
        cache_write: z.number().optional(),
        context_over_200k: z
          .object({
            input: z.number(),
            output: z.number(),
            cache_read: z.number().optional(),
            cache_write: z.number().optional(),
          })
          .optional(),
      })
      .optional(),
    limit: z.object({
      context: z.number(),
      input: z.number().optional(),
      output: z.number(),
    }),
    modalities: z
      .object({
        input: z.array(z.enum(["text", "audio", "image", "video", "pdf"])),
        output: z.array(z.enum(["text", "audio", "image", "video", "pdf"])),
      })
      .optional(),
    experimental: z.boolean().optional(),
    status: z.enum(["alpha", "beta", "deprecated"]).optional(),
    options: z.record(z.string(), z.any()),
    headers: z.record(z.string(), z.string()).optional(),
    provider: z.object({ npm: z.string() }).optional(),
    variants: z.record(z.string(), z.record(z.string(), z.any())).optional(),
  })
  export type Model = z.infer<typeof Model>

  export const Provider = z.object({
    api: z.string().optional(),
    name: z.string(),
    env: z.array(z.string()),
    id: z.string(),
    npm: z.string().optional(),
    models: z.record(z.string(), Model),
  })

  export type Provider = z.infer<typeof Provider>

  function url() {
    return Flag.CCODE_MODELS_URL || "https://models.dev"
  }

  export const Data = lazy(async () => {
    const file = Bun.file(filepath)
    const result = await file.json().catch(() => {})
    if (result) return result
    // @ts-ignore
    const snapshot = await import("./models-snapshot")
      .then((m) => m.snapshot as Record<string, unknown>)
      .catch(() => undefined)
    if (snapshot) return snapshot
    if (Flag.CCODE_DISABLE_MODELS_FETCH) return {}
    // Fetch with Promise.race for reliable timeout
    try {
      const fetchPromise = fetch(`${url()}/api.json`, {
        headers: {
          "User-Agent": USER_AGENT,
        },
      })
      const timeoutPromise = new Promise((_, reject) => {
        setTimeout(() => reject(new Error("timeout")), 3000)
      })
      const response = await Promise.race([fetchPromise, timeoutPromise])
      if (!(response instanceof Response)) {
        throw new Error("fetch failed")
      }
      if (!response.ok) {
        log.warn("models.dev fetch failed", { status: response.status })
        return {}
      }
      const json = await response.text()
      return JSON.parse(json)
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e)
      if (errorMsg !== "timeout") {
        log.warn("models.dev fetch error", { error: errorMsg })
      }
      return {}
    }
  })

  export async function get() {
    const result = await Data()
    return result as Record<string, Provider>
  }

  export async function refresh() {
    const file = Bun.file(filepath)
    log.info("refreshing", {
      file,
    })
    try {
      const controller = new AbortController()
      const timeout = setTimeout(() => controller.abort(), 5000)

      const result = await fetch(`${url()}/api.json`, {
        headers: {
          "User-Agent": USER_AGENT,
        },
        signal: controller.signal,
      }).catch((e) => {
        log.error("Failed to fetch models.dev", {
          error: e,
        })
        return null
      })

      clearTimeout(timeout)

      if (result && result.ok) {
        await Bun.write(file, await result.text())
        ModelsDev.Data.reset()
        log.info("models.dev refreshed successfully")
      } else {
        log.warn("models.dev fetch failed", { status: result?.status })
      }
    } catch (e) {
      log.error("models.dev refresh error", {
        error: e instanceof Error ? e.message : String(e),
      })
    }
  }
}

// Don't block startup with models.dev fetch
if (!Flag.CCODE_DISABLE_MODELS_FETCH) {
  // Refresh in background after a short delay
  setTimeout(() => {
    ModelsDev.refresh().catch(() => {})
  }, 100)

  // Refresh every hour
  setInterval(
    async () => {
      await ModelsDev.refresh().catch(() => {})
    },
    60 * 1000 * 60,
  ).unref()
}
