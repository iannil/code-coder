/**
 * Forum Moderator for multi-agent discussions.
 *
 * The moderator guides forum discussions, providing context and focus
 * areas for each speaker's turn, and summarizing outcomes.
 *
 * ## Design Principle
 *
 * Moderation requires **understanding** the discussion flow and providing
 * appropriate guidance. This is an inherently uncertain task that benefits
 * from LLM reasoning.
 */

/**
 * A single turn in a forum discussion.
 */
export interface ForumTurn {
  /** Round number (1-indexed) */
  round: number
  /** Speaker identifier */
  speaker: string
  /** Content of the turn */
  content: string
  /** Timestamp */
  timestamp: Date
}

/**
 * Guidance prompt for a speaker's turn.
 */
export interface GuidancePrompt {
  /** Main instruction for the speaker */
  instruction: string
  /** Context from previous turns */
  context: string
  /** Specific areas to focus on */
  focusAreas: string[]
  /** Questions to address */
  questionsToAddress: string[]
  /** Suggested structure for response */
  suggestedStructure?: string
}

/**
 * Result of consensus detection in a forum.
 */
export interface ForumConsensusResult {
  /** Whether consensus was reached */
  reached: boolean
  /** Consensus strength (0-1) */
  confidence: number
  /** Points of agreement */
  points: ConsensusPoint[]
  /** Speakers with dissenting views */
  dissenting: string[]
  /** Summary of the consensus (or lack thereof) */
  summary: string
}

/**
 * A point of consensus.
 */
export interface ConsensusPoint {
  /** The agreed-upon point */
  statement: string
  /** Speakers who agree */
  supporters: string[]
  /** Strength of agreement (0-1) */
  strength: number
}

/**
 * Summary of a forum discussion.
 */
export interface ForumSummary {
  /** Topic of discussion */
  topic: string
  /** Number of participants */
  participants: number
  /** Number of rounds */
  rounds: number
  /** Total turns taken */
  totalTurns: number
  /** Key points discussed */
  keyPoints: string[]
  /** Consensus result */
  consensus: ForumConsensusResult
  /** Action items identified */
  actionItems: string[]
  /** Final recommendation if any */
  recommendation?: string
}

/**
 * Forum Moderator for guiding multi-agent discussions.
 */
export class ForumModerator {
  private topic: string
  private participants: string[]
  private history: ForumTurn[]

  constructor(topic: string, participants: string[]) {
    this.topic = topic
    this.participants = participants
    this.history = []
  }

  /**
   * Generate guidance for a speaker's turn.
   */
  async guideTurn(
    currentSpeaker: string,
    currentRound: number
  ): Promise<GuidancePrompt> {
    const recentTurns = this.getRecentTurns(3)
    const context = this.buildContext(recentTurns)
    const focusAreas = this.identifyFocusAreas(recentTurns, currentRound)
    const questions = this.extractUnansweredQuestions(recentTurns)

    let instruction: string

    if (currentRound === 1) {
      // First round: introduce perspective
      instruction = `As ${currentSpeaker}, please share your initial perspective on: "${this.topic}". Consider your expertise and unique viewpoint.`
    } else if (this.isLastSpeakerInRound(currentSpeaker)) {
      // Last speaker in round: synthesize and advance
      instruction = `As ${currentSpeaker}, you're the last to speak this round. Please synthesize the key points raised and either build consensus or highlight remaining disagreements.`
    } else {
      // Middle of round: respond and build
      instruction = `As ${currentSpeaker}, please respond to the previous points and add your perspective. Build on areas of agreement and address any concerns.`
    }

    return {
      instruction,
      context,
      focusAreas,
      questionsToAddress: questions,
      suggestedStructure: this.suggestStructure(currentRound, focusAreas.length),
    }
  }

  /**
   * Record a turn in the discussion.
   */
  recordTurn(turn: ForumTurn): void {
    this.history.push(turn)
  }

