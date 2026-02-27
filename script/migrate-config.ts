#!/usr/bin/env bun
/**
 * Migration script to convert legacy single-file config to modular format.
 *
 * Splits ~/.codecoder/config.json into:
 * - config.json     - Core configuration (~80 lines)
 * - secrets.json    - Credentials (gitignored, 600 permissions)
 * - trading.json    - Trading module configuration
 * - channels.json   - IM channels configuration
 * - providers.json  - LLM provider configuration
 *
 * Usage:
 *   bun run script/migrate-config.ts [--dry-run] [--force]
 *
 * Options:
 *   --dry-run  Show what would be done without making changes
 *   --force    Overwrite existing modular files
 */

import path from "path"
import os from "os"
import fs from "fs/promises"
import { parse as parseJsonc, type ParseError as JsoncParseError } from "jsonc-parser"

const CONFIG_DIR = path.join(os.homedir(), ".codecoder")
const CONFIG_PATH = path.join(CONFIG_DIR, "config.json")

interface MigrationOptions {
  dryRun: boolean
  force: boolean
}

interface MigrationResult {
  success: boolean
  backupPath?: string
  filesCreated: string[]
  errors: string[]
}

/**
 * Load the existing config.json file.
 */
async function loadConfig(): Promise<Record<string, unknown> | null> {
  try {
    const text = await Bun.file(CONFIG_PATH).text()
    const errors: JsoncParseError[] = []
    const data = parseJsonc(text, errors, { allowTrailingComma: true })

    if (errors.length) {
      console.error("❌ Config file has JSON syntax errors")
      return null
    }

    return data as Record<string, unknown>
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      console.error("❌ Config file not found:", CONFIG_PATH)
      return null
    }
    throw error
  }
}

/**
 * Extract LLM secrets from provider.*.options.apiKey format.
 */
function extractLlmSecrets(config: Record<string, unknown>): Record<string, string | null> {
  const secrets: Record<string, string | null> = {}
  const provider = config.provider as Record<string, unknown> | undefined

  if (!provider) return secrets

  for (const [name, value] of Object.entries(provider)) {
    if (name === "_settings") continue
    if (typeof value !== "object" || value === null) continue

    const providerConfig = value as Record<string, unknown>
    const options = providerConfig.options as Record<string, unknown> | undefined
    const apiKey = options?.apiKey

    if (apiKey && typeof apiKey === "string") {
      secrets[name] = apiKey
    }
  }

  return secrets
}

/**
 * Extract channel secrets.
 */
function extractChannelSecrets(config: Record<string, unknown>): Record<string, string | null> {
  const secrets: Record<string, string | null> = {}
  const zerobot = config.zerobot as Record<string, unknown> | undefined
  const channels = zerobot?.channels as Record<string, unknown> | undefined

  if (!channels) return secrets

  // Telegram
  const telegram = channels.telegram as Record<string, unknown> | undefined
  if (telegram?.bot_token) {
    secrets.telegram_bot_token = telegram.bot_token as string
  }

  // Discord
  const discord = channels.discord as Record<string, unknown> | undefined
  if (discord?.bot_token) {
    secrets.discord_bot_token = discord.bot_token as string
  }

  // Slack
  const slack = channels.slack as Record<string, unknown> | undefined
  if (slack?.bot_token) {
    secrets.slack_bot_token = slack.bot_token as string
  }
  if (slack?.app_token) {
    secrets.slack_app_token = slack.app_token as string
  }

  // Feishu
  const feishu = channels.feishu as Record<string, unknown> | undefined
  if (feishu?.app_id) {
    secrets.feishu_app_id = feishu.app_id as string
  }
  if (feishu?.app_secret) {
    secrets.feishu_app_secret = feishu.app_secret as string
  }

  return secrets
}

/**
 * Extract external service secrets.
 */
function extractExternalSecrets(config: Record<string, unknown>): Record<string, string | null> {
  const secrets: Record<string, string | null> = {}

  // From config.secrets.external
  const existingSecrets = config.secrets as Record<string, unknown> | undefined
  const external = existingSecrets?.external as Record<string, unknown> | undefined
  if (external) {
    for (const [key, value] of Object.entries(external)) {
      if (typeof value === "string" || value === null) {
        secrets[key] = value
      }
    }
  }

  // From trading config (deprecated fields)
  const trading = config.trading as Record<string, unknown> | undefined
  if (trading?.lixin_token) {
    secrets.lixin = trading.lixin_token as string
  }
  if (trading?.itick_api_key) {
    secrets.itick = trading.itick_api_key as string
  }

  return secrets
}

/**
 * Build the secrets.json content.
 */
function buildSecretsConfig(config: Record<string, unknown>): Record<string, unknown> {
  const llm = extractLlmSecrets(config)
  const channels = extractChannelSecrets(config)
  const external = extractExternalSecrets(config)

  return {
    $schema: "https://code-coder.com/schemas/secrets.json",
    llm: Object.keys(llm).length > 0 ? llm : undefined,
    channels: Object.keys(channels).length > 0 ? channels : undefined,
    external: Object.keys(external).length > 0 ? external : undefined,
  }
}

