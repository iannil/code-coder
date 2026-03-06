/**
 * Project Tools - MCP tools for creating and managing user projects
 *
 * Provides tools for autonomous project creation from IM messages:
 * - project_create: Create a new project (from scratch or template)
 * - project_list: List all managed projects
 * - project_switch: Switch to a different project context
 * - project_push: Push project to remote repository
 *
 * These tools are primarily used by the autonomous agent when handling
 * project creation requests from Telegram, Discord, or other IM channels.
 */

import z from "zod"
import path from "path"
import { Tool } from "./tool"
import { Log } from "@/util/log"
import { ProjectRegistry, type ProjectEntry } from "@/autonomous/execution/project-scaffolder"
import {
  ProjectCreator,
  createProjectCreator,
  type ProjectCreationRequest,
  type ProjectCreationDecision,
} from "@/autonomous/execution/project-creator"
import {
  ProjectScaffolder,
  createProjectScaffolder,
  type ScaffoldOptions,
  type TemplateCloneOptions,
} from "@/autonomous/execution/project-scaffolder"
import { GitOps } from "@/autonomous/execution/git-ops"
import { Instance } from "@/project/instance"
import { Bus } from "@/bus"
import { AutonomousEvent } from "@/autonomous/events"

const log = Log.create({ service: "tool.project" })

// ============================================================================
// project_create
// ============================================================================

const PROJECT_CREATE_DESCRIPTION = `Create a new project in the managed projects directory (~/.codecoder/workspace/projects/).

Use this tool when a user asks to create a new project, application, or app.
The tool can create projects from scratch or from a template.

Detection patterns (Chinese):
- "帮我创建一个...项目/应用/App"
- "新建一个..."
- "创建一个..."

Detection patterns (English):
- "Create a new ... project/app"
- "Build a ... application"
- "Start a new ..."

Example usage:
- User: "帮我创建一个 React Todo App"
  → project_create(name: "Todo App", technology: ["react", "typescript"])
- User: "Create a new CLI tool in Rust"
  → project_create(name: "CLI Tool", technology: ["rust"])
`

