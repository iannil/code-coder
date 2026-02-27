// Auto-generated from JSON Schema - DO NOT EDIT
// Run `bun run script/generate-config.ts` to regenerate

/* eslint-disable */

// ═══════════════════════════════════════════════════
// Config Configuration Types
// ═══════════════════════════════════════════════════

/**
 * Permission configuration for tools
 */
export type PermissionConfig =
  | ("ask" | "allow" | "deny")
  | {
      [k: string]:
        | (
            | ("ask" | "allow" | "deny")
            | {
                [k: string]: ("ask" | "allow" | "deny") | undefined
              }
          )
        | undefined
    }

/**
 * Core configuration for CodeCoder services (unified TypeScript and Rust)
 */
export interface CodeCoderConfiguration {
  /**
   * JSON schema reference for configuration validation
   */
  $schema?: string
  network?: NetworkConfig
  services?: ServicesConfig
  redis?: RedisConfig
  auth?: AuthConfig
  observability?: ObservabilityConfig
  memory?: MemoryConfig
  /**
   * Agent configurations keyed by agent name
   */
  agent?: {
    [k: string]: AgentConfig | undefined
  }
  /**
   * MCP (Model Context Protocol) server configurations
   */
  mcp?: {
    server?: McpServerConfig
    [k: string]: (McpServerConfig | McpLocalConfig | McpRemoteConfig | McpDisabledConfig) | undefined
  }
  keybinds?: KeybindsConfig
  /**
   * Theme name to use for the interface
   */
  theme?: string
  /**
   * Log level for the application
   */
  logLevel?: "debug" | "info" | "warn" | "error"
  /**
   * Default model in provider/model format (e.g., anthropic/claude-sonnet-4-20250514)
   */
  model?: string
  /**
   * Small model for tasks like title generation
   */
  small_model?: string
  /**
   * Default agent to use when none is specified
   */
  default_agent?: string
  /**
   * Custom username to display in conversations
   */
  username?: string
  /**
   * Auto-update behavior: true, false, or 'notify'
   */
  autoupdate?: boolean | "notify"
  /**
   * Disable providers that are loaded automatically
   */
  disabled_providers?: string[]
  /**
   * When set, ONLY these providers will be enabled
   */
  enabled_providers?: string[]
  /**
   * Additional instruction files or patterns to include
   */
  instructions?: string[]
  permission?: PermissionConfig
  compaction?: CompactionConfig
  server?: ServerConfig
  tui?: TuiConfig
  vault?: VaultConfig
  experimental?: ExperimentalConfig
  autonomousMode?: AutonomousModeConfig
  [k: string]: unknown | undefined
}
/**
 * Global network configuration for all services
 */
export interface NetworkConfig {
  /**
   * Bind address for all services
   */
  bind?: string
  /**
   * Public URL for callbacks (optional)
   */
  public_url?: string | null
}
/**
 * Services port configuration
 */
export interface ServicesConfig {
  codecoder?: ServicePortConfig
  gateway?: ServicePortConfig1
  channels?: ServicePortConfig2
  workflow?: ServicePortConfig3
  trading?: ServicePortConfig4
}
/**
 * CodeCoder API service (default port: 4400)
 */
export interface ServicePortConfig {
  /**
   * Port number for the service
   */
  port?: number
}
/**
 * Gateway service (default port: 4430)
 */
export interface ServicePortConfig1 {
  /**
   * Port number for the service
   */
  port?: number
}
/**
 * Channels service (default port: 4431)
 */
export interface ServicePortConfig2 {
  /**
   * Port number for the service
   */
  port?: number
}
/**
 * Workflow service (default port: 4432)
 */
export interface ServicePortConfig3 {
  /**
   * Port number for the service
   */
  port?: number
}
/**
 * Trading service (default port: 4434)
 */
export interface ServicePortConfig4 {
  /**
   * Port number for the service
   */
  port?: number
}
/**
 * Redis configuration for conversation store
 */
export interface RedisConfig {
  /**
   * Redis connection URL
   */
  url?: string
  /**
   * Redis password
   */
  password?: string
  /**
   * Redis database number
   */
  db?: number
  /**
   * Key prefix for all Redis keys
   */
  keyPrefix?: string
  /**
   * Connection timeout in ms
   */
  connectTimeout?: number
  /**
   * Command timeout in ms
   */
  commandTimeout?: number
  /**
   * Max retries per request
   */
  maxRetriesPerRequest?: number
}
/**
 * Authentication configuration
 */