/**
 * Build the trading.json content.
 */
function buildTradingConfig(config: Record<string, unknown>): Record<string, unknown> | null {
  const trading = config.trading as Record<string, unknown> | undefined
  if (!trading) return null

  // Remove deprecated fields that are now in secrets
  const { lixin_token, itick_api_key, ...rest } = trading
  if (Object.keys(rest).length === 0) return null

  return {
    $schema: "https://code-coder.com/schemas/trading.json",
    ...rest,
  }
}

/**
 * Build the channels.json content.
 */
function buildChannelsConfig(config: Record<string, unknown>): Record<string, unknown> | null {
  const zerobot = config.zerobot as Record<string, unknown> | undefined
  const channels = zerobot?.channels as Record<string, unknown> | undefined
  if (!channels) return null

  // Remove secret fields from channel configs
  const cleanedChannels: Record<string, unknown> = {}

  for (const [name, channelConfig] of Object.entries(channels)) {
    if (typeof channelConfig !== "object" || channelConfig === null) {
      cleanedChannels[name] = channelConfig
      continue
    }

    const config = { ...(channelConfig as Record<string, unknown>) }

    // Remove secret fields
    if (name === "telegram") {
      delete config.bot_token
    } else if (name === "discord") {
      delete config.bot_token
    } else if (name === "slack") {
      delete config.bot_token
      delete config.app_token
    } else if (name === "feishu") {
      delete config.app_id
      delete config.app_secret
    }

    if (Object.keys(config).length > 0) {
      cleanedChannels[name] = config
    }
  }

  if (Object.keys(cleanedChannels).length === 0) return null

  return {
    $schema: "https://code-coder.com/schemas/channels.json",
    ...cleanedChannels,
  }
}

/**
 * Build the providers.json content.
 */
function buildProvidersConfig(config: Record<string, unknown>): Record<string, unknown> | null {
  const provider = config.provider as Record<string, unknown> | undefined
  if (!provider) return null

  // Remove API keys from provider options
  const cleanedProviders: Record<string, unknown> = {}

  for (const [name, providerConfig] of Object.entries(provider)) {
    if (name === "_settings") {
      cleanedProviders[name] = providerConfig
      continue
    }

    if (typeof providerConfig !== "object" || providerConfig === null) {
      cleanedProviders[name] = providerConfig
      continue
    }

    const config = { ...(providerConfig as Record<string, unknown>) }
    const options = config.options as Record<string, unknown> | undefined

    if (options) {
      const { apiKey, ...restOptions } = options
      if (Object.keys(restOptions).length > 0) {
        config.options = restOptions
      } else {
        delete config.options
      }
    }

    if (Object.keys(config).length > 0) {
      cleanedProviders[name] = config
    }
  }

  if (Object.keys(cleanedProviders).length === 0) return null

  return {
    $schema: "https://code-coder.com/schemas/providers.json",
    ...cleanedProviders,
  }
}

/**
 * Build the cleaned core config.json.
 */
function buildCoreConfig(config: Record<string, unknown>): Record<string, unknown> {
  const coreConfig = { ...config }

  // Remove fields that are now in separate files
  delete coreConfig.secrets
  delete coreConfig.trading
  delete coreConfig.provider

  // Clean up zerobot.channels
  const zerobot = coreConfig.zerobot as Record<string, unknown> | undefined
  if (zerobot) {
    const { channels, ...restZerobot } = zerobot
    if (Object.keys(restZerobot).length > 0) {
      coreConfig.zerobot = restZerobot
    } else {
      delete coreConfig.zerobot
    }
  }

  // Add schema reference
  coreConfig.$schema = "https://code-coder.com/schemas/config.json"

  return coreConfig
}

/**
 * Write a config file with optional permissions.
 */
async function writeConfigFile(
  filepath: string,
  content: Record<string, unknown>,
  options: MigrationOptions,
  permissions?: number
): Promise<boolean> {
  const exists = await Bun.file(filepath).exists()

  if (exists && !options.force) {
    console.log(`   ⚠️  Skipping ${path.basename(filepath)} (exists, use --force to overwrite)`)
    return false
  }

  if (options.dryRun) {
    console.log(`   📝 Would write ${path.basename(filepath)}`)
    return true
  }

  await Bun.write(filepath, JSON.stringify(content, null, 2) + "\n")

  if (permissions) {
    await fs.chmod(filepath, permissions)
  }

  console.log(`   ✅ Written ${path.basename(filepath)}`)
  return true
}

/**
 * Update .gitignore to include secrets.json.
 */
