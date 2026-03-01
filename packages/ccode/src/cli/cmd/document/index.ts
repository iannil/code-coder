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

// Check commands
export { CheckCommand, CheckStyleCommand } from "./check"

// Context commands
export { ContextCommand, SummaryGlobalCommand } from "./context"

// Core document commands
export {
  DocumentCreateCommand,
  DocumentListCommand,
  DocumentShowCommand,
  DocumentExportCommand,
  DocumentDeleteCommand,
  DocumentUpdateCommand,
  DocumentSetContentCommand,
} from "./core"

// Edit commands
export { SearchReplaceCommand, PolishCommand, ExpandCommand, CompressCommand } from "./edit"

// Entity commands
export {
  EntityCommand,
  EntityExtractCommand,
  EntityListCommand,
  EntityShowCommand,
  EntityDuplicatesCommand,
} from "./entity"

// Outline commands
export { DocumentOutlineCommand } from "./outline"

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

// Template commands
export {
  DocumentTemplateCommand,
  TemplateListCommand,
  TemplateUseCommand,
} from "./template"

// Volume commands
export {
  VolumeCommand,
  VolumeCreateCommand,
  VolumeListCommand,
  VolumeSummaryCommand,
  VolumeAutoCommand,
} from "./volume"

// Write commands
export {
  DocumentWriteCommand,
  DocumentWriteAllCommand,
} from "./write"
