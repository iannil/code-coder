/**
 * Multi-file configuration loader for CodeCoder.
 *
 * Supports modular configuration files:
 * - config.json     - Core configuration (~80 lines)
 * - secrets.json    - Credentials (gitignored)
 * - trading.json    - Trading module configuration
 * - channels.json   - IM channels configuration
 * - providers.json  - LLM provider configuration
 *
 * Files are loaded from ~/.codecoder/ with proper merging and fallbacks.
 */

import path from "path"
import os from "os"
import { Log } from "@/util/log"
import { mergeDeep } from "remeda"
import { parse as parseJsonc, type ParseError as JsoncParseError, printParseErrorCode } from "jsonc-parser"
import type { Config } from "./config"

const log = Log.create({ service: "config-loader" })

/** Configuration directory paths */
export const CONFIG_PATHS = {
  /** User's home .codecoder directory */
  home: path.join(os.homedir(), ".codecoder"),

  /** Core configuration file */
  config: "config.json",

  /** Secrets file (API keys, tokens) */
  secrets: "secrets.json",

  /** Trading module configuration */
  trading: "trading.json",

  /** IM channels configuration */
  channels: "channels.json",

  /** LLM providers configuration */
  providers: "providers.json",
} as const

type ConfigFile = "config" | "secrets" | "trading" | "channels" | "providers"

/**
 * Load a single JSON/JSONC configuration file.
 * Returns empty object if file doesn't exist.
 */
async function loadJsonFile<T>(filepath: string): Promise<T | null> {
  try {
    const text = await Bun.file(filepath).text()
    const errors: JsoncParseError[] = []
    const data = parseJsonc(text, errors, { allowTrailingComma: true })

    if (errors.length) {
      const errorDetails = errors.map((e) => printParseErrorCode(e.error)).join(", ")
      log.warn("JSONC parse errors", { path: filepath, errors: errorDetails })
      return null
    }

    return data as T
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      return null
    }
    log.error("Failed to load config file", { path: filepath, error })
    return null
  }
}

/**
 * Load secrets from secrets.json with security checks.
 */
async function loadSecrets(configDir: string): Promise<Config.Secrets | null> {
  const secretsPath = path.join(configDir, CONFIG_PATHS.secrets)

  // Check file permissions (warn if too permissive)
  try {
    const { stat } = await import("fs/promises")
    const stats = await stat(secretsPath)
    const mode = stats.mode & 0o777

    // Warn if file is readable by group or others (not 600)
    if (mode & 0o077) {
      log.warn("Secrets file has permissive permissions", {
        path: secretsPath,
        mode: mode.toString(8),
        recommended: "600",
      })
    }
  } catch {
    // File might not exist, that's okay
  }

  return loadJsonFile<Config.Secrets>(secretsPath)
}

/**
 * Extract secrets from legacy provider.*.options.apiKey format.
 */
function extractLegacySecrets(config: Config.Info): Config.Secrets {
  const llm: Record<string, string | null> = {}

  if (config.provider) {
    for (const [name, provider] of Object.entries(config.provider)) {
      if (name === "_settings") continue
      const apiKey = provider?.options?.apiKey
      if (apiKey && typeof apiKey === "string") {
        llm[name] = apiKey
      }
    }
  }

  return { llm }
}

/**
 * Load all configuration files and merge them.
 *
 * Priority (highest to lowest):
 * 1. Environment variables
 * 2. Modular files (secrets.json, trading.json, etc.)
 * 3. Legacy fields in config.json
 * 4. Default values
 */