export const ProjectCreateTool = Tool.define("project_create", {
  description: PROJECT_CREATE_DESCRIPTION,
  parameters: z.object({
    name: z.string().min(1).max(100).describe("Human-readable project name"),
    technology: z.array(z.string()).min(1).describe('Technology stack (e.g., ["react", "typescript"])'),
    description: z.string().max(500).optional().describe("Project description"),
    template: z.string().url().optional().describe("Template repository URL to clone from"),
    installDependencies: z.boolean().default(true).describe("Whether to install dependencies after scaffolding"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "project",
      patterns: [params.name],
      always: ["*"],
      metadata: {
        action: "create_project",
        name: params.name,
        technology: params.technology,
      },
    })

    log.info("Creating project", { name: params.name, technology: params.technology })

    try {
      // Generate unique slug
      const slug = await ProjectRegistry.generateUniqueSlug(params.name)
      const projectPath = ProjectRegistry.getProjectPath(slug)

      // Auto-detect channel info from context
      const channelType = ctx.extra?.channelType as "telegram" | "discord" | "slack" | "cli" | undefined
      const channelId = ctx.extra?.channelId as string | undefined

      const sourceChannel = channelType && channelId ? { type: channelType, chatId: channelId } : undefined

      const scaffolder = createProjectScaffolder()
      let result

      if (params.template) {
        // Clone from template
        const templateOptions: TemplateCloneOptions = {
          slug,
          name: params.name,
          description: params.description,
          technology: params.technology,
          sourceChannel,
          templateRepo: params.template,
        }
        result = await scaffolder.cloneTemplate(templateOptions)
      } else {
        // Create from scratch
        const emptyOptions: ScaffoldOptions = {
          slug,
          name: params.name,
          description: params.description,
          technology: params.technology,
          sourceChannel,
        }
        result = await scaffolder.createEmpty(emptyOptions)
      }

      if (!result.success) {
        throw new Error(result.error || "Failed to create project")
      }

      // Install dependencies if requested
      if (params.installDependencies && result.path) {
        log.info("Installing dependencies", { path: result.path })
        const installResult = await scaffolder.installDependencies(result.path)
        if (!installResult.success) {
          log.warn("Dependency installation failed", { error: installResult.error })
        }
      }

      // Publish event
      await Bus.publish(AutonomousEvent.ProjectCreated, {
        projectId: result.project?.id,
        name: params.name,
        slug,
        technology: params.technology,
        action: params.template ? "clone_template" : "create_from_scratch",
        template: params.template,
        success: true,
      })

      return {
        title: `Created project: ${params.name}`,
        metadata: {
          projectId: result.project?.id,
          slug,
          path: result.path,
          technology: params.technology,
        },
        output: `Successfully created project "${params.name}"

📁 Location: ${result.path}
🔗 Slug: ${slug}
🛠️ Technology: ${params.technology.join(", ")}
${params.template ? `📦 Template: ${params.template}` : "📦 Created from scratch"}

To start developing:
  cd ${result.path}
  bun dev

${result.logs.join("\n")}`,
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      log.error("Failed to create project", { name: params.name, error: errorMessage })
      throw new Error(`Failed to create project: ${errorMessage}`)
    }
  },
})

// ============================================================================
// project_list
// ============================================================================

const PROJECT_LIST_DESCRIPTION = `List all projects in the managed projects directory.

Returns information about projects created via IM channels or CLI, including:
- Project name and description
- Technology stack
- Creation date and last access
- Source channel (Telegram, Discord, etc.)

Use this tool to:
- See all available projects
- Find a specific project by name
- Check which projects exist before creating a new one`

export const ProjectListTool = Tool.define("project_list", {
  description: PROJECT_LIST_DESCRIPTION,
  parameters: z.object({
    includeArchived: z.boolean().default(false).describe("Include archived projects"),
    search: z.string().optional().describe("Search by name or technology"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "project",
      patterns: ["list"],
      always: ["*"],
      metadata: { action: "list_projects" },
    })

    log.info("Listing projects", { includeArchived: params.includeArchived, search: params.search })

    let projects: ProjectEntry[]

    if (params.search) {
      projects = await ProjectRegistry.search(params.search)
    } else {
      projects = await ProjectRegistry.list(params.includeArchived)
    }

    if (projects.length === 0) {
      return {
        title: "Projects: 0 found",
        metadata: { count: 0, projects: [] as Array<{ id: string; slug: string; name: string }> },
        output: params.search
          ? `No projects found matching "${params.search}".`
          : "No projects found. Use project_create to create a new project.",
      }
    }

    const formatDate = (timestamp: number) => {
      const date = new Date(timestamp)
      return date.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })
    }

    const projectList = projects
      .map((p) => {
        const sourceInfo = p.sourceChannel ? `[${p.sourceChannel.type}]` : "[cli]"
        return `📁 ${p.name} (${p.slug}) ${sourceInfo}
   Technology: ${p.technology.join(", ")}
   Path: ${p.path}
   Created: ${formatDate(p.createdAt)}
   ${p.description ? `Description: ${p.description}` : ""}`
      })
      .join("\n\n")

    return {
      title: `Projects: ${projects.length} found`,
      metadata: {
        count: projects.length,
        projects: projects.map((p) => ({ id: p.id, slug: p.slug, name: p.name })),
      },
      output: `Found ${projects.length} project(s):\n\n${projectList}`,
    }
  },
})

// ============================================================================
// project_switch
// ============================================================================

const PROJECT_SWITCH_DESCRIPTION = `Switch to a different project context for development.

When you switch to a project:
- The working directory changes to the project path
- All subsequent file operations happen in the project directory
- The project's last access time is updated

Use this after project_list to select a project to work on.`

