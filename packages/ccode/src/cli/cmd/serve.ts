/**
 * HTTP API Server CLI Command
 * Starts the CodeCoder HTTP API server
 */

import type { Argv } from "yargs"
import type { CommandModule } from "yargs"
import { cmd } from "./cmd"
import { withNetworkOptions, type NetworkOptions } from "../network"
import { bootstrap } from "../bootstrap"

const serveCommandImpl: CommandModule<NetworkOptions, any> = {
  command: "serve",
  describe: "Start HTTP API server",
  builder: (yargs: Argv<NetworkOptions>) =>
    withNetworkOptions(yargs)
      .option("api-key", {
        type: "string",
        describe: "API key for authentication",
        alias: "k",
      }),
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const { start } = await import("../../api/server")

      await start({
        port: args.port,
        hostname: args.hostname,
        cors: args.cors as string | string[],
        apiKey: args["api-key"],
      })

      // Keep process alive
      await new Promise(() => {})
    })
  },
}

export const ServeCommand = cmd(serveCommandImpl)
