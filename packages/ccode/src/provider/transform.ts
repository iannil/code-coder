import type { APICallError, ModelMessage } from "ai"
import { mergeDeep } from "remeda"
import type { JSONSchema } from "zod/v4/core"
import type { Provider } from "./provider"
import type { ModelsDev } from "./models"
import { iife } from "@/util/iife"

// Native bindings from @codecoder-ai/core
// These are required - if not available, the import will fail at startup
import {
  transformMessages as transformMessagesNative,
  getTemperature as getTemperatureNative,
  getTopP as getTopPNative,
  getTopK as getTopKNative,
  getSdkKey as getSdkKeyNative,
} from "@codecoder-ai/core"

type Modality = NonNullable<ModelsDev.Model["modalities"]>["input"][number]

function mimeToModality(mime: string): Modality | undefined {
  if (mime.startsWith("image/")) return "image"
  if (mime.startsWith("audio/")) return "audio"
  if (mime.startsWith("video/")) return "video"
  if (mime === "application/pdf") return "pdf"
  return undefined
}

export namespace ProviderTransform {
  // Maps npm package to the key the AI SDK expects for providerOptions
  // Implemented in Rust (zero-core)
  function sdkKey(npm: string): string | undefined {
    return getSdkKeyNative?.(npm) ?? undefined
  }

  function unsupportedParts(msgs: ModelMessage[], model: Provider.Model): ModelMessage[] {
    return msgs.map((msg) => {
      if (msg.role !== "user" || !Array.isArray(msg.content)) return msg

      const filtered = msg.content.map((part) => {
        if (part.type !== "file" && part.type !== "image") return part

        // Check for empty base64 image data
        if (part.type === "image") {
          const imageStr = part.image.toString()
          if (imageStr.startsWith("data:")) {
            const match = imageStr.match(/^data:([^;]+);base64,(.*)$/)
            if (match && (!match[2] || match[2].length === 0)) {
              return {
                type: "text" as const,
                text: "ERROR: Image file is empty or corrupted. Please provide a valid image.",
              }
            }
          }
        }

        const mime = part.type === "image" ? part.image.toString().split(";")[0].replace("data:", "") : part.mediaType
        const filename = part.type === "file" ? part.filename : undefined
        const modality = mimeToModality(mime)
        if (!modality) return part
        if (model.capabilities.input[modality]) return part

        const name = filename ? `"${filename}"` : modality
        return {
          type: "text" as const,
          text: `ERROR: Cannot read ${name} (this model does not support ${modality} input). Inform the user.`,
        }
      })

      return { ...msg, content: filtered }
    })
  }

  export function message(msgs: ModelMessage[], model: Provider.Model, options: Record<string, unknown>) {
    // Always run unsupportedParts in TypeScript (not migrated to native)
    // This handles model capability checking and error message generation
    msgs = unsupportedParts(msgs, model)

    // Use native implementation for normalize + cache + remap (Rust zero-core)
    if (!transformMessagesNative) {
      throw new Error("Native transformMessages not available - @codecoder-ai/core must be built")
    }

    // Native function signature: transformMessages(provider: string, messages: any[]): any[]
    const result = transformMessagesNative(model.providerID, msgs)
    return result as ModelMessage[]
  }

  // Sampling parameters - implemented in Rust (zero-core)
  export function temperature(model: Provider.Model): number | undefined {
    return getTemperatureNative?.(model.id) ?? undefined
  }

  export function topP(model: Provider.Model): number | undefined {
    return getTopPNative?.(model.id) ?? undefined
  }

  export function topK(model: Provider.Model): number | undefined {
    return getTopKNative?.(model.id) ?? undefined
  }

  const WIDELY_SUPPORTED_EFFORTS = ["low", "medium", "high"]
  const OPENAI_EFFORTS = ["none", "minimal", ...WIDELY_SUPPORTED_EFFORTS, "xhigh"]

