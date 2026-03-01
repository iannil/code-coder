/**
 * Workflow Generator
 *
 * Generates WORKFLOW concepts - multi-step orchestrated processes.
 * Creates WORKFLOW.md files with step definitions and transitions.
 *
 * Risk Level: HIGH
 * Auto-approvable: NO (always requires human approval)
 *
 * @package autonomous/builder/generators
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Agent } from "@/agent/agent"
import path from "path"
import { nanoid } from "nanoid"

import type {
  ConceptType,
  ConceptGenerator,
  GeneratorInput,
  GeneratedConcept,
} from "../types"
import { getLLMSolver } from "../../execution/llm-solver"

const log = Log.create({ service: "autonomous.builder.generators.workflow" })

// ============================================================================
// Workflow Step Types
// ============================================================================

interface WorkflowStep {
  id: string
  name: string
  description: string
  agent: string
  condition?: string
  onSuccess?: string
  onFailure?: string
}

// ============================================================================
// Workflow Generator
// ============================================================================

export class WorkflowGenerator implements ConceptGenerator {
  readonly conceptType: ConceptType = "WORKFLOW"

  async generate(input: GeneratorInput): Promise<GeneratedConcept> {
    log.info("Generating WORKFLOW concept", {
      gapId: input.gap.id,
      description: input.gap.description.slice(0, 100),
    })

    // Generate unique identifier
    const identifier = this.generateIdentifier(input)

    // Infer workflow steps
    const steps = await this.inferSteps(input)

    // Generate WORKFLOW.md content
    const workflowContent = this.generateWorkflowContent({
      identifier,
      displayName: this.toDisplayName(identifier),
      description: input.gap.description,
      steps,
      technology: input.gap.technology,
    })

    // Determine target path
    const targetDir = path.join(Instance.worktree, ".codecoder", "workflows", identifier)
    const targetPath = path.join(targetDir, "WORKFLOW.md")

    return {
      type: "WORKFLOW",
      identifier,
      displayName: this.toDisplayName(identifier),
      description: `Workflow: ${input.gap.description.slice(0, 100)}`,
      content: workflowContent,
      targetPath,
      metadata: {
        generatedAt: Date.now(),
        generatedBy: "WorkflowGenerator",
        version: "1.0.0",
        tags: this.extractTags(input),
        dependencies: steps.map((s) => s.agent),
      },
    }
  }

  async validateInput(input: GeneratorInput): Promise<{ valid: boolean; issues?: string[] }> {
    const issues: string[] = []

    if (!input.gap.description || input.gap.description.length < 20) {
      issues.push("Gap description too short for workflow generation (need at least 20 chars)")
    }

    // Verify agents exist
    const agents = await Agent.list()
    if (agents.length === 0) {
      issues.push("No agents available - workflows require agents")
    }

    return {
      valid: issues.length === 0,
      issues: issues.length > 0 ? issues : undefined,
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async inferSteps(input: GeneratorInput): Promise<WorkflowStep[]> {
    try {
      // Try to use LLM to break down into steps
      const llmSolver = getLLMSolver()
      const result = await llmSolver.generateCode({
        problem: `Break down this task into workflow steps: ${input.gap.description}

Return a JSON array of steps, each with:
- id: unique step identifier (snake_case)
- name: human-readable name
- description: what this step does
- agent: which agent type to use (use "build" for general tasks)

Example format:
[
  {"id": "analyze", "name": "Analyze Requirements", "description": "...", "agent": "build"},
  {"id": "implement", "name": "Implement Solution", "description": "...", "agent": "build"}
]

Return ONLY the JSON array.`,
        technology: "json",
        webSources: [],
        previousAttempts: [],
      })

      if (result) {
        try {
          const jsonMatch = result.code.match(/\[[\s\S]*\]/)
          if (jsonMatch) {
            const steps = JSON.parse(jsonMatch[0]) as WorkflowStep[]
            if (Array.isArray(steps) && steps.length > 0) {
              return this.enrichSteps(steps)
            }
          }
        } catch {
          // Fall through to default
        }
      }
    } catch (error) {
      log.warn("LLM step inference failed, using fallback", { error })
    }

    // Default workflow structure
    return this.getDefaultSteps(input)
  }

  private enrichSteps(steps: WorkflowStep[]): WorkflowStep[] {
    // Add transitions between steps
    return steps.map((step, index) => ({
      ...step,
      id: step.id || `step_${index + 1}`,
      agent: step.agent || "build",
      onSuccess: index < steps.length - 1 ? steps[index + 1].id : "complete",
      onFailure: "error",
    }))
  }

  private getDefaultSteps(input: GeneratorInput): WorkflowStep[] {
    const descLower = input.gap.description.toLowerCase()

    const steps: WorkflowStep[] = [
      {
        id: "analyze",
        name: "Analyze Requirements",
        description: "Understand and validate the requirements",
        agent: "build",
        onSuccess: "plan",
        onFailure: "error",
      },
      {
        id: "plan",
        name: "Create Plan",
        description: "Create an implementation plan",
        agent: "build",
        onSuccess: "execute",
        onFailure: "error",
      },
      {
        id: "execute",
        name: "Execute Plan",
        description: "Execute the planned steps",
        agent: "build",
        onSuccess: "verify",
        onFailure: "error",
      },
      {
        id: "verify",
        name: "Verify Results",
        description: "Verify the execution results",
        agent: "build",
        onSuccess: "complete",
        onFailure: "execute",
      },
    ]

    // Add review step for quality-related workflows
    if (descLower.includes("review") || descLower.includes("quality")) {
      steps.splice(3, 0, {
        id: "review",
        name: "Code Review",
        description: "Review code for quality and issues",
        agent: "code-reviewer",
        onSuccess: "verify",
        onFailure: "execute",
      })
    }

    // Add security step for security-related workflows
    if (descLower.includes("security") || descLower.includes("vulnerab")) {
      steps.splice(3, 0, {
        id: "security",
        name: "Security Check",
        description: "Check for security vulnerabilities",
        agent: "security-reviewer",
        onSuccess: "verify",
        onFailure: "execute",
      })
    }

    return steps
  }

  private generateWorkflowContent(params: {
    identifier: string
    displayName: string
    description: string
    steps: WorkflowStep[]
    technology?: string
  }): string {
    const stepsYaml = params.steps
      .map((step) => `  - id: ${step.id}
    name: "${step.name}"
    description: "${step.description}"
    agent: ${step.agent}
    ${step.onSuccess ? `on_success: ${step.onSuccess}` : ""}
    ${step.onFailure ? `on_failure: ${step.onFailure}` : ""}`)
      .join("\n")

    return `---
id: ${params.identifier}
name: ${params.displayName}
version: "1.0.0"
description: ${params.description}
enabled: false
initial_step: ${params.steps[0]?.id ?? "start"}
steps:
${stepsYaml}
---

# ${params.displayName}

${params.description}

## Overview

This workflow orchestrates a multi-step process to accomplish the described task.
Each step uses a specific agent and transitions based on success or failure.

## Workflow Diagram

\`\`\`
${this.generateAsciiDiagram(params.steps)}
\`\`\`

## Steps

${params.steps.map((step, i) => `### ${i + 1}. ${step.name}

**ID:** \`${step.id}\`
**Agent:** \`${step.agent}\`

${step.description}

- On success: ${step.onSuccess ?? "complete"}
- On failure: ${step.onFailure ?? "error"}
`).join("\n")}

## Configuration

### Enabling the Workflow

Set \`enabled: true\` in the frontmatter to activate this workflow.

### Customizing Steps

Each step can be customized with:
- \`condition\`: Expression that must be true to execute the step
- \`timeout\`: Maximum execution time in seconds
- \`retries\`: Number of retry attempts on failure

### Parameters

Add workflow parameters in the frontmatter:

\`\`\`yaml
params:
  key: value
\`\`\`

Parameters are available to all steps via \`{{params.key}}\`.

## Safety

This workflow requires manual approval before activation.
Review each step's agent and transitions carefully before enabling.

## Usage

### Trigger Workflow
\`\`\`bash
ccode workflow run ${params.identifier}
\`\`\`

### Check Status
\`\`\`bash
ccode workflow status ${params.identifier}
\`\`\`
`
  }

  private generateAsciiDiagram(steps: WorkflowStep[]): string {
    if (steps.length === 0) return "[No steps defined]"

    const lines: string[] = []
    lines.push("Start")
    lines.push("  │")

    for (let i = 0; i < steps.length; i++) {
      const step = steps[i]
      const isLast = i === steps.length - 1

      lines.push(`  ▼`)
      lines.push(`┌─────────────────────┐`)
      lines.push(`│ ${step.name.padEnd(19)} │`)
      lines.push(`│ (${step.agent})${" ".repeat(Math.max(0, 15 - step.agent.length))}│`)
      lines.push(`└─────────────────────┘`)

      if (!isLast) {
        lines.push("  │ success")
      }
    }

    lines.push("  │")
    lines.push("  ▼")
    lines.push("Complete")

    return lines.join("\n")
  }

  private generateIdentifier(input: GeneratorInput): string {
    if (input.gap.suggestedName) {
      const normalized = input.gap.suggestedName
        .toLowerCase()
        .replace(/[^a-z0-9-]/g, "-")
        .replace(/--+/g, "-")
        .slice(0, 40)

      if (!input.existingConcepts.includes(normalized)) {
        return normalized
      }
    }

    const words = input.gap.description
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, "")
      .split(/\s+/)
      .filter((w) => w.length > 2 && !["the", "and", "for", "with"].includes(w))
      .slice(0, 3)

    const base = words.join("-") || "workflow"
    return `${base}-${nanoid(6)}`
  }

  private toDisplayName(identifier: string): string {
    return identifier
      .split("-")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ")
  }

  private extractTags(input: GeneratorInput): string[] {
    const tags: string[] = ["workflow", "orchestration"]

    if (input.gap.technology) {
      tags.push(input.gap.technology.toLowerCase())
    }

    const descLower = input.gap.description.toLowerCase()
    if (descLower.includes("ci") || descLower.includes("cd")) tags.push("ci-cd")
    if (descLower.includes("deploy")) tags.push("deployment")
    if (descLower.includes("test")) tags.push("testing")
    if (descLower.includes("review")) tags.push("review")

    return [...new Set(tags)]
  }
}
