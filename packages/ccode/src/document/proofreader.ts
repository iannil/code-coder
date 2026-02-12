import { Document } from "./index"
import { DocumentSchema } from "./schema"

export namespace Proofreader {
  /**
   * Generate AI prompt for proofreading
   */
  export async function checkPrompt(input: {
    documentID: string
    chapterID?: string
    depth?: "quick" | "standard" | "deep"
    checkTypes?: Array<"grammar" | "spelling" | "punctuation" | "terminology" | "style" | "flow" | "readability" | "structure">
  }): Promise<string> {
    const doc = await Document.get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Document.Chapter.list(input.documentID)
    const depth = input.depth || "standard"
    const checkTypes = input.checkTypes || ["grammar", "spelling", "punctuation", "terminology", "style", "flow", "readability", "structure"]

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

    lines.push("# Document Proofreading")
    lines.push("")
    lines.push("## Document Information")
    lines.push("")
    lines.push(`**Title:** ${doc.title}`)
    lines.push(`**Chapters to Check:** ${chaptersToCheck.length}`)
    lines.push(`**Depth:** ${depth}`)
    lines.push("")

    lines.push("## Check Types")
    lines.push("")

    const typeLabels: Record<string, string> = {
      grammar: "Grammar - Syntax, verb agreement, sentence structure",
      spelling: "Spelling - Typos, homophones, character errors",
      punctuation: "Punctuation - Commas, quotes, dashes, proper marks",
      terminology: "Terminology - Consistent term usage, proper nouns",
      style: "Style - Tone consistency, voice, formatting",
      flow: "Flow - Transitions, rhythm, paragraph connections",
      readability: "Readability - Sentence length, complexity, clarity",
      structure: "Structure - Paragraph organization, section flow",
    }

    for (const type of checkTypes) {
      if (typeLabels[type]) {
        lines.push(`- ✓ **${type.charAt(0).toUpperCase() + type.slice(1)}**: ${typeLabels[type].split(" - ")[1]}`)
      }
    }
    lines.push("")

    // Style guide reference
    if (doc.styleGuide) {
      lines.push("## Style Guide")
      lines.push("")
      if (doc.styleGuide.tone) lines.push(`**Target Tone:** ${doc.styleGuide.tone}`)
      if (doc.styleGuide.audience) lines.push(`**Target Audience:** ${doc.styleGuide.audience}`)
      if (doc.styleGuide.requirements?.length) {
        lines.push("**Requirements:**")
        for (const req of doc.styleGuide.requirements) {
          lines.push(`  - ${req}`)
        }
      }
      lines.push("")
    }

    // Chapter summaries for context
    if (depth === "deep" && chaptersToCheck.length > 1) {
      lines.push("## Chapter Summaries (Context)")
      lines.push("")
      for (const chapter of chaptersToCheck) {
        lines.push(`### ${chapter.title}`)
        if (chapter.summary) {
          lines.push(chapter.summary)
        } else {
          lines.push(chapter.content.slice(0, 300) + "...")
        }
        lines.push("")
      }
    }

    // Full content to check
    lines.push("---")
    lines.push("")
    lines.push("## Content to Review")
    lines.push("")

    const maxContentLength = depth === "quick" ? 2000 : depth === "standard" ? 5000 : 10000

    for (const chapter of chaptersToCheck) {
      lines.push(`### ${chapter.title}`)
      lines.push(`**Chapter ID:** ${chapter.id}`)
      lines.push(`**Word Count:** ${chapter.wordCount}`)
      lines.push("")

      const content = chapter.content
      if (content.length > maxContentLength) {
        lines.push(content.slice(0, maxContentLength))
        lines.push("")
        lines.push(`[Content truncated for ${depth} mode...]`)
      } else {
        lines.push(content)
      }
      lines.push("")
    }

    lines.push("## Instructions")
    lines.push("")
    lines.push("Analyze the content above using the PROOF framework and report findings in JSON format:")
    lines.push("")
    lines.push("```json")
    lines.push("{")
    lines.push('  "issues": [')
    lines.push('    {')
    lines.push('      "type": "grammar|spelling|punctuation|terminology|style|flow|readability|structure",')
    lines.push('      "severity": "low|medium|high|critical",')
    lines.push('      "description": "Clear description of the issue",')
    lines.push('      "location": {')
    lines.push('        "chapterID": "chapter-id",')
    lines.push('        "chapterTitle": "Chapter Title",')
    lines.push('        "lineReference": "Paragraph X, Line Y (optional)",')
    lines.push('        "excerpt": "Relevant text excerpt (optional)"')
    lines.push('      },')
    lines.push('      "suggestion": "How to fix it",')
    lines.push('      "autoFixable": true')
    lines.push("    }")
    lines.push("  ],")
    lines.push('  "summary": {')
    lines.push('    "byType": {')
    lines.push('      "grammar": 0,')
    lines.push('      "spelling": 0,')
    lines.push('      "punctuation": 0,')
    lines.push('      "terminology": 0,')
    lines.push('      "style": 0,')
    lines.push('      "flow": 0,')
    lines.push('      "readability": 0,')
    lines.push('      "structure": 0')
    lines.push("    },")
    lines.push('    "bySeverity": {')
    lines.push('      "critical": 0,')
    lines.push('      "high": 0,')
    lines.push('      "medium": 0,')
    lines.push('      "low": 0')
    lines.push("    },")
    lines.push('    "autoFixable": 0')
    lines.push("  },")
    lines.push('  "readabilityScore": 75,')
    lines.push('  "readabilityMetrics": {')
    lines.push('    "avgSentenceLength": 15.5,')
    lines.push('    "avgWordLength": 4.2,')
    lines.push('    "complexWords": 42,')
    lines.push('    "totalSentences": 200,')
    lines.push('    "totalWords": 3100')
    lines.push("  }")
    lines.push("}")
    lines.push("```")
    lines.push("")

    if (depth === "quick") {
      lines.push("**Quick Mode:** Focus only on critical and high severity issues.")
      lines.push("")
    } else if (depth === "deep") {
      lines.push("**Deep Mode:** Provide detailed analysis including all severity levels and comprehensive readability metrics.")
      lines.push("")
    }

    return lines.join("\n")
  }

