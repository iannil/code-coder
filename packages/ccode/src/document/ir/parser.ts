/**
 * Document Parsers
 *
 * Parses various document formats into the Document IR.
 * Supports Markdown, HTML, and code files.
 *
 * ## Design Principle
 *
 * Parsing is a **deterministic** task, but extracting semantic
 * meaning from complex documents benefits from heuristics.
 * This module provides fast, regex-based parsing for common cases.
 */

import {
  type Document,
  type BlockNode,
  type InlineNode,
  type DocumentMetadata,
  type ListItemNode,
  type TableRowNode,
  type TableCellNode,
  text,
  code,
  heading,
  paragraph,
  list,
  listItem,
  link,
  image,
  table,
  tableRow,
  tableCell,
  blockquote,
  horizontalRule,
  createDocument,
} from "./types"

// ============================================================================
// Markdown Parser
// ============================================================================

/**
 * Parse Markdown content into Document IR
 */
export function parseMarkdown(content: string): Document {
  const lines = content.split("\n")
  const nodes: BlockNode[] = []
  const metadata: DocumentMetadata = { sourceFormat: "markdown" }

  let i = 0

  // Check for YAML frontmatter
  if (lines[0] === "---") {
    const endIndex = lines.indexOf("---", 1)
    if (endIndex > 0) {
      const frontmatter = lines.slice(1, endIndex).join("\n")
      const parsed = parseYamlFrontmatter(frontmatter)
      Object.assign(metadata, parsed)
      i = endIndex + 1
    }
  }

  while (i < lines.length) {
    const line = lines[i]

    // Skip empty lines
    if (line.trim() === "") {
      i++
      continue
    }

    // Heading (ATX style)
    const headingMatch = line.match(/^(#{1,6})\s+(.+)$/)
    if (headingMatch) {
      const level = headingMatch[1].length as 1 | 2 | 3 | 4 | 5 | 6
      const headingContent = headingMatch[2].replace(/\s*#+\s*$/, "") // Remove trailing #
      const anchor = slugify(headingContent)
      nodes.push(heading(level, headingContent, anchor))
      i++
      continue
    }

    // Horizontal rule
    if (/^(?:[-*_]){3,}\s*$/.test(line)) {
      nodes.push(horizontalRule())
      i++
      continue
    }

    // Code block (fenced)
    const codeMatch = line.match(/^```(\w*)(.*)$/)
    if (codeMatch) {
      const language = codeMatch[1] || undefined
      const filename = codeMatch[2].trim() || undefined
      const codeLines: string[] = []
      i++
      while (i < lines.length && !lines[i].startsWith("```")) {
        codeLines.push(lines[i])
        i++
      }
      nodes.push(code(codeLines.join("\n"), language, filename))
      i++ // Skip closing ```
      continue
    }

    // Blockquote
    if (line.startsWith(">")) {
      const quoteLines: string[] = []
      while (i < lines.length && lines[i].startsWith(">")) {
        quoteLines.push(lines[i].replace(/^>\s?/, ""))
        i++
      }
      nodes.push(blockquote(quoteLines.join("\n")))
      continue
    }

    // Unordered list
    if (/^[-*+]\s/.test(line)) {
      const items: ListItemNode[] = []
      while (i < lines.length && /^[-*+]\s/.test(lines[i])) {
        const itemLine = lines[i].replace(/^[-*+]\s/, "")
        // Check for task list item
        const taskMatch = itemLine.match(/^\[([ xX])\]\s(.*)$/)
        if (taskMatch) {
          const checked = taskMatch[1].toLowerCase() === "x"
          const itemContent = taskMatch[2]
          items.push(listItem(itemContent, checked))
        } else {
          items.push(listItem(itemLine))
        }
        i++
      }
      nodes.push(list(items, false))
      continue
    }

    // Ordered list
    if (/^\d+\.\s/.test(line)) {
      const items: ListItemNode[] = []
      const startMatch = line.match(/^(\d+)\.\s/)
      const start = startMatch ? parseInt(startMatch[1], 10) : 1
      while (i < lines.length && /^\d+\.\s/.test(lines[i])) {
        const itemLine = lines[i].replace(/^\d+\.\s/, "")
        items.push(listItem(itemLine))
        i++
      }
      nodes.push(list(items, true, start))
      continue
    }

    // Table
    if (line.includes("|") && i + 1 < lines.length && /^[\s|:-]+$/.test(lines[i + 1])) {
      const { tableNode, endIndex } = parseTable(lines, i)
      if (tableNode) {
        nodes.push(tableNode)
        i = endIndex
        continue
      }
    }

    // Paragraph (default)
    const paragraphLines: string[] = []
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !lines[i].startsWith("#") &&
      !lines[i].startsWith("```") &&
      !lines[i].startsWith(">") &&
      !/^[-*+]\s/.test(lines[i]) &&
      !/^\d+\.\s/.test(lines[i])
    ) {
      paragraphLines.push(lines[i])
      i++
    }
    if (paragraphLines.length > 0) {
      nodes.push(paragraph(parseInline(paragraphLines.join(" "))))
    }
  }

  return createDocument(nodes, metadata)
}

/**
 * Parse inline Markdown elements
 */
function parseInline(content: string): InlineNode[] {
  const nodes: InlineNode[] = []
  let remaining = content

  while (remaining.length > 0) {
    // Image: ![alt](src "title")
    const imageMatch = remaining.match(/^!\[([^\]]*)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/)
    if (imageMatch) {
      nodes.push(image(imageMatch[2], imageMatch[1], imageMatch[3]))
      remaining = remaining.slice(imageMatch[0].length)
      continue
    }

    // Link: [text](url "title")
    const linkMatch = remaining.match(/^\[([^\]]+)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/)
    if (linkMatch) {
      nodes.push(link(linkMatch[2], linkMatch[1], linkMatch[3]))
      remaining = remaining.slice(linkMatch[0].length)
      continue
    }

    // Bold: **text** or __text__
    const boldMatch = remaining.match(/^(\*\*|__)([^*_]+)\1/)
    if (boldMatch) {
      nodes.push(text(boldMatch[2], ["bold"]))
      remaining = remaining.slice(boldMatch[0].length)
      continue
    }

    // Italic: *text* or _text_
    const italicMatch = remaining.match(/^(\*|_)([^*_]+)\1/)
    if (italicMatch) {
      nodes.push(text(italicMatch[2], ["italic"]))
      remaining = remaining.slice(italicMatch[0].length)
      continue
    }

    // Inline code: `code`
    const codeInlineMatch = remaining.match(/^`([^`]+)`/)
    if (codeInlineMatch) {
      nodes.push(text(codeInlineMatch[1], ["code"]))
      remaining = remaining.slice(codeInlineMatch[0].length)
      continue
    }

    // Strikethrough: ~~text~~
    const strikeMatch = remaining.match(/^~~([^~]+)~~/)
    if (strikeMatch) {
      nodes.push(text(strikeMatch[1], ["strikethrough"]))
      remaining = remaining.slice(strikeMatch[0].length)
      continue
    }

    // Plain text - find the next special character
    const nextSpecial = remaining.search(/[!\[*_`~]/)
    if (nextSpecial === 0) {
      // Single special char that didn't match a pattern
      nodes.push(text(remaining[0]))
      remaining = remaining.slice(1)
    } else if (nextSpecial > 0) {
      nodes.push(text(remaining.slice(0, nextSpecial)))
      remaining = remaining.slice(nextSpecial)
    } else {
      nodes.push(text(remaining))
      remaining = ""
    }
  }

  return nodes
}

