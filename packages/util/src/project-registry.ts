/**
 * Project Registry - Manages metadata for user-created projects
 *
 * Projects created via IM (Telegram, Discord) or CLI are stored in
 * ~/.codecoder/workspace/projects/ and indexed in registry.json
 *
 * The registry provides fast lookup without filesystem scanning and
 * tracks project metadata like source channel, technology stack, etc.
 */

import { promises as fs } from "fs"
import path from "path"
import { getWorkspacePathsFromEnv } from "./workspace"

/**
 * Project entry stored in the registry
 */
export interface ProjectEntry {
  /** Unique identifier (UUID) */
  id: string
  /** URL-safe slug used as directory name */
  slug: string
  /** Human-readable project name */
  name: string
  /** Optional project description */
  description?: string
  /** Absolute path to project directory */
  path: string
  /** Template repository used (if any) */
  template?: string
  /** Technology stack (e.g., ["react", "typescript", "vite"]) */
  technology: string[]
  /** Creation timestamp (ms since epoch) */
  createdAt: number
  /** Last access timestamp (ms since epoch) */
  lastAccessedAt: number
  /** Source channel that initiated project creation */
  sourceChannel?: {
    type: "telegram" | "discord" | "slack" | "cli"
    chatId: string
  }
  /** Git remote URL (if configured) */
  remoteUrl?: string
  /** Project status */
  status: "active" | "archived" | "deleted"
}

/**
 * Registry file structure
 */
interface RegistryData {
  version: 1
  projects: ProjectEntry[]
  lastUpdated: number
}

/**
 * Project Registry namespace for CRUD operations
 */
export namespace ProjectRegistry {
  /**
   * Load the registry from disk
   */
  async function load(): Promise<RegistryData> {
    const { projectRegistry } = getWorkspacePathsFromEnv()

    try {
      const content = await fs.readFile(projectRegistry, "utf-8")
      return JSON.parse(content) as RegistryData
    } catch (error) {
      // Return empty registry if file doesn't exist
      if ((error as NodeJS.ErrnoException).code === "ENOENT") {
        return {
          version: 1,
          projects: [],
          lastUpdated: Date.now(),
        }
      }
      throw error
    }
  }

  /**
   * Save the registry to disk
   */
  async function save(data: RegistryData): Promise<void> {
    const { projectRegistry, projects: projectsDir } = getWorkspacePathsFromEnv()

    // Ensure projects directory exists
    await fs.mkdir(projectsDir, { recursive: true })

    // Update timestamp and write atomically
    data.lastUpdated = Date.now()
    const content = JSON.stringify(data, null, 2)
    const tempPath = `${projectRegistry}.tmp`

    await fs.writeFile(tempPath, content, "utf-8")
    await fs.rename(tempPath, projectRegistry)
  }

  /**
   * Generate a unique ID
   */
  function generateId(): string {
    return crypto.randomUUID()
  }

  /**
   * List all projects
   * @param includeArchived - Include archived projects (default: false)
   */
  export async function list(includeArchived = false): Promise<ProjectEntry[]> {
    const data = await load()
    return data.projects.filter((p) => includeArchived || p.status === "active")
  }

  /**
   * Get a project by ID
   */
  export async function get(id: string): Promise<ProjectEntry | null> {
    const data = await load()
    return data.projects.find((p) => p.id === id) ?? null
  }

  /**
   * Find a project by slug
   */
  export async function findBySlug(slug: string): Promise<ProjectEntry | null> {
    const data = await load()
    return data.projects.find((p) => p.slug === slug) ?? null
  }

  /**
   * Find projects by source channel
   */
  export async function findByChannel(
    channelType: string,
    chatId: string,
  ): Promise<ProjectEntry[]> {
    const data = await load()
    return data.projects.filter(
      (p) =>
        p.sourceChannel?.type === channelType &&
        p.sourceChannel?.chatId === chatId &&
        p.status === "active",
    )
  }

