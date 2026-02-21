/**
 * Prompt Template System
 *
 * Provides a flexible template system for generating prompts with:
 * - Variable substitution (Mustache/Handlebars style)
 * - Template inheritance and composition
 * - Category-based organization
 * - Built-in templates for common use cases
 */

import z from "zod"
import Handlebars from "handlebars"
import { Global } from "@/global"
import path from "path"

// ─────────────────────────────────────────────────────────────────────────────
// Schema Definitions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Template category for organization.
 */
export const TemplateCategory = z.enum([
  "document", // Documentation generation
  "review", // Code review reports
  "analysis", // Analysis reports
  "planning", // Implementation plans
  "communication", // Emails, messages, etc.
  "custom", // User-defined templates
])
export type TemplateCategory = z.infer<typeof TemplateCategory>

/**
 * Template variable definition.
 */
export const TemplateVariable = z.object({
  /** Variable name */
  name: z.string(),
  /** Description of the variable */
  description: z.string(),
  /** Whether this variable is required */
  required: z.boolean().default(true),
  /** Default value if not provided */
  defaultValue: z.string().optional(),
  /** Example value for documentation */
  example: z.string().optional(),
  /** Variable type for validation */
  type: z.enum(["string", "number", "boolean", "array", "object"]).optional().default("string"),
})
export type TemplateVariable = z.infer<typeof TemplateVariable>

/**
 * Prompt template metadata.
 */
export const TemplateMetadata = z.object({
  /** Template unique identifier */
  id: z.string(),
  /** Human-readable name */
  name: z.string(),
  /** Description */
  description: z.string(),
  /** Category */
  category: TemplateCategory,
  /** Template content (Handlebars syntax) */
  content: z.string(),
  /** Variables used in this template */
  variables: z.array(TemplateVariable).default([]),
  /** Parent template ID for inheritance */
  extends: z.string().optional(),
  /** Tags for search */
  tags: z.array(z.string()).default([]),
  /** Author */
  author: z.string().optional(),
  /** Version */
  version: z.string().default("1.0.0"),
  /** Whether this is a built-in template */
  builtin: z.boolean().default(false),
})
export type TemplateMetadata = z.infer<typeof TemplateMetadata>

/**
 * Rendered template result.
 */
export interface RenderResult {
  content: string
  variables: Record<string, unknown>
  template: TemplateMetadata
}

// ─────────────────────────────────────────────────────────────────────────────
// Built-in Templates
// ─────────────────────────────────────────────────────────────────────────────

type TemplateInput = z.input<typeof TemplateMetadata>