export interface AuthConfig {
  /**
   * Authentication mode
   */
  mode?: "pairing" | "jwt" | "none"
  /**
   * JWT secret for token signing
   */
  jwt_secret?: string
}
/**
 * Observability configuration
 */
export interface ObservabilityConfig {
  /**
   * Log level
   */
  log_level?: "debug" | "info" | "warn" | "error"
  /**
   * Enable observability logging
   */
  enabled?: boolean
  /**
   * Sampling rate for logs (0.0-1.0)
   */
  sampling?: number
}
/**
 * Memory/persistence configuration
 */
export interface MemoryConfig {
  /**
   * Memory storage backend
   */
  backend?: "sqlite" | "markdown" | "none"
  /**
   * Auto-save conversation context
   */
  auto_save?: boolean
}
/**
 * Agent configuration
 */
export interface AgentConfig {
  /**
   * Model to use for this agent
   */
  model?: string
  /**
   * Temperature for sampling
   */
  temperature?: number
  /**
   * Top-p for nucleus sampling
   */
  top_p?: number
  /**
   * System prompt for the agent
   */
  prompt?: string
  /**
   * Description of when to use the agent
   */
  description?: string
  /**
   * Agent mode
   */
  mode?: "subagent" | "primary" | "all"
  /**
   * Hide this agent from autocomplete menu
   */
  hidden?: boolean
  /**
   * Disable this agent
   */
  disable?: boolean
  /**
   * Hex color code for the agent
   */
  color?: string
  /**
   * Maximum agentic iterations
   */
  steps?: number
  permission?: PermissionConfig
  /**
   * Additional agent options
   */
  options?: {
    [k: string]: unknown | undefined
  }
  [k: string]: unknown | undefined
}
/**
 * MCP server configuration for 'mcp serve' command
 */
export interface McpServerConfig {
  /**
   * API key for MCP server authentication
   */
  apiKey?: string
  /**
   * Default port for HTTP transport
   */
  port?: number
  /**
   * Default transport mode
   */
  defaultTransport?: "stdio" | "http"
  /**
   * Glob patterns for additional resources
   */
  resources?: string[]
}
/**
 * Local MCP server configuration
 */
export interface McpLocalConfig {
  type: "local"
  /**
   * Command and arguments to run the MCP server
   */
  command: string[]
  /**
   * Environment variables
   */
  environment?: {
    [k: string]: string | undefined
  }
  /**
   * Enable or disable the MCP server
   */
  enabled?: boolean
  /**
   * Timeout in ms for MCP requests
   */
  timeout?: number
}
/**
 * Remote MCP server configuration
 */
export interface McpRemoteConfig {
  type: "remote"
  /**
   * URL of the remote MCP server
   */
  url: string
  /**
   * Enable or disable the MCP server
   */
  enabled?: boolean
  /**
   * Headers to send with requests
   */
  headers?: {
    [k: string]: string | undefined
  }
  /**
   * OAuth configuration
   */
  oauth?: McpOAuthConfig | false
  /**
   * Timeout in ms for MCP requests
   */
  timeout?: number
}
export interface McpOAuthConfig {
  /**
   * OAuth client ID
   */
  clientId?: string
  /**
   * OAuth client secret
   */
  clientSecret?: string
  /**
   * OAuth scopes to request
   */
  scope?: string
}
export interface McpDisabledConfig {
  enabled: false
}
/**
 * Keyboard shortcut configurations
 */
export interface KeybindsConfig {
  leader?: string
  app_exit?: string
  editor_open?: string
  theme_list?: string
  sidebar_toggle?: string
  session_new?: string
  session_list?: string
  model_list?: string
  agent_list?: string
  command_list?: string
  input_submit?: string
  input_newline?: string
  [k: string]: string | undefined
}
export interface CompactionConfig {
  /**
   * Enable automatic compaction
   */
  auto?: boolean
  /**
   * Enable pruning of old tool outputs
   */
  prune?: boolean
}
/**
 * Server configuration for codecoder serve
 */
