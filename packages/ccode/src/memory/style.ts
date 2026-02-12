import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { Preferences } from "./preferences"
import path from "path"
import z from "zod"

const log = Log.create({ service: "memory.style" })

export namespace Style {
  export const EditChoice = z.object({
    id: z.string(),
    timestamp: z.number(),
    type: z.enum(["accept", "modify", "reject"]),
    fileType: z.string(),
    originalSuggestion: z.string().optional(),
    finalCode: z.string().optional(),
    reason: z.string().optional(),
  })
  export type EditChoice = z.infer<typeof EditChoice>

  export const StyleObservation = z.object({
    pattern: z.string(),
    confidence: z.number().min(0).max(1),
    sampleCount: z.number(),
    lastObserved: z.number(),
    samples: z.array(z.string()).max(10),
  })
  export type StyleObservation = z.infer<typeof StyleObservation>

  export const LearningState = z.object({
    projectID: z.string(),
    observations: z.array(StyleObservation),
    editChoices: z.array(EditChoice),
    detectedStyle: z.record(z.string(), z.any()),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type LearningState = z.infer<typeof LearningState>

  const STYLE_PATTERNS = {
    indentation: /^(\s+).*/,
    quotes: /(['"`])[^'"`]*\1/,
    semicolons: /;$/,
    trailingCommas: /,\s*[\])}]/,
    imports: /import\s+.*from/,
    exports: /export\s+(default\s+)?/,
    naming: {
      variable: /\b(let|const|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)/,
      function: /\b(function\s+)?([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(/,
      class: /\bclass\s+([A-Z][a-zA-Z0-9_$]*)/,
      component: /\b([A-Z][a-zA-Z0-9_$]*)\s*[={(]/,
    },
  }

  export async function get(): Promise<LearningState> {
    const projectID = Instance.project.id
    try {
      const stored = await Storage.read<LearningState>(["memory", "style", projectID])
      return stored
    } catch {
      return create()
    }
  }

  export async function create(): Promise<LearningState> {
    const projectID = Instance.project.id
    const now = Date.now()

    const result: LearningState = {
      projectID,
      observations: [],
      editChoices: [],
      detectedStyle: {},
      time: {
        created: now,
        updated: now,
      },
    }

    await save(result)

    const existingPrefs = await Preferences.get()
    if (Object.keys(existingPrefs.codeStyle || {}).length > 0) {
      await detectFromExistingPrefs(existingPrefs)
    }

    return result
  }

  async function detectFromExistingPrefs(preferences: Preferences.Info): Promise<void> {
    const style = preferences.codeStyle
    if (!style) return

    if (style.indentation?.type) {
      await recordObservation(`indentation_${style.indentation.type}`, 1.0, `Uses ${style.indentation.type}`)
    }

    if (style.quotes) {
      await recordObservation(`quotes_${style.quotes}`, 1.0, style.quotes)
    }

    if (style.semicolons !== undefined) {
      await recordObservation(
        `semicolons_${style.semicolons ? "present" : "absent"}`,
        1.0,
        style.semicolons ? "uses semicolons" : "no semicolons",
      )
    }
  }

  export async function save(state: LearningState): Promise<void> {
    const projectID = Instance.project.id
    state.time.updated = Date.now()
    await Storage.write(["memory", "style", projectID], state)
  }

  export async function recordEditChoice(choice: Omit<EditChoice, "id" | "timestamp">): Promise<void> {
    const state = await get()

    const editChoice: EditChoice = {
      id: `edit_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      timestamp: Date.now(),
      ...choice,
    }

    state.editChoices.push(editChoice)

    if (state.editChoices.length > 1000) {
      state.editChoices = state.editChoices.slice(-500)
    }

    await save(state)

    if (choice.type === "accept" && choice.finalCode) {
      await analyzeCodeForStyle(choice.finalCode, choice.fileType)
    } else if (choice.type === "modify" && choice.finalCode && choice.originalSuggestion) {
      await analyzeModification(choice.originalSuggestion, choice.finalCode, choice.fileType)
    }
  }

  async function analyzeCodeForStyle(code: string, fileType: string): Promise<void> {
    const lines = code.split("\n")

    for (const line of lines) {
      if (!line.trim()) continue

      const indentMatch = line.match(STYLE_PATTERNS.indentation)
      if (indentMatch) {
        const indent = indentMatch[1]
        if (indent.startsWith("  ")) {
          await recordObservation("indentation_spaces", 0.8, "2-space indentation")
        } else if (indent.startsWith("\t")) {
          await recordObservation("indentation_tabs", 0.9, "tab indentation")
        } else if (indent.startsWith("    ")) {
          await recordObservation("indentation_4spaces", 0.8, "4-space indentation")
        }
      }

      const quoteMatch = line.match(STYLE_PATTERNS.quotes)
      if (quoteMatch && line.includes("=")) {
        const quote = quoteMatch[1]
        if (quote !== "`") {
          await recordObservation(
            quote === "'" ? "quotes_single" : "quotes_double",
            0.7,
            `${quote === "'" ? "single" : "double"} quotes`,
          )
        }
      }

      if (line.trim().endsWith(";")) {
        await recordObservation("semicolons_present", 0.6, "uses semicolons")
      }

      if (STYLE_PATTERNS.trailingCommas.test(line)) {
        await recordObservation("trailing_commas", 0.7, "trailing commas")
      }
    }

    for (const patternType of Object.keys(STYLE_PATTERNS.naming)) {
      const regex = (STYLE_PATTERNS.naming as any)[patternType]
      const matches = code.match(new RegExp(regex, "g"))
      if (matches) {
        const names = matches
          .map((m: string) => {
            const nameMatch = m.match(regex)
            return nameMatch ? nameMatch[2] || nameMatch[1] : ""
          })
          .filter(Boolean)

        const camelCase = names.filter((n: string) => /^[a-z][a-zA-Z0-9]*$/.test(n)).length
        const pascalCase = names.filter((n: string) => /^[A-Z][a-zA-Z0-9]*$/.test(n)).length
        const snakeCase = names.filter((n: string) => /^[a-z][a-z0-9_]*$/.test(n)).length
        const upperSnake = names.filter((n: string) => /^[A-Z][A-Z0-9_]*$/.test(n)).length

        const total = names.length || 1
        if (camelCase / total > 0.7) {
          await recordObservation(`naming_${patternType}_camelCase`, 0.6, "camelCase naming")
        } else if (pascalCase / total > 0.7) {
          await recordObservation(`naming_${patternType}_PascalCase`, 0.6, "PascalCase naming")
        } else if (snakeCase / total > 0.7) {
          await recordObservation(`naming_${patternType}_snake_case`, 0.6, "snake_case naming")
        } else if (upperSnake / total > 0.7) {
          await recordObservation(`naming_${patternType}_UPPER_SNAKE`, 0.6, "UPPER_SNAKE_CASE naming")
        }
      }
    }
  }

  async function analyzeModification(original: string, modified: string, fileType: string): Promise<void> {
    const originalLines = original.split("\n")
    const modifiedLines = modified.split("\n")

    const originalQuotes = (original.match(/['"`]/g) || []).reduce(
      (acc, q) => {
        if (q === "`") return acc
        acc[q] = (acc[q] || 0) + 1
        return acc
      },
      {} as Record<string, number>,
    )

    const modifiedQuotes = (modified.match(/['"`]/g) || []).reduce(
      (acc, q) => {
        if (q === "`") return acc
        acc[q] = (acc[q] || 0) + 1
        return acc
      },
      {} as Record<string, number>,
    )

    if (originalQuotes["'"] && modifiedQuotes['"']) {
      await recordObservation("quotes_double", 0.9, "prefers double quotes (changed from single)")
    } else if (originalQuotes['"'] && modifiedQuotes["'"]) {
      await recordObservation("quotes_single", 0.9, "prefers single quotes (changed from double)")
    }

    const originalSemicolons = original.match(/;/g)?.length || 0
    const modifiedSemicolons = modified.match(/;/g)?.length || 0

    if (modifiedSemicolons > originalSemicolons) {
      await recordObservation("semicolons_present", 0.8, "adds semicolons (user preference)")
    } else if (modifiedSemicolons < originalSemicolons) {
      await recordObservation("semicolons_absent", 0.8, "removes semicolons (user preference)")
    }
  }

  export async function recordObservation(pattern: string, confidence: number, sample: string): Promise<void> {
    const state = await get()

    let existing = state.observations.find((o) => o.pattern === pattern)

    if (existing) {
      const weight = 0.3
      existing.confidence = Math.min(1, existing.confidence * (1 - weight) + confidence * weight)
      existing.sampleCount++
      existing.lastObserved = Date.now()

      if (!existing.samples.includes(sample) && existing.samples.length < 10) {
        existing.samples.push(sample)
      }
    } else {
      state.observations.push({
        pattern,
        confidence,
        sampleCount: 1,
        lastObserved: Date.now(),
        samples: [sample],
      })
    }

    await save(state)

    await updatePreferencesFromObservations()
  }

  async function updatePreferencesFromObservations(): Promise<void> {
    const state = await get()

    const updates: Partial<Preferences.CodeStyle> = {}

    const indentSpaces = state.observations.find((o) => o.pattern === "indentation_spaces")
    const indentTabs = state.observations.find((o) => o.pattern === "indentation_tabs")
    const indent4Spaces = state.observations.find((o) => o.pattern === "indentation_4spaces")

    if (indentTabs && indentTabs.confidence > 0.7) {
      updates.indentation = { type: "tabs" }
    } else if (indent4Spaces && indent4Spaces.confidence > 0.7) {
      updates.indentation = { type: "spaces", spaces: 4 }
    } else if (indentSpaces && indentSpaces.confidence > 0.7) {
      updates.indentation = { type: "spaces", spaces: 2 }
    }

    const singleQuotes = state.observations.find((o) => o.pattern === "quotes_single")
    const doubleQuotes = state.observations.find((o) => o.pattern === "quotes_double")

    if (singleQuotes && singleQuotes.confidence > 0.7) {
      updates.quotes = "single"
    } else if (doubleQuotes && doubleQuotes.confidence > 0.7) {
      updates.quotes = "double"
    }

    const semicolonsPresent = state.observations.find((o) => o.pattern === "semicolons_present")
    const semicolonsAbsent = state.observations.find((o) => o.pattern === "semicolons_absent")

    if (semicolonsPresent && semicolonsPresent.confidence > 0.7) {
      updates.semicolons = true
    } else if (semicolonsAbsent && semicolonsAbsent.confidence > 0.7) {
      updates.semicolons = false
    }

    const trailingCommas = state.observations.find((o) => o.pattern === "trailing_commas")
    if (trailingCommas && trailingCommas.confidence > 0.7) {
      updates.trailingCommas = true
    }

    if (Object.keys(updates).length > 0) {
      await Preferences.update({ codeStyle: updates })
    }
  }

  export async function getDetectedStyle(): Promise<Partial<Preferences.CodeStyle>> {
    const state = await get()
    const prefs = await Preferences.get()

    return {
      ...prefs.codeStyle,
      ...state.detectedStyle,
    }
  }

  export async function getConfidenceForPattern(pattern: string): Promise<number> {
    const state = await get()
    const observation = state.observations.find((o) => o.pattern === pattern)
    return observation?.confidence ?? 0
  }

  export async function getTopPatterns(limit = 10): Promise<StyleObservation[]> {
    const state = await get()
    return state.observations.sort((a, b) => b.confidence - a.confidence).slice(0, limit)
  }

  export function describeStyle(state: LearningState): string {
    const parts: string[] = []

    const topPatterns = state.observations.sort((a, b) => b.confidence - a.confidence).slice(0, 10)

    if (topPatterns.length > 0) {
      parts.push("Learned style patterns:")
      for (const pattern of topPatterns) {
        parts.push(`  - ${pattern.pattern} (${(pattern.confidence * 100).toFixed(0)}% confidence)`)
      }
    }

    parts.push(`\nTotal edit choices recorded: ${state.editChoices.length}`)

    return parts.join("\n") || "No style patterns learned yet"
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "style", projectID])
  }
}
