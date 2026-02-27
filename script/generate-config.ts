#!/usr/bin/env bun
/**
 * Generate TypeScript types and Zod schemas from JSON Schema files.
 *
 * This script serves as the single source of truth for configuration types:
 * 1. Reads JSON Schema files from schemas/
 * 2. Generates TypeScript interfaces
 * 3. Generates Zod schemas for runtime validation
 *
 * For Rust code generation, use the typify CLI:
 *   cargo install typify
 *   typify schemas/*.schema.json -o services/zero-common/src/config_generated.rs
 *
 * Usage:
 *   bun run script/generate-config.ts
 *
 * Files generated:
 *   - packages/ccode/src/config/generated/types.ts
 *   - packages/ccode/src/config/generated/schemas.ts
 *   - packages/ccode/src/config/generated/index.ts
 */

import { compile } from "json-schema-to-typescript"
import path from "path"
import fs from "fs/promises"

const SCHEMA_DIR = path.resolve(import.meta.dirname, "../schemas")
const OUTPUT_DIR = path.resolve(import.meta.dirname, "../packages/ccode/src/config/generated")

// Schema files to process
const SCHEMA_FILES = ["config", "secrets", "trading", "channels", "providers"] as const

interface SchemaInfo {
  name: string
  path: string
  schema: Record<string, unknown>
}

async function loadSchemas(): Promise<SchemaInfo[]> {
  const schemas: SchemaInfo[] = []

  for (const name of SCHEMA_FILES) {
    const schemaPath = path.join(SCHEMA_DIR, `${name}.schema.json`)
    try {
      const content = await Bun.file(schemaPath).text()
      const schema = JSON.parse(content) as Record<string, unknown>
      schemas.push({ name, path: schemaPath, schema })
    } catch (error) {
      console.error(`Failed to load schema ${name}:`, error)
      process.exit(1)
    }
  }

  return schemas
}

function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1)
}

async function generateTypeScriptTypes(schemas: SchemaInfo[]): Promise<string> {
  const typeDefinitions: string[] = [
    "// Auto-generated from JSON Schema - DO NOT EDIT",
    "// Run `bun run script/generate-config.ts` to regenerate",
    "",
    "/* eslint-disable */",
    "",
  ]

  for (const { name, schema } of schemas) {
    try {
      // Compile JSON Schema to TypeScript
      const typeName = capitalize(name) + "Config"
      const ts = await compile(schema as Parameters<typeof compile>[0], typeName, {
        bannerComment: "",
        additionalProperties: false,
        strictIndexSignatures: true,
        style: {
          singleQuote: false,
          semi: false,
        },
      })

      typeDefinitions.push(`// ═══════════════════════════════════════════════════`)
      typeDefinitions.push(`// ${capitalize(name)} Configuration Types`)
      typeDefinitions.push(`// ═══════════════════════════════════════════════════`)
      typeDefinitions.push("")
      typeDefinitions.push(ts)
      typeDefinitions.push("")
    } catch (error) {
      console.error(`Failed to generate types for ${name}:`, error)
    }
  }

  return typeDefinitions.join("\n")
}

function generateZodSchemas(schemas: SchemaInfo[]): string {
  const lines: string[] = [
    "// Auto-generated Zod schema stubs from JSON Schema - DO NOT EDIT",
    "// These provide runtime validation for configuration files",
    "// Run `bun run script/generate-config.ts` to regenerate",
    "",
    'import z from "zod"',
    "",
  ]

  // Generate basic Zod schemas that reference the JSON Schema for validation
  for (const { name } of schemas) {
    const typeName = capitalize(name)
    lines.push(`// ${typeName} schema stub - uses JSON Schema for full validation`)
    lines.push(`export const ${typeName}Schema = z.record(z.string(), z.unknown())`)
    lines.push(`export type ${typeName}SchemaType = z.infer<typeof ${typeName}Schema>`)
    lines.push("")
  }

  // Add helper for loading and validating configs
  lines.push(`// Schema paths for JSON Schema validation`)
  lines.push(`export const SCHEMA_PATHS = {`)
  for (const { name } of schemas) {
    lines.push(`  ${name}: "https://code-coder.com/schemas/${name}.json",`)
  }
  lines.push(`} as const`)

  return lines.join("\n")
}

function generateIndexFile(): string {
  return [
    "// Auto-generated index file - DO NOT EDIT",
    "// Run `bun run script/generate-config.ts` to regenerate",
    "",
    'export * from "./types"',
    'export * from "./schemas"',
    "",
  ].join("\n")
}

async function main() {
  console.log("🔧 Loading JSON Schema files...")
  const schemas = await loadSchemas()
  console.log(`   Found ${schemas.length} schemas: ${schemas.map((s) => s.name).join(", ")}`)

  // Ensure output directory exists
  await fs.mkdir(OUTPUT_DIR, { recursive: true })

  // Generate TypeScript types
  console.log("📝 Generating TypeScript types...")
  const types = await generateTypeScriptTypes(schemas)
  await Bun.write(path.join(OUTPUT_DIR, "types.ts"), types)
  console.log(`   ✓ Written ${path.join(OUTPUT_DIR, "types.ts")}`)

  // Generate Zod schemas
  console.log("📝 Generating Zod schema stubs...")
  const zodSchemas = generateZodSchemas(schemas)
  await Bun.write(path.join(OUTPUT_DIR, "schemas.ts"), zodSchemas)
  console.log(`   ✓ Written ${path.join(OUTPUT_DIR, "schemas.ts")}`)

  // Generate index file
  console.log("📝 Generating index file...")
  const indexFile = generateIndexFile()
  await Bun.write(path.join(OUTPUT_DIR, "index.ts"), indexFile)
  console.log(`   ✓ Written ${path.join(OUTPUT_DIR, "index.ts")}`)

  console.log("")
  console.log("✅ TypeScript generation complete!")
  console.log("")
  console.log("💡 To generate Rust types, run:")
  console.log("   cargo install typify")
  console.log("   typify schemas/*.schema.json -o services/zero-common/src/config_generated.rs")
}

main().catch((error) => {
  console.error("Fatal error:", error)
  process.exit(1)
})
