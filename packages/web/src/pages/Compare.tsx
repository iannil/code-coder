/**
 * Compare Page
 *
 * Multi-model A/B testing interface with:
 * - Model selection from connected providers
 * - Parallel comparison of responses
 * - Side-by-side result display
 * - Token usage and latency stats
 * - Voting and rating functionality
 * - Comparison history
 * - Favorites and sharing
 */

import * as React from "react"
import {
  Sparkles,
  Send,
  Loader2,
  RotateCcw,
  Plus,
  X,
  Timer,
  Hash,
  Copy,
  Check,
  AlertCircle,
  ThumbsUp,
  Star,
  History,
  ChevronRight,
  Trash2,
  Heart,
  Share2,
  Link,
  Bookmark,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Textarea } from "@/components/ui/Textarea"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Badge } from "@/components/ui/Badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@/components/ui/Select"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/Tooltip"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { useToast } from "@/hooks/use-toast"
import { cn } from "@/lib/utils"

// ============================================================================
// Types
// ============================================================================

interface ModelOption {
  id: string
  provider: string
  name: string
  capabilities: {
    reasoning: boolean
    toolcall: boolean
  }
}

interface ModelResult {
  model: string
  provider: string
  model_id: string
  content: string
  tokens: {
    input: number
    output: number
    total: number
  }
  latency_ms: number
  error?: string
}

interface CompareResponse {
  id: string
  results: ModelResult[]
  total_tokens: number
  total_latency_ms: number
}

interface CompareHistoryItem {
  id: string
  timestamp: number
  prompt: string
  models: string[]
  total_tokens: number
  total_latency_ms: number
  votes: Record<string, number>
  vote_count: number
  avg_rating: Record<string, number>
}

// ============================================================================
// Provider Colors
// ============================================================================

const PROVIDER_COLORS: Record<string, { bg: string; text: string; badge: string }> = {
  anthropic: { bg: "bg-purple-500/10", text: "text-purple-600", badge: "bg-purple-100 text-purple-700" },
  openai: { bg: "bg-green-500/10", text: "text-green-600", badge: "bg-green-100 text-green-700" },
  google: { bg: "bg-blue-500/10", text: "text-blue-600", badge: "bg-blue-100 text-blue-700" },
  mistral: { bg: "bg-orange-500/10", text: "text-orange-600", badge: "bg-orange-100 text-orange-700" },
  groq: { bg: "bg-cyan-500/10", text: "text-cyan-600", badge: "bg-cyan-100 text-cyan-700" },
  xai: { bg: "bg-red-500/10", text: "text-red-600", badge: "bg-red-100 text-red-700" },
  default: { bg: "bg-gray-500/10", text: "text-gray-600", badge: "bg-gray-100 text-gray-700" },
}

function getProviderColors(provider: string) {
  return PROVIDER_COLORS[provider.toLowerCase()] ?? PROVIDER_COLORS.default
}

// ============================================================================
// Result Card Component
// ============================================================================

interface ResultCardProps {
  result: ModelResult
  comparisonId?: string
  votes?: Record<string, number>
  onVote?: (model: string, rating?: number) => void
}

