#!/usr/bin/env bun
/**
 * NAPI Type Definition Sync Script
 *
 * This script merges NAPI-generated types from services/zero-core/index.d.ts
 * into packages/core/src/binding.d.ts, preserving any manual types that
 * haven't been implemented in Rust yet.
 *
 * Usage:
 *   bun scripts/sync-napi-types.ts [--dry-run] [--verbose]
 *
 * The merge strategy:
 * 1. Parse both files to extract type declarations
 * 2. Use NAPI-generated types as authoritative (they reflect actual Rust impl)
 * 3. Preserve manual types that don't exist in NAPI output (forward declarations)
 * 4. Fix TypeScript reserved keywords (extends, interface) in parameter names
 */

import { readFileSync, writeFileSync, existsSync } from "fs"
import { join } from "path"

const projectRoot = join(import.meta.dir, "..")
const napiSource = join(projectRoot, "services/zero-core/index.d.ts")
const bindingTarget = join(projectRoot, "packages/core/src/binding.d.ts")

interface TypeDeclaration {
  name: string
  kind: "class" | "interface" | "type" | "function" | "const" | "enum"
  content: string
  startLine: number
  endLine: number
}

function parseTypeDeclarations(content: string): TypeDeclaration[] {
  const lines = content.split("\n")
  const declarations: TypeDeclaration[] = []

  let currentDecl: Partial<TypeDeclaration> | null = null
  let braceDepth = 0
  let startLine = 0

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]

    // Start of new declaration
    if (!currentDecl) {
      const classMatch = line.match(/^export declare class (\w+)/)
      const interfaceMatch = line.match(/^export interface (\w+)/)
      const typeMatch = line.match(/^export type (\w+)/)
      const functionMatch = line.match(/^export declare function (\w+)/)
      const constMatch = line.match(/^export declare const (\w+)/)
      const enumMatch = line.match(/^export declare enum (\w+)/)

      if (classMatch) {
        currentDecl = { name: classMatch[1], kind: "class", content: line }
        startLine = i
        braceDepth = (line.match(/{/g) || []).length - (line.match(/}/g) || []).length
      } else if (interfaceMatch) {
        currentDecl = { name: interfaceMatch[1], kind: "interface", content: line }
        startLine = i
        braceDepth = (line.match(/{/g) || []).length - (line.match(/}/g) || []).length
      } else if (typeMatch) {
        currentDecl = { name: typeMatch[1], kind: "type", content: line }
        startLine = i
        // Types can be single-line or multi-line
        if (!line.includes("{") || line.includes("}")) {
          declarations.push({
            ...currentDecl,
            content: line,
            startLine: i,
            endLine: i,
          } as TypeDeclaration)
          currentDecl = null
        } else {
          braceDepth = (line.match(/{/g) || []).length - (line.match(/}/g) || []).length
        }
      } else if (functionMatch) {
        // Check if function declaration spans multiple lines (ends with ): or ) =>)
        const isComplete = line.includes("):") || line.includes("): ") || /\):\s*\w/.test(line)
        if (isComplete) {
          declarations.push({
            name: functionMatch[1],
            kind: "function",
            content: line,
            startLine: i,
            endLine: i,
          })
        } else {
          // Multi-line function declaration
          currentDecl = { name: functionMatch[1], kind: "function", content: line }
          startLine = i
          // Track parentheses to find end of function signature
          braceDepth = (line.match(/\(/g) || []).length - (line.match(/\)/g) || []).length
        }
      } else if (constMatch) {
        declarations.push({
          name: constMatch[1],
          kind: "const",
          content: line,
          startLine: i,
          endLine: i,
        })
      } else if (enumMatch) {
        currentDecl = { name: enumMatch[1], kind: "enum", content: line }
        startLine = i
        braceDepth = (line.match(/{/g) || []).length - (line.match(/}/g) || []).length
      }
    } else {
      // Continue existing declaration
      currentDecl.content += "\n" + line

      if (currentDecl.kind === "function") {
        // For functions, track parentheses
        braceDepth += (line.match(/\(/g) || []).length - (line.match(/\)/g) || []).length
        // Function declaration ends when parentheses are balanced and we have a return type
        if (braceDepth <= 0 && (line.includes("):") || /\):\s*\w/.test(line) || line.includes("): "))) {
          declarations.push({
            ...currentDecl,
            startLine,
            endLine: i,
          } as TypeDeclaration)
          currentDecl = null
          braceDepth = 0
        }
      } else {
        // For classes, interfaces, enums - track braces
        braceDepth += (line.match(/{/g) || []).length - (line.match(/}/g) || []).length

        if (braceDepth <= 0) {
          declarations.push({
            ...currentDecl,
            startLine,
            endLine: i,
          } as TypeDeclaration)
          currentDecl = null
          braceDepth = 0
        }
      }
    }
  }

  return declarations
}

