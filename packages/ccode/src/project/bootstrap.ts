import { Format } from "@/util/format"
import { LSP } from "../lsp"
import { FileWatcher } from "../file/watcher"
import { File } from "@/file"
import { Project } from "./project"
import { Instance } from "./instance"
import { Vcs } from "./vcs"
import { Log } from "@/util/log"

export async function InstanceBootstrap() {
  Log.Default.info("bootstrapping", { directory: Instance.directory })
  Format.init()
  await LSP.init()
  FileWatcher.init()
  File.init()
  Vcs.init()

  // Note: Snapshot.init() and Command.Event subscription removed
  // Snapshot cleanup is handled by Rust daemon
  // Command.Event.Executed tracking moved to Rust
}
