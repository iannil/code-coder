/**
 * Memory Panel Component
 *
 * Displays and manages the memory system:
 * - Daily notes (flow layer)
 * - Long-term memory (sediment layer)
 * - Consolidation controls
 */

import * as React from "react"
import {
  Brain,
  Calendar,
  FileText,
  RefreshCw,
  ChevronRight,
  Clock,
  BookOpen,
  Layers,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { Skeleton } from "@/components/ui/Skeleton"
import {
  useMemoryStore,
  useDailyDates,
  useSelectedDate,
  useDailyEntries,
  useLongTermContent,
  useMemorySections,
  useConsolidationStats,
  useMemorySummary,
} from "@/stores"
import { cn } from "@/lib/utils"

// ============================================================================
// Daily Notes Component
// ============================================================================

function DailyNotes() {
  const dates = useDailyDates()
  const selectedDate = useSelectedDate()
  const entries = useDailyEntries()
  const { fetchDailyDates, selectDate } = useMemoryStore()
  const [isLoading, setIsLoading] = React.useState(false)

  React.useEffect(() => {
    setIsLoading(true)
    fetchDailyDates().finally(() => setIsLoading(false))
  }, [fetchDailyDates])

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr)
    return date.toLocaleDateString("zh-CN", {
      year: "numeric",
      month: "short",
      day: "numeric",
    })
  }

  return (
    <div className="flex gap-4 h-[500px]">
      {/* Date List */}
      <div className="w-48 border-r pr-4 overflow-y-auto">
        <div className="flex items-center gap-2 mb-3">
          <Calendar className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">Dates</span>
        </div>
        {isLoading ? (
          <div className="space-y-2">
            {[1, 2, 3].map((i) => (
              <Skeleton key={i} className="h-8 w-full" />
            ))}
          </div>
        ) : dates.length === 0 ? (
          <p className="text-sm text-muted-foreground">No daily notes yet</p>
        ) : (
          <div className="space-y-1">
            {dates.map((date) => (
              <button
                key={date}
                onClick={() => selectDate(date)}
                className={cn(
                  "w-full text-left px-3 py-2 rounded-md text-sm transition-colors",
                  selectedDate === date
                    ? "bg-primary text-primary-foreground"
                    : "hover:bg-muted"
                )}
              >
                {formatDate(date)}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-y-auto">
        {!selectedDate ? (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
            <FileText className="h-12 w-12 mb-4" />
            <p>Select a date to view notes</p>
          </div>
        ) : entries.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
            <FileText className="h-12 w-12 mb-4" />
            <p>No entries for this date</p>
          </div>
        ) : (
          <div className="space-y-3">
            <h3 className="font-medium">{formatDate(selectedDate)}</h3>
            {entries.map((entry, index) => (
              <Card key={index}>
                <CardContent className="p-4">
                  <pre className="text-sm whitespace-pre-wrap font-mono">{entry}</pre>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Long-term Memory Component
// ============================================================================

function LongTermMemory() {
  const content = useLongTermContent()
  const sections = useMemorySections()
  const { fetchLongTermMemory, fetchSections } = useMemoryStore()
  const [isLoading, setIsLoading] = React.useState(false)
  const [selectedSection, setSelectedSection] = React.useState<string | null>(null)

  React.useEffect(() => {
    setIsLoading(true)
    Promise.all([fetchLongTermMemory(), fetchSections()]).finally(() =>
      setIsLoading(false)
    )
  }, [fetchLongTermMemory, fetchSections])

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case "user_preferences":
        return "👤"
      case "project_context":
        return "📁"
      case "key_decisions":
        return "🎯"
      case "lessons_learned":
        return "💡"
      default:
        return "📝"
    }
  }

  const getCategoryLabel = (category: string) => {
    switch (category) {
      case "user_preferences":
        return "User Preferences"
      case "project_context":
        return "Project Context"
      case "key_decisions":
        return "Key Decisions"
      case "lessons_learned":
        return "Lessons Learned"
      default:
        return category
    }
  }

  if (isLoading) {
    return (
      <div className="space-y-4">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-24 w-full" />
        ))}
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {sections.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <Brain className="h-12 w-12 mb-4" />
          <p>No long-term memory entries yet</p>
          <p className="text-sm">Memories will be consolidated from daily notes</p>
        </div>
      ) : (
        sections.map((section) => (
          <Card key={section.category}>
            <CardHeader
              className="cursor-pointer"
              onClick={() =>
                setSelectedSection(
                  selectedSection === section.category ? null : section.category
                )
              }
            >
              <div className="flex items-center justify-between">
                <CardTitle className="text-base flex items-center gap-2">
                  <span>{getCategoryIcon(section.category)}</span>
                  {getCategoryLabel(section.category)}
                </CardTitle>
                <ChevronRight
                  className={cn(
                    "h-4 w-4 transition-transform",
                    selectedSection === section.category && "rotate-90"
                  )}
                />
              </div>
            </CardHeader>
            {selectedSection === section.category && (
              <CardContent>
                <pre className="text-sm whitespace-pre-wrap font-mono bg-muted p-4 rounded-md">
                  {section.content}
                </pre>
              </CardContent>
            )}
          </Card>
        ))
      )}

      {content && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <BookOpen className="h-4 w-4" />
              Raw Content
            </CardTitle>
          </CardHeader>
          <CardContent>
            <pre className="text-sm whitespace-pre-wrap font-mono bg-muted p-4 rounded-md max-h-64 overflow-y-auto">
              {content}
            </pre>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

// ============================================================================
// Consolidation Panel Component
// ============================================================================

function ConsolidationPanel() {
  const stats = useConsolidationStats()
  const summary = useMemorySummary()
  const consolidating = useMemoryStore((s) => s.consolidating)
  const fetchConsolidationStats = useMemoryStore((s) => s.fetchConsolidationStats)
  const fetchSummary = useMemoryStore((s) => s.fetchSummary)
  const triggerConsolidation = useMemoryStore((s) => s.triggerConsolidation)

  // Track initialization to avoid infinite loop
  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true

    fetchConsolidationStats()
    fetchSummary()
  }, [fetchConsolidationStats, fetchSummary])

  const formatTimestamp = (ts?: number) => {
    if (!ts) return "Never"
    return new Date(ts).toLocaleString("zh-CN")
  }

  return (
    <div className="space-y-6">
      {/* Stats */}
      <div className="grid grid-cols-3 gap-4">
        <Card>
          <CardContent className="p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-full bg-blue-500/10">
                <Layers className="h-5 w-5 text-blue-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Daily Notes</p>
                <p className="text-2xl font-bold">{summary?.dailyNotesCount ?? 0}</p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-full bg-purple-500/10">
                <Brain className="h-5 w-5 text-purple-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Long-term Size</p>
                <p className="text-2xl font-bold">
                  {summary?.longTermSize
                    ? `${(summary.longTermSize / 1024).toFixed(1)}KB`
                    : "0KB"}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-full bg-green-500/10">
                <Clock className="h-5 w-5 text-green-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Last Updated</p>
                <p className="text-lg font-medium truncate">
                  {formatTimestamp(summary?.lastUpdated)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Consolidation Info */}
      {stats && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Consolidation Status</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Last Run</span>
              <span>{formatTimestamp(stats.lastRun)}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Total Processed</span>
              <span>{stats.totalProcessed} entries</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Entries Extracted</span>
              <span>{stats.entriesExtracted}</span>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Actions */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Actions</CardTitle>
          <CardDescription>
            Consolidate daily notes into long-term memory
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button
            onClick={() => triggerConsolidation(7)}
            disabled={consolidating}
            className="w-full"
          >
            {consolidating ? (
              <>
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                Consolidating...
              </>
            ) : (
              <>
                <RefreshCw className="h-4 w-4 mr-2" />
                Consolidate Last 7 Days
              </>
            )}
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Main Memory Panel Component
// ============================================================================

export function MemoryPanel() {
  return (
    <Tabs defaultValue="daily" className="space-y-4" data-testid="memory-tabs">
      <TabsList>
        <TabsTrigger value="daily" className="flex items-center gap-2">
          <Calendar className="h-4 w-4" />
          Daily Notes
        </TabsTrigger>
        <TabsTrigger value="longterm" className="flex items-center gap-2">
          <Brain className="h-4 w-4" />
          Long-term Memory
        </TabsTrigger>
        <TabsTrigger value="consolidation" className="flex items-center gap-2">
          <RefreshCw className="h-4 w-4" />
          Consolidation
        </TabsTrigger>
      </TabsList>

      <TabsContent value="daily">
        <DailyNotes />
      </TabsContent>

      <TabsContent value="longterm">
        <LongTermMemory />
      </TabsContent>

      <TabsContent value="consolidation">
        <ConsolidationPanel />
      </TabsContent>
    </Tabs>
  )
}
