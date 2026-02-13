import { Document } from "./index"
import { DocumentSchema } from "./schema"
import { Entity } from "./entity"

export namespace Consistency {
  /**
   * Generate AI prompt for consistency checking
   */
  export async function checkPrompt(input: {
    documentID: string
    chapterID?: string
    checkTypes?: Array<"entity" | "plot" | "style" | "continuity">
  }): Promise<string> {
    const doc = await Document.get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Document.Chapter.list(input.documentID)
    const entities = await Entity.list(input.documentID)

    const checkTypes = input.checkTypes || ["entity", "plot", "style", "continuity"]

    let chaptersToCheck: DocumentSchema.Chapter[]
    if (input.chapterID) {
      const ch = chapters.find((c) => c.id === input.chapterID)
      chaptersToCheck = ch ? [ch] : []
    } else {
      chaptersToCheck = chapters.filter((c) => c.status === "completed")
    }

    if (chaptersToCheck.length === 0) {
      throw new Error("No chapters to check")
    }

    const lines: string[] = []

    lines.push("# Document Consistency Check")
    lines.push("")
    lines.push("## Document Information")
    lines.push("")
    lines.push(`**Title:** ${doc.title}`)
    lines.push(`**Chapters to Check:** ${chaptersToCheck.length}`)
    lines.push("")
    lines.push("## Check Types")
    lines.push("")

    if (checkTypes.includes("entity")) {
      lines.push("- ✓ **Entity Consistency**: Character names, traits, relationships")
    }
    if (checkTypes.includes("plot")) {
      lines.push("- ✓ **Plot Consistency**: Timeline, cause-effect, contradictions")
    }
    if (checkTypes.includes("style")) {
      lines.push("- ✓ **Style Consistency**: Tone, voice, language patterns")
    }
    if (checkTypes.includes("continuity")) {
      lines.push("- ✓ **Continuity**: Scene transitions, references, callbacks")
    }
    lines.push("")

    // Global summary for context
    if (doc.globalSummary) {
      lines.push("## Global Summary (for reference)")
      lines.push("")
      lines.push(doc.globalSummary.overallPlot)
      lines.push("")
      if (doc.globalSummary.keyArcs.length > 0) {
        lines.push("**Key Arcs:**")
        for (const arc of doc.globalSummary.keyArcs) {
          lines.push(`- ${arc.name} (${arc.status})`)
        }
        lines.push("")
      }
    }

    // Entity reference
    if (checkTypes.includes("entity") && entities.length > 0) {
      lines.push("## Tracked Entities")
      lines.push("")
      for (const entity of entities.slice(0, 20)) {
        lines.push(`- **${entity.name}** (${entity.type}): ${entity.description}`)
        if (entity.attributes && Object.keys(entity.attributes).length > 0) {
          const attrs = Object.entries(entity.attributes)
            .slice(0, 3)
            .map(([k, v]) => `${k}=${v}`)
            .join(", ")
          lines.push(`  Attributes: ${attrs}`)
        }
      }
      lines.push("")
    }

    // Chapter summaries for context
    lines.push("## Chapter Summaries")
    lines.push("")
    for (const chapter of chaptersToCheck) {
      lines.push(`### ${chapter.title}`)
      if (chapter.summary) {
        lines.push(chapter.summary)
      } else if (chapter.content) {
        lines.push(chapter.content.slice(0, 500))
      }
      lines.push("")
    }

    // Include full content for the most recent chapter if checking a specific range
    if (chaptersToCheck.length <= 3) {
      lines.push("---")
      lines.push("")
      lines.push("## Full Chapter Content(s)")
      lines.push("")
      for (const chapter of chaptersToCheck) {
        lines.push(`### ${chapter.title}`)
        lines.push("")
        lines.push(chapter.content.slice(0, 5000))
        if (chapter.content.length > 5000) {
          lines.push("")
          lines.push("[Content truncated...]")
        }
        lines.push("")
      }
    }

    lines.push("## Instructions")
    lines.push("")
    lines.push("Analyze the content for consistency issues and report findings in JSON format:")
    lines.push("")
    lines.push("```json")
    lines.push("{")
    lines.push('  "issues": [')
    lines.push('    {')
    lines.push('      "type": "entity|plot|style|continuity",')
    lines.push('      "severity": "low|medium|high|critical",')
    lines.push('      "description": "Clear description of the issue",')
    lines.push('      "location": "Chapter X or specific reference",')
    lines.push('      "suggestion": "How to fix it",')
    lines.push('      "autoFixable": true')
    lines.push("    }")
    lines.push("  ],")
    lines.push('  "summary": {')
    lines.push('    "byType": {')
    lines.push('      "entity": N,')
    lines.push('      "plot": N,')
    lines.push('      "style": N,')
    lines.push('      "continuity": N')
    lines.push("    },")
    lines.push('    "bySeverity": {')
    lines.push('      "critical": N,')
    lines.push('      "high": N,')
    lines.push('      "medium": N,')
    lines.push('      "low": N')
    lines.push("    }")
    lines.push("  }")
    lines.push("}")
    lines.push("```")
    lines.push("")
    lines.push("Issue types to look for:")
    lines.push("")
    lines.push("**Entity Issues:**")
    lines.push("- Name spelling variations")
    lines.push("- Contradictory character traits")
    lines.push("- Relationship inconsistencies")
    lines.push("- Dead characters reappearing")
    lines.push("- Age/timeline inconsistencies")
    lines.push("")
    lines.push("**Plot Issues:**")
    lines.push("- Contradictions in events")
    lines.push("- Timeline impossibilities")
    lines.push("- Unresolved plot holes")
    lines.push("- Forgotten subplots")
    lines.push("- Cause-effect violations")
    lines.push("")
    lines.push("**Style Issues:**")
    lines.push("- Sudden tone shifts")
    lines.push("- Vocabulary inconsistencies")
    lines.push("- Perspective changes")
    lines.push("- Formatting inconsistencies")
    lines.push("")
    lines.push("**Continuity Issues:**")
    lines.push("- Unexplained scene transitions")
    lines.push("- Contradictory references to past events")
    lines.push("- Location inconsistencies")
    lines.push("- Broken callbacks or foreshadowing")

    return lines.join("\n")
  }

