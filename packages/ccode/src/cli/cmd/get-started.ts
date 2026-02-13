import { Auth } from "../../auth"
import { cmd } from "./cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../ui"
import { ModelsDev } from "../../provider/models"
import { Config } from "../../config/config"
import { Instance } from "../../project/instance"

const PRIMARY_AGENTS = [
  { value: "build", label: "build", hint: "General coding and development" },
  { value: "plan", label: "plan", hint: "Planning and design tasks" },
]

const PROVIDER_PRIORITY: Record<string, number> = {
  ccode: 0,
  anthropic: 1,
  "github-copilot": 2,
  openai: 3,
  google: 4,
  "amazon-bedrock": 5,
  openrouter: 6,
  vercel: 7,
  xai: 8,
  cohere: 9,
  mistral: 10,
  groq: 11,
  deepseek: 12,
}

const CUSTOM_PROVIDER_OPTION = "__custom__"

interface ProviderConfig {
  id: string
  name: string
  apiKey?: string
  customUrl?: string
  isCustom?: boolean
  customModels?: CustomModel[]
}

interface CustomModel {
  id: string
  name: string
  contextLength?: number
  maxTokens?: {
    input?: number
    output?: number
  }
  supportsTools?: boolean
}

// Type for custom provider model configuration
type CustomModelConfig = {
  id?: string
  name?: string
  tool_call?: boolean
  limit?: {
    context?: number
    output?: number
  }
  cost?: {
    input?: number
    output?: number
  }
  modalities?: {
    input?: string[]
    output?: string[]
  }
  attachment?: boolean
  reasoning?: boolean
  temperature?: boolean
}

type CustomProviderConfig = {
  id: string
  name: string
  api: string
  npm?: string
  apiKey?: string
  models: Record<string, CustomModelConfig>
}

function sortProviders(providers: ModelsDev.Provider[]): ModelsDev.Provider[] {
  return [...providers].sort((a, b) => {
    const aPriority = PROVIDER_PRIORITY[a.id] ?? 999
    const bPriority = PROVIDER_PRIORITY[b.id] ?? 999
    return aPriority - bPriority
  })
}

async function selectProvider(
  allProviders: Record<string, ModelsDev.Provider>,
  configuredProviders: string[],
): Promise<string | undefined> {
  const providerOptions = sortProviders(Object.values(allProviders))
    .filter((p) => !configuredProviders.includes(p.id))
    .map((p) => ({
      label: p.name || p.id,
      value: p.id,
      hint: `${Object.keys(p.models ?? {}).length} models`,
    }))

  // Add custom provider option
  providerOptions.push({
    label: "+ Add custom third-party provider",
    value: CUSTOM_PROVIDER_OPTION,
    hint: "Configure a custom API endpoint",
  })

  const providerResult = await prompts.autocomplete({
    message: "Select your AI provider",
    maxItems: 10,
    options: providerOptions,
  })

  if (prompts.isCancel(providerResult)) throw new UI.CancelledError()
  if (providerResult === CUSTOM_PROVIDER_OPTION) return CUSTOM_PROVIDER_OPTION
  return providerResult
}

