/**
 * Project Scaffolder - Creates new projects from scratch or templates
 *
 * Handles the actual file system operations for project creation:
 * - Creating project directories
 * - Initializing git repositories
 * - Cloning templates
 * - Installing dependencies
 * - Creating initial files
 */

import { promises as fs } from "fs"
import path from "path"
import { z } from "zod"
import { Log } from "@/util/log"
import { GitOps } from "./git-ops"

// Re-export ProjectRegistry and ProjectEntry for convenience
export { ProjectRegistry, type ProjectEntry } from "@codecoder-ai/util/project-registry"
import { ProjectRegistry, type ProjectEntry } from "@codecoder-ai/util/project-registry"

const log = Log.create({ service: "autonomous.project-scaffolder" })

// ============================================================================
// Types & Schemas
// ============================================================================

export const TechnologySchema = z.enum([
  "react",
  "vue",
  "svelte",
  "angular",
  "nextjs",
  "nuxt",
  "astro",
  "node",
  "bun",
  "deno",
  "typescript",
  "javascript",
  "python",
  "rust",
  "go",
])
export type Technology = z.infer<typeof TechnologySchema>

export const PackageManagerSchema = z.enum(["bun", "npm", "pnpm", "yarn"])
export type PackageManager = z.infer<typeof PackageManagerSchema>

export interface ScaffoldOptions {
  /** Project slug (URL-safe name) */
  slug: string
  /** Human-readable project name */
  name: string
  /** Project description */
  description?: string
  /** Technology stack */
  technology: string[]
  /** Source channel (for IM-created projects) */
  sourceChannel?: {
    type: "telegram" | "discord" | "slack" | "cli"
    chatId: string
  }
}

export interface ScaffoldResult {
  success: boolean
  project?: ProjectEntry
  path?: string
  error?: string
  logs: string[]
}

export interface TemplateCloneOptions extends ScaffoldOptions {
  /** Template repository URL */
  templateRepo: string
  /** Branch to clone (optional) */
  branch?: string
}

// ============================================================================
// Technology Templates
// ============================================================================

const DEFAULT_TEMPLATES: Record<string, string> = {
  // React
  "react-typescript": "https://github.com/vitejs/vite/tree/main/packages/create-vite/template-react-ts",
  react: "https://github.com/vitejs/vite/tree/main/packages/create-vite/template-react",
  // Next.js
  nextjs: "https://github.com/vercel/next.js/tree/canary/examples/hello-world",
  "nextjs-typescript": "https://github.com/vercel/next.js/tree/canary/examples/with-typescript",
  // Vue
  vue: "https://github.com/vitejs/vite/tree/main/packages/create-vite/template-vue",
  "vue-typescript": "https://github.com/vitejs/vite/tree/main/packages/create-vite/template-vue-ts",
  // Node.js
  node: "https://github.com/expressjs/generator",
  "node-typescript": "https://github.com/jsynowiec/node-typescript-boilerplate",
  // CLI
  "cli-node": "https://github.com/vercel/pkg",
  "cli-bun": "https://github.com/oven-sh/bun",
  // Python
  python: "https://github.com/pypa/sampleproject",
  fastapi: "https://github.com/tiangolo/full-stack-fastapi-template",
  // Rust
  rust: "https://github.com/rust-lang/rust-by-example",
  // Go
  go: "https://github.com/golang-standards/project-layout",
}

// ============================================================================
// ProjectScaffolder Class
// ============================================================================

export class ProjectScaffolder {
  private projectsDir: string

  constructor() {
    this.projectsDir = ProjectRegistry.getProjectsDir()
  }

