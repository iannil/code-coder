/**
 * MessageInput Component
 *
 * Input area for composing messages with:
 * - Textarea for message input
 * - File attachment button
 * - Send button
 * - Auto-resize textarea
 */

import * as React from "react"
import {
  Send,
  Paperclip,
  X,
  Loader2,
  Square,
  CornerDownLeft,
} from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "../ui/Button"
import { Textarea } from "../ui/Textarea"

// ============================================================================
// Interfaces
// ============================================================================

export interface MessageInputProps {
  onSend: (content: string, files?: File[]) => void
  onCancel?: () => void
  disabled?: boolean
  loading?: boolean
  placeholder?: string
  minLength?: number
  maxLength?: number
  className?: string
  submitOnEnter?: boolean
}

export interface AttachedFile {
  file: File
  id: string
  preview?: string
}

// ============================================================================
// File Attachment Preview
// ============================================================================

interface FileAttachmentPreviewProps {
  files: AttachedFile[]
  onRemove: (id: string) => void
  className?: string
}

function FileAttachmentPreview({ files, onRemove, className }: FileAttachmentPreviewProps) {
  if (files.length === 0) return null

  return (
    <div className={cn("flex flex-wrap gap-2", className)}>
      {files.map((attachment) => (
        <div
          key={attachment.id}
          className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-muted text-sm"
        >
          {attachment.preview ? (
            <img
              src={attachment.preview}
              alt={attachment.file.name}
              className="h-5 w-5 object-cover rounded"
            />
          ) : (
            <Paperclip className="h-4 w-4 text-muted-foreground" />
          )}
          <span className="max-w-[150px] truncate">{attachment.file.name}</span>
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5 p-0 hover:bg-destructive/20 hover:text-destructive"
            onClick={() => onRemove(attachment.id)}
          >
            <X className="h-3 w-3" />
          </Button>
        </div>
      ))}
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export const MessageInput = React.forwardRef<HTMLTextAreaElement, MessageInputProps>(
  ({
    onSend,
    onCancel,
    disabled = false,
    loading = false,
    placeholder = "Type your message...",
    minLength = 1,
    maxLength = 10000,
    className,
    submitOnEnter = true,
  }, ref) => {
  const [content, setContent] = React.useState("")
  const [attachments, setAttachments] = React.useState<AttachedFile[]>([])
  const fileInputRef = React.useRef<HTMLInputElement>(null)

  const canSend =
    !disabled &&
    !loading &&
    content.trim().length >= minLength &&
    attachments.length < 10

  // Use a ref for the textarea, combining the forwarded ref and a local ref
  const internalTextareaRef = React.useRef<HTMLTextAreaElement>(null)

  React.useEffect(() => {
    const textarea = (ref as React.RefObject<HTMLTextAreaElement>)?.current || internalTextareaRef.current
    if (!textarea) return

    const resize = () => {
      textarea.style.height = "auto"
      const newHeight = Math.min(textarea.scrollHeight, 200)
      textarea.style.height = `${newHeight}px`
    }

    resize()

    textarea.addEventListener("input", resize)
    return () => textarea.removeEventListener("input", resize)
  }, [content, ref])

  // Handle file selection
  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files ?? [])
    if (files.length === 0) return

    const newAttachments: AttachedFile[] = []

    for (const file of files) {
      // Create object URL for image preview
      let preview: string | undefined
      if (file.type.startsWith("image/")) {
        preview = URL.createObjectURL(file)
      }

      newAttachments.push({
        file,
        id: `${file.name}-${Date.now()}-${Math.random()}`,
        preview,
      })
    }

    setAttachments((prev) => [...prev, ...newAttachments])

    // Reset file input
    if (fileInputRef.current) {
      fileInputRef.current.value = ""
    }
  }

  // Handle file removal
  const handleRemoveAttachment = (id: string) => {
    setAttachments((prev) => {
      const attachment = prev.find((a) => a.id === id)
      if (attachment?.preview) {
        URL.revokeObjectURL(attachment.preview)
      }
      return prev.filter((a) => a.id !== id)
    })
  }

  // Handle send
  const handleSend = () => {
    if (!canSend) return

    const files = attachments.map((a) => a.file)
    onSend(content.trim(), files.length > 0 ? files : undefined)

    // Reset state
    setContent("")
    setAttachments([])
  }

  // Handle keyboard shortcuts
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Submit on Enter (unless Shift is held)
    if (submitOnEnter && e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      if (canSend) {
        handleSend()
      }
    }

    // Cancel on Escape
    if (e.key === "Escape" && onCancel) {
      e.preventDefault()
      setContent("")
      setAttachments([])
      onCancel()
    }
  }

  // Cleanup previews on unmount
  React.useEffect(() => {
    return () => {
      attachments.forEach((a) => {
        if (a.preview) {
          URL.revokeObjectURL(a.preview)
        }
      })
    }
  }, [attachments])

  return (
    <div className={cn("border-t bg-background p-4", className)}>
      <div className="max-w-4xl mx-auto space-y-3">
        {/* File Attachments Preview */}
        <FileAttachmentPreview
          files={attachments}
          onRemove={handleRemoveAttachment}
        />

        {/* Input Area */}
        <div className="flex items-end gap-2">
          {/* File Attach Button */}
          <Button
            variant="ghost"
            size="icon"
            className="shrink-0 h-9 w-9"
            onClick={() => fileInputRef.current?.click()}
            disabled={disabled || loading}
            type="button"
          >
            <Paperclip className="h-4 w-4" />
          </Button>
          <input
            ref={fileInputRef}
            type="file"
            className="hidden"
            multiple
            onChange={handleFileSelect}
          />

          {/* Text Input */}
          <div className="flex-1 relative">
            <Textarea
              ref={(ref as React.RefObject<HTMLTextAreaElement>) || internalTextareaRef}
              value={content}
              onChange={(e) => setContent(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={placeholder}
              disabled={disabled}
              data-testid="message-input"
              className="min-h-[44px] max-h-[200px] resize-none pr-10"
              maxLength={maxLength}
            />
            {/* Character Count */}
            {maxLength && content.length > maxLength * 0.8 && (
              <div
                className={cn(
                  "absolute bottom-2 right-2 text-xs",
                  content.length >= maxLength
                    ? "text-destructive"
                    : "text-muted-foreground"
                )}
              >
                {content.length}/{maxLength}
              </div>
            )}
          </div>

          {/* Action Buttons */}
          <div className="flex items-center gap-1 shrink-0">
            {/* Cancel Button (when loading) */}
            {loading && onCancel && (
              <Button
                variant="ghost"
                size="icon"
                className="h-9 w-9"
                onClick={onCancel}
                type="button"
              >
                <Square className="h-4 w-4" />
              </Button>
            )}

            {/* Send Button */}
            <Button
              variant={canSend ? "default" : "ghost"}
              size="icon"
              className="h-9 w-9"
              onClick={handleSend}
              disabled={!canSend}
              type="button"
              data-testid="send-btn"
            >
              {loading ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Send className="h-4 w-4" />
              )}
            </Button>
          </div>
        </div>

        {/* Footer Hints */}
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <div className="flex items-center gap-2">
            <span className="hidden sm:inline">
              <CornerDownLeft className="h-3 w-3 inline mr-1" />
              Enter to send
            </span>
            <span className="hidden sm:inline">
              Shift + Enter for new line
            </span>
          </div>
          {attachments.length >= 10 && (
            <span className="text-destructive">Maximum 10 files</span>
          )}
        </div>
      </div>
    </div>
  )
})

MessageInput.displayName = "MessageInput"
