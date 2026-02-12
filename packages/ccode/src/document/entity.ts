import { Storage } from "../storage/storage"
import { Identifier } from "../id/id"
import { Instance } from "../project/instance"
import { DocumentSchema } from "./schema"

const state = Instance.state(async () => ({}))

export namespace Entity {
  const STORAGE_PREFIX = "document_entity"

  export async function create(input: {
    documentID: string
    type: DocumentSchema.EntityType
    name: string
    description: string
    firstAppearedChapterID: string
    aliases?: string[]
    attributes?: Record<string, string>
  }): Promise<DocumentSchema.Entity> {
    await state()
    const id = Identifier.create("entity" as const, false)

    const entity: DocumentSchema.Entity = {
      id,
      type: input.type,
      name: input.name,
      aliases: input.aliases ?? [],
      description: input.description,
      firstAppearedChapterID: input.firstAppearedChapterID,
      attributes: input.attributes ?? {},
      relationships: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    }

    await Storage.write([STORAGE_PREFIX, input.documentID, id], entity)
    return entity
  }

  export async function get(documentID: string, entityID: string): Promise<DocumentSchema.Entity | undefined> {
    await state()
    try {
      return await Storage.read<DocumentSchema.Entity>([STORAGE_PREFIX, documentID, entityID])
    } catch {
      return undefined
    }
  }

  export async function list(documentID: string): Promise<DocumentSchema.Entity[]> {
    await state()
    const keys = await Storage.list([STORAGE_PREFIX, documentID])
    const entities: DocumentSchema.Entity[] = []

    for (const key of keys) {
      const entity = await Storage.read<DocumentSchema.Entity>(key).catch(() => undefined)
      if (entity) entities.push(entity)
    }

    return entities.sort((a, b) => b.createdAt - a.createdAt)
  }

  export async function listByType(
    documentID: string,
    type: DocumentSchema.EntityType,
  ): Promise<DocumentSchema.Entity[]> {
    const all = await list(documentID)
    return all.filter((e) => e.type === type)
  }

  export async function update(input: {
    documentID: string
    entityID: string
    name?: string
    description?: string
    aliases?: string[]
    attributes?: Record<string, string>
    relationships?: Array<{
      targetEntityID: string
      type: string
      description: string
    }>
  }): Promise<void> {
    await state()
    const entity = await get(input.documentID, input.entityID)
    if (!entity) throw new Error("Entity not found")

    // Use immutable update pattern
    const updated: DocumentSchema.Entity = {
      ...entity,
      ...(input.name && { name: input.name }),
      ...(input.description && { description: input.description }),
      ...(input.aliases && { aliases: input.aliases }),
      ...(input.attributes && { attributes: { ...entity.attributes, ...input.attributes } }),
      ...(input.relationships && { relationships: input.relationships }),
      updatedAt: Date.now(),
    }

    await Storage.write([STORAGE_PREFIX, input.documentID, input.entityID], updated)
  }

  export async function remove(documentID: string, entityID: string): Promise<void> {
    await state()
    await Storage.remove([STORAGE_PREFIX, documentID, entityID])
  }

