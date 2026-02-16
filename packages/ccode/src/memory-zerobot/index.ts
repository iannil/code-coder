/**
 * ZeroBot Memory Integration
 *
 * Provides CodeCoder access to ZeroBot's SQLite-based memory system.
 * Enables shared memory between ZeroBot (Rust) and CodeCoder (TypeScript).
 *
 * Usage:
 * ```typescript
 * import { createZeroBotMemory } from "@/memory-zerobot"
 *
 * const memory = createZeroBotMemory()
 *
 * if (memory.isAvailable()) {
 *   // Store a memory
 *   memory.store("user_preference", "Prefers Rust over Python", "core")
 *
 *   // Recall related memories
 *   const results = memory.recall("programming language", 5)
 *
 *   // Get specific memory
 *   const pref = memory.get("user_preference")
 *
 *   // List all core memories
 *   const core = memory.list("core")
 *
 *   // Forget a memory
 *   memory.forget("outdated_key")
 * }
 * ```
 *
 * @module memory-zerobot
 */

export { ZeroBotMemoryProvider, createZeroBotMemory } from "./provider"
export type { MemoryEntry, MemoryCategory, ZeroBotMemoryConfig } from "./types"
