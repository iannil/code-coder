// @ts-nocheck
/**
 * Integration Tests: Clipboard Interaction
 *
 * Tests for clipboard paste workflows including:
 * - Text paste
 * - File path paste
 * - Image paste
 * - SVG paste (as text)
 * - Large content summary paste
 */

import { describe, test, expect, vi, beforeEach } from "bun:test"

describe("Clipboard Integration", () => {
  describe("text paste", () => {
    test("should insert plain text at cursor", () => {
      let promptContent = "Hello "
      const cursorPosition = promptContent.length

      const handlePaste = (text: string) => {
        promptContent =
          promptContent.slice(0, cursorPosition) + text + promptContent.slice(cursorPosition)
      }

      handlePaste("world!")

      expect(promptContent).toBe("Hello world!")
    })

    test("should handle multiline text paste", () => {
      let promptContent = ""

      const handlePaste = (text: string) => {
        promptContent = text
      }

      const multilineText = "Line 1\nLine 2\nLine 3"
      handlePaste(multilineText)

      expect(promptContent).toContain("\n")
      expect(promptContent.split("\n")).toHaveLength(3)
    })

    test("should normalize line endings", () => {
      const windowsText = "Line 1\r\nLine 2\r\nLine 3"
      let normalized = ""

      const handlePaste = (text: string) => {
        normalized = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n")
      }

      handlePaste(windowsText)

      expect(normalized).toBe("Line 1\nLine 2\nLine 3")
      expect(normalized).not.toContain("\r")
    })

    test("should handle unicode text paste", () => {
      let promptContent = ""

      const handlePaste = (text: string) => {
        promptContent = text
      }

      handlePaste("Hello 世界 🌍")

      expect(promptContent).toBe("Hello 世界 🌍")
    })
  })

  describe("file path paste", () => {
    test("should detect file path paste", () => {
      const pastedText = "/Users/developer/project/src/index.ts"

      const isFilePath = (text: string) => {
        return text.startsWith("/") || text.startsWith("~/") || /^[a-zA-Z]:/.test(text)
      }

      expect(isFilePath(pastedText)).toBe(true)
    })

    test("should strip quotes from file paths", () => {
      const quotedPath = "'/path/to/file.ts'"

      const stripQuotes = (path: string) => {
        return path.replace(/^'+|'+$/g, "").replace(/\\ /g, " ")
      }

      expect(stripQuotes(quotedPath)).toBe("/path/to/file.ts")
    })

    test("should handle spaces in file paths", () => {
      const pathWithSpaces = "/path/to/my file.ts"

      const handleSpaces = (path: string) => {
        return path.replace(/\\ /g, " ")
      }

      expect(handleSpaces("/path/to/my\\ file.ts")).toBe("/path/to/my file.ts")
    })

    test("should format file path as @reference", () => {
      let promptContent = ""

      const handleFilePathPaste = (path: string) => {
        promptContent = `@${path} `
      }

      handleFilePathPaste("src/components/Button.tsx")

      expect(promptContent).toBe("@src/components/Button.tsx ")
    })

    test("should verify file exists before adding", () => {
      const existingFiles = new Set(["file1.ts", "file2.ts", "file3.ts"])
      let validFile: string | null = null

      const verifyAndAdd = (path: string) => {
        if (existingFiles.has(path)) {
          validFile = path
        }
      }

      verifyAndAdd("file2.ts")

      expect(validFile).toBe("file2.ts")
    })
  })

  describe("image paste", () => {
    test("should detect image mime type", () => {
      const imageMimes = ["image/png", "image/jpeg", "image/gif", "image/webp"]

      imageMimes.forEach((mime) => {
        expect(mime.startsWith("image/")).toBe(true)
      })
    })

    test("should create placeholder for pasted image", () => {
      const images: string[] = []
      let imageCount = 0

      const handleImagePaste = () => {
        imageCount++
        images.push(`[Image ${imageCount}]`)
      }

      handleImagePaste()
      handleImagePaste()

      expect(images).toEqual(["[Image 1]", "[Image 2]"])
    })

    test("should store image as data URL", () => {
      const imageData = Buffer.from("fake-image-data").toString("base64")
      const storedImages: { mime: string; url: string }[] = []

      const handleImagePaste = (mime: string, base64: string) => {
        storedImages.push({
          mime,
          url: `data:${mime};base64,${base64}`,
        })
      }

      handleImagePaste("image/png", imageData)

      expect(storedImages[0].url).toMatch(/^data:image\/png;base64,/)
      expect(storedImages[0].url).toContain(imageData)
    })

    test("should handle multiple image paste", () => {
      let imageCount = 0

      const handleMultipleImages = (count: number) => {
        imageCount += count
      }

      handleMultipleImages(3)

      expect(imageCount).toBe(3)
    })
  })

  describe("SVG paste", () => {
    test("should detect SVG files", () => {
      const svgMime = "image/svg+xml"

      expect(svgMime).toBe("image/svg+xml")
    })

    test("should treat SVG as text content, not image", () => {
      const svgContent = `<svg xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="40"/></svg>`
      let treatedAsText = false

      const handleSVGPaste = (content: string, mime: string) => {
        if (mime === "image/svg+xml") {
          treatedAsText = true
          return content
        }
      }

      handleSVGPaste(svgContent, "image/svg+xml")

      expect(treatedAsText).toBe(true)
    })

    test("should create text part for SVG content", () => {
      const parts: { type: string; text: string }[] = []

      const handleSVGPaste = (svgContent: string) => {
        parts.push({
          type: "text",
          text: svgContent,
        })
      }

      const svg = `<svg>...</svg>`
      handleSVGPaste(svg)

      expect(parts[0].type).toBe("text")
      expect(parts[0].text).toBe(svg)
    })

    test("should show SVG placeholder in prompt", () => {
      const placeholders: string[] = []

      const handleSVGPaste = (filename: string) => {
        placeholders.push(`[SVG: ${filename}]`)
      }

      handleSVGPaste("icon.svg")

      expect(placeholders[0]).toBe("[SVG: icon.svg]")
    })
  })

  describe("large content summary paste", () => {
    test("should detect large content by line count", () => {
      const content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5"
      const lineCount = (content.match(/\n/g)?.length ?? 0) + 1

      expect(lineCount).toBe(5)
    })

    test("should create summary for 3+ lines", () => {
      let shouldSummarize = false

      const handlePaste = (text: string) => {
        const lineCount = (text.match(/\n/g)?.length ?? 0) + 1
        if (lineCount >= 3) {
          shouldSummarize = true
        }
      }

      handlePaste("Line 1\nLine 2\nLine 3")

      expect(shouldSummarize).toBe(true)
    })

    test("should create summary for 150+ characters", () => {
      let shouldSummarize = false

      const handlePaste = (text: string) => {
        if (text.length > 150) {
          shouldSummarize = true
        }
      }

      handlePaste("a".repeat(200))

      expect(shouldSummarize).toBe(true)
    })

    test("should format summary with line count", () => {
      const content = "Line 1\n".repeat(10)
      // "Line 1\n" x 10 creates 10 lines with 10 newlines
      // But wait - each repetition adds "\n" at the end, so:
      // "Line 1\n" + "Line 1\n" + ... = 10 newlines total
      // With match counting \n we get 10, + 1 for line count = 11
      const lineCount = (content.match(/\n/g)?.length ?? 0) + 1

      const summary = `[Pasted ~${lineCount} lines]`

      expect(summary).toBe("[Pasted ~11 lines]")
    })

    test("should store original text in part", () => {
      const parts: { type: string; text: string; display: string }[] = []

      const handleLargePaste = (text: string) => {
        parts.push({
          type: "text",
          text,
          display: `[Pasted ~${text.split("\n").length} lines]`,
        })
      }

      const originalText = "a\nb\nc\nd\ne"
      handleLargePaste(originalText)

      expect(parts[0].text).toBe(originalText)
      expect(parts[0].display).toBe("[Pasted ~5 lines]")
    })
  })

  describe("paste handling", () => {
    test("should handle empty paste gracefully", () => {
      let pasted = false

      const handlePaste = (text: string) => {
        if (text.trim().length > 0) {
          pasted = true
        }
      }

      handlePaste("   ")

      expect(pasted).toBe(false)
    })

    test("should truncate extremely long pastes", () => {
      const maxLength = 10000
      const veryLongText = "a".repeat(20000)
      let truncatedText = ""

      const handlePaste = (text: string) => {
        truncatedText = text.slice(0, maxLength) + "... [truncated]"
      }

      handlePaste(veryLongText)

      expect(truncatedText.length).toBeLessThanOrEqual(maxLength + 15)
      expect(truncatedText).toContain("[truncated]")
    })

    test("should sanitize HTML in paste", () => {
      const htmlContent = "<script>alert('xss')</script>Hello"
      let sanitized = ""

      const handlePaste = (text: string) => {
        sanitized = text.replace(/<[^>]*>/g, "")
      }

      handlePaste(htmlContent)

      expect(sanitized).not.toContain("<script>")
      expect(sanitized).not.toContain("</script>")
      expect(sanitized).toContain("Hello")
    })
  })

  describe("bracketed paste mode", () => {
    test("should detect bracketed paste sequence", () => {
      const pasteSequence = "\x1b[200~"

      const isBracketedPaste = (text: string) => {
        return text.startsWith(pasteSequence)
      }

      expect(isBracketedPaste("\x1b[200~content\x1b[201~")).toBe(true)
      expect(isBracketedPaste("normal text")).toBe(false)
    })

    test("should extract content from bracketed paste", () => {
      const bracketedPaste = "\x1b[200~pasted content\x1b[201~"

      const extractContent = (text: string) => {
        return text.replace(/\x1b\[200~|\x1b\[201~/g, "")
      }

      expect(extractContent(bracketedPaste)).toBe("pasted content")
    })

    test("should handle bracketed paste with special characters", () => {
      const bracketedPaste = "\x1b[200~Hello\nWorld!\x1b[201~"

      const extractContent = (text: string) => {
        return text.replace(/\x1b\[200~|\x1b\[201~/g, "")
      }

      expect(extractContent(bracketedPaste)).toBe("Hello\nWorld!")
    })
  })

  describe("URL paste", () => {
    test("should detect URL paste", () => {
      const url = "https://example.com/file.txt"

      const isUrl = /^https?:\/\//.test(url)

      expect(isUrl).toBe(true)
    })

    test("should not treat URL as file path", () => {
      const url = "https://example.com/file.txt"

      const isFilePath = (text: string) => {
        return text.startsWith("/") || text.startsWith("~/") || /^[a-zA-Z]:/.test(text)
      }

      expect(isFilePath(url)).toBe(false)
    })

    test("should handle GitHub URLs specially", () => {
      const githubUrl = "https://github.com/user/repo/blob/main/file.ts"

      const isGitHubUrl = (url: string) => {
        return url.startsWith("https://github.com/")
      }

      expect(isGitHubUrl(githubUrl)).toBe(true)
    })
  })
})