export interface ServerConfig {
  /**
   * Port to listen on
   */
  port?: number
  /**
   * Hostname to listen on
   */
  hostname?: string
  /**
   * Enable mDNS service discovery
   */
  mdns?: boolean
  /**
   * Additional domains to allow for CORS
   */
  cors?: string[]
  /**
   * API key for authenticating requests
   */
  apiKey?: string
}
/**
 * TUI specific settings
 */
export interface TuiConfig {
  /**
   * Scroll speed
   */
  scroll_speed?: number
  scroll_acceleration?: {
    enabled?: boolean
  }
  /**
   * Diff rendering style
   */
  diff_style?: "auto" | "stacked"
}
/**
 * Credential vault configuration
 */
export interface VaultConfig {
  /**
   * Enable the credential vault
   */
  enabled?: boolean
  /**
   * Auto-inject credentials into HTTP requests
   */
  autoInject?: boolean
}
/**
 * Experimental features
 */
export interface ExperimentalConfig {
  chatMaxRetries?: number
  disable_paste_summary?: boolean
  batch_tool?: boolean
  openTelemetry?: boolean
  primary_tools?: string[]
  continue_loop_on_deny?: boolean
  mcp_timeout?: number
  observability?: {
    enabled?: boolean
    level?: "debug" | "info" | "warn" | "error"
    sampling?: number
  }
  [k: string]: unknown | undefined
}
/**
 * Autonomous Mode configuration
 */
export interface AutonomousModeConfig {
  enabled?: boolean
  autonomyLevel?: "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"
  unattended?: boolean
  resourceLimits?: {
    maxTokens?: number
    maxCostUSD?: number
    maxDurationMinutes?: number
    maxFilesChanged?: number
    maxActions?: number
  }
}


// ═══════════════════════════════════════════════════
// Secrets Configuration Types
// ═══════════════════════════════════════════════════

/**
 * Credentials and API keys (stored separately for security)
 */
export interface CodeCoderSecrets {
  /**
   * JSON schema reference
   */
  $schema?: string
  llm?: LlmSecretsConfig
  channels?: ChannelSecretsConfig
  external?: ExternalSecretsConfig
}
/**
 * LLM provider API keys
 */
export interface LlmSecretsConfig {
  /**
   * Anthropic API key
   */
  anthropic?: string | null
  /**
   * OpenAI API key
   */
  openai?: string | null
  /**
   * DeepSeek API key
   */
  deepseek?: string | null
  /**
   * Google AI API key
   */
  google?: string | null
  /**
   * OpenRouter API key
   */
  openrouter?: string | null
  /**
   * Groq API key
   */
  groq?: string | null
  /**
   * Mistral API key
   */
  mistral?: string | null
  /**
   * xAI API key
   */
  xai?: string | null
  /**
   * Together AI API key
   */
  together?: string | null
  /**
   * Fireworks AI API key
   */
  fireworks?: string | null
  /**
   * Perplexity API key
   */
  perplexity?: string | null
  /**
   * Cohere API key
   */
  cohere?: string | null
  /**
   * Cloudflare Workers AI API key
   */
  cloudflare?: string | null
  /**
   * Venice AI API key
   */
  venice?: string | null
  /**
   * Moonshot API key
   */
  moonshot?: string | null
  /**
   * GLM (Zhipu) API key
   */
  glm?: string | null
  /**
   * MiniMax API key
   */
  minimax?: string | null
  /**
   * Qianfan API key
   */
  qianfan?: string | null
  /**
   * Additional LLM provider API key
   */
  [k: string]: (string | null) | undefined
}
/**
 * IM channel credentials
 */
export interface ChannelSecretsConfig {
  /**
   * Telegram Bot API token
   */
  telegram_bot_token?: string | null
  /**
   * Discord Bot token
   */
  discord_bot_token?: string | null
  /**
   * Slack Bot token
   */
  slack_bot_token?: string | null
  /**
   * Slack App token for Socket Mode
   */
  slack_app_token?: string | null
  /**
   * Feishu App ID
   */
  feishu_app_id?: string | null
  /**
   * Feishu App Secret
   */
  feishu_app_secret?: string | null
  [k: string]: (string | null) | undefined
}
/**
 * External service credentials
 */
export interface ExternalSecretsConfig {
  /**
   * Lixin (理杏仁) API token for A-share market data
   */
  lixin?: string | null
  /**
   * iTick API key for A-share market data
   */
  itick?: string | null
  /**
   * Cloudflare Tunnel token
   */
  cloudflare_tunnel?: string | null
  /**
   * ngrok authentication token
   */
  ngrok_auth?: string | null
  /**
   * ElevenLabs API key for TTS
   */
  elevenlabs?: string | null
  [k: string]: (string | null) | undefined
}


