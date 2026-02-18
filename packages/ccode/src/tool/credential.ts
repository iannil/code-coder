/**
 * GetCredential Tool
 *
 * Allows agents to retrieve credentials from the secure vault.
 * This enables agents to use up-to-date credentials instead of relying on
 * potentially outdated information from conversation history.
 */

import z from "zod"
import { Tool } from "./tool"
import { CredentialVault, type CredentialEntry } from "@/credential/vault"

const Parameters = z.object({
  service: z
    .string()
    .describe("The service name to look up credentials for (e.g., 'github', 'ASDC_OA')"),
  credential_id: z
    .string()
    .optional()
    .describe("Optional: specific credential ID if known"),
})

interface Metadata {
  credentialId: string
  service: string
  type: string
  hasPassword: boolean
  hasUsername: boolean
  hasApiKey: boolean
}

function formatCredentialOutput(credential: CredentialEntry): string {
  const lines: string[] = []

  lines.push(`Credential: ${credential.name}`)
  lines.push(`Service: ${credential.service}`)
  lines.push(`Type: ${credential.type}`)
  lines.push(`ID: ${credential.id}`)

  if (credential.patterns.length > 0) {
    lines.push(`URL Patterns: ${credential.patterns.join(", ")}`)
  }

  switch (credential.type) {
    case "api_key":
    case "bearer_token":
      if (credential.apiKey) {
        lines.push(``)
        lines.push(`API Key / Token: ${credential.apiKey}`)
      }
      break

    case "oauth":
      if (credential.oauth) {
        lines.push(``)
        lines.push(`Client ID: ${credential.oauth.clientId}`)
        if (credential.oauth.accessToken) {
          lines.push(`Access Token: ${credential.oauth.accessToken}`)
        }
        if (credential.oauth.scope) {
          lines.push(`Scope: ${credential.oauth.scope}`)
        }
      }
      break

    case "login":
      if (credential.login) {
        lines.push(``)
        lines.push(`Username: ${credential.login.username}`)
        lines.push(`Password: ${credential.login.password}`)
        if (credential.login.totpSecret) {
          lines.push(`TOTP Secret: ${credential.login.totpSecret} (use for 2FA code generation)`)
        }
      }
      break
  }

  return lines.join("\n")
}

export const GetCredentialTool = Tool.define<typeof Parameters, Metadata>("GetCredential", {
  description: `Retrieve credentials from the secure vault for automated login or API authentication.

Use this tool when you need to:
- Log into a website (get username/password)
- Authenticate with an API (get API key or token)
- Fill in authentication forms

IMPORTANT: Always use this tool to get current credentials instead of using credentials from conversation history, as they may be outdated.

The tool returns:
- For login credentials: username, password, and optional TOTP secret
- For API keys: the API key
- For OAuth: client ID and access token
- For bearer tokens: the token

Example usage:
- Get credentials for a service: { "service": "github" }
- Get specific credential by ID: { "credential_id": "cred_abc123" }`,

  parameters: Parameters,

  async execute(args, ctx) {
    const vault = await CredentialVault.load()

    let credential: CredentialEntry | undefined

    if (args.credential_id) {
      credential = vault.get(args.credential_id)
      if (!credential) {
        return {
          title: "Credential not found",
          metadata: {
            credentialId: args.credential_id,
            service: args.service,
            type: "unknown",
            hasPassword: false,
            hasUsername: false,
            hasApiKey: false,
          },
          output: `No credential found with ID: ${args.credential_id}`,
        }
      }
    } else {
      credential = vault.getByService(args.service)
      if (!credential) {
        // List available credentials to help the agent
        const available = vault.list()
        const suggestions = available.map((c) => `- ${c.name} (service: ${c.service}, type: ${c.type})`).join("\n")

        return {
          title: "Credential not found",
          metadata: {
            credentialId: "",
            service: args.service,
            type: "unknown",
            hasPassword: false,
            hasUsername: false,
            hasApiKey: false,
          },
          output: `No credential found for service: ${args.service}\n\nAvailable credentials:\n${suggestions || "No credentials stored"}`,
        }
      }
    }

    const output = formatCredentialOutput(credential)

    return {
      title: `Credential: ${credential.name}`,
      metadata: {
        credentialId: credential.id,
        service: credential.service,
        type: credential.type,
        hasPassword: !!credential.login?.password,
        hasUsername: !!credential.login?.username,
        hasApiKey: !!credential.apiKey,
      },
      output,
    }
  },
})