  /**
   * Parse AI response and create proofreader report
   */
  export async function saveReport(
    documentID: string,
    aiResponse: string,
    scope: "chapter" | "document" | "selection" = "document",
    chapterID?: string,
  ): Promise<DocumentSchema.ProofreaderReport> {
    try {
      const jsonMatch =
        aiResponse.match(/```json\s*([\s\S]*?)\s*```/) ||
        aiResponse.match(/\{[\s\S]*\}/)

      if (!jsonMatch) {
        throw new Error("No JSON found in AI response")
      }

      const data = JSON.parse(jsonMatch[1] || jsonMatch[0])

      const issues: DocumentSchema.ProofreaderIssue[] = (data.issues || []).map((issue: any) => ({
        id: `proof_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        type: issue.type || "grammar",
        severity: issue.severity || "medium",
        description: issue.description || "",
        location: issue.location,
        suggestion: issue.suggestion,
        autoFixable: issue.autoFixable || false,
        fixedContent: issue.fixedContent,
      }))

      const summary = data.summary || {
        byType: {},
        bySeverity: {},
        autoFixable: 0,
      }

      // Ensure all type counts exist
      const allTypes: DocumentSchema.ProofreaderIssueType[] = ["grammar", "spelling", "punctuation", "terminology", "style", "flow", "readability", "structure"]
      for (const type of allTypes) {
        if (summary.byType[type] === undefined) {
          summary.byType[type] = 0
        }
      }

      // Ensure all severity counts exist
      const allSeverities: Array<"low" | "medium" | "high" | "critical"> = ["low", "medium", "high", "critical"]
      for (const severity of allSeverities) {
        if (summary.bySeverity[severity] === undefined) {
          summary.bySeverity[severity] = 0
        }
      }

      const report: DocumentSchema.ProofreaderReport = {
        id: `proofreport_${Date.now()}`,
        documentID,
        timestamp: Date.now(),
        scope,
        chapterID,
        issues,
        summary: {
          byType: summary.byType,
          bySeverity: summary.bySeverity,
          autoFixable: summary.autoFixable || issues.filter((i) => i.autoFixable).length,
        },
        readabilityScore: data.readabilityScore,
        readabilityMetrics: data.readabilityMetrics,
      }

      // Store report
      const { Storage } = await import("../storage/storage")
      await Storage.write(["document_proofreader", documentID, report.id], report)

      return report
    } catch (error) {
      throw new Error(`Failed to parse proofreader report: ${error}`)
    }
  }

  /**
   * Get recent proofreader reports
   */
  export async function listReports(documentID: string): Promise<DocumentSchema.ProofreaderReport[]> {
    const { Storage } = await import("../storage/storage")
    const keys = await Storage.list(["document_proofreader", documentID])
    const reports: DocumentSchema.ProofreaderReport[] = []

    for (const key of keys) {
      try {
        const report = await Storage.read<DocumentSchema.ProofreaderReport>(key)
        if (report) reports.push(report)
      } catch {
        // Ignore read errors
      }
    }

    return reports.sort((a, b) => b.timestamp - a.timestamp)
  }

  /**
   * Get a specific report
   */
  export async function getReport(documentID: string, reportID: string): Promise<DocumentSchema.ProofreaderReport | undefined> {
    const { Storage } = await import("../storage/storage")
    try {
      return await Storage.read<DocumentSchema.ProofreaderReport>(["document_proofreader", documentID, reportID])
    } catch {
      return undefined
    }
  }

  /**
   * Format report as Markdown
   */
  export function formatReportAsMarkdown(report: DocumentSchema.ProofreaderReport): string {
    const lines: string[] = []

    lines.push("# Proofreading Report")
    lines.push("")
    lines.push(`**Report ID:** ${report.id}`)
    lines.push(`**Date:** ${new Date(report.timestamp).toLocaleString()}`)
    lines.push(`**Scope:** ${report.scope}`)
    if (report.chapterID) lines.push(`**Chapter ID:** ${report.chapterID}`)
    lines.push("")

    // Summary section
    lines.push("## Summary")
    lines.push("")

    // By severity
    lines.push("### Issues by Severity")
    lines.push("")
    for (const [severity, count] of Object.entries(report.summary.bySeverity)) {
      const numCount = count as number
      if (numCount > 0) {
        const icon = { critical: "🔴", high: "🟠", medium: "🟡", low: "🟢" }[severity] || "⚪"
        lines.push(`${icon} **${severity.charAt(0).toUpperCase() + severity.slice(1)}:** ${numCount}`)
      }
    }
    lines.push("")

    // By type
    lines.push("### Issues by Type")
    lines.push("")
    const typeIcons: Record<string, string> = {
      grammar: "📝",
      spelling: "🔤",
      punctuation: "❞",
      terminology: "🏷️",
      style: "🎨",
      flow: "🌊",
      readability: "📖",
      structure: "🏗️",
    }
    for (const [type, count] of Object.entries(report.summary.byType)) {
      const numCount = count as number
      if (numCount > 0) {
        const icon = typeIcons[type] || "•"
        lines.push(`${icon} **${type.charAt(0).toUpperCase() + type.slice(1)}:** ${count}`)
      }
    }
    lines.push("")

    if (report.summary.autoFixable > 0) {
      lines.push(`**Auto-fixable Issues:** ${report.summary.autoFixable}`)
      lines.push("")
    }

    // Readability score
    if (report.readabilityScore !== undefined) {
      lines.push("### Readability Assessment")
      lines.push("")
      const score = report.readabilityScore
      let level = "Standard"
      if (score >= 90) level = "Very Easy"
      else if (score >= 80) level = "Easy"
      else if (score >= 70) level = "Fairly Easy"
      else if (score >= 60) level = "Standard"
      else if (score >= 50) level = "Fairly Difficult"
      else if (score >= 30) level = "Difficult"
      else level = "Very Confusing"

      lines.push(`**Score:** ${score}/100 (${level})`)
      lines.push("")

      if (report.readabilityMetrics) {
        lines.push("### Metrics")
        lines.push("")
        lines.push(`- Average Sentence Length: ${report.readabilityMetrics.avgSentenceLength.toFixed(1)} words`)
        lines.push(`- Average Word Length: ${report.readabilityMetrics.avgWordLength.toFixed(1)} characters`)
        lines.push(`- Complex Words: ${report.readabilityMetrics.complexWords}`)
        lines.push(`- Total Sentences: ${report.readabilityMetrics.totalSentences}`)
        lines.push(`- Total Words: ${report.readabilityMetrics.totalWords}`)
        lines.push("")
      }
    }

    // Issues detail
    if (report.issues.length > 0) {
      lines.push("---")
      lines.push("")
      lines.push("## Issues Found")
      lines.push("")

      // Group by severity
      const bySeverity: Record<string, DocumentSchema.ProofreaderIssue[]> = {
        critical: [],
        high: [],
        medium: [],
        low: [],
      }
      for (const issue of report.issues) {
        bySeverity[issue.severity].push(issue)
      }

      for (const severity of ["critical", "high", "medium", "low"] as const) {
        const issues = bySeverity[severity]
        if (issues.length === 0) continue

        const icon = { critical: "🔴", high: "🟠", medium: "🟡", low: "🟢" }[severity]
        lines.push(`### ${icon} ${severity.charAt(0).toUpperCase() + severity.slice(1)} (${issues.length})`)
        lines.push("")

        for (const issue of issues) {
          lines.push(`#### ${issue.type.charAt(0).toUpperCase() + issue.type.slice(1)}`)
          lines.push("")
          lines.push(issue.description)
          lines.push("")

          if (issue.location) {
            lines.push(`**Location:** ${issue.location.chapterTitle}`)
            if (issue.location.lineReference) {
              lines.push(`(${issue.location.lineReference})`)
            }
            lines.push("")
            if (issue.location.excerpt) {
              lines.push(`> ${issue.location.excerpt}`)
              lines.push("")
            }
          }

          if (issue.suggestion) {
            lines.push(`**Suggestion:** ${issue.suggestion}`)
            lines.push("")
          }

          if (issue.autoFixable) {
            lines.push("*(Auto-fixable)*")
            lines.push("")
          }
        }
      }
    } else {
      lines.push("## No Issues Found")
      lines.push("")
      lines.push("Great job! No issues were detected in the checked content.")
      lines.push("")
    }

    return lines.join("\n")
  }

  /**
   * Generate prompt to fix specific proofreading issues
   */
  export async function generateFixPrompt(input: {
    documentID: string
    chapterID: string
    issues: DocumentSchema.ProofreaderIssue[]
  }): Promise<string> {
    const doc = await Document.get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapter = await Document.Chapter.get(input.documentID, input.chapterID)
    if (!chapter) throw new Error("Chapter not found")

    if (!chapter.content) throw new Error("Chapter has no content")

    const lines: string[] = []

    lines.push("# Proofreading Issue Fixes")
    lines.push("")
    lines.push("## Chapter")
    lines.push("")
    lines.push(`**Title:** ${chapter.title}`)
    lines.push(`**Document:** ${doc.title}`)
    lines.push("")

    lines.push("## Issues to Fix")
    lines.push("")

    // Group issues by type
    const byType: Record<string, DocumentSchema.ProofreaderIssue[]> = {}
    for (const issue of input.issues) {
      if (!byType[issue.type]) byType[issue.type] = []
      byType[issue.type].push(issue)
    }

    for (const [type, issues] of Object.entries(byType)) {
      lines.push(`### ${type.charAt(0).toUpperCase() + type.slice(1)} (${issues.length} issues)`)
      lines.push("")
      for (const issue of issues) {
        lines.push(`- [${issue.severity.toUpperCase()}] ${issue.description}`)
        if (issue.location?.lineReference) {
          lines.push(`  Location: ${issue.location.lineReference}`)
        }
        if (issue.suggestion) {
          lines.push(`  Fix: ${issue.suggestion}`)
        }
        lines.push("")
      }
    }

    lines.push("---")
    lines.push("")
    lines.push("## Original Content")
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
    lines.push("- Output ONLY the revised chapter content in Markdown")
    lines.push("")

    return lines.join("\n")
  }

  /**
   * Quick local grammar check (rule-based, no AI)
   */
  export async function quickGrammarCheck(documentID: string, chapterID: string): Promise<
    Array<{
      type: string
      description: string
      location: string
      suggestion: string
    }>
  > {
    const chapter = await Document.Chapter.get(documentID, chapterID)
    if (!chapter) throw new Error("Chapter not found")

    const issues: Array<{
      type: string
      description: string
      location: string
      suggestion: string
    }> = []

    const content = chapter.content
    const lines = content.split("\n")

    // Simple rule-based checks
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i]
      const trimmed = line.trim()

      // Check for multiple spaces
      if (/\s{3,}/.test(trimmed)) {
        issues.push({
          type: "spacing",
          description: "Excessive whitespace detected",
          location: `Line ${i + 1}`,
          suggestion: "Replace multiple spaces with single space",
        })
      }

      // Check for sentences ending with comma
      if (trimmed.endsWith(",") && !trimmed.startsWith("-") && !trimmed.startsWith("*")) {
        issues.push({
          type: "punctuation",
          description: "Sentence ends with comma",
          location: `Line ${i + 1}`,
          suggestion: "Replace comma with period or other appropriate punctuation",
        })
      }

      // Check for common Chinese punctuation issues
      if (/[，。！？；：][a-zA-Z]/.test(trimmed)) {
        issues.push({
          type: "punctuation",
          description: "Chinese punctuation followed by English letter without space",
          location: `Line ${i + 1}`,
          suggestion: "Add space after Chinese punctuation",
        })
      }

      // Check for mixed quotes
      const straightQuotes = (trimmed.match(/"/g) || []).length
      const curlyQuotes = (trimmed.match(/[""]/g) || []).length
      if (straightQuotes > 0 && curlyQuotes > 0) {
        issues.push({
          type: "punctuation",
          description: "Mixed quote styles detected",
          location: `Line ${i + 1}`,
          suggestion: "Use consistent quote style throughout",
        })
      }
    }

    return issues
  }

  /**
   * Analyze readability metrics
   */
  export async function analyzeReadability(documentID: string, chapterID?: string): Promise<{
    readabilityScore: number
    metrics: DocumentSchema.ProofreaderReadabilityMetrics
    assessment: string
  }> {
    const chapters = chapterID
      ? [await Document.Chapter.get(documentID, chapterID)].filter((c) => c !== undefined) as DocumentSchema.Chapter[]
      : await Document.Chapter.list(documentID)

    if (chapters.length === 0) {
      throw new Error("No chapters to analyze")
    }

    let totalSentences = 0
    let totalWords = 0
    let totalChars = 0
    let complexWords = 0

    for (const chapter of chapters) {
      const content = chapter.content

      // Count sentences (rough approximation for mixed English/Chinese)
      const sentenceEndings = content.match(/[。！？.!?\n]/g) || []
      totalSentences += Math.max(1, sentenceEndings.length)

      // Count words
      const chineseChars = (content.match(/[\u4e00-\u9fa5]/g) || []).length
      const englishWords = (content.match(/[a-zA-Z]+/g) || []).length
      const words = chineseChars + englishWords
      totalWords += words

      // Count characters for avg word length
      totalChars += content.replace(/\s/g, "").length

      // Count complex words (English words with 3+ syllables, approximation)
      const longEnglishWords = (content.match(/[a-zA-Z]{10,}/g) || []).length
      complexWords += longEnglishWords
    }

    const avgSentenceLength = totalSentences > 0 ? totalWords / totalSentences : 0
    const avgWordLength = totalWords > 0 ? totalChars / totalWords : 0

    // Calculate readability score (simplified Flesch-like score)
    // Higher score = easier to read
    let readabilityScore = 100

    // Penalize long sentences
    if (avgSentenceLength > 25) readabilityScore -= 20
    else if (avgSentenceLength > 20) readabilityScore -= 10
    else if (avgSentenceLength > 15) readabilityScore -= 5

    // Penalize complex words
    const complexWordRatio = totalWords > 0 ? (complexWords / totalWords) * 100 : 0
    if (complexWordRatio > 20) readabilityScore -= 20
    else if (complexWordRatio > 15) readabilityScore -= 10
    else if (complexWordRatio > 10) readabilityScore -= 5

    // Penalize long words
    if (avgWordLength > 6) readabilityScore -= 10
    else if (avgWordLength > 5) readabilityScore -= 5

    readabilityScore = Math.max(0, Math.min(100, readabilityScore))

    let assessment = "Standard"
    if (readabilityScore >= 90) assessment = "Very Easy"
    else if (readabilityScore >= 80) assessment = "Easy"
    else if (readabilityScore >= 70) assessment = "Fairly Easy"
    else if (readabilityScore >= 60) assessment = "Standard"
    else if (readabilityScore >= 50) assessment = "Fairly Difficult"
    else if (readabilityScore >= 30) assessment = "Difficult"
    else assessment = "Very Confusing"

    return {
      readabilityScore,
      metrics: {
        avgSentenceLength,
        avgWordLength,
        complexWords,
        totalSentences,
        totalWords,
      },
      assessment,
    }
  }

  /**
   * Check terminology consistency
   */
  export async function checkTerminology(documentID: string): Promise<
    Array<{
      type: string
      description: string
      severity: "low" | "medium" | "high"
      occurrences: Array<{ chapter: string; term: string }>
    }>
  > {
    const doc = await Document.get(documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Document.Chapter.list(documentID)
    const issues: Array<{
      type: string
      description: string
      severity: "low" | "medium" | "high"
      occurrences: Array<{ chapter: string; term: string }>
    }> = []

    // Collect all potential terms (capitalized words, Chinese terms in quotes, etc.)
    const termMap = new Map<string, Array<{ chapter: string; term: string }>>()

    for (const chapter of chapters) {
      if (!chapter.content) continue

      // Find capitalized terms (English)
      const capitalizedTerms = chapter.content.match(/\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\b/g) || []
      for (const term of capitalizedTerms) {
        if (term.length < 3) continue // Skip short words
        const key = term.toLowerCase()
        if (!termMap.has(key)) termMap.set(key, [])
        termMap.get(key)!.push({ chapter: chapter.title, term })
      }

      // Find quoted terms (Chinese)
      const quotedTerms = chapter.content.match(/「([^」]+)」/g) || []
      for (const term of quotedTerms) {
        const key = term.toLowerCase()
        if (!termMap.has(key)) termMap.set(key, [])
        termMap.get(key)!.push({ chapter: chapter.title, term })
      }
    }

    // Check for variations
    for (const [key, occurrences] of termMap.entries()) {
      if (occurrences.length < 2) continue

      const uniqueForms = new Set(occurrences.map((o) => o.term))
      if (uniqueForms.size > 1) {
        issues.push({
          type: "terminology_inconsistency",
          description: `Inconsistent terminology: "${Array.from(uniqueForms).join('", "')}"`,
          severity: "medium",
          occurrences,
        })
      }
    }

    return issues
  }

  /**
   * Batch proofread multiple chapters
   */
  export async function batchCheck(input: {
    documentID: string
    chapterIDs: string[]
    depth?: "quick" | "standard" | "deep"
    parallel?: number
  }): Promise<Array<{ chapterID: string; prompt: string }>> {
    const results: Array<{ chapterID: string; prompt: string }> = []

    for (const chapterID of input.chapterIDs) {
      const prompt = await checkPrompt({
        documentID: input.documentID,
        chapterID,
        depth: input.depth,
      })
      results.push({ chapterID, prompt })
    }

    return results
  }

  /**
   * Get latest report for a document
   */
  export async function getLatestReport(documentID: string): Promise<DocumentSchema.ProofreaderReport | undefined> {
    const reports = await listReports(documentID)
    return reports.length > 0 ? reports[0] : undefined
  }

  /**
   * Delete a report
   */
  export async function deleteReport(documentID: string, reportID: string): Promise<void> {
    const { Storage } = await import("../storage/storage")
    await Storage.remove(["document_proofreader", documentID, reportID])
  }

  /**
   * Clear all reports for a document
   */
  export async function clearReports(documentID: string): Promise<void> {
    const { Storage } = await import("../storage/storage")
    const keys = await Storage.list(["document_proofreader", documentID])
    for (const key of keys) {
      await Storage.remove(key)
    }
  }
}