/**
 * Parse a Markdown table
 */
function parseTable(
  lines: string[],
  startIndex: number
): { tableNode: BlockNode | null; endIndex: number } {
  const headerLine = lines[startIndex]
  const separatorLine = lines[startIndex + 1]

  // Parse header
  const headers = headerLine
    .split("|")
    .map((h) => h.trim())
    .filter((h) => h.length > 0)

  // Parse alignment from separator
  const alignments = separatorLine
    .split("|")
    .map((s) => s.trim())
    .filter((s) => s.length > 0)
    .map((s) => {
      if (s.startsWith(":") && s.endsWith(":")) return "center" as const
      if (s.endsWith(":")) return "right" as const
      return "left" as const
    })

  // Parse rows
  const rows: TableRowNode[] = []
  let i = startIndex + 2
  while (i < lines.length && lines[i].includes("|")) {
    const cells = lines[i]
      .split("|")
      .map((c) => c.trim())
      .filter((c, idx, arr) => idx > 0 || c.length > 0) // Handle leading |
      .slice(0, headers.length) // Limit to header count

    const tableCells: TableCellNode[] = cells.map((cellContent, idx) =>
      tableCell(cellContent, { align: alignments[idx] })
    )

    rows.push(tableRow(tableCells))
    i++
  }

  return {
    tableNode: table(headers, rows),
    endIndex: i,
  }
}

/**
 * Parse YAML frontmatter into metadata
 */
