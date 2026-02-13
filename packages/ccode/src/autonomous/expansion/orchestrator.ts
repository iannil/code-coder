import { Document } from "../../document"
import { Knowledge, KnowledgeSchema } from "../../document/knowledge"
import type {
  ExpansionContext,
} from "./states"
import {
  ExpansionState,
  VALID_EXPANSION_TRANSITIONS,
  isValidExpansionTransition,
  getExpansionPhase,
  createContext,
} from "./states"
import { Agent } from "../../agent/agent"

/**
 * Expansion Orchestrator - manages the autonomous book-writer workflow.
 *
 * The orchestrator follows a five-phase process:
 * 1. Idea Analysis
 * 2. Framework Building
 * 3. Outline Generation
 * 4. Iterative Writing
 * 5. Consistency Validation
 */

// Type aliases for commonly used types
type CoreIdeaAnalysis = KnowledgeSchema.CoreIdeaAnalysis
type ChapterOutline = KnowledgeSchema.ChapterOutline
type Outline = KnowledgeSchema.Outline

export namespace ExpansionOrchestrator {
  /**
   * Run a complete expansion workflow.
   */
  export async function run(input: {
    documentID: string
    coreIdea: string
    targetWords: number
    contentType?: "fiction" | "nonfiction" | "auto"
    autonomy?: "autonomous" | "stage-confirm" | "interactive"
    onProgress?: (context: ExpansionContext) => void
    onPhaseComplete?: (phase: string, context: ExpansionContext) => void
  }): Promise<{
    success: boolean
    context: ExpansionContext
    error?: string
  }> {
    const {
      documentID,
      coreIdea,
      targetWords,
      contentType = "auto",
      autonomy = "stage-confirm",
      onProgress,
      onPhaseComplete,
    } = input

    let context = createContext({
      documentID,
      coreIdea,
      targetWords,
      contentType,
      autonomy,
    })

    try {
      // Phase 1: Idea Analysis
      context = await transitionTo(context, ExpansionState.EXPANSION_ANALYZING)
      onProgress?.(context)

      context = await executeIdeaAnalysis(context)
      onPhaseComplete?.("idea_analysis", context)

      if (autonomy === "stage-confirm" || autonomy === "interactive") {
        // Wait for confirmation (in real implementation, this would pause)
        context = await transitionTo(context, ExpansionState.EXPANSION_ANALYSIS_COMPLETE)
      }

      // Phase 2: Framework Building
      context = await transitionTo(context, ExpansionState.EXPANSION_BUILDING)
      onProgress?.(context)

      context = await executeFrameworkBuilding(context)
      onPhaseComplete?.("framework_building", context)

      if (autonomy === "stage-confirm" || autonomy === "interactive") {
        context = await transitionTo(context, ExpansionState.EXPANSION_FRAMEWORK_COMPLETE)
      }

      // Phase 3: Outline Generation
      context = await transitionTo(context, ExpansionState.EXPANSION_OUTLINING)
      onProgress?.(context)

      context = await executeOutlineGeneration(context)
      onPhaseComplete?.("outline_generation", context)

      if (autonomy === "stage-confirm" || autonomy === "interactive") {
        context = await transitionTo(context, ExpansionState.EXPANSION_OUTLINE_COMPLETE)
      }

      // Phase 4: Iterative Writing
      context = await transitionTo(context, ExpansionState.EXPANSION_WRITING)
      onProgress?.(context)

      const outline = context.outline!
      for (let i = 0; i < outline.chapters.length; i++) {
        context = await executeChapterWriting(context, i)
        onProgress?.(context)

        if (context.error) {
          if (context.retryCount >= 3) {
            throw new Error(`Failed to write chapter ${i + 1} after 3 retries`)
          }
          context.retryCount++
          context = await transitionTo(context, ExpansionState.EXPANSION_WRITING)
        }
      }

      context = await transitionTo(context, ExpansionState.EXPANSION_WRITING_COMPLETE)
      onPhaseComplete?.("iterative_writing", context)

      // Phase 5: Consistency Validation
      context = await transitionTo(context, ExpansionState.EXPANSION_VALIDATING)
      onProgress?.(context)

      context = await executeConsistencyValidation(context)
      onPhaseComplete?.("consistency_validation", context)

      // Complete
      context = await transitionTo(context, ExpansionState.EXPANSION_COMPLETE)

      return { success: true, context }
    } catch (error) {
      context = await transitionTo(context, ExpansionState.EXPANSION_FAILED)
      context.error = error instanceof Error ? error.message : String(error)
      return { success: false, context, error: context.error }
    }
  }

