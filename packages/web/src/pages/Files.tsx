/**
 * Files Page
 *
 * File browser with:
 * - File search
 * - File tree view
 * - File content preview
 */

import * as React from "react"
import {
  File,
  Folder,
  FolderOpen,
  Search,
  ChevronRight,
  FileText,
  Image,
  Code,
  Archive,
  FileCode,
  Download,
  Copy,
  ExternalLink,
  Grid3x3,
  List,
  ArrowUpDown,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Input } from "@/components/ui/Input"
import { Card, CardContent } from "@/components/ui/Card"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Skeleton } from "@/components/ui/Skeleton"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/DropdownMenu"
import { api } from "@/lib/api"
import { cn } from "@/lib/utils"
import type { FileInfo } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface FileNode {
  name: string
  path: string
  type: "file" | "directory"
  children?: FileNode[]
  extension?: string
  size?: number
}

interface FileSearchResult {
  path: string
  name: string
  type: string
  extension?: string
  matches?: Array<{ line: number; content: string }>
}

// ============================================================================
// View Mode
// ============================================================================

type ViewMode = "tree" | "list" | "grid"
type SortBy = "name" | "type" | "size" | "modified"

// ============================================================================
// File Icon Component
// ============================================================================

interface FileIconProps {
  fileName: string
  fileType?: string
  isOpen?: boolean
  className?: string
}

function FileIcon({ fileName, fileType, isOpen, className }: FileIconProps) {
  const extension = fileName.split(".").pop()?.toLowerCase()

  const iconProps = { className: cn("h-4 w-4", className) }

  if (fileType === "directory") {
    return isOpen ? <FolderOpen {...iconProps} /> : <Folder {...iconProps} />
  }

  if (extension === "tsx" || extension === "ts" || extension === "js" || extension === "jsx") {
    return <Code {...iconProps} />
  }

  if (extension === "json" || extension === "md" || extension === "txt") {
    return <FileText {...iconProps} />
  }

  if (extension === "png" || extension === "jpg" || extension === "jpeg" || extension === "gif" || extension === "svg") {
    return <Image {...iconProps} />
  }

  if (extension === "zip" || extension === "tar" || extension === "gz") {
    return <Archive {...iconProps} />
  }

  return <File {...iconProps} />
}

// ============================================================================
// File Tree Item Component
// ============================================================================

interface FileTreeItemProps {
  node: FileNode
  level: number
  isSelected: boolean
  isExpanded: boolean
  onSelect: (node: FileNode) => void
  onToggle: (node: FileNode) => void
}

function FileTreeItem({
  node,
  level,
  isSelected,
  isExpanded,
  onSelect,
  onToggle,
}: FileTreeItemProps) {
  const handleClick = () => {
    if (node.type === "directory") {
      onToggle(node)
    }
    onSelect(node)
  }

  return (
    <div>
      <div
        data-testid="file-tree-item"
        className={cn(
          "flex items-center gap-2 py-1.5 px-2 rounded-md cursor-pointer transition-colors",
          "hover:bg-accent/50",
          isSelected && "bg-accent text-accent-foreground"
        )}
        style={{ paddingLeft: `${level * 16 + 8}px` }}
        onClick={handleClick}
      >
        {node.type === "directory" && (
          <ChevronRight
            data-testid="file-directory"
            className={cn(
              "h-3 w-3 shrink-0 transition-transform",
              isExpanded && "transform rotate-90"
            )}
          />
        )}
        <FileIcon
          fileName={node.name}
          fileType={node.type}
          isOpen={isExpanded}
        />
        <span className="text-sm truncate">{node.name}</span>
        {node.size && node.type === "file" && (
          <span className="ml-auto text-xs text-muted-foreground">
            {formatFileSize(node.size)}
          </span>
        )}
      </div>

      {node.type === "directory" && isExpanded && node.children && (
        <div>
          {node.children.map((child) => (
            <FileTreeItem
              key={child.path}
              node={child}
              level={level + 1}
              isSelected={false}
              isExpanded={false}
              onSelect={onSelect}
              onToggle={onToggle}
            />
          ))}
        </div>
      )}
    </div>
  )
}

// ============================================================================
// File Card Component (Grid View)
// ============================================================================

interface FileCardProps {
  file: FileSearchResult
  isSelected: boolean
  onSelect: () => void
}

