import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import z from "zod"

const log = Log.create({ service: "memory.knowledge.patterns" })

export namespace Patterns {
  export const CodePattern = z.object({
    id: z.string(),
    name: z.string(),
    category: z.enum([
      "error-handling",
      "async",
      "data-fetching",
      "state-management",
      "validation",
      "authentication",
      "authorization",
      "logging",
      "caching",
      "formatting",
      "routing",
      "database",
      "api",
      "component",
      "hook",
      "other",
    ]),
    template: z.string(),
    description: z.string().optional(),
    files: z.array(z.string()),
    frequency: z.number(),
    confidence: z.number().min(0).max(1),
    lastSeen: z.number(),
  })
  export type CodePattern = z.infer<typeof CodePattern>

  export const TeamConvention = z.object({
    id: z.string(),
    name: z.string(),
    type: z.enum(["naming", "structure", "import-order", "comment-style", "testing", "other"]),
    rule: z.string(),
    examples: z.array(z.string()),
    enforcement: z.enum(["strict", "recommended", "optional"]),
    created: z.number(),
    updated: z.number(),
  })
  export type TeamConvention = z.infer<typeof TeamConvention>

  export const AntiPattern = z.object({
    id: z.string(),
    name: z.string(),
    description: z.string(),
    severity: z.enum(["low", "medium", "high", "critical"]),
    files: z.array(z.string()),
    suggestions: z.array(z.string()),
    detected: z.number(),
  })
  export type AntiPattern = z.infer<typeof AntiPattern>