const BUILTIN_TEMPLATES: TemplateInput[] = [
  // ── Document Generation ────────────────────────────────────────────────────
  {
    id: "prd-template",
    name: "PRD (Product Requirements Document)",
    description: "Generate a comprehensive product requirements document",
    category: "document",
    content: `# {{title}}

## Overview
{{overview}}

## Problem Statement
{{problem}}

## Goals & Success Metrics
{{#each goals}}
- {{this}}
{{/each}}

## User Stories
{{#each userStories}}
### {{this.title}}
**As a** {{this.actor}}, **I want** {{this.action}}, **so that** {{this.benefit}}.

**Acceptance Criteria:**
{{#each this.criteria}}
- {{this}}
{{/each}}
{{/each}}

## Technical Requirements
{{technicalRequirements}}

## Non-Functional Requirements
- **Performance:** {{nfr.performance}}
- **Security:** {{nfr.security}}
- **Scalability:** {{nfr.scalability}}

## Timeline
{{timeline}}

## Risks & Mitigations
{{#each risks}}
- **Risk:** {{this.risk}}
  **Mitigation:** {{this.mitigation}}
{{/each}}

## Appendix
{{appendix}}`,
    variables: [
      { name: "title", description: "Document title", required: true, example: "User Authentication System" },
      { name: "overview", description: "Brief overview of the product", required: true },
      { name: "problem", description: "Problem being solved", required: true },
      { name: "goals", description: "List of goals", required: true, type: "array" },
      { name: "userStories", description: "User stories", required: true, type: "array" },
      { name: "technicalRequirements", description: "Technical requirements", required: false },
      { name: "nfr", description: "Non-functional requirements object", required: false, type: "object" },
      { name: "timeline", description: "Timeline information", required: false },
      { name: "risks", description: "Risks and mitigations", required: false, type: "array" },
      { name: "appendix", description: "Additional information", required: false },
    ],
    tags: ["product", "requirements", "planning"],
    builtin: true,
    version: "1.0.0",
  },
  {
    id: "api-doc-template",
    name: "API Documentation",
    description: "Generate API endpoint documentation",
    category: "document",
    content: `# {{endpoint}} API

## Overview
{{description}}

## Endpoint
\`{{method}} {{path}}\`

## Authentication
{{#if auth}}
{{auth}}
{{else}}
No authentication required.
{{/if}}

## Request
{{#if requestBody}}
### Request Body
\`\`\`json
{{requestBody}}
\`\`\`
{{/if}}

{{#if queryParams}}
### Query Parameters
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
{{#each queryParams}}
| {{this.name}} | {{this.type}} | {{this.required}} | {{this.description}} |
{{/each}}
{{/if}}

## Response
### Success ({{successCode}})
\`\`\`json
{{successResponse}}
\`\`\`

{{#if errorResponses}}
### Error Responses
{{#each errorResponses}}
#### {{this.code}} - {{this.name}}
\`\`\`json
{{this.body}}
\`\`\`
{{/each}}
{{/if}}

## Example
\`\`\`bash
{{curlExample}}
\`\`\``,
    variables: [
      { name: "endpoint", description: "API endpoint name", required: true, example: "Create User" },
      { name: "description", description: "Endpoint description", required: true },
      { name: "method", description: "HTTP method", required: true, example: "POST" },
      { name: "path", description: "API path", required: true, example: "/api/v1/users" },
      { name: "auth", description: "Authentication details", required: false },
      { name: "requestBody", description: "Request body JSON", required: false },
      { name: "queryParams", description: "Query parameters", required: false, type: "array" },
      { name: "successCode", description: "Success status code", required: true, defaultValue: "200" },
      { name: "successResponse", description: "Success response JSON", required: true },
      { name: "errorResponses", description: "Error responses", required: false, type: "array" },
      { name: "curlExample", description: "Curl example", required: false },
    ],
    tags: ["api", "documentation", "rest"],
    builtin: true,
    version: "1.0.0",
  },

  // ── Code Review ────────────────────────────────────────────────────────────
  {
    id: "code-review-template",
    name: "Code Review Report",
    description: "Generate a structured code review report",
    category: "review",
    content: `# Code Review Report

## Summary
**Files Reviewed:** {{filesCount}}
**Total Issues:** {{issuesCount}}
**Overall Rating:** {{rating}}/5

## Overview
{{overview}}

## Critical Issues
{{#each criticalIssues}}
### 🔴 {{this.title}}
**File:** \`{{this.file}}\` (Line {{this.line}})
**Description:** {{this.description}}
**Recommendation:** {{this.recommendation}}
{{/each}}

## Major Issues
{{#each majorIssues}}
### 🟠 {{this.title}}
**File:** \`{{this.file}}\` (Line {{this.line}})
**Description:** {{this.description}}
**Recommendation:** {{this.recommendation}}
{{/each}}

## Minor Issues
{{#each minorIssues}}
### 🟡 {{this.title}}
**File:** \`{{this.file}}\` (Line {{this.line}})
**Description:** {{this.description}}
{{/each}}

## Positive Observations
{{#each positives}}
- ✅ {{this}}
{{/each}}

## Recommendations
{{recommendations}}`,
    variables: [
      { name: "filesCount", description: "Number of files reviewed", required: true, type: "number" },
      { name: "issuesCount", description: "Total issues found", required: true, type: "number" },
      { name: "rating", description: "Overall rating (1-5)", required: true, type: "number" },
      { name: "overview", description: "Review overview", required: true },
      { name: "criticalIssues", description: "Critical issues list", required: false, type: "array" },
      { name: "majorIssues", description: "Major issues list", required: false, type: "array" },
      { name: "minorIssues", description: "Minor issues list", required: false, type: "array" },
      { name: "positives", description: "Positive observations", required: false, type: "array" },
      { name: "recommendations", description: "Final recommendations", required: false },
    ],
    tags: ["review", "code", "quality"],
    builtin: true,
    version: "1.0.0",
  },
  {
    id: "security-review-template",
    name: "Security Review Report",
    description: "Generate a security-focused code review report",
    category: "review",
    content: `# Security Review Report

## Executive Summary
**Risk Level:** {{riskLevel}}
**Vulnerabilities Found:** {{vulnerabilityCount}}
**Compliance Status:** {{complianceStatus}}

## Scope
{{scope}}

## Vulnerabilities

{{#each vulnerabilities}}
### {{this.severity}}: {{this.title}}
**CWE:** {{this.cwe}}
**CVSS:** {{this.cvss}}
**Location:** \`{{this.file}}:{{this.line}}\`

**Description:**
{{this.description}}

**Proof of Concept:**
\`\`\`
{{this.poc}}
\`\`\`

**Remediation:**
{{this.remediation}}

---
{{/each}}

## OWASP Top 10 Coverage
| Category | Status | Notes |
|----------|--------|-------|
{{#each owaspChecks}}
| {{this.category}} | {{this.status}} | {{this.notes}} |
{{/each}}

## Recommendations
{{recommendations}}

## Next Steps
{{nextSteps}}`,
    variables: [
      { name: "riskLevel", description: "Overall risk level", required: true, example: "HIGH" },
      { name: "vulnerabilityCount", description: "Number of vulnerabilities", required: true, type: "number" },
      { name: "complianceStatus", description: "Compliance status", required: false },
      { name: "scope", description: "Review scope description", required: true },
      { name: "vulnerabilities", description: "List of vulnerabilities", required: false, type: "array" },
      { name: "owaspChecks", description: "OWASP Top 10 checklist", required: false, type: "array" },
      { name: "recommendations", description: "Security recommendations", required: true },
      { name: "nextSteps", description: "Next steps for remediation", required: false },
    ],
    tags: ["security", "review", "vulnerability"],
    builtin: true,
    version: "1.0.0",
  },

  // ── Analysis ───────────────────────────────────────────────────────────────
  {
    id: "data-insight-template",
    name: "Data Insight Report",
    description: "Generate a data analysis insight report",
    category: "analysis",
    content: `# {{title}}

## Key Findings
{{#each keyFindings}}
{{@index}}. {{this}}
{{/each}}

## Data Overview
{{dataOverview}}

## Analysis

### Trends
{{trends}}

### Patterns
{{patterns}}

### Anomalies
{{#if anomalies}}
{{#each anomalies}}
- **{{this.metric}}:** {{this.description}}
{{/each}}
{{else}}
No significant anomalies detected.
{{/if}}

## Visualizations
{{#each visualizations}}
### {{this.title}}
{{this.description}}
{{/each}}

## Recommendations
{{#each recommendations}}
- {{this}}
{{/each}}

## Methodology
{{methodology}}

## Limitations
{{limitations}}`,
    variables: [
      { name: "title", description: "Report title", required: true },
      { name: "keyFindings", description: "Key findings list", required: true, type: "array" },
      { name: "dataOverview", description: "Data overview", required: true },
      { name: "trends", description: "Trend analysis", required: true },
      { name: "patterns", description: "Pattern analysis", required: true },
      { name: "anomalies", description: "Anomalies found", required: false, type: "array" },
      { name: "visualizations", description: "Visualization descriptions", required: false, type: "array" },
      { name: "recommendations", description: "Recommendations", required: true, type: "array" },
      { name: "methodology", description: "Analysis methodology", required: false },
      { name: "limitations", description: "Analysis limitations", required: false },
    ],
    tags: ["data", "analysis", "insight"],
    builtin: true,
    version: "1.0.0",
  },

  // ── Planning ───────────────────────────────────────────────────────────────
  {
    id: "implementation-plan-template",
    name: "Implementation Plan",
    description: "Generate a technical implementation plan",
    category: "planning",
    content: `# Implementation Plan: {{title}}

## Overview
{{overview}}

## Goals
{{#each goals}}
- {{this}}
{{/each}}

## Phases

{{#each phases}}
### Phase {{@index}}: {{this.name}}

**Objective:** {{this.objective}}

**Tasks:**
{{#each this.tasks}}
- [ ] {{this.name}}{{#if this.assignee}} ({{this.assignee}}){{/if}}
{{/each}}

**Deliverables:**
{{#each this.deliverables}}
- {{this}}
{{/each}}

**Dependencies:**
{{#each this.dependencies}}
- {{this}}
{{/each}}

---
{{/each}}

## Technical Approach
{{technicalApproach}}

## Testing Strategy
{{testingStrategy}}

## Rollout Strategy
{{rolloutStrategy}}

## Risk Assessment
| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
{{#each risks}}
| {{this.risk}} | {{this.impact}} | {{this.probability}} | {{this.mitigation}} |
{{/each}}

## Success Criteria
{{#each successCriteria}}
- {{this}}
{{/each}}`,
    variables: [
      { name: "title", description: "Plan title", required: true },
      { name: "overview", description: "Plan overview", required: true },
      { name: "goals", description: "Goals list", required: true, type: "array" },
      { name: "phases", description: "Implementation phases", required: true, type: "array" },
      { name: "technicalApproach", description: "Technical approach", required: true },
      { name: "testingStrategy", description: "Testing strategy", required: true },
      { name: "rolloutStrategy", description: "Rollout strategy", required: false },
      { name: "risks", description: "Risk assessment", required: false, type: "array" },
      { name: "successCriteria", description: "Success criteria", required: true, type: "array" },
    ],
    tags: ["planning", "implementation", "technical"],
    builtin: true,
    version: "1.0.0",
  },

  // ── Communication ──────────────────────────────────────────────────────────
  {
    id: "release-notes-template",
    name: "Release Notes",
    description: "Generate release notes for a software release",
    category: "communication",
    content: `# Release Notes - v{{version}}

**Release Date:** {{releaseDate}}

## Highlights
{{highlights}}

## New Features
{{#each features}}
- **{{this.title}}**: {{this.description}}
{{/each}}

## Improvements
{{#each improvements}}
- {{this}}
{{/each}}

## Bug Fixes
{{#each bugFixes}}
- Fixed: {{this}}
{{/each}}

## Breaking Changes
{{#if breakingChanges}}
⚠️ **The following changes may require action:**
{{#each breakingChanges}}
- {{this}}
{{/each}}
{{else}}
No breaking changes in this release.
{{/if}}

## Deprecations
{{#if deprecations}}
{{#each deprecations}}
- {{this}}
{{/each}}
{{else}}
No deprecations in this release.
{{/if}}

## Upgrade Guide
{{upgradeGuide}}

## Known Issues
{{#if knownIssues}}
{{#each knownIssues}}
- {{this}}
{{/each}}
{{else}}
No known issues at this time.
{{/if}}

## Contributors
Thanks to all contributors who made this release possible!
{{#each contributors}}
- @{{this}}
{{/each}}`,
    variables: [
      { name: "version", description: "Version number", required: true, example: "2.0.0" },
      { name: "releaseDate", description: "Release date", required: true, example: "2026-02-21" },
      { name: "highlights", description: "Release highlights", required: true },
      { name: "features", description: "New features", required: false, type: "array" },
      { name: "improvements", description: "Improvements", required: false, type: "array" },
      { name: "bugFixes", description: "Bug fixes", required: false, type: "array" },
      { name: "breakingChanges", description: "Breaking changes", required: false, type: "array" },
      { name: "deprecations", description: "Deprecations", required: false, type: "array" },
      { name: "upgradeGuide", description: "Upgrade guide", required: false },
      { name: "knownIssues", description: "Known issues", required: false, type: "array" },
      { name: "contributors", description: "Contributors list", required: false, type: "array" },
    ],
    tags: ["release", "notes", "communication"],
    builtin: true,
    version: "1.0.0",
  },
]

