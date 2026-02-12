import { Config } from "@/config/config"
import { fn } from "@/util/fn"
import z from "zod"

export namespace LocalConfig {
  export const get = async () => Config.get()

  export const update = async (updates: Record<string, any>) => {
    await Config.update(updates as any)
    return true
  }
}
