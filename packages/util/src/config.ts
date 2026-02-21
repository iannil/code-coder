/**
 * TypeScript type definitions for Zero ecosystem configuration.
 *
 * These types mirror the Rust config structures in `services/zero-common/src/config.rs`.
 * Keep these in sync when modifying the Rust configuration.
 */

import z from "zod"
import { watch, type FSWatcher } from "fs"

/**
 * Root configuration structure for all Zero services.
 * Stored at `~/.codecoder/config.json`.
 */
export interface Config {
  /** Gateway configuration */
  gateway?: GatewayConfig
  /** Channels configuration */
  channels?: ChannelsConfig
  /** Workflow configuration */
  workflow?: WorkflowConfig
  /** CodeCoder integration */
  codecoder?: CodeCoderConfig
  /** Observability configuration */
  observability?: ObservabilityConfig
  /** Memory/persistence configuration */
  memory?: MemoryConfig
}

/**
 * Gateway service configuration.
 */
export interface GatewayConfig {
  /** Gateway HTTP port (default: 4402) */
  port?: number
  /** Gateway HTTP host (default: "127.0.0.1") */
  host?: string
  /** JWT secret for token signing (auto-generated if not set) */
  jwt_secret?: string
  /** Token expiry in seconds (default: 86400 = 24 hours) */
  token_expiry_secs?: number
  /** Enable rate limiting (default: true) */
  rate_limiting?: boolean
  /** Requests per minute per user (default: 60) */
  rate_limit_rpm?: number
  /** CodeCoder API endpoint to proxy to */
  codecoder_endpoint?: string
}

/**
 * Channels service configuration.
 */
export interface ChannelsConfig {
  /** Channels HTTP port (default: 4404) */
  port?: number
  /** Channels HTTP host (default: "127.0.0.1") */
  host?: string
  /** Telegram bot configuration */
  telegram?: TelegramConfig
  /** Discord bot configuration */
  discord?: DiscordConfig
  /** Slack bot configuration */
  slack?: SlackConfig
  /** Feishu bot configuration */
  feishu?: FeishuConfig
  /** TTS configuration */
  tts?: TtsConfig
  /** STT configuration */
  stt?: SttConfig
}

/**
 * Telegram channel configuration.
 */
export interface TelegramConfig {
  enabled: boolean
  bot_token: string
  allowed_users?: string[]
  allowed_chats?: number[]
}

/**
 * Discord channel configuration.
 */
export interface DiscordConfig {
  enabled: boolean
  bot_token: string
  allowed_guilds?: string[]
  allowed_channels?: string[]
}

/**
 * Slack channel configuration.
 */
export interface SlackConfig {
  enabled: boolean
  bot_token: string
  app_token: string
  signing_secret?: string
}

/**
 * Feishu channel configuration.
 */
export interface FeishuConfig {
  enabled: boolean
  app_id: string
  app_secret: string
  encrypt_key?: string
  verification_token?: string
  allowed_users?: string[]
}

/**
 * TTS (Text-to-Speech) configuration.
 */
export interface TtsConfig {
  /** Provider: "openai" | "elevenlabs" | "azure" */
  provider: string
  api_key?: string
  voice?: string
}

/**
 * STT (Speech-to-Text) configuration.
 */
export interface SttConfig {
  /** Provider: "openai" | "azure" | "google" | "compatible" */
  provider: string
  api_key?: string
  model?: string
}

/**
 * Workflow service configuration.
 */
export interface WorkflowConfig {
  /** Cron scheduler configuration */
  cron?: CronConfig
  /** Webhook configuration */
  webhook?: WebhookConfig
  /** Git integration configuration */
  git?: GitIntegrationConfig
}

/**
 * Cron scheduler configuration.
 */
export interface CronConfig {
  /** Enable cron scheduler */
  enabled?: boolean
  /** Scheduled tasks */
  tasks?: CronTask[]
}

/**
 * A scheduled cron task.
 */
export interface CronTask {
  /** Task ID */
  id: string
  /** Cron expression (6-field format) */
  expression: string
  /** Command or workflow to execute */
  command: string
  /** Task description */
  description?: string
}

/**
 * Webhook configuration.
 */
