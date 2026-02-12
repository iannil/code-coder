/**
 * Markdown Memory Layer
 *
 * Transparent dual-layer memory architecture:
 * - Flow layer: Daily chronological notes in ./memory/daily/
 * - Sediment layer: Consolidated knowledge in ./memory/MEMORY.md
 *
 * Principles:
 * - Completely independent from existing memory module
 * - Human-readable markdown files
 * - Git-friendly storage
 * - No complex embedding retrieval
 *
 * Storage Configuration:
 * - Environment: CCODE_MEMORY_DIR (base path), CCODE_MEMORY_PROJECT_ID (project ID)
 * - Config file: codecoder.json -> memory.storage section
 * - Default: {process.cwd()}/memory
 */

// Types
export type {
  DailyEntry,
  DailyEntryType,
  MemoryCategory,
  MemorySection,
  LoadOptions,
  MemoryContext,
  MemoryConfig,
  MemoryStorageConfig,
} from "./types"

// Storage provider
export {
  getStorage,
  resetStorage,
  configureMemory,
  getMemoryConfig,
  type MarkdownStorageProvider,
  type LocalMarkdownStorage,
} from "./storage"

// Configuration
export { loadStorageConfig, resetConfigCache } from "./config"

// Project detection
export {
  detectProjectId,
  detectProjectIdSync,
  sanitizeProjectId,
  getProjectContext,
  type ProjectContext,
} from "./project"

// Daily notes (flow layer)
export {
  appendDailyNote,
  loadDailyNotes,
  getTodayNotes,
  listDailyNoteDates,
  getDailyPath,
  createEntry,
} from "./daily"

// Long-term memory (sediment layer)
export {
  loadLongTermMemory,
  loadCategory,
  updateCategory,
  mergeToCategory,
  getMemorySections,
  addListItem,
  removeListItem,
} from "./long-term"

// Context loader
export { loadMarkdownMemoryContext, loadCategoryContext, loadRecentContext, getMemorySummary } from "./loader"

// Utilities
export {
  formatDate,
  formatTimestamp,
  parseDate,
  formatDailyEntry,
  formatSectionHeader,
  extractCategory,
  getLastNDays,
  sanitizeFilename,
  withProjectContext,
  formatProjectMetadata,
  getProjectIdFromEntry,
  isEntryFromProject,
  filterEntriesByProject,
  getUniqueProjectIds,
} from "./util"

// Consolidation
export {
  consolidateMemory,
  getConsolidationStats,
  type ConsolidateOptions,
  type ExtractionResult,
  type ExtractedEntry,
} from "./consolidate"
