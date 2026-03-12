/**
 * Command API - Rust API Client Wrapper
 *
 * Provides slash command functionality via the Rust prompts API.
 * Commands are loaded from prompt templates.
 *
 * @module api/command
 */

import { getRustClient } from "./rust-client"

export namespace Command {
  export interface Info {
    name: string
    description?: string
    agent?: string
    model?: string
    template: Promise<string> | string
    subtask?: boolean
    hints: string[]
  }

  /**
   * Get a command by name
   */
  export async function get(name: string): Promise<Info | undefined> {
    const client = getRustClient()
    const response = await client.getPrompt(name)

    if (!response.success || !response.data) {
      return undefined
    }

    const content = response.data.content

    return {
      name,
      template: content,
      hints: extractHints(content),
    }
  }

  /**
   * List all available commands
   */
  export async function list(): Promise<Info[]> {
    const client = getRustClient()
    const response = await client.listPrompts()

    if (!response.success || !response.data) {
      return []
    }

    const commands: Info[] = []
    for (const prompt of response.data.prompts) {
      const detail = await client.getPrompt(prompt.name)
      if (detail.success && detail.data) {
        commands.push({
          name: prompt.name,
          template: detail.data.content,
          hints: extractHints(detail.data.content),
        })
      }
    }

    return commands
  }

  /**
   * Extract argument hints from template ($1, $2, $ARGUMENTS, etc.)
   */
  function extractHints(template: string): string[] {
    const result: string[] = []
    const numbered = template.match(/\$\d+/g)
    if (numbered) {
      for (const match of [...new Set(numbered)].sort()) {
        result.push(match)
      }
    }
    if (template.includes("$ARGUMENTS")) {
      result.push("$ARGUMENTS")
    }
    return result
  }
}