function ResultCard({ result, comparisonId, votes, onVote }: ResultCardProps) {
  const [copied, setCopied] = React.useState(false)
  const [rating, setRating] = React.useState(0)
  const [hasVoted, setHasVoted] = React.useState(false)
  const colors = getProviderColors(result.provider)

  const handleCopy = () => {
    navigator.clipboard.writeText(result.content)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleVote = () => {
    if (!comparisonId || hasVoted) return
    onVote?.(result.model, rating > 0 ? rating : undefined)
    setHasVoted(true)
  }

  const voteCount = votes?.[result.model] ?? 0

  return (
    <Card className={cn("flex flex-col h-full", colors.bg)}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Badge className={cn("font-medium", colors.badge)}>
              {result.provider}
            </Badge>
            <CardTitle className="text-sm font-medium">{result.model_id}</CardTitle>
          </div>
          <div className="flex items-center gap-1">
            {!result.error && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleCopy}>
                    {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Copy response</TooltipContent>
              </Tooltip>
            )}
          </div>
        </div>
        <div className="flex items-center gap-4 text-xs text-muted-foreground">
          <div className="flex items-center gap-1">
            <Timer className="h-3 w-3" />
            <span>{result.latency_ms}ms</span>
          </div>
          <div className="flex items-center gap-1">
            <Hash className="h-3 w-3" />
            <span>{result.tokens.total} tokens</span>
          </div>
        </div>
      </CardHeader>
      <CardContent className="flex-1 pt-0">
        {result.error ? (
          <div className="flex items-center gap-2 text-destructive">
            <AlertCircle className="h-4 w-4" />
            <span className="text-sm">{result.error}</span>
          </div>
        ) : (
          <ScrollArea className="h-[250px]">
            <div className="prose prose-sm dark:prose-invert max-w-none">
              <p className="whitespace-pre-wrap text-sm leading-relaxed">{result.content}</p>
            </div>
          </ScrollArea>
        )}
      </CardContent>

      {/* Voting Section */}
      {comparisonId && !result.error && (
        <div className="border-t px-4 py-3">
          <div className="flex items-center justify-between">
            {/* Star Rating */}
            <div className="flex items-center gap-1">
              {[1, 2, 3, 4, 5].map((star) => (
                <button
                  key={star}
                  onClick={() => setRating(star)}
                  disabled={hasVoted}
                  className={cn(
                    "p-0.5 transition-colors",
                    hasVoted ? "cursor-not-allowed opacity-50" : "hover:text-yellow-500"
                  )}
                >
                  <Star
                    className={cn(
                      "h-4 w-4",
                      star <= rating ? "fill-yellow-500 text-yellow-500" : "text-muted-foreground"
                    )}
                  />
                </button>
              ))}
            </div>

            {/* Vote Button */}
            <div className="flex items-center gap-2">
              {voteCount > 0 && (
                <span className="text-xs text-muted-foreground">
                  {voteCount} vote{voteCount !== 1 ? "s" : ""}
                </span>
              )}
              <Button
                variant={hasVoted ? "secondary" : "outline"}
                size="sm"
                onClick={handleVote}
                disabled={hasVoted}
                className="h-7 gap-1"
              >
                <ThumbsUp className={cn("h-3.5 w-3.5", hasVoted && "fill-current")} />
                {hasVoted ? "Voted" : "Vote"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </Card>
  )
}

// ============================================================================
// Model Selector Component
// ============================================================================

interface ModelSelectorProps {
  models: ModelOption[]
  selectedModels: string[]
  onAdd: (model: string) => void
  onRemove: (model: string) => void
  maxModels?: number
}

function ModelSelector({ models, selectedModels, onAdd, onRemove, maxModels = 5 }: ModelSelectorProps) {
  const availableModels = models.filter((m) => !selectedModels.includes(m.id))
  const canAddMore = selectedModels.length < maxModels

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2">
        {selectedModels.map((modelId) => {
          const model = models.find((m) => m.id === modelId)
          if (!model) return null
          const colors = getProviderColors(model.provider)

          return (
            <Badge
              key={modelId}
              variant="secondary"
              className={cn("gap-1 pr-1", colors.badge)}
            >
              {model.name}
              <Button
                variant="ghost"
                size="icon"
                className="h-4 w-4 ml-1 hover:bg-transparent"
                onClick={() => onRemove(modelId)}
              >
                <X className="h-3 w-3" />
              </Button>
            </Badge>
          )
        })}
      </div>

      {canAddMore && availableModels.length > 0 && (
        <Select
          value=""
          onValueChange={(value) => {
            if (value) onAdd(value)
          }}
        >
          <SelectTrigger className="w-[280px]">
            <div className="flex items-center gap-2">
              <Plus className="h-4 w-4" />
              <span>Add model ({selectedModels.length}/{maxModels})</span>
            </div>
          </SelectTrigger>
          <SelectContent>
            {availableModels.map((model) => (
              <SelectItem key={model.id} value={model.id}>
                <div className="flex items-center gap-2">
                  <Badge variant="outline" className="text-xs">
                    {model.provider}
                  </Badge>
                  <span>{model.name}</span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      )}

      {selectedModels.length === 0 && (
        <p className="text-sm text-muted-foreground">
          Select at least 2 models to compare
        </p>
      )}
    </div>
  )
}

// ============================================================================
// Compare Page Component
// ============================================================================

export function Compare() {
  const { toast } = useToast()

  const [models, setModels] = React.useState<ModelOption[]>([])
  const [selectedModels, setSelectedModels] = React.useState<string[]>([])
  const [prompt, setPrompt] = React.useState("")
  const [isLoading, setIsLoading] = React.useState(false)
  const [isLoadingModels, setIsLoadingModels] = React.useState(true)
  const [results, setResults] = React.useState<CompareResponse | null>(null)
  const [votes, setVotes] = React.useState<Record<string, number>>({})
  const [activeTab, setActiveTab] = React.useState("compare")
  const [history, setHistory] = React.useState<CompareHistoryItem[]>([])
  const [isLoadingHistory, setIsLoadingHistory] = React.useState(false)
  const [favorites, setFavorites] = React.useState<Set<string>>(new Set())
  const [showFavoritesOnly, setShowFavoritesOnly] = React.useState(false)
  const [isCopied, setIsCopied] = React.useState(false)

  // Load available models on mount
  React.useEffect(() => {
    loadModels()
  }, [])

  // Load history when switching to history tab
  React.useEffect(() => {
    if (activeTab === "history") {
      loadHistory()
    }
  }, [activeTab])

  const loadModels = async () => {
    setIsLoadingModels(true)
    try {
      const response = await fetch("/api/v1/compare/models")
      const data = await response.json()

      if (data.success && data.data?.models) {
        setModels(data.data.models)

        // Pre-select first two models if available
        if (data.data.models.length >= 2) {
          const defaultModels = data.data.models.slice(0, 2).map((m: ModelOption) => m.id)
          setSelectedModels(defaultModels)
        }
      }
    } catch (error) {
      toast({
        title: "Failed to load models",
        description: "Could not fetch available models for comparison",
        variant: "destructive",
      })
    } finally {
      setIsLoadingModels(false)
    }
  }

  const loadHistory = async () => {
    setIsLoadingHistory(true)
    try {
      const response = await fetch("/api/v1/compare/history?limit=20")
      const data = await response.json()
      if (data.success && data.data?.items) {
        setHistory(data.data.items)
      }
    } catch (error) {
      toast({
        title: "Failed to load history",
        description: "Could not fetch comparison history",
        variant: "destructive",
      })
    } finally {
      setIsLoadingHistory(false)
    }
  }

  const handleCompare = async () => {
    if (selectedModels.length < 2) {
      toast({
        title: "Select more models",
        description: "Please select at least 2 models to compare",
        variant: "destructive",
      })
      return
    }

    if (!prompt.trim()) {
      toast({
        title: "Enter a prompt",
        description: "Please enter a prompt to send to the models",
        variant: "destructive",
      })
      return
    }

    setIsLoading(true)
    setResults(null)
    setVotes({})

    try {
      const response = await fetch("/api/v1/compare", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          models: selectedModels,
          prompt: prompt.trim(),
          max_tokens: 4096,
          temperature: 0.7,
        }),
      })

      const data = await response.json()

      if (data.success && data.data) {
        setResults(data.data)
        toast({
          title: "Comparison complete",
          description: `Received responses from ${data.data.results.length} models`,
        })
      } else {
        throw new Error(data.error || "Comparison failed")
      }
    } catch (error) {
      toast({
        title: "Comparison failed",
        description: error instanceof Error ? error.message : "An error occurred",
        variant: "destructive",
      })
    } finally {
      setIsLoading(false)
    }
  }

  const handleVote = async (model: string, rating?: number) => {
    if (!results?.id) return

    try {
      const response = await fetch(`/api/v1/compare/${results.id}/vote`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model, rating }),
      })

      const data = await response.json()
      if (data.success && data.data?.votes) {
        setVotes(data.data.votes)
        toast({
          title: "Vote recorded",
          description: `Your vote for ${model} has been saved`,
        })
      }
    } catch (error) {
      toast({
        title: "Failed to vote",
        description: "Could not record your vote",
        variant: "destructive",
      })
    }
  }

  const handleDeleteHistory = async (id: string) => {
    try {
      const response = await fetch(`/api/v1/compare/history/${id}`, {
        method: "DELETE",
      })

      if (response.ok) {
        setHistory((prev) => prev.filter((item) => item.id !== id))
        toast({
          title: "Deleted",
          description: "Comparison removed from history",
        })
      }
    } catch (error) {
      toast({
        title: "Failed to delete",
        description: "Could not remove comparison",
        variant: "destructive",
      })
    }
  }

  const handleToggleFavorite = (id: string) => {
    setFavorites((prev) => {
      const newFavorites = new Set(prev)
      if (newFavorites.has(id)) {
        newFavorites.delete(id)
        toast({
          title: "Removed from favorites",
          description: "Comparison removed from your favorites",
        })
      } else {
        newFavorites.add(id)
        toast({
          title: "Added to favorites",
          description: "Comparison saved to your favorites",
        })
      }
      // Persist to localStorage
      localStorage.setItem("comparePageFavorites", JSON.stringify([...newFavorites]))
      return newFavorites
    })
  }

  const handleShare = async (comparisonId: string) => {
    const baseUrl = window.location.origin
    const url = `${baseUrl}/compare?id=${comparisonId}`

    try {
      await navigator.clipboard.writeText(url)
      setIsCopied(true)
      setTimeout(() => setIsCopied(false), 2000)
      toast({
        title: "Link copied!",
        description: "Share link copied to clipboard",
      })
    } catch {
      toast({
        title: "Share link",
        description: url,
      })
    }
  }

  // Load favorites from localStorage on mount
  React.useEffect(() => {
    const storedFavorites = localStorage.getItem("comparePageFavorites")
    if (storedFavorites) {
      try {
        const parsed = JSON.parse(storedFavorites)
        setFavorites(new Set(parsed))
      } catch {
        // Ignore parse errors
      }
    }
  }, [])

  // Filter history by favorites if enabled
  const displayHistory = showFavoritesOnly
    ? history.filter((item) => favorites.has(item.id))
    : history

  const handleReset = () => {
    setResults(null)
    setPrompt("")
    setVotes({})
  }

  const handleAddModel = (modelId: string) => {
    setSelectedModels((prev) => [...prev, modelId])
  }

  const handleRemoveModel = (modelId: string) => {
    setSelectedModels((prev) => prev.filter((id) => id !== modelId))
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault()
      handleCompare()
    }
  }

  const formatTimestamp = (ts: number) => {
    const date = new Date(ts)
    return date.toLocaleString()
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b bg-background px-6 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br from-purple-500 to-blue-500">
              <Sparkles className="h-5 w-5 text-white" />
            </div>
            <div>
              <h1 className="text-xl font-semibold">Model Compare</h1>
              <p className="text-sm text-muted-foreground">
                Compare responses from multiple AI models side-by-side
              </p>
            </div>
          </div>
          {results && activeTab === "compare" && (
            <Button variant="outline" onClick={handleReset}>
              <RotateCcw className="mr-2 h-4 w-4" />
              New Comparison
            </Button>
          )}
        </div>
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
        <div className="border-b px-6">
          <TabsList className="h-12">
            <TabsTrigger value="compare" className="gap-2">
              <Sparkles className="h-4 w-4" />
              Compare
            </TabsTrigger>
            <TabsTrigger value="history" className="gap-2">
              <History className="h-4 w-4" />
              History
              {history.length > 0 && (
                <Badge variant="secondary" className="ml-1 h-5 px-1.5">
                  {history.length}
                </Badge>
              )}
            </TabsTrigger>
          </TabsList>
        </div>

        {/* Compare Tab */}
        <TabsContent value="compare" className="flex-1 overflow-auto p-6 mt-0">
          {/* Model Selection */}
          <Card className="mb-6">
            <CardHeader>
              <CardTitle className="text-base">Select Models</CardTitle>
              <CardDescription>
                Choose 2-5 models to compare. Results will be generated in parallel.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {isLoadingModels ? (
                <div className="flex items-center gap-2 text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  <span>Loading available models...</span>
                </div>
              ) : models.length === 0 ? (
                <div className="text-sm text-muted-foreground">
                  No models available. Please connect a provider in Settings.
                </div>
              ) : (
                <ModelSelector
                  models={models}
                  selectedModels={selectedModels}
                  onAdd={handleAddModel}
                  onRemove={handleRemoveModel}
                />
              )}
            </CardContent>
          </Card>

          {/* Prompt Input */}
          <Card className="mb-6">
            <CardHeader>
              <CardTitle className="text-base">Prompt</CardTitle>
              <CardDescription>
                Enter the prompt to send to all selected models
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <Textarea
                  placeholder="Enter your prompt here... (Cmd+Enter to submit)"
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  onKeyDown={handleKeyDown}
                  className="min-h-[120px] resize-none"
                  disabled={isLoading}
                />
                <div className="flex items-center justify-between">
                  <div className="text-xs text-muted-foreground">
                    {selectedModels.length} model{selectedModels.length !== 1 ? "s" : ""} selected
                  </div>
                  <Button
                    onClick={handleCompare}
                    disabled={isLoading || selectedModels.length < 2 || !prompt.trim()}
                  >
                    {isLoading ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        Comparing...
                      </>
                    ) : (
                      <>
                        <Send className="mr-2 h-4 w-4" />
                        Compare Models
                      </>
                    )}
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Results */}
          {results && (
            <div className="space-y-4">
              {/* Stats Bar */}
              <div className="flex items-center justify-between rounded-lg bg-muted/50 px-4 py-2">
                <div className="flex items-center gap-6 text-sm">
                  <div className="flex items-center gap-2">
                    <Hash className="h-4 w-4 text-muted-foreground" />
                    <span className="font-medium">{results.total_tokens}</span>
                    <span className="text-muted-foreground">total tokens</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <Timer className="h-4 w-4 text-muted-foreground" />
                    <span className="font-medium">{results.total_latency_ms}ms</span>
                    <span className="text-muted-foreground">max latency</span>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Badge variant="outline">
                    {results.results.length} responses
                  </Badge>
                  <Badge variant="secondary" className="gap-1">
                    <ThumbsUp className="h-3 w-3" />
                    {Object.values(votes).reduce((a, b) => a + b, 0)} votes
                  </Badge>
                  {/* Share current comparison */}
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-7 gap-1"
                        onClick={() => handleShare(results.id)}
                      >
                        {isCopied ? <Check className="h-3.5 w-3.5" /> : <Share2 className="h-3.5 w-3.5" />}
                        Share
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>Copy share link</TooltipContent>
                  </Tooltip>
                </div>
              </div>

              {/* Result Cards Grid */}
              <div className={cn(
                "grid gap-4",
                results.results.length === 2 ? "grid-cols-2" :
                results.results.length === 3 ? "grid-cols-3" :
                results.results.length === 4 ? "grid-cols-2 lg:grid-cols-4" :
                "grid-cols-2 lg:grid-cols-3"
              )}>
                {results.results.map((result) => (
                  <ResultCard
                    key={result.model}
                    result={result}
                    comparisonId={results.id}
                    votes={votes}
                    onVote={handleVote}
                  />
                ))}
              </div>
            </div>
          )}
        </TabsContent>

        {/* History Tab */}
        <TabsContent value="history" className="flex-1 overflow-auto p-6 mt-0">
          {/* Favorites Filter Toggle */}
          {history.length > 0 && (
            <div className="flex items-center justify-between mb-4">
              <div className="text-sm text-muted-foreground">
                {showFavoritesOnly ? (
                  <span>{displayHistory.length} favorite{displayHistory.length !== 1 ? "s" : ""}</span>
                ) : (
                  <span>{history.length} comparison{history.length !== 1 ? "s" : ""}</span>
                )}
              </div>
              <Button
                variant={showFavoritesOnly ? "secondary" : "outline"}
                size="sm"
                onClick={() => setShowFavoritesOnly(!showFavoritesOnly)}
                className="gap-2"
              >
                {showFavoritesOnly ? (
                  <>
                    <Heart className="h-4 w-4 fill-current" />
                    Favorites Only
                  </>
                ) : (
                  <>
                    <Bookmark className="h-4 w-4" />
                    Show Favorites
                  </>
                )}
              </Button>
            </div>
          )}

          {isLoadingHistory ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : history.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <History className="h-12 w-12 text-muted-foreground/50 mb-4" />
              <h3 className="text-lg font-medium mb-1">No comparison history</h3>
              <p className="text-sm text-muted-foreground">
                Your model comparisons will appear here
              </p>
              <Button
                variant="outline"
                className="mt-4"
                onClick={() => setActiveTab("compare")}
              >
                Start a comparison
              </Button>
            </div>
          ) : displayHistory.length === 0 && showFavoritesOnly ? (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <Heart className="h-12 w-12 text-muted-foreground/50 mb-4" />
              <h3 className="text-lg font-medium mb-1">No favorites yet</h3>
              <p className="text-sm text-muted-foreground">
                Click the heart icon on comparisons to save them
              </p>
              <Button
                variant="outline"
                className="mt-4"
                onClick={() => setShowFavoritesOnly(false)}
              >
                Show all comparisons
              </Button>
            </div>
          ) : (
            <div className="space-y-3">
              {displayHistory.map((item) => (
                <Card key={item.id} className="hover:bg-muted/50 transition-colors">
                  <CardContent className="p-4">
                    <div className="flex items-start justify-between gap-4">
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium truncate mb-1">
                          {item.prompt}
                        </p>
                        <div className="flex items-center gap-3 text-xs text-muted-foreground">
                          <span>{formatTimestamp(item.timestamp)}</span>
                          <span className="flex items-center gap-1">
                            <Hash className="h-3 w-3" />
                            {item.total_tokens} tokens
                          </span>
                          <span className="flex items-center gap-1">
                            <Timer className="h-3 w-3" />
                            {item.total_latency_ms}ms
                          </span>
                          {item.vote_count > 0 && (
                            <span className="flex items-center gap-1">
                              <ThumbsUp className="h-3 w-3" />
                              {item.vote_count} votes
                            </span>
                          )}
                        </div>
                        <div className="flex flex-wrap gap-1 mt-2">
                          {item.models.map((model) => {
                            const provider = model.split("/")[0]
                            const colors = getProviderColors(provider)
                            return (
                              <Badge
                                key={model}
                                variant="secondary"
                                className={cn("text-xs", colors.badge)}
                              >
                                {model.split("/")[1]}
                              </Badge>
                            )
                          })}
                        </div>
                      </div>
                      <div className="flex items-center gap-1">
                        {/* Favorite Button */}
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              className={cn(
                                "h-8 w-8",
                                favorites.has(item.id)
                                  ? "text-red-500 hover:text-red-600"
                                  : "text-muted-foreground hover:text-red-500"
                              )}
                              onClick={() => handleToggleFavorite(item.id)}
                            >
                              <Heart className={cn(
                                "h-4 w-4",
                                favorites.has(item.id) && "fill-current"
                              )} />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>
                            {favorites.has(item.id) ? "Remove from favorites" : "Add to favorites"}
                          </TooltipContent>
                        </Tooltip>
                        {/* Share Button */}
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8 text-muted-foreground hover:text-foreground"
                              onClick={() => handleShare(item.id)}
                            >
                              <Link className="h-4 w-4" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>Copy share link</TooltipContent>
                        </Tooltip>
                        {/* Delete Button */}
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8 text-muted-foreground hover:text-destructive"
                              onClick={() => handleDeleteHistory(item.id)}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>Delete</TooltipContent>
                        </Tooltip>
                        <ChevronRight className="h-4 w-4 text-muted-foreground" />
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </TabsContent>
      </Tabs>
    </div>
  )
}

export default Compare