// ─────────────────────────────────────────────────────────────────────────────
// Template Engine
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Prompt Template Engine.
 */
export class TemplateEngine {
  private templates: Map<string, TemplateMetadata> = new Map()
  private compiledCache: Map<string, HandlebarsTemplateDelegate> = new Map()

  constructor() {
    // Register Handlebars helpers
    this.registerHelpers()

    // Load built-in templates (parse through Zod to apply defaults)
    for (const template of BUILTIN_TEMPLATES) {
      const parsed = TemplateMetadata.parse(template)
      this.templates.set(parsed.id, parsed)
    }
  }

  /**
   * Register Handlebars helpers.
   */
  private registerHelpers(): void {
    // Conditional helpers
    Handlebars.registerHelper("ifEquals", function (arg1, arg2, options) {
      // @ts-expect-error - Handlebars context
      return arg1 === arg2 ? options.fn(this) : options.inverse(this)
    })

    Handlebars.registerHelper("ifNotEquals", function (arg1, arg2, options) {
      // @ts-expect-error - Handlebars context
      return arg1 !== arg2 ? options.fn(this) : options.inverse(this)
    })

    // String helpers
    Handlebars.registerHelper("uppercase", (str: string) => str?.toUpperCase() ?? "")
    Handlebars.registerHelper("lowercase", (str: string) => str?.toLowerCase() ?? "")
    Handlebars.registerHelper("capitalize", (str: string) => {
      if (!str) return ""
      return str.charAt(0).toUpperCase() + str.slice(1)
    })

    // Array helpers
    Handlebars.registerHelper("join", (arr: unknown[], separator: string) => {
      if (!Array.isArray(arr)) return ""
      return arr.join(typeof separator === "string" ? separator : ", ")
    })

    Handlebars.registerHelper("length", (arr: unknown[]) => {
      if (!Array.isArray(arr)) return 0
      return arr.length
    })

    // Date helpers
    Handlebars.registerHelper("now", () => new Date().toISOString().split("T")[0])
    Handlebars.registerHelper("year", () => new Date().getFullYear())

    // Code block helper
    Handlebars.registerHelper("codeBlock", (lang: string, code: string) => {
      return new Handlebars.SafeString(`\`\`\`${lang}\n${code}\n\`\`\``)
    })
  }

