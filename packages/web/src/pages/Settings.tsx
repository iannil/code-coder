/**
 * Settings Page
 *
 * Application settings with:
 * - Config form
 * - Provider management
 * - MCP server management
 * - Agent configuration
 * - Permission settings
 * - API key management
 */

import * as React from "react"
import {
  Settings as SettingsIcon,
  Key,
  Shield,
  Bot,
  Save,
  RotateCw,
  Eye,
  EyeOff,
  Check,
  X,
  Sparkles,
  Plug,
  Power,
  AlertCircle,
  CheckCircle,
  XCircle,
  RefreshCw,
  Brain,
  Zap,
  Code,
  Database,
  Palette,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Input } from "@/components/ui/Input"
import { Label } from "@/components/ui/Label"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { Separator } from "@/components/ui/Separator"
import { Skeleton } from "@/components/ui/Skeleton"
import { Badge } from "@/components/ui/Badge"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select"
import { useConfig, useConfigLoading, useConfigStore } from "@/stores/config"
import { useAgents, useAgentsLoading } from "@/stores/agent"
import {
  useProviderStore,
  useProviders,
  useConnectedProviders,
  useProviderLoading,
} from "@/stores/provider"
import {
  useMcpStore,
  useMcpStatus,
  useMcpTools,
  useMcpLoading,
} from "@/stores/mcp"
import { useToast } from "@/hooks/use-toast"
import { cn } from "@/lib/utils"
import type { ConfigData, ProviderInfo, McpStatus } from "@/lib/types"
import { MemoryPanel } from "@/components/memory"
import { HooksPanel } from "@/components/hooks"
import { LspPanel } from "@/components/lsp"
import { StoragePanel } from "@/components/storage"
import { ThemeSelector } from "@/components/theme"

// ============================================================================
// Types
// ============================================================================

interface ConfigField {
  key: string
  label: string
  description?: string
  type: "text" | "password" | "textarea" | "number" | "boolean"
  placeholder?: string
  sensitive?: boolean
}

// ============================================================================
// Config Form Component
// ============================================================================

interface ConfigFormProps {
  config: Record<string, unknown> | null
  isLoading: boolean
  isSaving: boolean
  onSave: (updates: Partial<ConfigData>) => Promise<ConfigData>
}

