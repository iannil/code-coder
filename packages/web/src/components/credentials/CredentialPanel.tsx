/**
 * Credential Panel Component
 *
 * Displays and manages credentials:
 * - List all credentials (API keys, OAuth, Login, Bearer tokens)
 * - Add new credentials
 * - Delete credentials
 */

import * as React from "react"
import {
  Key,
  Shield,
  User,
  Lock,
  Plus,
  Trash2,
  RefreshCw,
  Globe,
  Clock,
  AlertCircle,
} from "lucide-react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card"
import { Button } from "@/components/ui/Button"
import { Badge } from "@/components/ui/Badge"
import { Skeleton } from "@/components/ui/Skeleton"
import { Separator } from "@/components/ui/Separator"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/AlertDialog"
import { useToast } from "@/hooks/use-toast"
import { useCredentialStore, useCredentials, useCredentialLoading, useCredentialError } from "@/stores/credential"
import type { CredentialSummary, CredentialType } from "@/lib/types"
import { CredentialForm } from "./CredentialForm"

// ============================================================================
// Type Badge Component
// ============================================================================

type BadgeVariant = "info" | "success" | "purple" | "warning"

const TYPE_VARIANTS: Record<CredentialType, BadgeVariant> = {
  api_key: "info",
  oauth: "success",
  login: "purple",
  bearer_token: "warning",
}

const TYPE_ICONS: Record<CredentialType, React.ReactNode> = {
  api_key: <Key className="h-3 w-3" />,
  oauth: <Shield className="h-3 w-3" />,
  login: <User className="h-3 w-3" />,
  bearer_token: <Lock className="h-3 w-3" />,
}

const TYPE_LABELS: Record<CredentialType, string> = {
  api_key: "API Key",
  oauth: "OAuth",
  login: "Login",
  bearer_token: "Bearer Token",
}

function TypeBadge({ type }: { type: CredentialType }) {
  return (
    <Badge variant={TYPE_VARIANTS[type]} className="gap-1">
      {TYPE_ICONS[type]}
      {TYPE_LABELS[type]}
    </Badge>
  )
}

// ============================================================================
// Credential Card Component
// ============================================================================

interface CredentialCardProps {
  credential: CredentialSummary
  onDelete: (id: string) => void
  isDeleting: boolean
}

function CredentialCard({ credential, onDelete, isDeleting }: CredentialCardProps) {
  const [showDeleteDialog, setShowDeleteDialog] = React.useState(false)

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr)
    return date.toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    })
  }

  return (
    <>
      <Card>
        <CardContent className="p-4">
          <div className="flex items-start justify-between">
            <div className="flex items-start gap-3 flex-1 min-w-0">
              <div className="p-2 bg-muted rounded-md">
                {TYPE_ICONS[credential.type]}
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <h4 className="font-medium truncate">{credential.name}</h4>
                  <TypeBadge type={credential.type} />
                </div>
                <p className="text-sm text-muted-foreground mb-2">
                  {credential.service}
                </p>
                {credential.patterns.length > 0 && (
                  <div className="flex items-center gap-1 flex-wrap mb-2">
                    <Globe className="h-3 w-3 text-muted-foreground" />
                    {credential.patterns.slice(0, 3).map((pattern) => (
                      <Badge key={pattern} variant="outline" className="font-mono text-xs">
                        {pattern}
                      </Badge>
                    ))}
                    {credential.patterns.length > 3 && (
                      <Badge variant="outline" className="text-xs">
                        +{credential.patterns.length - 3} more
                      </Badge>
                    )}
                  </div>
                )}
                <div className="flex items-center gap-3 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    Created {formatDate(credential.createdAt)}
                  </span>
                </div>
              </div>
            </div>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setShowDeleteDialog(true)}
              disabled={isDeleting}
              className="text-muted-foreground hover:text-destructive"
            >
              {isDeleting ? (
                <RefreshCw className="h-4 w-4 animate-spin" />
              ) : (
                <Trash2 className="h-4 w-4" />
              )}
            </Button>
          </div>
        </CardContent>
      </Card>

      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Credential</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{credential.name}"? This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                onDelete(credential.id)
                setShowDeleteDialog(false)
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}

// ============================================================================
// Credentials List Component
// ============================================================================