async function updateGitignore(options: MigrationOptions): Promise<void> {
  const gitignorePath = path.join(CONFIG_DIR, ".gitignore")
  const secretsEntry = "secrets.json"

  try {
    const content = await Bun.file(gitignorePath).text()
    if (content.includes(secretsEntry)) {
      console.log("   ℹ️  .gitignore already includes secrets.json")
      return
    }

    if (options.dryRun) {
      console.log("   📝 Would add secrets.json to .gitignore")
      return
    }

    await Bun.write(gitignorePath, content + "\n" + secretsEntry + "\n")
    console.log("   ✅ Added secrets.json to .gitignore")
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      if (options.dryRun) {
        console.log("   📝 Would create .gitignore with secrets.json")
        return
      }
      await Bun.write(gitignorePath, secretsEntry + "\n")
      console.log("   ✅ Created .gitignore with secrets.json")
    }
  }
}

/**
 * Run the migration.
 */
async function migrate(options: MigrationOptions): Promise<MigrationResult> {
  const result: MigrationResult = {
    success: false,
    filesCreated: [],
    errors: [],
  }

  console.log("")
  console.log("🔄 CodeCoder Configuration Migration")
  console.log("=====================================")
  console.log("")

  if (options.dryRun) {
    console.log("🔍 DRY RUN MODE - No changes will be made")
    console.log("")
  }

  // Load existing config
  console.log("📖 Loading existing configuration...")
  const config = await loadConfig()
  if (!config) {
    result.errors.push("Failed to load config.json")
    return result
  }
  console.log(`   ✅ Loaded config.json (${Object.keys(config).length} top-level keys)`)
  console.log("")

  // Create backup
  console.log("💾 Creating backup...")
  const backupPath = `${CONFIG_PATH}.backup.${Date.now()}`
  if (!options.dryRun) {
    await fs.copyFile(CONFIG_PATH, backupPath)
    result.backupPath = backupPath
    console.log(`   ✅ Backup created: ${path.basename(backupPath)}`)
  } else {
    console.log(`   📝 Would create backup: config.json.backup.${Date.now()}`)
  }
  console.log("")

  // Build and write modular configs
  console.log("📦 Extracting modular configuration files...")
  console.log("")

  // Secrets (with 600 permissions)
  const secrets = buildSecretsConfig(config)
  if (Object.keys(secrets).length > 1) {
    const secretsPath = path.join(CONFIG_DIR, "secrets.json")
    if (await writeConfigFile(secretsPath, secrets, options, 0o600)) {
      result.filesCreated.push("secrets.json")
    }
  }

  // Trading
  const trading = buildTradingConfig(config)
  if (trading) {
    const tradingPath = path.join(CONFIG_DIR, "trading.json")
    if (await writeConfigFile(tradingPath, trading, options)) {
      result.filesCreated.push("trading.json")
    }
  }

  // Channels
  const channels = buildChannelsConfig(config)
  if (channels) {
    const channelsPath = path.join(CONFIG_DIR, "channels.json")
    if (await writeConfigFile(channelsPath, channels, options)) {
      result.filesCreated.push("channels.json")
    }
  }

  // Providers
  const providers = buildProvidersConfig(config)
  if (providers) {
    const providersPath = path.join(CONFIG_DIR, "providers.json")
    if (await writeConfigFile(providersPath, providers, options)) {
      result.filesCreated.push("providers.json")
    }
  }

  // Core config (cleaned)
  const coreConfig = buildCoreConfig(config)
  if (await writeConfigFile(CONFIG_PATH, coreConfig, options)) {
    result.filesCreated.push("config.json (updated)")
  }

  console.log("")

  // Update .gitignore
  console.log("🔒 Updating .gitignore...")
  await updateGitignore(options)
  console.log("")

  // Summary
  console.log("📊 Migration Summary")
  console.log("====================")
  console.log(`   Files created/updated: ${result.filesCreated.length}`)
  for (const file of result.filesCreated) {
    console.log(`     - ${file}`)
  }
  if (result.backupPath) {
    console.log(`   Backup: ${path.basename(result.backupPath)}`)
  }
  console.log("")

  if (options.dryRun) {
    console.log("ℹ️  Run without --dry-run to apply changes")
  } else {
    console.log("✅ Migration complete!")
    console.log("")
    console.log("💡 Next steps:")
    console.log("   1. Review the generated files in ~/.codecoder/")
    console.log("   2. Verify secrets.json has correct permissions (600)")
    console.log("   3. Test your configuration: bun dev")
  }
  console.log("")

  result.success = true
  return result
}

// Parse CLI arguments
const args = process.argv.slice(2)
const options: MigrationOptions = {
  dryRun: args.includes("--dry-run"),
  force: args.includes("--force"),
}

if (args.includes("--help") || args.includes("-h")) {
  console.log(`
Usage: bun run script/migrate-config.ts [options]

Migrate legacy single-file config to modular format.

Options:
  --dry-run  Show what would be done without making changes
  --force    Overwrite existing modular files
  --help     Show this help message

Files created:
  config.json     Core configuration (cleaned)
  secrets.json    Credentials (600 permissions, gitignored)
  trading.json    Trading module configuration
  channels.json   IM channels configuration
  providers.json  LLM provider configuration
`)
  process.exit(0)
}

migrate(options).catch((error) => {
  console.error("Fatal error:", error)
  process.exit(1)
})