  /**
   * Create an empty project from scratch
   */
  async createEmpty(options: ScaffoldOptions): Promise<ScaffoldResult> {
    const logs: string[] = []
    const projectPath = path.join(this.projectsDir, options.slug)

    try {
      logs.push(`Creating project at ${projectPath}`)

      // Check if directory already exists
      try {
        await fs.access(projectPath)
        return {
          success: false,
          error: `Directory already exists: ${projectPath}`,
          logs,
        }
      } catch {
        // Directory doesn't exist, good to proceed
      }

      // Create project directory
      await fs.mkdir(projectPath, { recursive: true })
      logs.push("Created project directory")

      // Create initial files based on technology
      await this.createInitialFiles(projectPath, options.technology, options.name, options.description)
      logs.push("Created initial files")

      // Initialize git repository
      const initResult = await GitOps.init(projectPath, {
        initialCommit: true,
        commitMessage: `[project-scaffold] Initial commit: ${options.name}`,
      })

      if (!initResult.success) {
        logs.push(`Git init failed: ${initResult.error}`)
      } else {
        logs.push("Initialized git repository")
      }

      // Register project in registry
      const project = await ProjectRegistry.create({
        slug: options.slug,
        name: options.name,
        description: options.description,
        path: projectPath,
        technology: options.technology,
        sourceChannel: options.sourceChannel,
      })
      logs.push(`Registered project with ID: ${project.id}`)

      log.info("Empty project created", { slug: options.slug, path: projectPath })

      return {
        success: true,
        project,
        path: projectPath,
        logs,
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      logs.push(`Error: ${errorMessage}`)
      log.error("Failed to create empty project", { slug: options.slug, error: errorMessage })

      // Cleanup on failure
      try {
        await fs.rm(projectPath, { recursive: true, force: true })
      } catch {
        // Ignore cleanup errors
      }

      return {
        success: false,
        error: errorMessage,
        logs,
      }
    }
  }

  /**
   * Create a project from a template
   */
  async cloneTemplate(options: TemplateCloneOptions): Promise<ScaffoldResult> {
    const logs: string[] = []
    const projectPath = path.join(this.projectsDir, options.slug)

    try {
      logs.push(`Cloning template from ${options.templateRepo}`)

      // Check if directory already exists
      try {
        await fs.access(projectPath)
        return {
          success: false,
          error: `Directory already exists: ${projectPath}`,
          logs,
        }
      } catch {
        // Directory doesn't exist, good to proceed
      }

      // Clone the template repository
      const cloneResult = await GitOps.clone(options.templateRepo, projectPath, {
        depth: 1,
        branch: options.branch,
        reinitialize: true,
      })

      if (!cloneResult.success) {
        return {
          success: false,
          error: `Clone failed: ${cloneResult.error}`,
          logs,
        }
      }
      logs.push("Cloned template repository")

      // Update README or create project-specific files
      await this.customizeFromTemplate(projectPath, options)
      logs.push("Customized project from template")

      // Register project in registry
      const project = await ProjectRegistry.create({
        slug: options.slug,
        name: options.name,
        description: options.description,
        path: projectPath,
        template: options.templateRepo,
        technology: options.technology,
        sourceChannel: options.sourceChannel,
      })
      logs.push(`Registered project with ID: ${project.id}`)

      log.info("Template project created", {
        slug: options.slug,
        template: options.templateRepo,
        path: projectPath,
      })

      return {
        success: true,
        project,
        path: projectPath,
        logs,
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      logs.push(`Error: ${errorMessage}`)
      log.error("Failed to create project from template", {
        slug: options.slug,
        template: options.templateRepo,
        error: errorMessage,
      })

      // Cleanup on failure
      try {
        await fs.rm(projectPath, { recursive: true, force: true })
      } catch {
        // Ignore cleanup errors
      }

      return {
        success: false,
        error: errorMessage,
        logs,
      }
    }
  }

  /**
   * Install project dependencies
   */
  async installDependencies(
    projectPath: string,
    packageManager?: PackageManager,
  ): Promise<{ success: boolean; output?: string; error?: string }> {
    // Detect package manager if not specified
    const pm = packageManager ?? (await this.detectPackageManager(projectPath))

    try {
      log.info("Installing dependencies", { path: projectPath, packageManager: pm })

      const proc = Bun.spawn([pm, "install"], {
        cwd: projectPath,
        env: process.env,
      })

      const output = await new Response(proc.stdout).text()
      const exitCode = await proc.exited

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text()
        return {
          success: false,
          error: stderr || "Installation failed",
        }
      }

      log.info("Dependencies installed", { path: projectPath })
      return {
        success: true,
        output,
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /**
   * Detect package manager from lockfiles
   */
  async detectPackageManager(projectPath: string): Promise<PackageManager> {
    const lockfiles = {
      "bun.lockb": "bun" as const,
      "package-lock.json": "npm" as const,
      "pnpm-lock.yaml": "pnpm" as const,
      "yarn.lock": "yarn" as const,
    }

    for (const [file, pm] of Object.entries(lockfiles)) {
      try {
        await fs.access(path.join(projectPath, file))
        return pm
      } catch {
        // File doesn't exist, continue
      }
    }

    // Default to bun
    return "bun"
  }

  /**
   * Create initial files for an empty project
   */
  private async createInitialFiles(
    projectPath: string,
    technology: string[],
    name: string,
    description?: string,
  ): Promise<void> {
    // Create .gitignore
    const gitignore = this.generateGitignore(technology)
    await fs.writeFile(path.join(projectPath, ".gitignore"), gitignore)

    // Create README.md
    const readme = this.generateReadme(name, description, technology)
    await fs.writeFile(path.join(projectPath, "README.md"), readme)

    // Create technology-specific files
    if (technology.includes("typescript") || technology.includes("node") || technology.includes("react")) {
      await this.createNodeProject(projectPath, name, description, technology)
    } else if (technology.includes("python")) {
      await this.createPythonProject(projectPath, name, description)
    } else if (technology.includes("rust")) {
      await this.createRustProject(projectPath, name, description)
    } else if (technology.includes("go")) {
      await this.createGoProject(projectPath, name, description)
    } else {
      // Default: create a basic Node.js project
      await this.createNodeProject(projectPath, name, description, technology)
    }
  }

  /**
   * Create a Node.js/TypeScript project structure
   */
  private async createNodeProject(
    projectPath: string,
    name: string,
    description?: string,
    technology: string[] = [],
  ): Promise<void> {
    const isTypeScript = technology.includes("typescript")

    // Create package.json
    const packageJson = {
      name: name.toLowerCase().replace(/[^a-z0-9-]/g, "-"),
      version: "0.1.0",
      description: description || "",
      main: isTypeScript ? "dist/index.js" : "index.js",
      type: "module",
      scripts: {
        start: isTypeScript ? "bun run dist/index.js" : "bun run index.js",
        dev: isTypeScript ? "bun run --watch src/index.ts" : "bun run --watch index.js",
        build: isTypeScript ? "bun build src/index.ts --outdir dist" : undefined,
        test: "bun test",
      },
      keywords: [],
      author: "",
      license: "MIT",
      devDependencies: isTypeScript
        ? {
            typescript: "^5.0.0",
            "@types/bun": "latest",
          }
        : {},
    }

    // Remove undefined values
    Object.keys(packageJson.scripts).forEach((key) => {
      if (packageJson.scripts[key as keyof typeof packageJson.scripts] === undefined) {
        delete packageJson.scripts[key as keyof typeof packageJson.scripts]
      }
    })

    await fs.writeFile(path.join(projectPath, "package.json"), JSON.stringify(packageJson, null, 2))

    // Create source directory and entry point
    if (isTypeScript) {
      await fs.mkdir(path.join(projectPath, "src"), { recursive: true })
      await fs.writeFile(
        path.join(projectPath, "src", "index.ts"),
        `/**
 * ${name}
 * ${description || ""}
 */

console.log("Hello from ${name}!")
`,
      )

      // Create tsconfig.json
      const tsconfig = {
        compilerOptions: {
          target: "ES2022",
          module: "ESNext",
          moduleResolution: "bundler",
          strict: true,
          esModuleInterop: true,
          skipLibCheck: true,
          outDir: "./dist",
          rootDir: "./src",
        },
        include: ["src/**/*"],
        exclude: ["node_modules", "dist"],
      }
      await fs.writeFile(path.join(projectPath, "tsconfig.json"), JSON.stringify(tsconfig, null, 2))
    } else {
      await fs.writeFile(
        path.join(projectPath, "index.js"),
        `/**
 * ${name}
 * ${description || ""}
 */

console.log("Hello from ${name}!")
`,
      )
    }

    // Create test directory
    await fs.mkdir(path.join(projectPath, "test"), { recursive: true })
    await fs.writeFile(
      path.join(projectPath, "test", isTypeScript ? "index.test.ts" : "index.test.js"),
      `import { describe, it, expect } from "bun:test"

describe("${name}", () => {
  it("should pass", () => {
    expect(true).toBe(true)
  })
})
`,
    )
  }

  /**
   * Create a Python project structure
   */
  private async createPythonProject(projectPath: string, name: string, description?: string): Promise<void> {
    const moduleName = name.toLowerCase().replace(/[^a-z0-9_]/g, "_")

    // Create module directory
    await fs.mkdir(path.join(projectPath, moduleName), { recursive: true })
    await fs.writeFile(path.join(projectPath, moduleName, "__init__.py"), `"""${description || name}"""\n`)
    await fs.writeFile(
      path.join(projectPath, moduleName, "main.py"),
      `"""Main entry point for ${name}."""


def main():
    """Main function."""
    print(f"Hello from ${name}!")


if __name__ == "__main__":
    main()
`,
    )

    // Create pyproject.toml
    const pyprojectToml = `[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"

[project]
name = "${moduleName}"
version = "0.1.0"
description = "${description || ""}"
readme = "README.md"
requires-python = ">=3.9"
classifiers = [
    "Programming Language :: Python :: 3",
    "License :: OSI Approved :: MIT License",
    "Operating System :: OS Independent",
]

[project.scripts]
${moduleName} = "${moduleName}.main:main"
`
    await fs.writeFile(path.join(projectPath, "pyproject.toml"), pyprojectToml)

    // Create tests directory
    await fs.mkdir(path.join(projectPath, "tests"), { recursive: true })
    await fs.writeFile(
      path.join(projectPath, "tests", "test_main.py"),
      `"""Tests for ${name}."""

import pytest
from ${moduleName}.main import main


def test_main():
    """Test main function."""
    assert True
`,
    )
  }

  /**
   * Create a Rust project structure
   */
  private async createRustProject(projectPath: string, name: string, description?: string): Promise<void> {
    const crateName = name.toLowerCase().replace(/[^a-z0-9_-]/g, "-")

    // Create src directory
    await fs.mkdir(path.join(projectPath, "src"), { recursive: true })
    await fs.writeFile(
      path.join(projectPath, "src", "main.rs"),
      `//! ${description || name}

fn main() {
    println!("Hello from ${name}!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
`,
    )

    // Create Cargo.toml
    const cargoToml = `[package]
name = "${crateName}"
version = "0.1.0"
edition = "2021"
description = "${description || ""}"

[dependencies]
`
    await fs.writeFile(path.join(projectPath, "Cargo.toml"), cargoToml)
  }

  /**
   * Create a Go project structure
   */
  private async createGoProject(projectPath: string, name: string, description?: string): Promise<void> {
    const moduleName = name.toLowerCase().replace(/[^a-z0-9-]/g, "-")

    // Create main.go
    await fs.writeFile(
      path.join(projectPath, "main.go"),
      `// Package main is the entry point for ${name}.
// ${description || ""}
package main

import "fmt"

func main() {
	fmt.Println("Hello from ${name}!")
}
`,
    )

    // Create go.mod
    await fs.writeFile(
      path.join(projectPath, "go.mod"),
      `module ${moduleName}

go 1.21
`,
    )

    // Create main_test.go
    await fs.writeFile(
      path.join(projectPath, "main_test.go"),
      `package main

import "testing"

func TestMain(t *testing.T) {
	// Add tests here
}
`,
    )
  }

  /**
   * Generate .gitignore content
   */
  private generateGitignore(technology: string[]): string {
    const sections: string[] = []

    // Common ignores
    sections.push(`# Dependencies
node_modules/
.pnp/
.pnp.js

# Build outputs
dist/
build/
out/
*.pyc
__pycache__/
target/

# IDE
.idea/
.vscode/
*.swp
*.swo
.DS_Store

# Environment
.env
.env.local
.env.*.local

# Logs
logs/
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Testing
coverage/
.nyc_output/
.pytest_cache/

# Misc
*.tgz
.cache/
`)

    if (technology.includes("typescript")) {
      sections.push(`# TypeScript
*.tsbuildinfo
tsconfig.tsbuildinfo
`)
    }

    if (technology.includes("python")) {
      sections.push(`# Python
venv/
.venv/
*.egg-info/
.eggs/
*.egg
.mypy_cache/
.ruff_cache/
`)
    }

    if (technology.includes("rust")) {
      sections.push(`# Rust
target/
Cargo.lock
`)
    }

    return sections.join("\n")
  }

  /**
   * Generate README.md content
   */
  private generateReadme(name: string, description?: string, technology: string[] = []): string {
    const techBadges = technology
      .slice(0, 5)
      .map((t) => `![${t}](https://img.shields.io/badge/-${t}-blue)`)
      .join(" ")

    return `# ${name}

${description || "A new project created by CodeCoder."}

${techBadges}

## Getting Started

\`\`\`bash
# Install dependencies
bun install

# Run development server
bun dev

# Run tests
bun test
\`\`\`

## Project Structure

\`\`\`
${name}/
├── src/           # Source code
├── test/          # Tests
├── package.json   # Dependencies
└── README.md      # This file
\`\`\`

## Created By

This project was scaffolded by [CodeCoder](https://github.com/codecoder-ai/codecoder) autonomous mode.
`
  }

  /**
   * Customize a cloned template
   */
  private async customizeFromTemplate(projectPath: string, options: TemplateCloneOptions): Promise<void> {
    // Try to update README with project-specific info
    const readmePath = path.join(projectPath, "README.md")
    try {
      const existingReadme = await fs.readFile(readmePath, "utf-8")
      // Prepend project name and description
      const customizedReadme = `# ${options.name}

${options.description || ""}

---

*Scaffolded from template: ${options.templateRepo}*

---

${existingReadme}
`
      await fs.writeFile(readmePath, customizedReadme)
    } catch {
      // Create new README if none exists
      await fs.writeFile(readmePath, this.generateReadme(options.name, options.description, options.technology))
    }

    // Create .codecoder directory for project-specific config
    const codeCoderDir = path.join(projectPath, ".codecoder")
    await fs.mkdir(codeCoderDir, { recursive: true })

    // Create context.json with project metadata
    const context = {
      name: options.name,
      description: options.description,
      template: options.templateRepo,
      technology: options.technology,
      createdAt: new Date().toISOString(),
      sourceChannel: options.sourceChannel,
    }
    await fs.writeFile(path.join(codeCoderDir, "context.json"), JSON.stringify(context, null, 2))
  }

  /**
   * Get suggested template for a technology stack
   */
  getSuggestedTemplate(technology: string[]): string | null {
    // Build a key from technology
    const key = technology.slice(0, 2).sort().join("-")

    return DEFAULT_TEMPLATES[key] ?? DEFAULT_TEMPLATES[technology[0]] ?? null
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a new ProjectScaffolder instance
 */
export function createProjectScaffolder(): ProjectScaffolder {
  return new ProjectScaffolder()
}

/**
 * Quick scaffold: create empty project
 */
export async function scaffoldEmptyProject(options: ScaffoldOptions): Promise<ScaffoldResult> {
  const scaffolder = createProjectScaffolder()
  return scaffolder.createEmpty(options)
}

/**
 * Quick scaffold: clone from template
 */
export async function scaffoldFromTemplate(options: TemplateCloneOptions): Promise<ScaffoldResult> {
  const scaffolder = createProjectScaffolder()
  return scaffolder.cloneTemplate(options)
}
