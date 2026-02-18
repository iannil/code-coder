/**
 * LSP Panel Component
 *
 * Displays and manages Language Server Protocol:
 * - LSP server status
 * - Diagnostics overview
 * - Configuration
 */

import * as React from "react"
import {
  Code,
  AlertCircle,
  AlertTriangle,
  Info,
  CheckCircle,
  XCircle,
  RefreshCw,
  Settings,
  FileWarning,
  Zap,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { Skeleton } from "@/components/ui/Skeleton"
import { Badge } from "@/components/ui/Badge"
import {
  useLspStore,
  useLspServers,
  useLspStatusLoading,
  useLspDiagnostics,
  useLspConfig,
  useConnectedServersCount,
  useErrorServersCount,
  useTotalDiagnostics,
} from "@/stores"

// ============================================================================
// Severity Badge Component
// ============================================================================

type BadgeVariant = "destructive" | "warning" | "info" | "secondary"

function SeverityBadge({ severity }: { severity: 1 | 2 | 3 | 4 }) {
  const configs: Record<number, { label: string; variant: BadgeVariant; icon: React.ReactNode }> = {
    1: {
      label: "Error",
      variant: "destructive",
      icon: <XCircle className="h-3 w-3" />,
    },
    2: {
      label: "Warning",
      variant: "warning",
      icon: <AlertTriangle className="h-3 w-3" />,
    },
    3: {
      label: "Info",
      variant: "info",
      icon: <Info className="h-3 w-3" />,
    },
    4: {
      label: "Hint",
      variant: "secondary",
      icon: <Info className="h-3 w-3" />,
    },
  }

  const config = configs[severity]

  return (
    <Badge variant={config.variant} className="gap-1">
      {config.icon}
      {config.label}
    </Badge>
  )
}

// ============================================================================
// Server Status Component
// ============================================================================

function ServerStatus() {
  const servers = useLspServers()
  const isLoading = useLspStatusLoading()
  const connectedCount = useConnectedServersCount()
  const errorCount = useErrorServersCount()
  const fetchStatus = useLspStore((s) => s.fetchStatus)
  const initLsp = useLspStore((s) => s.initLsp)

  // Track initialization to prevent infinite loop
  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchStatus()
  }, [fetchStatus])

  if (isLoading) {
    return (
      <div className="space-y-3">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-16 w-full" />
        ))}
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {/* Summary */}
      <div className="grid grid-cols-3 gap-4">
        <Card>
          <CardContent className="p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-full bg-primary/10">
                <Code className="h-5 w-5 text-primary" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Total Servers</p>
                <p className="text-2xl font-bold">{servers.length}</p>
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
                <p className="text-sm text-muted-foreground">Connected</p>
                <p className="text-2xl font-bold">{connectedCount}</p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-full bg-red-500/10">
                <XCircle className="h-5 w-5 text-red-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Errors</p>
                <p className="text-2xl font-bold">{errorCount}</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Server List */}
      {servers.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <Code className="h-12 w-12 mb-4" />
          <p>No LSP servers configured</p>
          <Button onClick={initLsp} className="mt-4">
            <Zap className="h-4 w-4 mr-2" />
            Initialize LSP
          </Button>
        </div>
      ) : (
        <div className="space-y-2">
          {servers.map((server) => (
            <Card key={server.id}>
              <CardContent className="p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <Code className="h-5 w-5 text-muted-foreground" />
                    <div>
                      <p className="font-medium">{server.name}</p>
                      <p className="text-sm text-muted-foreground font-mono">
                        {server.root}
                      </p>
                    </div>
                  </div>
                  <Badge
                    variant={server.status === "connected" ? "success" : "destructive"}
                    className="gap-1"
                  >
                    {server.status === "connected" ? (
                      <CheckCircle className="h-3 w-3" />
                    ) : (
                      <XCircle className="h-3 w-3" />
                    )}
                    {server.status}
                  </Badge>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-2">
        <Button variant="outline" onClick={() => fetchStatus()}>
          <RefreshCw className="h-4 w-4 mr-2" />
          Refresh Status
        </Button>
        <Button variant="outline" onClick={initLsp}>
          <Zap className="h-4 w-4 mr-2" />
          Reinitialize
        </Button>
      </div>
    </div>
  )
}

// ============================================================================
// Diagnostics Component
// ============================================================================

function DiagnosticsPanel() {
  const diagnostics = useLspDiagnostics()
  const totalCount = useTotalDiagnostics()
  const fetchDiagnostics = useLspStore((s) => s.fetchDiagnostics)

  // Track initialization to prevent infinite loop
  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchDiagnostics()
  }, [fetchDiagnostics])

  // Count by severity
  const counts = { errors: 0, warnings: 0, info: 0, hints: 0 }
  for (const file of diagnostics) {
    for (const diag of file.diagnostics) {
      if (diag.severity === 1) counts.errors++
      else if (diag.severity === 2) counts.warnings++
      else if (diag.severity === 3) counts.info++
      else if (diag.severity === 4) counts.hints++
    }
  }

  return (
    <div className="space-y-4">
      {/* Summary */}
      <div className="grid grid-cols-4 gap-3">
        <Card>
          <CardContent className="p-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Errors</span>
              <span className="text-lg font-bold text-red-600">{counts.errors}</span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Warnings</span>
              <span className="text-lg font-bold text-amber-600">{counts.warnings}</span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Info</span>
              <span className="text-lg font-bold text-blue-600">{counts.info}</span>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Hints</span>
              <span className="text-lg font-bold text-gray-600">{counts.hints}</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Diagnostics List */}
      {totalCount === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <CheckCircle className="h-12 w-12 mb-4 text-green-500" />
          <p>No diagnostics</p>
          <p className="text-sm">Your code looks good!</p>
        </div>
      ) : (
        <div className="space-y-3 max-h-[400px] overflow-y-auto">
          {diagnostics.map(
            (file) =>
              file.diagnostics.length > 0 && (
                <Card key={file.filePath}>
                  <CardHeader className="py-3">
                    <CardTitle className="text-sm font-mono flex items-center gap-2">
                      <FileWarning className="h-4 w-4" />
                      {file.filePath}
                      <span className="text-muted-foreground font-normal">
                        ({file.diagnostics.length})
                      </span>
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="pt-0">
                    <div className="space-y-2">
                      {file.diagnostics.map((diag, index) => (
                        <div
                          key={index}
                          className="p-2 bg-muted rounded-md text-sm"
                        >
                          <div className="flex items-center gap-2 mb-1">
                            <SeverityBadge severity={diag.severity} />
                            <span className="text-muted-foreground text-xs">
                              Line {diag.range.start.line + 1}:
                              {diag.range.start.character + 1}
                            </span>
                            {diag.source && (
                              <span className="text-xs text-muted-foreground">
                                [{diag.source}]
                              </span>
                            )}
                          </div>
                          <p className="font-mono text-xs">{diag.message}</p>
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )
          )}
        </div>
      )}

      {/* Refresh */}
      <Button variant="outline" onClick={() => fetchDiagnostics()}>
        <RefreshCw className="h-4 w-4 mr-2" />
        Refresh Diagnostics
      </Button>
    </div>
  )
}

// ============================================================================
// Configuration Component
// ============================================================================

function ConfigurationPanel() {
  const config = useLspConfig()
  const fetchConfig = useLspStore((s) => s.fetchConfig)

  // Track initialization to prevent infinite loop
  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchConfig()
  }, [fetchConfig])

  if (!config) {
    return (
      <div className="flex items-center justify-center py-12">
        <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Settings className="h-4 w-4" />
            LSP Settings
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">LSP Enabled</p>
              <p className="text-sm text-muted-foreground">
                Whether LSP integration is enabled
              </p>
            </div>
            <Badge variant={config.enabled ? "success" : "destructive"}>
              {config.enabled ? "Enabled" : "Disabled"}
            </Badge>
          </div>
        </CardContent>
      </Card>

      {config.servers && Object.keys(config.servers).length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Configured Servers</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {Object.entries(config.servers).map(([name, server]) => (
                <div
                  key={name}
                  className="p-3 bg-muted rounded-md"
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="font-medium">{name}</span>
                    {server.disabled && (
                      <Badge variant="destructive">
                        Disabled
                      </Badge>
                    )}
                  </div>
                  <code className="text-xs font-mono block mb-2">
                    {server.command.join(" ")}
                  </code>
                  {server.extensions && server.extensions.length > 0 && (
                    <div className="flex flex-wrap gap-1">
                      {server.extensions.map((ext) => (
                        <Badge key={ext} variant="outline">
                          {ext}
                        </Badge>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

// ============================================================================
// Main LSP Panel Component
// ============================================================================

export function LspPanel() {
  return (
    <Tabs defaultValue="status" className="space-y-4">
      <TabsList>
        <TabsTrigger value="status" className="flex items-center gap-2">
          <Code className="h-4 w-4" />
          Servers
        </TabsTrigger>
        <TabsTrigger value="diagnostics" className="flex items-center gap-2">
          <AlertCircle className="h-4 w-4" />
          Diagnostics
        </TabsTrigger>
        <TabsTrigger value="config" className="flex items-center gap-2">
          <Settings className="h-4 w-4" />
          Configuration
        </TabsTrigger>
      </TabsList>

      <TabsContent value="status">
        <ServerStatus />
      </TabsContent>

      <TabsContent value="diagnostics">
        <DiagnosticsPanel />
      </TabsContent>

      <TabsContent value="config">
        <ConfigurationPanel />
      </TabsContent>
    </Tabs>
  )
}
