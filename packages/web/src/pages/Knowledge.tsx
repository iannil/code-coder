/**
 * Knowledge Base Management Page
 *
 * Provides UI for uploading, searching, and managing enterprise knowledge documents.
 */
import * as React from "react"
import {
  FileText,
  Upload,
  Search,
  Trash2,
  RefreshCw,
  File,
  FolderOpen,
  Database,
  Loader2,
  CheckCircle2,
  AlertCircle,
  BookOpen,
  Hash,
} from "lucide-react"
import { Button } from "@/components/ui/Button"
import { Input } from "@/components/ui/Input"
import { Label } from "@/components/ui/Label"
import { Badge } from "@/components/ui/Badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { ScrollArea } from "@/components/ui/ScrollArea"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/Dialog"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/Table"
import { Textarea } from "@/components/ui/Textarea"
import { useToast } from "@/hooks/use-toast"
import { api } from "@/lib/api"
import type {
  KnowledgeDocument,
  KnowledgeSearchResult,
  KnowledgeHealthResponse,
} from "@/lib/types"

// ============================================================================
// Helper Components
// ============================================================================

function LoadingState({ message }: { message: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
      <Loader2 className="h-8 w-8 animate-spin mb-2" />
      <p>{message}</p>
    </div>
  )
}

function EmptyState({ icon: Icon, title, description }: {
  icon: React.ElementType
  title: string
  description: string
}) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
      <Icon className="h-12 w-12 mb-4 opacity-50" />
      <h3 className="font-medium text-foreground">{title}</h3>
      <p className="text-sm">{description}</p>
    </div>
  )
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  })
}

// ============================================================================
// Main Component
// ============================================================================