// ═══════════════════════════════════════════════════
// Trading Configuration Types
// ═══════════════════════════════════════════════════

/**
 * Trading module configuration for zero-trading service
 */
export interface CodeCoderTradingConfiguration {
  /**
   * JSON schema reference
   */
  $schema?: string
  /**
   * Enable the trading module
   */
  enabled?: boolean
  /**
   * Enable paper trading (simulation mode)
   */
  paper_trading?: boolean
  /**
   * Enable automatic execution
   */
  auto_execute?: boolean
  /**
   * Trading service HTTP port
   */
  port?: number
  /**
   * Trading service HTTP host
   */
  host?: string
  /**
   * Tracked symbols for market data updates (e.g., ['000001.SH', '000300.SH'])
   */
  tracked_symbols?: string[]
  /**
   * Timeframes for multi-timeframe analysis (e.g., ['D', 'H4', 'H1'])
   */
  timeframes?: string[]
  telegram_notification?: TradingNotificationConfig
  schedule?: TradingScheduleConfig
  data_sources?: DataSourcesConfig
  local_storage?: LocalStorageConfig
  screener?: ScreenerConfig
  macro_agent?: MacroAgentConfig
  loop_config?: TradingLoopConfig
  preparation_tasks?: PreparationTaskConfig
  /**
   * SMT pairs for divergence detection
   */
  smt_pairs?: SmtPairConfig[]
  /**
   * Maximum number of open positions
   */
  max_positions?: number
  /**
   * Maximum capital per position (percentage)
   */
  max_position_pct?: number
  /**
   * Maximum daily capital deployment (percentage)
   */
  max_daily_capital_pct?: number
  /**
   * Default stop loss percentage
   */
  default_stop_loss_pct?: number
  /**
   * Minimum bars for accumulation phase detection
   */
  min_accumulation_bars?: number
  /**
   * Manipulation threshold (ATR multiple)
   */
  manipulation_threshold?: number
  /**
   * Require multi-timeframe alignment
   */
  require_alignment?: boolean
  /**
   * Signal expiry in minutes
   */
  signal_expiry_minutes?: number
  /**
   * Enable macro economic filter
   */
  macro_filter_enabled?: boolean
  /**
   * Macro data cache duration in seconds
   */
  macro_cache_secs?: number
  [k: string]: unknown | undefined
}
/**
 * Telegram notification settings
 */
export interface TradingNotificationConfig {
  /**
   * Enable notifications
   */
  enabled?: boolean
  /**
   * Telegram chat ID for notifications
   */
  chat_id?: string
  /**
   * Notify on new signals
   */
  notify_signals?: boolean
  /**
   * Notify on trade execution
   */
  notify_trades?: boolean
  /**
   * Notify on errors
   */
  notify_errors?: boolean
  /**
   * Notify on macro alerts
   */
  notify_macro_alerts?: boolean
}
/**
 * Session schedule configuration
 */
export interface TradingScheduleConfig {
  /**
   * Enable scheduled sessions
   */
  enabled?: boolean
  /**
   * Cron expression for session start (e.g., '0 25 9 * * 1-5')
   */
  session_start?: string
  /**
   * Cron expression for session end (optional)
   */
  session_end?: string
  /**
   * Timezone for schedule
   */
  timezone?: string
}
/**
 * Multi-data-source configuration
 */
export interface DataSourcesConfig {
  /**
   * List of data source configurations
   */
  sources?: DataSourceEntry[]
  /**
   * Health check interval in seconds
   */
  health_check_interval_secs?: number
  /**
   * Consecutive failures before marking unhealthy
   */
  unhealthy_threshold?: number
  /**
   * Maximum retries per provider
   */
  max_retries?: number
  /**
   * Health check timeout in seconds
   */
  health_check_timeout_secs?: number
}
/**
 * Data source entry configuration
 */
export interface DataSourceEntry {
  /**
   * Data provider name
   */
  provider: "itick" | "lixin" | "ashare"
  /**
   * Priority (lower = higher priority)
   */
  priority?: number
  /**
   * Enable this data source
   */
  enabled?: boolean
  /**
   * Supported capabilities
   */
  capabilities?: ("realtime" | "historical" | "fundamentals" | "financial")[]
}
/**
 * Local storage configuration for persistent data
 */
