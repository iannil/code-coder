/**
 * Task API Module
 * Async task flow model for ZeroBot integration
 *
 * This module provides:
 * - Task lifecycle management (create, run, complete, fail)
 * - Per-task SSE event streaming
 * - HITL (Human-in-the-Loop) approval flow
 * - Remote context injection
 */

export * from "./types"
export { TaskStore } from "./store"
export { TaskEmitter } from "./emitter"