export const ProjectSwitchTool = Tool.define("project_switch", {
  description: PROJECT_SWITCH_DESCRIPTION,
  parameters: z.object({
    projectId: z
      .string()
      .describe("Project ID or slug. Use project_list to find available projects."),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "project",
      patterns: [params.projectId],
      always: ["*"],
      metadata: {
        action: "switch_project",
        projectId: params.projectId,
      },
    })

    log.info("Switching project", { projectId: params.projectId })

    // Try to find by ID first, then by slug
    let project = await ProjectRegistry.get(params.projectId)
    if (!project) {
      project = await ProjectRegistry.findBySlug(params.projectId)
    }

    if (!project) {
      throw new Error(`Project not found: ${params.projectId}. Use project_list to see available projects.`)
    }

    // Update last accessed time
    await ProjectRegistry.touch(project.id)

    // Publish event
    await Bus.publish(AutonomousEvent.ProjectSwitched, {
      projectId: project.id,
      slug: project.slug,
      path: project.path,
    })

    return {
      title: `Switched to: ${project.name}`,
      metadata: {
        projectId: project.id,
        slug: project.slug,
        path: project.path,
      },
      output: `Switched to project "${project.name}"

📁 Path: ${project.path}
🛠️ Technology: ${project.technology.join(", ")}
${project.description ? `📝 Description: ${project.description}` : ""}
${project.remoteUrl ? `🔗 Remote: ${project.remoteUrl}` : ""}

You can now work on this project. All file operations will target this directory.`,
    }
  },
})

// ============================================================================
// project_push
// ============================================================================

const PROJECT_PUSH_DESCRIPTION = `Push a project to a remote Git repository.

This tool:
1. Adds a remote origin if not already set
2. Pushes the project to the remote
3. Updates the project registry with the remote URL

Use this when the user wants to:
- Share their project on GitHub
- Back up their project to a remote
- Collaborate with others

IMPORTANT: The user must provide the remote URL. This tool does NOT create GitHub repositories automatically.`

export const ProjectPushTool = Tool.define("project_push", {
  description: PROJECT_PUSH_DESCRIPTION,
  parameters: z.object({
    projectId: z.string().describe("Project ID or slug"),
    remote: z.string().url().describe("Remote repository URL (e.g., https://github.com/user/repo.git)"),
    force: z.boolean().default(false).describe("Force push (use with caution)"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "project",
      patterns: [params.projectId, params.remote],
      always: ["*"],
      metadata: {
        action: "push_project",
        projectId: params.projectId,
        remote: params.remote,
        force: params.force,
      },
    })

    log.info("Pushing project", { projectId: params.projectId, remote: params.remote })

    // Find project
    let project = await ProjectRegistry.get(params.projectId)
    if (!project) {
      project = await ProjectRegistry.findBySlug(params.projectId)
    }

    if (!project) {
      throw new Error(`Project not found: ${params.projectId}`)
    }

    // Check if it's a git repo
    const isRepo = GitOps.isGitRepo(project.path)
    if (!isRepo) {
      // Initialize git if needed
      const initResult = GitOps.init(project.path, {
        initialCommit: true,
        commitMessage: "[project-push] Initialize repository",
      })
      if (!initResult.success) {
        throw new Error(`Failed to initialize git: ${initResult.error}`)
      }
    }

    // Check current remote
    const currentRemote = GitOps.getRemoteUrl(project.path)

    if (currentRemote && currentRemote !== params.remote) {
      // Remove old remote and add new one
      GitOps.removeRemote(project.path)
    }

    if (!currentRemote || currentRemote !== params.remote) {
      // Add the remote
      const addResult = GitOps.addRemote(project.path, "origin", params.remote)
      if (!addResult.success) {
        throw new Error(`Failed to add remote: ${addResult.error}`)
      }
    }

    // Push to remote
    const pushResult = GitOps.push(project.path, "origin", "main", true)

    if (!pushResult.success) {
      // Publish failure event
      await Bus.publish(AutonomousEvent.ProjectPushed, {
        projectId: project.id,
        slug: project.slug,
        remote: params.remote,
        success: false,
        error: pushResult.error,
      })

      throw new Error(`Failed to push: ${pushResult.error}`)
    }

    // Update project registry with remote URL
    await ProjectRegistry.update(project.id, { remoteUrl: params.remote })

    // Publish success event
    await Bus.publish(AutonomousEvent.ProjectPushed, {
      projectId: project.id,
      slug: project.slug,
      remote: params.remote,
      success: true,
    })

    return {
      title: `Pushed: ${project.name}`,
      metadata: {
        projectId: project.id,
        slug: project.slug,
        remote: params.remote,
      },
      output: `Successfully pushed "${project.name}" to remote repository.

📁 Project: ${project.path}
🔗 Remote: ${params.remote}

The project is now available at the remote URL.`,
    }
  },
})