  /**
   * Extract entities from chapter content using AI
   * Returns a prompt that can be sent to an AI to extract entities
   */
  export async function extractEntitiesPrompt(documentID: string, chapterID: string): Promise<string> {
    const chapter = await getChapterForExtraction(documentID, chapterID)
    if (!chapter?.content) throw new Error("Chapter has no content")

    const existingEntities = await list(documentID)
    const existingNames = new Set(
      existingEntities.flatMap((e) => [e.name, ...e.aliases]),
    )

    const lines: string[] = []
    lines.push("# Entity Extraction Task")
    lines.push("")
    lines.push("Extract all entities (characters, locations, concepts, items, events) from the following chapter content.")
    lines.push("")
    lines.push("## Entity Types")
    lines.push("")
    lines.push("- **character**: People, sentient beings")
    lines.push("- **location**: Places, buildings, geographical areas")
    lines.push("- **concept**: Ideas, organizations, abstract concepts")
    lines.push("- **item**: Objects, weapons, tools, significant items")
    lines.push("- **event**: Incidents, battles, meetings, important events")
    lines.push("")
    lines.push("## Existing Entities (already tracked)")
    lines.push("")
    if (existingNames.size > 0) {
      lines.push(`Do NOT duplicate these existing entities: ${Array.from(existingNames).join(", ")}`)
    } else {
      lines.push("(None - this is the first chapter)")
    }
    lines.push("")
    lines.push("## Chapter Content")
    lines.push("")
    lines.push(`**Title:** ${chapter.title}`)
    lines.push("")
    lines.push(chapter.content.slice(0, 8000)) // Limit content size
    lines.push("")
    lines.push("## Output Format")
    lines.push("")
    lines.push("Return a JSON array of entities. For NEW entities only:")
    lines.push("")
    lines.push("```json")
    lines.push("[")
    lines.push('  {')
    lines.push('    "type": "character|location|concept|item|event",')
    lines.push('    "name": "Entity Name",')
    lines.push('    "aliases": ["Alternative name 1", "Alternative name 2"],')
    lines.push('    "description": "Brief description (1-2 sentences)",')
    lines.push('    "attributes": {')
    lines.push('      "key_attribute": "value",')
    lines.push('      "another_attribute": "value"')
    lines.push('    }')
    lines.push("  }")
    lines.push("]")
    lines.push('```')
    lines.push("")
    lines.push("Also provide a list of all entity names mentioned in this chapter (including existing ones):")
    lines.push("")
    lines.push("```json")
    lines.push("{")
    lines.push('  "mentionedEntityIDs": ["entity_id_or_name_1", "entity_id_or_name_2"]')
    lines.push("}")
    lines.push("```")

    return lines.join("\n")
  }

  /**
   * Parse AI response and create/update entities
   */
  export async function processExtractionResponse(
    documentID: string,
    chapterID: string,
    aiResponse: string,
  ): Promise<{ created: number; updated: number; mentioned: string[] }> {
    const chapter = await getChapterForExtraction(documentID, chapterID)
    if (!chapter) throw new Error("Chapter not found")

    const existingEntities = await list(documentID)
    const entityMap = new Map(existingEntities.map((e) => [e.name.toLowerCase(), e]))

    let created = 0
    let updated = 0
    const mentioned: string[] = []

    try {
      // Try to extract JSON from response
      const jsonMatch = aiResponse.match(/```json\s*([\s\S]*?)\s*```/) || aiResponse.match(/\[[\s\S]*\]/)
      if (!jsonMatch) throw new Error("No JSON found in response")

      const entitiesData = JSON.parse(jsonMatch[1] || jsonMatch[0])

      // Process new entities
      for (const data of entitiesData) {
        const normalizedName = data.name.toLowerCase()
        const existing = entityMap.get(normalizedName)

        if (existing) {
          // Update existing entity with new attributes
          await update({
            documentID,
            entityID: existing.id,
            attributes: { ...existing.attributes, ...data.attributes },
          })
          updated++
          mentioned.push(existing.id)
        } else {
          // Create new entity
          const newEntity = await create({
            documentID,
            type: data.type,
            name: data.name,
            description: data.description,
            firstAppearedChapterID: chapterID,
            aliases: data.aliases || [],
            attributes: data.attributes || {},
          })
          created++
          mentioned.push(newEntity.id)
        }
      }

      // Update chapter with mentioned entity IDs
      // Note: This would require extending the Chapter update function
      // For now, we'll return the list to be used by the caller

      return { created, updated, mentioned }
    } catch (error) {
      throw new Error(`Failed to parse extraction response: ${error}`)
    }
  }

