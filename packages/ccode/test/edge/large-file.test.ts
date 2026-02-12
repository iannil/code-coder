/**
 * Edge Case Test: Large File Handling
 *
 * Tests file size limits, truncation behavior, line limits,
 * memory management, and binary file detection.
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { ReadTool } from "../../src/tool/read"

const ctx = {
  sessionID: "test",
  messageID: "",
  callID: "",
  agent: "build",
  abort: AbortSignal.any([]),
  metadata: () => {},
  ask: async () => {},
}

describe("Large File Handling", () => {
  describe("File Size Truncation", () => {
    test("should truncate file over 50KB limit", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create a file larger than 50KB with multiple lines
          const line = "x".repeat(100) // 100 bytes per line
          const lines = Array.from({ length: 600 }, () => line).join("\n") // 60KB total
          await Bun.write(path.join(dir, "large.txt"), lines)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "large.txt") }, ctx)

          expect(result.metadata.truncated).toBe(true)
          expect(result.output).toContain("Output truncated at")
          expect(result.output).toContain("bytes")
        },
      })
    })

    test("should not truncate small file under limit", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "small.txt"), "Hello, World!")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "small.txt") }, ctx)

          expect(result.metadata.truncated).toBe(false)
          expect(result.output).toContain("Hello, World!")
          expect(result.output).toContain("End of file")
        },
      })
    })

    test("should indicate remaining bytes when truncated", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const line = "a".repeat(100) // 100 bytes per line
          const lines = Array.from({ length: 1200 }, () => line).join("\n") // 120KB total
          await Bun.write(path.join(dir, "verylarge.txt"), lines)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "verylarge.txt") }, ctx)

          expect(result.metadata.truncated).toBe(true)
          // Should mention how many bytes were truncated
          expect(result.output).toContain("truncated")
        },
      })
    })
  })

  describe("Line Count Handling", () => {
    test("should handle file with > 2000 lines", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create file with 2500 lines
          const lines = Array.from({ length: 2500 }, (_, i) => `Line ${i + 1}: content`).join("\n")
          await Bun.write(path.join(dir, "manylines.txt"), lines)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "manylines.txt") }, ctx)

          // Default behavior reads up to 2000 lines
          expect(result.output).toContain("Line 1")
          expect(result.metadata.truncated).toBe(true)
        },
      })
    })

    test("should respect line limit parameter", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const lines = Array.from({ length: 100 }, (_, i) => `Line ${i + 1}`).join("\n")
          await Bun.write(path.join(dir, "lines.txt"), lines)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "lines.txt"), limit: 10 }, ctx)

          expect(result.metadata.truncated).toBe(true)
          expect(result.output).toContain("Line 1")
          expect(result.output).toContain("Line 10")
          expect(result.output).not.toContain("Line 11")
          expect(result.output).toContain("File has more lines")
        },
      })
    })

    test("should support offset parameter", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const lines = Array.from({ length: 50 }, (_, i) => `Line ${i + 1}`).join("\n")
          await Bun.write(path.join(dir, "offset.txt"), lines)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "offset.txt"), offset: 20, limit: 10 }, ctx)

          expect(result.output).toContain("Line 21")
          expect(result.output).toContain("Line 30")
          expect(result.output).not.toContain("Line 1:")
          expect(result.output).not.toContain("Line 31")
        },
      })
    })
  })

  describe("Long Line Handling", () => {
    test("should truncate lines over 2000 characters", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const longLine = "x".repeat(3000)
          await Bun.write(path.join(dir, "longline.txt"), longLine)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "longline.txt") }, ctx)

          // Lines should be truncated with ellipsis
          expect(result.output).toContain("...")
          // Output should be shorter than original
          const outputLength = result.output.length
          expect(outputLength).toBeLessThan(3000)
        },
      })
    })

    test("should handle mixed long and short lines", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const content = [
            "Short line 1",
            "x".repeat(3000), // Long line
            "Short line 2",
            "y".repeat(2500), // Another long line
            "Short line 3",
          ].join("\n")
          await Bun.write(path.join(dir, "mixed.txt"), content)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "mixed.txt") }, ctx)

          expect(result.output).toContain("Short line 1")
          expect(result.output).toContain("Short line 2")
          expect(result.output).toContain("Short line 3")
          // Long lines should be truncated
          expect(result.output).toContain("...")
        },
      })
    })
  })

  describe("Memory Efficiency", () => {
    test("should process 10MB file without excessive memory", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create a 10MB file
          const chunk = "x".repeat(1024 * 100) + "\n" // 100KB per line
          const content = Array.from({ length: 100 }, () => chunk).join("")
          await Bun.write(path.join(dir, "10mb.txt"), content)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Force garbage collection if available
          if (typeof Bun !== "undefined") {
            Bun.gc(true)
          }

          const heapBefore = process.memoryUsage().heapUsed

          const read = await ReadTool.init()
          await read.execute({ filePath: path.join(tmp.path, "10mb.txt") }, ctx)

          if (typeof Bun !== "undefined") {
            Bun.gc(true)
          }

          const heapAfter = process.memoryUsage().heapUsed
          const heapDelta = heapAfter - heapBefore

          // Memory increase should be reasonable (not loading entire 10MB)
          // Truncation should limit memory usage
          expect(heapDelta).toBeLessThan(20 * 1024 * 1024) // Less than 20MB increase
        },
      })
    })

    test("should not leak memory on repeated reads", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const content = "Test content " + "x".repeat(10000)
          await Bun.write(path.join(dir, "repeat.txt"), content)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          if (typeof Bun !== "undefined") {
            Bun.gc(true)
          }

          const initialHeap = process.memoryUsage().heapUsed

          const read = await ReadTool.init()

          // Read file multiple times
          for (let i = 0; i < 10; i++) {
            await read.execute({ filePath: path.join(tmp.path, "repeat.txt") }, ctx)
          }

          if (typeof Bun !== "undefined") {
            Bun.gc(true)
          }

          const finalHeap = process.memoryUsage().heapUsed
          const heapGrowth = finalHeap - initialHeap

          // Memory growth should be bounded
          expect(heapGrowth).toBeLessThan(10 * 1024 * 1024) // Less than 10MB growth
        },
      })
    })
  })

  describe("Binary File Detection", () => {
    test("should detect PNG image as binary", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // 1x1 red PNG
          const png = Buffer.from(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==",
            "base64",
          )
          await Bun.write(path.join(dir, "image.png"), png)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "image.png") }, ctx)

          // Should be handled as image, not truncated text
          expect(result.metadata.truncated).toBe(false)
          expect(result.attachments).toBeDefined()
          expect(result.attachments?.length).toBe(1)
        },
      })
    })

    test("should detect JPEG image", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Minimal JPEG header
          const jpeg = Buffer.from([
            0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01,
            0x00, 0x00, 0xff, 0xd9,
          ])
          await Bun.write(path.join(dir, "image.jpg"), jpeg)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "image.jpg") }, ctx)

          expect(result.attachments).toBeDefined()
        },
      })
    })

    test("should not treat text files as binary", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "text.txt"), "Hello, this is plain text content.")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "text.txt") }, ctx)

          // Should be read as text, not as attachment
          expect(result.output).toContain("Hello, this is plain text content.")
          expect(result.attachments).toBeUndefined()
        },
      })
    })

    test("should read .fbs (FlatBuffers schema) as text", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const fbsContent = `namespace MyGame;

table Monster {
  pos:Vec3;
  name:string;
}

root_type Monster;`
          await Bun.write(path.join(dir, "schema.fbs"), fbsContent)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "schema.fbs") }, ctx)

          // Should be read as text
          expect(result.attachments).toBeUndefined()
          expect(result.output).toContain("namespace MyGame")
          expect(result.output).toContain("table Monster")
        },
      })
    })
  })

  describe("Special File Types", () => {
    test("should handle empty file", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "empty.txt"), "")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "empty.txt") }, ctx)

          expect(result.metadata.truncated).toBe(false)
          // Should handle empty file gracefully
          expect(result.output).toBeDefined()
        },
      })
    })

    test("should handle file with only whitespace", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "whitespace.txt"), "   \n\n\t\t  \n")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "whitespace.txt") }, ctx)

          expect(result.metadata.truncated).toBe(false)
        },
      })
    })

    test("should handle file with unicode characters", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const unicodeContent = "Hello \u4e16\u754c \ud83c\udf0d \u0e2a\u0e27\u0e31\u0e2a\u0e14\u0e35 \u3053\u3093\u306b\u3061\u306f"
          await Bun.write(path.join(dir, "unicode.txt"), unicodeContent)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "unicode.txt") }, ctx)

          expect(result.output).toContain("Hello")
          expect(result.output).toContain("\u4e16\u754c")
        },
      })
    })

    test("should handle file with mixed line endings", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const content = "Line1\r\nLine2\nLine3\rLine4"
          await Bun.write(path.join(dir, "mixedlines.txt"), content)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "mixedlines.txt") }, ctx)

          expect(result.output).toContain("Line1")
          expect(result.output).toContain("Line2")
        },
      })
    })
  })
})