  export function variants(model: Provider.Model): Record<string, Record<string, any>> {
    if (!model.capabilities.reasoning) return {}

    const id = model.id.toLowerCase()
    if (id.includes("deepseek") || id.includes("minimax") || id.includes("glm") || id.includes("mistral")) return {}

    // see: https://docs.x.ai/docs/guides/reasoning#control-how-hard-the-model-thinks
    if (id.includes("grok") && id.includes("grok-3-mini")) {
      if (model.api.npm === "@openrouter/ai-sdk-provider") {
        return {
          low: { reasoning: { effort: "low" } },
          high: { reasoning: { effort: "high" } },
        }
      }
      return {
        low: { reasoningEffort: "low" },
        high: { reasoningEffort: "high" },
      }
    }
    if (id.includes("grok")) return {}

    switch (model.api.npm) {
      case "@openrouter/ai-sdk-provider":
        if (!model.id.includes("gpt") && !model.id.includes("gemini-3")) return {}
        return Object.fromEntries(OPENAI_EFFORTS.map((effort) => [effort, { reasoning: { effort } }]))

      // Gateway provider: reasoningEffort is handled internally by the SDK
      // Important: When reasoningEffort is set, max_tokens must NOT be set (see maxOutputTokens function)
      case "@ai-sdk/gateway":
        return Object.fromEntries(OPENAI_EFFORTS.map((effort) => [effort, { reasoningEffort: effort }]))

      case "@ai-sdk/github-copilot":
        const copilotEfforts = iife(() => {
          if (id.includes("5.1-codex-max") || id.includes("5.2")) return [...WIDELY_SUPPORTED_EFFORTS, "xhigh"]
          return WIDELY_SUPPORTED_EFFORTS
        })
        return Object.fromEntries(
          copilotEfforts.map((effort) => [
            effort,
            {
              reasoningEffort: effort,
              reasoningSummary: "auto",
              include: ["reasoning.encrypted_content"],
            },
          ]),
        )

      case "@ai-sdk/cerebras":
      // https://v5.ai-sdk.dev/providers/ai-sdk-providers/cerebras
      case "@ai-sdk/togetherai":
      // https://v5.ai-sdk.dev/providers/ai-sdk-providers/togetherai
      case "@ai-sdk/xai":
      // https://v5.ai-sdk.dev/providers/ai-sdk-providers/xai
      case "@ai-sdk/deepinfra":
      // https://v5.ai-sdk.dev/providers/ai-sdk-providers/deepinfra
      case "@ai-sdk/openai-compatible":
        return Object.fromEntries(WIDELY_SUPPORTED_EFFORTS.map((effort) => [effort, { reasoningEffort: effort }]))

      case "@ai-sdk/azure":
        // https://v5.ai-sdk.dev/providers/ai-sdk-providers/azure
        if (id === "o1-mini") return {}
        const azureEfforts = ["low", "medium", "high"]
        if (id.includes("gpt-5-") || id === "gpt-5") {
          azureEfforts.unshift("minimal")
        }
        return Object.fromEntries(
          azureEfforts.map((effort) => [
            effort,
            {
              reasoningEffort: effort,
              reasoningSummary: "auto",
              include: ["reasoning.encrypted_content"],
            },
          ]),
        )
      case "@ai-sdk/openai":
        // https://v5.ai-sdk.dev/providers/ai-sdk-providers/openai
        if (id === "gpt-5-pro") return {}
        const openaiEfforts = iife(() => {
          if (id.includes("codex")) {
            if (id.includes("5.2")) return [...WIDELY_SUPPORTED_EFFORTS, "xhigh"]
            return WIDELY_SUPPORTED_EFFORTS
          }
          const arr = [...WIDELY_SUPPORTED_EFFORTS]
          if (id.includes("gpt-5-") || id === "gpt-5") {
            arr.unshift("minimal")
          }
          if (model.release_date >= "2025-11-13") {
            arr.unshift("none")
          }
          if (model.release_date >= "2025-12-04") {
            arr.push("xhigh")
          }
          return arr
        })
        return Object.fromEntries(
          openaiEfforts.map((effort) => [
            effort,
            {
              reasoningEffort: effort,
              reasoningSummary: "auto",
              include: ["reasoning.encrypted_content"],
            },
          ]),
        )

      case "@ai-sdk/anthropic":
      // https://v5.ai-sdk.dev/providers/ai-sdk-providers/anthropic
      case "@ai-sdk/google-vertex/anthropic":
        // https://v5.ai-sdk.dev/providers/ai-sdk-providers/google-vertex#anthropic-provider
        return {
          high: {
            thinking: {
              type: "enabled",
              budgetTokens: 16000,
            },
          },
          max: {
            thinking: {
              type: "enabled",
              budgetTokens: 31999,
            },
          },
        }

      case "@ai-sdk/amazon-bedrock":
        // https://v5.ai-sdk.dev/providers/ai-sdk-providers/amazon-bedrock
        // For Anthropic models on Bedrock, use reasoningConfig with budgetTokens
        if (model.api.id.includes("anthropic")) {
          return {
            high: {
              reasoningConfig: {
                type: "enabled",
                budgetTokens: 16000,
              },
            },
            max: {
              reasoningConfig: {
                type: "enabled",
                budgetTokens: 31999,
              },
            },
          }
        }

        // For Amazon Nova models, use reasoningConfig with maxReasoningEffort
        return Object.fromEntries(
          WIDELY_SUPPORTED_EFFORTS.map((effort) => [
            effort,
            {
              reasoningConfig: {
                type: "enabled",
                maxReasoningEffort: effort,
              },
            },
          ]),
        )

      case "@ai-sdk/google-vertex":
      // https://v5.ai-sdk.dev/providers/ai-sdk-providers/google-vertex
      case "@ai-sdk/google":
        // https://v5.ai-sdk.dev/providers/ai-sdk-providers/google-generative-ai
        if (id.includes("2.5")) {
          return {
            high: {
              thinkingConfig: {
                includeThoughts: true,
                thinkingBudget: 16000,
              },
            },
            max: {
              thinkingConfig: {
                includeThoughts: true,
                thinkingBudget: 24576,
              },
            },
          }
        }
        return Object.fromEntries(
          ["low", "high"].map((effort) => [
            effort,
            {
              includeThoughts: true,
              thinkingLevel: effort,
            },
          ]),
        )

      case "@ai-sdk/mistral":
        // https://v5.ai-sdk.dev/providers/ai-sdk-providers/mistral
        return {}

      case "@ai-sdk/cohere":
        // https://v5.ai-sdk.dev/providers/ai-sdk-providers/cohere
        return {}

      case "@ai-sdk/groq":
        // https://v5.ai-sdk.dev/providers/ai-sdk-providers/groq
        const groqEffort = ["none", ...WIDELY_SUPPORTED_EFFORTS]
        return Object.fromEntries(
          groqEffort.map((effort) => [
            effort,
            {
              includeThoughts: true,
              thinkingLevel: effort,
            },
          ]),
        )

      case "@ai-sdk/perplexity":
        // https://v5.ai-sdk.dev/providers/ai-sdk-providers/perplexity
        return {}
    }
    return {}
  }