function ConfigForm({ config, isLoading, isSaving, onSave }: ConfigFormProps) {
  const [formData, setFormData] = React.useState<Record<string, unknown>>({})
  const [visibleKeys, setVisibleKeys] = React.useState<Set<string>>(new Set())
  const [showSuccess, setShowSuccess] = React.useState(false)
  const { toast } = useToast()

  React.useEffect(() => {
    if (config) {
      setFormData(config)
    }
  }, [config])

  const configFields: ConfigField[] = [
    {
      key: "model",
      label: "Default Model",
      description: "Model to use in the format of provider/model (e.g., anthropic/claude-sonnet-4-5)",
      type: "text",
      placeholder: "anthropic/claude-sonnet-4-5",
    },
    {
      key: "small_model",
      label: "Small Model",
      description: "Fast model for tasks like title generation",
      type: "text",
      placeholder: "anthropic/claude-haiku-4-5",
    },
    {
      key: "default_agent",
      label: "Default Agent",
      description: "Primary agent to use when none specified",
      type: "text",
      placeholder: "build",
    },
    {
      key: "username",
      label: "Display Name",
      description: "Custom username to display in conversations",
      type: "text",
      placeholder: "Your name",
    },
    {
      key: "theme",
      label: "Theme",
      description: "Theme name for the interface",
      type: "text",
      placeholder: "default",
    },
    {
      key: "logLevel",
      label: "Log Level",
      description: "Logging verbosity (debug, info, warn, error)",
      type: "text",
      placeholder: "info",
    },
  ]

  const handleFieldChange = (key: string, value: unknown) => {
    setFormData((prev) => ({ ...prev, [key]: value }))
  }

  const handleSave = async () => {
    try {
      await onSave(formData as Partial<ConfigData>)
      setShowSuccess(true)
      setTimeout(() => setShowSuccess(false), 3000)
      toast({
        title: "Settings saved",
        description: "Your configuration has been updated successfully.",
      })
    } catch {
      toast({
        title: "Failed to save",
        description: "An error occurred while saving your settings.",
        variant: "destructive",
      })
    }
  }

  const handleReset = () => {
    if (config) {
      setFormData(config)
      toast({
        title: "Settings reset",
        description: "Changes have been discarded.",
      })
    }
  }

  const toggleVisibility = (key: string) => {
    setVisibleKeys((prev) => {
      const next = new Set(prev)
      if (next.has(key)) {
        next.delete(key)
      } else {
        next.add(key)
      }
      return next
    })
  }

  const hasChanges = JSON.stringify(formData) !== JSON.stringify(config)

  if (isLoading) {
    return (
      <div className="space-y-6">
        {[1, 2, 3].map((i) => (
          <div key={i} className="space-y-2">
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-10 w-full" />
          </div>
        ))}
      </div>
    )
  }

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault()
        handleSave()
      }}
      className="space-y-6"
    >
      {configFields.map((field) => {
        const value = formData[field.key] as string | number | undefined
        const isVisible = visibleKeys.has(field.key) || !field.sensitive

        return (
          <div key={field.key} className="space-y-2">
            <Label htmlFor={field.key} className="flex items-center gap-2">
              {field.label}
            </Label>
            <div className="relative">
              <Input
                id={field.key}
                type={isVisible ? field.type : "password"}
                value={value ?? ""}
                onChange={(e) => {
                  const newValue = field.type === "number"
                    ? e.target.value ? Number.parseFloat(e.target.value) : undefined
                    : e.target.value
                  handleFieldChange(field.key, newValue)
                }}
                placeholder={field.placeholder}
                className={cn(field.sensitive && "pr-10")}
              />
              {field.sensitive && (
                <button
                  type="button"
                  onClick={() => toggleVisibility(field.key)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                >
                  {isVisible ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              )}
            </div>
            {field.description && (
              <p className="text-sm text-muted-foreground">{field.description}</p>
            )}
          </div>
        )
      })}

      {/* Actions */}
      <div className="flex items-center gap-2 pt-4">
        <Button type="submit" disabled={!hasChanges || isSaving} data-testid="save-settings-btn">
          {isSaving ? (
            <>
              <RotateCw className="mr-2 h-4 w-4 animate-spin" />
              Saving...
            </>
          ) : (
            <>
              <Save className="mr-2 h-4 w-4" />
              Save Changes
            </>
          )}
        </Button>
        {hasChanges && (
          <Button type="button" variant="ghost" onClick={handleReset}>
            Reset
          </Button>
        )}
        {showSuccess && (
          <div data-testid="save-success" className="flex items-center gap-2 text-sm text-green-600">
            <Check className="h-4 w-4" />
            Saved successfully
          </div>
        )}
      </div>
    </form>
  )
}

// ============================================================================
// API Key Management Component
// ============================================================================

interface ApiKey {
  id: string
  name: string
  key: string
  createdAt: number
  lastUsed?: number
}

interface ApiKeyManagementProps {
  apiKeys: ApiKey[]
}