  /**
   * Execute Phase 1: Idea Analysis
   */
  async function executeIdeaAnalysis(
    context: ExpansionContext,
  ): Promise<ExpansionContext> {
    const agent = await Agent.get("expander")
    if (!agent) throw new Error("Expander agent not found")

    // Create analysis prompt
    const analysisPrompt = `
Analyze the following core idea for book expansion:

"${context.coreIdea}"

Target word count: ${context.targetWords}
Content type preference: ${context.contentType}

Please provide:
1. Content type detection (fiction/nonfiction/mixed)
2. Core thesis/theme extraction
3. Main themes identification
4. Key concepts extraction
5. Suggested chapter count
6. For fiction: potential conflicts
7. For non-fiction: potential arguments

Output in YAML format compatible with CoreIdeaAnalysis schema.
`

    // In production, this would call the AI agent
    // For now, use the Framework module
    const analysis = await Knowledge.Framework.analyzeCoreIdea({
      idea: context.coreIdea,
      targetWords: context.targetWords,
      contentType: context.contentType,
    })

    return {
      ...context,
      ideaAnalysis: analysis,
      currentState: ExpansionState.EXPANSION_ANALYSIS_COMPLETE,
      updatedAt: Date.now(),
    }
  }

  /**
   * Execute Phase 2: Framework Building
   */
  async function executeFrameworkBuilding(
    context: ExpansionContext,
  ): Promise<ExpansionContext> {
    if (!context.ideaAnalysis) {
      throw new Error("Idea analysis required for framework building")
    }

    // Create thematic framework
    const thematicFramework = await Knowledge.Framework.createThematicFramework({
      documentID: context.documentID,
      analysis: context.ideaAnalysis,
    })

    // Save framework
    await Knowledge.Framework.save(context.documentID, thematicFramework)

    const updates: Partial<ExpansionContext> = {
      thematicFramework,
      currentState: ExpansionState.EXPANSION_FRAMEWORK_COMPLETE,
      updatedAt: Date.now(),
    }

    // Create world framework for fiction
    if (context.ideaAnalysis.contentType === "fiction") {
      const worldFramework = await Knowledge.Framework.createWorldFramework({
        documentID: context.documentID,
        analysis: context.ideaAnalysis,
      })

      await Knowledge.StoryElements.createWorldFramework(context.documentID, worldFramework)
      updates.worldFramework = worldFramework
    }

    return { ...context, ...updates }
  }

  /**
   * Execute Phase 3: Outline Generation
   */
  async function executeOutlineGeneration(
    context: ExpansionContext,
  ): Promise<ExpansionContext> {
    if (!context.thematicFramework) {
      throw new Error("Thematic framework required for outline generation")
    }

    const ideaAnalysis = context.ideaAnalysis!

    // Generate chapter outlines based on analysis
    const chapters: ChapterOutline[] = []
    const chapterCount = ideaAnalysis.suggestedChapterCount
    const wordsPerChapter = Math.floor(context.targetWords / chapterCount)

    for (let i = 0; i < chapterCount; i++) {
      const chapterNumber = i + 1
      chapters.push({
        id: `outline_ch_${String(i + 1).padStart(3, "0")}`,
        title: generateChapterTitle(context.ideaAnalysis!.contentType, chapterNumber, ideaAnalysis),
        description: generateChapterDescription(
          context.ideaAnalysis!.contentType,
          chapterNumber,
          chapterCount,
          ideaAnalysis,
        ),
        estimatedWords: wordsPerChapter,
        subsections: undefined,
      })
    }

    const outline: Outline = {
      title: ideaAnalysis.coreThesis.slice(0, 50),
      description: `Expanded from: ${context.coreIdea.slice(0, 200)}`,
      chapters,
    }

    // Update document with outline
    // Convert KnowledgeSchema.Outline to DocumentSchema.Outline
    const documentOutline = {
      title: outline.title,
      description: outline.description,
      chapters: outline.chapters.map((ch) => ({
        id: ch.id,
        title: ch.title,
        description: ch.description ?? "",
        estimatedWords: ch.estimatedWords,
        subsections: undefined,
      })),
    }
    await Document.updateOutline({
      documentID: context.documentID,
      outline: documentOutline,
    })

    return {
      ...context,
      outline,
      currentState: ExpansionState.EXPANSION_OUTLINE_COMPLETE,
      updatedAt: Date.now(),
    }
  }