export interface WebhookConfig {
  /** Enable webhook receiver */
  enabled?: boolean
  /** Webhook port (if separate from gateway) */
  port?: number
  /** Webhook secret for signature verification */
  secret?: string
}

/**
 * Git integration configuration.
 */
export interface GitIntegrationConfig {
  /** Enable Git webhook handling */
  enabled?: boolean
  /** GitHub webhook secret */
  github_secret?: string
  /** GitLab webhook token */
  gitlab_token?: string
}

/**
 * CodeCoder integration configuration.
 */
export interface CodeCoderConfig {
  /** Enable CodeCoder integration (default: true) */
  enabled?: boolean
  /** CodeCoder API endpoint (default: "http://127.0.0.1:4400") */
  endpoint?: string
  /** API timeout in seconds (default: 300) */
  timeout_secs?: number
}

/**
 * Observability configuration.
 */
export interface ObservabilityConfig {
  /** Log level: "trace" | "debug" | "info" | "warn" | "error" (default: "info") */
  log_level?: "trace" | "debug" | "info" | "warn" | "error"
  /** Log format: "json" | "pretty" (default: "pretty") */
  log_format?: "json" | "pretty"
  /** Enable request tracing (default: true) */
  tracing?: boolean
}

/**
 * Memory/persistence configuration.
 */
export interface MemoryConfig {
  /** Backend type: "sqlite" | "postgres" (default: "sqlite") */
  backend?: "sqlite" | "postgres"
  /** Database path (for SQLite) */
  path?: string
  /** Connection string (for PostgreSQL) */
  connection_string?: string
}

// ============================================================================
// Workflow Types (mirror services/zero-workflow/src/workflow.rs)
// ============================================================================

/**
 * Workflow definition.
 */
export interface Workflow {
  /** Workflow name */
  name: string
  /** Workflow description */
  description?: string
  /** Trigger configuration */
  trigger: Trigger
  /** Workflow steps */
  steps: Step[]
  /** Global variables */
  vars?: Record<string, unknown>
}

/**
 * Workflow trigger.
 */
export type Trigger =
  | { type: "webhook"; events?: string[]; filter?: TriggerFilter }
  | { type: "cron"; expression: string }
  | { type: "manual" }

/**
 * Trigger filter for webhook events.
 */
export interface TriggerFilter {
  /** Branch filter (for git events) */
  branch?: string
  /** Action filter (e.g., "opened", "closed") */
  action?: string[]
  /** Custom JSONPath conditions */
  conditions?: string[]
}

/**
 * Workflow step.
 */
export interface Step {
  /** Step name */
  name: string
  /** Condition for running this step (expression) */
  condition?: string
  /** Continue on error */
  continue_on_error?: boolean
  /** Timeout in seconds */
  timeout_secs?: number
}

/**
 * Step types.
 */
export type StepType =
  | { type: "agent"; agent: string; input: unknown }
  | { type: "notify"; channel: string; template: string }
  | { type: "shell"; command: string; cwd?: string }
  | { type: "http"; method: string; url: string; body?: unknown; headers?: Record<string, string> }

/**
 * Execution status.
 */
export type ExecutionStatus = "running" | "success" | "failed" | "cancelled"

/**
 * Workflow execution result.
 */
export interface WorkflowResult {
  /** Workflow name */
  workflow: string
  /** Execution ID */
  execution_id: string
  /** Overall status */
  status: ExecutionStatus
  /** Step results */
  steps: StepResult[]
  /** Start time (Unix millis) */
  started_at: number
  /** End time (Unix millis) */
  ended_at?: number
}

/**
 * Step execution result.
 */
export interface StepResult {
  /** Step name */
  name: string
  /** Step status */
  status: ExecutionStatus
  /** Step output */
  output?: unknown
  /** Error message (if failed) */
  error?: string
  /** Duration in milliseconds */
  duration_ms: number
}

// ============================================================================
// Channel Message Types (mirror services/zero-channels/src/message.rs)
// ============================================================================

/**
 * Channel type enum.
 */
export type ChannelType =
  | "telegram"
  | "discord"
  | "slack"
  | "feishu"
  | "whatsapp"
  | "matrix"
  | "imessage"
  | "email"
  | "cli"

