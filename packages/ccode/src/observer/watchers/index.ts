/**
 * Watchers Module
 *
 * Exports all watcher implementations for the Observer Network.
 *
 * @deprecated This entire TypeScript watchers module is deprecated in favor of
 * the Rust implementation in services/zero-cli/src/observer/watchers/. The Rust
 * implementation provides:
 * - Better performance with native async and git2 integration
 * - Integrated daemon lifecycle management
 * - Direct connection to ConsensusEngine via channels
 *
 * Migration was completed in Phase 6-7 of the architecture refactoring.
 * These exports are retained for backward compatibility but will be removed
 * in a future release.
 *
 * @see services/zero-cli/src/observer/watchers/mod.rs - Rust WatcherManager
 * @see services/zero-cli/src/observer/watchers/code_watch.rs - Rust CodeWatch
 * @see services/zero-cli/src/observer/watchers/world_watch.rs - Rust WorldWatch
 * @see services/zero-cli/src/observer/watchers/self_watch.rs - Rust SelfWatch
 * @see services/zero-cli/src/observer/watchers/meta_watch.rs - Rust MetaWatch
 *
 * @module observer/watchers
 */

export { BaseWatcher, type WatcherOptions, type WatcherType } from "./base-watcher"
export { CodeWatch, createCodeWatch, type CodeWatchOptions } from "./code-watch"
export { WorldWatch, createWorldWatch, type WorldWatchOptions } from "./world-watch"
export { SelfWatch, createSelfWatch, type SelfWatchOptions } from "./self-watch"
export { MetaWatch, createMetaWatch, type MetaWatchOptions } from "./meta-watch"