  /**
   * Register a new template.
   */
  register(template: TemplateMetadata): void {
    const validated = TemplateMetadata.parse(template)
    this.templates.set(validated.id, validated)
    this.compiledCache.delete(validated.id)
  }

  /**
   * Get a template by ID.
   */
  get(id: string): TemplateMetadata | undefined {
    return this.templates.get(id)
  }

  /**
   * List all templates.
   */
  list(): TemplateMetadata[] {
    return Array.from(this.templates.values())
  }

  /**
   * List templates by category.
   */
  listByCategory(category: TemplateCategory): TemplateMetadata[] {
    return this.list().filter((t) => t.category === category)
  }

  /**
   * Search templates.
   */
  search(query: string): TemplateMetadata[] {
    const lowercaseQuery = query.toLowerCase()
    return this.list().filter(
      (t) =>
        t.name.toLowerCase().includes(lowercaseQuery) ||
        t.description.toLowerCase().includes(lowercaseQuery) ||
        t.tags.some((tag) => tag.toLowerCase().includes(lowercaseQuery))
    )
  }

  /**
   * Render a template with variables.
   */
  render(templateId: string, variables: Record<string, unknown>): RenderResult {
    const template = this.templates.get(templateId)
    if (!template) {
      throw new Error(`Template not found: ${templateId}`)
    }

    // Handle template inheritance
    let content = template.content
    if (template.extends) {
      const parent = this.templates.get(template.extends)
      if (parent) {
        content = this.mergeTemplates(parent.content, content)
      }
    }

    // Compile template
    let compiled = this.compiledCache.get(templateId)
    if (!compiled) {
      compiled = Handlebars.compile(content, { noEscape: true })
      this.compiledCache.set(templateId, compiled)
    }

    // Validate required variables
    const missingVars = template.variables
      .filter((v) => v.required && !(v.name in variables))
      .map((v) => v.name)

    if (missingVars.length > 0) {
      throw new Error(`Missing required variables: ${missingVars.join(", ")}`)
    }

    // Apply default values
    const finalVariables: Record<string, unknown> = {}
    for (const v of template.variables) {
      if (v.name in variables) {
        finalVariables[v.name] = variables[v.name]
      } else if (v.defaultValue !== undefined) {
        finalVariables[v.name] = v.defaultValue
      }
    }

    // Render
    const rendered = compiled(finalVariables)

    return {
      content: rendered,
      variables: finalVariables,
      template,
    }
  }

