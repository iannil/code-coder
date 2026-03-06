import z from "zod"
import { spawn } from "child_process"
import { Tool } from "./tool"
import path from "path"
import DESCRIPTION from "./bash.txt"
import { Log } from "@/util/log"
import { Instance } from "../project/instance"

import { $ } from "bun"
import { Flag } from "@/flag/flag.ts"
import { Shell } from "@/shell/shell"

import { BashArity } from "@/permission/arity"
import { Truncate } from "./truncation"
import {
  point,
  apiCallPoint as apiCall,
  runWithChildSpanAsync,
  functionStart,
  functionEnd,
} from "@/observability"

// Import native shell parser from @codecoder-ai/core
import { parseShellCommand as nativeParseShellCommand } from "@codecoder-ai/core"
import type { NapiShellParseResult } from "@codecoder-ai/core"

// Module-level validation: fail fast if native bindings are not available
if (!nativeParseShellCommand) {
  throw new Error(
    "Native shell parser bindings not available. Ensure @codecoder-ai/core is built with 'bun run build' in packages/core",
  )
}

// Store validated reference for TypeScript type narrowing
const parseShellCommand = nativeParseShellCommand

const MAX_METADATA_LENGTH = 30_000
const DEFAULT_TIMEOUT = Flag.CCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS || 2 * 60 * 1000

export const log = Log.create({ service: "bash-tool" })

// Use native tree-sitter based shell parser (fail-fast mode)
const parseCommand = (command: string): NapiShellParseResult => {
  return parseShellCommand(command)
}

// File-manipulating commands that need directory resolution
const FILE_COMMANDS = ["cd", "rm", "cp", "mv", "mkdir", "touch", "chmod", "chown", "cat", "rmdir", "ln"]

