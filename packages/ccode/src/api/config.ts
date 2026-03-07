import { Config } from "@/config/config"
import { fn } from "@/util/fn"
import z from "zod"

export namespace LocalConfig {
  export const get = async () => Config.get()

  /**
   * Update config with partial values.
   * Accepts partial Config.Info for type safety while allowing flexible API input.
   */
  export const update = async (updates: Partial<Config.Info>) => {
    // Config.update accepts full or partial Config.Info via mergeDeep internally
    await Config.update(updates as Config.Info)
    return true
  }
}
