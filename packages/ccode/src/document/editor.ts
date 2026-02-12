import { Document } from "./index"
import { DocumentSchema } from "./schema"
import { Volume } from "./volume"

export namespace Editor {
  /**
   * Search and replace text across chapters
   */
  export async function searchAndReplace(input: {
    documentID: string
    search: string | RegExp
    replace: string
    scope?: "global" | "chapter" | "volume"
    chapterID?: string
    volumeID?: string
    caseSensitive?: boolean
    previewOnly?: boolean
  }): Promise<{
    chaptersModified: Array<{
      chapterID: string
      chapterTitle: string
      replacementCount: number
    }>
    totalReplacements: number
  }> {
    const chapters = await getChaptersByScope(
      input.documentID,
      input.scope || "global",
      input.chapterID,
      input.volumeID,
    )

    const results: Array<{
      chapterID: string
      chapterTitle: string
      replacementCount: number
    }> = []
    let totalReplacements = 0

    const searchPattern =
      typeof input.search === "string"
        ? new RegExp(
            input.search.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
            input.caseSensitive ? "g" : "gi",
          )
        : input.search

    for (const chapter of chapters) {
      const content = chapter.content
      const matches = content.match(searchPattern)
      const replacementCount = matches ? matches.length : 0

      if (replacementCount > 0 && !input.previewOnly) {
        const newContent = content.replace(searchPattern, input.replace)
        await Document.Chapter.update({
          documentID: input.documentID,
          chapterID: chapter.id,
          content: newContent,
        })
      }

      if (replacementCount > 0) {
        results.push({
          chapterID: chapter.id,
          chapterTitle: chapter.title,
          replacementCount,
        })
        totalReplacements += replacementCount
      }
    }

    return {
      chaptersModified: results,
      totalReplacements,
    }
  }

  /**
   * Generate AI prompt for polishing chapter content
   */
  export async function polishPrompt(input: {
    documentID: string
    chapterID: string
    aspect: "fluency" | "clarity" | "style" | "tone" | "all"
    preserveVoice?: boolean
  }): Promise<string> {
    const doc = await Document.get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapter = await Document.Chapter.get(input.documentID, input.chapterID)
    if (!chapter) throw new Error("Chapter not found")

    if (!chapter.content) throw new Error("Chapter has no content")

    const lines: string[] = []

    lines.push("# Chapter Polishing Task")
    lines.push("")
    lines.push("## Chapter Information")
    lines.push("")
    lines.push(`**Document:** ${doc.title}`)
    lines.push(`**Chapter:** ${chapter.title}`)
    lines.push(`**Current Word Count:** ${chapter.wordCount}`)
    lines.push("")

    if (doc.styleGuide) {
      lines.push("## Style Guide")
      lines.push("")
      if (doc.styleGuide.tone) lines.push(`**Target Tone:** ${doc.styleGuide.tone}`)
      if (doc.styleGuide.audience) lines.push(`**Target Audience:** ${doc.styleGuide.audience}`)
      if (doc.styleGuide.requirements) {
        lines.push("**Requirements:**")
        for (const req of doc.styleGuide.requirements) {
          lines.push(`  - ${req}`)
        }
      }
      lines.push("")
    }

    lines.push("## Polishing Aspect")
    lines.push("")

    const aspectInstructions: Record<string, string[]> = {
      fluency: [
        "Improve sentence flow and rhythm",
        "Fix awkward phrasing",
        "Vary sentence structure for better readability",
        "Eliminate run-on sentences",
      ],
      clarity: [
        "Make complex ideas clearer",
        "Remove ambiguity",
        "Improve word choice",
        "Clarify pronoun references",
      ],
      style: [
        "Ensure consistent writing style throughout",
        "Match the target style guide",
        "Remove clichés and overused expressions",
        "Improve descriptive language",
      ],
      tone: [
        "Adjust tone to match target audience",
        "Ensure emotional resonance is appropriate",
        "Fix tonal inconsistencies",
      ],
      all: [
        "Improve overall quality across all aspects",
        "Enhance fluency, clarity, style, and tone",
        "Fix any issues while preserving the original meaning",
      ],
    }

    lines.push(`**Focus Area:** ${input.aspect}`)
    lines.push("")
    lines.push("Specific improvements to make:")
    for (const instruction of aspectInstructions[input.aspect] || aspectInstructions.all) {
      lines.push(`  - ${instruction}`)
    }
    lines.push("")

    if (input.preserveVoice) {
      lines.push("**Important:** Preserve the author's unique voice and personality. Only make changes that improve quality without altering the distinctive style.")
      lines.push("")
    }

    lines.push("## Chapter Content")
    lines.push("")
    lines.push(chapter.content)
    lines.push("")
    lines.push("## Instructions")
    lines.push("")
    lines.push("Please polish the chapter content above focusing on the specified aspects.")
    lines.push("")
    lines.push("Output the polished version in Markdown format, followed by a brief summary of changes made.")
    lines.push("")
    lines.push("Format:")
    lines.push("```markdown")
    lines.push("[Polished chapter content]")
    lines.push("```")
    lines.push("")
    lines.push("**Summary of Changes:**")
    lines.push("- [List key improvements made]")

    return lines.join("\n")
  }