  /**
   * Find potential duplicate entities that should be merged
   */
  export async function findDuplicates(documentID: string): Promise<
    Array<{
      entities: DocumentSchema.Entity[]
      reason: string
      confidence: number
    }>
  > {
    const entities = await list(documentID)
    const duplicates: Array<{
      entities: DocumentSchema.Entity[]
      reason: string
      confidence: number
    }> = []

    // Check by name similarity
    for (let i = 0; i < entities.length; i++) {
      for (let j = i + 1; j < entities.length; j++) {
        const a = entities[i]
        const b = entities[j]

        // Same type
        if (a.type !== b.type) continue

        // Exact name match (shouldn't happen but check anyway)
        if (a.name.toLowerCase() === b.name.toLowerCase()) {
          duplicates.push({
            entities: [a, b],
            reason: "Exact name match",
            confidence: 1.0,
          })
          continue
        }

        // Alias match
        const aAliases = [a.name.toLowerCase(), ...a.aliases.map((x) => x.toLowerCase())]
        const bAliases = [b.name.toLowerCase(), ...b.aliases.map((x) => x.toLowerCase())]
        const aliasIntersection = aAliases.filter((x) => bAliases.includes(x))

        if (aliasIntersection.length > 0) {
          duplicates.push({
            entities: [a, b],
            reason: `Shared alias: ${aliasIntersection.join(", ")}`,
            confidence: 0.95,
          })
          continue
        }

        // Similar names (Levenshtein distance simplified)
        const similarity = calculateStringSimilarity(a.name.toLowerCase(), b.name.toLowerCase())
        if (similarity > 0.85) {
          duplicates.push({
            entities: [a, b],
            reason: `Similar names (${Math.round(similarity * 100)}% match)`,
            confidence: similarity,
          })
        }
      }
    }

    return duplicates
  }

  /**
   * Merge duplicate entities
   */
  export async function mergeEntities(input: {
    documentID: string
    keepEntityID: string
    mergeEntityIDs: string[]
  }): Promise<DocumentSchema.Entity> {
    await state()
    const keepEntity = await get(input.documentID, input.keepEntityID)
    if (!keepEntity) throw new Error("Entity to keep not found")

    const allEntities = await list(input.documentID)
    const mergedEntities = input.mergeEntityIDs
      .map((id) => allEntities.find((e) => e.id === id))
      .filter((e): e is DocumentSchema.Entity => e !== undefined)

    if (mergedEntities.length === 0) return keepEntity

    // Combine aliases
    const allAliases = new Set([...keepEntity.aliases])
    for (const entity of mergedEntities) {
      allAliases.add(entity.name)
      for (const alias of entity.aliases) allAliases.add(alias)
    }

    // Combine attributes (keep entity takes precedence)
    // Use spread operator instead of Object.assign for immutability
    const combinedAttributes: Record<string, string> = {}
    for (const entity of [...mergedEntities, keepEntity]) {
      for (const [key, value] of Object.entries(entity.attributes)) {
        combinedAttributes[key] = value
      }
    }

    // Combine relationships
    const combinedRelationships = [...keepEntity.relationships]
    for (const entity of mergedEntities) {
      for (const rel of entity.relationships) {
        // Avoid duplicates
        const exists = combinedRelationships.some(
          (r) =>
            r.targetEntityID === rel.targetEntityID &&
            r.type === rel.type,
        )
        if (!exists) combinedRelationships.push(rel)
      }
    }

    // Update the kept entity
    await update({
      documentID: input.documentID,
      entityID: input.keepEntityID,
      aliases: Array.from(allAliases),
      attributes: combinedAttributes,
      relationships: combinedRelationships,
    })

    // Delete merged entities
    for (const entity of mergedEntities) {
      await remove(input.documentID, entity.id)
    }

    return (await get(input.documentID, input.keepEntityID))!
  }