  /**
   * Parse AI response and create consistency report
   */
  export async function saveReport(
    documentID: string,
    aiResponse: string,
  ): Promise<DocumentSchema.ConsistencyReport> {
    try {
      const jsonMatch =
        aiResponse.match(/```json\s*([\s\S]*?)\s*```/) ||
        aiResponse.match(/\{[\s\S]*\}/)

      if (!jsonMatch) {
        throw new Error("No JSON found in AI response")
      }

      const data = JSON.parse(jsonMatch[1] || jsonMatch[0])

      const issues: DocumentSchema.ConsistencyIssue[] = (data.issues || []).map((issue: any) => ({
        id: `issue_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        type: issue.type || "plot",
        severity: issue.severity || "medium",
        description: issue.description || "",
        location: issue.location,
        suggestion: issue.suggestion,
        autoFixable: issue.autoFixable || false,
      }))

      const summary = data.summary || {
        critical: issues.filter((i) => i.severity === "critical").length,
        high: issues.filter((i) => i.severity === "high").length,
        medium: issues.filter((i) => i.severity === "medium").length,
        low: issues.filter((i) => i.severity === "low").length,
      }

      const report: DocumentSchema.ConsistencyReport = {
        documentID,
        timestamp: Date.now(),
        issues,
        summary: {
          critical: summary.critical || 0,
          high: summary.high || 0,
          medium: summary.medium || 0,
          low: summary.low || 0,
        },
      }

      // Store report
      const { Storage } = await import("../storage/storage")
      await Storage.write(["document_consistency", documentID, `report_${report.timestamp}`], report)

      return report
    } catch (error) {
      throw new Error(`Failed to parse consistency report: ${error}`)
    }
  }

  /**
   * Get recent consistency reports
   */
  export async function listReports(documentID: string): Promise<DocumentSchema.ConsistencyReport[]> {
    const { Storage } = await import("../storage/storage")
    const keys = await Storage.list(["document_consistency", documentID])
    const reports: DocumentSchema.ConsistencyReport[] = []

    for (const key of keys) {
      try {
        const report = await Storage.read<DocumentSchema.ConsistencyReport>(key)
        if (report) reports.push(report)
      } catch {
        // Ignore read errors
      }
    }

    return reports.sort((a, b) => b.timestamp - a.timestamp)
  }

  /**
   * Quick entity consistency check (without AI)
   */
  export async function quickEntityCheck(documentID: string): Promise<
    Array<{
      type: string
      description: string
      severity: "low" | "medium" | "high"
      entities: string[]
    }>
  > {
    const entities = await Entity.list(documentID)
    const chapters = await Document.Chapter.list(documentID)
    const issues: Array<{
      type: string
      description: string
      severity: "low" | "medium" | "high"
      entities: string[]
    }> = []

    // Check for duplicate names
    const nameMap = new Map<string, string[]>()
    for (const entity of entities) {
      const normalizedName = entity.name.toLowerCase().trim()
      if (!nameMap.has(normalizedName)) {
        nameMap.set(normalizedName, [])
      }
      nameMap.get(normalizedName)!.push(entity.id)
    }

    const duplicates = Array.from(nameMap.entries()).filter(([_, ids]) => ids.length > 1)
    if (duplicates.length > 0) {
      issues.push({
        type: "duplicate_names",
        description: "Entities with duplicate or similar names found",
        severity: "medium",
        entities: duplicates.map(([_, ids]) => ids.join(", ")),
      })
    }

    // Check for entities without descriptions
    const noDescription = entities.filter((e) => !e.description || e.description.length < 10)
    if (noDescription.length > 0) {
      issues.push({
        type: "missing_descriptions",
        description: "Entities missing descriptions",
        severity: "low",
        entities: noDescription.map((e) => e.id),
      })
    }

    // Check for orphaned entities (first appearance chapter doesn't exist)
    const chapterIDs = new Set(chapters.map((c) => c.id))
    const orphaned = entities.filter((e) => !chapterIDs.has(e.firstAppearedChapterID))
    if (orphaned.length > 0) {
      issues.push({
        type: "orphaned_entities",
        description: "Entities referencing non-existent chapters",
        severity: "high",
        entities: orphaned.map((e) => e.id),
      })
    }

    return issues
  }

  /**
   * Generate prompt to fix specific consistency issues
   */
  export async function fixIssuesPrompt(input: {
    documentID: string
    chapterID: string
    issues: DocumentSchema.ConsistencyIssue[]
  }): Promise<string> {
    const doc = await Document.get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapter = await Document.Chapter.get(input.documentID, input.chapterID)
    if (!chapter) throw new Error("Chapter not found")

    if (!chapter.content) throw new Error("Chapter has no content")

    const lines: string[] = []

    lines.push("# Consistency Issue Fixes")
    lines.push("")
    lines.push("## Chapter")
    lines.push("")
    lines.push(`**Title:** ${chapter.title}`)
    lines.push("")
    lines.push("## Issues to Fix")
    lines.push("")

    for (const issue of input.issues) {
      lines.push(`### [${issue.severity.toUpperCase()}] ${issue.type}`)
      lines.push("")
      lines.push(`**Description:** ${issue.description}`)
      if (issue.location) lines.push(`**Location:** ${issue.location}`)
      if (issue.suggestion) lines.push(`**Suggested Fix:** ${issue.suggestion}`)
      lines.push("")
    }

    lines.push("## Chapter Content")
    lines.push("")
    lines.push(chapter.content)
    lines.push("")
    lines.push("## Instructions")
    lines.push("")
    lines.push("Please revise the chapter content to fix the identified issues.")
    lines.push("")
    lines.push("Requirements:")
    lines.push("- Make minimal changes necessary to fix the issues")
    lines.push("- Preserve the overall writing style and voice")
    lines.push("- Ensure changes flow naturally with surrounding content")
    lines.push("- Output the full revised chapter in Markdown")

    return lines.join("\n")
  }

  /**
   * Check style consistency across chapters
   */
  export async function checkStyleConsistency(documentID: string): Promise<{
    overallConsistency: number // 0-1 score
    issues: Array<{
      chapterID: string
      chapterTitle: string
      issue: string
    }>
  }> {
    const chapters = await Document.Chapter.list(documentID)
    const completed = chapters.filter((c) => c.status === "completed")

    if (completed.length < 2) {
      return {
        overallConsistency: 1,
        issues: [],
      }
    }

    const issues: Array<{
      chapterID: string
      chapterTitle: string
      issue: string
    }> = []

    // Simple heuristic checks
    const avgParagraphLengths: number[] = []

    for (const chapter of completed) {
      if (!chapter.content) continue

      const paragraphs = chapter.content
        .split("\n\n")
        .filter((p) => p.trim().length > 0 && !p.startsWith("#"))

      if (paragraphs.length === 0) continue

      const avgLength = paragraphs.reduce((sum, p) => sum + p.length, 0) / paragraphs.length
      avgParagraphLengths.push(avgLength)
    }

    // Check for significant variance in paragraph length
    if (avgParagraphLengths.length > 1) {
      const mean = avgParagraphLengths.reduce((a, b) => a + b, 0) / avgParagraphLengths.length
      const variance = avgParagraphLengths.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) / avgParagraphLengths.length
      const stdDev = Math.sqrt(variance)
      const cv = stdDev / mean // Coefficient of variation

      if (cv > 0.5) {
        // High variance
        issues.push({
          chapterID: "multiple",
          chapterTitle: "Multiple chapters",
          issue: "High variance in paragraph length across chapters - may indicate inconsistent writing style",
        })
      }
    }

    // Check for missing summaries
    for (const chapter of completed) {
      if (!chapter.summary) {
        issues.push({
          chapterID: chapter.id,
          chapterTitle: chapter.title,
          issue: "Missing chapter summary - affects context passing",
        })
      }
    }

    // Calculate consistency score
    const consistencyScore = 1 - Math.min(1, issues.length * 0.1)

    return {
      overallConsistency: consistencyScore,
      issues,
    }
  }

  /**
   * Auto-fixable issues detection
   */
  export async function findAutoFixableIssues(documentID: string): Promise<
    Array<{
      issueID: string
      type: string
      description: string
      fix: {
        type: string
        params: Record<string, unknown>
      }
    }>
  > {
    const issues = await quickEntityCheck(documentID)
    const autoFixable: Array<{
      issueID: string
      type: string
      description: string
      fix: {
        type: string
        params: Record<string, unknown>
      }
    }> = []

    for (const issue of issues) {
      if (issue.type === "duplicate_names" && issue.severity === "medium") {
        // Can be auto-fixed with user confirmation
        autoFixable.push({
          issueID: `fix_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
          type: "merge_duplicate_entities",
          description: issue.description,
          fix: {
            type: "entity_merge",
            params: {
              entityIDs: issue.entities,
            },
          },
        })
      }
    }

    return autoFixable
  }

  // ============================================================================
  // Extended Consistency Checks (BookExpander)
  // ============================================================================

  /**
   * Check argument coherence for non-fiction content.
   */
  export async function checkArgumentCoherence(
    documentID: string,
  ): Promise<{
    overallScore: number
    issues: Array<{
      type: string
      description: string
      severity: "low" | "medium" | "high"
      argumentID?: string
    }>
  }> {
    const { ArgumentChain } = await import("./knowledge/argument")
    const chains = await ArgumentChain.list(documentID)

    const issues: Array<{
      type: string
      description: string
      severity: "low" | "medium" | "high"
      argumentID?: string
    }> = []

    // Check for circular reasoning
    const cycles = await ArgumentChain.detectCircularReasoning(documentID)
    for (const cycle of cycles) {
      issues.push({
        type: "circular_reasoning",
        description: `Circular argument detected: ${cycle.description}`,
        severity: "high",
      })
    }

    // Check each argument chain
    for (const chain of chains) {
      const validation = await ArgumentChain.validate(documentID, chain.id)
      for (const issue of validation.issues) {
        issues.push({
          type: issue.type,
          description: issue.description,
          severity: issue.severity === "high" ? "high" : issue.severity === "medium" ? "medium" : "low",
          argumentID: chain.id,
        })
      }
    }

    const score = chains.length > 0
      ? 1 - (issues.filter((i) => i.severity === "high").length * 0.3) - (issues.filter((i) => i.severity === "medium").length * 0.1)
      : 1

    return { overallScore: Math.max(0, score), issues }
  }

  /**
   * Check thematic alignment with the framework.
   */
  export async function checkThematicAlignment(
    content: string,
    framework: { thesis: string; corePrinciples: string[]; mainThemes: string[] },
  ): Promise<{
    score: number
    issues: Array<{
      type: string
      description: string
      severity: "low" | "medium" | "high"
    }>
  }> {
    const issues: Array<{
      type: string
      description: string
      severity: "low" | "medium" | "high"
    }> = []

    const contentLower = content.toLowerCase()
    const thesisLower = framework.thesis.toLowerCase()

    // Check if thesis is supported
    const supportsThesis = framework.mainThemes.some((theme) =>
      contentLower.includes(theme.toLowerCase()),
    )

    if (!supportsThesis) {
      issues.push({
        type: "thematic_misalignment",
        description: "Content does not clearly support the central thesis",
        severity: "medium",
      })
    }

    // Check for contradictions with core principles
    for (const principle of framework.corePrinciples) {
      const principleLower = principle.toLowerCase()
      const negationWords = ["not", "never", "cannot", "impossible", "contrary"]

      for (const negation of negationWords) {
        if (contentLower.includes(`${negation} ${principleLower}`)) {
          issues.push({
            type: "principle_contradiction",
            description: `Content may contradict principle: "${principle}"`,
            severity: "high",
          })
        }
      }
    }

    const score = Math.max(0, 1 - issues.length * 0.15)
    return { score, issues }
  }

  /**
   * Detect circular reasoning patterns in arguments.
   */
  export async function detectCircularReasoning(
    documentID: string,
  ): Promise<string[]> {
    const { ArgumentChain } = await import("./knowledge/argument")
    const cycles = await ArgumentChain.detectCircularReasoning(documentID)

    return cycles.map((c) => c.description)
  }

  /**
   * Check worldview consistency for fiction content.
   */
  export async function checkWorldviewConsistency(
    documentID: string,
    worldFramework: {
      rules: string[]
      magicSystem?: string
      technology?: string
    },
  ): Promise<DocumentSchema.ConsistencyIssue[]> {
    const issues: DocumentSchema.ConsistencyIssue[] = []
    const doc = await Document.get(documentID)

    if (!doc) return issues

    const chapters = await Document.Chapter.list(documentID)
    const content = chapters.map((ch) => ch.content).join("\n\n").toLowerCase()

    // Check for rule violations
    for (const rule of worldFramework.rules) {
      const ruleLower = rule.toLowerCase()
      const violationPatterns = [
        `impossible ${ruleLower}`,
        `cannot ${ruleLower}`,
        `can't ${ruleLower}`,
        `never ${ruleLower}`,
        `against ${ruleLower}`,
      ]

      for (const pattern of violationPatterns) {
        if (content.includes(pattern)) {
          issues.push({
            id: `worldview_${Date.now()}`,
            type: "plot",
            severity: "medium",
            description: `Potential worldview violation: ${pattern}`,
            suggestion: `Verify if "${pattern}" contradicts established world rules`,
            autoFixable: false,
          })
        }
      }
    }

    // Check magic system consistency
    if (worldFramework.magicSystem) {
      const magicKeywords = extractMagicKeywords(worldFramework.magicSystem)

      for (const keyword of magicKeywords) {
        // Check for inconsistent magic use (e.g., magic works when it shouldn't)
        const inconsistentPattern = new RegExp(
          `${keyword}\\s+(?:stopped|failed|didn't work|became powerless)`,
          "i",
        )

        if (inconsistentPattern.test(content)) {
          issues.push({
            id: `magic_${Date.now()}`,
            type: "plot",
            severity: "medium",
            description: `Inconsistent magic system behavior for: ${keyword}`,
            suggestion: "Review magic system rules and ensure consistent application",
            autoFixable: false,
          })
        }
      }
    }

    return issues
  }

  /**
   * Extract magic-related keywords from a magic system description.
   */
  function extractMagicKeywords(magicSystem: string): string[] {
    const keywords: string[] = []

    // Extract quoted terms
    const quotedMatches = magicSystem.matchAll(/"([^"]{2,30})"/g)
    for (const match of quotedMatches) {
      keywords.push(match[1])
    }

    // Extract key terms after common magic words
    const patternMatches = magicSystem.matchAll(
      /(?:magic|spell|power|ability|can|must)(?:\s+([a-z]+)){1,3}/gi,
    )
    for (const match of patternMatches) {
      if (match[1]) keywords.push(match[1].toLowerCase())
    }

    return keywords
  }

  /**
   * Comprehensive consistency check combining all checks.
   */
  export async function comprehensiveCheck(documentID: string): Promise<{
    overallScore: number
    report: {
      entity: DocumentSchema.ConsistencyReport | null
      argument: { overallScore: number; issues: unknown[] } | null
      thematic: { score: number; issues: unknown[] } | null
      worldview: DocumentSchema.ConsistencyReport | null
    }
  }> {
    const doc = await Document.get(documentID)
    if (!doc) {
      throw new Error("Document not found")
    }

    let overallScore = 1
    const report: {
      entity: DocumentSchema.ConsistencyReport | null
      argument: { overallScore: number; issues: unknown[] } | null
      thematic: { score: number; issues: unknown[] } | null
      worldview: DocumentSchema.ConsistencyReport | null
    } = {
      entity: null,
      argument: null,
      thematic: null,
      worldview: null,
    }

    // Run entity consistency check
    try {
      const entityIssues = await quickEntityCheck(documentID)
      report.entity = {
        documentID,
        timestamp: Date.now(),
        issues: entityIssues.map((issue) => ({
          id: `entity_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
          type: "entity",
          severity: issue.severity,
          description: issue.description,
          suggestion: issue.description,
          autoFixable: issue.severity === "low",
        })),
        summary: {
          critical: 0,
          high: entityIssues.filter((i) => i.severity === "high").length,
          medium: entityIssues.filter((i) => i.severity === "medium").length,
          low: entityIssues.filter((i) => i.severity === "low").length,
        },
      }
      overallScore *= 0.95
    } catch {
      // Skip entity check if it fails
    }

    // Run argument coherence check (for non-fiction)
    try {
      const { ArgumentChain } = await import("./knowledge/argument")
      const chains = await ArgumentChain.list(documentID)
      if (chains.length > 0) {
        const argCheck = await checkArgumentCoherence(documentID)
        report.argument = argCheck
        overallScore *= argCheck.overallScore
      }
    } catch {
      // Skip argument check if it fails
    }

    // Run thematic alignment check
    try {
      if (doc.globalSummary?.mainThemes) {
        const chapters = await Document.Chapter.list(documentID)
        const allContent = chapters.filter((ch) => ch.status === "completed").map((ch) => ch.content).join("\n\n")
        const themeCheck = await checkThematicAlignment(
          allContent,
          {
            thesis: doc.globalSummary.overallPlot,
            corePrinciples: doc.globalSummary.mainThemes,
            mainThemes: doc.globalSummary.mainThemes,
          },
        )
        report.thematic = themeCheck
        overallScore *= themeCheck.score
      }
    } catch {
      // Skip thematic check if it fails
    }

    // Run worldview check (for fiction)
    try {
      const { Knowledge } = await import("./knowledge")
      const worlds = await Knowledge.StoryElements.listWorldFrameworks(documentID)
      if (worlds.length > 0) {
        const worldviewIssues = await checkWorldviewConsistency(documentID, worlds[0])
        report.worldview = {
          documentID,
          timestamp: Date.now(),
          issues: worldviewIssues,
          summary: {
            critical: worldviewIssues.filter((i) => i.severity === "critical").length,
            high: worldviewIssues.filter((i) => i.severity === "high").length,
            medium: worldviewIssues.filter((i) => i.severity === "medium").length,
            low: worldviewIssues.filter((i) => i.severity === "low").length,
          },
        }
        overallScore *= 0.95
      }
    } catch {
      // Skip worldview check if it fails
    }

    return { overallScore, report }
  }
}