  /**
   * Detect consensus across the discussion.
   */
  async detectConsensus(): Promise<ForumConsensusResult> {
    if (this.history.length === 0) {
      return {
        reached: false,
        confidence: 0,
        points: [],
        dissenting: [],
        summary: 'No discussion has occurred yet.',
      }
    }

    // Extract key points from each speaker
    const speakerPoints = new Map<string, string[]>()
    for (const turn of this.history) {
      const points = this.extractKeyPoints(turn.content)
      const existing = speakerPoints.get(turn.speaker) || []
      speakerPoints.set(turn.speaker, [...existing, ...points])
    }

    // Find overlapping points
    const allPoints = Array.from(speakerPoints.values()).flat()
    const pointFrequency = new Map<string, string[]>()

    for (const [speaker, points] of speakerPoints.entries()) {
      for (const point of points) {
        const normalized = this.normalizePoint(point)
        const supporters = pointFrequency.get(normalized) || []
        if (!supporters.includes(speaker)) {
          supporters.push(speaker)
          pointFrequency.set(normalized, supporters)
        }
      }
    }

    // Identify consensus points (supported by majority)
    const threshold = Math.ceil(this.participants.length / 2)
    const consensusPoints: ConsensusPoint[] = []

    for (const [point, supporters] of pointFrequency.entries()) {
      if (supporters.length >= threshold) {
        consensusPoints.push({
          statement: point,
          supporters,
          strength: supporters.length / this.participants.length,
        })
      }
    }

    // Identify dissenting speakers
    const consensusSupporters = new Set(
      consensusPoints.flatMap((p) => p.supporters)
    )
    const dissenting = this.participants.filter(
      (p) => !consensusSupporters.has(p)
    )

    // Calculate overall confidence
    const confidence =
      consensusPoints.length > 0
        ? consensusPoints.reduce((sum, p) => sum + p.strength, 0) /
          consensusPoints.length
        : 0

    // Build summary
    const summary = this.buildConsensusSummary(consensusPoints, dissenting)

    return {
      reached: confidence >= 0.5 && dissenting.length < this.participants.length / 2,
      confidence,
      points: consensusPoints,
      dissenting,
      summary,
    }
  }

  /**
   * Generate a summary of the discussion.
   */
  async summarize(): Promise<ForumSummary> {
    const consensus = await this.detectConsensus()
    const keyPoints = this.extractAllKeyPoints()
    const actionItems = this.extractActionItems()

    const rounds = this.history.length > 0
      ? Math.max(...this.history.map((t) => t.round))
      : 0

    return {
      topic: this.topic,
      participants: this.participants.length,
      rounds,
      totalTurns: this.history.length,
      keyPoints,
      consensus,
      actionItems,
      recommendation: this.generateRecommendation(consensus),
    }
  }

  /**
   * Get the discussion history.
   */
  getHistory(): ForumTurn[] {
    return [...this.history]
  }

  // Private helper methods

  private getRecentTurns(count: number): ForumTurn[] {
    return this.history.slice(-count)
  }

  private buildContext(recentTurns: ForumTurn[]): string {
    if (recentTurns.length === 0) {
      return 'This is the beginning of the discussion.'
    }

    return recentTurns
      .map((t) => `${t.speaker} (Round ${t.round}): ${this.truncate(t.content, 200)}`)
      .join('\n\n')
  }

  private identifyFocusAreas(
    recentTurns: ForumTurn[],
    currentRound: number
  ): string[] {
    const areas: string[] = []

    // First round: broad focus
    if (currentRound === 1) {
      areas.push('Your unique perspective on the topic')
      areas.push('Key considerations from your expertise')
      return areas
    }

    // Extract themes from recent turns
    const themes = new Set<string>()
    for (const turn of recentTurns) {
      const turnThemes = this.extractThemes(turn.content)
      turnThemes.forEach((t) => themes.add(t))
    }

    // Convert to focus areas
    for (const theme of themes) {
      areas.push(`Address: ${theme}`)
    }

    // Add standard focus areas
    if (areas.length < 3) {
      areas.push('Areas of agreement')
      areas.push('Remaining concerns')
    }

    return areas.slice(0, 4)
  }

  private extractUnansweredQuestions(recentTurns: ForumTurn[]): string[] {
    const questions: string[] = []

    for (const turn of recentTurns) {
      // Simple question extraction: look for ? marks
      const sentences = turn.content.split(/[.!?]/)
      for (const sentence of sentences) {
        if (sentence.includes('?') || sentence.toLowerCase().startsWith('what') ||
            sentence.toLowerCase().startsWith('how') || sentence.toLowerCase().startsWith('why')) {
          const trimmed = sentence.trim()
          if (trimmed.length > 10 && trimmed.length < 200) {
            questions.push(trimmed)
          }
        }
      }
    }

    // Return unique questions
    return Array.from(new Set(questions)).slice(0, 3)
  }