  export const PatternStore = z.object({
    projectID: z.string(),
    patterns: z.array(CodePattern),
    conventions: z.array(TeamConvention),
    antiPatterns: z.array(AntiPattern),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type PatternStore = z.infer<typeof PatternStore>

  const COMMON_PATTERNS = {
    "error-handling": [
      {
        name: "try-catch-async",
        template: "try { await operation() } catch (error) { handleError(error) }",
        description: "Standard async error handling",
      },
      {
        name: "error-boundary",
        template: "ErrorBoundary fallback UI",
        description: "React error boundary pattern",
      },
    ],
    async: [
      {
        name: "async-await",
        template: "async function() { await result }",
        description: "Async/await pattern",
      },
      {
        name: "promise-all",
        template: "await Promise.all([op1, op2])",
        description: "Parallel async operations",
      },
    ],
    "data-fetching": [
      {
        name: "use-fetch",
        template: "const { data, error, loading } = useFetch(url)",
        description: "Custom hook for data fetching",
      },
      {
        name: "swr-pattern",
        template: "useSWR(key, fetcher)",
        description: "Stale-while-revalidate pattern",
      },
    ],
    "state-management": [
      {
        name: "use-state",
        template: "const [state, setState] = useState(initial)",
        description: "React useState pattern",
      },
      {
        name: "use-reducer",
        template: "const [state, dispatch] = useReducer(reducer, initial)",
        description: "React useReducer pattern",
      },
      {
        name: "zustand-store",
        template: "const useStore = create((set) => ({ ... }))",
        description: "Zustand store pattern",
      },
    ],
    validation: [
      {
        name: "schema-validation",
        template: "Schema.parse(data)",
        description: "Schema validation with Zod",
      },
      {
        name: "form-validation",
        template: "validate: { rules }",
        description: "Form validation pattern",
      },
    ],
    authentication: [
      {
        name: "protected-route",
        template: "ProtectedRoute component with auth check",
        description: "Route protection with authentication",
      },
      {
        name: "auth-guard",
        template: "AuthGuard HOC or hook",
        description: "Higher-order component for auth",
      },
    ],
  }

  export async function get(): Promise<PatternStore> {
    const projectID = Instance.project.id
    try {
      const stored = await Storage.read<PatternStore>(["memory", "knowledge", "patterns", projectID])
      return stored
    } catch {
      return create()
    }
  }

  export async function create(): Promise<PatternStore> {
    const projectID = Instance.project.id
    const now = Date.now()

    const result: PatternStore = {
      projectID,
      patterns: [],
      conventions: [],
      antiPatterns: [],
      time: {
        created: now,
        updated: now,
      },
    }

    await save(result)
    return result
  }

  export async function save(store: PatternStore): Promise<void> {
    const projectID = Instance.project.id
    store.time.updated = Date.now()
    await Storage.write(["memory", "knowledge", "patterns", projectID], store)
  }

  export async function recordPattern(
    category: CodePattern["category"],
    name: string,
    template: string,
    filePath: string,
  ): Promise<void> {
    const store = await get()

    let pattern = store.patterns.find((p) => p.name === name && p.category === category)

    if (pattern) {
      pattern.frequency++
      pattern.lastSeen = Date.now()
      if (!pattern.files.includes(filePath)) {
        pattern.files.push(filePath)
      }
    } else {
      pattern = {
        id: `pattern_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
        name,
        category,
        template,
        frequency: 1,
        confidence: 0.5,
        lastSeen: Date.now(),
        files: [filePath],
      }
      store.patterns.push(pattern)
    }

    await save(store)
  }

  export async function getPatterns(category?: CodePattern["category"], minConfidence = 0.5): Promise<CodePattern[]> {
    const store = await get()

    let patterns = store.patterns

    if (category) {
      patterns = patterns.filter((p) => p.category === category)
    }

    return patterns.filter((p) => p.confidence >= minConfidence).sort((a, b) => b.frequency - a.frequency)
  }

  export async function getPattern(name: string): Promise<CodePattern | undefined> {
    const store = await get()
    return store.patterns.find((p) => p.name === name)
  }

  export async function addConvention(
    name: string,
    type: TeamConvention["type"],
    rule: string,
    examples: string[] = [],
    enforcement: TeamConvention["enforcement"] = "recommended",
  ): Promise<TeamConvention> {
    const store = await get()

    const convention: TeamConvention = {
      id: `conv_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      name,
      type,
      rule,
      examples,
      enforcement,
      created: Date.now(),
      updated: Date.now(),
    }

    store.conventions.push(convention)

    if (store.conventions.length > 50) {
      store.conventions = store.conventions.sort((a, b) => b.updated - a.updated).slice(0, 50)
    }

    await save(store)
    return convention
  }

  export async function getConventions(type?: TeamConvention["type"]): Promise<TeamConvention[]> {
    const store = await get()

    let conventions = store.conventions

    if (type) {
      conventions = conventions.filter((c) => c.type === type)
    }

    return conventions.sort((a, b) => {
      const enforcementOrder = { strict: 0, recommended: 1, optional: 2 }
      const aOrder = enforcementOrder[a.enforcement]
      const bOrder = enforcementOrder[b.enforcement]
      if (aOrder !== bOrder) return aOrder - bOrder
      return b.updated - a.updated
    })
  }

  export async function recordAntiPattern(
    name: string,
    description: string,
    severity: AntiPattern["severity"],
    filePath: string,
    suggestions: string[] = [],
  ): Promise<void> {
    const store = await get()

    let antiPattern = store.antiPatterns.find((p) => p.name === name)

    if (antiPattern) {
      antiPattern.detected++
      if (!antiPattern.files.includes(filePath)) {
        antiPattern.files.push(filePath)
      }
    } else {
      antiPattern = {
        id: `antipattern_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
        name,
        description,
        severity,
        files: [filePath],
        suggestions,
        detected: 1,
      }
      store.antiPatterns.push(antiPattern)
    }

    await save(store)
  }

  export async function getAntiPatterns(minSeverity: AntiPattern["severity"] = "low"): Promise<AntiPattern[]> {
    const store = await get()

    const severityOrder = { low: 0, medium: 1, high: 2, critical: 3 }
    const minSeverityValue = severityOrder[minSeverity]

    return store.antiPatterns
      .filter((ap) => severityOrder[ap.severity] >= minSeverityValue)
      .sort((a, b) => severityOrder[b.severity] - severityOrder[a.severity])
  }

  export async function detectCommonPatterns(): Promise<void> {
    log.info("detecting common patterns")

    const store = await get()

    for (const [category, patterns] of Object.entries(COMMON_PATTERNS)) {
      for (const pattern of patterns) {
        const existing = store.patterns.find((p) => p.name === pattern.name && p.category === category)

        if (!existing) {
          store.patterns.push({
            id: `pattern_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
            name: pattern.name,
            category: category as CodePattern["category"],
            template: pattern.template,
            description: pattern.description,
            frequency: 0,
            confidence: 0.3,
            lastSeen: Date.now(),
            files: [],
          })
        }
      }
    }

    await save(store)
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "knowledge", "patterns", projectID])
  }
}