export async function loadModularConfig(configDir?: string): Promise<Config.Info> {
  const dir = configDir ?? CONFIG_PATHS.home
  log.info("Loading modular configuration", { dir })

  // Load core config first
  const configPath = path.join(dir, CONFIG_PATHS.config)
  const baseConfig = (await loadJsonFile<Config.Info>(configPath)) ?? {}

  // Load modular config files
  const [secrets, trading, channels, providers] = await Promise.all([
    loadSecrets(dir),
    loadJsonFile<Config.Info["trading"]>(path.join(dir, CONFIG_PATHS.trading)),
    loadJsonFile<Config.Info["channels"]>(path.join(dir, CONFIG_PATHS.channels)),
    loadJsonFile<Record<string, unknown>>(path.join(dir, CONFIG_PATHS.providers)),
  ])

  // Start with base config
  let config = { ...baseConfig }

  // Merge secrets (modular file takes precedence over legacy)
  if (secrets) {
    const legacySecrets = extractLegacySecrets(config)
    config.secrets = mergeDeep(legacySecrets, secrets) as Config.Secrets
    log.debug("Loaded secrets from modular file")
  }

  // Merge trading config
  if (trading) {
    config.trading = mergeDeep(config.trading ?? {}, trading) as Config.Info["trading"]
    log.debug("Loaded trading config from modular file")
  }

  // Merge channels config
  if (channels) {
    // Channels in zerobot.channels format
    if (config.zerobot) {
      config.zerobot.channels = mergeDeep(config.zerobot.channels ?? {}, channels) as Config.ZeroBot["channels"]
    } else {
      config.zerobot = { channels } as Config.ZeroBot
    }
    log.debug("Loaded channels config from modular file")
  }

  // Merge providers config
  if (providers) {
    config.provider = mergeDeep(config.provider ?? {}, providers) as Config.Info["provider"]
    log.debug("Loaded providers config from modular file")
  }

  // Apply environment variable overrides
  config = applyEnvironmentOverrides(config)

  return config
}

/**
 * Apply environment variable overrides to config.
 */
function applyEnvironmentOverrides(config: Config.Info): Config.Info {
  const env = process.env

  // LLM API keys
  const llmKeyMap: Record<string, string> = {
    ANTHROPIC_API_KEY: "anthropic",
    OPENAI_API_KEY: "openai",
    DEEPSEEK_API_KEY: "deepseek",
    GOOGLE_API_KEY: "google",
    OPENROUTER_API_KEY: "openrouter",
    GROQ_API_KEY: "groq",
    MISTRAL_API_KEY: "mistral",
    XAI_API_KEY: "xai",
    TOGETHER_API_KEY: "together",
    FIREWORKS_API_KEY: "fireworks",
    PERPLEXITY_API_KEY: "perplexity",
  }

  for (const [envVar, provider] of Object.entries(llmKeyMap)) {
    if (env[envVar]) {
      config.secrets = config.secrets ?? {}
      config.secrets.llm = config.secrets.llm ?? {}
      config.secrets.llm[provider] = env[envVar]!
    }
  }

  // External API keys
  if (env.LIXIN_API_KEY) {
    config.secrets = config.secrets ?? {}
    config.secrets.external = config.secrets.external ?? {}
    config.secrets.external.lixin = env.LIXIN_API_KEY
  }
  if (env.ITICK_API_KEY) {
    config.secrets = config.secrets ?? {}
    config.secrets.external = config.secrets.external ?? {}
    config.secrets.external.itick = env.ITICK_API_KEY
  }

  // Channel tokens
  if (env.TELEGRAM_BOT_TOKEN) {
    config.secrets = config.secrets ?? {}
    config.secrets.channels = config.secrets.channels ?? {}
    config.secrets.channels.telegram_bot_token = env.TELEGRAM_BOT_TOKEN
  }

  return config
}

/**
 * Get the path to a modular config file.
 */
export function getConfigPath(file: ConfigFile, configDir?: string): string {
  const dir = configDir ?? CONFIG_PATHS.home
  return path.join(dir, CONFIG_PATHS[file])
}

/**
 * Check if modular config files exist.
 */
export async function hasModularConfig(configDir?: string): Promise<{
  config: boolean
  secrets: boolean
  trading: boolean
  channels: boolean
  providers: boolean
}> {
  const dir = configDir ?? CONFIG_PATHS.home

  const [config, secrets, trading, channels, providers] = await Promise.all([
    Bun.file(path.join(dir, CONFIG_PATHS.config)).exists(),
    Bun.file(path.join(dir, CONFIG_PATHS.secrets)).exists(),
    Bun.file(path.join(dir, CONFIG_PATHS.trading)).exists(),
    Bun.file(path.join(dir, CONFIG_PATHS.channels)).exists(),
    Bun.file(path.join(dir, CONFIG_PATHS.providers)).exists(),
  ])

  return { config, secrets, trading, channels, providers }
}
