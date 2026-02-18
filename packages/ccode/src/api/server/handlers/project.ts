/**
 * Project API Handler
 * Handles /api/projects/* endpoints
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { Project } from "@/project/project"
import { Storage } from "@/storage/storage"
import z from "zod"

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/projects
 * List all projects
 */
export async function listProjects(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const projects = await Project.list()

    // Sort by updated time, most recent first
    const sorted = projects.sort((a, b) => b.time.updated - a.time.updated)

    return jsonResponse({
      success: true,
      data: sorted,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/projects/:id
 * Get a specific project by ID
 */
export async function getProject(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Project ID is required", 400)
    }

    const project = await Storage.read<Project.Info>(["project", id]).catch(() => null)

    if (!project) {
      return errorResponse("Project not found", 404)
    }

    return jsonResponse({
      success: true,
      data: project,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/projects
 * Create a new project from a directory
 */
export async function createProject(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as { directory: string; name?: string }

    if (!input.directory) {
      return errorResponse("Directory is required", 400)
    }

    // Use Project.fromDirectory to create/get the project
    const { project } = await Project.fromDirectory(input.directory)

    // Update name if provided
    if (input.name) {
      await Project.update({
        projectID: project.id,
        name: input.name,
      })
      project.name = input.name
    }

    return jsonResponse(
      {
        success: true,
        data: project,
      },
      201,
    )
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PATCH /api/projects/:id
 * Update a project
 */
export async function updateProject(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Project ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as {
      name?: string
      icon?: { url?: string; override?: string; color?: string }
    }

    const project = await Project.update({
      projectID: id,
      name: input.name,
      icon: input.icon,
    })

    return jsonResponse({
      success: true,
      data: project,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/projects/:id
 * Delete a project (removes project metadata, not files)
 */
export async function deleteProject(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Project ID is required", 400)
    }

    // Don't allow deleting the global project
    if (id === "global") {
      return errorResponse("Cannot delete the global project", 400)
    }

    await Storage.remove(["project", id])

    return jsonResponse({
      success: true,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/projects/:id/sessions
 * Get all sessions for a project
 */
export async function getProjectSessions(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Project ID is required", 400)
    }

    // List sessions for this project
    const sessionKeys = await Storage.list(["session", id])
    const sessions = await Promise.all(
      sessionKeys.map((key) => Storage.read(key).catch(() => null))
    )

    const validSessions = sessions
      .filter((s): s is NonNullable<typeof s> => s !== null)
      .sort((a: any, b: any) => b.time.updated - a.time.updated)

    return jsonResponse({
      success: true,
      data: validSessions,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