  /**
   * Execute Phase 4: Chapter Writing
   */
  async function executeChapterWriting(
    context: ExpansionContext,
    chapterIndex: number,
  ): Promise<ExpansionContext> {
    if (!context.outline) {
      throw new Error("Outline required for chapter writing")
    }

    const chapterOutline = context.outline.chapters[chapterIndex]
    if (!chapterOutline) {
      throw new Error(`Chapter outline not found for index ${chapterIndex}`)
    }

    // Get existing chapters to find or create the chapter
    const chapters = await Document.Chapter.list(context.documentID)
    let chapter = chapters.find((ch) => ch.outlineID === chapterOutline.id)

    if (!chapter) {
      chapter = await Document.Chapter.create({
        documentID: context.documentID,
        outlineID: chapterOutline.id,
        title: chapterOutline.title,
      })
    }

    // Get knowledge context for writing
    const { KnowledgeNode } = await import("../../document/knowledge")
    const knowledgeNodes = await KnowledgeNode.list(context.documentID)

    // Select appropriate agent
    const agentType = context.ideaAnalysis?.contentType === "fiction"
      ? "expander-fiction"
      : "expander-nonfiction"

    const agent = await Agent.get(agentType)
    if (!agent) throw new Error(`${agentType} agent not found`)

    // In production, this would call the AI agent to write the chapter
    // For now, we create a placeholder
    const content = `# ${chapter.title}

[Chapter content will be generated by ${agentType} agent based on:
- Thematic framework: ${context.thematicFramework?.thesis}
- Previous chapters: ${chapterIndex} completed
- Target word count: ${chapterOutline.estimatedWords}
- Knowledge context: ${knowledgeNodes.length} nodes established]

### Chapter Summary
[Summary will be auto-generated after writing]
`

    await Document.Chapter.update({
      documentID: context.documentID,
      chapterID: chapter.id,
      content,
      status: "completed",
    })

    const newWordsWritten = context.wordsWritten + chapterOutline.estimatedWords

    return {
      ...context,
      currentChapterIndex: chapterIndex + 1,
      wordsWritten: newWordsWritten,
      currentState:
        chapterIndex + 1 >= context.outline.chapters.length
          ? ExpansionState.EXPANSION_WRITING_COMPLETE
          : ExpansionState.EXPANSION_CHAPTER_COMPLETE,
      updatedAt: Date.now(),
    }
  }

  /**
   * Execute Phase 5: Consistency Validation
   */
  async function executeConsistencyValidation(
    context: ExpansionContext,
  ): Promise<ExpansionContext> {
    // Validate framework if it exists
    let consistencyScore = 0.5 // Default score
    let issuesCount = 0

    if (context.thematicFramework) {
      const validation = await Knowledge.Framework.validateFramework({
        documentID: context.documentID,
        framework: context.thematicFramework,
      })

      consistencyScore = validation.isValid ? 0.9 : 0.7
      issuesCount = validation.issues.length
    }

    // Check for argument consistency in non-fiction
    if (context.ideaAnalysis?.contentType === "nonfiction") {
      const coherenceCheck = await Knowledge.ArgumentChain.checkCoherence(context.documentID)
      consistencyScore = (consistencyScore + coherenceCheck.overallScore) / 2
      issuesCount += coherenceCheck.issues.length
    }

    // Check for worldview consistency in fiction
    if (context.worldFramework) {
      const chapters = await Document.Chapter.list(context.documentID)
      const worldviewCheck = await Knowledge.StoryElements.checkWorldviewConsistency({
        documentID: context.documentID,
        worldFramework: context.worldFramework,
        chapters,
      })

      if (!worldviewCheck.isConsistent) {
        consistencyScore *= 0.8
        issuesCount += worldviewCheck.issues.length
      }
    }

    return {
      ...context,
      consistencyScore,
      consistencyIssues: issuesCount,
      currentState: ExpansionState.EXPANSION_VALIDATION_COMPLETE,
      updatedAt: Date.now(),
    }
  }

