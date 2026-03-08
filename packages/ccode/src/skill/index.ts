/**
 * Skill management module.
 *
 * This module provides skill preloading and management utilities.
 */

export { Skill } from './skill'

export {
  SkillPreloader,
  createPreloader,
  extractSignals,
  predictFromHeuristics,
  type PreloadPrediction,
  type ContextSignals,
  type PreloaderConfig,
  type PreloadContext,
} from './preloader'