async function createCustomProvider(): Promise<CustomProviderConfig | undefined> {
  UI.empty()
  prompts.intro("Configure Custom Third-Party Provider")

  const providerIdResult = await prompts.text({
    message: "Provider ID (e.g., my-custom-provider)",
    placeholder: "my-custom-provider",
    validate: (value) => {
      if (!value || value.trim().length === 0) return "Provider ID is required"
      if (!/^[a-z0-9-]+$/.test(value.trim())) {
        return "Provider ID must contain only lowercase letters, numbers, and hyphens"
      }
      return undefined
    },
  })

  if (prompts.isCancel(providerIdResult)) throw new UI.CancelledError()
  const providerId = providerIdResult.trim()

  const providerNameResult = await prompts.text({
    message: "Provider name (e.g., My Custom Provider)",
    placeholder: "My Custom Provider",
    validate: (value) => {
      if (!value || value.trim().length === 0) return "Provider name is required"
      return undefined
    },
  })

  if (prompts.isCancel(providerNameResult)) throw new UI.CancelledError()
  const providerName = providerNameResult.trim()

  const apiBaseUrlResult = await prompts.text({
    message: "API Base URL (e.g., https://api.example.com/v1)",
    placeholder: "https://api.example.com/v1",
    validate: (value) => {
      if (!value || value.trim().length === 0) return "API Base URL is required"
      try {
        new URL(value.trim())
      } catch {
        return "Invalid URL format"
      }
      return undefined
    },
  })

  if (prompts.isCancel(apiBaseUrlResult)) throw new UI.CancelledError()
  const apiBaseUrl = apiBaseUrlResult.trim()

  // Ask for SDK package (default to openai-compatible)
  const npmResult = await prompts.text({
    message: "NPM package (default: @ai-sdk/openai-compatible)",
    placeholder: "@ai-sdk/openai-compatible",
    initialValue: "@ai-sdk/openai-compatible",
  })

  if (prompts.isCancel(npmResult)) throw new UI.CancelledError()
  const npm = npmResult.trim() || "@ai-sdk/openai-compatible"

  // Ask for API key
  const apiKeyResult = await prompts.password({
    message: "API Key (optional, press Enter to skip)",
    validate: () => undefined,
  })

  if (prompts.isCancel(apiKeyResult)) throw new UI.CancelledError()
  const apiKey = apiKeyResult?.trim()

  // Ask if user wants to add models
  const addModels = await prompts.confirm({
    message: "Would you like to add models for this provider?",
    initialValue: true,
  })

  if (prompts.isCancel(addModels)) throw new UI.CancelledError()

  const models: Record<string, CustomModelConfig> = {}

  if (addModels) {
    let addingModels = true
    while (addingModels) {
      const model = await createCustomModel(providerId)
      if (model) {
        models[model.id] = model
        prompts.log.success(`Added model: ${model.name || model.id}`)

        const addAnother = await prompts.confirm({
          message: "Add another model?",
          initialValue: false,
        })

        if (prompts.isCancel(addAnother)) throw new UI.CancelledError()
        addingModels = addAnother
      } else {
        addingModels = false
      }
    }
  }

  if (Object.keys(models).length === 0) {
    prompts.log.warn("No models configured for custom provider")
    prompts.log.info("You can add models later in your config file")
  }

  // Save credentials if provided
  if (apiKey) {
    await Auth.set(providerId, {
      type: "api",
      key: apiKey,
    })
    prompts.log.success("API key saved")
  }

  return {
    id: providerId,
    name: providerName,
    api: apiBaseUrl,
    npm,
    apiKey,
    models,
  }
}

async function createCustomModel(providerId: string): Promise<CustomModelConfig | undefined> {
  UI.empty()
  prompts.intro("Add Custom Model")

  const modelIdResult = await prompts.text({
    message: "Model ID (e.g., my-model)",
    placeholder: "my-model",
    validate: (value) => {
      if (!value || value.trim().length === 0) return "Model ID is required"
      return undefined
    },
  })

  if (prompts.isCancel(modelIdResult)) throw new UI.CancelledError()
  const modelId = modelIdResult.trim()

  const modelNameResult = await prompts.text({
    message: "Model name (e.g., My Model)",
    placeholder: "My Model",
    validate: (value) => {
      if (!value || value.trim().length === 0) return "Model name is required"
      return undefined
    },
  })

  if (prompts.isCancel(modelNameResult)) throw new UI.CancelledError()
  const modelName = modelNameResult.trim()

  const supportsTools = await prompts.confirm({
    message: "Does this model support tool calling?",
    initialValue: true,
  })

  if (prompts.isCancel(supportsTools)) throw new UI.CancelledError()

  const contextLengthResult = await prompts.text({
    message: "Context length in tokens (optional, default: 128000)",
    placeholder: "128000",
    initialValue: "128000",
  })

  if (prompts.isCancel(contextLengthResult)) throw new UI.CancelledError()
  const contextLength = Number.parseInt(contextLengthResult?.trim() || "128000", 10) || 128000

  const maxOutputResult = await prompts.text({
    message: "Max output tokens (optional, default: 4096)",
    placeholder: "4096",
    initialValue: "4096",
  })

  if (prompts.isCancel(maxOutputResult)) throw new UI.CancelledError()
  const maxOutput = Number.parseInt(maxOutputResult?.trim() || "4096", 10) || 4096

  return {
    id: modelId,
    name: modelName,
    tool_call: supportsTools,
    limit: {
      context: contextLength,
      output: maxOutput,
    },
    cost: {
      input: 0,
      output: 0,
    },
    modalities: {
      input: ["text"],
      output: ["text"],
    },
    attachment: false,
    reasoning: false,
    temperature: true,
  }
}