export interface LocalStorageConfig {
  /**
   * Enable local storage
   */
  enabled?: boolean
  /**
   * Storage directory path
   */
  directory?: string
  /**
   * Symbols to store locally
   */
  symbols?: string[]
  /**
   * Data retention period in days
   */
  retention_days?: number
}
/**
 * Full market screener configuration
 */
export interface ScreenerConfig {
  /**
   * Enable screener
   */
  enabled?: boolean
  /**
   * Stock universe to screen
   */
  universe?: string[]
  filters?: ScreenerFilters
  /**
   * Refresh interval in minutes
   */
  refresh_interval_mins?: number
}
/**
 * Screener filter configuration
 */
export interface ScreenerFilters {
  valuation?: {
    pe_max?: number
    pb_max?: number
    ps_max?: number
  }
  momentum?: {
    rsi_min?: number
    rsi_max?: number
  }
  volume?: {
    min_avg_volume?: number
    volume_ratio_min?: number
  }
  price?: {
    min_price?: number
    max_price?: number
  }
  market_cap?: {
    min?: number
    max?: number
  }
  [k: string]: unknown | undefined
}
/**
 * Macro agent configuration for intelligent analysis
 */
export interface MacroAgentConfig {
  /**
   * Enable macro agent
   */
  enabled?: boolean
  /**
   * LLM model for macro analysis
   */
  model?: string
  /**
   * Temperature for LLM
   */
  temperature?: number
  /**
   * Cache TTL in seconds
   */
  cache_ttl_secs?: number
}
/**
 * Trading loop configuration
 */
export interface TradingLoopConfig {
  /**
   * Enable trading loop
   */
  enabled?: boolean
  /**
   * Loop interval in seconds
   */
  interval_secs?: number
  /**
   * Signal cooldown in seconds
   */
  signal_cooldown_secs?: number
}
/**
 * Preparation task configuration (24/7 operation)
 */
export interface PreparationTaskConfig {
  /**
   * Enable preparation tasks
   */
  enabled?: boolean
  /**
   * Preload market data
   */
  preload_data?: boolean
  /**
   * Precompute technical indicators
   */
  precompute_indicators?: boolean
  /**
   * Cron expression for preparation tasks
   */
  schedule?: string
}
/**
 * SMT pair for divergence detection
 */
export interface SmtPairConfig {
  /**
   * First symbol in pair
   */
  symbol_a: string
  /**
   * Second symbol in pair
   */
  symbol_b: string
  /**
   * Minimum correlation threshold
   */
  correlation_threshold?: number
}


// ═══════════════════════════════════════════════════
// Channels Configuration Types
// ═══════════════════════════════════════════════════

/**
 * IM channels configuration for zero-channels service
 */
export interface CodeCoderChannelsConfiguration {
  /**
   * JSON schema reference
   */
  $schema?: string
  /**
   * Enable CLI channel
   */
  cli?: boolean
  telegram?: TelegramConfig
  discord?: DiscordConfig
  slack?: SlackConfig
  whatsapp?: WhatsAppConfig
  feishu?: FeishuConfig
}
/**
 * Telegram channel configuration
 */
export interface TelegramConfig {
  /**
   * Enable Telegram channel
   */
  enabled?: boolean
  /**
   * Allowed Telegram usernames
   */
  allowed_users?: string[]
  voice?: TelegramVoiceConfig
}
/**
 * Voice message configuration
 */
export interface TelegramVoiceConfig {
  /**
   * Enable voice message support
   */
  enabled?: boolean
  /**
   * Speech-to-text provider
   */
  stt_provider?: "openai" | "uniapi" | "groq" | "deepinfra" | "compatible"
  /**
   * Speech-to-text model
   */
  stt_model?: string
  /**
   * Base URL for OpenAI-compatible STT providers
   */
  stt_base_url?: string
}
/**
 * Discord channel configuration
 */
export interface DiscordConfig {
  /**
   * Enable Discord channel
   */
  enabled?: boolean
  /**
   * Discord guild/server ID
   */
  guild_id?: string
  /**
   * Allowed Discord user IDs
   */
  allowed_users?: string[]
}
/**
 * Slack channel configuration
 */