/**
 * Unified message format for all channels.
 */
export interface ChannelMessage {
  /** Message ID (channel-specific) */
  id: string
  /** Channel type */
  channel_type: ChannelType
  /** Channel-specific identifier (chat ID, channel ID, etc.) */
  channel_id: string
  /** User identifier */
  user_id: string
  /** Message content */
  content: MessageContent
  /** Attachments (images, files, voice, etc.) */
  attachments?: Attachment[]
  /** Additional metadata */
  metadata?: Record<string, string>
  /** Timestamp (Unix millis) */
  timestamp: number
}

/**
 * Message content types.
 */
export type MessageContent =
  | { type: "text"; text: string }
  | { type: "voice"; url: string; duration_secs?: number }
  | { type: "image"; url: string; caption?: string }
  | { type: "file"; url: string; filename: string; mime_type?: string }
  | { type: "location"; latitude: number; longitude: number; title?: string }

/**
 * Attachment types.
 */
export interface Attachment {
  attachment_type: AttachmentType
  url: string
  filename?: string
  mime_type?: string
  size_bytes?: number
}

/**
 * Attachment type enum.
 */
export type AttachmentType = "image" | "audio" | "video" | "document" | "other"

/**
 * Outgoing message to send to a channel.
 */
export interface OutgoingMessage {
  /** Target channel type */
  channel_type: ChannelType
  /** Target channel ID */
  channel_id: string
  /** Reply to message ID (optional) */
  reply_to?: string
  /** Message content */
  content: OutgoingContent
}

/**
 * Outgoing message content.
 */
export type OutgoingContent =
  | { type: "text"; text: string }
  | { type: "markdown"; text: string }
  | { type: "voice"; data: Uint8Array; format: string }
  | { type: "image"; data: Uint8Array; caption?: string }
  | { type: "file"; data: Uint8Array; filename: string }

// ============================================================================
// Default Configuration Values
// ============================================================================

