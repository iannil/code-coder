function truthy(key: string) {
  const value = process.env[key]?.toLowerCase()
  return value === "true" || value === "1"
}

export namespace Flag {
  export const CCODE_AUTO_SHARE = truthy("CCODE_AUTO_SHARE")
  export const CCODE_GIT_BASH_PATH = process.env["CCODE_GIT_BASH_PATH"]
  export const CCODE_CONFIG = process.env["CCODE_CONFIG"]
  export declare const CCODE_CONFIG_DIR: string | undefined
  export const CCODE_CONFIG_CONTENT = process.env["CCODE_CONFIG_CONTENT"]
  export const CCODE_DISABLE_AUTOUPDATE = truthy("CCODE_DISABLE_AUTOUPDATE")
  export const CCODE_DISABLE_PRUNE = truthy("CCODE_DISABLE_PRUNE")
  export const CCODE_DISABLE_TERMINAL_TITLE = truthy("CCODE_DISABLE_TERMINAL_TITLE")
  export const CCODE_PERMISSION = process.env["CCODE_PERMISSION"]
  export const CCODE_DISABLE_DEFAULT_PLUGINS = truthy("CCODE_DISABLE_DEFAULT_PLUGINS")
  export const CCODE_DISABLE_LSP_DOWNLOAD = truthy("CCODE_DISABLE_LSP_DOWNLOAD")
  export const CCODE_ENABLE_EXPERIMENTAL_MODELS = truthy("CCODE_ENABLE_EXPERIMENTAL_MODELS")
  export const CCODE_DISABLE_AUTOCOMPACT = truthy("CCODE_DISABLE_AUTOCOMPACT")
  export const CCODE_DISABLE_MODELS_FETCH = truthy("CCODE_DISABLE_MODELS_FETCH")
  export const CCODE_DISABLE_CLAUDE_CODE = truthy("CCODE_DISABLE_CLAUDE_CODE")
  export const CCODE_DISABLE_CLAUDE_CODE_PROMPT =
    CCODE_DISABLE_CLAUDE_CODE || truthy("CCODE_DISABLE_CLAUDE_CODE_PROMPT")
  export const CCODE_DISABLE_CLAUDE_CODE_SKILLS =
    CCODE_DISABLE_CLAUDE_CODE || truthy("CCODE_DISABLE_CLAUDE_CODE_SKILLS")
  export declare const CCODE_DISABLE_PROJECT_CONFIG: boolean
  export const CCODE_FAKE_VCS = process.env["CCODE_FAKE_VCS"]
  export const CCODE_CLIENT = process.env["CCODE_CLIENT"] ?? "cli"
  export const CCODE_SERVER_PASSWORD = process.env["CCODE_SERVER_PASSWORD"]
  export const CCODE_SERVER_USERNAME = process.env["CCODE_SERVER_USERNAME"]

  // Experimental
  export const CCODE_EXPERIMENTAL = truthy("CCODE_EXPERIMENTAL")
  export const CCODE_EXPERIMENTAL_FILEWATCHER = truthy("CCODE_EXPERIMENTAL_FILEWATCHER")
  export const CCODE_EXPERIMENTAL_DISABLE_FILEWATCHER = truthy("CCODE_EXPERIMENTAL_DISABLE_FILEWATCHER")
  export const CCODE_EXPERIMENTAL_ICON_DISCOVERY =
    CCODE_EXPERIMENTAL || truthy("CCODE_EXPERIMENTAL_ICON_DISCOVERY")
  export const CCODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT = truthy("CCODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT")
  export const CCODE_ENABLE_EXA =
    truthy("CCODE_ENABLE_EXA") || CCODE_EXPERIMENTAL || truthy("CCODE_EXPERIMENTAL_EXA")
  export const CCODE_EXPERIMENTAL_BASH_MAX_OUTPUT_LENGTH = number("CCODE_EXPERIMENTAL_BASH_MAX_OUTPUT_LENGTH")
  export const CCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS = number("CCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS")
  export const CCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX = number("CCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX")
  export const CCODE_EXPERIMENTAL_OXFMT = CCODE_EXPERIMENTAL || truthy("CCODE_EXPERIMENTAL_OXFMT")
  export const CCODE_EXPERIMENTAL_LSP_TY = truthy("CCODE_EXPERIMENTAL_LSP_TY")
  export const CCODE_EXPERIMENTAL_LSP_TOOL = CCODE_EXPERIMENTAL || truthy("CCODE_EXPERIMENTAL_LSP_TOOL")
  export const CCODE_DISABLE_FILETIME_CHECK = truthy("CCODE_DISABLE_FILETIME_CHECK")
  export const CCODE_EXPERIMENTAL_PLAN_MODE = CCODE_EXPERIMENTAL || truthy("CCODE_EXPERIMENTAL_PLAN_MODE")
  export const CCODE_MODELS_URL = process.env["CCODE_MODELS_URL"]

  function number(key: string) {
    const value = process.env[key]
    if (!value) return undefined
    const parsed = Number(value)
    return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined
  }
}

// Dynamic getter for CCODE_DISABLE_PROJECT_CONFIG
// This must be evaluated at access time, not module load time,
// because external tooling may set this env var at runtime
Object.defineProperty(Flag, "CCODE_DISABLE_PROJECT_CONFIG", {
  get() {
    return truthy("CCODE_DISABLE_PROJECT_CONFIG")
  },
  enumerable: true,
  configurable: false,
})

// Dynamic getter for CCODE_CONFIG_DIR
// This must be evaluated at access time, not module load time,
// because external tooling may set this env var at runtime
Object.defineProperty(Flag, "CCODE_CONFIG_DIR", {
  get() {
    return process.env["CCODE_CONFIG_DIR"]
  },
  enumerable: true,
  configurable: false,
})