function CredentialsList() {
  const credentials = useCredentials()
  const { isLoading, isLoaded, isDeleting } = useCredentialLoading()
  const error = useCredentialError()
  const fetchCredentials = useCredentialStore((s) => s.fetchCredentials)
  const deleteCredential = useCredentialStore((s) => s.deleteCredential)
  const { toast } = useToast()

  // Track initialization to prevent infinite loop
  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchCredentials()
  }, [fetchCredentials])

  const handleDelete = async (id: string) => {
    try {
      await deleteCredential(id)
      toast({
        title: "Credential deleted",
        description: "The credential has been removed successfully.",
      })
    } catch {
      toast({
        title: "Failed to delete",
        description: "An error occurred while deleting the credential.",
        variant: "destructive",
      })
    }
  }

  if (isLoading && !isLoaded) {
    return (
      <div className="space-y-3">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-24 w-full" />
        ))}
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <AlertCircle className="h-12 w-12 mb-4 text-destructive" />
        <p>Failed to load credentials</p>
        <p className="text-sm">{error}</p>
        <Button
          variant="outline"
          className="mt-4"
          onClick={() => fetchCredentials()}
        >
          <RefreshCw className="mr-2 h-4 w-4" />
          Retry
        </Button>
      </div>
    )
  }

  if (credentials.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Key className="h-12 w-12 mb-4" />
        <p>No credentials stored</p>
        <p className="text-sm">Add credentials to enable automatic authentication</p>
      </div>
    )
  }

  // Group credentials by type
  const groupedCredentials: Record<CredentialType, CredentialSummary[]> = {
    api_key: [],
    oauth: [],
    login: [],
    bearer_token: [],
  }

  for (const cred of credentials) {
    groupedCredentials[cred.type].push(cred)
  }

  return (
    <div className="space-y-6">
      {/* Summary */}
      <div className="grid grid-cols-4 gap-3">
        {(Object.entries(groupedCredentials) as [CredentialType, CredentialSummary[]][]).map(
          ([type, creds]) => (
            <Card key={type}>
              <CardContent className="p-3">
                <div className="flex items-center justify-between">
                  <TypeBadge type={type} />
                  <span className="text-lg font-bold">{creds.length}</span>
                </div>
              </CardContent>
            </Card>
          )
        )}
      </div>

      {/* Credentials by type */}
      {(Object.entries(groupedCredentials) as [CredentialType, CredentialSummary[]][]).map(
        ([type, creds]) =>
          creds.length > 0 && (
            <div key={type}>
              <h3 className="text-sm font-medium mb-3 flex items-center gap-2">
                <TypeBadge type={type} />
                <span className="text-muted-foreground">
                  ({creds.length} credential{creds.length !== 1 ? "s" : ""})
                </span>
              </h3>
              <div className="space-y-2">
                {creds.map((cred) => (
                  <CredentialCard
                    key={cred.id}
                    credential={cred}
                    onDelete={handleDelete}
                    isDeleting={isDeleting === cred.id}
                  />
                ))}
              </div>
            </div>
          )
      )}
    </div>
  )
}

// ============================================================================
// Main Credential Panel Component
// ============================================================================

export function CredentialPanel() {
  const [showAddForm, setShowAddForm] = React.useState(false)
  const { toast } = useToast()

  const handleAddSuccess = () => {
    setShowAddForm(false)
    toast({
      title: "Credential added",
      description: "The credential has been stored securely.",
    })
  }

  return (
    <div className="space-y-6">
      {/* Add Credential Button / Form */}
      {!showAddForm ? (
        <Button
          variant="outline"
          onClick={() => setShowAddForm(true)}
          className="w-full border-dashed"
        >
          <Plus className="mr-2 h-4 w-4" />
          Add new credential
        </Button>
      ) : (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base flex items-center gap-2">
              <Key className="h-4 w-4" />
              Add Credential
            </CardTitle>
          </CardHeader>
          <CardContent>
            <CredentialForm
              onSuccess={handleAddSuccess}
              onCancel={() => setShowAddForm(false)}
            />
          </CardContent>
        </Card>
      )}

      <Separator />

      {/* Credentials List */}
      <CredentialsList />
    </div>
  )
}