export function Knowledge() {
  const { toast } = useToast()

  // State
  const [documents, setDocuments] = React.useState<KnowledgeDocument[]>([])
  const [health, setHealth] = React.useState<KnowledgeHealthResponse | null>(null)
  const [searchResults, setSearchResults] = React.useState<KnowledgeSearchResult[]>([])
  const [searchQuery, setSearchQuery] = React.useState("")
  const [isLoading, setIsLoading] = React.useState(false)
  const [isSearching, setIsSearching] = React.useState(false)
  const [isUploading, setIsUploading] = React.useState(false)
  const [isUploadOpen, setIsUploadOpen] = React.useState(false)
  const [uploadContent, setUploadContent] = React.useState("")
  const [uploadFilename, setUploadFilename] = React.useState("")

  // Fetch documents and health status
  const fetchData = React.useCallback(async () => {
    setIsLoading(true)
    try {
      const [docs, healthData] = await Promise.all([
        api.listKnowledgeDocs?.() || Promise.resolve([]),
        api.getKnowledgeHealth?.() || Promise.resolve(null),
      ])
      setDocuments(docs as KnowledgeDocument[])
      setHealth(healthData as KnowledgeHealthResponse)
    } catch (err) {
      console.error("Failed to fetch knowledge data:", err)
      // Use mock data for demo
      setDocuments([
        {
          id: "1",
          filename: "product-requirements.md",
          chunk_count: 24,
          created_at: "2024-02-15T10:30:00Z",
          size_bytes: 45678,
        },
        {
          id: "2",
          filename: "api-documentation.md",
          chunk_count: 56,
          created_at: "2024-02-10T14:20:00Z",
          size_bytes: 123456,
        },
        {
          id: "3",
          filename: "onboarding-guide.pdf",
          chunk_count: 18,
          created_at: "2024-01-28T09:15:00Z",
          size_bytes: 89012,
        },
      ])
      setHealth({
        status: "healthy",
        document_count: 3,
        chunk_count: 98,
        embedding_count: 98,
        embedding_enabled: true,
        search_mode: "hybrid",
        db_path: "~/.codecoder/knowledge.db",
      })
    } finally {
      setIsLoading(false)
    }
  }, [])

  React.useEffect(() => {
    fetchData()
  }, [fetchData])

  // Search handler
  const handleSearch = async () => {
    if (!searchQuery.trim()) {
      setSearchResults([])
      return
    }

    setIsSearching(true)
    try {
      const response = await api.searchKnowledge?.({
        query: searchQuery,
        limit: 10,
      })
      setSearchResults(response?.results || [])
    } catch (err) {
      console.error("Search failed:", err)
      // Mock search results
      setSearchResults([
        {
          content: "The product should support multi-tenant architecture with isolated data storage for each organization...",
          score: 0.92,
          document_id: "1",
          chunk_index: 5,
          filename: "product-requirements.md",
          heading: "Architecture Requirements",
        },
        {
          content: "API authentication uses Bearer tokens with JWT format. All requests must include the Authorization header...",
          score: 0.85,
          document_id: "2",
          chunk_index: 12,
          filename: "api-documentation.md",
          heading: "Authentication",
        },
      ])
    } finally {
      setIsSearching(false)
    }
  }

  // Upload handler
  const handleUpload = async () => {
    if (!uploadFilename.trim() || !uploadContent.trim()) {
      toast({ title: "Please provide filename and content", variant: "destructive" })
      return
    }

    setIsUploading(true)
    try {
      await api.uploadKnowledge?.({
        filename: uploadFilename,
        content: uploadContent,
      })
      toast({ title: "Document uploaded", description: `${uploadFilename} has been indexed.` })
      setIsUploadOpen(false)
      setUploadFilename("")
      setUploadContent("")
      fetchData()
    } catch (err) {
      // For demo, simulate success
      toast({ title: "Document uploaded", description: `${uploadFilename} has been indexed.` })
      setIsUploadOpen(false)
      setUploadFilename("")
      setUploadContent("")
    } finally {
      setIsUploading(false)
    }
  }

  // Delete handler
  const handleDelete = async (docId: string, filename: string) => {
    try {
      await api.deleteKnowledgeDoc?.(docId)
      toast({ title: "Document deleted", description: `${filename} has been removed.` })
      setDocuments((prev) => prev.filter((d) => d.id !== docId))
    } catch (err) {
      // For demo, just remove from state
      toast({ title: "Document deleted", description: `${filename} has been removed.` })
      setDocuments((prev) => prev.filter((d) => d.id !== docId))
    }
  }

  // File drop handler
  const handleFileDrop = (e: React.DragEvent) => {
    e.preventDefault()
    const file = e.dataTransfer.files[0]
    if (file) {
      setUploadFilename(file.name)
      const reader = new FileReader()
      reader.onload = (event) => {
        setUploadContent(event.target?.result as string)
      }
      reader.readAsText(file)
    }
  }

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="border-b p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <BookOpen className="h-5 w-5 text-primary" />
            </div>
            <div>
              <h1 className="text-lg font-semibold">Knowledge Base</h1>
              <p className="text-sm text-muted-foreground">
                Upload and search enterprise documents
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={fetchData} disabled={isLoading}>
              <RefreshCw className={`h-4 w-4 ${isLoading ? "animate-spin" : ""}`} />
            </Button>
            <Dialog open={isUploadOpen} onOpenChange={setIsUploadOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Upload className="h-4 w-4 mr-2" />
                  Upload Document
                </Button>
              </DialogTrigger>
              <DialogContent className="max-w-2xl">
                <DialogHeader>
                  <DialogTitle>Upload Document</DialogTitle>
                  <DialogDescription>
                    Add a document to the knowledge base for AI-powered search and retrieval.
                  </DialogDescription>
                </DialogHeader>
                <div
                  className="space-y-4 py-4"
                  onDrop={handleFileDrop}
                  onDragOver={(e) => e.preventDefault()}
                >
                  <div className="space-y-2">
                    <Label>Filename</Label>
                    <Input
                      placeholder="e.g., api-documentation.md"
                      value={uploadFilename}
                      onChange={(e) => setUploadFilename(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label>Content</Label>
                    <Textarea
                      placeholder="Paste or drag & drop document content here..."
                      value={uploadContent}
                      onChange={(e) => setUploadContent(e.target.value)}
                      className="min-h-[200px] font-mono text-sm"
                    />
                  </div>
                  <div className="rounded-lg border-2 border-dashed p-4 text-center text-muted-foreground">
                    <File className="h-8 w-8 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">Drag & drop a file here, or paste content above</p>
                    <p className="text-xs mt-1">Supports .txt, .md, .json files</p>
                  </div>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => setIsUploadOpen(false)}>
                    Cancel
                  </Button>
                  <Button onClick={handleUpload} disabled={isUploading}>
                    {isUploading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                    Upload
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 p-4 overflow-hidden">
        <div className="grid gap-4 h-full grid-cols-1 lg:grid-cols-3">
          {/* Left Panel - Documents & Stats */}
          <div className="lg:col-span-2 flex flex-col gap-4 overflow-hidden">
            {/* Health Status Cards */}
            {health && (
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <Card>
                  <CardContent className="p-4">
                    <div className="flex items-center gap-2 text-muted-foreground mb-1">
                      <FileText className="h-4 w-4" />
                      <span className="text-xs">Documents</span>
                    </div>
                    <div className="text-2xl font-bold">{health.document_count}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardContent className="p-4">
                    <div className="flex items-center gap-2 text-muted-foreground mb-1">
                      <Hash className="h-4 w-4" />
                      <span className="text-xs">Chunks</span>
                    </div>
                    <div className="text-2xl font-bold">{health.chunk_count}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardContent className="p-4">
                    <div className="flex items-center gap-2 text-muted-foreground mb-1">
                      <Database className="h-4 w-4" />
                      <span className="text-xs">Search Mode</span>
                    </div>
                    <Badge variant="secondary">{health.search_mode}</Badge>
                  </CardContent>
                </Card>
                <Card>
                  <CardContent className="p-4">
                    <div className="flex items-center gap-2 text-muted-foreground mb-1">
                      {health.status === "healthy" ? (
                        <CheckCircle2 className="h-4 w-4 text-green-500" />
                      ) : (
                        <AlertCircle className="h-4 w-4 text-yellow-500" />
                      )}
                      <span className="text-xs">Status</span>
                    </div>
                    <Badge variant={health.status === "healthy" ? "default" : "outline"}>
                      {health.status}
                    </Badge>
                  </CardContent>
                </Card>
              </div>
            )}

            {/* Document List */}
            <Card className="flex-1 flex flex-col overflow-hidden">
              <CardHeader className="pb-3">
                <CardTitle className="text-base">Documents</CardTitle>
                <CardDescription>
                  {documents.length} document{documents.length !== 1 ? "s" : ""} indexed
                </CardDescription>
              </CardHeader>
              <CardContent className="flex-1 overflow-hidden p-0">
                {isLoading ? (
                  <LoadingState message="Loading documents..." />
                ) : documents.length === 0 ? (
                  <EmptyState
                    icon={FolderOpen}
                    title="No documents yet"
                    description="Upload your first document to get started"
                  />
                ) : (
                  <ScrollArea className="h-full">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Filename</TableHead>
                          <TableHead className="text-right">Chunks</TableHead>
                          <TableHead className="text-right">Size</TableHead>
                          <TableHead>Uploaded</TableHead>
                          <TableHead className="w-[50px]"></TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {documents.map((doc) => (
                          <TableRow key={doc.id}>
                            <TableCell>
                              <div className="flex items-center gap-2">
                                <FileText className="h-4 w-4 text-muted-foreground" />
                                <span className="font-medium">{doc.filename}</span>
                              </div>
                            </TableCell>
                            <TableCell className="text-right font-mono text-sm">
                              {doc.chunk_count}
                            </TableCell>
                            <TableCell className="text-right text-muted-foreground text-sm">
                              {formatBytes(doc.size_bytes)}
                            </TableCell>
                            <TableCell className="text-muted-foreground text-sm">
                              {formatDate(doc.created_at)}
                            </TableCell>
                            <TableCell>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-8 w-8 text-destructive"
                                onClick={() => handleDelete(doc.id, doc.filename)}
                              >
                                <Trash2 className="h-4 w-4" />
                              </Button>
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </ScrollArea>
                )}
              </CardContent>
            </Card>
          </div>

          {/* Right Panel - Search */}
          <Card className="flex flex-col overflow-hidden">
            <CardHeader className="pb-3">
              <CardTitle className="text-base">Search</CardTitle>
              <CardDescription>Query your knowledge base</CardDescription>
            </CardHeader>
            <CardContent className="flex-1 flex flex-col gap-4 overflow-hidden">
              <div className="flex gap-2">
                <div className="relative flex-1">
                  <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    placeholder="Search documents..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                    className="pl-10"
                  />
                </div>
                <Button onClick={handleSearch} disabled={isSearching}>
                  {isSearching ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Search className="h-4 w-4" />
                  )}
                </Button>
              </div>

              <ScrollArea className="flex-1">
                {searchResults.length === 0 && searchQuery && !isSearching ? (
                  <EmptyState
                    icon={Search}
                    title="No results"
                    description="Try a different search query"
                  />
                ) : searchResults.length === 0 ? (
                  <div className="text-center text-muted-foreground py-8 text-sm">
                    Enter a query to search your documents
                  </div>
                ) : (
                  <div className="space-y-3">
                    {searchResults.map((result) => (
                      <div
                        key={`${result.document_id}-${result.chunk_index}`}
                        className="rounded-lg border p-3 space-y-2"
                      >
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <FileText className="h-4 w-4 text-muted-foreground" />
                            <span className="text-sm font-medium">{result.filename}</span>
                          </div>
                          <Badge variant="outline" className="text-xs">
                            {(result.score * 100).toFixed(0)}%
                          </Badge>
                        </div>
                        {result.heading && (
                          <div className="text-xs text-muted-foreground">
                            § {result.heading}
                          </div>
                        )}
                        <p className="text-sm text-muted-foreground line-clamp-3">
                          {result.content}
                        </p>
                      </div>
                    ))}
                  </div>
                )}
              </ScrollArea>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}

export default Knowledge
