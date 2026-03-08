/**
 * Document Renderers
 *
 * Renders Document IR to various output formats.
 * Supports Markdown, HTML, and plain text output.
 *
 * ## Design Principle
 *
 * Rendering is a **deterministic** transformation from IR to text.
 * Each renderer is pure and produces consistent output for the same input.
 */

import type {
  Document,
  BlockNode,
  InlineNode,
  TextNode,
  CodeNode,
  HeadingNode,
  ParagraphNode,
  ListNode,
  ListItemNode,
  LinkNode,
  ImageNode,
  TableNode,
  BlockquoteNode,
  HorizontalRuleNode,
  RawHtmlNode,
} from "./types"

// ============================================================================
// Markdown Renderer
// ============================================================================

/**
 * Render Document IR to Markdown
 */
export function toMarkdown(doc: Document): string {
  const lines: string[] = []

  // Render frontmatter if metadata exists
  if (doc.metadata && Object.keys(doc.metadata).length > 0) {
    const frontmatter = renderFrontmatter(doc.metadata)
    if (frontmatter) {
      lines.push("---")
      lines.push(frontmatter)
      lines.push("---")
      lines.push("")
    }
  }

  // Render nodes
  for (const node of doc.nodes) {
    lines.push(renderBlockToMarkdown(node))
    lines.push("")
  }

  return lines.join("\n").trimEnd() + "\n"
}

/**
 * Render a block node to Markdown
 */
function renderBlockToMarkdown(node: BlockNode): string {
  switch (node.type) {
    case "text":
      return renderTextToMarkdown(node)

    case "code":
      return renderCodeToMarkdown(node)

    case "heading":
      return renderHeadingToMarkdown(node)

    case "paragraph":
      return renderParagraphToMarkdown(node)

    case "list":
      return renderListToMarkdown(node)

    case "list_item":
      return renderListItemToMarkdown(node, false, 0)

    case "link":
      return renderLinkToMarkdown(node)

    case "image":
      return renderImageToMarkdown(node)

    case "table":
      return renderTableToMarkdown(node)

    case "blockquote":
      return renderBlockquoteToMarkdown(node)

    case "horizontal_rule":
      return "---"

    case "raw_html":
      return node.content

    default:
      return ""
  }
}

function renderTextToMarkdown(node: TextNode): string {
  let content = node.content
  const marks = node.marks || []

  // Apply marks in order
  if (marks.includes("code")) {
    content = `\`${content}\``
  }
  if (marks.includes("bold")) {
    content = `**${content}**`
  }
  if (marks.includes("italic")) {
    content = `*${content}*`
  }
  if (marks.includes("strikethrough")) {
    content = `~~${content}~~`
  }

  return content
}

function renderCodeToMarkdown(node: CodeNode): string {
  const lang = node.language || ""
  const filename = node.filename ? ` ${node.filename}` : ""
  return `\`\`\`${lang}${filename}\n${node.content}\n\`\`\``
}

function renderHeadingToMarkdown(node: HeadingNode): string {
  const prefix = "#".repeat(node.level)
  return `${prefix} ${node.content}`
}

function renderParagraphToMarkdown(node: ParagraphNode): string {
  return node.children.map((child: InlineNode) => renderInlineToMarkdown(child)).join("")
}

function renderListToMarkdown(node: ListNode): string {
  return node.items
    .map((item: ListItemNode, idx: number) => renderListItemToMarkdown(item, node.ordered, idx, node.start))
    .join("\n")
}

function renderListItemToMarkdown(
  node: ListItemNode,
  ordered: boolean,
  index: number,
  start = 1
): string {
  const prefix = ordered ? `${start + index}.` : "-"
  const checkbox =
    node.checked !== undefined ? (node.checked ? "[x] " : "[ ] ") : ""

  return `${prefix} ${checkbox}${node.content}`
}

function renderLinkToMarkdown(node: LinkNode): string {
  const title = node.title ? ` "${node.title}"` : ""
  return `[${node.text}](${node.url}${title})`
}

function renderImageToMarkdown(node: ImageNode): string {
  const alt = node.alt || ""
  const title = node.title ? ` "${node.title}"` : ""
  return `![${alt}](${node.src}${title})`
}

function renderTableToMarkdown(node: TableNode): string {
  const lines: string[] = []

  // Header row
  lines.push(`| ${node.headers.join(" | ")} |`)

  // Separator row with alignment
  const separators = node.rows[0]?.cells.map((cell) => {
    const align = cell.align || "left"
    switch (align) {
      case "center":
        return ":---:"
      case "right":
        return "---:"
      default:
        return "---"
    }
  }) || node.headers.map(() => "---")
  lines.push(`| ${separators.join(" | ")} |`)

  // Data rows
  for (const row of node.rows) {
    const cells = row.cells.map((cell) => cell.content)
    lines.push(`| ${cells.join(" | ")} |`)
  }

  return lines.join("\n")
}