  /**
   * Generate AI prompt for expanding chapter
   */
  export async function expandPrompt(input: {
    documentID: string
    chapterID: string
    targetWords: number
    focusAreas?: string[]
  }): Promise<string> {
    const doc = await Document.get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapter = await Document.Chapter.get(input.documentID, input.chapterID)
    if (!chapter) throw new Error("Chapter not found")

    if (!chapter.content) throw new Error("Chapter has no content")

    const wordsToAdd = Math.max(0, input.targetWords - chapter.wordCount)

    const lines: string[] = []

    lines.push("# Chapter Expansion Task")
    lines.push("")
    lines.push("## Chapter Information")
    lines.push("")
    lines.push(`**Document:** ${doc.title}`)
    lines.push(`**Chapter:** ${chapter.title}`)
    lines.push(`**Current Word Count:** ${chapter.wordCount}`)
    lines.push(`**Target Word Count:** ${input.targetWords}`)
    lines.push(`**Words to Add:** ${wordsToAdd}`)
    lines.push("")

    const outline = doc.outline.chapters.find((c) => c.id === chapter.outlineID)
    if (outline) {
      lines.push("## Chapter Outline")
      lines.push("")
      lines.push(`**Description:** ${outline.description}`)
      if (outline.subsections?.length) {
        lines.push(`**Sections:** ${outline.subsections.join(", ")}`)
      }
      lines.push("")
    }

    if (input.focusAreas?.length) {
      lines.push("## Focus Areas for Expansion")
      lines.push("")
      for (const area of input.focusAreas) {
        lines.push(`  - ${area}`)
      }
      lines.push("")
    }

    lines.push("## Current Content")
    lines.push("")
    lines.push(chapter.content)
    lines.push("")
    lines.push("## Instructions")
    lines.push("")
    lines.push(`Expand this chapter by approximately ${wordsToAdd} words while maintaining quality and coherence.`)
    lines.push("")
    lines.push("Expansion strategies:")
    lines.push("- Add more detailed descriptions and sensory details")
    lines.push("- Expand on character thoughts and emotions")
    lines.push("- Add dialogue or internal monologue")
    lines.push("- Develop scenes that are currently summarized")
    lines.push("- Add examples or illustrations for concepts")
    lines.push("- Deepen analysis or exploration of ideas")
    lines.push("")
    lines.push("Output the expanded version in Markdown format.")

    return lines.join("\n")
  }

  /**
   * Generate AI prompt for compressing chapter
   */
  export async function compressPrompt(input: {
    documentID: string
    chapterID: string
    targetWords: number
    preserveKeyElements?: string[]
  }): Promise<string> {
    const doc = await Document.get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapter = await Document.Chapter.get(input.documentID, input.chapterID)
    if (!chapter) throw new Error("Chapter not found")

    if (!chapter.content) throw new Error("Chapter has no content")

    const wordsToRemove = Math.max(0, chapter.wordCount - input.targetWords)

    const lines: string[] = []

    lines.push("# Chapter Compression Task")
    lines.push("")
    lines.push("## Chapter Information")
    lines.push("")
    lines.push(`**Document:** ${doc.title}`)
    lines.push(`**Chapter:** ${chapter.title}`)
    lines.push(`**Current Word Count:** ${chapter.wordCount}`)
    lines.push(`**Target Word Count:** ${input.targetWords}`)
    lines.push(`**Words to Remove:** ${wordsToRemove}`)
    lines.push("")

    const outline = doc.outline.chapters.find((c) => c.id === chapter.outlineID)
    if (outline) {
      lines.push("## Chapter Outline")
      lines.push("")
      lines.push(`**Description:** ${outline.description}`)
      lines.push("")
    }

    if (input.preserveKeyElements?.length) {
      lines.push("## Key Elements to Preserve")
      lines.push("")
      for (const element of input.preserveKeyElements) {
        lines.push(`  - ${element}`)
      }
      lines.push("")
    }

    lines.push("## Current Content")
    lines.push("")
    lines.push(chapter.content)
    lines.push("")
    lines.push("## Instructions")
    lines.push("")
    lines.push(`Compress this chapter by approximately ${wordsToRemove} words while preserving the core meaning and important elements.`)
    lines.push("")
    lines.push("Compression strategies:")
    lines.push("- Remove redundant phrases and explanations")
    lines.push("- Tighten wordy sentences")
    lines.push("- Combine related points")
    lines.push("- Remove unnecessary adjectives and adverbs")
    lines.push("- Summarize extended examples")
    lines.push("- Cut tangential content")
    lines.push("")
    lines.push("Important:")
    lines.push("- Preserve all key plot points and character developments")
    lines.push("- Maintain narrative flow")
    lines.push("- Keep the essential voice and style")
    lines.push("")
    lines.push("Output the compressed version in Markdown format.")

    return lines.join("\n")
  }

