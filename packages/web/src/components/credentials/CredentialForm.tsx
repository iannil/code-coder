/**
 * Credential Form Component
 *
 * Form for adding or editing credentials with dynamic fields based on type
 */

import * as React from "react"
import { Plus, X, Check, Key, Shield, User, Lock, RefreshCw } from "lucide-react"
import { Button } from "@/components/ui/Button"
import { Input } from "@/components/ui/Input"
import { Label } from "@/components/ui/Label"
import { Badge } from "@/components/ui/Badge"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select"
import { useCredentialStore, useCredentialLoading } from "@/stores/credential"
import type { CredentialType, CredentialCreateInput, CredentialEntry } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface CredentialFormProps {
  onSuccess: () => void
  onCancel: () => void
  editCredential?: CredentialEntry
}

// ============================================================================
// Type Options
// ============================================================================

const TYPE_OPTIONS: Array<{ value: CredentialType; label: string; icon: React.ReactNode; description: string }> = [
  {
    value: "api_key",
    label: "API Key",
    icon: <Key className="h-4 w-4" />,
    description: "For services that use API key authentication",
  },
  {
    value: "bearer_token",
    label: "Bearer Token",
    icon: <Lock className="h-4 w-4" />,
    description: "For services that use Bearer token authentication",
  },
  {
    value: "oauth",
    label: "OAuth",
    icon: <Shield className="h-4 w-4" />,
    description: "For OAuth 2.0 authentication flows",
  },
  {
    value: "login",
    label: "Login Credentials",
    icon: <User className="h-4 w-4" />,
    description: "Username and password for web logins",
  },
]

// ============================================================================
// Pattern Input Component
// ============================================================================

interface PatternInputProps {
  patterns: string[]
  onChange: (patterns: string[]) => void
}

function PatternInput({ patterns, onChange }: PatternInputProps) {
  const [inputValue, setInputValue] = React.useState("")

  const addPattern = () => {
    if (inputValue.trim() && !patterns.includes(inputValue.trim())) {
      onChange([...patterns, inputValue.trim()])
      setInputValue("")
    }
  }

  const removePattern = (pattern: string) => {
    onChange(patterns.filter((p) => p !== pattern))
  }

  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        <Input
          placeholder="e.g., *.github.com or api.openai.com"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault()
              addPattern()
            }
          }}
        />
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={addPattern}
          disabled={!inputValue.trim()}
        >
          <Plus className="h-4 w-4" />
        </Button>
      </div>
      {patterns.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {patterns.map((pattern) => (
            <Badge key={pattern} variant="secondary" className="gap-1">
              {pattern}
              <button
                type="button"
                onClick={() => removePattern(pattern)}
                className="ml-1 hover:text-destructive"
              >
                <X className="h-3 w-3" />
              </button>
            </Badge>
          ))}
        </div>
      )}
    </div>
  )
}

// ============================================================================
// Main Form Component
// ============================================================================