export const DEFAULT_CONFIG: Config = {
  gateway: {
    port: 4402,
    host: "127.0.0.1",
    token_expiry_secs: 86400,
    rate_limiting: true,
    rate_limit_rpm: 60,
    codecoder_endpoint: "http://127.0.0.1:4400",
  },
  channels: {
    port: 4404,
    host: "127.0.0.1",
  },
  workflow: {
    cron: { enabled: false, tasks: [] },
    webhook: { enabled: false },
    git: { enabled: false },
  },
  codecoder: {
    enabled: true,
    endpoint: "http://127.0.0.1:4400",
    timeout_secs: 300,
  },
  observability: {
    log_level: "info",
    log_format: "pretty",
    tracing: true,
  },
  memory: {
    backend: "sqlite",
  },
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Get the configuration directory path.
 */
export function configDir(): string {
  const home = process.env.HOME || process.env.USERPROFILE || "."
  return `${home}/.codecoder`
}

/**
 * Get the configuration file path.
 */
export function configPath(): string {
  return `${configDir()}/config.json`
}

// ============================================================================
// Config Schema (Zod validation)
// ============================================================================

/**
 * Zod schema for the root configuration.
 * Provides runtime validation and TypeScript type inference.
 */
export const ConfigSchema = z.object({
  gateway: z.object({
    port: z.number().int().positive().optional(),
    host: z.string().optional(),
    jwt_secret: z.string().optional(),
    token_expiry_secs: z.number().int().positive().optional(),
    rate_limiting: z.boolean().optional(),
    rate_limit_rpm: z.number().int().positive().optional(),
    codecoder_endpoint: z.string().optional(),
  }).optional(),
  channels: z.object({
    port: z.number().int().positive().optional(),
    host: z.string().optional(),
    telegram: z.object({
      enabled: z.boolean(),
      bot_token: z.string(),
      allowed_users: z.array(z.string()).optional(),
      allowed_chats: z.array(z.number()).optional(),
    }).optional(),
    discord: z.object({
      enabled: z.boolean(),
      bot_token: z.string(),
      allowed_guilds: z.array(z.string()).optional(),
      allowed_channels: z.array(z.string()).optional(),
    }).optional(),
    slack: z.object({
      enabled: z.boolean(),
      bot_token: z.string(),
      app_token: z.string(),
      signing_secret: z.string().optional(),
    }).optional(),
    feishu: z.object({
      enabled: z.boolean(),
      app_id: z.string(),
      app_secret: z.string(),
      encrypt_key: z.string().optional(),
      verification_token: z.string().optional(),
      allowed_users: z.array(z.string()).optional(),
    }).optional(),
    tts: z.object({
      provider: z.string(),
      api_key: z.string().optional(),
      voice: z.string().optional(),
    }).optional(),
    stt: z.object({
      provider: z.string(),
      api_key: z.string().optional(),
      model: z.string().optional(),
    }).optional(),
  }).optional(),
  workflow: z.object({
    cron: z.object({
      enabled: z.boolean().optional(),
      tasks: z.array(z.object({
        id: z.string(),
        expression: z.string(),
        command: z.string(),
        description: z.string().optional(),
      })).optional(),
    }).optional(),
    webhook: z.object({
      enabled: z.boolean().optional(),
      port: z.number().int().positive().optional(),
      secret: z.string().optional(),
    }).optional(),
    git: z.object({
      enabled: z.boolean().optional(),
      github_secret: z.string().optional(),
      gitlab_token: z.string().optional(),
    }).optional(),
  }).optional(),
  codecoder: z.object({
    enabled: z.boolean().optional(),
    endpoint: z.string().optional(),
    timeout_secs: z.number().int().positive().optional(),
  }).optional(),
  observability: z.object({
    log_level: z.enum(["trace", "debug", "info", "warn", "error"]).optional(),
    log_format: z.enum(["json", "pretty"]).optional(),
    tracing: z.boolean().optional(),
  }).optional(),
  memory: z.object({
    backend: z.enum(["sqlite", "postgres"]).optional(),
    path: z.string().optional(),
    connection_string: z.string().optional(),
  }).optional(),
}).passthrough()

export type ValidatedConfig = z.infer<typeof ConfigSchema>

// ============================================================================
// Unified Configuration Manager
// ============================================================================

type ConfigChangeHandler = (newConfig: Config, oldConfig: Config) => void

/**
 * Configuration manager with hot reload support.
 *
 * Features:
 * - Single config source: ~/.codecoder/config.json
 * - Hot reload via file system watching
 * - Environment variable resolution
 * - Schema validation with Zod
 */
export class ConfigManager {
  private config: Config = DEFAULT_CONFIG
  private watcher: FSWatcher | null = null
  private handlers: ConfigChangeHandler[] = []
  private loaded = false

  /**
   * Load configuration from disk.
   * Applies environment variable overrides and validates with Zod.
   */
  async load(): Promise<Config> {
    const path = configPath()

    try {
      const file = Bun.file(path)
      const exists = await file.exists()

      if (exists) {
        const text = await file.text()
        const parsed = JSON.parse(text)
        const withEnvOverrides = this.applyEnvOverrides(parsed)
        const validated = ConfigSchema.safeParse(withEnvOverrides)

        this.config = validated.success
          ? this.mergeWithDefaults(validated.data)
          : this.mergeWithDefaults(withEnvOverrides)
      }
    } catch {
      // Use defaults on error
      this.config = { ...DEFAULT_CONFIG }
    }

    this.loaded = true
    return this.config
  }

  /**
   * Save configuration to disk.
   */
  async save(config: Partial<Config>): Promise<void> {
    const path = configPath()
    const dir = configDir()

    // Ensure directory exists
    const fs = await import("fs/promises")
    await fs.mkdir(dir, { recursive: true })

    // Merge with existing config
    const oldConfig = this.config
    this.config = this.mergeWithDefaults({ ...this.config, ...config })

    // Write to disk
    await Bun.write(path, JSON.stringify(this.config, null, 2))

    // Notify handlers
    this.notifyHandlers(this.config, oldConfig)
  }

  /**
   * Get current configuration.
   * Loads from disk if not yet loaded.
   */
  async get(): Promise<Config> {
    return this.loaded ? this.config : this.load()
  }

  /**
   * Get configuration synchronously.
   * Returns cached config or defaults if not yet loaded.
   */
  getSync(): Config {
    return this.loaded ? this.config : { ...DEFAULT_CONFIG }
  }

  /**
   * Start watching for configuration changes.
   * Calls handlers when config file changes on disk.
   */
  startWatching(): void {
    if (this.watcher) return

    const path = configPath()

    try {
      this.watcher = watch(path, async (eventType) => {
        if (eventType === "change") {
          const oldConfig = this.config
          await this.load()
          this.notifyHandlers(this.config, oldConfig)
        }
      })
    } catch {
      // File might not exist yet, that's OK
    }
  }

  /**
   * Stop watching for configuration changes.
   */
  stopWatching(): void {
    if (this.watcher) {
      this.watcher.close()
      this.watcher = null
    }
  }

  /**
   * Register a handler for configuration changes.
   * Returns a function to unregister the handler.
   */
  onChange(handler: ConfigChangeHandler): () => void {
    this.handlers.push(handler)
    return () => {
      const index = this.handlers.indexOf(handler)
      if (index >= 0) this.handlers.splice(index, 1)
    }
  }

  /**
   * Apply environment variable overrides.
   * Supports patterns like CODECODER_GATEWAY_PORT=8080
   */
  private applyEnvOverrides(config: Config): Config {
    const result = { ...config }

    // Gateway overrides
    if (process.env.CODECODER_GATEWAY_PORT) {
      result.gateway = {
        ...result.gateway,
        port: parseInt(process.env.CODECODER_GATEWAY_PORT, 10),
      }
    }
    if (process.env.CODECODER_GATEWAY_HOST) {
      result.gateway = { ...result.gateway, host: process.env.CODECODER_GATEWAY_HOST }
    }
    if (process.env.CODECODER_JWT_SECRET) {
      result.gateway = { ...result.gateway, jwt_secret: process.env.CODECODER_JWT_SECRET }
    }

    // Channels overrides
    if (process.env.CODECODER_CHANNELS_PORT) {
      result.channels = {
        ...result.channels,
        port: parseInt(process.env.CODECODER_CHANNELS_PORT, 10),
      }
    }
    if (process.env.TELEGRAM_BOT_TOKEN) {
      result.channels = {
        ...result.channels,
        telegram: {
          ...result.channels?.telegram,
          enabled: true,
          bot_token: process.env.TELEGRAM_BOT_TOKEN,
        },
      }
    }
    if (process.env.DISCORD_BOT_TOKEN) {
      result.channels = {
        ...result.channels,
        discord: {
          ...result.channels?.discord,
          enabled: true,
          bot_token: process.env.DISCORD_BOT_TOKEN,
        },
      }
    }

    // CodeCoder overrides
    if (process.env.CODECODER_ENDPOINT) {
      result.codecoder = { ...result.codecoder, endpoint: process.env.CODECODER_ENDPOINT }
    }

    // Observability overrides
    if (process.env.CODECODER_LOG_LEVEL) {
      const level = process.env.CODECODER_LOG_LEVEL.toLowerCase()
      if (["trace", "debug", "info", "warn", "error"].includes(level)) {
        result.observability = {
          ...result.observability,
          log_level: level as "trace" | "debug" | "info" | "warn" | "error",
        }
      }
    }

    return result
  }

  /**
   * Merge config with defaults.
   */
  private mergeWithDefaults(config: Partial<Config>): Config {
    return {
      gateway: { ...DEFAULT_CONFIG.gateway, ...config.gateway },
      channels: { ...DEFAULT_CONFIG.channels, ...config.channels },
      workflow: { ...DEFAULT_CONFIG.workflow, ...config.workflow },
      codecoder: { ...DEFAULT_CONFIG.codecoder, ...config.codecoder },
      observability: { ...DEFAULT_CONFIG.observability, ...config.observability },
      memory: { ...DEFAULT_CONFIG.memory, ...config.memory },
    }
  }

  /**
   * Notify all registered handlers of config change.
   */
  private notifyHandlers(newConfig: Config, oldConfig: Config): void {
    for (const handler of this.handlers) {
      try {
        handler(newConfig, oldConfig)
      } catch {
        // Ignore handler errors
      }
    }
  }
}

/**
 * Global singleton instance of ConfigManager.
 */
export const configManager = new ConfigManager()