  /**
   * Batch polish multiple chapters
   */
  export async function batchPolish(input: {
    documentID: string
    chapterIDs: string[]
    aspect: "fluency" | "clarity" | "style" | "tone" | "all"
  }): Promise<Array<{ chapterID: string; prompt: string }>> {
    const results: Array<{ chapterID: string; prompt: string }> = []

    for (const chapterID of input.chapterIDs) {
      const prompt = await polishPrompt({
        documentID: input.documentID,
        chapterID,
        aspect: input.aspect,
      })
      results.push({ chapterID, prompt })
    }

    return results
  }

  /**
   * Get chapter statistics for editing decisions
   */
  export async function getChapterStats(documentID: string, chapterID: string): Promise<{
    wordCount: number
    paragraphCount: number
    sentenceCount: number
    avgWordsPerSentence: number
    avgWordsPerParagraph: number
    readingTimeMinutes: number
  }> {
    const chapter = await Document.Chapter.get(documentID, chapterID)
    if (!chapter) throw new Error("Chapter not found")

    const content = chapter.content

    // Count paragraphs (non-empty lines)
    const paragraphs = content
      .split("\n")
      .filter((line) => line.trim().length > 0)
      .filter((line) => !line.startsWith("#")) // Exclude headings

    // Estimate sentences (rough approximation)
    const sentenceEndings = content.match(/[。！？.!?\n]/g) || []
    const sentences = Math.max(1, sentenceEndings.length)

    const paragraphCount = paragraphs.length
    const sentenceCount = sentences
    const avgWordsPerSentence = chapter.wordCount / sentenceCount
    const avgWordsPerParagraph = chapter.wordCount / paragraphCount
    const readingTimeMinutes = Math.ceil(chapter.wordCount / 500) // ~500 words per minute for Chinese

    return {
      wordCount: chapter.wordCount,
      paragraphCount,
      sentenceCount,
      avgWordsPerSentence: Math.round(avgWordsPerSentence * 10) / 10,
      avgWordsPerParagraph: Math.round(avgWordsPerParagraph * 10) / 10,
      readingTimeMinutes,
    }
  }

  /**
   * Find chapters that might need editing
   */
  export async function findChaptersNeedingAttention(documentID: string): Promise<
    Array<{
      chapterID: string
      chapterTitle: string
      issues: string[]
      wordCount: number
    }>
  > {
    const chapters = await Document.Chapter.list(documentID)
    const doc = await Document.get(documentID)
    if (!doc) return []

    const results: Array<{
      chapterID: string
      chapterTitle: string
      issues: string[]
      wordCount: number
    }> = []

    for (const chapter of chapters) {
      const issues: string[] = []

      // Check word count against target
      const outline = doc.outline.chapters.find((c) => c.id === chapter.outlineID)
      if (outline && chapter.wordCount > 0) {
        const ratio = chapter.wordCount / outline.estimatedWords
        if (ratio < 0.5) {
          issues.push("Significantly under target word count")
        } else if (ratio > 1.5) {
          issues.push("Significantly over target word count")
        }
      }

      // Check for missing summary
      if (!chapter.summary && chapter.status === "completed") {
        issues.push("Missing chapter summary")
      }

      // Check for very short content
      if (chapter.wordCount < 500 && chapter.status === "completed") {
        issues.push("Very short chapter - may need expansion")
      }

      // Check for empty content
      if (chapter.wordCount === 0 && chapter.status !== "pending") {
        issues.push("Empty content")
      }

      if (issues.length > 0) {
        results.push({
          chapterID: chapter.id,
          chapterTitle: chapter.title,
          issues,
          wordCount: chapter.wordCount,
        })
      }
    }

    return results
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  async function getChaptersByScope(
    documentID: string,
    scope: string,
    chapterID?: string,
    volumeID?: string,
  ): Promise<DocumentSchema.Chapter[]> {
    switch (scope) {
      case "chapter":
        if (chapterID) {
          const chapter = await Document.Chapter.get(documentID, chapterID)
          return chapter ? [chapter] : []
        }
        return []

      case "volume":
        if (volumeID) {
          return await Volume.getChapters(documentID, volumeID)
        }
        return await Document.Chapter.list(documentID)

      case "global":
      default:
        return await Document.Chapter.list(documentID)
    }
  }
}