export function CredentialForm({ onSuccess, onCancel, editCredential }: CredentialFormProps) {
  const isEditMode = !!editCredential
  const [type, setType] = React.useState<CredentialType>(editCredential?.type ?? "api_key")
  const [name, setName] = React.useState(editCredential?.name ?? "")
  const [service, setService] = React.useState(editCredential?.service ?? "")
  const [patterns, setPatterns] = React.useState<string[]>(editCredential?.patterns ?? [])

  // API Key / Bearer Token
  const [apiKey, setApiKey] = React.useState(editCredential?.apiKey ?? "")

  // OAuth
  const [clientId, setClientId] = React.useState(editCredential?.oauth?.clientId ?? "")
  const [clientSecret, setClientSecret] = React.useState(editCredential?.oauth?.clientSecret ?? "")

  // Login
  const [username, setUsername] = React.useState(editCredential?.login?.username ?? "")
  const [password, setPassword] = React.useState(editCredential?.login?.password ?? "")
  const [totpSecret, setTotpSecret] = React.useState(editCredential?.login?.totpSecret ?? "")

  const { isAdding, isUpdating } = useCredentialLoading()
  const addCredential = useCredentialStore((s) => s.addCredential)
  const updateCredential = useCredentialStore((s) => s.updateCredential)
  const isSubmitting = isAdding || isUpdating === editCredential?.id

  const isValid = () => {
    if (!name.trim() || !service.trim()) return false

    // In edit mode, sensitive fields are optional (empty means keep existing)
    if (isEditMode) return true

    switch (type) {
      case "api_key":
      case "bearer_token":
        return apiKey.trim().length > 0
      case "oauth":
        return clientId.trim().length > 0
      case "login":
        return username.trim().length > 0 && password.trim().length > 0
      default:
        return false
    }
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    const input: CredentialCreateInput = {
      type,
      name: name.trim(),
      service: service.trim(),
      patterns,
    }

    switch (type) {
      case "api_key":
      case "bearer_token":
        if (apiKey.trim()) input.apiKey = apiKey.trim()
        break
      case "oauth":
        if (clientId.trim() || clientSecret.trim()) {
          input.oauth = {
            clientId: clientId.trim(),
            clientSecret: clientSecret.trim() || undefined,
          }
        }
        break
      case "login":
        if (username.trim() || password.trim()) {
          input.login = {
            username: username.trim(),
            password: password.trim(),
            totpSecret: totpSecret.trim() || undefined,
          }
        }
        break
    }

    try {
      if (isEditMode && editCredential) {
        await updateCredential(editCredential.id, input)
      } else {
        await addCredential(input)
      }
      onSuccess()
    } catch {
      // Error is handled by the store
    }
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      {/* Type Selection */}
      <div className="space-y-2">
        <Label htmlFor="type">Credential Type</Label>
        <Select value={type} onValueChange={(v) => setType(v as CredentialType)}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {TYPE_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                <div className="flex items-center gap-2">
                  {option.icon}
                  <span>{option.label}</span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-sm text-muted-foreground">
          {TYPE_OPTIONS.find((o) => o.value === type)?.description}
        </p>
      </div>

      {/* Basic Info */}
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label htmlFor="name">Name</Label>
          <Input
            id="name"
            placeholder="e.g., GitHub Production"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="service">Service</Label>
          <Input
            id="service"
            placeholder="e.g., github, openai"
            value={service}
            onChange={(e) => setService(e.target.value)}
          />
        </div>
      </div>

      {/* Type-specific fields */}
      {(type === "api_key" || type === "bearer_token") && (
        <div className="space-y-2">
          <Label htmlFor="apiKey">
            {type === "api_key" ? "API Key" : "Bearer Token"}
          </Label>
          <Input
            id="apiKey"
            type="password"
            autoComplete="off"
            placeholder={isEditMode ? "Leave empty to keep existing" : (type === "api_key" ? "sk-..." : "token...")}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
          />
          {isEditMode && (
            <p className="text-sm text-muted-foreground">
              Leave empty to keep the existing value
            </p>
          )}
        </div>
      )}

      {type === "oauth" && (
        <>
          <div className="space-y-2">
            <Label htmlFor="clientId">Client ID</Label>
            <Input
              id="clientId"
              autoComplete="off"
              placeholder={isEditMode ? "Leave empty to keep existing" : "Client ID"}
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="clientSecret">Client Secret (optional)</Label>
            <Input
              id="clientSecret"
              type="password"
              autoComplete="off"
              placeholder={isEditMode ? "Leave empty to keep existing" : "Client Secret"}
              value={clientSecret}
              onChange={(e) => setClientSecret(e.target.value)}
            />
          </div>
          {isEditMode && (
            <p className="text-sm text-muted-foreground">
              Leave fields empty to keep existing values
            </p>
          )}
        </>
      )}

      {type === "login" && (
        <>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="username">Username</Label>
              <Input
                id="username"
                autoComplete="off"
                placeholder={isEditMode ? "Leave empty to keep existing" : "Username or email"}
                value={username}
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                autoComplete="new-password"
                placeholder={isEditMode ? "Leave empty to keep existing" : "Password"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="totpSecret">TOTP Secret (optional)</Label>
            <Input
              id="totpSecret"
              type="password"
              autoComplete="off"
              placeholder={isEditMode ? "Leave empty to keep existing" : "For 2FA auto-fill"}
              value={totpSecret}
              onChange={(e) => setTotpSecret(e.target.value)}
            />
            <p className="text-sm text-muted-foreground">
              {isEditMode ? "Leave fields empty to keep existing values" : "If provided, 2FA codes will be generated automatically"}
            </p>
          </div>
        </>
      )}

      {/* URL Patterns */}
      <div className="space-y-2">
        <Label>URL Patterns (optional)</Label>
        <PatternInput patterns={patterns} onChange={setPatterns} />
        <p className="text-sm text-muted-foreground">
          URLs that match these patterns will use this credential automatically
        </p>
      </div>

      {/* Actions */}
      <div className="flex gap-2 pt-2">
        <Button type="submit" disabled={!isValid() || isSubmitting}>
          {isSubmitting ? (
            <>
              <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
              {isEditMode ? "Updating..." : "Adding..."}
            </>
          ) : (
            <>
              <Check className="mr-2 h-4 w-4" />
              {isEditMode ? "Update Credential" : "Add Credential"}
            </>
          )}
        </Button>
        <Button type="button" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </form>
  )
}
