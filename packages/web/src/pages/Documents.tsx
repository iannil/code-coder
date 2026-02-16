/**
 * Documents Page
 *
 * Main document/writing system interface:
 * - Document list with create/delete
 * - Document detail view with chapters
 * - Chapter content editing
 * - Entity management
 * - Statistics overview
 */

import * as React from "react"
import {
  BookOpen,
  FileText,
  Plus,
  Trash2,
  Users,
  BarChart3,
  Edit3,
  Check,
  X,
  RefreshCw,
  Download,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Input } from "@/components/ui/Input"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { Skeleton } from "@/components/ui/Skeleton"
import { Badge } from "@/components/ui/Badge"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/AlertDialog"
import {
  useDocumentStore,
  useDocuments,
  useSelectedDocument,
  useDocumentChapters,
  useSelectedChapter,
  useDocumentEntities,
  useDocumentStats,
  useDocumentsLoading,
} from "@/stores"
import { cn } from "@/lib/utils"
import type {
  DocumentEntity,
  ChapterStatus,
  EntityType,
} from "@/lib/types"

// ============================================================================
// Status Badge Component
// ============================================================================

type BadgeVariant = "info" | "warning" | "purple" | "success" | "secondary"

function StatusBadge({ status }: { status: string }) {
  const configs: Record<string, { label: string; variant: BadgeVariant }> = {
    planning: { label: "Planning", variant: "info" },
    writing: { label: "Writing", variant: "warning" },
    reviewing: { label: "Reviewing", variant: "purple" },
    completed: { label: "Completed", variant: "success" },
    pending: { label: "Pending", variant: "secondary" },
    drafting: { label: "Drafting", variant: "warning" },
    revision: { label: "Revision", variant: "purple" },
  }

  const config = configs[status] ?? { label: status, variant: "secondary" as BadgeVariant }

  return (
    <Badge variant={config.variant}>
      {config.label}
    </Badge>
  )
}

// ============================================================================
// Document List Component
// ============================================================================