function fixReservedKeywords(content: string): string {
  // Fix TypeScript reserved keywords used as parameter names
  return content
    .replace(/\bextends:/g, "extendsFrom:")
    .replace(/\bextends\?:/g, "extendsFrom?:")
    .replace(/\binterface:/g, "interfaceName:")
    .replace(/\binterface\?:/g, "interfaceName?:")
}

function main() {
  const args = process.argv.slice(2)
  const dryRun = args.includes("--dry-run")
  const verbose = args.includes("--verbose")

  if (!existsSync(napiSource)) {
    console.error(`❌ NAPI source not found: ${napiSource}`)
    console.error("   Run './ops.sh build rust' first to generate types.")
    process.exit(1)
  }

  if (!existsSync(bindingTarget)) {
    console.error(`❌ Binding target not found: ${bindingTarget}`)
    process.exit(1)
  }

  console.log("🔍 Analyzing type definitions...")

  const napiContent = readFileSync(napiSource, "utf-8")
  const bindingContent = readFileSync(bindingTarget, "utf-8")

  const napiDecls = parseTypeDeclarations(napiContent)
  const bindingDecls = parseTypeDeclarations(bindingContent)

  const napiNames = new Set(napiDecls.map((d) => `${d.kind}:${d.name}`))
  const bindingNames = new Set(bindingDecls.map((d) => `${d.kind}:${d.name}`))

  // Types in NAPI but not in binding (new Rust implementations)
  const newInNapi = napiDecls.filter((d) => !bindingNames.has(`${d.kind}:${d.name}`))

  // Types in binding but not in NAPI (manual forward declarations)
  const manualOnly = bindingDecls.filter((d) => !napiNames.has(`${d.kind}:${d.name}`))

  // Types in both (use NAPI version as authoritative)
  const shared = napiDecls.filter((d) => bindingNames.has(`${d.kind}:${d.name}`))

  console.log(`\n📊 Analysis Results:`)
  console.log(`   NAPI types:     ${napiDecls.length}`)
  console.log(`   Binding types:  ${bindingDecls.length}`)
  console.log(`   New in NAPI:    ${newInNapi.length}`)
  console.log(`   Manual only:    ${manualOnly.length}`)
  console.log(`   Shared:         ${shared.length}`)

  if (verbose) {
    if (newInNapi.length > 0) {
      console.log("\n   📥 New types from NAPI:")
      for (const d of newInNapi) {
        console.log(`      - ${d.kind} ${d.name}`)
      }
    }

    if (manualOnly.length > 0) {
      console.log("\n   📝 Manual types (not in Rust yet):")
      for (const d of manualOnly) {
        console.log(`      - ${d.kind} ${d.name}`)
      }
    }
  }

  // Build merged content
  const header = `/* auto-generated by NAPI-RS + manual forward declarations */
/* Synced from services/zero-core/index.d.ts on ${new Date().toISOString().split("T")[0]} */
/* Run 'bun scripts/sync-napi-types.ts' to update */
/* eslint-disable */
`

  // Start with NAPI content (authoritative)
  let mergedContent = fixReservedKeywords(napiContent)

  // Append manual-only declarations
  if (manualOnly.length > 0) {
    mergedContent += "\n\n// ============================================================================"
    mergedContent += "\n// Manual Forward Declarations (not yet implemented in Rust)"
    mergedContent += "\n// ============================================================================\n"

    for (const decl of manualOnly) {
      mergedContent += "\n" + decl.content + "\n"
    }
  }

  // Replace header
  mergedContent = mergedContent.replace(/^\/\* auto-generated by NAPI-RS \*\/\n\/\* eslint-disable \*\//, header)

  if (dryRun) {
    console.log("\n🔄 Dry run - would write merged content to:")
    console.log(`   ${bindingTarget}`)
    console.log(`\n   Total lines: ${mergedContent.split("\n").length}`)
  } else {
    writeFileSync(bindingTarget, mergedContent)
    console.log(`\n✅ Types synced to ${bindingTarget}`)
    console.log(`   Total lines: ${mergedContent.split("\n").length}`)
  }
}

main()
