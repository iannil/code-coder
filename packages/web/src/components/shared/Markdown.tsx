/**
 * Markdown Component
 *
 * Renders markdown content with syntax highlighting using:
 * - react-markdown for markdown parsing
 * - shiki for code syntax highlighting
 * - Custom renderers for links and other elements
 */

import * as React from "react"
import ReactMarkdown, { type Components } from "react-markdown"
import { ExternalLink } from "lucide-react"

import { cn } from "@/lib/utils"

// ============================================================================
// Types
// ============================================================================

export interface MarkdownProps {
  /** Markdown content to render */
  content: string
  /** Optional className */
  className?: string
  /** Whether to enable syntax highlighting */
  highlightSyntax?: boolean
  /** Theme for syntax highlighting */
  theme?: "light" | "dark" | "github-light" | "github-dark" | "nord" | "monokai"
}

// ============================================================================
// Syntax Highlighting
// ============================================================================

interface CodeBlockProps {
  language: string | undefined
  code: string
}

function CodeHighlighter({ language, code }: CodeBlockProps) {
  const [html, setHtml] = React.useState<string>("")
  const [loading, setLoading] = React.useState(true)

  React.useEffect(() => {
    const highlight = async () => {
      try {
        const { codeToHtml } = await import("shiki")

        const highlighted = await codeToHtml(code, {
          lang: language ?? "text",
          theme: "github-dark",
        })

        setHtml(highlighted)
      } catch {
        // Fallback to preformatted text if highlighting fails
        setHtml(`<pre class="shiki"><code>${escapeHtml(code)}</code></pre>`)
      } finally {
        setLoading(false)
      }
    }

    highlight()
  }, [code, language])

  if (loading) {
    return (
      <pre className="rounded-md bg-muted p-4 overflow-x-auto">
        <code className="text-sm">{code}</code>
      </pre>
    )
  }

  return (
    <div
      className="rounded-md overflow-x-auto"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}

function escapeHtml(text: string): string {
  const div = document.createElement("div")
  div.textContent = text
  return div.innerHTML
}

// ============================================================================
// Custom Components
// ============================================================================

const components: Partial<Components> = {
  // Code blocks with syntax highlighting
  pre: ({ children, ...props }) => {
    const child = React.Children.only(children)
    if (
      React.isValidElement(child) &&
      "props" in child &&
      typeof child.props === "object" &&
      child.props !== null &&
      "className" in child.props &&
      typeof child.props.className === "string" &&
      child.props.className.includes("language-")
    ) {
      const language = child.props.className.replace("language-", "")
      const code = (child.props as { children?: string }).children

      if (typeof code === "string") {
        return <CodeHighlighter language={language} code={code} />
      }
    }

    return <pre {...props}>{children}</pre>
  },

  // Inline code
  code: ({ className, children, ...props }) => {
    const isInline = !className
    if (isInline) {
      return (
        <code
          className="px-1.5 py-0.5 rounded-md bg-muted text-sm font-mono"
          {...props}
        >
          {children}
        </code>
      )
    }
    return <code className={className} {...props}>{children as React.ReactNode}</code>
  },

  // Links with external icon
  a: ({ href, children, ...props }) => {
    const isExternal = href?.startsWith("http") ?? false

    if (isExternal) {
      return (
        <a
          href={href}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-0.5 text-primary hover:underline"
          {...props}
        >
          {children}
          <ExternalLink className="h-3 w-3 shrink-0" />
        </a>
      )
    }

    return (
      <a href={href} className="text-primary hover:underline" {...props}>
        {children}
      </a>
    )
  },

  // Headings
  h1: ({ children, ...props }) => (
    <h1 className="text-2xl font-bold mt-6 mb-4 first:mt-0" {...props}>
      {children}
    </h1>
  ),
  h2: ({ children, ...props }) => (
    <h2 className="text-xl font-bold mt-5 mb-3 first:mt-0" {...props}>
      {children}
    </h2>
  ),
  h3: ({ children, ...props }) => (
    <h3 className="text-lg font-semibold mt-4 mb-2 first:mt-0" {...props}>
      {children}
    </h3>
  ),
  h4: ({ children, ...props }) => (
    <h4 className="text-base font-semibold mt-3 mb-2 first:mt-0" {...props}>
      {children}
    </h4>
  ),

  // Paragraphs
  p: ({ children, ...props }) => (
    <p className="my-3 leading-7 first:mt-0 last:mb-0" {...props}>
      {children}
    </p>
  ),

  // Lists
  ul: ({ children, ...props }) => (
    <ul className="my-3 ml-6 list-disc space-y-1" {...props}>
      {children}
    </ul>
  ),
  ol: ({ children, ...props }) => (
    <ol className="my-3 ml-6 list-decimal space-y-1" {...props}>
      {children}
    </ol>
  ),
  li: ({ children, ...props }) => (
    <li className="leading-7" {...props}>
      {children}
    </li>
  ),

  // Blockquotes
  blockquote: ({ children, ...props }) => (
    <blockquote
      className="my-3 border-l-4 border-muted-foreground/20 pl-4 italic text-muted-foreground"
      {...props}
    >
      {children}
    </blockquote>
  ),

  // Horizontal rule
  hr: (props) => <hr className="my-6 border-t border-border" {...props} />,

  // Tables
  table: ({ children, ...props }) => (
    <div className="my-4 overflow-x-auto">
      <table className="min-w-full divide-y divide-border" {...props}>
        {children}
      </table>
    </div>
  ),
  thead: ({ children, ...props }) => (
    <thead className="bg-muted" {...props}>
      {children}
    </thead>
  ),
  tbody: ({ children, ...props }) => (
    <tbody className="divide-y divide-border" {...props}>
      {children}
    </tbody>
  ),
  tr: ({ children, ...props }) => (
    <tr className="hover:bg-muted/50" {...props}>
      {children}
    </tr>
  ),
  th: ({ children, ...props }) => (
    <th className="px-4 py-2 text-left text-sm font-semibold" {...props}>
      {children}
    </th>
  ),
  td: ({ children, ...props }) => (
    <td className="px-4 py-2 text-sm" {...props}>
      {children}
    </td>
  ),

  // Images
  img: ({ src, alt, ...props }) => (
    <img
      src={src}
      alt={alt}
      className="my-4 rounded-lg max-w-full h-auto"
      loading="lazy"
      {...props}
    />
  ),

  // Strong
  strong: ({ children, ...props }) => (
    <strong className="font-semibold" {...props}>
      {children}
    </strong>
  ),

  // Emphasis
  em: ({ children, ...props }) => (
    <em className="italic" {...props}>
      {children}
    </em>
  ),
}

// ============================================================================
// Markdown Component
// ============================================================================

export function Markdown({
  content,
  className,
}: MarkdownProps) {
  return (
    <div className={cn("prose prose-sm max-w-none dark:prose-invert", className)}>
      <ReactMarkdown components={components}>{content}</ReactMarkdown>
    </div>
  )
}

// ============================================================================
// Lazy-loaded Markdown (for large content)
// ============================================================================

export interface LazyMarkdownProps extends Omit<MarkdownProps, "content"> {
  /** Function that returns the markdown content */
  getContent: () => Promise<string> | string
  /** Optional fallback while loading */
  fallback?: React.ReactNode
}

export function LazyMarkdown({ getContent, fallback, ...props }: LazyMarkdownProps) {
  const [content, setContent] = React.useState<string>("")
  const [loading, setLoading] = React.useState(true)

  React.useEffect(() => {
    const load = async () => {
      try {
        const result = await getContent()
        setContent(result)
      } finally {
        setLoading(false)
      }
    }

    load()
  }, [getContent])

  if (loading) {
    return <>{fallback ?? <div className="animate-pulse h-4 bg-muted rounded" />}</>
  }

  return <Markdown content={content} {...props} />
}
