// Auto-generated Zod schema stubs from JSON Schema - DO NOT EDIT
// These provide runtime validation for configuration files
// Run `bun run script/generate-config.ts` to regenerate

import z from "zod"

// Config schema stub - uses JSON Schema for full validation
export const ConfigSchema = z.record(z.string(), z.unknown())
export type ConfigSchemaType = z.infer<typeof ConfigSchema>

// Secrets schema stub - uses JSON Schema for full validation
export const SecretsSchema = z.record(z.string(), z.unknown())
export type SecretsSchemaType = z.infer<typeof SecretsSchema>

// Trading schema stub - uses JSON Schema for full validation
export const TradingSchema = z.record(z.string(), z.unknown())
export type TradingSchemaType = z.infer<typeof TradingSchema>

// Channels schema stub - uses JSON Schema for full validation
export const ChannelsSchema = z.record(z.string(), z.unknown())
export type ChannelsSchemaType = z.infer<typeof ChannelsSchema>

// Providers schema stub - uses JSON Schema for full validation
export const ProvidersSchema = z.record(z.string(), z.unknown())
export type ProvidersSchemaType = z.infer<typeof ProvidersSchema>

// Schema paths for JSON Schema validation
export const SCHEMA_PATHS = {
  config: "https://code-coder.com/schemas/config.json",
  secrets: "https://code-coder.com/schemas/secrets.json",
  trading: "https://code-coder.com/schemas/trading.json",
  channels: "https://code-coder.com/schemas/channels.json",
  providers: "https://code-coder.com/schemas/providers.json",
} as const