  /**
   * Merge parent and child template contents.
   */
  private mergeTemplates(parent: string, child: string): string {
    // Simple merge: child content replaces {{> content}} in parent
    if (parent.includes("{{> content}}")) {
      return parent.replace("{{> content}}", child)
    }
    // If no placeholder, append child after parent
    return `${parent}\n\n${child}`
  }

  /**
   * Validate variables against template definition.
   */
  validate(templateId: string, variables: Record<string, unknown>): { valid: boolean; errors: string[] } {
    const template = this.templates.get(templateId)
    if (!template) {
      return { valid: false, errors: [`Template not found: ${templateId}`] }
    }

    const errors: string[] = []

    for (const v of template.variables) {
      const value = variables[v.name]

      // Check required
      if (v.required && value === undefined) {
        errors.push(`Missing required variable: ${v.name}`)
        continue
      }

      if (value === undefined) continue

      // Type check
      switch (v.type) {
        case "string":
          if (typeof value !== "string") {
            errors.push(`Variable ${v.name} must be a string`)
          }
          break
        case "number":
          if (typeof value !== "number") {
            errors.push(`Variable ${v.name} must be a number`)
          }
          break
        case "boolean":
          if (typeof value !== "boolean") {
            errors.push(`Variable ${v.name} must be a boolean`)
          }
          break
        case "array":
          if (!Array.isArray(value)) {
            errors.push(`Variable ${v.name} must be an array`)
          }
          break
        case "object":
          if (typeof value !== "object" || value === null || Array.isArray(value)) {
            errors.push(`Variable ${v.name} must be an object`)
          }
          break
      }
    }

    return { valid: errors.length === 0, errors }
  }

