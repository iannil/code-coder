import { Document } from "./index"
import { DocumentSchema } from "./schema"
import { Context } from "./context"

export namespace Writer {
  export async function generatePrompt(input: {
    documentID: string
    chapterID: string
  }): Promise<string> {
    const doc = await Document.get(input.documentID)
    if (!doc) throw new Error("Document not found")

    const chapter = await Document.Chapter.get(input.documentID, input.chapterID)
    if (!chapter) throw new Error("Chapter not found")

    const chapters = await Document.Chapter.list(input.documentID)
    const currentIndex = chapters.findIndex((c) => c.id === input.chapterID)
    const outline = doc.outline.chapters.find((c) => c.id === chapter.outlineID)
    if (!outline) throw new Error("Chapter outline not found")

    const completed = chapters.filter((c) => c.status === "completed").length
    const total = chapters.length

    // Use new intelligent context selection
    const selectedContext = await Context.selectContextForChapter({
      documentID: input.documentID,
      chapterID: input.chapterID,
    })

    const lines: string[] = []

    // Header with progress
    lines.push("# Writing Task")
    lines.push("")
    lines.push(`**Progress:** ${completed}/${total} chapters completed (${currentIndex + 1}/${total} current)`)
    lines.push("")

    // Format and add context
    lines.push(Context.formatContextForPrompt(selectedContext, doc.title))
    lines.push("")

    // Current chapter task
    lines.push("---")
    lines.push("")
    lines.push("## Current Chapter Task")
    lines.push("")
    lines.push(`**Chapter:** ${chapter.title}`)
    lines.push(`**Description:** ${outline.description}`)
    lines.push(`**Estimated Words:** ${outline.estimatedWords}`)
    if (outline.subsections?.length) {
      lines.push(`**Sections to cover:** ${outline.subsections.join(", ")}`)
    }
    lines.push("")

    // Writing instructions
    lines.push("### Instructions")
    lines.push("")
    lines.push("Please write this chapter following these requirements:")
    lines.push("")
    lines.push("1. **Content Quality**")
    lines.push("   - Write engaging, well-structured content")
    lines.push("   - Use clear, concise language")
    lines.push("   - Vary sentence structure for better flow")
    lines.push("")
    lines.push("2. **Consistency**")
    lines.push("   - Maintain consistency with all tracked entities (characters, locations, etc.)")
    lines.push("   - Follow the established writing style")
    lines.push("   - Ensure smooth transitions from previous chapters")
    lines.push("")
    lines.push("3. **Structure**")
    if (outline.subsections?.length) {
      lines.push(`   - Cover these sections: ${outline.subsections.join(", ")}`)
    }
    lines.push("   - Begin with a clear introduction")
    lines.push("   - End with a transition to the next chapter")
    lines.push("")
    lines.push("4. **Output Format**")
    lines.push("   - Write the chapter content in Markdown")
    lines.push("   - After the content, provide a 200-300 word summary in Chinese")
    lines.push("   - The summary should capture: main topics, key points, characters/entities introduced, and any setup for future chapters")
    lines.push("")
    lines.push("Please begin writing now.")

    return lines.join("\n")
  }

  export function parseSummaryFromResponse(response: string): string {
    const lines = response.split("\n")

    let summaryStart = -1
    for (let i = lines.length - 1; i >= 0; i--) {
      const line = lines[i].trim().toLowerCase()
      if (line.includes("summary") || line.includes("摘要") || line.includes("总结")) {
        summaryStart = i
        break
      }
    }

    if (summaryStart >= 0) {
      const summary = lines.slice(summaryStart + 1).join("\n").trim()
      if (summary.length > 50) return summary
    }

    const lastPara = response.split("\n\n").pop()
    if (lastPara && lastPara.length > 50 && lastPara.length < 500) {
      return lastPara
    }

    return response.slice(-300).trim()
  }

  export function extractContentFromResponse(response: string): string {
    const lines = response.split("\n")
    const content: string[] = []

    let inSummary = false
    for (const line of lines) {
      const lower = line.trim().toLowerCase()
      if (inSummary) continue
      if (
        lower.includes("## summary") ||
        lower.includes("## 摘要") ||
        lower.includes("## 总结") ||
        lower.startsWith("summary:") ||
        lower.startsWith("摘要:")
      ) {
        inSummary = true
        continue
      }
      content.push(line)
    }

    return content.join("\n").trim()
  }

  /**
   * Check if chapter content appears to be truncated
   * @param content - Generated chapter content
   * @param estimatedWords - Target word count from outline
   * @returns Warning message if truncation detected, undefined otherwise
   */
  export function detectTruncation(content: string, estimatedWords: number): string | undefined {
    // Count words (Chinese: ~1 word per 2 chars, English: split by spaces)
    const chineseChars = (content.match(/[\u4e00-\u9fa5]/g) || []).length
    const nonChineseText = content.replace(/[\u4e00-\u9fa5]/g, "")
    const nonChineseWords = nonChineseText.trim() ? nonChineseText.split(/\s+/).length : 0
    const actualWords = Math.floor(chineseChars / 2) + nonChineseWords

    // Check for truncation indicators
    const truncationIndicators = [
      "[content truncated",
      "[由于token限制",
      "[output truncated",
      "content was cut",
      "to be continued",
      "... (to be",
    ]
    const hasTruncationMarker = truncationIndicators.some((indicator) =>
      content.toLowerCase().includes(indicator.toLowerCase()),
    )

    // Check if content ends abruptly (mid-sentence)
    const endsAbruptly = /[^。！？.……\s]\s*$/.test(content.slice(-50))

    // Significant word count deviation (less than 50% of target)
    const significantDeviation = actualWords < estimatedWords * 0.5

    if (hasTruncationMarker || (endsAbruptly && significantDeviation)) {
      return `⚠️ Content may be truncated. Generated ~${actualWords} words, target was ${estimatedWords} words.`
    }

    if (significantDeviation) {
      return `ℹ️ Content is shorter than target. Generated ~${actualWords} words, target was ${estimatedWords} words.`
    }

    return undefined
  }
}