async function configureProviderCredentials(
  providerId: string,
  providerName: string,
): Promise<{ apiKey?: string; customUrl?: string }> {
  UI.empty()
  prompts.intro(`Configure ${providerName}`)

  const urlResult = await prompts.text({
    message: "Custom API URL (optional, press Enter to skip)",
    placeholder: "https://api.example.com/v1",
    initialValue: "",
  })

  if (prompts.isCancel(urlResult)) throw new UI.CancelledError()

  const keyResult = await prompts.password({
    message: "API Key (optional, press Enter to skip)",
    validate: () => undefined,
  })

  if (prompts.isCancel(keyResult)) throw new UI.CancelledError()

  const customUrl = urlResult?.trim()
  const apiKey = keyResult?.trim()

  // Save credentials if provided
  if (apiKey) {
    await Auth.set(providerId, {
      type: "api",
      key: apiKey,
    })
    prompts.log.success("API key saved")
  }

  if (customUrl) {
    prompts.log.info(`Custom URL: ${customUrl}`)
  }

  return { apiKey, customUrl }
}

async function selectDefaultModel(
  allProviders: Record<string, ModelsDev.Provider>,
  configuredProviders: string[],
  customProviders: ProviderConfig[],
): Promise<string | undefined> {
  UI.empty()
  prompts.intro("Select Default Model")

  // Build model options from all configured providers
  const allModelOptions: Array<{ label: string; value: string; hint: string; provider: string }> = []

  // Add models from official providers
  for (const providerId of configuredProviders) {
    const provider = allProviders[providerId]
    if (!provider) continue

    const models = Object.entries(provider?.models ?? {})
    const sortedModels = [...models].sort((a, b) => {
      const aModel = a[1]
      const bModel = b[1]
      if (aModel?.status === "alpha") return 1
      if (bModel?.status === "alpha") return -1
      if (aModel?.status === "beta") return 1
      if (bModel?.status === "beta") return -1
      return 0
    })

    for (const [id, model] of sortedModels) {
      if (model.tool_call === false) continue
      allModelOptions.push({
        label: model.name || id,
        value: `${providerId}/${id}`,
        hint: `${providerId} ${model.status ? `(${model.status})` : ""}`,
        provider: providerId,
      })
    }
  }

  // Add models from custom providers
  for (const providerConfig of customProviders) {
    if (providerConfig.isCustom && providerConfig.customModels) {
      for (const model of providerConfig.customModels) {
        allModelOptions.push({
          label: model.name,
          value: `${providerConfig.id}/${model.id}`,
          hint: `${providerConfig.id} (custom)`,
          provider: providerConfig.id,
        })
      }
    }
  }

  if (allModelOptions.length === 0) {
    prompts.log.error("No models available from configured providers")
    return undefined
  }

  prompts.log.info(`Found ${allModelOptions.length} models from ${configuredProviders.length} official provider(s)`)

  const model = await prompts.autocomplete({
    message: "Select your default model",
    maxItems: 30,
    options: allModelOptions,
  })

  if (prompts.isCancel(model)) throw new UI.CancelledError()
  return model
}

