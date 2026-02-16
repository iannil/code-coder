/**
 * Hooks Panel Component
 *
 * Displays and manages the hooks system:
 * - Hook entries by lifecycle
 * - Hook settings
 * - Hook file locations
 * - Action type reference
 */

import * as React from "react"
import {
  Zap,
  Settings,
  FileCode,
  Info,
  ChevronRight,
  Play,
  Square,
  AlertTriangle,
  CheckCircle,
  RefreshCw,
} from "lucide-react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { Badge } from "@/components/ui/Badge"
import { Skeleton } from "@/components/ui/Skeleton"
import {
  useHooksStore,
  useHooks,
  useHooksLoading,
  useHooksSettings,
  useHooksLocations,
  useHooksActionTypes,
  useHookCounts,
} from "@/stores"
import { cn } from "@/lib/utils"
import type { HookLifecycle, HookEntry } from "@/lib/types"

// ============================================================================
// Lifecycle Badge Component
// ============================================================================

type BadgeVariant = "info" | "success" | "purple" | "destructive"

const LIFECYCLE_VARIANTS: Record<HookLifecycle, BadgeVariant> = {
  PreToolUse: "info",
  PostToolUse: "success",
  PreResponse: "purple",
  Stop: "destructive",
}

const LIFECYCLE_ICONS: Record<HookLifecycle, React.ReactNode> = {
  PreToolUse: <Play className="h-3 w-3" />,
  PostToolUse: <CheckCircle className="h-3 w-3" />,
  PreResponse: <AlertTriangle className="h-3 w-3" />,
  Stop: <Square className="h-3 w-3" />,
}

function LifecycleBadge({ lifecycle }: { lifecycle: HookLifecycle }) {
  return (
    <Badge variant={LIFECYCLE_VARIANTS[lifecycle]} className="gap-1">
      {LIFECYCLE_ICONS[lifecycle]}
      {lifecycle}
    </Badge>
  )
}

// ============================================================================
// Hook Entry Card Component
// ============================================================================

