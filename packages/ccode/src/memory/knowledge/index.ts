import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import z from "zod"

const log = Log.create({ service: "memory.knowledge" })

export namespace Knowledge {
  export const ApiEndpoint = z.object({
    path: z.string(),
    method: z.string(),
    file: z.string(),
    description: z.string().optional(),
    handler: z.string().optional(),
    middleware: z.array(z.string()).optional(),
    authRequired: z.boolean().optional(),
  })
  export type ApiEndpoint = z.infer<typeof ApiEndpoint>

  export const DataModel = z.object({
    name: z.string(),
    file: z.string(),
    type: z.enum(["interface", "type", "class", "enum"]),
    properties: z
      .array(
        z.object({
          name: z.string(),
          type: z.string(),
          optional: z.boolean().optional(),
        }),
      )
      .optional(),
    extends: z.array(z.string()).optional(),
    related: z.array(z.string()).optional(),
  })
  export type DataModel = z.infer<typeof DataModel>

  export const ComponentInfo = z.object({
    name: z.string(),
    file: z.string(),
    type: z.enum(["component", "page", "layout", "hook", "util"]),
    props: z
      .array(
        z.object({
          name: z.string(),
          type: z.string(),
          required: z.boolean().optional(),
        }),
      )
      .optional(),
    usage: z.array(z.string()).optional(),
    description: z.string().optional(),
  })
  export type ComponentInfo = z.infer<typeof ComponentInfo>

  export const EnvironmentVariable = z.object({
    name: z.string(),
    type: z.enum(["string", "number", "boolean", "url"]),
    required: z.boolean(),
    default: z.string().optional(),
    description: z.string().optional(),
  })
  export type EnvironmentVariable = z.infer<typeof EnvironmentVariable>

  export const ProjectNote = z.object({
    id: z.string(),
    title: z.string(),
    content: z.string(),
    tags: z.array(z.string()),
    category: z.enum(["architecture", "convention", "troubleshooting", "explanation", "other"]),
    created: z.number(),
    updated: z.number(),
  })
  export type ProjectNote = z.infer<typeof ProjectNote>

