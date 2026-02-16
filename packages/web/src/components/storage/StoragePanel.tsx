/**
 * Storage Panel Component
 *
 * Displays storage information and management controls:
 * - Session count
 * - Estimated storage usage
 * - Cache management
 */

import * as React from "react"
import {
  Trash2,
  RefreshCw,
  HardDrive,
  FileText,
  CheckCircle,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
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
import { useSessions, useSessionStore } from "@/stores"

// ============================================================================
// Storage Info Component
// ============================================================================

function StorageInfo() {
  const sessions = useSessions()
  const [localStorageSize, setLocalStorageSize] = React.useState(0)

  // Calculate localStorage usage
  React.useEffect(() => {
    let total = 0
    for (const key in localStorage) {
      if (Object.prototype.hasOwnProperty.call(localStorage, key)) {
        total += localStorage[key].length * 2 // UTF-16 characters = 2 bytes
      }
    }
    setLocalStorageSize(total)
  }, [])

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`
  }

  return (
    <div className="grid grid-cols-3 gap-4">
      <Card>
        <CardContent className="p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-full bg-blue-500/10">
              <FileText className="h-5 w-5 text-blue-500" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Sessions</p>
              <p className="text-2xl font-bold">{sessions.length}</p>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-full bg-purple-500/10">
              <HardDrive className="h-5 w-5 text-purple-500" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Local Storage</p>
              <p className="text-2xl font-bold">{formatSize(localStorageSize)}</p>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-full bg-green-500/10">
              <CheckCircle className="h-5 w-5 text-green-500" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Status</p>
              <p className="text-lg font-medium">Healthy</p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Cache Management Component
// ============================================================================

function CacheManagement() {
  const { loadSessions } = useSessionStore()
  const [clearing, setClearing] = React.useState(false)
  const [cleared, setCleared] = React.useState(false)

  const clearCache = async () => {
    setClearing(true)
    try {
      // Clear specific cache keys (not all localStorage)
      const keysToRemove: string[] = []
      for (const key in localStorage) {
        if (
          key.startsWith("codecoder-") &&
          key !== "codecoder-theme" // Preserve theme preference
        ) {
          keysToRemove.push(key)
        }
      }

      for (const key of keysToRemove) {
        localStorage.removeItem(key)
      }

      // Refresh data
      await loadSessions()
      setCleared(true)
      setTimeout(() => setCleared(false), 3000)
    } finally {
      setClearing(false)
    }
  }

  const refreshData = async () => {
    setClearing(true)
    try {
      await loadSessions()
    } finally {
      setClearing(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Cache Management</CardTitle>
        <CardDescription>Manage local cached data</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex gap-2">
          <Button variant="outline" onClick={refreshData} disabled={clearing}>
            {clearing ? (
              <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4 mr-2" />
            )}
            Refresh Data
          </Button>

          <Button variant="outline" onClick={clearCache} disabled={clearing}>
            {cleared ? (
              <>
                <CheckCircle className="h-4 w-4 mr-2 text-green-500" />
                Cleared!
              </>
            ) : (
              <>
                <Trash2 className="h-4 w-4 mr-2" />
                Clear Cache
              </>
            )}
          </Button>
        </div>

        <p className="text-sm text-muted-foreground">
          Clearing cache will remove locally stored preferences (except theme).
          Your sessions are stored on the server and will not be affected.
        </p>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Local Storage Keys Component
// ============================================================================

function LocalStorageKeys() {
  const [keys, setKeys] = React.useState<Array<{ key: string; size: number }>>([])
  const [keyToRemove, setKeyToRemove] = React.useState<string | null>(null)

  React.useEffect(() => {
    const items: Array<{ key: string; size: number }> = []
    for (const key in localStorage) {
      if (Object.prototype.hasOwnProperty.call(localStorage, key)) {
        items.push({
          key,
          size: localStorage[key].length * 2,
        })
      }
    }
    items.sort((a, b) => b.size - a.size)
    setKeys(items)
  }, [])

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    return `${(bytes / 1024).toFixed(1)} KB`
  }

  const removeKey = (key: string) => {
    localStorage.removeItem(key)
    setKeys((prev) => prev.filter((k) => k.key !== key))
    setKeyToRemove(null)
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Local Storage Details</CardTitle>
        <CardDescription>Individual storage entries</CardDescription>
      </CardHeader>
      <CardContent>
        {keys.length === 0 ? (
          <p className="text-sm text-muted-foreground">No local storage entries</p>
        ) : (
          <div className="space-y-2 max-h-64 overflow-y-auto">
            {keys.map(({ key, size }) => (
              <div
                key={key}
                className="flex items-center justify-between p-2 bg-muted rounded-md"
              >
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-mono truncate">{key}</p>
                  <p className="text-xs text-muted-foreground">{formatSize(size)}</p>
                </div>
                <AlertDialog open={keyToRemove === key} onOpenChange={(open) => !open && setKeyToRemove(null)}>
                  <AlertDialogTrigger asChild>
                    <Button
                      size="icon"
                      variant="ghost"
                      className="h-6 w-6 text-muted-foreground hover:text-destructive"
                      onClick={() => setKeyToRemove(key)}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </AlertDialogTrigger>
                  <AlertDialogContent>
                    <AlertDialogHeader>
                      <AlertDialogTitle>Remove storage entry?</AlertDialogTitle>
                      <AlertDialogDescription>
                        Are you sure you want to remove "{key}" from local storage?
                        This action cannot be undone.
                      </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                      <AlertDialogCancel>Cancel</AlertDialogCancel>
                      <AlertDialogAction onClick={() => removeKey(key)}>
                        Remove
                      </AlertDialogAction>
                    </AlertDialogFooter>
                  </AlertDialogContent>
                </AlertDialog>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Main Storage Panel Component
// ============================================================================

export function StoragePanel() {
  return (
    <div className="space-y-6">
      <StorageInfo />
      <CacheManagement />
      <LocalStorageKeys />
    </div>
  )
}