export const GetStartedCommand = cmd({
  command: "get-started",
  describe: "Initialize CodeCoder with interactive setup wizard",
  async handler() {
    await Instance.provide({
      directory: process.cwd(),
      async fn() {
        UI.empty()
        prompts.intro("Welcome to CodeCoder!")

        const existingAuth = await Auth.all()
        const existingConfig = await Config.getGlobal()

        let needsProvider = Object.keys(existingAuth).length === 0
        let needsModel = !existingConfig.model
        let needsAgent = !existingConfig.default_agent

        if (!needsProvider && !needsModel && !needsAgent) {
          prompts.log.info("You already have a basic configuration set up.")
          const shouldContinue = await prompts.confirm({
            message: "Would you like to reconfigure anyway?",
            initialValue: false,
          })
          if (prompts.isCancel(shouldContinue)) throw new UI.CancelledError()
          if (!shouldContinue) {
            prompts.outro("Setup complete. Run `codecoder` to get started!")
            return
          }
          needsProvider = true
          needsModel = true
          needsAgent = true
        }

        // Load all providers for use across steps
        await ModelsDev.refresh().catch(() => {})
        ModelsDev.Data.reset()
        const allProviders = await ModelsDev.get()

        const configuredProviders: string[] = []
        const customProviders: ProviderConfig[] = []
        const providerConfigs: ProviderConfig[] = []

        // Step 1: Configure Providers (loop to support multiple)
        if (needsProvider) {
          let addingProviders = true

          while (addingProviders) {
            UI.empty()
            prompts.intro(
              configuredProviders.length === 0
                ? "Step 1: Configure AI Providers"
                : `Step 1: Configure AI Providers (${configuredProviders.length} configured)`,
            )

            const selectedProvider = await selectProvider(allProviders, configuredProviders)
            if (selectedProvider === CUSTOM_PROVIDER_OPTION) {
              // Create custom provider
              const customProvider = await createCustomProvider()
              if (customProvider) {
                // Save custom provider to config
                const providerConfig: Record<string, Config.Provider> = {}
                providerConfig[customProvider.id] = {
                  name: customProvider.name,
                  api: customProvider.api,
                  npm: customProvider.npm,
                  options: customProvider.apiKey
                    ? {
                        apiKey: customProvider.apiKey,
                      }
                    : undefined,
                  models: customProvider.models,
                }

                await Config.updateGlobal({ provider: providerConfig })

                configuredProviders.push(customProvider.id)
                customProviders.push({
                  id: customProvider.id,
                  name: customProvider.name,
                  apiKey: customProvider.apiKey,
                  customUrl: customProvider.api,
                  isCustom: true,
                })

                prompts.log.success(`Configured custom provider: ${customProvider.name}`)
              }
            } else if (selectedProvider) {
              const provider = allProviders[selectedProvider]
              prompts.log.success(`Selected ${provider?.name || selectedProvider}`)

              const credentials = await configureProviderCredentials(selectedProvider, provider?.name || selectedProvider)

              configuredProviders.push(selectedProvider)
              providerConfigs.push({
                id: selectedProvider,
                name: provider?.name || selectedProvider,
                apiKey: credentials.apiKey,
                customUrl: credentials.customUrl,
              })

              // Ask if user wants to add another provider
              const addAnother = await prompts.confirm({
                message: "Would you like to add another provider?",
                initialValue: false,
              })

              if (prompts.isCancel(addAnother)) throw new UI.CancelledError()
              addingProviders = addAnother
            } else {
              addingProviders = false
            }
          }

          if (configuredProviders.length === 0) {
            prompts.log.warn("No providers configured")
            prompts.outro("Run `codecoder auth login` to authenticate with a provider")
            return
          }

          const customCount = customProviders.length
          const officialCount = configuredProviders.length - customCount
          prompts.log.success(
            `Configured ${configuredProviders.length} provider(s)` +
              (customCount > 0 ? ` (${officialCount} official, ${customCount} custom)` : ""),
          )
        } else {
          // Use existing auth providers
          configuredProviders.push(...Object.keys(existingAuth))
        }

        // Step 2: Select Default Model
        let selectedModel: string | undefined
        if (needsModel) {
          selectedModel = await selectDefaultModel(allProviders, configuredProviders, customProviders)
          if (selectedModel) {
            const [providerId, modelId] = selectedModel.split("/")
            const provider = allProviders[providerId]
            const model = provider?.models[modelId]
            const customProvider = customProviders.find((p) => p.id === providerId)
            const modelName = customProvider?.customModels?.find((m) => m.id === modelId)?.name || model?.name || modelId
            prompts.log.success(`Selected ${modelName} (${providerId})`)
          }
        }

        // Step 3: Select Default Agent
        if (needsAgent) {
          UI.empty()
          prompts.intro("Step 3: Select Default Agent")

          const agent = await prompts.select({
            message: "Select your default agent",
            options: PRIMARY_AGENTS,
            initialValue: "build",
          })

          if (prompts.isCancel(agent)) throw new UI.CancelledError()

          prompts.log.success(`Selected ${agent} agent`)

          await Config.updateGlobal({
            default_agent: agent,
          })
        }

        // Save configuration
        const configUpdates: Record<string, any> = {}

        if (selectedModel) {
          configUpdates.model = selectedModel
        }

        if (Object.keys(configUpdates).length > 0) {
          await Config.updateGlobal(configUpdates)
        }

        UI.empty()
        prompts.intro("Setup Complete!")
        prompts.log.success(`Configured ${configuredProviders.length} provider(s):`)
        for (const providerId of configuredProviders) {
          const provider = allProviders[providerId]
          const customProvider = customProviders.find((p) => p.id === providerId)
          const name = customProvider?.name || provider?.name || providerId
          const type = customProvider ? " (custom)" : ""
          prompts.log.message(`  - ${name}${type}`)
        }
        if (selectedModel) {
          const [providerId, modelId] = selectedModel.split("/")
          const provider = allProviders[providerId]
          const customProvider = customProviders.find((p) => p.id === providerId)
          const model = provider?.models[modelId]
          const customModel = customProvider?.customModels?.find((m) => m.id === modelId)
          const modelName = customModel?.name || model?.name || modelId
          prompts.log.success(`Default model: ${modelName}`)
        }
        prompts.log.message("Run `codecoder` to get started")
        prompts.log.message("See https://code-coder.com/docs for more information")
        prompts.outro("Happy coding!")
      },
    })
  },
})