function renderBlockquoteToMarkdown(node: BlockquoteNode): string {
  return node.content
    .split("\n")
    .map((line: string) => `> ${line}`)
    .join("\n")
}

function renderInlineToMarkdown(node: InlineNode): string {
  switch (node.type) {
    case "text":
      return renderTextToMarkdown(node)
    case "link":
      return renderLinkToMarkdown(node)
    case "image":
      return renderImageToMarkdown(node)
    default:
      return ""
  }
}

function renderFrontmatter(metadata: Document["metadata"]): string {
  if (!metadata) return ""

  const lines: string[] = []

  if (metadata.title) lines.push(`title: ${metadata.title}`)
  if (metadata.author) lines.push(`author: ${metadata.author}`)
  if (metadata.date) lines.push(`date: ${metadata.date}`)
  if (metadata.description) lines.push(`description: ${metadata.description}`)
  if (metadata.language) lines.push(`language: ${metadata.language}`)
  if (metadata.tags && metadata.tags.length > 0) {
    lines.push(`tags: ${metadata.tags.join(", ")}`)
  }

  // Custom fields
  if (metadata.custom) {
    for (const [key, value] of Object.entries(metadata.custom)) {
      lines.push(`${key}: ${value}`)
    }
  }

  return lines.join("\n")
}

// ============================================================================
// HTML Renderer
// ============================================================================

/**
 * Render Document IR to HTML
 */
export function toHtml(doc: Document, options: HtmlRenderOptions = {}): string {
  const { fullDocument = false, className, styles } = options
  const bodyContent = doc.nodes.map((node) => renderBlockToHtml(node)).join("\n")

  if (!fullDocument) {
    const wrapper = className ? `<div class="${className}">\n${bodyContent}\n</div>` : bodyContent
    return wrapper
  }

  const title = doc.metadata?.title || "Document"
  const styleTag = styles ? `<style>\n${styles}\n</style>` : ""

  return `<!DOCTYPE html>
<html lang="${doc.metadata?.language || "en"}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${escapeHtml(title)}</title>
  ${doc.metadata?.description ? `<meta name="description" content="${escapeHtml(doc.metadata.description)}">` : ""}
  ${styleTag}
</head>
<body>
${bodyContent}
</body>
</html>`
}

interface HtmlRenderOptions {
  fullDocument?: boolean
  className?: string
  styles?: string
}

function renderBlockToHtml(node: BlockNode): string {
  switch (node.type) {
    case "text":
      return renderTextToHtml(node)

    case "code":
      return renderCodeToHtml(node)

    case "heading":
      return renderHeadingToHtml(node)

    case "paragraph":
      return renderParagraphToHtml(node)

    case "list":
      return renderListToHtml(node)

    case "list_item":
      return renderListItemToHtml(node)

    case "link":
      return renderLinkToHtml(node)

    case "image":
      return renderImageToHtml(node)

    case "table":
      return renderTableToHtml(node)

    case "blockquote":
      return renderBlockquoteToHtml(node)

    case "horizontal_rule":
      return "<hr>"

    case "raw_html":
      return node.content

    default:
      return ""
  }
}

function renderTextToHtml(node: TextNode): string {
  let content = escapeHtml(node.content)
  const marks = node.marks || []

  if (marks.includes("code")) {
    content = `<code>${content}</code>`
  }
  if (marks.includes("bold")) {
    content = `<strong>${content}</strong>`
  }
  if (marks.includes("italic")) {
    content = `<em>${content}</em>`
  }
  if (marks.includes("strikethrough")) {
    content = `<del>${content}</del>`
  }
  if (marks.includes("underline")) {
    content = `<u>${content}</u>`
  }

  return content
}

function renderCodeToHtml(node: CodeNode): string {
  const langClass = node.language ? ` class="language-${node.language}"` : ""
  return `<pre><code${langClass}>${escapeHtml(node.content)}</code></pre>`
}

function renderHeadingToHtml(node: HeadingNode): string {
  const anchor = node.anchor ? ` id="${node.anchor}"` : ""
  return `<h${node.level}${anchor}>${escapeHtml(node.content)}</h${node.level}>`
}

function renderParagraphToHtml(node: ParagraphNode): string {
  const content = node.children.map((child: InlineNode) => renderInlineToHtml(child)).join("")
  return `<p>${content}</p>`
}

function renderListToHtml(node: ListNode): string {
  const tag = node.ordered ? "ol" : "ul"
  const start = node.ordered && node.start && node.start !== 1 ? ` start="${node.start}"` : ""
  const items = node.items.map((item: ListItemNode) => renderListItemToHtml(item)).join("\n")
  return `<${tag}${start}>\n${items}\n</${tag}>`
}