  /**
   * Create a new project entry
   */
  export async function create(
    entry: Omit<ProjectEntry, "id" | "createdAt" | "lastAccessedAt" | "status">,
  ): Promise<ProjectEntry> {
    const data = await load()

    // Check for slug collision
    const existing = data.projects.find((p) => p.slug === entry.slug)
    if (existing) {
      throw new Error(`Project with slug "${entry.slug}" already exists`)
    }

    const now = Date.now()
    const newEntry: ProjectEntry = {
      ...entry,
      id: generateId(),
      createdAt: now,
      lastAccessedAt: now,
      status: "active",
    }

    data.projects.push(newEntry)
    await save(data)

    return newEntry
  }

  /**
   * Update an existing project
   */
  export async function update(
    id: string,
    updates: Partial<Omit<ProjectEntry, "id" | "createdAt">>,
  ): Promise<ProjectEntry> {
    const data = await load()
    const index = data.projects.findIndex((p) => p.id === id)

    if (index === -1) {
      throw new Error(`Project with ID "${id}" not found`)
    }

    // Update last accessed time on any update
    const updatedEntry: ProjectEntry = {
      ...data.projects[index],
      ...updates,
      lastAccessedAt: Date.now(),
    }

    data.projects[index] = updatedEntry
    await save(data)

    return updatedEntry
  }

  /**
   * Mark a project's last accessed time
   */
  export async function touch(id: string): Promise<ProjectEntry> {
    return update(id, { lastAccessedAt: Date.now() })
  }

  /**
   * Archive a project (soft delete)
   */
  export async function archive(id: string): Promise<ProjectEntry> {
    return update(id, { status: "archived" })
  }

  /**
   * Remove a project from registry (does not delete files)
   */
  export async function remove(id: string): Promise<void> {
    const data = await load()
    const index = data.projects.findIndex((p) => p.id === id)

    if (index === -1) {
      throw new Error(`Project with ID "${id}" not found`)
    }

    // Mark as deleted rather than removing (for history)
    data.projects[index].status = "deleted"
    await save(data)
  }

  /**
   * Permanently delete a project entry (use with caution)
   */
  export async function purge(id: string): Promise<void> {
    const data = await load()
    data.projects = data.projects.filter((p) => p.id !== id)
    await save(data)
  }

  /**
   * Check if a slug is available
   */
  export async function isSlugAvailable(slug: string): Promise<boolean> {
    const data = await load()
    return !data.projects.some((p) => p.slug === slug && p.status !== "deleted")
  }

  /**
   * Generate a unique slug from a project name
   */
  export async function generateUniqueSlug(name: string): Promise<string> {
    // Convert to URL-safe slug
    let baseSlug = name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "")

    // Ensure it's not empty
    if (!baseSlug) {
      baseSlug = "project"
    }

    // Check if available
    if (await isSlugAvailable(baseSlug)) {
      return baseSlug
    }

    // Append number until unique
    let counter = 1
    while (!(await isSlugAvailable(`${baseSlug}-${counter}`))) {
      counter++
      if (counter > 100) {
        // Fallback to timestamp
        return `${baseSlug}-${Date.now()}`
      }
    }

    return `${baseSlug}-${counter}`
  }

  /**
   * Get the projects directory path
   */
  export function getProjectsDir(): string {
    return getWorkspacePathsFromEnv().projects
  }

  /**
   * Get the full path for a project slug
   */
  export function getProjectPath(slug: string): string {
    return path.join(getProjectsDir(), slug)
  }

  /**
   * Get recently accessed projects
   */
  export async function getRecent(limit = 5): Promise<ProjectEntry[]> {
    const projects = await list()
    return projects.sort((a, b) => b.lastAccessedAt - a.lastAccessedAt).slice(0, limit)
  }

  /**
   * Search projects by name or description
   */
  export async function search(query: string): Promise<ProjectEntry[]> {
    const projects = await list()
    const normalizedQuery = query.toLowerCase()

    return projects.filter(
      (p) =>
        p.name.toLowerCase().includes(normalizedQuery) ||
        p.description?.toLowerCase().includes(normalizedQuery) ||
        p.technology.some((t) => t.toLowerCase().includes(normalizedQuery)),
    )
  }
}