function FileCard({ file, isSelected, onSelect }: FileCardProps) {
  return (
    <Card
      className={cn(
        "cursor-pointer transition-all hover:shadow-md",
        isSelected && "ring-2 ring-primary"
      )}
      onClick={onSelect}
    >
      <CardContent className="p-4">
        <div className="flex flex-col items-center text-center space-y-2">
          <div className="h-12 w-12 rounded-lg bg-primary/10 flex items-center justify-center">
            <FileIcon fileName={file.name} fileType={file.type} className="h-6 w-6 text-primary" />
          </div>
          <div className="w-full space-y-1">
            <p className="text-sm font-medium truncate">{file.name}</p>
            <p className="text-xs text-muted-foreground truncate">{file.path}</p>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// File List Item Component (List View)
// ============================================================================

interface FileListItemProps {
  file: FileSearchResult
  isSelected: boolean
  onSelect: () => void
}

function FileListItem({ file, isSelected, onSelect }: FileListItemProps) {
  return (
    <div
      className={cn(
        "flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors",
        "hover:bg-accent/50",
        isSelected && "bg-accent text-accent-foreground"
      )}
      onClick={onSelect}
    >
      <FileIcon fileName={file.name} fileType={file.type} className="h-5 w-5 shrink-0" />
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate">{file.name}</p>
        <p className="text-xs text-muted-foreground truncate">{file.path}</p>
      </div>
      <DropdownMenu>
        <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
          <Button variant="ghost" size="icon" className="h-8 w-8">
            <ExternalLink className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem>
            <Copy className="mr-2 h-4 w-4" />
            Copy path
          </DropdownMenuItem>
          <DropdownMenuItem>
            <Download className="mr-2 h-4 w-4" />
            Download
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  )
}

// ============================================================================
// File Preview Component
// ============================================================================

interface FilePreviewProps {
  file: FileSearchResult | null
  isLoading: boolean
}

function FilePreview({ file, isLoading }: FilePreviewProps) {
  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center space-y-4">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent mx-auto" />
          <p className="text-sm text-muted-foreground">Loading file...</p>
        </div>
      </div>
    )
  }

  if (!file) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center space-y-4 max-w-md">
          <div className="h-16 w-16 rounded-full bg-muted flex items-center justify-center mx-auto">
            <FileCode className="h-8 w-8 text-muted-foreground" />
          </div>
          <div className="space-y-2">
            <h3 className="font-semibold">No file selected</h3>
            <p className="text-sm text-muted-foreground">
              Select a file from the list to preview its contents
            </p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      {/* File header */}
      <div className="flex items-center justify-between p-4 border-b">
        <div className="flex items-center gap-3 min-w-0">
          <FileIcon fileName={file.name} fileType={file.type} />
          <div className="min-w-0">
            <p className="font-medium truncate">{file.name}</p>
            <p className="text-xs text-muted-foreground truncate">{file.path}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="icon" className="h-8 w-8">
            <Copy className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="icon" className="h-8 w-8">
            <Download className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* File content placeholder */}
      <ScrollArea className="flex-1">
        <div className="p-6">
          <Card>
            <CardContent className="pt-6">
              <p className="text-sm text-muted-foreground text-center py-8">
                File content preview would be displayed here.
                <br />
                <span className="text-xs">Full file reading requires backend implementation.</span>
              </p>
            </CardContent>
          </Card>
        </div>
      </ScrollArea>
    </div>
  )
}

// ============================================================================
// Utility Functions
// ============================================================================

function formatFileSize(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
}

function buildFileTree(files: FileInfo[]): FileNode[] {
  const tree: FileNode[] = []
  const map = new Map<string, FileNode>()

  // Sort files by path
  const sorted = [...files].sort((a, b) => a.path.localeCompare(b.path))

  for (const file of sorted) {
    const parts = file.path.split("/")
    let currentPath = ""
    let currentLevel = tree

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i]
      currentPath = i === 0 ? part : `${currentPath}/${part}`
      const isFile = i === parts.length - 1

      let node = map.get(currentPath)

      if (!node) {
        node = {
          name: part,
          path: currentPath,
          type: isFile ? "file" : "directory",
          children: isFile ? undefined : [],
        }
        map.set(currentPath, node)
        currentLevel.push(node)
      }

      if (node.children) {
        currentLevel = node.children
      }
    }
  }

  return tree
}

// ============================================================================
// Main Files Component
// ============================================================================

