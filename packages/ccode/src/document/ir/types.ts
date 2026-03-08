/**
 * Document Intermediate Representation (IR) Types
 *
 * Provides a structured representation for documents that decouples
 * parsing from rendering. This enables format-agnostic processing
 * and lossless round-trips between formats.
 *
 * ## Design Principle
 *
 * The IR is designed to be:
 * - **LLM-friendly**: Structured data that models can easily manipulate
 * - **Format-agnostic**: Same IR works for Markdown, HTML, and plain text
 * - **Extensible**: New node types can be added without breaking existing code
 */

import z from "zod"

// ============================================================================
// Node Types
// ============================================================================

/**
 * Text node - inline text content
 */
export const TextNodeSchema = z.object({
  type: z.literal("text"),
  content: z.string(),
  marks: z.array(z.enum(["bold", "italic", "code", "strikethrough", "underline"])).optional(),
})
export type TextNode = z.infer<typeof TextNodeSchema>

/**
 * Code block node - fenced or indented code
 */
export const CodeNodeSchema = z.object({
  type: z.literal("code"),
  language: z.string().optional(),
  content: z.string(),
  filename: z.string().optional(),
  highlighted: z.boolean().optional(),
})
export type CodeNode = z.infer<typeof CodeNodeSchema>

/**
 * Heading node - h1-h6
 */
export const HeadingNodeSchema = z.object({
  type: z.literal("heading"),
  level: z.union([z.literal(1), z.literal(2), z.literal(3), z.literal(4), z.literal(5), z.literal(6)]),
  content: z.string(),
  anchor: z.string().optional(),
})
export type HeadingNode = z.infer<typeof HeadingNodeSchema>

/**
 * Link node - hyperlink
 */
export const LinkNodeSchema = z.object({
  type: z.literal("link"),
  url: z.string(),
  title: z.string().optional(),
  text: z.string(), // Simplified from children for non-circular reference
})
export type LinkNode = z.infer<typeof LinkNodeSchema>

/**
 * Image node - embedded image
 */
export const ImageNodeSchema = z.object({
  type: z.literal("image"),
  src: z.string(),
  alt: z.string().optional(),
  title: z.string().optional(),
  width: z.number().optional(),
  height: z.number().optional(),
})
export type ImageNode = z.infer<typeof ImageNodeSchema>

/**
 * Inline nodes - can appear within paragraphs
 */
export const InlineNodeSchema = z.discriminatedUnion("type", [
  TextNodeSchema,
  LinkNodeSchema,
  ImageNodeSchema,
])
export type InlineNode = z.infer<typeof InlineNodeSchema>

/**
 * Paragraph node - block of text
 */
export const ParagraphNodeSchema = z.object({
  type: z.literal("paragraph"),
  children: z.array(InlineNodeSchema),
})
export type ParagraphNode = z.infer<typeof ParagraphNodeSchema>

/**
 * List item node
 */
export const ListItemNodeSchema = z.object({
  type: z.literal("list_item"),
  content: z.string(), // Simplified - just text content
  checked: z.boolean().optional(), // For task lists
})
export type ListItemNode = z.infer<typeof ListItemNodeSchema>

/**
 * List node - ordered or unordered
 */
export const ListNodeSchema = z.object({
  type: z.literal("list"),
  ordered: z.boolean(),
  start: z.number().optional(), // Starting number for ordered lists
  items: z.array(ListItemNodeSchema),
})
export type ListNode = z.infer<typeof ListNodeSchema>

/**
 * Table cell node
 */
export const TableCellNodeSchema = z.object({
  type: z.literal("table_cell"),
  header: z.boolean().optional(),
  align: z.enum(["left", "center", "right"]).optional(),
  content: z.string(),
})
export type TableCellNode = z.infer<typeof TableCellNodeSchema>

/**
 * Table row node
 */
export const TableRowNodeSchema = z.object({
  type: z.literal("table_row"),
  cells: z.array(TableCellNodeSchema),
})
export type TableRowNode = z.infer<typeof TableRowNodeSchema>

/**
 * Table node - tabular data
 */
export const TableNodeSchema = z.object({
  type: z.literal("table"),
  headers: z.array(z.string()),
  rows: z.array(TableRowNodeSchema),
  caption: z.string().optional(),
})
export type TableNode = z.infer<typeof TableNodeSchema>

/**
 * Blockquote node - quoted content
 */
export const BlockquoteNodeSchema = z.object({
  type: z.literal("blockquote"),
  content: z.string(), // Simplified - just text content
})
export type BlockquoteNode = z.infer<typeof BlockquoteNodeSchema>