function parseYamlFrontmatter(yaml: string): Partial<DocumentMetadata> {
  const metadata: Partial<DocumentMetadata> = {}
  const lines = yaml.split("\n")

  for (const line of lines) {
    const match = line.match(/^(\w+):\s*(.+)$/)
    if (match) {
      const key = match[1]
      const value = match[2].trim().replace(/^["']|["']$/g, "")

      switch (key) {
        case "title":
          metadata.title = value
          break
        case "author":
          metadata.author = value
          break
        case "date":
          metadata.date = value
          break
        case "description":
          metadata.description = value
          break
        case "language":
        case "lang":
          metadata.language = value
          break
        case "tags":
          metadata.tags = value.split(",").map((t) => t.trim())
          break
        default:
          metadata.custom = { ...metadata.custom, [key]: value }
      }
    }
  }

  return metadata
}

/**
 * Convert a string to a URL-safe slug
 */
function slugify(textContent: string): string {
  return textContent
    .toLowerCase()
    .replace(/[^\w\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .trim()
}

// ============================================================================
// HTML Parser
// ============================================================================

/**
 * Parse HTML content into Document IR
 * Uses regex-based parsing for simplicity (not a full DOM parser)
 */
export function parseHtml(content: string): Document {
  const nodes: BlockNode[] = []
  const metadata: DocumentMetadata = { sourceFormat: "html" }

  // Extract title from <title> or <h1>
  const titleMatch = content.match(/<title>([^<]+)<\/title>/i)
  if (titleMatch) {
    metadata.title = titleMatch[1].trim()
  }

  // Extract meta description
  const descMatch = content.match(/<meta\s+name=["']description["']\s+content=["']([^"']+)["']/i)
  if (descMatch) {
    metadata.description = descMatch[1]
  }

  // Extract body content
  const bodyMatch = content.match(/<body[^>]*>([\s\S]*)<\/body>/i)
  const bodyContent = bodyMatch ? bodyMatch[1] : content

  // Parse headings
  const headingRegex = /<h([1-6])[^>]*>([^<]+)<\/h\1>/gi
  let match
  while ((match = headingRegex.exec(bodyContent)) !== null) {
    const level = parseInt(match[1], 10) as 1 | 2 | 3 | 4 | 5 | 6
    const headingContent = stripHtmlTags(match[2])
    nodes.push(heading(level, headingContent, slugify(headingContent)))
  }

  // Parse paragraphs
  const pRegex = /<p[^>]*>([\s\S]*?)<\/p>/gi
  while ((match = pRegex.exec(bodyContent)) !== null) {
    const pContent = stripHtmlTags(match[1])
    if (pContent.trim()) {
      nodes.push(paragraph([text(pContent)]))
    }
  }

  // Parse code blocks
  const preRegex = /<pre[^>]*><code[^>]*(?:class=["'](?:language-)?(\w+)["'])?[^>]*>([\s\S]*?)<\/code><\/pre>/gi
  while ((match = preRegex.exec(bodyContent)) !== null) {
    const language = match[1]
    const codeContent = decodeHtmlEntities(match[2])
    nodes.push(code(codeContent, language))
  }

  // Parse lists
  const ulRegex = /<ul[^>]*>([\s\S]*?)<\/ul>/gi
  while ((match = ulRegex.exec(bodyContent)) !== null) {
    const items = parseListItems(match[1])
    if (items.length > 0) {
      nodes.push(list(items, false))
    }
  }

  const olRegex = /<ol[^>]*>([\s\S]*?)<\/ol>/gi
  while ((match = olRegex.exec(bodyContent)) !== null) {
    const items = parseListItems(match[1])
    if (items.length > 0) {
      nodes.push(list(items, true))
    }
  }

  return createDocument(nodes, metadata)
}

/**
 * Parse list items from HTML
 */
function parseListItems(content: string): ListItemNode[] {
  const items: ListItemNode[] = []
  const liRegex = /<li[^>]*>([\s\S]*?)<\/li>/gi
  let match

  while ((match = liRegex.exec(content)) !== null) {
    const itemContent = stripHtmlTags(match[1])
    if (itemContent.trim()) {
      items.push(listItem(itemContent))
    }
  }

  return items
}

/**
 * Strip HTML tags from content
 */
function stripHtmlTags(html: string): string {
  return html
    .replace(/<[^>]+>/g, "")
    .replace(/\s+/g, " ")
    .trim()
}

/**
 * Decode HTML entities
 */
function decodeHtmlEntities(html: string): string {
  const entities: Record<string, string> = {
    "&lt;": "<",
    "&gt;": ">",
    "&amp;": "&",
    "&quot;": '"',
    "&#39;": "'",
    "&nbsp;": " ",
  }

  return html.replace(/&[^;]+;/g, (entity) => entities[entity] || entity)
}

// ============================================================================
// Code Parser
// ============================================================================

/**
 * Parse source code into Document IR
 * Wraps the entire content in a code block with optional language detection
 */
export function parseCode(content: string, language?: string): Document {
  const detectedLanguage = language || detectLanguage(content)
  const metadata: DocumentMetadata = {
    sourceFormat: "code",
    language: detectedLanguage,
  }

  return createDocument([code(content, detectedLanguage)], metadata)
}

/**
 * Detect programming language from content heuristics
 */
function detectLanguage(content: string): string | undefined {
  const indicators: Record<string, RegExp[]> = {
    typescript: [/^import\s+.*\s+from\s+['"]/, /:\s*(string|number|boolean|void)\b/, /interface\s+\w+\s*\{/],
    javascript: [/^import\s+.*\s+from\s+['"]/, /const\s+\w+\s*=\s*\(\)/, /module\.exports\s*=/],
    python: [/^import\s+\w+/, /^from\s+\w+\s+import/, /def\s+\w+\s*\(/, /class\s+\w+:/],
    rust: [/^use\s+\w+/, /fn\s+\w+\s*\(/, /impl\s+\w+/, /pub\s+(fn|struct|enum)/],
    go: [/^package\s+\w+/, /func\s+\w+\s*\(/, /import\s+\(/],
    java: [/^package\s+[\w.]+;/, /public\s+class\s+\w+/, /public\s+static\s+void\s+main/],
    cpp: [/#include\s*</, /std::\w+/, /int\s+main\s*\(/],
    c: [/#include\s*</, /int\s+main\s*\(/, /printf\s*\(/],
    html: [/<!DOCTYPE\s+html/i, /<html/i, /<head/i, /<body/i],
    css: [/{\s*[\w-]+\s*:/, /@media\s+/, /\.[\w-]+\s*\{/],
    json: [/^\s*\{/, /"\w+"\s*:/, /^\s*\[/],
    yaml: [/^\w+:/, /^\s+-\s+\w+/],
    sql: [/SELECT\s+.*\s+FROM/i, /INSERT\s+INTO/i, /CREATE\s+TABLE/i],
    bash: [/^#!/, /^\s*\w+=/, /^\s*(if|for|while)\s+/],
    markdown: [/^#\s+/, /\[.*\]\(.*\)/, /\*\*.*\*\*/],
  }

  for (const [lang, patterns] of Object.entries(indicators)) {
    const matchCount = patterns.filter((p) => p.test(content)).length
    if (matchCount >= 2) {
      return lang
    }
  }

  return undefined
}

// ============================================================================
// Plain Text Parser
// ============================================================================

/**
 * Parse plain text into Document IR
 * Splits on double newlines to create paragraphs
 */
export function parsePlainText(content: string): Document {
  const metadata: DocumentMetadata = { sourceFormat: "plain" }
  const paragraphs = content
    .split(/\n\n+/)
    .map((p) => p.trim())
    .filter((p) => p.length > 0)

  const nodes: BlockNode[] = paragraphs.map((p) => paragraph([text(p)]))

  return createDocument(nodes, metadata)
}

// ============================================================================
// Universal Parser
// ============================================================================

/**
 * Auto-detect format and parse content
 */
export function parse(content: string, hint?: "markdown" | "html" | "code" | "plain"): Document {
  if (hint) {
    switch (hint) {
      case "markdown":
        return parseMarkdown(content)
      case "html":
        return parseHtml(content)
      case "code":
        return parseCode(content)
      case "plain":
        return parsePlainText(content)
    }
  }

  // Auto-detect based on content
  if (content.trim().startsWith("<!DOCTYPE") || content.trim().startsWith("<html")) {
    return parseHtml(content)
  }

  if (content.startsWith("---\n") || /^#\s+/.test(content) || /\[.*\]\(.*\)/.test(content)) {
    return parseMarkdown(content)
  }

  // Check if it looks like code
  const codeIndicators = [/^import\s+/, /^package\s+/, /^#include/, /^use\s+/, /^from\s+\w+\s+import/]
  if (codeIndicators.some((p) => p.test(content))) {
    return parseCode(content)
  }

  // Default to plain text
  return parsePlainText(content)
}

// ============================================================================
// Exports
// ============================================================================

export const Parser = {
  markdown: parseMarkdown,
  html: parseHtml,
  code: parseCode,
  plainText: parsePlainText,
  parse,
  detectLanguage,
}