  export function options(input: {
    model: Provider.Model
    sessionID: string
    providerOptions?: Record<string, any>
  }): Record<string, any> {
    const result: Record<string, any> = {}

    // openai and providers using openai package should set store to false by default.
    if (
      input.model.providerID === "openai" ||
      input.model.api.npm === "@ai-sdk/openai" ||
      input.model.api.npm === "@ai-sdk/github-copilot"
    ) {
      result["store"] = false
    }

    if (input.model.api.npm === "@openrouter/ai-sdk-provider") {
      result["usage"] = {
        include: true,
      }
      if (input.model.api.id.includes("gemini-3")) {
        result["reasoning"] = { effort: "high" }
      }
    }

    if (
      input.model.providerID === "baseten" ||
      (input.model.providerID === "ccode" && ["kimi-k2-thinking", "glm-4.6"].includes(input.model.api.id))
    ) {
      result["chat_template_args"] = { enable_thinking: true }
    }

    if (["zai", "zhipuai"].includes(input.model.providerID) && input.model.api.npm === "@ai-sdk/openai-compatible") {
      result["thinking"] = {
        type: "enabled",
        clear_thinking: false,
      }
    }

    if (input.model.providerID === "openai" || input.providerOptions?.setCacheKey) {
      result["promptCacheKey"] = input.sessionID
    }

    if (input.model.api.npm === "@ai-sdk/google" || input.model.api.npm === "@ai-sdk/google-vertex") {
      result["thinkingConfig"] = {
        includeThoughts: true,
      }
      if (input.model.api.id.includes("gemini-3")) {
        result["thinkingConfig"]["thinkingLevel"] = "high"
      }
    }

    if (input.model.api.id.includes("gpt-5") && !input.model.api.id.includes("gpt-5-chat")) {
      if (!input.model.api.id.includes("gpt-5-pro")) {
        result["reasoningEffort"] = "medium"
      }

      if (
        input.model.api.id.includes("gpt-5.") &&
        !input.model.api.id.includes("codex") &&
        input.model.providerID !== "azure"
      ) {
        result["textVerbosity"] = "low"
      }

      if (input.model.providerID.startsWith("ccode")) {
        result["promptCacheKey"] = input.sessionID
        result["include"] = ["reasoning.encrypted_content"]
        result["reasoningSummary"] = "auto"
      }
    }

    if (input.model.providerID === "venice") {
      result["promptCacheKey"] = input.sessionID
    }

    return result
  }

