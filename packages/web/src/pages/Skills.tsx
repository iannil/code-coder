/**
 * Skills Page
 *
 * Skill browsing and management with:
 * - Installed skills list (from real API)
 * - Skill details and documentation
 * - Enable/disable functionality
 * - Uninstall functionality
 */

import * as React from "react"
import {
  Puzzle,
  Search,
  Check,
  Package,
  Clock,
  User,
  Filter,
  Trash2,
  Loader2,
  RefreshCw,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { Input } from "@/components/ui/Input"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Badge } from "@/components/ui/Badge"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { Separator } from "@/components/ui/Separator"
import { Switch } from "@/components/ui/Switch"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/Dialog"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/DropdownMenu"
import { useToast } from "@/hooks/use-toast"

// ============================================================================
// Types
// ============================================================================

interface Skill {
  id: string
  name: string
  description: string
  version: string
  author: string
  category: string
  location: string
  installed: boolean
  enabled: boolean
  lastUpdated?: string
  dependencies?: string[]
  readme?: string
}

interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

// ============================================================================
// API Functions
// ============================================================================

async function fetchSkillsApi(): Promise<Skill[]> {
  const res = await fetch("/api/skills")
  const data: ApiResponse<Skill[]> = await res.json()
  return data.success && data.data ? data.data : []
}

async function fetchCategoriesApi(): Promise<string[]> {
  const res = await fetch("/api/skills/categories")
  const data: ApiResponse<string[]> = await res.json()
  return data.success && data.data ? data.data : []
}

