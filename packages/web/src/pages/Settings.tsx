/**
 * Settings Page
 *
 * Application settings with:
 * - Config form
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
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Input } from "@/components/ui/Input"
import { Label } from "@/components/ui/Label"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { Separator } from "@/components/ui/Separator"
import { useConfig, useConfigLoading, useConfigStore } from "@/stores/config"
import { useAgents, useAgentsLoading } from "@/stores/agent"
import { useToast } from "@/hooks/use-toast"
import { cn } from "@/lib/utils"
import type { ConfigData } from "@/lib/types"

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
  const { toast } = useToast()

  React.useEffect(() => {
    if (config) {
      setFormData(config)
    }
  }, [config])

  const configFields: ConfigField[] = [
    {
      key: "apiEndpoint",
      label: "API Endpoint",
      description: "The base URL for the CodeCoder API",
      type: "text",
      placeholder: "https://api.codecoder.ai",
    },
    {
      key: "apiKey",
      label: "API Key",
      description: "Your API key for authentication",
      type: "password",
      sensitive: true,
      placeholder: "sk-...",
    },
    {
      key: "userName",
      label: "Your Name",
      description: "Display name for the current user",
      type: "text",
      placeholder: "John Doe",
    },
    {
      key: "userEmail",
      label: "Email",
      description: "Email address for notifications",
      type: "text",
      placeholder: "john@example.com",
    },
    {
      key: "maxTokens",
      label: "Max Tokens",
      description: "Maximum tokens for AI responses",
      type: "number",
      placeholder: "4096",
    },
    {
      key: "temperature",
      label: "Temperature",
      description: "AI response randomness (0.0 - 1.0)",
      type: "number",
      placeholder: "0.7",
    },
  ]

  const handleFieldChange = (key: string, value: unknown) => {
    setFormData((prev) => ({ ...prev, [key]: value }))
  }

  const handleSave = async () => {
    try {
      await onSave(formData as Partial<ConfigData>)
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
          <div key={i} className="space-y-2 animate-pulse">
            <div className="h-4 w-32 bg-muted rounded" />
            <div className="h-10 bg-muted rounded" />
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
        <Button type="submit" disabled={!hasChanges || isSaving}>
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
          <div key={i} className="p-4 rounded-lg border animate-pulse">
            <div className="h-5 w-32 bg-muted rounded mb-2" />
            <div className="h-4 w-full bg-muted rounded mb-1" />
            <div className="h-4 w-2/3 bg-muted rounded" />
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
                    <span className="text-xs px-2 py-0.5 rounded-full bg-primary/10 text-primary">
                      {agent.category}
                    </span>
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

interface PermissionRule {
  id: string
  tool: string
  action: "allow" | "deny" | "prompt"
  description?: string
}

function PermissionSettings() {
  const [rules, setRules] = React.useState<PermissionRule[]>([
    {
      id: "file-write",
      tool: "file.write",
      action: "prompt",
      description: "Prompt before writing files",
    },
    {
      id: "file-delete",
      tool: "file.delete",
      action: "prompt",
      description: "Prompt before deleting files",
    },
    {
      id: "shell-exec",
      tool: "shell.exec",
      action: "prompt",
      description: "Prompt before executing shell commands",
    },
  ])

  const updateRule = (id: string, action: "allow" | "deny" | "prompt") => {
    setRules((prev) =>
      prev.map((rule) => (rule.id === id ? { ...rule, action } : rule))
    )
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">
        Configure how CodeCoder handles sensitive operations.
      </p>

      {rules.map((rule) => (
        <div key={rule.id} className="flex items-center justify-between p-4 rounded-lg border">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <Shield className="h-4 w-4 text-primary" />
              <code className="text-sm font-medium">{rule.tool}</code>
            </div>
            {rule.description && (
              <p className="text-sm text-muted-foreground">{rule.description}</p>
            )}
          </div>

          <div className="flex gap-1">
            {(["allow", "prompt", "deny"] as const).map((action) => (
              <Button
                key={action}
                variant={rule.action === action ? "default" : "outline"}
                size="sm"
                onClick={() => updateRule(rule.id, action)}
                className="capitalize"
              >
                {action}
              </Button>
            ))}
          </div>
        </div>
      ))}
    </div>
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
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="general">General</TabsTrigger>
            <TabsTrigger value="agents">Agents</TabsTrigger>
            <TabsTrigger value="permissions">Permissions</TabsTrigger>
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
