/**
 * Document Commands Index
 *
 * Re-exports all document-related commands from their individual modules.
 */

// Chapter commands
export {
  ChapterCommand,
  ChapterListCommand,
  ChapterShowCommand,
  ChapterResetCommand,
  ChapterEditCommand,
  ChapterStatsCommand,
} from "./chapter"

// Proofread commands
export {
  ProofreadCommand,
  ProofreadCheckCommand,
  ProofreadReportsCommand,
  ProofreadShowCommand,
  ProofreadFixCommand,
  ProofreadReadabilityCommand,
  ProofreadTerminologyCommand,
  ProofreadBatchCommand,
  ProofreadQuickCommand,
} from "./proofread"

// Snapshot commands
export {
  SnapshotCommand,
  SnapshotCreateCommand,
  SnapshotListCommand,
  SnapshotRollbackCommand,
  SnapshotDiffCommand,
} from "./snapshot"

// Volume commands
export {
  VolumeCommand,
  VolumeCreateCommand,
  VolumeListCommand,
  VolumeSummaryCommand,
  VolumeAutoCommand,
} from "./volume"

// Entity commands
export {
  EntityCommand,
  EntityExtractCommand,
  EntityListCommand,
  EntityShowCommand,
  EntityDuplicatesCommand,
} from "./entity"