function HookEntryCard({ hook }: { hook: HookEntry }) {
  const [expanded, setExpanded] = React.useState(false)

  return (
    <Card>
      <CardHeader
        className="cursor-pointer py-3"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Zap className="h-4 w-4 text-muted-foreground" />
            <div>
              <h4 className="font-medium">{hook.name}</h4>
              {hook.definition.description && (
                <p className="text-sm text-muted-foreground">
                  {hook.definition.description}
                </p>
              )}
            </div>
          </div>
          <div className="flex items-center gap-2">
            <LifecycleBadge lifecycle={hook.lifecycle} />
            <ChevronRight
              className={cn(
                "h-4 w-4 transition-transform",
                expanded && "rotate-90"
              )}
            />
          </div>
        </div>
      </CardHeader>
      {expanded && (
        <CardContent className="pt-0">
          <div className="space-y-3">
            {hook.definition.pattern && (
              <div>
                <span className="text-sm text-muted-foreground">Pattern:</span>
                <code className="ml-2 px-2 py-0.5 bg-muted rounded text-sm">
                  {hook.definition.pattern}
                </code>
              </div>
            )}
            {hook.definition.command_pattern && (
              <div>
                <span className="text-sm text-muted-foreground">
                  Command Pattern:
                </span>
                <code className="ml-2 px-2 py-0.5 bg-muted rounded text-sm">
                  {hook.definition.command_pattern}
                </code>
              </div>
            )}
            <div>
              <span className="text-sm text-muted-foreground">Actions:</span>
              <div className="mt-2 space-y-2">
                {hook.definition.actions.map((action, index) => (
                  <div
                    key={index}
                    className="p-2 bg-muted rounded-md text-sm font-mono"
                  >
                    <div className="flex items-center gap-2">
                      <span className="font-semibold">{action.type}</span>
                      {action.block && (
                        <Badge variant="destructive" className="text-xs">
                          blocking
                        </Badge>
                      )}
                      {action.async && (
                        <Badge variant="info" className="text-xs">
                          async
                        </Badge>
                      )}
                    </div>
                    {action.message && (
                      <p className="mt-1 text-muted-foreground">{action.message}</p>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </div>
        </CardContent>
      )}
    </Card>
  )
}

// ============================================================================
// Hooks List Component
// ============================================================================

function HooksList() {
  const hooks = useHooks()
  const isLoading = useHooksLoading()
  const counts = useHookCounts()
  const { fetchHooks } = useHooksStore()

  React.useEffect(() => {
    fetchHooks()
  }, [fetchHooks])

  if (isLoading) {
    return (
      <div className="space-y-3">
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-20 w-full" />
        ))}
      </div>
    )
  }

  if (hooks.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Zap className="h-12 w-12 mb-4" />
        <p>No hooks configured</p>
        <p className="text-sm">Add hooks in your config file</p>
      </div>
    )
  }

  // Group by lifecycle
  const groupedHooks: Record<HookLifecycle, HookEntry[]> = {
    PreToolUse: [],
    PostToolUse: [],
    PreResponse: [],
    Stop: [],
  }

  for (const hook of hooks) {
    groupedHooks[hook.lifecycle].push(hook)
  }

  return (
    <div className="space-y-6">
      {/* Summary */}
      <div className="grid grid-cols-4 gap-3">
        {(Object.entries(counts) as [HookLifecycle, number][]).map(
          ([lifecycle, count]) => (
            <Card key={lifecycle}>
              <CardContent className="p-3">
                <div className="flex items-center justify-between">
                  <LifecycleBadge lifecycle={lifecycle} />
                  <span className="text-lg font-bold">{count}</span>
                </div>
              </CardContent>
            </Card>
          )
        )}
      </div>

      {/* Hooks by lifecycle */}
      {(Object.entries(groupedHooks) as [HookLifecycle, HookEntry[]][]).map(
        ([lifecycle, lifecycleHooks]) =>
          lifecycleHooks.length > 0 && (
            <div key={lifecycle}>
              <h3 className="text-sm font-medium mb-3 flex items-center gap-2">
                <LifecycleBadge lifecycle={lifecycle} />
                <span className="text-muted-foreground">
                  ({lifecycleHooks.length} hooks)
                </span>
              </h3>
              <div className="space-y-2">
                {lifecycleHooks.map((hook) => (
                  <HookEntryCard key={`${hook.lifecycle}-${hook.name}`} hook={hook} />
                ))}
              </div>
            </div>
          )
      )}
    </div>
  )
}

// ============================================================================
// Settings Panel Component
// ============================================================================

function SettingsPanel() {
  const settings = useHooksSettings()
  const { fetchSettings } = useHooksStore()

  React.useEffect(() => {
    fetchSettings()
  }, [fetchSettings])

  if (!settings) {
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
          <CardTitle className="text-base">Hook Settings</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">Enabled</p>
              <p className="text-sm text-muted-foreground">
                Whether hooks are enabled
              </p>
            </div>
            <Badge variant={settings.enabled ? "success" : "destructive"}>
              {settings.enabled ? "Yes" : "No"}
            </Badge>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">Blocking Mode</p>
              <p className="text-sm text-muted-foreground">
                How blocking hooks are handled
              </p>
            </div>
            <Badge variant="outline">
              {settings.blocking_mode ?? "interactive"}
            </Badge>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">Log Level</p>
              <p className="text-sm text-muted-foreground">
                Hook logging verbosity
              </p>
            </div>
            <Badge variant="outline">
              {settings.log_level ?? "info"}
            </Badge>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Locations Panel Component
// ============================================================================

function LocationsPanel() {
  const locations = useHooksLocations()
  const { fetchLocations } = useHooksStore()

  React.useEffect(() => {
    fetchLocations()
  }, [fetchLocations])

  return (
    <div className="space-y-3">
      {locations.map((loc) => (
        <Card key={loc.path}>
          <CardContent className="p-4">
            <div className="flex items-start justify-between">
              <div className="flex items-start gap-3">
                <FileCode className="h-5 w-5 text-muted-foreground mt-0.5" />
                <div>
                  <p className="font-medium font-mono text-sm">{loc.path}</p>
                  <p className="text-sm text-muted-foreground">
                    {loc.description}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Badge variant={loc.scope === "global" ? "info" : "purple"}>
                  {loc.scope}
                </Badge>
                {loc.exists ? (
                  <CheckCircle className="h-4 w-4 text-green-500" />
                ) : (
                  <AlertTriangle className="h-4 w-4 text-amber-500" />
                )}
              </div>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

// ============================================================================
// Action Types Reference Component
// ============================================================================

function ActionTypesReference() {
  const actionTypes = useHooksActionTypes()
  const { fetchActionTypes } = useHooksStore()

  React.useEffect(() => {
    fetchActionTypes()
  }, [fetchActionTypes])

  return (
    <div className="space-y-3">
      {actionTypes.map((action) => (
        <Card key={action.type}>
          <CardContent className="p-4">
            <div className="flex items-start gap-3">
              <Info className="h-5 w-5 text-muted-foreground mt-0.5" />
              <div>
                <code className="font-mono text-sm font-semibold">
                  {action.type}
                </code>
                <p className="text-sm text-muted-foreground mt-1">
                  {action.description}
                </p>
                {action.params.length > 0 && (
                  <div className="mt-2 flex flex-wrap gap-1">
                    {action.params.map((param) => (
                      <Badge key={param} variant="outline" className="font-mono text-xs">
                        {param}
                      </Badge>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

// ============================================================================
// Main Hooks Panel Component
// ============================================================================

export function HooksPanel() {
  return (
    <Tabs defaultValue="hooks" className="space-y-4">
      <TabsList>
        <TabsTrigger value="hooks" className="flex items-center gap-2">
          <Zap className="h-4 w-4" />
          Hooks
        </TabsTrigger>
        <TabsTrigger value="settings" className="flex items-center gap-2">
          <Settings className="h-4 w-4" />
          Settings
        </TabsTrigger>
        <TabsTrigger value="locations" className="flex items-center gap-2">
          <FileCode className="h-4 w-4" />
          Locations
        </TabsTrigger>
        <TabsTrigger value="reference" className="flex items-center gap-2">
          <Info className="h-4 w-4" />
          Reference
        </TabsTrigger>
      </TabsList>

      <TabsContent value="hooks">
        <HooksList />
      </TabsContent>

      <TabsContent value="settings">
        <SettingsPanel />
      </TabsContent>

      <TabsContent value="locations">
        <LocationsPanel />
      </TabsContent>

      <TabsContent value="reference">
        <ActionTypesReference />
      </TabsContent>
    </Tabs>
  )
}