  /**
   * Transition to a new state with validation.
   */
  async function transitionTo(
    context: ExpansionContext,
    newState: ExpansionState,
  ): Promise<ExpansionContext> {
    if (!isValidExpansionTransition(context.currentState, newState)) {
      throw new Error(
        `Invalid transition from ${context.currentState} to ${newState}`,
      )
    }

    return {
      ...context,
      previousState: context.currentState,
      currentState: newState,
      updatedAt: Date.now(),
    }
  }

  /**
   * Generate a chapter title based on content type and position.
   */
  function generateChapterTitle(
    contentType: "fiction" | "nonfiction" | "mixed",
    chapterNumber: number,
    analysis: CoreIdeaAnalysis,
  ): string {
    const fictionTitles = [
      "The Beginning",
      "Crossing the Threshold",
      "Trials and Challenges",
      "The Revelation",
      "The Final Choice",
      "Resolution",
      "A New Dawn",
    ]

    const nonfictionTitles = [
      "Introduction and Foundations",
      "Understanding the Problem",
      "Key Principles",
      "The Evidence",
      "Analysis and Insights",
      "Practical Applications",
      "Case Studies",
      "Implementation Strategies",
      "Future Directions",
      "Conclusion and Synthesis",
    ]

    if (contentType === "fiction") {
      return fictionTitles[chapterNumber % fictionTitles.length] || `Chapter ${chapterNumber}`
    }

    return nonfictionTitles[chapterNumber % nonfictionTitles.length] || `Chapter ${chapterNumber}`
  }

  /**
   * Generate a chapter description.
   */
  function generateChapterDescription(
    contentType: "fiction" | "nonfiction" | "mixed",
    chapterNumber: number,
    totalChapters: number,
    analysis: CoreIdeaAnalysis,
  ): string {
    const position =
      chapterNumber === 1
        ? "opening"
        : chapterNumber === totalChapters
          ? "conclusion"
          : chapterNumber >= totalChapters * 0.7
            ? "climax"
            : "development"

    if (contentType === "fiction") {
      const descriptions: Record<string, string> = {
        opening: "Establish characters, setting, and initial situation. Introduce the central conflict.",
        development: "Advance the plot through complications and character growth. Build tension.",
        climax: "The peak of emotional intensity. Major revelations or confrontations occur.",
        conclusion: "Resolve conflicts and show character transformation. Provide closure.",
      }
      return descriptions[position] || "Advance the story."
    }

    const descriptions: Record<string, string> = {
      opening: "Introduce the main thesis and outline the scope of the work.",
      development: `Develop supporting arguments and present evidence. Build the case systematically.`,
      climax: "Address counterarguments and present the strongest evidence for the thesis.",
      conclusion: "Synthesize arguments and demonstrate how the thesis is proven. Discuss implications.",
    }

    return descriptions[position] || "Develop the argument."
  }

  /**
   * Get current progress summary.
   */
  export async function getProgress(
    documentID: string,
  ): Promise<{
    phase: string
    progress: number
    chaptersCompleted: number
    chaptersTotal: number
    wordsWritten: number
    targetWords: number
  }> {
    const doc = await Document.get(documentID)
    if (!doc) {
      throw new Error("Document not found")
    }

    const chapters = await Document.Chapter.list(documentID)
    const completedChapters = chapters.filter((ch) => ch.status === "completed").length

    return {
      phase: "unknown", // Would be retrieved from storage in production
      progress: doc.targetWords > 0 ? Math.round((doc.currentWords / doc.targetWords) * 100) : 0,
      chaptersCompleted: completedChapters,
      chaptersTotal: doc.outline.chapters.length,
      wordsWritten: doc.currentWords,
      targetWords: doc.targetWords,
    }
  }

  /**
   * Pause an active expansion.
   */
  export async function pause(documentID: string): Promise<void> {
    // In production, this would update the stored context
    // For now, it's a placeholder
    console.log(`Pausing expansion for document: ${documentID}`)
  }

  /**
   * Resume a paused expansion.
   */
  export async function resume(documentID: string): Promise<void> {
    // In production, this would retrieve the stored context and continue
    // For now, it's a placeholder
    console.log(`Resuming expansion for document: ${documentID}`)
  }

  /**
   * Cancel an expansion.
   */
  export async function cancel(documentID: string): Promise<void> {
    // Clean up any stored expansion state
    console.log(`Canceling expansion for document: ${documentID}`)
  }
}
