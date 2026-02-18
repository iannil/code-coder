/**
 * DirectoryPicker Component
 *
 * A dialog-based component for browsing and selecting directories.
 * Shows a file-system-like interface for navigating directories.
 */

import * as React from "react"
import { ChevronRight, Folder, FolderOpen, Home, ArrowUp } from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/Button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/Dialog"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { api } from "@/lib/api"
import type { DirectoryEntry } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

export interface DirectoryPickerProps {
  /** Whether the dialog is open */
  open: boolean

  /** Callback when dialog open state changes */
  onOpenChange: (open: boolean) => void

  /** Callback when a directory is selected */
  onSelect: (path: string) => void

  /** Initial path to show */
  initialPath?: string

  /** Dialog title */
  title?: string

  /** Dialog description */
  description?: string
}

// ============================================================================
// Breadcrumb Component
// ============================================================================

interface BreadcrumbProps {
  path: string
  onNavigate: (path: string) => void
}

function Breadcrumb({ path, onNavigate }: BreadcrumbProps) {
  const parts = path.split("/").filter(Boolean)
  const paths: string[] = []

  return (
    <div className="flex items-center gap-1 text-sm overflow-x-auto pb-2">
      <button
        type="button"
        onClick={() => onNavigate("/")}
        className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent text-muted-foreground hover:text-foreground shrink-0"
      >
        <Home className="h-4 w-4" />
      </button>
      {parts.map((part, index) => {
        paths.push("/" + parts.slice(0, index + 1).join("/"))
        const fullPath = paths[index]

        return (
          <React.Fragment key={fullPath}>
            <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
            <button
              type="button"
              onClick={() => onNavigate(fullPath)}
              className={cn(
                "px-2 py-1 rounded hover:bg-accent truncate max-w-[150px]",
                index === parts.length - 1
                  ? "text-foreground font-medium"
                  : "text-muted-foreground hover:text-foreground",
              )}
              title={part}
            >
              {part}
            </button>
          </React.Fragment>
        )
      })}
    </div>
  )
}

// ============================================================================
// Directory Item Component
// ============================================================================

interface DirectoryItemProps {
  entry: DirectoryEntry
  isSelected: boolean
  onClick: () => void
  onDoubleClick: () => void
}

function DirectoryItem({ entry, isSelected, onClick, onDoubleClick }: DirectoryItemProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      className={cn(
        "w-full flex items-center gap-2 px-3 py-2 rounded-md text-sm text-left transition-colors",
        isSelected
          ? "bg-primary text-primary-foreground"
          : "hover:bg-accent hover:text-accent-foreground",
      )}
    >
      {isSelected ? (
        <FolderOpen className="h-4 w-4 shrink-0" />
      ) : (
        <Folder className="h-4 w-4 shrink-0" />
      )}
      <span className="truncate">{entry.name}</span>
    </button>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function DirectoryPicker({
  open,
  onOpenChange,
  onSelect,
  initialPath,
  title = "Select Directory",
  description = "Browse and select a directory for your project.",
}: DirectoryPickerProps) {
  const [currentPath, setCurrentPath] = React.useState(initialPath ?? "")
  const [directories, setDirectories] = React.useState<DirectoryEntry[]>([])
  const [parentPath, setParentPath] = React.useState<string | null>(null)
  const [selectedPath, setSelectedPath] = React.useState<string | null>(null)
  const [isLoading, setIsLoading] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)

  // Load directories when path changes
  const loadDirectories = React.useCallback(async (path?: string) => {
    setIsLoading(true)
    setError(null)

    try {
      const response = await api.listDirectories(path)
      setCurrentPath(response.path)
      setDirectories(response.directories)
      setParentPath(response.parent)
      setSelectedPath(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load directories")
      setDirectories([])
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Load directories when dialog opens
  React.useEffect(() => {
    if (open) {
      loadDirectories(initialPath)
    }
  }, [open, initialPath, loadDirectories])

  // Handle navigation
  const handleNavigate = (path: string) => {
    loadDirectories(path)
  }

  // Handle going up
  const handleGoUp = () => {
    if (parentPath) {
      loadDirectories(parentPath)
    }
  }

  // Handle directory click
  const handleDirectoryClick = (entry: DirectoryEntry) => {
    setSelectedPath(entry.path)
  }

  // Handle directory double-click (navigate into)
  const handleDirectoryDoubleClick = (entry: DirectoryEntry) => {
    loadDirectories(entry.path)
  }

  // Handle confirm selection
  const handleConfirm = () => {
    const pathToSelect = selectedPath ?? currentPath
    onSelect(pathToSelect)
    onOpenChange(false)
  }

  // Handle selecting current directory
  const handleSelectCurrent = () => {
    setSelectedPath(null)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Folder className="h-5 w-5 text-primary" />
            {title}
          </DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Breadcrumb navigation */}
          <Breadcrumb path={currentPath} onNavigate={handleNavigate} />

          {/* Directory list */}
          <div className="border rounded-lg">
            {/* Current directory row */}
            <button
              type="button"
              onClick={handleSelectCurrent}
              className={cn(
                "w-full flex items-center gap-2 px-3 py-2 text-sm text-left border-b transition-colors",
                selectedPath === null
                  ? "bg-primary/10 text-primary"
                  : "hover:bg-accent hover:text-accent-foreground",
              )}
            >
              <FolderOpen className="h-4 w-4 shrink-0" />
              <span className="font-medium">Current Directory</span>
              <span className="text-muted-foreground ml-auto truncate max-w-[200px]" title={currentPath}>
                {currentPath}
              </span>
            </button>

            {/* Parent directory row */}
            {parentPath && (
              <button
                type="button"
                onClick={handleGoUp}
                className="w-full flex items-center gap-2 px-3 py-2 text-sm text-left border-b hover:bg-accent hover:text-accent-foreground transition-colors"
              >
                <ArrowUp className="h-4 w-4 shrink-0" />
                <span className="text-muted-foreground">..</span>
              </button>
            )}

            {/* Directory scroll area */}
            <ScrollArea className="h-[300px]">
              {isLoading ? (
                <div className="flex items-center justify-center py-8">
                  <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
                </div>
              ) : error ? (
                <div className="flex items-center justify-center py-8 text-destructive text-sm">
                  {error}
                </div>
              ) : directories.length === 0 ? (
                <div className="flex items-center justify-center py-8 text-muted-foreground text-sm">
                  No subdirectories
                </div>
              ) : (
                <div className="p-2 space-y-1">
                  {directories.map((entry) => (
                    <DirectoryItem
                      key={entry.path}
                      entry={entry}
                      isSelected={selectedPath === entry.path}
                      onClick={() => handleDirectoryClick(entry)}
                      onDoubleClick={() => handleDirectoryDoubleClick(entry)}
                    />
                  ))}
                </div>
              )}
            </ScrollArea>
          </div>

          {/* Selected path display */}
          <div className="flex items-center gap-2 p-2 bg-muted rounded-md text-sm">
            <span className="text-muted-foreground">Selected:</span>
            <code className="flex-1 truncate font-mono text-xs" title={selectedPath ?? currentPath}>
              {selectedPath ?? currentPath}
            </code>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleConfirm}>Select Directory</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
