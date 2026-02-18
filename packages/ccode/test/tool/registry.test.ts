import { describe, expect, test } from "bun:test"
import path from "path"
import fs from "fs/promises"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { ToolRegistry } from "../../src/tool/registry"

describe("tool.registry", () => {
  test("loads tools from .codecoder/tool (singular)", async () => {
    await using tmp = await tmpdir({
      init: async (dir) => {
        const ccodeDir = path.join(dir, ".codecoder")
        await fs.mkdir(ccodeDir, { recursive: true })

        const toolDir = path.join(ccodeDir, "tool")
        await fs.mkdir(toolDir, { recursive: true })

        await Bun.write(
          path.join(toolDir, "hello.ts"),
          [
            "const helloTool = {",
            "  id: 'hello',",
            "  description: 'hello tool',",
            "  parameters: {},",
            "  execute: async () => {",
            "    return { title: 'hello', output: 'hello world' }",
            "  },",
            "}",
            "",
            "export default async function init() {",
            "  return helloTool",
            "}",
            "",
            "Object.assign(init, { tool: helloTool })",
            "",
          ].join("\n"),
        )
      },
    })

    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const ids = await ToolRegistry.ids()
        expect(ids).toContain("hello")
      },
    })
  })

  test("loads tools from .codecoder/tools (plural)", async () => {
    await using tmp = await tmpdir({
      init: async (dir) => {
        const ccodeDir = path.join(dir, ".codecoder")
        await fs.mkdir(ccodeDir, { recursive: true })

        const toolsDir = path.join(ccodeDir, "tools")
        await fs.mkdir(toolsDir, { recursive: true })

        await Bun.write(
          path.join(toolsDir, "hello.ts"),
          [
            "const helloTool = {",
            "  id: 'hello',",
            "  description: 'hello tool',",
            "  parameters: {},",
            "  execute: async () => {",
            "    return { title: 'hello', output: 'hello world' }",
            "  },",
            "}",
            "",
            "export default async function init() {",
            "  return helloTool",
            "}",
            "",
            "Object.assign(init, { tool: helloTool })",
            "",
          ].join("\n"),
        )
      },
    })

    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const ids = await ToolRegistry.ids()
        expect(ids).toContain("hello")
      },
    })
  })
})