export function Files() {
  const [searchQuery, setSearchQuery] = React.useState("")
  const [viewMode, setViewMode] = React.useState<ViewMode>("list")
  const [, setSortBy] = React.useState<SortBy>("name")
  const [selectedFile, setSelectedFile] = React.useState<FileSearchResult | null>(null)
  const [expandedFolders, setExpandedFolders] = React.useState<Set<string>>(new Set())

  const [files, setFiles] = React.useState<FileInfo[]>([])
  const [isLoading, setIsLoading] = React.useState(false)
  const [isSearching, setIsSearching] = React.useState(false)

  const [searchResults, setSearchResults] = React.useState<FileSearchResult[]>([])

  // Load initial files
  React.useEffect(() => {
    loadFiles()
  }, [])

  const loadFiles = async () => {
    setIsLoading(true)
    try {
      const result = await api.findFiles()
      setFiles(result)
      setSearchResults(
        result.map((f) => ({
          path: f.path,
          name: f.name ?? f.path.split("/").pop() ?? "",
          type: f.type ?? "unknown",
        }))
      )
    } catch {
      // Handle error silently
    } finally {
      setIsLoading(false)
    }
  }

  const handleSearch = async (query: string) => {
    setSearchQuery(query)

    if (!query.trim()) {
      setSearchResults(
        files.map((f) => ({
          path: f.path,
          name: f.name ?? f.path.split("/").pop() ?? "",
          type: f.type ?? "unknown",
        }))
      )
      return
    }

    setIsSearching(true)
    try {
      const results = await api.findFiles(query)
      setSearchResults(
        results.map((f) => ({
          path: f.path,
          name: f.name ?? f.path.split("/").pop() ?? "",
          type: f.type ?? "unknown",
        }))
      )
    } catch {
      // Handle error silently
    } finally {
      setIsSearching(false)
    }
  }

  const toggleFolder = (node: FileNode) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev)
      if (next.has(node.path)) {
        next.delete(node.path)
      } else {
        next.add(node.path)
      }
      return next
    })
  }

  const fileTree = buildFileTree(files)

  return (
    <div className="flex-1 flex overflow-hidden" data-testid="file-browser">
      {/* Sidebar - File List */}
      <div className="w-80 border-r flex flex-col bg-background">
        {/* Search Header */}
        <div className="p-4 border-b space-y-3">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search files..."
              value={searchQuery}
              onChange={(e) => handleSearch(e.target.value)}
              className="pl-9"
            />
          </div>

          {/* View Mode & Sort Controls */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-1">
              <Button
                variant={viewMode === "tree" ? "default" : "ghost"}
                size="icon"
                className="h-8 w-8"
                onClick={() => setViewMode("tree")}
                title="Tree view"
              >
                <List className="h-4 w-4" />
              </Button>
              <Button
                variant={viewMode === "list" ? "default" : "ghost"}
                size="icon"
                className="h-8 w-8"
                onClick={() => setViewMode("list")}
                title="List view"
              >
                <List className="h-4 w-4" />
              </Button>
              <Button
                variant={viewMode === "grid" ? "default" : "ghost"}
                size="icon"
                className="h-8 w-8"
                onClick={() => setViewMode("grid")}
                title="Grid view"
              >
                <Grid3x3 className="h-4 w-4" />
              </Button>
            </div>

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" className="h-8 w-8">
                  <ArrowUpDown className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => setSortBy("name")}>Sort by name</DropdownMenuItem>
                <DropdownMenuItem onClick={() => setSortBy("type")}>Sort by type</DropdownMenuItem>
                <DropdownMenuItem onClick={() => setSortBy("size")}>Sort by size</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>

        {/* File List */}
        <ScrollArea className="flex-1">
          <div className="p-2">
            {isLoading ? (
              <div className="space-y-2">
                {[1, 2, 3, 4, 5].map((i) => (
                  <Skeleton key={i} className="h-10 w-full" />
                ))}
              </div>
            ) : viewMode === "tree" ? (
              <div className="space-y-0.5" data-testid="file-tree">
                {fileTree.map((node) => (
                  <FileTreeItem
                    key={node.path}
                    node={node}
                    level={0}
                    isSelected={selectedFile?.path === node.path}
                    isExpanded={expandedFolders.has(node.path)}
                    onSelect={(n) => setSelectedFile({
                      path: n.path,
                      name: n.name,
                      type: n.type,
                    })}
                    onToggle={toggleFolder}
                  />
                ))}
              </div>
            ) : viewMode === "grid" ? (
              <div className="grid grid-cols-2 gap-2">
                {searchResults.map((file) => (
                  <FileCard
                    key={file.path}
                    file={file}
                    isSelected={selectedFile?.path === file.path}
                    onSelect={() => setSelectedFile(file)}
                  />
                ))}
              </div>
            ) : (
              <div className="space-y-1">
                {searchResults.map((file) => (
                  <FileListItem
                    key={file.path}
                    file={file}
                    isSelected={selectedFile?.path === file.path}
                    onSelect={() => setSelectedFile(file)}
                  />
                ))}
              </div>
            )}

            {searchResults.length === 0 && !isLoading && (
              <div className="text-center py-8">
                <File className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
                <p className="text-sm text-muted-foreground">
                  {searchQuery ? "No files found" : "No files available"}
                </p>
              </div>
            )}
          </div>
        </ScrollArea>

        {/* Stats Footer */}
        <div className="p-3 border-t text-xs text-muted-foreground">
          {searchResults.length} file{searchResults.length !== 1 ? "s" : ""}
        </div>
      </div>

      {/* File Preview */}
      <div className="flex-1 flex flex-col">
        <FilePreview file={selectedFile} isLoading={isSearching} />
      </div>
    </div>
  )
}

export default Files