  export function smallOptions(model: Provider.Model) {
    if (model.providerID === "openai" || model.api.id.includes("gpt-5")) {
      if (model.api.id.includes("5.")) {
        return { reasoningEffort: "low" }
      }
      return { reasoningEffort: "minimal" }
    }
    if (model.providerID === "google") {
      // gemini-3 uses thinkingLevel, gemini-2.5 uses thinkingBudget
      if (model.api.id.includes("gemini-3")) {
        return { thinkingConfig: { thinkingLevel: "minimal" } }
      }
      return { thinkingConfig: { thinkingBudget: 0 } }
    }
    if (model.providerID === "openrouter") {
      if (model.api.id.includes("google")) {
        return { reasoning: { enabled: false } }
      }
      return { reasoningEffort: "minimal" }
    }
    return {}
  }

  export function providerOptions(model: Provider.Model, options: { [x: string]: any }) {
    const key = sdkKey(model.api.npm) ?? model.providerID
    return { [key]: options }
  }

  // Provider-specific hard limits for max output tokens
  // These caps take precedence over model-defined limits to prevent API errors
  // when the actual provider has stricter limits than advertised
  const PROVIDER_MAX_OUTPUT_CAPS: Record<string, number> = {
    // Add provider-specific caps here
    // Example: "@ai-sdk/openai": 16384,
    // These caps override model limits when lower
  }

