/**
 * AI Provider Mock for Testing
 *
 * Provides mocking utilities for AI provider APIs
 * Used by lifecycle tests that need to simulate AI interactions
 */

import { mock } from "bun:test"

export interface MockMessage {
  role: "user" | "assistant"
  content: string
}

export interface MockToolCall {
  name: string
  arguments: Record<string, unknown>
}

export interface MockProviderState {
  responses: string[]
  currentResponseIndex: number
  shouldError: boolean
  errorMessage: string
  latencyMs: number
  tokenUsage: {
    prompt: number
    completion: number
  }
  toolCalls: MockToolCall[]
}

const state: MockProviderState = {
  responses: ["Mock response"],
  currentResponseIndex: 0,
  shouldError: false,
  errorMessage: "Mock provider error",
  latencyMs: 0,
  tokenUsage: {
    prompt: 10,
    completion: 20,
  },
  toolCalls: [],
}

/**
 * Creates a mock AI provider for testing
 */
export function createMockProvider() {
  return {
    /**
     * Generate text response (non-streaming)
     */
    generateText: async (_options: { messages: MockMessage[]; model?: string }) => {
      if (state.latencyMs > 0) {
        await new Promise((resolve) => setTimeout(resolve, state.latencyMs))
      }

      if (state.shouldError) {
        throw new Error(state.errorMessage)
      }

      const response = state.responses[state.currentResponseIndex % state.responses.length]
      state.currentResponseIndex++

      return {
        text: response,
        usage: {
          promptTokens: state.tokenUsage.prompt,
          completionTokens: state.tokenUsage.completion,
        },
        toolCalls: state.toolCalls,
      }
    },

    /**
     * Stream text response
     */
    streamText: async function* (_options: { messages: MockMessage[]; model?: string }) {
      if (state.latencyMs > 0) {
        await new Promise((resolve) => setTimeout(resolve, state.latencyMs))
      }

      if (state.shouldError) {
        throw new Error(state.errorMessage)
      }

      const response = state.responses[state.currentResponseIndex % state.responses.length]
      state.currentResponseIndex++

      // Stream response word by word
      const words = response.split(" ")
      for (const word of words) {
        yield { textDelta: word + " " }
      }

      // Emit tool calls if any
      for (const toolCall of state.toolCalls) {
        yield {
          type: "tool_call",
          toolName: toolCall.name,
          args: toolCall.arguments,
        }
      }
    },

    /**
     * Generate with tool calls
     */
    generateWithTools: async (_options: {
      messages: MockMessage[]
      tools: Array<{ name: string; description: string }>
    }) => {
      if (state.shouldError) {
        throw new Error(state.errorMessage)
      }

      return {
        text: state.responses[state.currentResponseIndex % state.responses.length],
        toolCalls: state.toolCalls,
        usage: {
          promptTokens: state.tokenUsage.prompt,
          completionTokens: state.tokenUsage.completion,
        },
      }
    },
  }
}

/**
 * Mock controller for configuring provider behavior
 */
export function createMockProviderController() {
  return {
    /**
     * Set responses the mock will return (cycles through)
     */
    setResponses: (responses: string[]) => {
      state.responses = responses
      state.currentResponseIndex = 0
    },

    /**
     * Set the mock to error on next call
     */
    setError: (shouldError: boolean, message?: string) => {
      state.shouldError = shouldError
      if (message) state.errorMessage = message
    },

    /**
     * Set simulated latency
     */
    setLatency: (ms: number) => {
      state.latencyMs = ms
    },

    /**
     * Set token usage to return
     */
    setTokenUsage: (prompt: number, completion: number) => {
      state.tokenUsage = { prompt, completion }
    },

    /**
     * Set tool calls to return
     */
    setToolCalls: (toolCalls: MockToolCall[]) => {
      state.toolCalls = toolCalls
    },

    /**
     * Get current token usage
     */
    getTokenUsage: () => ({ ...state.tokenUsage }),

    /**
     * Reset state to defaults
     */
    reset: () => {
      state.responses = ["Mock response"]
      state.currentResponseIndex = 0
      state.shouldError = false
      state.errorMessage = "Mock provider error"
      state.latencyMs = 0
      state.tokenUsage = { prompt: 10, completion: 20 }
      state.toolCalls = []
    },
  }
}

/**
 * Setup mock for Anthropic SDK
 */
export function setupAnthropicMock() {
  const mockProvider = createMockProvider()
  const controller = createMockProviderController()

  mock.module("@anthropic-ai/sdk", () => ({
    default: class MockAnthropic {
      messages = {
        create: async (options: { messages: Array<{ role: string; content: string }>; stream?: boolean }) => {
          const messages = options.messages.map((m) => ({
            role: m.role as "user" | "assistant",
            content: m.content,
          }))

          if (options.stream) {
            return {
              async *[Symbol.asyncIterator]() {
                for await (const chunk of mockProvider.streamText({ messages })) {
                  yield {
                    type: "content_block_delta",
                    delta: { type: "text_delta", text: chunk.textDelta },
                  }
                }
              },
            }
          }

          const result = await mockProvider.generateText({ messages })
          return {
            content: [{ type: "text", text: result.text }],
            usage: {
              input_tokens: result.usage.promptTokens,
              output_tokens: result.usage.completionTokens,
            },
          }
        },
      }
    },
  }))

  return controller
}

/**
 * Setup mock for OpenAI SDK
 */
export function setupOpenAIMock() {
  const mockProvider = createMockProvider()
  const controller = createMockProviderController()

  mock.module("openai", () => ({
    default: class MockOpenAI {
      chat = {
        completions: {
          create: async (options: {
            messages: Array<{ role: string; content: string }>
            stream?: boolean
          }) => {
            const messages = options.messages.map((m) => ({
              role: m.role as "user" | "assistant",
              content: m.content,
            }))

            if (options.stream) {
              return {
                async *[Symbol.asyncIterator]() {
                  for await (const chunk of mockProvider.streamText({ messages })) {
                    yield {
                      choices: [{ delta: { content: chunk.textDelta } }],
                    }
                  }
                },
              }
            }

            const result = await mockProvider.generateText({ messages })
            return {
              choices: [{ message: { content: result.text } }],
              usage: {
                prompt_tokens: result.usage.promptTokens,
                completion_tokens: result.usage.completionTokens,
              },
            }
          },
        },
      }
    },
  }))

  return controller
}