function renderListItemToHtml(node: ListItemNode): string {
  const checkbox =
    node.checked !== undefined
      ? `<input type="checkbox"${node.checked ? " checked" : ""} disabled> `
      : ""
  return `<li>${checkbox}${escapeHtml(node.content)}</li>`
}

function renderLinkToHtml(node: LinkNode): string {
  const title = node.title ? ` title="${escapeHtml(node.title)}"` : ""
  return `<a href="${escapeHtml(node.url)}"${title}>${escapeHtml(node.text)}</a>`
}

function renderImageToHtml(node: ImageNode): string {
  const alt = node.alt ? ` alt="${escapeHtml(node.alt)}"` : ""
  const title = node.title ? ` title="${escapeHtml(node.title)}"` : ""
  const dims =
    (node.width ? ` width="${node.width}"` : "") +
    (node.height ? ` height="${node.height}"` : "")
  return `<img src="${escapeHtml(node.src)}"${alt}${title}${dims}>`
}

function renderTableToHtml(node: TableNode): string {
  const lines: string[] = ["<table>"]

  // Caption
  if (node.caption) {
    lines.push(`<caption>${escapeHtml(node.caption)}</caption>`)
  }

  // Header
  lines.push("<thead>")
  lines.push("<tr>")
  for (const header of node.headers) {
    lines.push(`<th>${escapeHtml(header)}</th>`)
  }
  lines.push("</tr>")
  lines.push("</thead>")

  // Body
  lines.push("<tbody>")
  for (const row of node.rows) {
    lines.push("<tr>")
    for (const cell of row.cells) {
      const align = cell.align ? ` style="text-align: ${cell.align}"` : ""
      lines.push(`<td${align}>${escapeHtml(cell.content)}</td>`)
    }
    lines.push("</tr>")
  }
  lines.push("</tbody>")
  lines.push("</table>")

  return lines.join("\n")
}

function renderBlockquoteToHtml(node: BlockquoteNode): string {
  return `<blockquote>\n<p>${escapeHtml(node.content)}</p>\n</blockquote>`
}

function renderInlineToHtml(node: InlineNode): string {
  switch (node.type) {
    case "text":
      return renderTextToHtml(node)
    case "link":
      return renderLinkToHtml(node)
    case "image":
      return renderImageToHtml(node)
    default:
      return ""
  }
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;")
}

// ============================================================================
// Plain Text Renderer
// ============================================================================

/**
 * Render Document IR to plain text
 */
export function toPlainText(doc: Document): string {
  return doc.nodes.map((node) => renderBlockToPlainText(node)).join("\n\n")
}

function renderBlockToPlainText(node: BlockNode): string {
  switch (node.type) {
    case "text":
      return node.content

    case "code":
      return node.content

    case "heading":
      return node.content

    case "paragraph":
      return node.children.map((child: InlineNode) => renderInlineToPlainText(child)).join("")

    case "list":
      return node.items
        .map((item: ListItemNode, idx: number) => {
          const prefix = node.ordered ? `${(node.start || 1) + idx}.` : "•"
          const checkbox =
            item.checked !== undefined ? (item.checked ? "[x]" : "[ ]") + " " : ""
          return `${prefix} ${checkbox}${item.content}`
        })
        .join("\n")

    case "list_item":
      return node.content

    case "link":
      return node.text

    case "image":
      return node.alt || "[Image]"

    case "table":
      return renderTableToPlainText(node)

    case "blockquote":
      return node.content
        .split("\n")
        .map((line: string) => `| ${line}`)
        .join("\n")

    case "horizontal_rule":
      return "────────────────"

    case "raw_html":
      // Strip HTML tags for plain text
      return node.content.replace(/<[^>]+>/g, "")

    default:
      return ""
  }
}

function renderTableToPlainText(node: TableNode): string {
  const widths = node.headers.map((h, i) => {
    const headerWidth = h.length
    const maxCellWidth = Math.max(
      ...node.rows.map((row) => {
        const cell = row.cells[i]
        return cell ? cell.content.length : 0
      })
    )
    return Math.max(headerWidth, maxCellWidth)
  })

  const lines: string[] = []

  // Header
  const headerLine = node.headers
    .map((h, i) => h.padEnd(widths[i]))
    .join(" | ")
  lines.push(headerLine)

  // Separator
  lines.push(widths.map((w) => "-".repeat(w)).join("-+-"))

  // Rows
  for (const row of node.rows) {
    const rowLine = row.cells
      .map((cell, i) => cell.content.padEnd(widths[i] || 0))
      .join(" | ")
    lines.push(rowLine)
  }

  return lines.join("\n")
}

function renderInlineToPlainText(node: InlineNode): string {
  switch (node.type) {
    case "text":
      return node.content
    case "link":
      return node.text
    case "image":
      return node.alt || "[Image]"
    default:
      return ""
  }
}

// ============================================================================
// Exports
// ============================================================================

export const Renderer = {
  markdown: toMarkdown,
  html: toHtml,
  plainText: toPlainText,
}