function ApiKeyManagement({ apiKeys }: ApiKeyManagementProps) {
  const [keys, setKeys] = React.useState<ApiKey[]>(apiKeys)
  const [newKeyName, setNewKeyName] = React.useState("")
  const [showCreateForm, setShowCreateForm] = React.useState(false)
  const { toast } = useToast()

  const handleCreateKey = () => {
    if (!newKeyName.trim()) return

    const newKey: ApiKey = {
      id: `key-${Date.now()}`,
      name: newKeyName,
      key: `sk-${Math.random().toString(36).substring(2, 15)}${Math.random().toString(36).substring(2, 15)}`,
      createdAt: Date.now(),
    }

    setKeys((prev) => [...prev, newKey])
    setNewKeyName("")
    setShowCreateForm(false)

    toast({
      title: "API key created",
      description: "Your new API key has been created successfully.",
    })
  }

  const handleDeleteKey = (keyId: string) => {
    setKeys((prev) => prev.filter((k) => k.id !== keyId))
    toast({
      title: "API key deleted",
      description: "The API key has been deleted.",
    })
  }

  const maskKey = (key: string) => `${key.slice(0, 7)}${"*".repeat(key.length - 7)}`

  return (
    <div className="space-y-4">
      {/* Create New Key */}
      {!showCreateForm ? (
        <Button
          variant="outline"
          onClick={() => setShowCreateForm(true)}
          className="w-full border-dashed"
        >
          <Key className="mr-2 h-4 w-4" />
          Create new API key
        </Button>
      ) : (
        <Card>
          <CardContent className="pt-6 space-y-4">
            <div className="space-y-2">
              <Label htmlFor="key-name">Key Name</Label>
              <Input
                id="key-name"
                placeholder="e.g., Production API Key"
                value={newKeyName}
                onChange={(e) => setNewKeyName(e.target.value)}
                data-testid="api-key-input"
              />
            </div>
            <div className="flex gap-2">
              <Button onClick={handleCreateKey} disabled={!newKeyName.trim()}>
                <Check className="mr-2 h-4 w-4" />
                Create
              </Button>
              <Button
                variant="ghost"
                onClick={() => {
                  setShowCreateForm(false)
                  setNewKeyName("")
                }}
              >
                Cancel
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      <Separator />

      {/* Existing Keys */}
      <div className="space-y-3">
        {keys.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-8">
            No API keys yet. Create one to get started.
          </p>
        ) : (
          keys.map((key) => (
            <div
              key={key.id}
              className="flex items-center justify-between p-3 rounded-lg border bg-card"
            >
              <div className="space-y-1">
                <p className="font-medium">{key.name}</p>
                <code className="text-sm text-muted-foreground">{maskKey(key.key)}</code>
                <p className="text-xs text-muted-foreground">
                  Created {new Date(key.createdAt).toLocaleDateString()}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => handleDeleteKey(key.id)}
                className="text-destructive hover:text-destructive"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          ))
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Agent Configuration Component
// ============================================================================

interface AgentConfigurationProps {
  agents: Array<{ id: string; name: string; description?: string; category?: string }>
  isLoading: boolean
}

function AgentConfiguration({ agents, isLoading }: AgentConfigurationProps) {
  if (isLoading) {
    return (
      <div className="space-y-4">
        {[1, 2, 3].map((i) => (
          <div key={i} className="p-4 rounded-lg border">
            <Skeleton className="h-5 w-32 mb-2" />
            <Skeleton className="h-4 w-full mb-1" />
            <Skeleton className="h-4 w-2/3" />
          </div>
        ))}
      </div>
    )
  }

  if (agents.length === 0) {
    return (
      <div className="text-center py-8">
        <Bot className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <p className="text-muted-foreground">No agents configured yet.</p>
      </div>
    )
  }

  return (
    <div className="space-y-3">
      {agents.map((agent) => (
        <Card key={agent.id}>
          <CardContent className="p-4">
            <div className="flex items-start justify-between">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <h4 className="font-medium">{agent.name}</h4>
                  {agent.category && (
                    <Badge variant="default">
                      {agent.category}
                    </Badge>
                  )}
                </div>
                {agent.description && (
                  <p className="text-sm text-muted-foreground">{agent.description}</p>
                )}
              </div>
              <Button variant="ghost" size="icon">
                <SettingsIcon className="h-4 w-4" />
              </Button>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

// ============================================================================
// Permission Settings Component
// ============================================================================

// All supported permission types
const PERMISSION_TYPES = [
  { id: "Read", description: "Read file contents", icon: "read" },
  { id: "Edit", description: "Edit existing files", icon: "edit" },
  { id: "Write", description: "Create new files", icon: "write" },
  { id: "Glob", description: "Search for files by pattern", icon: "search" },
  { id: "Grep", description: "Search file contents", icon: "search" },
  { id: "Bash", description: "Execute shell commands", icon: "terminal" },
  { id: "Task", description: "Launch sub-agents", icon: "bot" },
  { id: "WebFetch", description: "Fetch web content", icon: "web" },
  { id: "WebSearch", description: "Search the web", icon: "web" },
  { id: "MCP", description: "Use MCP tools", icon: "plugin" },
  { id: "NotebookEdit", description: "Edit Jupyter notebooks", icon: "notebook" },
] as const

interface PermissionRule {
  id: string
  type: string
  pattern?: string
  action: "allow" | "deny" | "prompt"
  description?: string
}

function PermissionSettings() {
  const [rules, setRules] = React.useState<PermissionRule[]>([
    {
      id: "read-all",
      type: "Read",
      action: "allow",
      description: "Allow reading all files",
    },
    {
      id: "edit-all",
      type: "Edit",
      action: "prompt",
      description: "Prompt before editing files",
    },
    {
      id: "write-all",
      type: "Write",
      action: "prompt",
      description: "Prompt before creating files",
    },
    {
      id: "bash-all",
      type: "Bash",
      action: "prompt",
      description: "Prompt before running commands",
    },
    {
      id: "glob-all",
      type: "Glob",
      action: "allow",
      description: "Allow file search",
    },
    {
      id: "grep-all",
      type: "Grep",
      action: "allow",
      description: "Allow content search",
    },
    {
      id: "webfetch-all",
      type: "WebFetch",
      action: "prompt",
      description: "Prompt before fetching URLs",
    },
    {
      id: "task-all",
      type: "Task",
      action: "allow",
      description: "Allow launching sub-agents",
    },
    {
      id: "mcp-all",
      type: "MCP",
      action: "prompt",
      description: "Prompt before using MCP tools",
    },
  ])

  const [showAddRule, setShowAddRule] = React.useState(false)
  const [newRuleType, setNewRuleType] = React.useState("")
  const [newRulePattern, setNewRulePattern] = React.useState("")
  const { toast } = useToast()

  const updateRule = (id: string, action: "allow" | "deny" | "prompt") => {
    setRules((prev) =>
      prev.map((rule) => (rule.id === id ? { ...rule, action } : rule))
    )
    toast({
      title: "Permission updated",
      description: "The permission rule has been updated.",
    })
  }

  const deleteRule = (id: string) => {
    setRules((prev) => prev.filter((rule) => rule.id !== id))
    toast({
      title: "Rule deleted",
      description: "The permission rule has been deleted.",
    })
  }

  const addRule = () => {
    if (!newRuleType) return

    const typeInfo = PERMISSION_TYPES.find((t) => t.id === newRuleType)
    const newRule: PermissionRule = {
      id: `${newRuleType.toLowerCase()}-${Date.now()}`,
      type: newRuleType,
      pattern: newRulePattern || undefined,
      action: "prompt",
      description: newRulePattern
        ? `${typeInfo?.description} for pattern: ${newRulePattern}`
        : typeInfo?.description,
    }

    setRules((prev) => [...prev, newRule])
    setNewRuleType("")
    setNewRulePattern("")
    setShowAddRule(false)
    toast({
      title: "Rule added",
      description: "A new permission rule has been added.",
    })
  }

  const getActionVariant = (action: "allow" | "deny" | "prompt") => {
    switch (action) {
      case "allow":
        return "success" as const
      case "deny":
        return "destructive" as const
      case "prompt":
        return "warning" as const
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          Configure how CodeCoder handles sensitive operations.
        </p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setShowAddRule(!showAddRule)}
        >
          {showAddRule ? "Cancel" : "Add Rule"}
        </Button>
      </div>

      {/* Add Rule Form */}
      {showAddRule && (
        <Card className="border-dashed">
          <CardContent className="pt-4 space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="rule-type">Permission Type</Label>
                <Select value={newRuleType} onValueChange={setNewRuleType}>
                  <SelectTrigger>
                    <SelectValue placeholder="Select type..." />
                  </SelectTrigger>
                  <SelectContent>
                    {PERMISSION_TYPES.map((type) => (
                      <SelectItem key={type.id} value={type.id}>
                        {type.id} - {type.description}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="rule-pattern">Pattern (optional)</Label>
                <Input
                  id="rule-pattern"
                  placeholder="e.g., *.config.* or /tmp/*"
                  value={newRulePattern}
                  onChange={(e) => setNewRulePattern(e.target.value)}
                />
              </div>
            </div>
            <div className="flex gap-2">
              <Button onClick={addRule} disabled={!newRuleType} size="sm">
                <Check className="mr-2 h-4 w-4" />
                Add Rule
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Rules List */}
      <div className="space-y-2">
        {rules.map((rule) => (
          <div
            key={rule.id}
            className="flex items-center justify-between p-3 rounded-lg border hover:bg-muted/30 transition-colors"
          >
            <div className="flex items-center gap-3 flex-1 min-w-0">
              <Shield className="h-4 w-4 text-primary shrink-0" />
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <code className="text-sm font-medium">{rule.type}</code>
                  {rule.pattern && (
                    <Badge variant="outline" className="font-mono truncate max-w-[150px]">
                      {rule.pattern}
                    </Badge>
                  )}
                  <Badge variant={getActionVariant(rule.action)} className="capitalize">
                    {rule.action}
                  </Badge>
                </div>
                {rule.description && (
                  <p className="text-xs text-muted-foreground truncate">
                    {rule.description}
                  </p>
                )}
              </div>
            </div>

            <div className="flex items-center gap-1">
              {(["allow", "prompt", "deny"] as const).map((action) => (
                <Button
                  key={action}
                  variant={rule.action === action ? "default" : "ghost"}
                  size="sm"
                  onClick={() => updateRule(rule.id, action)}
                  className="h-7 px-2 text-xs"
                >
                  {action === "allow" ? "✓" : action === "deny" ? "✕" : "?"}
                </Button>
              ))}
              <Button
                variant="ghost"
                size="sm"
                onClick={() => deleteRule(rule.id)}
                className="h-7 px-2 text-xs text-destructive hover:text-destructive"
              >
                <X className="h-3 w-3" />
              </Button>
            </div>
          </div>
        ))}
      </div>

      {/* Permission Type Legend */}
      <Separator className="my-4" />
      <div className="space-y-2">
        <h4 className="text-sm font-medium">Permission Types</h4>
        <div className="grid grid-cols-2 md:grid-cols-3 gap-2 text-xs">
          {PERMISSION_TYPES.map((type) => (
            <div key={type.id} className="flex items-center gap-2 text-muted-foreground">
              <span className="font-mono text-primary">{type.id}</span>
              <span>-</span>
              <span className="truncate">{type.description}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}

// ============================================================================
// Provider Management Component
// ============================================================================

function ProviderManagement() {
  const providers = useProviders()
  const connectedIds = useConnectedProviders()
  const { isLoading } = useProviderLoading()
  const fetchProviders = useProviderStore((state) => state.fetchProviders)

  // Track initialization to prevent infinite loop
  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchProviders()
  }, [fetchProviders])

  if (isLoading) {
    return (
      <div className="space-y-4">
        {[1, 2, 3].map((i) => (
          <div key={i} className="p-4 rounded-lg border">
            <Skeleton className="h-5 w-32 mb-2" />
            <Skeleton className="h-4 w-full mb-1" />
            <Skeleton className="h-4 w-2/3" />
          </div>
        ))}
      </div>
    )
  }

  if (providers.length === 0) {
    return (
      <div className="text-center py-8">
        <Sparkles className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <p className="text-muted-foreground">No providers configured yet.</p>
        <p className="text-sm text-muted-foreground mt-2">
          Configure providers in your codecoder.json file.
        </p>
      </div>
    )
  }

  const connectedProviders = providers.filter((p) => connectedIds.includes(p.id))
  const availableProviders = providers.filter((p) => !connectedIds.includes(p.id))

  return (
    <div className="space-y-6">
      {/* Connected Providers */}
      {connectedProviders.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-medium flex items-center gap-2">
            <CheckCircle className="h-4 w-4 text-green-500" />
            Connected ({connectedProviders.length})
          </h3>
          {connectedProviders.map((provider) => (
            <ProviderCard key={provider.id} provider={provider} isConnected />
          ))}
        </div>
      )}

      {/* Available Providers */}
      {availableProviders.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
            <XCircle className="h-4 w-4" />
            Available ({availableProviders.length})
          </h3>
          {availableProviders.map((provider) => (
            <ProviderCard key={provider.id} provider={provider} isConnected={false} />
          ))}
        </div>
      )}
    </div>
  )
}

interface ProviderCardProps {
  provider: ProviderInfo
  isConnected: boolean
}

function ProviderCard({ provider, isConnected }: ProviderCardProps) {
  const modelCount = Object.keys(provider.models).length

  return (
    <Card className={cn(!isConnected && "opacity-60")}>
      <CardContent className="p-4">
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <Sparkles className={cn("h-4 w-4", isConnected ? "text-primary" : "text-muted-foreground")} />
              <h4 className="font-medium">{provider.name}</h4>
              <Badge variant={isConnected ? "success" : "outline"}>
                {isConnected ? "Connected" : "Not connected"}
              </Badge>
            </div>
            <p className="text-sm text-muted-foreground">
              {modelCount} model{modelCount !== 1 ? "s" : ""} available
            </p>
            {provider.env.length > 0 && (
              <p className="text-xs text-muted-foreground">
                Env: {provider.env.join(", ")}
              </p>
            )}
          </div>
          {!isConnected && (
            <Button variant="outline" size="sm" disabled>
              <Key className="mr-2 h-3 w-3" />
              Add API Key
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// MCP Management Component
// ============================================================================

function McpManagement() {
  const status = useMcpStatus()
  const tools = useMcpTools()
  const { isLoading, isToggling } = useMcpLoading()
  const fetchAll = useMcpStore((s) => s.fetchAll)
  const toggle = useMcpStore((s) => s.toggle)
  const { toast } = useToast()

  // Track initialization to prevent infinite loop
  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchAll()
  }, [fetchAll])

  const handleToggle = async (name: string) => {
    try {
      await toggle(name)
      toast({
        title: "MCP server toggled",
        description: `${name} has been ${status[name]?.status === "disabled" ? "enabled" : "disabled"}.`,
      })
    } catch {
      toast({
        title: "Failed to toggle MCP server",
        description: "An error occurred. Please try again.",
        variant: "destructive",
      })
    }
  }

  const servers = Object.entries(status)

  if (isLoading) {
    return (
      <div className="space-y-4">
        {[1, 2, 3].map((i) => (
          <div key={i} className="p-4 rounded-lg border">
            <Skeleton className="h-5 w-32 mb-2" />
            <Skeleton className="h-4 w-full" />
          </div>
        ))}
      </div>
    )
  }

  if (servers.length === 0) {
    return (
      <div className="text-center py-8">
        <Plug className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <p className="text-muted-foreground">No MCP servers configured.</p>
        <p className="text-sm text-muted-foreground mt-2">
          Add MCP servers in your codecoder.json config file.
        </p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Server List */}
      <div className="space-y-3">
        {servers.map(([name, serverStatus]) => (
          <McpServerCard
            key={name}
            name={name}
            status={serverStatus}
            isToggling={isToggling === name}
            onToggle={() => handleToggle(name)}
          />
        ))}
      </div>

      {/* Connected Tools Summary */}
      {tools.length > 0 && (
        <div className="space-y-3">
          <Separator />
          <h3 className="text-sm font-medium">
            Available Tools ({tools.length})
          </h3>
          <div className="grid grid-cols-2 gap-2">
            {tools.slice(0, 6).map((tool) => (
              <div key={tool.name} className="text-xs p-2 rounded bg-muted truncate">
                {tool.name}
              </div>
            ))}
            {tools.length > 6 && (
              <div className="text-xs p-2 rounded bg-muted text-muted-foreground text-center col-span-2">
                +{tools.length - 6} more tools
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}

interface McpServerCardProps {
  name: string
  status: McpStatus
  isToggling: boolean
  onToggle: () => void
}

function McpServerCard({ name, status, isToggling, onToggle }: McpServerCardProps) {
  const getStatusIcon = () => {
    switch (status.status) {
      case "connected":
        return <CheckCircle className="h-4 w-4 text-green-500" />
      case "disabled":
        return <Power className="h-4 w-4 text-muted-foreground" />
      case "failed":
        return <XCircle className="h-4 w-4 text-red-500" />
      case "needs_auth":
        return <AlertCircle className="h-4 w-4 text-yellow-500" />
      case "needs_client_registration":
        return <AlertCircle className="h-4 w-4 text-yellow-500" />
      default:
        return <AlertCircle className="h-4 w-4 text-muted-foreground" />
    }
  }

  const getStatusLabel = () => {
    switch (status.status) {
      case "connected":
        return "Connected"
      case "disabled":
        return "Disabled"
      case "failed":
        return `Failed: ${status.error}`
      case "needs_auth":
        return "Authentication required"
      case "needs_client_registration":
        return "Client registration required"
      default:
        return "Unknown"
    }
  }

  const isEnabled = status.status === "connected"

  return (
    <Card className={cn(!isEnabled && "opacity-75")}>
      <CardContent className="p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Plug className={cn("h-5 w-5", isEnabled ? "text-primary" : "text-muted-foreground")} />
            <div>
              <h4 className="font-medium">{name}</h4>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                {getStatusIcon()}
                <span className="truncate max-w-[200px]">{getStatusLabel()}</span>
              </div>
            </div>
          </div>
          <Button
            variant={isEnabled ? "default" : "outline"}
            size="sm"
            onClick={onToggle}
            disabled={isToggling || status.status === "needs_auth" || status.status === "needs_client_registration"}
          >
            {isToggling ? (
              <RefreshCw className="h-4 w-4 animate-spin" />
            ) : isEnabled ? (
              <>
                <Power className="mr-2 h-3 w-3" />
                Disable
              </>
            ) : (
              <>
                <Power className="mr-2 h-3 w-3" />
                Enable
              </>
            )}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Main Settings Component
// ============================================================================

export function Settings() {
  const config = useConfig()
  const { isLoading, isUpdating } = useConfigLoading()
  const updateConfig = useConfigStore((state) => state.updateConfig)

  const agents = useAgents()
  const { isLoading: agentsLoading } = useAgentsLoading()

  return (
    <div className="flex-1 overflow-auto">
      <div className="container max-w-3xl mx-auto p-6 space-y-8">
        {/* Header */}
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <SettingsIcon className="h-6 w-6 text-primary" />
            <h1 className="text-3xl font-bold tracking-tight">Settings</h1>
          </div>
          <p className="text-muted-foreground">
            Configure your CodeCoder experience
          </p>
        </div>

        {/* Settings Tabs */}
        <Tabs defaultValue="general" className="space-y-6">
          <TabsList className="flex flex-wrap gap-1">
            <TabsTrigger value="general">General</TabsTrigger>
            <TabsTrigger value="appearance">Appearance</TabsTrigger>
            <TabsTrigger value="providers">Providers</TabsTrigger>
            <TabsTrigger value="mcp">MCP</TabsTrigger>
            <TabsTrigger value="agents">Agents</TabsTrigger>
            <TabsTrigger value="permissions">Permissions</TabsTrigger>
            <TabsTrigger value="memory">Memory</TabsTrigger>
            <TabsTrigger value="hooks">Hooks</TabsTrigger>
            <TabsTrigger value="lsp">LSP</TabsTrigger>
            <TabsTrigger value="storage">Storage</TabsTrigger>
            <TabsTrigger value="api">API Keys</TabsTrigger>
          </TabsList>

          {/* General Settings */}
          <TabsContent value="general" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle>General Configuration</CardTitle>
                <CardDescription>
                  Manage your application settings and preferences
                </CardDescription>
              </CardHeader>
              <CardContent>
                <ConfigForm
                  config={config as Record<string, unknown> | null}
                  isLoading={isLoading}
                  isSaving={isUpdating}
                  onSave={updateConfig}
                />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Appearance Settings */}
          <TabsContent value="appearance" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Palette className="h-5 w-5 text-primary" />
                  Appearance
                </CardTitle>
                <CardDescription>
                  Customize the look and feel of the application
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-6">
                <div>
                  <h4 className="text-sm font-medium mb-3">Theme</h4>
                  <ThemeSelector />
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          {/* Providers Settings */}
          <TabsContent value="providers" className="space-y-6" data-testid="provider-settings">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Sparkles className="h-5 w-5 text-primary" />
                  AI Providers
                </CardTitle>
                <CardDescription>
                  Manage AI model providers and API keys
                </CardDescription>
              </CardHeader>
              <CardContent>
                <ProviderManagement />
              </CardContent>
            </Card>
          </TabsContent>

          {/* MCP Settings */}
          <TabsContent value="mcp" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Plug className="h-5 w-5 text-primary" />
                  MCP Servers
                </CardTitle>
                <CardDescription>
                  Manage Model Context Protocol server connections
                </CardDescription>
              </CardHeader>
              <CardContent>
                <McpManagement />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Agents Settings */}
          <TabsContent value="agents" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle>Agent Configuration</CardTitle>
                <CardDescription>
                  Configure and manage available AI agents
                </CardDescription>
              </CardHeader>
              <CardContent>
                <AgentConfiguration agents={agents} isLoading={agentsLoading} />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Permissions Settings */}
          <TabsContent value="permissions" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle>Permission Rules</CardTitle>
                <CardDescription>
                  Set default behavior for sensitive operations
                </CardDescription>
              </CardHeader>
              <CardContent>
                <PermissionSettings />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Memory Settings */}
          <TabsContent value="memory" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Brain className="h-5 w-5 text-primary" />
                  Memory System
                </CardTitle>
                <CardDescription>
                  View and manage daily notes and long-term memory
                </CardDescription>
              </CardHeader>
              <CardContent>
                <MemoryPanel />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Hooks Settings */}
          <TabsContent value="hooks" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Zap className="h-5 w-5 text-primary" />
                  Hooks Configuration
                </CardTitle>
                <CardDescription>
                  View and manage lifecycle hooks
                </CardDescription>
              </CardHeader>
              <CardContent>
                <HooksPanel />
              </CardContent>
            </Card>
          </TabsContent>

          {/* LSP Settings */}
          <TabsContent value="lsp" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Code className="h-5 w-5 text-primary" />
                  Language Server Protocol
                </CardTitle>
                <CardDescription>
                  Manage LSP servers and view diagnostics
                </CardDescription>
              </CardHeader>
              <CardContent>
                <LspPanel />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Storage Settings */}
          <TabsContent value="storage" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Database className="h-5 w-5 text-primary" />
                  Storage Management
                </CardTitle>
                <CardDescription>
                  View storage usage and manage cached data
                </CardDescription>
              </CardHeader>
              <CardContent>
                <StoragePanel />
              </CardContent>
            </Card>
          </TabsContent>

          {/* API Keys Settings */}
          <TabsContent value="api" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle>API Key Management</CardTitle>
                <CardDescription>
                  Create and manage your API keys for external integrations
                </CardDescription>
              </CardHeader>
              <CardContent>
                <ApiKeyManagement apiKeys={[]} />
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      </div>
    </div>
  )
}

export default Settings