async function toggleSkillApi(id: string, enabled: boolean): Promise<boolean> {
  const res = await fetch(`/api/skills/${id}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ enabled }),
  })
  const data = await res.json()
  return data.success
}

async function uninstallSkillApi(id: string): Promise<boolean> {
  const res = await fetch(`/api/skills/${id}`, { method: "DELETE" })
  const data = await res.json()
  return data.success
}

// ============================================================================
// Skill Card Component
// ============================================================================

interface SkillCardProps {
  skill: Skill
  onToggle: (id: string) => void
  onViewDetails: (skill: Skill) => void
  isToggling: boolean
}

function SkillCard({ skill, onToggle, onViewDetails, isToggling }: SkillCardProps) {
  return (
    <Card
      className="cursor-pointer hover:border-primary/50 transition-colors"
      onClick={() => onViewDetails(skill)}
    >
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <Puzzle className="h-5 w-5 text-primary" />
            </div>
            <div>
              <CardTitle className="text-base">{skill.name}</CardTitle>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span>v{skill.version}</span>
                <span>•</span>
                <span className="flex items-center gap-1">
                  <User className="h-3 w-3" />
                  {skill.author}
                </span>
              </div>
            </div>
          </div>
          <Switch
            checked={skill.enabled}
            onCheckedChange={() => onToggle(skill.id)}
            onClick={(e) => e.stopPropagation()}
            disabled={isToggling}
          />
        </div>
      </CardHeader>
      <CardContent>
        <CardDescription className="line-clamp-2">{skill.description}</CardDescription>
        <div className="flex items-center gap-3 mt-3 text-xs text-muted-foreground">
          <Badge variant="secondary" className="text-xs">
            {skill.category}
          </Badge>
          {skill.lastUpdated && (
            <span className="flex items-center gap-1">
              <Clock className="h-3 w-3" />
              {skill.lastUpdated}
            </span>
          )}
        </div>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Skills Page Component
// ============================================================================

export function Skills() {
  const { toast } = useToast()

  const [skills, setSkills] = React.useState<Skill[]>([])
  const [categories, setCategories] = React.useState<string[]>(["All"])
  const [searchQuery, setSearchQuery] = React.useState("")
  const [selectedCategory, setSelectedCategory] = React.useState("All")
  const [activeTab, setActiveTab] = React.useState("all")
  const [selectedSkill, setSelectedSkill] = React.useState<Skill | null>(null)
  const [togglingSkill, setTogglingSkill] = React.useState<string | null>(null)
  const [isLoading, setIsLoading] = React.useState(true)

  // Fetch skills from API
  const fetchSkills = React.useCallback(async () => {
    setIsLoading(true)
    try {
      const data = await fetchSkillsApi()
      setSkills(data)
    } catch {
      toast({
        title: "Failed to load skills",
        variant: "destructive",
      })
    } finally {
      setIsLoading(false)
    }
  }, [toast])

  // Fetch categories from API
  const fetchCategories = React.useCallback(async () => {
    try {
      const data = await fetchCategoriesApi()
      setCategories(["All", ...data])
    } catch {
      // Ignore category fetch errors
    }
  }, [])

  // Initial data load
  React.useEffect(() => {
    fetchSkills()
    fetchCategories()
  }, [fetchSkills, fetchCategories])

  const filteredSkills = React.useMemo(() => {
    let result = skills

    // Filter by tab
    if (activeTab === "enabled") {
      result = result.filter((s) => s.enabled)
    } else if (activeTab === "disabled") {
      result = result.filter((s) => !s.enabled)
    }

    // Filter by category
    if (selectedCategory !== "All") {
      result = result.filter((s) => s.category === selectedCategory)
    }

    // Filter by search
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase()
      result = result.filter(
        (s) =>
          s.name.toLowerCase().includes(query) ||
          s.description.toLowerCase().includes(query) ||
          s.category.toLowerCase().includes(query)
      )
    }

    return result
  }, [skills, activeTab, selectedCategory, searchQuery])

  const handleToggle = async (id: string) => {
    setTogglingSkill(id)
    const skill = skills.find((s) => s.id === id)
    const newEnabled = !skill?.enabled

    const success = await toggleSkillApi(id, newEnabled)

    if (success) {
      setSkills((prev) =>
        prev.map((s) => (s.id === id ? { ...s, enabled: newEnabled } : s))
      )
      toast({
        title: newEnabled ? "Skill enabled" : "Skill disabled",
        description: `${skill?.name} has been ${newEnabled ? "enabled" : "disabled"}.`,
      })
    } else {
      toast({
        title: "Failed to update skill",
        variant: "destructive",
      })
    }
    setTogglingSkill(null)
  }

  const handleUninstall = async (id: string) => {
    const skill = skills.find((s) => s.id === id)

    const success = await uninstallSkillApi(id)

    if (success) {
      setSkills((prev) => prev.filter((s) => s.id !== id))
      setSelectedSkill(null)
      toast({
        title: "Skill uninstalled",
        description: `${skill?.name} has been uninstalled.`,
      })
    } else {
      toast({
        title: "Failed to uninstall skill",
        variant: "destructive",
      })
    }
  }

  const enabledCount = skills.filter((s) => s.enabled).length
  const disabledCount = skills.filter((s) => !s.enabled).length

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex flex-col gap-4 p-4 border-b bg-muted/30">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <Puzzle className="h-5 w-5 text-primary" />
            </div>
            <div>
              <h1 className="text-lg font-semibold">Skills</h1>
              <p className="text-sm text-muted-foreground">
                {skills.length} installed • {enabledCount} enabled
              </p>
            </div>
          </div>
          <Button variant="outline" size="sm" onClick={fetchSkills} disabled={isLoading}>
            {isLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
          </Button>
        </div>

        <div className="flex items-center gap-3">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              type="text"
              placeholder="Search skills..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10"
            />
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm">
                <Filter className="h-4 w-4 mr-2" />
                {selectedCategory}
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuLabel>Category</DropdownMenuLabel>
              <DropdownMenuSeparator />
              {categories.map((cat) => (
                <DropdownMenuItem
                  key={cat}
                  onClick={() => setSelectedCategory(cat)}
                  className={selectedCategory === cat ? "bg-accent" : ""}
                >
                  {selectedCategory === cat && <Check className="h-4 w-4 mr-2" />}
                  {cat}
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
        <div className="border-b px-4">
          <TabsList className="h-12">
            <TabsTrigger value="all">All ({skills.length})</TabsTrigger>
            <TabsTrigger value="enabled">Enabled ({enabledCount})</TabsTrigger>
            <TabsTrigger value="disabled">Disabled ({disabledCount})</TabsTrigger>
          </TabsList>
        </div>

        <ScrollArea className="flex-1 p-4">
          {isLoading && skills.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-64">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              <p className="text-sm text-muted-foreground mt-2">Loading skills...</p>
            </div>
          ) : (
            <div className="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
              {filteredSkills.map((skill) => (
                <SkillCard
                  key={skill.id}
                  skill={skill}
                  onToggle={handleToggle}
                  onViewDetails={setSelectedSkill}
                  isToggling={togglingSkill === skill.id}
                />
              ))}
            </div>
          )}
          {!isLoading && filteredSkills.length === 0 && (
            <div className="flex flex-col items-center justify-center h-64 text-center">
              <Package className="h-12 w-12 text-muted-foreground/50 mb-4" />
              <h3 className="text-lg font-semibold">No skills found</h3>
              <p className="text-sm text-muted-foreground">
                {skills.length === 0
                  ? "No skills are installed yet."
                  : "Try adjusting your search or filter criteria."}
              </p>
            </div>
          )}
        </ScrollArea>
      </Tabs>

      {/* Skill Details Dialog */}
      <Dialog open={!!selectedSkill} onOpenChange={() => setSelectedSkill(null)}>
        <DialogContent className="max-w-2xl">
          {selectedSkill && (
            <>
              <DialogHeader>
                <div className="flex items-center gap-3">
                  <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10">
                    <Puzzle className="h-6 w-6 text-primary" />
                  </div>
                  <div>
                    <DialogTitle className="text-xl">{selectedSkill.name}</DialogTitle>
                    <DialogDescription className="flex items-center gap-2">
                      <span>v{selectedSkill.version}</span>
                      <span>•</span>
                      <span>by {selectedSkill.author}</span>
                    </DialogDescription>
                  </div>
                </div>
              </DialogHeader>
              <div className="space-y-4">
                <p className="text-sm text-muted-foreground">{selectedSkill.description}</p>
                <div className="flex items-center gap-4">
                  <Badge variant="secondary">{selectedSkill.category}</Badge>
                  {selectedSkill.lastUpdated && (
                    <span className="flex items-center gap-1 text-sm text-muted-foreground">
                      <Clock className="h-4 w-4" />
                      Updated {selectedSkill.lastUpdated}
                    </span>
                  )}
                </div>
                {selectedSkill.dependencies && selectedSkill.dependencies.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium mb-2">Dependencies</h4>
                    <div className="flex flex-wrap gap-2">
                      {selectedSkill.dependencies.map((dep) => (
                        <Badge key={dep} variant="outline">{dep}</Badge>
                      ))}
                    </div>
                  </div>
                )}
                <Separator />
                <div className="prose prose-sm dark:prose-invert max-w-none max-h-64 overflow-y-auto">
                  {selectedSkill.readme ? (
                    <pre className="whitespace-pre-wrap text-sm">{selectedSkill.readme}</pre>
                  ) : (
                    <p className="text-muted-foreground">No documentation available.</p>
                  )}
                </div>
                <div className="flex justify-between">
                  <Button
                    variant="destructive"
                    size="sm"
                    onClick={() => handleUninstall(selectedSkill.id)}
                  >
                    <Trash2 className="h-4 w-4 mr-2" />
                    Uninstall
                  </Button>
                  <Button
                    variant={selectedSkill.enabled ? "outline" : "default"}
                    onClick={() => {
                      handleToggle(selectedSkill.id)
                      setSelectedSkill(null)
                    }}
                  >
                    {selectedSkill.enabled ? "Disable" : "Enable"}
                  </Button>
                </div>
              </div>
            </>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}

export default Skills
