/**
 * Model Selector Dialog
 *
 * A dialog for selecting AI models with:
 * - Fuzzy search
 * - Favorites
 * - Recent selections
 * - Provider grouping
 */

import * as React from "react"
import { Search, Star, Clock, ChevronRight, Check, Sparkles } from "lucide-react"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/Dialog"
import { Input } from "@/components/ui/Input"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Button } from "@/components/ui/Button"
import { cn } from "@/lib/utils"
import {
  useProviderStore,
  useProviders,
  useSelectedModel,
  useModelRecents,
  useProviderLoading,
} from "@/stores/provider"
import type { ModelSelection, ProviderInfo, ProviderModel } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface ModelOption {
  provider: ProviderInfo
  model: ProviderModel
  selection: ModelSelection
  category: string
  isFavorite: boolean
  isRecent: boolean
}

interface ModelSelectorProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSelect?: (selection: ModelSelection) => void
  providerId?: string // Optional: filter to specific provider
}

// ============================================================================
// Fuzzy Search Helper
// ============================================================================

function fuzzyMatch(query: string, text: string): boolean {
  const lowerQuery = query.toLowerCase()
  const lowerText = text.toLowerCase()

  let queryIndex = 0
  for (let i = 0; i < lowerText.length && queryIndex < lowerQuery.length; i++) {
    if (lowerText[i] === lowerQuery[queryIndex]) {
      queryIndex++
    }
  }
  return queryIndex === lowerQuery.length
}

// ============================================================================
// Model Option Component
// ============================================================================

interface ModelOptionItemProps {
  option: ModelOption
  isSelected: boolean
  onSelect: () => void
  onToggleFavorite: () => void
}

function ModelOptionItem({ option, isSelected, onSelect, onToggleFavorite }: ModelOptionItemProps) {
  const { model, provider, isFavorite } = option

  const formatCost = (cost: number | undefined) => {
    if (cost === undefined) return null
    if (cost === 0) return "Free"
    return `$${cost.toFixed(4)}/1K`
  }

  return (
    <div
      className={cn(
        "flex items-center justify-between px-3 py-2 rounded-lg cursor-pointer transition-colors",
        isSelected ? "bg-primary/10 border border-primary/20" : "hover:bg-muted/50",
      )}
      onClick={onSelect}
    >
      <div className="flex items-center gap-3 flex-1 min-w-0">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium truncate">{model.name || model.id}</span>
            {model.status === "preview" && (
              <span className="text-xs px-1.5 py-0.5 rounded bg-yellow-500/10 text-yellow-600">
                Preview
              </span>
            )}
            {model.cost?.input === 0 && (
              <span className="text-xs px-1.5 py-0.5 rounded bg-green-500/10 text-green-600">
                Free
              </span>
            )}
          </div>
          <div className="text-xs text-muted-foreground flex items-center gap-2">
            <span>{provider.name}</span>
            {model.context && (
              <>
                <span className="text-muted-foreground/50">•</span>
                <span>{(model.context / 1000).toFixed(0)}K ctx</span>
              </>
            )}
          </div>
        </div>
      </div>

      <div className="flex items-center gap-2">
        {model.cost && model.cost.input > 0 && (
          <span className="text-xs text-muted-foreground">{formatCost(model.cost.input)}</span>
        )}
        <button
          onClick={(e) => {
            e.stopPropagation()
            onToggleFavorite()
          }}
          className={cn(
            "p-1 rounded hover:bg-muted transition-colors",
            isFavorite ? "text-yellow-500" : "text-muted-foreground/50 hover:text-muted-foreground",
          )}
        >
          <Star className={cn("h-4 w-4", isFavorite && "fill-current")} />
        </button>
        {isSelected && <Check className="h-4 w-4 text-primary" />}
      </div>
    </div>
  )
}

// ============================================================================
// Model Selector Dialog
// ============================================================================