/**
 * Horizontal rule node - thematic break
 */
export const HorizontalRuleNodeSchema = z.object({
  type: z.literal("horizontal_rule"),
})
export type HorizontalRuleNode = z.infer<typeof HorizontalRuleNodeSchema>

/**
 * Raw HTML node - passthrough HTML
 */
export const RawHtmlNodeSchema = z.object({
  type: z.literal("raw_html"),
  content: z.string(),
})
export type RawHtmlNode = z.infer<typeof RawHtmlNodeSchema>

// ============================================================================
// Composite Types
// ============================================================================

/**
 * Block nodes - top-level document elements
 */
export const BlockNodeSchema = z.discriminatedUnion("type", [
  TextNodeSchema,
  CodeNodeSchema,
  HeadingNodeSchema,
  ParagraphNodeSchema,
  ListNodeSchema,
  ListItemNodeSchema,
  LinkNodeSchema,
  ImageNodeSchema,
  TableNodeSchema,
  BlockquoteNodeSchema,
  HorizontalRuleNodeSchema,
  RawHtmlNodeSchema,
])
export type BlockNode = z.infer<typeof BlockNodeSchema>

/**
 * Document node - alias for BlockNode
 */
export const DocumentNodeSchema = BlockNodeSchema
export type DocumentNode = BlockNode

// ============================================================================
// Document Structure
// ============================================================================

/**
 * Document metadata
 */
export const DocumentMetadataSchema = z.object({
  title: z.string().optional(),
  author: z.string().optional(),
  date: z.string().optional(),
  description: z.string().optional(),
  tags: z.array(z.string()).optional(),
  language: z.string().optional(),
  sourceFormat: z.enum(["markdown", "html", "code", "plain"]).optional(),
  custom: z.record(z.string(), z.unknown()).optional(),
})
export type DocumentMetadata = z.infer<typeof DocumentMetadataSchema>

/**
 * Complete document with metadata and content
 */
export const DocumentSchema = z.object({
  nodes: z.array(BlockNodeSchema),
  metadata: DocumentMetadataSchema.optional(),
})
export type Document = z.infer<typeof DocumentSchema>

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Create a text node
 */
export function text(content: string, marks?: TextNode["marks"]): TextNode {
  return { type: "text", content, ...(marks && { marks }) }
}

/**
 * Create a code block node
 */
export function code(content: string, language?: string, filename?: string): CodeNode {
  return { type: "code", content, ...(language && { language }), ...(filename && { filename }) }
}

/**
 * Create a heading node
 */
export function heading(level: HeadingNode["level"], content: string, anchor?: string): HeadingNode {
  return { type: "heading", level, content, ...(anchor && { anchor }) }
}

/**
 * Create a paragraph node
 */
export function paragraph(children: InlineNode[]): ParagraphNode {
  return { type: "paragraph", children }
}

/**
 * Create a list node
 */
export function list(items: ListItemNode[], ordered = false, start?: number): ListNode {
  return { type: "list", ordered, items, ...(start !== undefined && { start }) }
}

/**
 * Create a list item node
 */
export function listItem(content: string, checked?: boolean): ListItemNode {
  return { type: "list_item", content, ...(checked !== undefined && { checked }) }
}

/**
 * Create a link node
 */
export function link(url: string, text: string, title?: string): LinkNode {
  return { type: "link", url, text, ...(title && { title }) }
}

/**
 * Create an image node
 */
export function image(src: string, alt?: string, title?: string): ImageNode {
  return { type: "image", src, ...(alt && { alt }), ...(title && { title }) }
}

/**
 * Create a table node
 */
export function table(headers: string[], rows: TableRowNode[], caption?: string): TableNode {
  return { type: "table", headers, rows, ...(caption && { caption }) }
}

/**
 * Create a table row
 */
export function tableRow(cells: TableCellNode[]): TableRowNode {
  return { type: "table_row", cells }
}

/**
 * Create a table cell
 */
export function tableCell(content: string, options?: { header?: boolean; align?: TableCellNode["align"] }): TableCellNode {
  return { type: "table_cell", content, ...options }
}

/**
 * Create a blockquote node
 */
export function blockquote(content: string): BlockquoteNode {
  return { type: "blockquote", content }
}

/**
 * Create a horizontal rule node
 */
export function horizontalRule(): HorizontalRuleNode {
  return { type: "horizontal_rule" }
}

/**
 * Create an empty document
 */
export function createDocument(nodes: BlockNode[] = [], metadata?: DocumentMetadata): Document {
  return { nodes, ...(metadata && { metadata }) }
}
