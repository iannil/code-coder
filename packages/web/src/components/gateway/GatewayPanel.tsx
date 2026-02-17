/**
 * Gateway Panel Component
 *
 * Displays and manages the HTTP gateway:
 * - Gateway status (running/stopped)
 * - Start/stop controls
 * - Endpoint list
 * - Recent requests
 */

import * as React from "react"
import {
  Server,
  Power,
  RefreshCw,
  XCircle,
  Globe,
  Clock,
  ArrowRight,
} from "lucide-react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card"
import { Button } from "@/components/ui/Button"
import { Badge } from "@/components/ui/Badge"
import { Skeleton } from "@/components/ui/Skeleton"
import {
  useGatewayStore,
  useGatewayStatus,
  useGatewayLoading,
} from "@/stores"
import { cn } from "@/lib/utils"
import type { GatewayRequest } from "@/lib/types"

// ============================================================================
// Status Card Component
// ============================================================================

function StatusCard() {
  const status = useGatewayStatus()
  const { isStarting, isStopping } = useGatewayLoading()
  const start = useGatewayStore((s) => s.start)
  const stop = useGatewayStore((s) => s.stop)

  if (!status) return null

  const formatUptime = (ms: number) => {
    const hours = Math.floor(ms / 3600000)
    const minutes = Math.floor((ms % 3600000) / 60000)
    return `${hours}h ${minutes}m`
  }

  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div
              className={cn(
                "flex h-14 w-14 items-center justify-center rounded-xl",
                status.running
                  ? "bg-green-500/10 text-green-600"
                  : "bg-muted text-muted-foreground"
              )}
            >
              <Server className="h-7 w-7" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-xl font-semibold">HTTP Gateway</h3>
                <Badge variant={status.running ? "success" : "destructive"}>
                  {status.running ? "Running" : "Stopped"}
                </Badge>
              </div>
              {status.running && (
                <div className="flex items-center gap-4 mt-1 text-sm text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Globe className="h-3 w-3" />
                    {status.host}:{status.port}
                  </span>
                  {status.uptime && (
                    <span className="flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      Uptime: {formatUptime(status.uptime)}
                    </span>
                  )}
                </div>
              )}
            </div>
          </div>

          <Button
            variant={status.running ? "destructive" : "default"}
            onClick={status.running ? stop : start}
            disabled={isStarting || isStopping}
          >
            {isStarting || isStopping ? (
              <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Power className="mr-2 h-4 w-4" />
            )}
            {status.running ? "Stop" : "Start"}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Endpoints List Component
// ============================================================================

function EndpointsList() {
  const status = useGatewayStatus()

  if (!status?.endpoints?.length) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Endpoints</CardTitle>
        </CardHeader>
        <CardContent className="text-center py-8">
          <Globe className="h-10 w-10 text-muted-foreground mx-auto mb-2" />
          <p className="text-muted-foreground">No endpoints registered</p>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">
          Endpoints ({status.endpoints.length})
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {status.endpoints.map((endpoint) => (
          <div
            key={`${endpoint.method}-${endpoint.path}`}
            className="flex items-center justify-between p-3 rounded-lg bg-muted/50"
          >
            <div className="flex items-center gap-3">
              <Badge variant="outline" className="font-mono text-xs">
                {endpoint.method}
              </Badge>
              <code className="text-sm">{endpoint.path}</code>
            </div>
            {endpoint.description && (
              <span className="text-xs text-muted-foreground">
                {endpoint.description}
              </span>
            )}
          </div>
        ))}
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Request Item Component
// ============================================================================

function RequestItem({ request }: { request: GatewayRequest }) {
  const getStatusColor = (status: number) => {
    if (status >= 200 && status < 300) return "text-green-600"
    if (status >= 400 && status < 500) return "text-yellow-600"
    if (status >= 500) return "text-red-600"
    return "text-muted-foreground"
  }

  return (
    <div className="flex items-center justify-between p-3 rounded-lg border">
      <div className="flex items-center gap-3">
        <Badge variant="outline" className="font-mono text-xs">
          {request.method}
        </Badge>
        <code className="text-sm">{request.path}</code>
      </div>
      <div className="flex items-center gap-4 text-sm">
        <span className={cn("font-mono", getStatusColor(request.status))}>
          {request.status}
        </span>
        <span className="text-muted-foreground">{request.duration}ms</span>
        <span className="text-muted-foreground text-xs">
          {new Date(request.timestamp).toLocaleTimeString()}
        </span>
      </div>
    </div>
  )
}

// ============================================================================
// Recent Requests Component
// ============================================================================

function RecentRequests() {
  const status = useGatewayStatus()

  if (!status?.recentRequests?.length) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Recent Requests</CardTitle>
        </CardHeader>
        <CardContent className="text-center py-8">
          <ArrowRight className="h-10 w-10 text-muted-foreground mx-auto mb-2" />
          <p className="text-muted-foreground">No recent requests</p>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle className="text-base">Recent Requests</CardTitle>
        <Badge variant="outline">{status.requestCount} total</Badge>
      </CardHeader>
      <CardContent className="space-y-2">
        {status.recentRequests.map((request) => (
          <RequestItem key={request.id} request={request} />
        ))}
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Main Gateway Panel Component
// ============================================================================

export function GatewayPanel() {
  const status = useGatewayStatus()
  const { isLoading, error } = useGatewayLoading()
  const fetchStatus = useGatewayStore((s) => s.fetchStatus)

  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchStatus()
  }, [fetchStatus])

  if (isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-32" />
        <Skeleton className="h-48" />
        <Skeleton className="h-64" />
      </div>
    )
  }

  if (error) {
    return (
      <Card className="border-destructive">
        <CardContent className="p-6 text-center">
          <XCircle className="h-12 w-12 text-destructive mx-auto mb-4" />
          <p className="text-destructive font-medium">Failed to load gateway status</p>
          <p className="text-sm text-muted-foreground mt-1">{error}</p>
          <Button variant="outline" className="mt-4" onClick={() => fetchStatus()}>
            <RefreshCw className="mr-2 h-4 w-4" />
            Retry
          </Button>
        </CardContent>
      </Card>
    )
  }

  if (!status) {
    return (
      <Card>
        <CardContent className="p-12 text-center">
          <Server className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
          <p className="text-muted-foreground">Gateway not configured</p>
          <p className="text-sm text-muted-foreground mt-1">
            Configure the gateway in your zero-bot configuration
          </p>
        </CardContent>
      </Card>
    )
  }

  return (
    <div className="space-y-6">
      <StatusCard />
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <EndpointsList />
        <RecentRequests />
      </div>
    </div>
  )
}