  /**
   * Update entity relationships based on chapter content
   */
  export async function updateRelationshipsPrompt(documentID: string, chapterID: string): Promise<string> {
    const chapter = await getChapterForExtraction(documentID, chapterID)
    if (!chapter?.content) throw new Error("Chapter has no content")

    const entities = await list(documentID)
    const entityList = entities
      .map((e) => `- ${e.id}: ${e.name} (${e.type})`)
      .join("\n")

    const lines: string[] = []
    lines.push("# Relationship Extraction Task")
    lines.push("")
    lines.push("Analyze the chapter content for relationships between entities.")
    lines.push("")
    lines.push("## Known Entities")
    lines.push("")
    lines.push(entityList)
    lines.push("")
    lines.push("## Chapter Content")
    lines.push("")
    lines.push(`**Title:** ${chapter.title}`)
    lines.push("")
    lines.push(chapter.content.slice(0, 8000))
    lines.push("")
    lines.push("## Output Format")
    lines.push("")
    lines.push("Return a JSON array of relationships found or updated in this chapter:")
    lines.push("")
    lines.push("```json")
    lines.push("[")
    lines.push('  {')
    lines.push('    "sourceEntityID": "entity_id",')
    lines.push('    "targetEntityID": "target_entity_id",')
    lines.push('    "type": "relationship_type",')
    lines.push('    "description": "Brief description of the relationship"')
    lines.push("  }")
    lines.push("]")
    lines.push("```")

    return lines.join("\n")
  }

  /**
   * Find entity conflicts (inconsistent attributes, etc.)
   */
  export async function findConflicts(documentID: string): Promise<
    Array<{
      entityID: string
      entityName: string
      conflictType: string
      description: string
      severity: "low" | "medium" | "high"
    }>
  > {
    const entities = await list(documentID)
    const conflicts: Array<{
      entityID: string
      entityName: string
      conflictType: string
      description: string
      severity: "low" | "medium" | "high"
    }> = []

    // Check characters with conflicting attributes
    const characters = entities.filter((e) => e.type === "character")
    for (const char of characters) {
      // Check for age inconsistency (if age is tracked)
      if (char.attributes.age && char.attributes.ageAtIntroduction) {
        const age = parseInt(char.attributes.age)
        const introAge = parseInt(char.attributes.ageAtIntroduction)
        if (!isNaN(age) && !isNaN(introAge) && age < introAge) {
          conflicts.push({
            entityID: char.id,
            entityName: char.name,
            conflictType: "attribute_conflict",
            description: `Character age (${age}) is less than age at introduction (${introAge})`,
            severity: "high",
          })
        }
      }

      // Check for status inconsistency (dead vs alive)
      if (char.attributes.status === "dead" && char.attributes.lastSeen === "alive") {
        conflicts.push({
          entityID: char.id,
          entityName: char.name,
          conflictType: "status_conflict",
          description: "Character marked as dead but last seen alive",
          severity: "high",
        })
      }
    }

    return conflicts
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  async function getChapterForExtraction(
    documentID: string,
    chapterID: string,
  ): Promise<DocumentSchema.Chapter | undefined> {
    // Import Chapter namespace directly to avoid circular dependency
    const chapterModule = await import("./index")
    // The Chapter namespace is exposed via Document but we need to access it differently
    // For now, use a direct storage read
    const { Storage } = await import("../storage/storage")
    try {
      return await Storage.read<DocumentSchema.Chapter>(["document_chapter", documentID, chapterID])
    } catch {
      return undefined
    }
  }

  function calculateStringSimilarity(a: string, b: string): number {
    if (a === b) return 1
    if (a.length === 0 || b.length === 0) return 0

    // Simple similarity based on common prefix/suffix and containment
    const maxLength = Math.max(a.length, b.length)
    let common = 0

    // Check prefix
    let i = 0
    while (i < a.length && i < b.length && a[i] === b[i]) {
      common++
      i++
    }

    // Check suffix
    let j = 1
    while (j <= a.length - i && j <= b.length - i && a[a.length - j] === b[b.length - j]) {
      common++
      j++
    }

    return common / maxLength
  }
}