// ============================================================================
// project_get
// ============================================================================

const PROJECT_GET_DESCRIPTION = `Get detailed information about a specific project.

Returns full project metadata including:
- Project ID, name, and description
- File path and technology stack
- Creation and last access timestamps
- Source channel information
- Remote repository URL (if configured)`

export const ProjectGetTool = Tool.define("project_get", {
  description: PROJECT_GET_DESCRIPTION,
  parameters: z.object({
    projectId: z.string().describe("Project ID or slug"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "project",
      patterns: [params.projectId],
      always: ["*"],
      metadata: {
        action: "get_project",
        projectId: params.projectId,
      },
    })

    // Find project
    let project = await ProjectRegistry.get(params.projectId)
    if (!project) {
      project = await ProjectRegistry.findBySlug(params.projectId)
    }

    if (!project) {
      throw new Error(`Project not found: ${params.projectId}`)
    }

    const formatDate = (timestamp: number) => {
      return new Date(timestamp).toISOString()
    }

    return {
      title: `Project: ${project.name}`,
      metadata: project,
      output: `Project: ${project.name}

📋 ID: ${project.id}
📁 Slug: ${project.slug}
📂 Path: ${project.path}
🛠️ Technology: ${project.technology.join(", ")}
📝 Description: ${project.description || "(none)"}
📦 Template: ${project.template || "(created from scratch)"}
🔗 Remote: ${project.remoteUrl || "(not configured)"}
📅 Created: ${formatDate(project.createdAt)}
📅 Last Access: ${formatDate(project.lastAccessedAt)}
📡 Source: ${project.sourceChannel ? `${project.sourceChannel.type}:${project.sourceChannel.chatId}` : "cli"}
📊 Status: ${project.status}`,
    }
  },
})

// ============================================================================
// project_archive
// ============================================================================

const PROJECT_ARCHIVE_DESCRIPTION = `Archive a project (soft delete).

Archived projects:
- Are hidden from project_list by default
- Can be shown with includeArchived: true
- Still exist on disk and can be accessed directly
- Can be unarchived by updating their status

Use this for projects that are no longer actively developed.`

export const ProjectArchiveTool = Tool.define("project_archive", {
  description: PROJECT_ARCHIVE_DESCRIPTION,
  parameters: z.object({
    projectId: z.string().describe("Project ID or slug to archive"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "project",
      patterns: [params.projectId],
      always: ["*"],
      metadata: {
        action: "archive_project",
        projectId: params.projectId,
      },
    })

    // Find project
    let project = await ProjectRegistry.get(params.projectId)
    if (!project) {
      project = await ProjectRegistry.findBySlug(params.projectId)
    }

    if (!project) {
      throw new Error(`Project not found: ${params.projectId}`)
    }

    // Archive the project
    const updatedProject = await ProjectRegistry.archive(project.id)

    return {
      title: `Archived: ${project.name}`,
      metadata: {
        projectId: updatedProject.id,
        slug: updatedProject.slug,
        status: updatedProject.status,
      },
      output: `Project "${project.name}" has been archived.

The project files still exist at: ${project.path}
To see archived projects, use project_list with includeArchived: true.`,
    }
  },
})

// ============================================================================
// Export all tools
// ============================================================================

export const ProjectTools = [
  ProjectCreateTool,
  ProjectListTool,
  ProjectSwitchTool,
  ProjectPushTool,
  ProjectGetTool,
  ProjectArchiveTool,
]
