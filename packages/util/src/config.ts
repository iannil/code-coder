/**
 * TypeScript type definitions for Zero ecosystem configuration.
 *
 * These types mirror the Rust config structures in `services/zero-common/src/config.rs`.
 * Keep these in sync when modifying the Rust configuration.
 */

import z from "zod"
import { watch, type FSWatcher } from "fs"

// ============================================================================
// Network Configuration
// ============================================================================

/**
 * Global network configuration.
 * Controls the bind address for all services.
 */
export interface NetworkConfig {
  /** Bind address for all services. Default: "127.0.0.1" (local only) */
  bind?: string
  /** Public URL for callbacks (optional) */
  public_url?: string
}

// ============================================================================
// Services Port Configuration
// ============================================================================

/**
 * Simplified service port configuration.
 */
export interface ServicesConfig {
  codecoder?: { port?: number }
  gateway?: { port?: number }
  channels?: { port?: number }
  workflow?: { port?: number }
  trading?: { port?: number }
}

// ============================================================================
// Authentication Configuration
// ============================================================================

/**
 * Authentication configuration.
 */
export interface AuthConfig {
  /** Authentication mode: "pairing" | "jwt" | "none" */
  mode?: string
  /** JWT secret (auto-generated if not set) */
  jwt_secret?: string
  /** Token expiry in seconds */
  token_expiry_secs?: number
}

// ============================================================================
// Secrets Configuration
// ============================================================================

/**
 * Grouped secrets configuration.
 */
export interface SecretsConfig {
  /** LLM provider API keys */
  llm?: LlmSecretsConfig
  /** IM channel credentials */
  channels?: ChannelSecretsConfig
  /** External service credentials */
  external?: ExternalSecretsConfig
}

export interface LlmSecretsConfig {
  anthropic?: string
  openai?: string
  deepseek?: string
  google?: string
  openrouter?: string
  groq?: string
  mistral?: string
  xai?: string
  together?: string
  fireworks?: string
  perplexity?: string
  cohere?: string
  cloudflare?: string
  venice?: string
  moonshot?: string
  glm?: string
  minimax?: string
  qianfan?: string
}

export interface ChannelSecretsConfig {
  telegram_bot_token?: string
  discord_bot_token?: string
  slack_bot_token?: string
  slack_app_token?: string
  feishu_app_id?: string
  feishu_app_secret?: string
}

export interface ExternalSecretsConfig {
  tushare?: string
  lixin?: string
  cloudflare_tunnel?: string
  ngrok_auth?: string
  elevenlabs?: string
}

// ============================================================================
// LLM Configuration
// ============================================================================

/**
 * Simplified LLM configuration.
 */
export interface LlmConfigNew {
  /** Default model in provider/model format */
  default?: string
  /** Custom provider configurations */
  providers?: Record<string, { base_url?: string; models?: string[] }>
}

// ============================================================================
// Voice Configuration
// ============================================================================

/**
 * Voice configuration (TTS/STT).
 */
export interface VoiceConfigNew {
  tts?: { provider?: string; voice?: string }
  stt?: { provider?: string; model?: string }
}

// ============================================================================
// Tunnel Configuration
// ============================================================================

/**
 * Tunnel configuration for external access.
 */
export interface TunnelConfigNew {
  /** Tunnel provider: "none" | "cloudflare" | "tailscale" | "ngrok" */
  provider?: string
  cloudflare_token?: string
  ngrok_auth_token?: string
}

// ============================================================================
// Channel Enable Configuration (simplified from legacy)
// ============================================================================

/**
 * Simplified channel enable/disable configuration.
 * Used for channels.telegram.enabled etc.
 */
export interface ChannelEnableConfig {
  enabled?: boolean
  allowed_users?: string[]
}

// ============================================================================
// Root Configuration
// ============================================================================

/**
 * Root configuration structure for all Zero services.
 * Stored at `~/.codecoder/config.json`.
 */
