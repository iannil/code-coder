/**
 * CodeBlock Component
 *
 * A code block component with syntax highlighting using shiki.
 * Features:
 * - Syntax highlighting for many languages
 * - Language label display
 * - Copy to clipboard button
 * - Line numbers (optional)
 */

import * as React from "react"
import { Check, Copy, Code } from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/Button"

// ============================================================================
// Types
// ============================================================================

export interface CodeBlockProps {
  /** The code to display */
  code: string
  /** Programming language for syntax highlighting */
  language?: string
  /** Optional filename to display */
  filename?: string
  /** Whether to show line numbers */
  showLineNumbers?: boolean
  /** Whether to show the copy button */
  showCopyButton?: boolean
  /** Maximum height before scrolling */
  maxHeight?: string | number
  /** Optional className */
  className?: string
}

// ============================================================================
// Code Block Component
// ============================================================================

export function CodeBlock({
  code,
  language = "text",
  filename,
  showLineNumbers = false,
  showCopyButton = true,
  maxHeight,
  className,
}: CodeBlockProps) {
  const [highlightedCode, setHighlightedCode] = React.useState<string>("")
  const [copied, setCopied] = React.useState(false)
  const [loading, setLoading] = React.useState(true)

  // Syntax highlighting
  React.useEffect(() => {
    const highlight = async () => {
      try {
        const { codeToHtml } = await import("shiki")

        const result = await codeToHtml(code, {
          lang: language,
          theme: "github-dark",
        })

        setHighlightedCode(result)
      } catch {
        // Fallback: escape HTML and wrap in pre
        const escaped = code
          .replace(/&/g, "&amp;")
          .replace(/</g, "&lt;")
          .replace(/>/g, "&gt;")
        setHighlightedCode(`<pre class="shiki"><code>${escaped}</code></pre>`)
      } finally {
        setLoading(false)
      }
    }

    highlight()
  }, [code, language])

  // Copy to clipboard
  const handleCopy = React.useCallback(() => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }, [code])

  // Generate line numbers
  const lines = React.useMemo(() => {
    return code.split("\n").length
  }, [code])

  return (
    <div
      className={cn(
        "group relative my-4 rounded-lg overflow-hidden border bg-[#0d1117]",
        className
      )}
    >
      {/* Header bar */}
      <div className="flex items-center justify-between px-4 py-2 bg-[#161b22] border-b border-[#30363d]">
        <div className="flex items-center gap-2">
          <Code className="h-4 w-4 text-muted-foreground" />
          {filename ? (
            <span className="text-sm font-medium">{filename}</span>
          ) : (
            <span className="text-sm text-muted-foreground capitalize">{language}</span>
          )}
        </div>

        {showCopyButton && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 opacity-0 group-hover:opacity-100 transition-opacity"
            onClick={handleCopy}
          >
            {copied ? (
              <>
                <Check className="h-3.5 w-3.5 mr-1 text-green-500" />
                <span className="text-xs">Copied!</span>
              </>
            ) : (
              <>
                <Copy className="h-3.5 w-3.5 mr-1" />
                <span className="text-xs">Copy</span>
              </>
            )}
          </Button>
        )}
      </div>

      {/* Code content */}
      <div
        className={cn("overflow-x-auto")}
        style={{ maxHeight: maxHeight ? `${maxHeight}px` : undefined }}
      >
        {loading ? (
          <pre className="p-4">
            <code className="text-sm text-muted-foreground">{code}</code>
          </pre>
        ) : (
          <div
            className="code-block-wrapper relative"
            dangerouslySetInnerHTML={{ __html: highlightedCode }}
          />
        )}
      </div>

      {/* Line numbers (optional) */}
      {showLineNumbers && (
        <div className="absolute left-0 top-10 bottom-0 w-10 bg-[#161b22] border-r border-[#30363d] flex flex-col items-end pr-2 pt-2 text-sm text-muted-foreground select-none">
          {Array.from({ length: lines }).map((_, i) => (
            <span key={i} className="leading-6">
              {i + 1}
            </span>
          ))}
        </div>
      )}
    </div>
  )
}

// ============================================================================
// Inline Code Component
// ============================================================================

export interface InlineCodeProps {
  children: string
  /** Optional className */
  className?: string
}

export function InlineCode({ children, className }: InlineCodeProps) {
  return (
    <code
      className={cn(
        "px-1.5 py-0.5 rounded-md bg-muted text-sm font-mono text-foreground",
        className
      )}
    >
      {children}
    </code>
  )
}

// ============================================================================
// Code Diff Block (for showing file changes)
// ============================================================================

export type DiffLineType = "added" | "removed" | "neutral" | "header"

export interface DiffLine {
  content: string
  type: DiffLineType
  lineNumber?: number
}

export interface CodeDiffBlockProps {
  /** Diff lines to display */
  lines: DiffLine[]
  /** Optional filename */
  filename?: string
  /** Optional className */
  className?: string
}

const DIFF_COLORS: Record<DiffLineType, string> = {
  added: "bg-green-500/10 text-green-400",
  removed: "bg-red-500/10 text-red-400",
  neutral: "",
  header: "bg-muted text-muted-foreground font-medium",
}

export function CodeDiffBlock({ lines, filename, className }: CodeDiffBlockProps) {
  return (
    <div className={cn("my-4 rounded-lg overflow-hidden border bg-[#0d1117]", className)}>
      {/* Header */}
      {filename && (
        <div className="px-4 py-2 bg-[#161b22] border-b border-[#30363d]">
          <span className="text-sm font-medium">{filename}</span>
        </div>
      )}

      {/* Diff lines */}
      <div className="overflow-x-auto font-mono text-sm">
        {lines.map((line, i) => (
          <div
            key={i}
            className={cn(
              "flex px-4 py-0.5 leading-6",
              DIFF_COLORS[line.type],
              line.type !== "header" && "hover:bg-white/5"
            )}
          >
            {line.lineNumber !== undefined && (
              <span className="w-12 text-right text-muted-foreground select-none mr-4">
                {line.lineNumber}
              </span>
            )}
            <span className="flex-1 whitespace-pre">{line.content}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