  private suggestStructure(round: number, focusCount: number): string {
    if (round === 1) {
      return `1. State your main position\n2. Provide supporting rationale\n3. Note any concerns or caveats`
    }

    if (focusCount > 2) {
      return `1. Address the key points raised\n2. State your perspective\n3. Suggest path forward`
    }

    return `1. Respond to previous points\n2. Add your contribution\n3. Identify common ground`
  }

  private isLastSpeakerInRound(speaker: string): boolean {
    const idx = this.participants.indexOf(speaker)
    return idx === this.participants.length - 1
  }

  private extractKeyPoints(content: string): string[] {
    const points: string[] = []
    const lines = content.split('\n')

    for (const line of lines) {
      const trimmed = line.trim()

      // Look for bullet points
      if (/^[-*•]\s/.test(trimmed)) {
        const point = trimmed.replace(/^[-*•]\s*/, '')
        if (point.length > 10 && point.length < 200) {
          points.push(point)
        }
      }

      // Look for numbered items
      if (/^\d+\.\s/.test(trimmed)) {
        const point = trimmed.replace(/^\d+\.\s*/, '')
        if (point.length > 10 && point.length < 200) {
          points.push(point)
        }
      }
    }

    return points
  }

  private normalizePoint(point: string): string {
    return point
      .toLowerCase()
      .replace(/[^\w\s]/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
  }

  private extractThemes(content: string): string[] {
    const themes: string[] = []
    const keywords = [
      'should', 'must', 'need', 'recommend', 'suggest',
      'concern', 'risk', 'benefit', 'advantage', 'disadvantage',
      'important', 'critical', 'key'
    ]

    const sentences = content.split(/[.!?]/)
    for (const sentence of sentences) {
      const lower = sentence.toLowerCase()
      if (keywords.some((k) => lower.includes(k))) {
        const trimmed = sentence.trim()
        if (trimmed.length > 15 && trimmed.length < 150) {
          themes.push(trimmed)
        }
      }
    }

    return themes.slice(0, 5)
  }

  private buildConsensusSummary(
    points: ConsensusPoint[],
    dissenting: string[]
  ): string {
    if (points.length === 0) {
      return 'No clear consensus emerged from the discussion.'
    }

    let summary = `The group reached consensus on ${points.length} point(s):\n`
    for (const point of points.slice(0, 3)) {
      summary += `- ${point.statement} (${(point.strength * 100).toFixed(0)}% agreement)\n`
    }

    if (dissenting.length > 0) {
      summary += `\n${dissenting.join(', ')} expressed dissenting views.`
    }

    return summary
  }

  private extractAllKeyPoints(): string[] {
    const allPoints: string[] = []
    for (const turn of this.history) {
      allPoints.push(...this.extractKeyPoints(turn.content))
    }
    return Array.from(new Set(allPoints)).slice(0, 10)
  }

  private extractActionItems(): string[] {
    const items: string[] = []
    const actionKeywords = ['should', 'must', 'need to', 'will', 'action', 'next step']

    for (const turn of this.history) {
      const sentences = turn.content.split(/[.!?]/)
      for (const sentence of sentences) {
        const lower = sentence.toLowerCase()
        if (actionKeywords.some((k) => lower.includes(k))) {
          const trimmed = sentence.trim()
          if (trimmed.length > 20 && trimmed.length < 200) {
            items.push(trimmed)
          }
        }
      }
    }

    return Array.from(new Set(items)).slice(0, 5)
  }

  private generateRecommendation(consensus: ForumConsensusResult): string | undefined {
    if (!consensus.reached) {
      return 'Further discussion or additional input is needed to reach consensus.'
    }

    if (consensus.points.length === 0) {
      return undefined
    }

    // Take the highest-strength consensus point
    const strongest = consensus.points.reduce((a, b) =>
      a.strength > b.strength ? a : b
    )

    return `Based on group consensus: ${strongest.statement}`
  }

  private truncate(text: string, maxLength: number): string {
    if (text.length <= maxLength) return text
    return text.slice(0, maxLength - 3) + '...'
  }
}

/**
 * Create a forum moderator for a discussion.
 */
export function createModerator(
  topic: string,
  participants: string[]
): ForumModerator {
  return new ForumModerator(topic, participants)
}
