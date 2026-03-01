/**
 * Document Command
 *
 * Main entry point for document commands. Imports from modular subcommands.
 */
import { cmd } from "./cmd"

// Import all commands from the document directory
import {
  // Core document commands
  DocumentCreateCommand,
  DocumentListCommand,
  DocumentShowCommand,
  DocumentExportCommand,
  DocumentDeleteCommand,
  DocumentUpdateCommand,
  DocumentSetContentCommand,
  // Template commands
  DocumentTemplateCommand,
  // Outline commands
  DocumentOutlineCommand,
  // Write commands
  DocumentWriteCommand,
  DocumentWriteAllCommand,
  // Context commands
  ContextCommand,
  SummaryGlobalCommand,
  // Chapter commands
  ChapterCommand,
  // Entity commands
  EntityCommand,
  // Volume commands
  VolumeCommand,
  // Snapshot commands
  SnapshotCommand,
  // Check commands
  CheckCommand,
  CheckStyleCommand,
  // Edit commands
  SearchReplaceCommand,
  PolishCommand,
  ExpandCommand,
  CompressCommand,
  // Proofread commands
  ProofreadCommand,
} from "./document/index"

// ============================================================================
// Main Document Command
// ============================================================================

export const DocumentCommand = cmd({
  command: "document",
  describe: "manage long-form documents",
  builder: (yargs) =>
    yargs
      .command(DocumentCreateCommand)
      .command(DocumentTemplateCommand)
      .command(DocumentOutlineCommand)
      .command(DocumentWriteCommand)
      .command(DocumentWriteAllCommand)
      .command(DocumentListCommand)
      .command(DocumentShowCommand)
      .command(DocumentExportCommand)
      .command(DocumentSetContentCommand)
      .command(DocumentDeleteCommand)
      .command(DocumentUpdateCommand)
      // New commands for long document support
      .command(ContextCommand)
      .command(SummaryGlobalCommand)
      .demandCommand(),
  async handler() {},
})

// Re-export command groups for use elsewhere
export { ChapterCommand } from "./document/index"
export { EntityCommand, VolumeCommand, SnapshotCommand } from "./document/index"
export { CheckCommand, CheckStyleCommand } from "./document/index"
export { SearchReplaceCommand, PolishCommand, ExpandCommand, CompressCommand } from "./document/index"
export { ProofreadCommand } from "./document/index"
