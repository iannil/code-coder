/**
 * HTTP API Server CLI Command
 *
 * Redirects to the Rust unified API server (zero-cli serve).
 * The TypeScript API server has been deprecated in favor of Rust.
 */

import type { Argv } from "yargs"
import type { CommandModule } from "yargs"
import { cmd } from "./cmd"
import { withNetworkOptions, type NetworkOptions } from "../network"
import { UI } from "../ui"

const serveCommandImpl: CommandModule<NetworkOptions, any> = {
  command: "serve",
  describe: "Start HTTP API server (uses Rust backend)",
  builder: (yargs: Argv<NetworkOptions>) =>
    withNetworkOptions(yargs).option("api-key", {
      type: "string",
      describe: "API key for authentication",
      alias: "k",
    }),
  handler: async (args) => {
    UI.println(UI.Style.TEXT_WARNING_BOLD + "Note:", UI.Style.TEXT_NORMAL, "The TypeScript API server has been deprecated.")
    UI.println("Please use the Rust unified API server instead:")
    UI.println()
    UI.println(UI.Style.TEXT_HIGHLIGHT_BOLD + "  zero-cli serve" + UI.Style.TEXT_NORMAL)
    UI.println()
    if (args.port) {
      UI.println(`  zero-cli serve --port ${args.port}`)
    }
    if (args.hostname) {
      UI.println(`  zero-cli serve --host ${args.hostname}`)
    }
    UI.println()
    process.exit(1)
  },
}

export const ServeCommand = cmd(serveCommandImpl)