function DocumentList() {
  const documents = useDocuments()
  const isLoading = useDocumentsLoading()
  const selectedDocument = useSelectedDocument()
  const { fetchDocuments, selectDocument, createDocument, deleteDocument } = useDocumentStore()
  const [showCreate, setShowCreate] = React.useState(false)
  const [newTitle, setNewTitle] = React.useState("")
  const [newTarget, setNewTarget] = React.useState(50000)
  const [creating, setCreating] = React.useState(false)
  const [docToDelete, setDocToDelete] = React.useState<string | null>(null)

  React.useEffect(() => {
    fetchDocuments()
  }, [fetchDocuments])

  const handleCreate = async () => {
    if (!newTitle.trim()) return
    setCreating(true)
    try {
      const doc = await createDocument({ title: newTitle, targetWords: newTarget })
      await selectDocument(doc.id)
      setShowCreate(false)
      setNewTitle("")
      setNewTarget(50000)
    } finally {
      setCreating(false)
    }
  }

  const handleDelete = async (id: string) => {
    await deleteDocument(id)
    setDocToDelete(null)
  }

  if (isLoading) {
    return (
      <div className="space-y-2 p-4">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-16 w-full" />
        ))}
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between p-4 border-b">
        <h2 className="font-semibold flex items-center gap-2">
          <BookOpen className="h-4 w-4" />
          Documents
        </h2>
        <Button size="sm" variant="ghost" onClick={() => setShowCreate(!showCreate)}>
          <Plus className="h-4 w-4" />
        </Button>
      </div>

      {showCreate && (
        <div className="p-4 border-b space-y-3">
          <Input
            placeholder="Document title"
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
          />
          <div className="flex items-center gap-2">
            <Input
              type="number"
              placeholder="Target words"
              value={newTarget}
              onChange={(e) => setNewTarget(Number(e.target.value))}
              className="w-32"
            />
            <span className="text-sm text-muted-foreground">words</span>
          </div>
          <div className="flex gap-2">
            <Button size="sm" onClick={handleCreate} disabled={creating || !newTitle.trim()}>
              {creating ? <RefreshCw className="h-4 w-4 animate-spin" /> : <Check className="h-4 w-4" />}
              Create
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setShowCreate(false)}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>
      )}

      <div className="flex-1 overflow-y-auto">
        {documents.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-4">
            <BookOpen className="h-12 w-12 mb-4" />
            <p>No documents yet</p>
            <Button size="sm" variant="outline" className="mt-2" onClick={() => setShowCreate(true)}>
              <Plus className="h-4 w-4 mr-1" />
              Create first document
            </Button>
          </div>
        ) : (
          <div className="divide-y">
            {documents.map((doc) => (
              <button
                key={doc.id}
                onClick={() => selectDocument(doc.id)}
                className={cn(
                  "w-full text-left p-4 hover:bg-muted/50 transition-colors",
                  selectedDocument?.id === doc.id && "bg-muted"
                )}
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <h3 className="font-medium truncate">{doc.title}</h3>
                    <div className="flex items-center gap-2 mt-1">
                      <StatusBadge status={doc.status} />
                      <span className="text-xs text-muted-foreground">
                        {doc.currentWords.toLocaleString()} / {doc.targetWords.toLocaleString()}
                      </span>
                    </div>
                  </div>
                  <AlertDialog open={docToDelete === doc.id} onOpenChange={(open) => !open && setDocToDelete(null)}>
                    <AlertDialogTrigger asChild>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-6 w-6 text-muted-foreground hover:text-destructive"
                        onClick={(e) => {
                          e.stopPropagation()
                          setDocToDelete(doc.id)
                        }}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </AlertDialogTrigger>
                    <AlertDialogContent>
                      <AlertDialogHeader>
                        <AlertDialogTitle>Delete document?</AlertDialogTitle>
                        <AlertDialogDescription>
                          Are you sure you want to delete "{doc.title}"? This action cannot be undone.
                        </AlertDialogDescription>
                      </AlertDialogHeader>
                      <AlertDialogFooter>
                        <AlertDialogCancel>Cancel</AlertDialogCancel>
                        <AlertDialogAction onClick={() => handleDelete(doc.id)}>
                          Delete
                        </AlertDialogAction>
                      </AlertDialogFooter>
                    </AlertDialogContent>
                  </AlertDialog>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Chapter List Component
// ============================================================================

function ChapterList() {
  const chapters = useDocumentChapters()
  const selectedChapter = useSelectedChapter()
  const { selectChapter } = useDocumentStore()

  const getStatusIcon = (status: ChapterStatus) => {
    switch (status) {
      case "completed":
        return <Check className="h-3 w-3 text-green-500" />
      case "drafting":
        return <Edit3 className="h-3 w-3 text-amber-500" />
      case "revision":
        return <RefreshCw className="h-3 w-3 text-purple-500" />
      default:
        return <FileText className="h-3 w-3 text-gray-400" />
    }
  }

  if (chapters.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-4">
        <FileText className="h-8 w-8 mb-2" />
        <p className="text-sm">No chapters in outline</p>
      </div>
    )
  }

  return (
    <div className="divide-y">
      {chapters.map((chapter, index) => (
        <button
          key={chapter.id}
          onClick={() => selectChapter(chapter.id)}
          className={cn(
            "w-full text-left p-3 hover:bg-muted/50 transition-colors",
            selectedChapter?.id === chapter.id && "bg-muted"
          )}
        >
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground w-6">{index + 1}.</span>
            {getStatusIcon(chapter.status)}
            <span className="flex-1 truncate text-sm">{chapter.title}</span>
            <span className="text-xs text-muted-foreground">
              {chapter.wordCount.toLocaleString()}
            </span>
          </div>
        </button>
      ))}
    </div>
  )
}

// ============================================================================
// Chapter Content Component
// ============================================================================

function ChapterContent() {
  const selectedDocument = useSelectedDocument()
  const selectedChapter = useSelectedChapter()
  const { updateChapter } = useDocumentStore()
  const [editing, setEditing] = React.useState(false)
  const [content, setContent] = React.useState("")

  React.useEffect(() => {
    if (selectedChapter) {
      setContent(selectedChapter.content)
    }
  }, [selectedChapter])

  const handleSave = async () => {
    if (!selectedDocument || !selectedChapter) return
    await updateChapter(selectedDocument.id, selectedChapter.id, { content })
    setEditing(false)
  }

  if (!selectedChapter) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
        <FileText className="h-12 w-12 mb-4" />
        <p>Select a chapter to view content</p>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between p-4 border-b">
        <div>
          <h3 className="font-semibold">{selectedChapter.title}</h3>
          <div className="flex items-center gap-2 mt-1">
            <StatusBadge status={selectedChapter.status} />
            <span className="text-sm text-muted-foreground">
              {selectedChapter.wordCount.toLocaleString()} words
            </span>
          </div>
        </div>
        <div className="flex gap-2">
          {editing ? (
            <>
              <Button size="sm" onClick={handleSave}>
                <Check className="h-4 w-4 mr-1" />
                Save
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setEditing(false)}>
                <X className="h-4 w-4" />
              </Button>
            </>
          ) : (
            <Button size="sm" variant="outline" onClick={() => setEditing(true)}>
              <Edit3 className="h-4 w-4 mr-1" />
              Edit
            </Button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {editing ? (
          <textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            className="w-full h-full min-h-[400px] p-4 bg-muted rounded-md font-mono text-sm resize-none focus:outline-none focus:ring-2 focus:ring-primary"
          />
        ) : (
          <div className="prose prose-sm max-w-none dark:prose-invert">
            {selectedChapter.content ? (
              <pre className="whitespace-pre-wrap font-sans">{selectedChapter.content}</pre>
            ) : (
              <p className="text-muted-foreground italic">No content yet</p>
            )}
          </div>
        )}

        {selectedChapter.summary && (
          <div className="mt-6 p-4 bg-muted rounded-md">
            <h4 className="text-sm font-medium mb-2">Chapter Summary</h4>
            <p className="text-sm text-muted-foreground">{selectedChapter.summary}</p>
          </div>
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Entity List Component
// ============================================================================

function EntityList() {
  const entities = useDocumentEntities()

  const getEntityIcon = (type: EntityType) => {
    switch (type) {
      case "character":
        return "👤"
      case "location":
        return "📍"
      case "concept":
        return "💡"
      case "item":
        return "📦"
      case "event":
        return "📅"
      default:
        return "📝"
    }
  }

  const groupedEntities = entities.reduce(
    (acc, entity) => {
      if (!acc[entity.type]) acc[entity.type] = []
      acc[entity.type].push(entity)
      return acc
    },
    {} as Record<EntityType, DocumentEntity[]>
  )

  if (entities.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-4">
        <Users className="h-12 w-12 mb-4" />
        <p>No entities defined</p>
        <p className="text-sm">Entities are extracted from chapters</p>
      </div>
    )
  }

  return (
    <div className="space-y-6 p-4">
      {(Object.entries(groupedEntities) as [EntityType, DocumentEntity[]][]).map(
        ([type, typeEntities]) => (
          <div key={type}>
            <h3 className="text-sm font-medium mb-2 flex items-center gap-2">
              <span>{getEntityIcon(type)}</span>
              <span className="capitalize">{type}s</span>
              <span className="text-muted-foreground">({typeEntities.length})</span>
            </h3>
            <div className="space-y-2">
              {typeEntities.map((entity) => (
                <Card key={entity.id}>
                  <CardContent className="p-3">
                    <div className="flex items-start justify-between">
                      <div>
                        <h4 className="font-medium">{entity.name}</h4>
                        {entity.aliases.length > 0 && (
                          <p className="text-xs text-muted-foreground">
                            Also known as: {entity.aliases.join(", ")}
                          </p>
                        )}
                      </div>
                    </div>
                    <p className="text-sm text-muted-foreground mt-2 line-clamp-2">
                      {entity.description}
                    </p>
                  </CardContent>
                </Card>
              ))}
            </div>
          </div>
        )
      )}
    </div>
  )
}

// ============================================================================
// Stats Panel Component
// ============================================================================

function StatsPanel() {
  const stats = useDocumentStats()
  const selectedDocument = useSelectedDocument()

  if (!stats || !selectedDocument) {
    return (
      <div className="flex items-center justify-center h-full">
        <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  const progressPercent = Math.round(stats.progress * 100)

  return (
    <div className="p-4 space-y-6">
      {/* Progress */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Progress</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            <div className="flex justify-between text-sm">
              <span>{stats.totalWords.toLocaleString()} words</span>
              <span className="text-muted-foreground">
                of {stats.targetWords.toLocaleString()}
              </span>
            </div>
            <div className="h-2 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-primary transition-all"
                style={{ width: `${Math.min(progressPercent, 100)}%` }}
              />
            </div>
            <p className="text-sm text-muted-foreground text-center">
              {progressPercent}% complete
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Chapters */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Chapters</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-2xl font-bold">{stats.completedChapters}</p>
              <p className="text-sm text-muted-foreground">Completed</p>
            </div>
            <div>
              <p className="text-2xl font-bold">{stats.pendingChapters}</p>
              <p className="text-sm text-muted-foreground">Pending</p>
            </div>
          </div>
          <div className="mt-4">
            <p className="text-sm text-muted-foreground">
              {stats.totalChapters} total chapters
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Remaining */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Estimated Remaining</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-2xl font-bold">
            {stats.estimatedRemaining.toLocaleString()}
          </p>
          <p className="text-sm text-muted-foreground">words to write</p>
        </CardContent>
      </Card>

      {/* Actions */}
      <div className="flex gap-2">
        <Button variant="outline" className="flex-1">
          <Download className="h-4 w-4 mr-2" />
          Export
        </Button>
      </div>
    </div>
  )
}

// ============================================================================
// Document Detail Component
// ============================================================================

function DocumentDetail() {
  const selectedDocument = useSelectedDocument()

  if (!selectedDocument) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
        <BookOpen className="h-16 w-16 mb-4" />
        <p className="text-lg">Select a document to view</p>
        <p className="text-sm">Or create a new one from the sidebar</p>
      </div>
    )
  }

  return (
    <div className="flex h-full">
      {/* Chapter List */}
      <div className="w-64 border-r flex flex-col">
        <div className="p-4 border-b">
          <h2 className="font-semibold truncate">{selectedDocument.title}</h2>
          <div className="flex items-center gap-2 mt-1">
            <StatusBadge status={selectedDocument.status} />
          </div>
        </div>
        <Tabs defaultValue="chapters" className="flex-1 flex flex-col">
          <TabsList className="mx-4 mt-2">
            <TabsTrigger value="chapters" className="flex-1">
              <FileText className="h-3 w-3 mr-1" />
              Chapters
            </TabsTrigger>
            <TabsTrigger value="entities" className="flex-1">
              <Users className="h-3 w-3 mr-1" />
              Entities
            </TabsTrigger>
          </TabsList>
          <TabsContent value="chapters" className="flex-1 overflow-y-auto m-0">
            <ChapterList />
          </TabsContent>
          <TabsContent value="entities" className="flex-1 overflow-y-auto m-0">
            <EntityList />
          </TabsContent>
        </Tabs>
      </div>

      {/* Chapter Content */}
      <div className="flex-1 flex flex-col">
        <ChapterContent />
      </div>

      {/* Stats Sidebar */}
      <div className="w-72 border-l overflow-y-auto">
        <div className="p-4 border-b">
          <h3 className="font-semibold flex items-center gap-2">
            <BarChart3 className="h-4 w-4" />
            Statistics
          </h3>
        </div>
        <StatsPanel />
      </div>
    </div>
  )
}

// ============================================================================
// Main Documents Page
// ============================================================================

export function Documents() {
  return (
    <div className="flex h-full">
      {/* Document List Sidebar */}
      <div className="w-72 border-r flex flex-col bg-background">
        <DocumentList />
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-hidden">
        <DocumentDetail />
      </div>
    </div>
  )
}

export default Documents