  /**
   * Generate example variables for a template.
   */
  generateExample(templateId: string): Record<string, unknown> {
    const template = this.templates.get(templateId)
    if (!template) {
      throw new Error(`Template not found: ${templateId}`)
    }

    const example: Record<string, unknown> = {}

    for (const v of template.variables) {
      if (v.example !== undefined) {
        example[v.name] = v.example
      } else if (v.defaultValue !== undefined) {
        example[v.name] = v.defaultValue
      } else {
        // Generate placeholder based on type
        switch (v.type) {
          case "string":
            example[v.name] = `[${v.name}]`
            break
          case "number":
            example[v.name] = 0
            break
          case "boolean":
            example[v.name] = true
            break
          case "array":
            example[v.name] = [`[${v.name} item 1]`, `[${v.name} item 2]`]
            break
          case "object":
            example[v.name] = { key: "value" }
            break
        }
      }
    }

    return example
  }

  /**
   * Remove a template.
   */
  remove(templateId: string): boolean {
    const template = this.templates.get(templateId)
    if (template?.builtin) {
      throw new Error("Cannot remove built-in templates")
    }
    this.compiledCache.delete(templateId)
    return this.templates.delete(templateId)
  }

  /**
   * Export templates to JSON.
   */
  toJSON(): TemplateMetadata[] {
    return this.list()
  }

  /**
   * Import templates from JSON.
   */
  fromJSON(templates: unknown[]): void {
    for (const t of templates) {
      try {
        const validated = TemplateMetadata.parse(t)
        if (!validated.builtin) {
          this.register(validated)
        }
      } catch (e) {
        console.warn("Failed to import template:", e)
      }
    }
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let engineInstance: TemplateEngine | null = null

/**
 * Get the global template engine instance.
 */
export function getTemplateEngine(): TemplateEngine {
  if (!engineInstance) {
    engineInstance = new TemplateEngine()
  }
  return engineInstance
}

/**
 * Reset the template engine (for testing).
 */
export function resetTemplateEngine(): void {
  engineInstance = null
}
