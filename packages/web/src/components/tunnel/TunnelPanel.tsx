/**
 * Tunnel Panel Component
 *
 * Displays and manages tunnel/proxy connections:
 * - Tunnel status
 * - Connect/disconnect controls
 * - Public URL display
 * - Connection quality indicators
 */

import * as React from "react"
import {
  Globe,
  Link2,
  Unlink,
  RefreshCw,
  CheckCircle,
  XCircle,
  Copy,
  ExternalLink,
  Wifi,
  Clock,
} from "lucide-react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card"
import { Button } from "@/components/ui/Button"
import { Badge } from "@/components/ui/Badge"
import { Skeleton } from "@/components/ui/Skeleton"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/Select"
import {
  useTunnelStore,
  useTunnelStatus,
  useTunnelLoading,
  useAvailableTunnelTypes,
} from "@/stores"
import { useToast } from "@/hooks/use-toast"
import { cn } from "@/lib/utils"
import type { TunnelType } from "@/lib/types"

// ============================================================================
// Tunnel Type Labels
// ============================================================================

const TUNNEL_LABELS: Record<TunnelType, { name: string; description: string }> = {
  cloudflare: {
    name: "Cloudflare Tunnel",
    description: "Free, secure tunnels via Cloudflare",
  },
  ngrok: {
    name: "ngrok",
    description: "Popular tunneling service",
  },
  tailscale: {
    name: "Tailscale Funnel",
    description: "WireGuard-based mesh VPN",
  },
  custom: {
    name: "Custom",
    description: "Your own tunnel solution",
  },
  none: {
    name: "None",
    description: "No tunnel configured",
  },
}

// ============================================================================
// Status Card Component
// ============================================================================

function StatusCard() {
  const status = useTunnelStatus()
  const { isConnecting, isDisconnecting } = useTunnelLoading()
  const availableTypes = useAvailableTunnelTypes()
  const connect = useTunnelStore((s) => s.connect)
  const disconnect = useTunnelStore((s) => s.disconnect)

  const [selectedType, setSelectedType] = React.useState<TunnelType>("cloudflare")

  const handleConnect = async () => {
    await connect(selectedType)
  }

  const formatUptime = (startedAt?: number) => {
    if (!startedAt) return ""
    const ms = Date.now() - startedAt
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
                status?.connected
                  ? "bg-green-500/10 text-green-600"
                  : "bg-muted text-muted-foreground"
              )}
            >
              <Globe className="h-7 w-7" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-xl font-semibold">Tunnel</h3>
                <Badge variant={status?.connected ? "success" : "outline"}>
                  {status?.connected ? "Connected" : "Disconnected"}
                </Badge>
                {status?.connected && status.type !== "none" && (
                  <Badge variant="outline">
                    {TUNNEL_LABELS[status.type].name}
                  </Badge>
                )}
              </div>
              {status?.connected && (
                <div className="flex items-center gap-4 mt-1 text-sm text-muted-foreground">
                  {status.latency && (
                    <span className="flex items-center gap-1">
                      <Wifi className="h-3 w-3" />
                      {status.latency}ms
                    </span>
                  )}
                  {status.startedAt && (
                    <span className="flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {formatUptime(status.startedAt)}
                    </span>
                  )}
                </div>
              )}
            </div>
          </div>

          {status?.connected ? (
            <Button
              variant="destructive"
              onClick={disconnect}
              disabled={isDisconnecting}
            >
              {isDisconnecting ? (
                <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Unlink className="mr-2 h-4 w-4" />
              )}
              Disconnect
            </Button>
          ) : (
            <div className="flex items-center gap-2">
              <Select
                value={selectedType}
                onValueChange={(v) => setSelectedType(v as TunnelType)}
              >
                <SelectTrigger className="w-[180px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {availableTypes.map((type) => (
                    <SelectItem key={type} value={type}>
                      {TUNNEL_LABELS[type].name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button onClick={handleConnect} disabled={isConnecting}>
                {isConnecting ? (
                  <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <Link2 className="mr-2 h-4 w-4" />
                )}
                Connect
              </Button>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// URL Card Component
// ============================================================================

function UrlCard() {
  const status = useTunnelStatus()
  const { toast } = useToast()

  if (!status?.connected || !status.publicUrl) return null

  const copyUrl = () => {
    navigator.clipboard.writeText(status.publicUrl!)
    toast({
      title: "URL copied",
      description: "Public URL copied to clipboard",
    })
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Public URL</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-2">
          <code className="flex-1 px-3 py-2 bg-muted rounded-lg text-sm truncate">
            {status.publicUrl}
          </code>
          <Button variant="outline" size="icon" onClick={copyUrl}>
            <Copy className="h-4 w-4" />
          </Button>
          <Button variant="outline" size="icon" asChild>
            <a href={status.publicUrl} target="_blank" rel="noopener noreferrer">
              <ExternalLink className="h-4 w-4" />
            </a>
          </Button>
        </div>
        {status.localUrl && (
          <p className="text-sm text-muted-foreground mt-2">
            Forwarding to: <code>{status.localUrl}</code>
          </p>
        )}
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Tunnel Types Card Component
// ============================================================================

function TunnelTypesCard() {
  const availableTypes = useAvailableTunnelTypes()

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Available Tunnel Types</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {availableTypes.map((type) => (
          <div
            key={type}
            className="flex items-center justify-between p-3 rounded-lg bg-muted/50"
          >
            <div>
              <p className="font-medium">{TUNNEL_LABELS[type].name}</p>
              <p className="text-sm text-muted-foreground">
                {TUNNEL_LABELS[type].description}
              </p>
            </div>
            <CheckCircle className="h-5 w-5 text-green-500" />
          </div>
        ))}
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Main Tunnel Panel Component
// ============================================================================

export function TunnelPanel() {
  const { isLoading, error } = useTunnelLoading()
  const fetchStatus = useTunnelStore((s) => s.fetchStatus)

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
        <Skeleton className="h-32" />
        <Skeleton className="h-48" />
      </div>
    )
  }

  if (error) {
    return (
      <Card className="border-destructive">
        <CardContent className="p-6 text-center">
          <XCircle className="h-12 w-12 text-destructive mx-auto mb-4" />
          <p className="text-destructive font-medium">Failed to load tunnel status</p>
          <p className="text-sm text-muted-foreground mt-1">{error}</p>
          <Button variant="outline" className="mt-4" onClick={() => fetchStatus()}>
            <RefreshCw className="mr-2 h-4 w-4" />
            Retry
          </Button>
        </CardContent>
      </Card>
    )
  }

  return (
    <div className="space-y-6">
      <StatusCard />
      <UrlCard />
      <TunnelTypesCard />
    </div>
  )
}