// NOTE: Tool is named "bash" for historical reasons but supports multiple shells.
// Shell detection is handled by Shell.acceptable() which returns the user's preferred shell.
// Renaming to "shell" or "terminal" would be a breaking change for existing prompts and tools.
// Cross-shell compatibility is maintained through the Shell module.
export const BashTool = Tool.define("bash", async () => {
  const shell = Shell.acceptable()
  log.info("bash tool using shell", { shell, nativeParser: true })

  return {
    description: DESCRIPTION.replaceAll("${directory}", Instance.directory)
      .replaceAll("${maxLines}", String(Truncate.MAX_LINES))
      .replaceAll("${maxBytes}", String(Truncate.MAX_BYTES)),
    parameters: z.object({
      command: z.string().describe("The command to execute"),
      timeout: z.number().describe("Optional timeout in milliseconds").optional(),
      workdir: z
        .string()
        .describe(
          `The working directory to run the command in. Defaults to ${Instance.directory}. Use this instead of 'cd' commands.`,
        )
        .optional(),
      description: z
        .string()
        .describe(
          "Clear, concise description of what this command does in 5-10 words. Examples:\nInput: ls\nOutput: Lists files in current directory\n\nInput: git status\nOutput: Shows working tree status\n\nInput: npm install\nOutput: Installs package dependencies\n\nInput: mkdir foo\nOutput: Creates directory 'foo'",
        ),
    }),
    async execute(params, ctx) {
      // Wrap tool execution in a child span for tracing
      return runWithChildSpanAsync(async () => {
        const startTime = Date.now()
        functionStart("BashTool.execute", {
          command: params.command.slice(0, 100),
          description: params.description,
        })

        const bashCall = apiCall("BashTool.execute", {
          command: params.command.slice(0, 100),
          description: params.description,
        })
        point("bash_execute_start", { command: params.command.slice(0, 200) })

        const cwd = params.workdir || Instance.directory
        if (params.timeout !== undefined && params.timeout < 0) {
          throw new Error(`Invalid timeout value: ${params.timeout}. Timeout must be a positive number.`)
        }
        const timeout = params.timeout ?? DEFAULT_TIMEOUT

        // Parse command using native tree-sitter parser (or fallback)
        const parseResult = parseCommand(params.command)
        if (!parseResult.success) {
          throw new Error(parseResult.error ?? "Failed to parse command")
        }

        const directories = new Set<string>()
        if (!Instance.containsPath(cwd)) directories.add(cwd)
        const patterns = new Set<string>()
        const always = new Set<string>()

        // Process each parsed command
        for (const cmd of parseResult.commands) {
          // Check for file-manipulating commands that need directory resolution
          if (FILE_COMMANDS.includes(cmd.name)) {
            for (const arg of cmd.args) {
              // Skip flags
              if (arg.startsWith("-") || (cmd.name === "chmod" && arg.startsWith("+"))) continue

              const resolved = await $`realpath ${arg}`
                .cwd(cwd)
                .quiet()
                .nothrow()
                .text()
                .then((x) => x.trim())

              log.info("resolved path", { arg, resolved })
              if (resolved) {
                // Git Bash on Windows returns Unix-style paths like /c/Users/...
                const normalized =
                  process.platform === "win32" && resolved.match(/^\/[a-z]\//)
                    ? resolved.replace(/^\/([a-z])\//, (_, drive: string) => `${drive.toUpperCase()}:\\`).replace(/\//g, "\\")
                    : resolved
                if (!Instance.containsPath(normalized)) directories.add(normalized)
              }
            }
          }

          // Build permission patterns (cd is covered by directory check above)
          if (cmd.name !== "cd") {
            const fullCommand = cmd.args.length > 0 ? `${cmd.name} ${cmd.args.join(" ")}` : cmd.name
            patterns.add(fullCommand)
            always.add(BashArity.prefix([cmd.name, ...cmd.args]).join(" ") + "*")
          }
        }

        if (directories.size > 0) {
          await ctx.ask({
            permission: "external_directory",
            patterns: Array.from(directories),
            always: Array.from(directories).map((x) => path.dirname(x) + "*"),
            metadata: {},
          })
        }

        if (patterns.size > 0) {
          await ctx.ask({
            permission: "bash",
            patterns: Array.from(patterns),
            always: Array.from(always),
            metadata: {},
          })
        }

        const proc = spawn(params.command, {
          shell,
          cwd,
          env: {
            ...process.env,
          },
          stdio: ["ignore", "pipe", "pipe"],
          detached: process.platform !== "win32",
        })

        let output = ""

        // Initialize metadata with empty output
        ctx.metadata({
          metadata: {
            output: "",
            description: params.description,
          },
        })

        const append = (chunk: Buffer) => {
          output += chunk.toString()
          ctx.metadata({
            metadata: {
              // truncate the metadata to avoid GIANT blobs of data (has nothing to do w/ what agent can access)
              output: output.length > MAX_METADATA_LENGTH ? output.slice(0, MAX_METADATA_LENGTH) + "\n\n..." : output,
              description: params.description,
            },
          })
        }

        proc.stdout?.on("data", append)
        proc.stderr?.on("data", append)

        let timedOut = false
        let aborted = false
        let exited = false

        const kill = () => Shell.killTree(proc, { exited: () => exited })

        if (ctx.abort.aborted) {
          aborted = true
          await kill()
        }

        const abortHandler = () => {
          aborted = true
          void kill()
        }

        ctx.abort.addEventListener("abort", abortHandler, { once: true })

        const timeoutTimer = setTimeout(() => {
          timedOut = true
          void kill()
        }, timeout + 100)

        await new Promise<void>((resolve, reject) => {
          const cleanup = () => {
            clearTimeout(timeoutTimer)
            ctx.abort.removeEventListener("abort", abortHandler)
          }

          proc.once("exit", () => {
            exited = true
            cleanup()
            resolve()
          })

          proc.once("error", (error) => {
            exited = true
            cleanup()
            reject(error)
          })
        })

        const resultMetadata: string[] = []

        if (timedOut) {
          resultMetadata.push(`bash tool terminated command after exceeding timeout ${timeout} ms`)
        }

        if (aborted) {
          resultMetadata.push("User aborted the command")
        }

        if (resultMetadata.length > 0) {
          output += "\n\n<bash_metadata>\n" + resultMetadata.join("\n") + "\n</bash_metadata>"
        }

        bashCall.end({ exitCode: proc.exitCode, outputLength: output.length })
        point("bash_execute_end", {
          exitCode: proc.exitCode,
          timedOut,
          aborted,
          outputLength: output.length,
        })

        functionEnd("BashTool.execute", {
          exitCode: proc.exitCode,
          timedOut,
          aborted,
        }, Date.now() - startTime)

        return {
          title: params.description,
          metadata: {
            output: output.length > MAX_METADATA_LENGTH ? output.slice(0, MAX_METADATA_LENGTH) + "\n\n..." : output,
            exit: proc.exitCode,
            description: params.description,
          },
          output,
        }
      }) // end runWithChildSpanAsync
    },
  }
})
