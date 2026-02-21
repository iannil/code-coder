import { Log } from "@/util/log"
import { Global } from "@/global"
import { Filesystem } from "@/util/filesystem"
import { Lock } from "@/util/lock"
import path from "path"
import fs from "fs/promises"
import { BootstrapTypes } from "./types"

const log = Log.create({ service: "bootstrap.candidate-store" })

/**
 * CandidateStore manages persistence of skill candidates.
 * Candidates are stored in ~/.codecoder/bootstrap/candidates.json
 */
export namespace CandidateStore {
  const STORE_VERSION = 1

  // Dynamic path getters to respect CCODE_TEST_HOME
  function getStoreDir(): string {
    return path.join(Global.Path.config, "bootstrap")
  }

  function getStoreFile(): string {
    return path.join(getStoreDir(), "candidates.json")
  }

  /**
   * Initialize store directory
   */
  async function ensureDir(): Promise<void> {
    await fs.mkdir(getStoreDir(), { recursive: true })
  }

  /**
   * Create a new empty store
   */
  function createEmpty(): BootstrapTypes.CandidateStore {
    const now = Date.now()
    return {
      version: STORE_VERSION,
      candidates: [],
      time: {
        created: now,
        updated: now,
      },
    }
  }

  /**
   * Read the candidate store
   */
  export async function read(): Promise<BootstrapTypes.CandidateStore> {
    await ensureDir()
    const storeFile = getStoreFile()

    try {
      using _ = await Lock.read(storeFile)
      const file = Bun.file(storeFile)
      if (!(await file.exists())) {
        return createEmpty()
      }

      const text = await file.text()
      const data = JSON.parse(text)
      const parsed = BootstrapTypes.CandidateStore.safeParse(data)

      if (!parsed.success) {
        log.warn("invalid candidate store, creating new", { issues: parsed.error.issues })
        return createEmpty()
      }

      return parsed.data
    } catch (error) {
      log.error("failed to read candidate store", { error })
      return createEmpty()
    }
  }

  /**
   * Write the candidate store
   */
  export async function write(store: BootstrapTypes.CandidateStore): Promise<void> {
    await ensureDir()
    const storeFile = getStoreFile()

    store.time.updated = Date.now()

    using _ = await Lock.write(storeFile)
    await Filesystem.atomicWrite(storeFile, JSON.stringify(store, null, 2))
    log.info("wrote candidate store", { candidateCount: store.candidates.length })
  }

  /**
   * Add a new candidate to the store
   */
  export async function add(candidate: BootstrapTypes.SkillCandidate): Promise<void> {
    const store = await read()

    // Check for duplicate by name
    const existing = store.candidates.find((c) => c.name === candidate.name)
    if (existing) {
      log.warn("candidate already exists, updating", { name: candidate.name })
      const index = store.candidates.indexOf(existing)
      store.candidates[index] = {
        ...candidate,
        metadata: {
          ...candidate.metadata,
          created: existing.metadata.created,
        },
      }
    } else {
      store.candidates.push(candidate)
    }

    await write(store)
  }

  /**
   * Get a candidate by ID
   */
  export async function get(id: string): Promise<BootstrapTypes.SkillCandidate | undefined> {
    const store = await read()
    return store.candidates.find((c) => c.id === id)
  }

  /**
   * Get a candidate by name
   */
  export async function getByName(name: string): Promise<BootstrapTypes.SkillCandidate | undefined> {
    const store = await read()
    return store.candidates.find((c) => c.name === name)
  }

  /**
   * Update a candidate
   */
  export async function update(
    id: string,
    updater: (candidate: BootstrapTypes.SkillCandidate) => void,
  ): Promise<BootstrapTypes.SkillCandidate | undefined> {
    const store = await read()
    const candidate = store.candidates.find((c) => c.id === id)

    if (!candidate) {
      log.warn("candidate not found for update", { id })
      return undefined
    }

    updater(candidate)
    candidate.metadata.updated = Date.now()

    await write(store)
    return candidate
  }

  /**
   * Remove a candidate by ID
   */
  export async function remove(id: string): Promise<boolean> {
    const store = await read()
    const index = store.candidates.findIndex((c) => c.id === id)

    if (index < 0) {
      return false
    }

    store.candidates.splice(index, 1)
    await write(store)
    return true
  }

  /**
   * List all candidates
   */
  export async function list(): Promise<BootstrapTypes.SkillCandidate[]> {
    const store = await read()
    return store.candidates
  }

  /**
   * List candidates by verification status
   */
  export async function listByStatus(
    status: BootstrapTypes.VerificationStatus,
  ): Promise<BootstrapTypes.SkillCandidate[]> {
    const store = await read()
    return store.candidates.filter((c) => c.verification.status === status)
  }

  /**
   * List candidates by confidence threshold
   */
  export async function listByConfidence(minConfidence: number): Promise<BootstrapTypes.SkillCandidate[]> {
    const store = await read()
    return store.candidates.filter((c) => c.verification.confidence >= minConfidence)
  }

  /**
   * List candidates ready for promotion (passed verification + high confidence)
   */
  export async function listReadyForPromotion(
    minConfidence = 0.6,
  ): Promise<BootstrapTypes.SkillCandidate[]> {
    const store = await read()
    return store.candidates.filter(
      (c) => c.verification.status === "passed" && c.verification.confidence >= minConfidence,
    )
  }

  /**
   * Clean up old or low-confidence candidates
   */
  export async function cleanup(options?: {
    maxAge?: number // milliseconds
    minConfidence?: number
    maxCandidates?: number
  }): Promise<number> {
    const store = await read()
    const now = Date.now()
    const initialCount = store.candidates.length

    const {
      maxAge = 30 * 24 * 60 * 60 * 1000, // 30 days
      minConfidence = 0.2,
      maxCandidates = 100,
    } = options ?? {}

    // Filter out old, low-confidence candidates
    store.candidates = store.candidates.filter((c) => {
      const age = now - c.metadata.created
      // Keep if recently created OR has decent confidence
      return age < maxAge || c.verification.confidence >= minConfidence
    })

    // Sort by confidence and keep top N
    if (store.candidates.length > maxCandidates) {
      store.candidates.sort((a, b) => b.verification.confidence - a.verification.confidence)
      store.candidates = store.candidates.slice(0, maxCandidates)
    }

    const removed = initialCount - store.candidates.length
    if (removed > 0) {
      await write(store)
      log.info("cleaned up candidates", { removed, remaining: store.candidates.length })
    }

    return removed
  }

  /**
   * Generate a unique candidate ID
   */
  export function generateId(): string {
    const timestamp = Date.now()
    const random = Math.random().toString(36).slice(2, 9)
    return `cand_${timestamp}_${random}`
  }

  /**
   * Create a new candidate with defaults
   */
  export function create(input: {
    type: BootstrapTypes.SkillType
    name: string
    description: string
    trigger: BootstrapTypes.SkillTrigger
    content: BootstrapTypes.SkillContent
    source: BootstrapTypes.SkillSource
  }): BootstrapTypes.SkillCandidate {
    const now = Date.now()
    return {
      id: generateId(),
      type: input.type,
      name: input.name,
      description: input.description,
      trigger: input.trigger,
      content: input.content,
      source: input.source,
      verification: {
        status: "pending",
        attempts: 0,
        confidence: 0,
      },
      metadata: {
        created: now,
        updated: now,
        usageCount: 0,
      },
    }
  }
}