export interface Config {
  /** Global network configuration */
  network?: NetworkConfig
  /** Simplified service port configuration */
  services?: ServicesConfig
  /** Authentication configuration */
  auth?: AuthConfig
  /** Grouped secrets */
  secrets?: SecretsConfig
  /** Simplified LLM configuration */
  llm?: LlmConfigNew
  /** Voice configuration */
  voice?: VoiceConfigNew
  /** Tunnel configuration */
  tunnel?: TunnelConfigNew
  /** Channel enable configuration (telegram, discord, etc.) */
  channels?: {
    telegram?: ChannelEnableConfig
    discord?: ChannelEnableConfig
    slack?: ChannelEnableConfig
    feishu?: ChannelEnableConfig
  }
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
  network: {
    bind: "127.0.0.1",
  },
  services: {
    codecoder: { port: 4400 },
    gateway: { port: 4430 },
    channels: { port: 4431 },
    workflow: { port: 4432 },
    trading: { port: 4434 },
  },
  auth: {
    mode: "pairing",
    token_expiry_secs: 86400,
  },
  voice: {
    tts: { provider: "compatible", voice: "nova" },
    stt: { provider: "local", model: "base" },
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

const NetworkSchema = z.object({
  bind: z.string().optional(),
  public_url: z.string().optional(),
}).optional()

const ServicePortSchema = z.object({
  port: z.number().int().positive().optional(),
}).optional()

const ServicesSchema = z.object({
  codecoder: ServicePortSchema,
  gateway: ServicePortSchema,
  channels: ServicePortSchema,
  workflow: ServicePortSchema,
  trading: ServicePortSchema,
}).optional()

const AuthSchema = z.object({
  mode: z.enum(["pairing", "jwt", "none"]).optional(),
  jwt_secret: z.string().optional(),
  token_expiry_secs: z.number().int().positive().optional(),
}).optional()

const LlmSecretsSchema = z.object({
  anthropic: z.string().optional(),
  openai: z.string().optional(),
  deepseek: z.string().optional(),
  google: z.string().optional(),
  openrouter: z.string().optional(),
  groq: z.string().optional(),
  mistral: z.string().optional(),
  xai: z.string().optional(),
  together: z.string().optional(),
  fireworks: z.string().optional(),
  perplexity: z.string().optional(),
  cohere: z.string().optional(),
  cloudflare: z.string().optional(),
  venice: z.string().optional(),
  moonshot: z.string().optional(),
  glm: z.string().optional(),
  minimax: z.string().optional(),
  qianfan: z.string().optional(),
}).optional()

const ChannelSecretsSchema = z.object({
  telegram_bot_token: z.string().optional(),
  discord_bot_token: z.string().optional(),
  slack_bot_token: z.string().optional(),
  slack_app_token: z.string().optional(),
  feishu_app_id: z.string().optional(),
  feishu_app_secret: z.string().optional(),
}).optional()

const ExternalSecretsSchema = z.object({
  tushare: z.string().optional(),
  lixin: z.string().optional(),
  cloudflare_tunnel: z.string().optional(),
  ngrok_auth: z.string().optional(),
  elevenlabs: z.string().optional(),
}).optional()

const SecretsSchema = z.object({
  llm: LlmSecretsSchema,
  channels: ChannelSecretsSchema,
  external: ExternalSecretsSchema,
}).optional()

const LlmSchema = z.object({
  default: z.string().optional(),
  providers: z.record(z.string(), z.object({
    base_url: z.string().optional(),
    models: z.array(z.string()).optional(),
  })).optional(),
}).optional()

const VoiceSchema = z.object({
  tts: z.object({
    provider: z.string().optional(),
    voice: z.string().optional(),
  }).optional(),
  stt: z.object({
    provider: z.string().optional(),
    model: z.string().optional(),
  }).optional(),
}).optional()

const TunnelSchema = z.object({
  provider: z.enum(["none", "cloudflare", "tailscale", "ngrok"]).optional(),
  cloudflare_token: z.string().optional(),
  ngrok_auth_token: z.string().optional(),
}).optional()

const ChannelEnableSchema = z.object({
  enabled: z.boolean().optional(),
  allowed_users: z.array(z.string()).optional(),
}).optional()

/**
 * Zod schema for the root configuration.
 * Provides runtime validation and TypeScript type inference.
 */
export const ConfigSchema = z.object({
  network: NetworkSchema,
  services: ServicesSchema,
  auth: AuthSchema,
  secrets: SecretsSchema,
  llm: LlmSchema,
  voice: VoiceSchema,
  tunnel: TunnelSchema,
  channels: z.object({
    telegram: ChannelEnableSchema,
    discord: ChannelEnableSchema,
    slack: ChannelEnableSchema,
    feishu: ChannelEnableSchema,
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

  // ===========================================================================
  // Endpoint helper methods
  // ===========================================================================

  /**
   * Get the effective bind address.
   */
  getBindAddress(): string {
    return this.config.network?.bind ?? "127.0.0.1"
  }

  /**
   * Get the effective port for CodeCoder service.
   */
  getCodeCoderPort(): number {
    return this.config.services?.codecoder?.port ?? 4400
  }

  /**
   * Get the effective port for Gateway service.
   */
  getGatewayPort(): number {
    return this.config.services?.gateway?.port ?? 4430
  }

  /**
   * Get the effective port for Channels service.
   */
  getChannelsPort(): number {
    return this.config.services?.channels?.port ?? 4431
  }

  /**
   * Get the effective port for Workflow service.
   */
  getWorkflowPort(): number {
    return this.config.services?.workflow?.port ?? 4432
  }

  /**
   * Get the effective port for Trading service.
   */
  getTradingPort(): number {
    return this.config.services?.trading?.port ?? 4434
  }

  /**
   * Get the CodeCoder service endpoint URL.
   */
  getCodeCoderEndpoint(): string {
    return `http://${this.getBindAddress()}:${this.getCodeCoderPort()}`
  }

  /**
   * Get the Gateway service endpoint URL.
   */
  getGatewayEndpoint(): string {
    return `http://${this.getBindAddress()}:${this.getGatewayPort()}`
  }

  /**
   * Get the Channels service endpoint URL.
   */
  getChannelsEndpoint(): string {
    return `http://${this.getBindAddress()}:${this.getChannelsPort()}`
  }

  /**
   * Get the Workflow service endpoint URL.
   */
  getWorkflowEndpoint(): string {
    return `http://${this.getBindAddress()}:${this.getWorkflowPort()}`
  }

  /**
   * Get the Trading service endpoint URL.
   */
  getTradingEndpoint(): string {
    return `http://${this.getBindAddress()}:${this.getTradingPort()}`
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

    // Ensure nested objects exist
    if (!result.services) result.services = {}
    if (!result.network) result.network = {}
    if (!result.auth) result.auth = {}
    if (!result.secrets) result.secrets = {}
    if (!result.secrets.channels) result.secrets.channels = {}
    if (!result.channels) result.channels = {}

    // Network bind override
    if (process.env.CODECODER_GATEWAY_HOST) {
      result.network = { ...result.network, bind: process.env.CODECODER_GATEWAY_HOST }
    }

    // Gateway port override
    if (process.env.CODECODER_GATEWAY_PORT) {
      result.services = {
        ...result.services,
        gateway: {
          ...result.services.gateway,
          port: parseInt(process.env.CODECODER_GATEWAY_PORT, 10),
        },
      }
    }

    // JWT secret override
    if (process.env.CODECODER_JWT_SECRET) {
      result.auth = { ...result.auth, jwt_secret: process.env.CODECODER_JWT_SECRET }
    }

    // Channels port override
    if (process.env.CODECODER_CHANNELS_PORT) {
      result.services = {
        ...result.services,
        channels: {
          ...result.services.channels,
          port: parseInt(process.env.CODECODER_CHANNELS_PORT, 10),
        },
      }
    }

    // Telegram bot token override
    if (process.env.TELEGRAM_BOT_TOKEN) {
      result.secrets = {
        ...result.secrets,
        channels: {
          ...result.secrets.channels,
          telegram_bot_token: process.env.TELEGRAM_BOT_TOKEN,
        },
      }
      result.channels = {
        ...result.channels,
        telegram: { ...result.channels.telegram, enabled: true },
      }
    }

    // Discord bot token override
    if (process.env.DISCORD_BOT_TOKEN) {
      result.secrets = {
        ...result.secrets,
        channels: {
          ...result.secrets.channels,
          discord_bot_token: process.env.DISCORD_BOT_TOKEN,
        },
      }
      result.channels = {
        ...result.channels,
        discord: { ...result.channels.discord, enabled: true },
      }
    }

    // CodeCoder endpoint override (parse URL to extract port)
    if (process.env.CODECODER_ENDPOINT) {
      try {
        const url = new URL(process.env.CODECODER_ENDPOINT)
        if (url.hostname) {
          result.network = { ...result.network, bind: url.hostname }
        }
        if (url.port) {
          result.services = {
            ...result.services,
            codecoder: {
              ...result.services.codecoder,
              port: parseInt(url.port, 10),
            },
          }
        }
      } catch {
        // Ignore invalid URL
      }
    }

    return result
  }

  /**
   * Merge config with defaults.
   */
  private mergeWithDefaults(config: Partial<Config>): Config {
    return {
      network: { ...DEFAULT_CONFIG.network, ...config.network },
      services: {
        codecoder: { ...DEFAULT_CONFIG.services?.codecoder, ...config.services?.codecoder },
        gateway: { ...DEFAULT_CONFIG.services?.gateway, ...config.services?.gateway },
        channels: { ...DEFAULT_CONFIG.services?.channels, ...config.services?.channels },
        workflow: { ...DEFAULT_CONFIG.services?.workflow, ...config.services?.workflow },
        trading: { ...DEFAULT_CONFIG.services?.trading, ...config.services?.trading },
      },
      auth: { ...DEFAULT_CONFIG.auth, ...config.auth },
      secrets: config.secrets,
      llm: config.llm,
      voice: { ...DEFAULT_CONFIG.voice, ...config.voice },
      tunnel: config.tunnel,
      channels: config.channels,
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
