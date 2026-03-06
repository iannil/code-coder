/**
 * Centralized Timeout Configuration
 *
 * All timeout values are defined here with sensible defaults.
 * Each timeout can be overridden via environment variables.
 *
 * Naming convention: CCODE_TIMEOUT_<CATEGORY>_<OPERATION>
 */

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Parse timeout from environment variable with fallback to default.
 * Returns value in milliseconds.
 */
function parseEnvTimeout(envVar: string, defaultMs: number): number {
  const value = process.env[envVar]
  if (!value) return defaultMs
  const parsed = parseInt(value, 10)
  return isNaN(parsed) || parsed <= 0 ? defaultMs : parsed
}

// ============================================================================
// Tool Timeouts
// ============================================================================

/** Default timeout for Bash commands (2 minutes) */
export const BASH_DEFAULT_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_BASH_DEFAULT", 2 * 60 * 1000)

/** Maximum timeout for Bash commands (10 minutes) */
export const BASH_MAX_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_BASH_MAX", 10 * 60 * 1000)

/** WebFetch default timeout (30 seconds) */
export const WEBFETCH_DEFAULT_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_WEBFETCH_DEFAULT", 30 * 1000)

/** WebFetch maximum timeout (2 minutes) */
export const WEBFETCH_MAX_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_WEBFETCH_MAX", 2 * 60 * 1000)

/** WebSearch API timeout (25 seconds) */
export const WEBSEARCH_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_WEBSEARCH", 25 * 1000)

/** CodeSearch timeout (30 seconds) */
export const CODESEARCH_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_CODESEARCH", 30 * 1000)

/** Scheduler API request timeout (10 seconds) */
export const SCHEDULER_REQUEST_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_SCHEDULER_REQUEST", 10 * 1000)

/** Network analyzer timeout (30 seconds) */
export const NETWORK_ANALYZER_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_NETWORK_ANALYZER", 30 * 1000)

// ============================================================================
// Reach Tool Timeouts (External Platform APIs)
// ============================================================================

/** Default timeout for media operations (60 seconds) */
export const REACH_MEDIA_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_REACH_MEDIA", 60 * 1000)

/** Timeout for fetching posts/videos (30 seconds) */
export const REACH_FETCH_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_REACH_FETCH", 30 * 1000)

/** Timeout for downloading media (60 seconds) */
export const REACH_DOWNLOAD_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_REACH_DOWNLOAD", 60 * 1000)

/** Timeout for which command check (5 seconds) */
export const REACH_WHICH_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_REACH_WHICH", 5 * 1000)

// ============================================================================
// API Server Timeouts
// ============================================================================

/** Task execution timeout (5 minutes) */
export const API_TASK_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_API_TASK", 5 * 60 * 1000)

/** Channel health check timeout (2 seconds) */
export const API_CHANNEL_HEALTH_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_API_CHANNEL_HEALTH", 2 * 1000)

// ============================================================================
// Autonomous Mode Timeouts
// ============================================================================

/** Default timeout for Hands Bridge operations (30 seconds) */
export const HANDS_BRIDGE_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_HANDS_BRIDGE", 30 * 1000)

/** Health check timeout for autonomous services (5 seconds) */
export const AUTONOMOUS_HEALTH_CHECK_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_AUTONOMOUS_HEALTH", 5 * 1000)

/** Default event wait timeout (30 seconds) */
export const AUTONOMOUS_EVENT_WAIT_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_AUTONOMOUS_EVENT_WAIT", 30 * 1000)

/** Phase runner default timeout (5 minutes) */
export const PHASE_RUNNER_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_PHASE_RUNNER", 5 * 60 * 1000)

/** LLM solver timeout (60 seconds) */
export const LLM_SOLVER_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_LLM_SOLVER", 60 * 1000)

/** Docker sandbox timeout (30 seconds) */
export const DOCKER_SANDBOX_TIMEOUT_SECS = parseEnvTimeout("CCODE_TIMEOUT_DOCKER_SANDBOX_SECS", 30)

// ============================================================================
// HITL (Human-in-the-Loop) Timeouts
// ============================================================================

/** HITL client default timeout (30 seconds) */
export const HITL_CLIENT_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_HITL_CLIENT", 30 * 1000)

/** HITL health check timeout (5 seconds) */
export const HITL_HEALTH_CHECK_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_HITL_HEALTH", 5 * 1000)

// ============================================================================
// IPC / Socket Timeouts
// ============================================================================

/** Default IPC request timeout (30 seconds) */
export const IPC_REQUEST_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_IPC_REQUEST", 30 * 1000)

/** Socket ready check timeout (10 seconds) */
export const IPC_SOCKET_READY_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_IPC_SOCKET_READY", 10 * 1000)

/** Socket ready check default (1 second) */
export const SOCKET_READY_DEFAULT_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_SOCKET_READY", 1 * 1000)

// ============================================================================
// MCP (Model Context Protocol) Timeouts
// ============================================================================

/** MCP request default timeout (30 seconds) */
export const MCP_DEFAULT_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_MCP_DEFAULT", 30 * 1000)

/** MCP OAuth callback timeout (5 minutes) */
export const MCP_OAUTH_CALLBACK_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_MCP_OAUTH_CALLBACK", 5 * 60 * 1000)

// ============================================================================
// Memory / Knowledge Timeouts
// ============================================================================

/** LLM abstractor timeout for knowledge operations (30 seconds) */
export const MEMORY_LLM_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_MEMORY_LLM", 30 * 1000)

// ============================================================================
// File Watcher Timeouts
// ============================================================================

/** File watcher subscribe timeout (10 seconds) */
export const FILE_WATCHER_SUBSCRIBE_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_FILE_WATCHER_SUBSCRIBE", 10 * 1000)

// ============================================================================
// Writer Agent Timeouts
// ============================================================================

/** Writer timeout warning threshold (45 seconds) */
export const WRITER_TIMEOUT_WARNING_MS = parseEnvTimeout("CCODE_TIMEOUT_WRITER_WARNING", 45 * 1000)

/** Writer timeout critical threshold (90 seconds) */
export const WRITER_TIMEOUT_CRITICAL_MS = parseEnvTimeout("CCODE_TIMEOUT_WRITER_CRITICAL", 90 * 1000)

// ============================================================================
// Verifier Timeouts
// ============================================================================

/** Property checker timeout (5 seconds) */
export const VERIFIER_CHECKER_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_VERIFIER_CHECKER", 5 * 1000)

// ============================================================================
// Shell Timeouts
// ============================================================================

/** SIGKILL timeout after SIGTERM (200ms) */
export const SHELL_SIGKILL_TIMEOUT_MS = parseEnvTimeout("CCODE_TIMEOUT_SHELL_SIGKILL", 200)
