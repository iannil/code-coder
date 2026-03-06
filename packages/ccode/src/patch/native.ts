/**
 * Native Patch/Diff Bindings
 *
 * Provides native Rust implementations for patch and diff operations.
 * Falls back to TypeScript implementation if native bindings are unavailable.
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "patch.native" })

// ============================================================================
// Type Definitions (must match NAPI types)
// ============================================================================

export interface NapiPatchChunk {
  contextBefore: string[]
  removals: string[]
  additions: string[]
  contextAfter: string[]
}

export interface NapiPatchHunk {
  path: string
  patchType: "add" | "update" | "delete" | "move"
  movePath?: string
  content?: string
  chunks: NapiPatchChunk[]
}

export interface NapiPatchFileResult {
  filePath: string
  relativePath: string
  patchType: "add" | "update" | "delete" | "move"
  before: string
  after: string
  diff: string
  additions: number
  deletions: number
  movePath?: string
}

export interface NapiApplyPatchResult {
  success: boolean
  files: NapiPatchFileResult[]
  combinedDiff: string
  output: string
  error?: string
  filesChanged: number
  totalAdditions: number
  totalDeletions: number
}

export interface NapiApplyPatchOptions {
  workingDir?: string
  dryRun?: boolean
  createBackups?: boolean
  fuzz?: number
}

export interface NapiEditOperation {
  oldString: string
  newString: string
  replaceAll?: boolean
}

export interface NapiEditResult {
  success: boolean
  replacements: number
  diff: string
  error?: string
  originalHash?: string
}

export interface NapiBestMatch {
  text: string
  ratio: number
}

// ============================================================================
// Handle Interfaces
// ============================================================================

export interface PatchApplicatorHandle {
  parsePatch(patchText: string): NapiPatchHunk[]
  apply(patchText: string, options?: NapiApplyPatchOptions): NapiApplyPatchResult
}

export interface EditorHandle {
  edit(filePath: string, operation: NapiEditOperation): NapiEditResult
  editMultiple(filePath: string, operations: NapiEditOperation[]): NapiEditResult
  generateDiff(oldContent: string, newContent: string, filePath: string): string
  diffFiles(oldPath: string, newPath: string): string
}

// ============================================================================
// Native Bindings Interface
// ============================================================================

interface NativeToolsBindings {
  PatchApplicatorHandle: new () => PatchApplicatorHandle
  EditorHandle: new () => EditorHandle
  similarityRatio(s1: string, s2: string): number
  findBestMatch(needle: string, haystack: string[]): NapiBestMatch | null
  computeDiff(oldContent: string, newContent: string, filePath: string): string
}

// ============================================================================
// Native Bindings Loader
// ============================================================================

let nativeBindings: NativeToolsBindings | null = null
let loadAttempted = false

async function loadNativeBindings(): Promise<NativeToolsBindings | null> {
  if (loadAttempted) return nativeBindings
  loadAttempted = true

  try {
    const bindings = (await import("@codecoder-ai/core")) as unknown as Record<string, unknown>

    // Check if required exports exist
    if (
      typeof bindings.PatchApplicatorHandle === "function" &&
      typeof bindings.EditorHandle === "function" &&
      typeof bindings.similarityRatio === "function" &&
      typeof bindings.findBestMatch === "function" &&
      typeof bindings.computeDiff === "function"
    ) {
      nativeBindings = bindings as unknown as NativeToolsBindings
      log.info("Loaded native patch/edit bindings")
      return nativeBindings
    }
  } catch (e) {
    log.debug("Native patch/edit bindings not available", { error: e })
  }

  return null
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Check if native patch/edit bindings are available
 */
export async function isNativeAvailable(): Promise<boolean> {
  const bindings = await loadNativeBindings()
  return bindings !== null
}

/**
 * Create a new PatchApplicator (native or null)
 */
export async function createPatchApplicator(): Promise<PatchApplicatorHandle | null> {
  const bindings = await loadNativeBindings()
  if (bindings) {
    try {
      return new bindings.PatchApplicatorHandle()
    } catch (e) {
      log.error("Failed to create native PatchApplicator", { error: e })
    }
  }
  return null
}

/**
 * Create a new Editor (native or null)
 */
export async function createEditor(): Promise<EditorHandle | null> {
  const bindings = await loadNativeBindings()
  if (bindings) {
    try {
      return new bindings.EditorHandle()
    } catch (e) {
      log.error("Failed to create native Editor", { error: e })
    }
  }
  return null
}

/**
 * Compute similarity ratio between two strings (0.0 to 1.0)
 * Falls back to null if native bindings unavailable
 */
export async function similarityRatioNative(s1: string, s2: string): Promise<number | null> {
  const bindings = await loadNativeBindings()
  if (bindings) {
    try {
      return bindings.similarityRatio(s1, s2)
    } catch (e) {
      log.debug("Native similarityRatio failed", { error: e })
    }
  }
  return null
}

/**
 * Find the best match for a string in a list of candidates
 * Falls back to null if native bindings unavailable
 */
export async function findBestMatchNative(
  needle: string,
  haystack: string[],
): Promise<NapiBestMatch | null> {
  const bindings = await loadNativeBindings()
  if (bindings) {
    try {
      return bindings.findBestMatch(needle, haystack)
    } catch (e) {
      log.debug("Native findBestMatch failed", { error: e })
    }
  }
  return null
}

/**
 * Compute unified diff between two strings
 * Falls back to null if native bindings unavailable
 */
export async function computeDiffNative(
  oldContent: string,
  newContent: string,
  filePath: string,
): Promise<string | null> {
  const bindings = await loadNativeBindings()
  if (bindings) {
    try {
      return bindings.computeDiff(oldContent, newContent, filePath)
    } catch (e) {
      log.debug("Native computeDiff failed", { error: e })
    }
  }
  return null
}

/**
 * Parse a patch text and return hunks using native implementation
 * Falls back to null if native bindings unavailable
 */
export async function parsePatchNative(patchText: string): Promise<NapiPatchHunk[] | null> {
  const applicator = await createPatchApplicator()
  if (applicator) {
    try {
      return applicator.parsePatch(patchText)
    } catch (e) {
      log.debug("Native parsePatch failed", { error: e })
    }
  }
  return null
}

/**
 * Apply a patch using native implementation
 * Falls back to null if native bindings unavailable
 */
export async function applyPatchNative(
  patchText: string,
  options?: NapiApplyPatchOptions,
): Promise<NapiApplyPatchResult | null> {
  const applicator = await createPatchApplicator()
  if (applicator) {
    try {
      return applicator.apply(patchText, options)
    } catch (e) {
      log.debug("Native applyPatch failed", { error: e })
    }
  }
  return null
}

/**
 * Edit a file using native implementation
 * Falls back to null if native bindings unavailable
 */
export async function editFileNative(
  filePath: string,
  oldString: string,
  newString: string,
  replaceAll?: boolean,
): Promise<NapiEditResult | null> {
  const editor = await createEditor()
  if (editor) {
    try {
      return editor.edit(filePath, { oldString, newString, replaceAll })
    } catch (e) {
      log.debug("Native editFile failed", { error: e })
    }
  }
  return null
}
