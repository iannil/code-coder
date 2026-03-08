/**
 * Document Intermediate Representation (IR) Module
 *
 * Provides a structured representation for documents that enables:
 * - **Format-agnostic processing**: Parse once, render to multiple formats
 * - **LLM-friendly structure**: Structured data for model manipulation
 * - **Lossless round-trips**: Parse → IR → Render preserves fidelity
 *
 * ## Quick Start
 *
 * ```typescript
 * import { DocumentIR } from "@/document/ir"
 *
 * // Parse Markdown to IR
 * const doc = DocumentIR.parse("# Hello\n\nWorld", "markdown")
 *
 * // Render to HTML
 * const html = DocumentIR.toHtml(doc)
 *
 * // Or use the factory functions
 * const custom = DocumentIR.createDocument([
 *   DocumentIR.heading(1, "Hello"),
 *   DocumentIR.paragraph([DocumentIR.text("World")]),
 * ])
 * ```
 *
 * ## Design Principle
 *
 * The IR layer bridges **deterministic** parsing/rendering with
 * **uncertain** semantic understanding. Parsers and renderers are
 * pure functions; semantic analysis would use LLM reasoning.
 */

// Re-export types
export type {
  TextNode,
  CodeNode,
  HeadingNode,
  ParagraphNode,
  ListNode,
  ListItemNode,
  LinkNode,
  ImageNode,
  TableNode,
  TableRowNode,
  TableCellNode,
  BlockquoteNode,
  HorizontalRuleNode,
  RawHtmlNode,
  InlineNode,
  BlockNode,
  DocumentNode,
  DocumentMetadata,
  Document,
} from "./types"

// Re-export Zod schemas (with Schema suffix to avoid conflicts)
export {
  TextNodeSchema,
  CodeNodeSchema,
  HeadingNodeSchema,
  ParagraphNodeSchema,
  ListNodeSchema,
  ListItemNodeSchema,
  LinkNodeSchema,
  ImageNodeSchema,
  TableNodeSchema,
  TableRowNodeSchema,
  TableCellNodeSchema,
  BlockquoteNodeSchema,
  HorizontalRuleNodeSchema,
  RawHtmlNodeSchema,
  InlineNodeSchema,
  BlockNodeSchema,
  DocumentNodeSchema,
  DocumentMetadataSchema,
  DocumentSchema,
  // Factory functions
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

// Re-export parsers
export {
  parseMarkdown,
  parseHtml,
  parseCode,
  parsePlainText,
  parse,
  Parser,
} from "./parser"

// Re-export renderers
export {
  toMarkdown,
  toHtml,
  toPlainText,
  Renderer,
} from "./renderer"

// ============================================================================
// Convenience Namespace
// ============================================================================

import {
  text,
  code,
  heading,
  paragraph,
  list,
  listItem,
  link,
  image,
  table,
  blockquote,
  horizontalRule,
  createDocument,
  type Document,
} from "./types"

import { parse, parseMarkdown, parseHtml, parseCode, parsePlainText } from "./parser"
import { toMarkdown, toHtml, toPlainText } from "./renderer"

/**
 * Document IR namespace for convenient access to all functionality
 */
export const DocumentIR = {
  // Factory functions
  text,
  code,
  heading,
  paragraph,
  list,
  listItem,
  link,
  image,
  table,
  blockquote,
  horizontalRule,
  createDocument,

  // Parsers
  parse,
  parseMarkdown,
  parseHtml,
  parseCode,
  parsePlainText,

  // Renderers
  toMarkdown,
  toHtml,
  toPlainText,

  /**
   * Round-trip test: parse and re-render to verify losslessness
   */
  roundTrip(content: string, format: "markdown" | "html" | "code" | "plain" = "markdown"): {
    original: string
    parsed: Document
    rendered: string
    isLossless: boolean
  } {
    const parsed = parse(content, format)
    const rendered =
      format === "html"
        ? toHtml(parsed)
        : format === "code" || format === "plain"
          ? toPlainText(parsed)
          : toMarkdown(parsed)

    return {
      original: content,
      parsed,
      rendered,
      isLossless: normalize(content) === normalize(rendered),
    }
  },
}

/**
 * Normalize whitespace for comparison
 */
function normalize(textContent: string): string {
  return textContent.trim().replace(/\s+/g, " ")
}
