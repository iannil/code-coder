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

        let selectedProvider: string | undefined
        let selectedModel: string | undefined

        // Load all providers for use across steps
        await ModelsDev.refresh().catch(() => {})
        ModelsDev.Data.reset()
        const allProviders = await ModelsDev.get()

        // Step 1: Select Provider
        if (needsProvider) {
          UI.empty()
          prompts.intro("Step 1: Select AI Provider")

          const priority: Record<string, number> = {
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

          const providerOptions = Object.values(allProviders).map((p) => ({
            label: p.name || p.id,
            value: p.id,
            hint: `${Object.keys(p.models ?? {}).length} models`,
          }))

          const providerResult = await prompts.autocomplete({
            message: "Select your AI provider",
            maxItems: 10,
            options: providerOptions,
          })

          if (prompts.isCancel(providerResult)) throw new UI.CancelledError()
          selectedProvider = providerResult

          prompts.log.success(`Selected ${allProviders[selectedProvider]?.name || selectedProvider}`)
        }

        // Step 2: Select Model from chosen provider
        if (needsModel) {
          UI.empty()
          prompts.intro("Step 2: Select Default Model")

          if (!selectedProvider) {
            const authProviders = Object.keys(await Auth.all())
            if (authProviders.length === 0) {
              prompts.log.error("No authenticated provider found")
              prompts.outro("Please run `codecoder auth login` first")
              return
            }
            if (authProviders.length === 1) {
              selectedProvider = authProviders[0]
            } else {
              const providerSelectResult = await prompts.select({
                message: "Select provider",
                options: authProviders.map((id) => ({
                  label: allProviders[id]?.name || id,
                  value: id,
                })),
              })
              if (prompts.isCancel(providerSelectResult)) throw new UI.CancelledError()
              selectedProvider = providerSelectResult
            }
            prompts.log.info(`Using ${allProviders[selectedProvider]?.name || selectedProvider}`)
          }

          const provider = allProviders[selectedProvider!]
          if (!provider) {
            prompts.log.error(`Provider ${selectedProvider} not found`)
            return
          }

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

          const modelOptions = sortedModels
            .filter(([id, model]) => model.tool_call !== false)
            .map(([id, model]) => ({
              label: model.name || id,
              value: `${selectedProvider}/${id}`,
              hint: model.status ?? "",
            }))

          if (modelOptions.length === 0) {
            prompts.log.error(`No models available for ${provider.name || selectedProvider}`)
            return
          }

          prompts.log.info(`Found ${modelOptions.length} models for ${provider.name || selectedProvider}`)

          const model = await prompts.autocomplete({
            message: "Select your default model",
            maxItems: 30,
            options: modelOptions,
          })

          if (prompts.isCancel(model)) throw new UI.CancelledError()
          selectedModel = model

          const modelName = modelOptions.find((m) => m.value === model)?.label || model
          prompts.log.success(`Selected ${modelName}`)
        }

        // Step 3: Optional custom configuration
        if (needsProvider || needsModel) {
          UI.empty()
          prompts.intro("Step 3: Optional Custom Configuration")

          const shouldConfigure = await prompts.confirm({
            message: "Do you want to configure custom URL or API key?",
            initialValue: false,
          })

          if (prompts.isCancel(shouldConfigure)) throw new UI.CancelledError()

          if (shouldConfigure) {
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
            const customKey = keyResult?.trim()

            if (customUrl || customKey) {
              if (!selectedProvider) {
                prompts.log.error("No provider selected")
                return
              }
              await Auth.set(selectedProvider, {
                type: "api",
                key: customKey,
              })
              prompts.log.success("Credentials saved")
            }

            if (customUrl) {
              prompts.log.info(`Custom URL: ${customUrl}`)
            }
          }
        }

        // Step 4: Select Default Agent
        if (needsAgent) {
          UI.empty()
          prompts.intro("Step 4: Select Default Agent")

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
        prompts.log.success("CodeCoder is ready to use")
        prompts.log.message("Run `codecoder` to get started")
        prompts.log.message("See https://code-coder.com/docs for more information")
        prompts.outro("Happy coding!")
      },
    })
  },
})