export interface SlackConfig {
  /**
   * Enable Slack channel
   */
  enabled?: boolean
  /**
   * Default Slack channel ID
   */
  channel_id?: string
}
/**
 * WhatsApp channel configuration
 */
export interface WhatsAppConfig {
  /**
   * Enable WhatsApp channel
   */
  enabled?: boolean
  /**
   * WhatsApp phone number ID
   */
  phone_number_id?: string
  /**
   * Webhook verification token
   */
  verify_token?: string
  /**
   * Allowed phone numbers (E.164 format)
   */
  allowed_numbers?: string[]
}
/**
 * Feishu/Lark channel configuration
 */
export interface FeishuConfig {
  /**
   * Enable Feishu channel
   */
  enabled?: boolean
  /**
   * Encrypt key for event callback decryption
   */
  encrypt_key?: string
  /**
   * Verification token for event callback verification
   */
  verification_token?: string
  /**
   * Allowed user open_ids or '*' for all
   */
  allowed_users?: string[]
  /**
   * Use Lark API instead of Feishu API (for international users)
   */
  use_lark_api?: boolean
}


// ═══════════════════════════════════════════════════
// Providers Configuration Types
// ═══════════════════════════════════════════════════

/**
 * LLM provider configuration for CodeCoder services
 */
export interface CodeCoderLLMProvidersConfiguration {
  /**
   * JSON schema reference
   */
  $schema?: string
  _settings?: ProviderSettings
  [k: string]: (string | ProviderSettings1 | ProviderConfig) | undefined
}
/**
 * Global LLM settings
 */
export interface ProviderSettings {
  /**
   * Default model in provider/model format (e.g., deepseek/deepseek-chat)
   */
  default?: string
  /**
   * Number of retries for failed requests
   */
  retries?: number
  /**
   * Base backoff in milliseconds
   */
  backoff_ms?: number
  /**
   * Fallback provider chain
   */
  fallbacks?: string[]
}
/**
 * Global LLM settings stored in provider._settings
 */
export interface ProviderSettings1 {
  /**
   * Default model in provider/model format (e.g., deepseek/deepseek-chat)
   */
  default?: string
  /**
   * Number of retries for failed requests
   */
  retries?: number
  /**
   * Base backoff in milliseconds
   */
  backoff_ms?: number
  /**
   * Fallback provider chain
   */
  fallbacks?: string[]
}
/**
 * Individual LLM provider configuration
 */
export interface ProviderConfig {
  /**
   * Provider ID
   */
  id?: string
  /**
   * Provider display name
   */
  name?: string
  /**
   * Provider API base URL
   */
  api?: string
  /**
   * NPM package for the provider SDK
   */
  npm?: string
  /**
   * Only enable these models
   */
  whitelist?: string[]
  /**
   * Disable these models
   */
  blacklist?: string[]
  /**
   * Model definitions
   */
  models?: {
    [k: string]: ModelConfig | undefined
  }
  options?: ProviderOptions
  [k: string]: unknown | undefined
}
/**
 * Model configuration within a provider
 */
export interface ModelConfig {
  /**
   * Model ID
   */
  id?: string
  /**
   * Model display name
   */
  name?: string
  /**
   * Whether this model supports tool calling
   */
  tool_call?: boolean
  limit?: ModelLimits
  /**
   * Model variant configurations
   */
  variants?: {
    [k: string]: VariantConfig | undefined
  }
  [k: string]: unknown | undefined
}
/**
 * Token limits for a model
 */
export interface ModelLimits {
  /**
   * Context window size (input tokens)
   */
  context?: number
  /**
   * Maximum output tokens
   */
  output?: number
}
/**
 * Model variant configuration
 */
export interface VariantConfig {
  /**
   * Disable this variant
   */
  disabled?: boolean
  [k: string]: unknown | undefined
}
/**
 * Provider options including credentials
 */
export interface ProviderOptions {
  /**
   * API key for this provider
   */
  apiKey?: string
  /**
   * Base URL override
   */
  baseURL?: string
  /**
   * GitHub Enterprise URL for copilot authentication
   */
  enterpriseUrl?: string
  /**
   * Enable promptCacheKey for this provider
   */
  setCacheKey?: boolean
  /**
   * Timeout for requests
   */
  timeout?: number | false
  [k: string]: unknown | undefined
}

