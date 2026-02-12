import { Document } from "./index"
import { DocumentSchema } from "./schema"

export namespace Summary {
  /**
   * Generate global summary from all completed chapters
   * This produces a prompt that can be sent to an AI to generate the summary
   */
  export async function generateGlobalSummaryPrompt(documentID: string): Promise<string> {
    const doc = await Document.get(documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Document.Chapter.list(documentID)
    const completedChapters = chapters.filter((c) => c.status === "completed")

    if (completedChapters.length === 0) {
      throw new Error("No completed chapters to summarize")
    }

    const lines: string[] = []

    lines.push("# Global Summary Generation")
    lines.push("")
    lines.push("Generate a comprehensive global summary for this long document.")
    lines.push("")
    lines.push("## Document Information")
    lines.push("")
    lines.push(`**Title:** ${doc.title}`)
    if (doc.description) lines.push(`**Description:** ${doc.description}`)
    lines.push(`**Total Chapters:** ${chapters.length}`)
    lines.push(`**Completed Chapters:** ${completedChapters.length}`)
    lines.push(`**Target Words:** ${doc.targetWords}`)
    lines.push(`**Current Words:** ${doc.currentWords}`)
    lines.push("")
    lines.push("## Complete Outline")
    lines.push("")
    for (let i = 0; i < doc.outline.chapters.length; i++) {
      const ch = doc.outline.chapters[i]
      const isCompleted = completedChapters.find((c) => c.outlineID === ch.id)
      lines.push(`${i + 1}. **${ch.title}**${isCompleted ? " ✓" : " (pending)"}`)
      lines.push(`   ${ch.description}`)
    }
    lines.push("")
    lines.push("## Chapter Summaries")
    lines.push("")

    // Include summaries from all completed chapters
    for (const chapter of completedChapters) {
      lines.push(`### ${chapter.title}`)
      if (chapter.summary) {
        lines.push(chapter.summary)
      }
      lines.push("")
    }

    // If there's an existing global summary, include it for incremental updates
    if (doc.globalSummary) {
      lines.push("---")
      lines.push("")
      lines.push("## Existing Global Summary (for reference)")
      lines.push("")
      lines.push("**Overall Plot:**")
      lines.push(doc.globalSummary.overallPlot)
      lines.push("")
      if (doc.globalSummary.mainThemes.length > 0) {
        lines.push("**Main Themes:**")
        lines.push(doc.globalSummary.mainThemes.join(", "))
        lines.push("")
      }
      if (doc.globalSummary.writingStyle) {
        lines.push("**Writing Style:**")
        lines.push(doc.globalSummary.writingStyle)
        lines.push("")
      }
      if (doc.globalSummary.keyArcs.length > 0) {
        lines.push("**Key Story Arcs:**")
        for (const arc of doc.globalSummary.keyArcs) {
          lines.push(`- ${arc.name} (${arc.status}): ${arc.description}`)
        }
        lines.push("")
      }
      lines.push("")
    }

    lines.push("## Output Format")
    lines.push("")
    lines.push("Generate a comprehensive global summary in the following JSON format:")
    lines.push("")
    lines.push("```json")
    lines.push("{")
    lines.push('  "overallPlot": "A concise summary of the entire document plot (500-1000 characters)",')
    lines.push('  "mainThemes": ["Theme 1", "Theme 2", "Theme 3"],')
    lines.push('  "writingStyle": "Description of the writing style, tone, and narrative voice (200-500 characters)",')
    lines.push('  "keyArcs": [')
    lines.push('    {')
 // Continue with keyArcs
    lines.push('      "name": "Arc Name",')
    lines.push('      "description": "Brief description",')
    lines.push('      "status": "setup|developing|resolved|abandoned"')
    lines.push("    }")
    lines.push("  ]")
    lines.push("}")
    lines.push("```")
    lines.push("")
    lines.push("Guidelines:")
    lines.push("- Keep overallPlot concise but comprehensive")
    lines.push("- Extract 3-7 main themes")
    lines.push("- Describe writing style objectively (tone, perspective, language)")
    lines.push("- Identify 3-10 key story arcs and their current status")

    return lines.join("\n")
  }

  /**
   * Parse and save global summary from AI response
   */
  export async function saveGlobalSummary(
    documentID: string,
    aiResponse: string,
  ): Promise<DocumentSchema.GlobalSummary> {
    try {
      // Extract JSON from response
      const jsonMatch =
        aiResponse.match(/```json\s*([\s\S]*?)\s*```/) ||
        aiResponse.match(/\{[\s\S]*\}/)

      if (!jsonMatch) {
        throw new Error("No JSON found in AI response")
      }

      const summaryData = JSON.parse(jsonMatch[1] || jsonMatch[0])

      // Validate against schema
      const globalSummary: DocumentSchema.GlobalSummary = {
        overallPlot: summaryData.overallPlot || "",
        mainThemes: summaryData.mainThemes || [],
        writingStyle: summaryData.writingStyle || "",
        keyArcs: (summaryData.keyArcs || []).map((arc: any) => ({
          name: arc.name || "Untitled",
          description: arc.description || "",
          status: arc.status || "developing",
        })),
        lastUpdated: Date.now(),
      }

      // Update document with global summary
      const doc = await Document.get(documentID)
      if (doc) {
        await Document.update({
          documentID,
          globalSummary,
        })
      }

      return globalSummary
    } catch (error) {
      throw new Error(`Failed to parse global summary: ${error}`)
    }
  }

  /**
   * Generate a prompt to incrementally update global summary with new chapters
   */
  export async function updateGlobalSummaryPrompt(
    documentID: string,
    newChapterIDs: string[],
  ): Promise<string> {
    const doc = await Document.get(documentID)
    if (!doc) throw new Error("Document not found")

    if (!doc.globalSummary) {
      return generateGlobalSummaryPrompt(documentID)
    }

    const chapters = await Document.Chapter.list(documentID)
    const newChapters = newChapterIDs
      .map((id) => chapters.find((c) => c.id === id))
      .filter((c): c is DocumentSchema.Chapter => c !== undefined)

    if (newChapters.length === 0) {
      throw new Error("No valid chapter IDs provided")
    }

    const lines: string[] = []

    lines.push("# Global Summary Update")
    lines.push("")
    lines.push("Update the existing global summary with information from new chapters.")
    lines.push("")
    lines.push("## Current Global Summary")
    lines.push("")
    lines.push("**Overall Plot:**")
    lines.push(doc.globalSummary.overallPlot)
    lines.push("")
    lines.push("**Main Themes:**")
    lines.push(doc.globalSummary.mainThemes.join(", "))
    lines.push("")
    lines.push("**Writing Style:**")
    lines.push(doc.globalSummary.writingStyle)
    lines.push("")
    lines.push("**Key Story Arcs:**")
    for (const arc of doc.globalSummary.keyArcs) {
      lines.push(`- ${arc.name} (${arc.status}): ${arc.description}`)
    }
    lines.push("")
    lines.push("## New Chapters to Integrate")
    lines.push("")

    for (const chapter of newChapters) {
      lines.push(`### ${chapter.title}`)
      if (chapter.summary) {
        lines.push(chapter.summary)
      } else if (chapter.content) {
        lines.push(chapter.content.slice(0, 2000))
      }
      lines.push("")
    }

    lines.push("## Instructions")
    lines.push("")
    lines.push("Update the global summary considering the new chapters:")
    lines.push("")
    lines.push("1. **overallPlot**: Expand if new developments change the big picture")
    lines.push('2. **mainThemes**: Add new themes, remove any that are no longer relevant')
    lines.push("3. **writingStyle**: Update if style has evolved")
    lines.push('4. **keyArcs**: Update arc statuses, add new arcs, resolve completed ones')
    lines.push("")
    lines.push("Output the updated summary in the same JSON format.")

    return lines.join("\n")
  }

  /**
   * Analyze writing style from completed chapters
   */
  export async function analyzeWritingStylePrompt(documentID: string): Promise<string> {
    const doc = await Document.get(documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Document.Chapter.list(documentID)
    const completedChapters = chapters.filter((c) => c.status === "completed").slice(0, 5) // First 5 chapters

    if (completedChapters.length === 0) {
      throw new Error("No completed chapters to analyze")
    }

    const lines: string[] = []

    lines.push("# Writing Style Analysis")
    lines.push("")
    lines.push("Analyze the writing style of this document based on the sample chapters.")
    lines.push("")
    lines.push("## Document")
    lines.push("")
    lines.push(`**Title:** ${doc.title}`)
    if (doc.styleGuide?.tone) lines.push(`**Target Tone:** ${doc.styleGuide.tone}`)
    if (doc.styleGuide?.audience) lines.push(`**Target Audience:** ${doc.styleGuide.audience}`)
    lines.push("")
    lines.push("## Sample Chapters")
    lines.push("")

    for (const chapter of completedChapters) {
      lines.push(`### ${chapter.title}`)
      lines.push("")
      // Include first 1500 characters of each chapter
      const sample = chapter.content.slice(0, 1500)
      lines.push(sample)
      lines.push("")
    }

    lines.push("## Analysis Instructions")
    lines.push("")
    lines.push("Analyze and describe:")
    lines.push("")
    lines.push("1. **Narrative Voice**: First-person, third-person, omniscient, etc.")
    lines.push("2. **Sentence Structure**: Short & punchy, long & flowing, varied, etc.")
    lines.push("3. **Language Style**: Formal, casual, poetic, technical, etc.")
    lines.push("4. **Dialogue Style**: Natural, stylized, minimal, extensive")
    lines.push("5. **Description Style**: Concise, detailed, metaphor-heavy, etc.")
    lines.push("6. **Pacing**: Fast, slow, varies by scene")
    lines.push("")
    lines.push("Provide a 200-500 character description that captures the essence of this writing style.")

    return lines.join("\n")
  }

  /**
   * Extract key story arcs from the document
   */
  export async function extractKeyArcsPrompt(documentID: string): Promise<string> {
    const doc = await Document.get(documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Document.Chapter.list(documentID)
    const completedChapters = chapters.filter((c) => c.status === "completed")

    if (completedChapters.length === 0) {
      throw new Error("No completed chapters to analyze")
    }

    const lines: string[] = []

    lines.push("# Key Story Arcs Extraction")
    lines.push("")
    lines.push("Identify and track the key story arcs running through this document.")
    lines.push("")
    lines.push("## Document Information")
    lines.push("")
    lines.push(`**Title:** ${doc.title}`)
    lines.push(`**Completed Chapters:** ${completedChapters.length}/${chapters.length}`)
    lines.push("")
    lines.push("## Chapter Summaries")
    lines.push("")

    for (const chapter of completedChapters) {
      lines.push(`### ${chapter.title}`)
      if (chapter.summary) {
        lines.push(chapter.summary)
      }
      lines.push("")
    }

    if (doc.globalSummary?.keyArcs) {
      lines.push("## Currently Tracked Arcs")
      lines.push("")
      for (const arc of doc.globalSummary.keyArcs) {
        lines.push(`- ${arc.name} (${arc.status}): ${arc.description}`)
      }
      lines.push("")
    }

    lines.push("## Instructions")
    lines.push("")
    lines.push("Identify the key story arcs and provide:")
    lines.push("")
    lines.push("```json")
    lines.push("[")
    lines.push('  {')
    lines.push('    "name": "Arc Name",')
    lines.push('    "description": "What this arc is about",')
    lines.push('    "status": "setup|developing|resolved|abandoned",')
    lines.push('    "firstMentionedChapter": "chapter_id_or_title",')
    lines.push('    "keyCharacters": ["character1", "character2"]')
    lines.push("  }")
    lines.push("]")
    lines.push("```")
    lines.push("")
    lines.push("Types of arcs to look for:")
    lines.push("- **Character arcs**: Growth, redemption, fall from grace")
    lines.push("- **Relationship arcs**: Romance, friendship, rivalry")
    lines.push("- **Plot arcs**: Main quest, subplot, mystery")
    lines.push("- **Thematic arcs**: Ideas that develop over time")

    return lines.join("\n")
  }

  /**
   * Get a summary of document progress for status reporting
   */
  export async function getProgressSummary(documentID: string): Promise<{
    documentTitle: string
    totalChapters: number
    completedChapters: number
    completionPercentage: number
    totalWords: number
    targetWords: number
    hasGlobalSummary: boolean
    themes: string[]
    arcCount: number
    estimatedRemainingChapters: number
  }> {
    const doc = await Document.get(documentID)
    if (!doc) throw new Error("Document not found")

    const chapters = await Document.Chapter.list(documentID)
    const completed = chapters.filter((c) => c.status === "completed")

    return {
      documentTitle: doc.title,
      totalChapters: chapters.length,
      completedChapters: completed.length,
      completionPercentage: chapters.length > 0
        ? Math.round((completed.length / chapters.length) * 100)
        : 0,
      totalWords: doc.currentWords,
      targetWords: doc.targetWords,
      hasGlobalSummary: !!doc.globalSummary,
      themes: doc.globalSummary?.mainThemes ?? [],
      arcCount: doc.globalSummary?.keyArcs.length ?? 0,
      estimatedRemainingChapters: chapters.length - completed.length,
    }
  }
}