export function ModelSelector({ open, onOpenChange, onSelect, providerId }: ModelSelectorProps) {
  const [query, setQuery] = React.useState("")
  const inputRef = React.useRef<HTMLInputElement>(null)

  const providers = useProviders()
  const selectedModel = useSelectedModel()
  const recents = useModelRecents()
  const { isLoading } = useProviderLoading()
  const { fetchProviders, selectModel, toggleFavorite, isConnected, isFavorite } = useProviderStore()

  // Fetch providers on mount
  React.useEffect(() => {
    if (open && providers.length === 0) {
      fetchProviders()
    }
  }, [open, providers.length, fetchProviders])

  // Focus input when opened
  React.useEffect(() => {
    if (open) {
      setTimeout(() => inputRef.current?.focus(), 100)
    } else {
      setQuery("")
    }
  }, [open])

  // Build model options
  const options = React.useMemo(() => {
    const allOptions: ModelOption[] = []

    // Filter to connected providers or specific provider
    const relevantProviders = providers.filter((p) => {
      if (providerId) return p.id === providerId
      return isConnected(p.id)
    })

    for (const provider of relevantProviders) {
      for (const [modelId, model] of Object.entries(provider.models)) {
        // Skip deprecated models
        if (model.status === "deprecated") continue

        const selection: ModelSelection = {
          providerID: provider.id,
          modelID: modelId,
        }

        allOptions.push({
          provider,
          model,
          selection,
          category: provider.name,
          isFavorite: isFavorite(selection),
          isRecent: recents.some(
            (r) => r.providerID === selection.providerID && r.modelID === selection.modelID,
          ),
        })
      }
    }

    // Sort: Free models last, then alphabetically
    allOptions.sort((a, b) => {
      const aFree = a.model.cost?.input === 0
      const bFree = b.model.cost?.input === 0
      if (aFree !== bFree) return aFree ? 1 : -1
      return (a.model.name || a.model.id).localeCompare(b.model.name || b.model.id)
    })

    return allOptions
  }, [providers, providerId, isConnected, isFavorite, recents])

  // Filter options based on search query
  const filteredOptions = React.useMemo(() => {
    if (!query.trim()) return options

    return options.filter((opt) => {
      const searchText = `${opt.model.name || opt.model.id} ${opt.provider.name}`
      return fuzzyMatch(query, searchText)
    })
  }, [options, query])

  // Group options by category (for non-search view)
  const groupedOptions = React.useMemo(() => {
    if (query.trim()) {
      // Return flat list for search results
      return { "Search Results": filteredOptions }
    }

    const groups: Record<string, ModelOption[]> = {}

    // Add favorites section
    const favOptions = filteredOptions.filter((o) => o.isFavorite)
    if (favOptions.length > 0) {
      groups["Favorites"] = favOptions
    }

    // Add recents section (excluding favorites)
    const recentOptions = filteredOptions.filter(
      (o) => o.isRecent && !favOptions.some((f) => f.selection.providerID === o.selection.providerID && f.selection.modelID === o.selection.modelID),
    )
    if (recentOptions.length > 0) {
      groups["Recent"] = recentOptions
    }

    // Group rest by provider
    const remaining = filteredOptions.filter(
      (o) =>
        !o.isFavorite &&
        !recentOptions.some((r) => r.selection.providerID === o.selection.providerID && r.selection.modelID === o.selection.modelID),
    )

    for (const opt of remaining) {
      const category = opt.category
      if (!groups[category]) groups[category] = []
      groups[category].push(opt)
    }

    return groups
  }, [filteredOptions, query])

  const handleSelect = (option: ModelOption) => {
    selectModel(option.selection)
    onSelect?.(option.selection)
    onOpenChange(false)
  }

  const handleToggleFavorite = (option: ModelOption) => {
    toggleFavorite(option.selection)
  }

  const isSelected = (selection: ModelSelection) =>
    selectedModel?.providerID === selection.providerID && selectedModel?.modelID === selection.modelID

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg p-0 gap-0">
        <DialogHeader className="px-4 pt-4 pb-2">
          <DialogTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-primary" />
            Select Model
          </DialogTitle>
        </DialogHeader>

        {/* Search Input */}
        <div className="px-4 pb-2">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              ref={inputRef}
              placeholder="Search models..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              className="pl-10"
            />
          </div>
        </div>

        {/* Model List */}
        <ScrollArea className="h-[400px] px-4 pb-4">
          {isLoading ? (
            <div className="flex items-center justify-center h-32">
              <span className="text-muted-foreground">Loading providers...</span>
            </div>
          ) : Object.keys(groupedOptions).length === 0 ? (
            <div className="flex flex-col items-center justify-center h-32 gap-2">
              <span className="text-muted-foreground">No models found</span>
              {query && (
                <Button variant="ghost" size="sm" onClick={() => setQuery("")}>
                  Clear search
                </Button>
              )}
            </div>
          ) : (
            <div className="space-y-4">
              {Object.entries(groupedOptions).map(([category, opts]) => (
                <div key={category}>
                  <div className="flex items-center gap-2 mb-2">
                    {category === "Favorites" && <Star className="h-4 w-4 text-yellow-500" />}
                    {category === "Recent" && <Clock className="h-4 w-4 text-muted-foreground" />}
                    <span className="text-sm font-medium text-muted-foreground">{category}</span>
                    <span className="text-xs text-muted-foreground/50">({opts.length})</span>
                  </div>
                  <div className="space-y-1">
                    {opts.map((opt) => (
                      <ModelOptionItem
                        key={`${opt.selection.providerID}-${opt.selection.modelID}`}
                        option={opt}
                        isSelected={isSelected(opt.selection)}
                        onSelect={() => handleSelect(opt)}
                        onToggleFavorite={() => handleToggleFavorite(opt)}
                      />
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </ScrollArea>
      </DialogContent>
    </Dialog>
  )
}

// ============================================================================
// Model Selector Trigger
// ============================================================================

interface ModelSelectorTriggerProps {
  className?: string
}

export function ModelSelectorTrigger({ className }: ModelSelectorTriggerProps) {
  const [open, setOpen] = React.useState(false)
  const selectedModel = useSelectedModel()
  const { getModel } = useProviderStore()

  const model = selectedModel ? getModel(selectedModel.providerID, selectedModel.modelID) : null

  return (
    <>
      <Button
        variant="outline"
        className={cn("justify-between", className)}
        onClick={() => setOpen(true)}
      >
        <div className="flex items-center gap-2 truncate">
          <Sparkles className="h-4 w-4 text-primary shrink-0" />
          <span className="truncate">
            {model ? model.name || model.id : "Select Model"}
          </span>
        </div>
        <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
      </Button>
      <ModelSelector open={open} onOpenChange={setOpen} />
    </>
  )
}

export default ModelSelector