  export const Info = z.object({
    projectID: z.string(),
    apiEndpoints: z.array(ApiEndpoint),
    dataModels: z.array(DataModel),
    components: z.array(ComponentInfo),
    environmentVariables: z.array(EnvironmentVariable),
    notes: z.array(ProjectNote),
    summaries: z.record(z.string(), z.string()),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type Info = z.infer<typeof Info>

  export async function get(): Promise<Info> {
    const projectID = Instance.project.id
    try {
      const stored = await Storage.read<Info>(["memory", "knowledge", projectID])
      return stored
    } catch {
      return create()
    }
  }

  export async function create(): Promise<Info> {
    const projectID = Instance.project.id
    const now = Date.now()

    const result: Info = {
      projectID,
      apiEndpoints: [],
      dataModels: [],
      components: [],
      environmentVariables: [],
      notes: [],
      summaries: {},
      time: {
        created: now,
        updated: now,
      },
    }

    await save(result)
    return result
  }

  export async function save(knowledge: Info): Promise<void> {
    const projectID = Instance.project.id
    knowledge.time.updated = Date.now()
    await Storage.write(["memory", "knowledge", projectID], knowledge)
  }

  export async function update(updates: Partial<Info>): Promise<Info> {
    const knowledge = await get()
    Object.assign(knowledge, updates)
    knowledge.time.updated = Date.now()
    await save(knowledge)
    return knowledge
  }

  export async function addApiEndpoint(endpoint: ApiEndpoint): Promise<void> {
    const knowledge = await get()

    const existingIndex = knowledge.apiEndpoints.findIndex(
      (e) => e.path === endpoint.path && e.method === endpoint.method,
    )

    if (existingIndex >= 0) {
      knowledge.apiEndpoints[existingIndex] = endpoint
    } else {
      knowledge.apiEndpoints.push(endpoint)
    }

    await save(knowledge)
  }

  export async function addDataModel(model: DataModel): Promise<void> {
    const knowledge = await get()

    const existingIndex = knowledge.dataModels.findIndex((m) => m.name === model.name)

    if (existingIndex >= 0) {
      knowledge.dataModels[existingIndex] = model
    } else {
      knowledge.dataModels.push(model)
    }

    await save(knowledge)
  }

  export async function addComponent(component: ComponentInfo): Promise<void> {
    const knowledge = await get()

    const existingIndex = knowledge.components.findIndex((c) => c.name === component.name)

    if (existingIndex >= 0) {
      knowledge.components[existingIndex] = component
    } else {
      knowledge.components.push(component)
    }

    await save(knowledge)
  }

  export async function addEnvironmentVariable(envVar: EnvironmentVariable): Promise<void> {
    const knowledge = await get()

    const existingIndex = knowledge.environmentVariables.findIndex((e) => e.name === envVar.name)

    if (existingIndex >= 0) {
      knowledge.environmentVariables[existingIndex] = envVar
    } else {
      knowledge.environmentVariables.push(envVar)
    }

    await save(knowledge)
  }

  export async function addNote(note: Omit<ProjectNote, "id" | "created" | "updated">): Promise<ProjectNote> {
    const knowledge = await get()

    const newNote: ProjectNote = {
      id: `note_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      ...note,
      created: Date.now(),
      updated: Date.now(),
    }

    knowledge.notes.push(newNote)

    if (knowledge.notes.length > 100) {
      knowledge.notes = knowledge.notes.sort((a, b) => b.updated - a.updated).slice(0, 100)
    }

    await save(knowledge)
    return newNote
  }

  export async function updateNote(
    id: string,
    updates: Partial<Omit<ProjectNote, "id" | "created">>,
  ): Promise<ProjectNote | undefined> {
    const knowledge = await get()

    const note = knowledge.notes.find((n) => n.id === id)
    if (!note) return undefined

    Object.assign(note, updates, { updated: Date.now() })

    await save(knowledge)
    return note
  }

  export async function deleteNote(id: string): Promise<boolean> {
    const knowledge = await get()

    const index = knowledge.notes.findIndex((n) => n.id === id)
    if (index < 0) return false

    knowledge.notes.splice(index, 1)
    await save(knowledge)
    return true
  }

  export async function setSummary(key: string, summary: string): Promise<void> {
    const knowledge = await get()
    knowledge.summaries[key] = summary

    if (Object.keys(knowledge.summaries).length > 50) {
      const entries = Object.entries(knowledge.summaries)
      entries.sort((a, b) => a[0].localeCompare(b[0]))
      knowledge.summaries = Object.fromEntries(entries.slice(0, 50))
    }

    await save(knowledge)
  }

  export async function getSummary(key: string): Promise<string | undefined> {
    const knowledge = await get()
    return knowledge.summaries[key]
  }

  export async function getApiEndpoints(): Promise<ApiEndpoint[]> {
    const knowledge = await get()
    return knowledge.apiEndpoints
  }

  export async function getDataModels(): Promise<DataModel[]> {
    const knowledge = await get()
    return knowledge.dataModels
  }

  export async function getComponents(): Promise<ComponentInfo[]> {
    const knowledge = await get()
    return knowledge.components
  }

  export async function getEnvironmentVariables(): Promise<EnvironmentVariable[]> {
    const knowledge = await get()
    return knowledge.environmentVariables
  }

  export async function getNotes(category?: ProjectNote["category"], tags?: string[]): Promise<ProjectNote[]> {
    const knowledge = await get()
    let notes = knowledge.notes

    if (category) {
      notes = notes.filter((n) => n.category === category)
    }

    if (tags && tags.length > 0) {
      notes = notes.filter((n) => tags.some((t) => n.tags.includes(t)))
    }

    return notes.sort((a, b) => b.updated - a.updated)
  }

  export async function search(query: string): Promise<{
    apiEndpoints: ApiEndpoint[]
    dataModels: DataModel[]
    components: ComponentInfo[]
    notes: ProjectNote[]
  }> {
    const knowledge = await get()
    const lowerQuery = query.toLowerCase()

    return {
      apiEndpoints: knowledge.apiEndpoints.filter(
        (e) =>
          e.path.toLowerCase().includes(lowerQuery) ||
          e.handler?.toLowerCase().includes(lowerQuery) ||
          e.description?.toLowerCase().includes(lowerQuery),
      ),
      dataModels: knowledge.dataModels.filter(
        (m) => m.name.toLowerCase().includes(lowerQuery) || m.file.toLowerCase().includes(lowerQuery),
      ),
      components: knowledge.components.filter(
        (c) =>
          c.name.toLowerCase().includes(lowerQuery) ||
          c.file.toLowerCase().includes(lowerQuery) ||
          c.description?.toLowerCase().includes(lowerQuery),
      ),
      notes: knowledge.notes.filter(
        (n) =>
          n.title.toLowerCase().includes(lowerQuery) ||
          n.content.toLowerCase().includes(lowerQuery) ||
          n.tags.some((t) => t.toLowerCase().includes(lowerQuery)),
      ),
    }
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "knowledge", projectID])
  }
}