  export function maxOutputTokens(
    npm: string,
    options: Record<string, any>,
    modelLimit: number,
    globalLimit: number,
  ): number | undefined {
    // @ai-sdk/gateway: Cannot set max_tokens when reasoningEffort is set
    // OpenAI reasoning models (o1, o3, etc.) use max_completion_tokens internally
    // and setting both parameters causes API errors
    if (npm === "@ai-sdk/gateway" && options?.["reasoningEffort"] !== undefined) {
      return undefined
    }

    const modelCap = modelLimit || globalLimit
    let standardLimit = Math.min(modelCap, globalLimit)

    // Apply provider-specific hard caps if defined
    const providerCap = PROVIDER_MAX_OUTPUT_CAPS[npm]
    if (providerCap && providerCap > 0) {
      standardLimit = Math.min(standardLimit, providerCap)
    }

    // Apply provider-level cap from options (allows config-based override)
    // Users can set provider.options.maxOutputTokens in their config
    const optionsCap = options?.maxOutputTokens
    if (typeof optionsCap === "number" && optionsCap > 0) {
      standardLimit = Math.min(standardLimit, optionsCap)
    }

    if (npm === "@ai-sdk/anthropic" || npm === "@ai-sdk/google-vertex/anthropic") {
      const thinking = options?.["thinking"]

      // Check if thinking is explicitly disabled - return standard limit without modification
      if (thinking?.["type"] === "disabled") {
        return standardLimit
      }

      const budgetTokens = typeof thinking?.["budgetTokens"] === "number" ? thinking["budgetTokens"] : 0
      const enabled = thinking?.["type"] === "enabled"

      if (enabled && budgetTokens > 0) {
        // When thinking mode is enabled with budgetTokens:
        // 1. If standardLimit + budgetTokens fits in modelCap, use standardLimit
        //    This preserves agent/user configuration when there's enough capacity
        // 2. Otherwise, calculate available space and ensure we get reasonable output
        const availableForOutput = modelCap - budgetTokens

        if (standardLimit + budgetTokens <= modelCap) {
          // Standard limit fits with thinking budget - preserve user/agent preference
          return standardLimit
        }

        // Standard limit doesn't fit, return available space
        // At minimum, ensure 80% of available space for meaningful output
        const standardOutput = Math.min(standardLimit, availableForOutput)
        const minimumOutput = Math.min(modelCap * 0.8, availableForOutput)
        return Math.max(standardOutput, minimumOutput)
      }
    }

    return standardLimit
  }

  export function schema(model: Provider.Model, schema: JSONSchema.BaseSchema) {
    /*
    if (["openai", "azure"].includes(providerID)) {
      if (schema.type === "object" && schema.properties) {
        for (const [key, value] of Object.entries(schema.properties)) {
          if (schema.required?.includes(key)) continue
          schema.properties[key] = {
            anyOf: [
              value as JSONSchema.JSONSchema,
              {
                type: "null",
              },
            ],
          }
        }
      }
    }
    */

    // Convert integer enums to string enums for Google/Gemini
    if (model.providerID === "google" || model.api.id.includes("gemini")) {
      const sanitizeGemini = (obj: any, depth = 0): any => {
        if (obj === null || typeof obj !== "object") {
          return obj
        }

        if (Array.isArray(obj)) {
          return obj.map((item) => sanitizeGemini(item, depth))
        }

        const result: any = {}
        for (const [key, value] of Object.entries(obj)) {
          // Skip zod's internal 'ref' field - Google API doesn't recognize it
          if (key === "ref") continue

          // Skip 'additionalProperties' as it can cause issues with nested schemas
          if (key === "additionalProperties" && depth > 0) continue

          if (key === "enum" && Array.isArray(value)) {
            // Convert all enum values to strings
            result[key] = value.map((v) => String(v))
            // If we have integer type with enum, change type to string
            if (result.type === "integer" || result.type === "number") {
              result.type = "string"
            }
          } else if (typeof value === "object" && value !== null) {
            result[key] = sanitizeGemini(value, depth + 1)
          } else {
            result[key] = value
          }
        }

        // Filter required array to only include fields that exist in properties
        if (result.type === "object" && result.properties && Array.isArray(result.required)) {
          result.required = result.required.filter((field: any) => field in result.properties)
        }

        if (result.type === "array" && result.items == null) {
          result.items = {}
        }

        return result
      }

      schema = sanitizeGemini(schema)
    }

    return schema
  }

  export function error(providerID: string, error: APICallError) {
    let message = error.message
    if (providerID.includes("github-copilot") && error.statusCode === 403) {
      return "Please reauthenticate with the copilot provider to ensure your credentials work properly with CodeCoder."
    }
    if (providerID.includes("github-copilot") && message.includes("The requested model is not supported")) {
      return (
        message +
        "\n\nMake sure the model is enabled in your copilot settings: https://github.com/settings/copilot/features"
      )
    }

    return message
  }
}
