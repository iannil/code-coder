/**
 * Channel Panel Component
 *
 * Displays and manages messaging channels:
 * - Channel list with status
 * - Enable/disable channels
 * - Health indicators
 */

import * as React from "react"
import {
  MessageSquare,
  Power,
  RefreshCw,
  CheckCircle,
  XCircle,
  AlertTriangle,
  Send,
  Hash,
  Mail,
  Phone,
} from "lucide-react"
import { Card, CardContent } from "@/components/ui/Card"
import { Button } from "@/components/ui/Button"
import { Badge } from "@/components/ui/Badge"
import { Skeleton } from "@/components/ui/Skeleton"
import {
  useChannelStore,
  useChannels,
  useChannelLoading,
  useChannelCounts,
} from "@/stores"
import { cn } from "@/lib/utils"
import type { ChannelStatus, ChannelType, ChannelHealth } from "@/lib/types"

// ============================================================================
// Channel Icons
// ============================================================================

const CHANNEL_ICONS: Record<ChannelType, React.ReactNode> = {
  cli: <Hash className="h-4 w-4" />,
  telegram: <Send className="h-4 w-4" />,
  discord: <MessageSquare className="h-4 w-4" />,
  slack: <Hash className="h-4 w-4" />,
  matrix: <MessageSquare className="h-4 w-4" />,
  whatsapp: <Phone className="h-4 w-4" />,
  imessage: <MessageSquare className="h-4 w-4" />,
  email: <Mail className="h-4 w-4" />,
  feishu: <MessageSquare className="h-4 w-4" />,
}

const CHANNEL_LABELS: Record<ChannelType, string> = {
  cli: "CLI",
  telegram: "Telegram",
  discord: "Discord",
  slack: "Slack",
  matrix: "Matrix",
  whatsapp: "WhatsApp",
  imessage: "iMessage",
  email: "Email",
  feishu: "飞书",
}

// ============================================================================
// Health Badge Component
// ============================================================================

function HealthBadge({ health }: { health: ChannelHealth }) {
  const variants: Record<ChannelHealth, { variant: "success" | "warning" | "destructive"; icon: React.ReactNode }> = {
    healthy: { variant: "success", icon: <CheckCircle className="h-3 w-3" /> },
    degraded: { variant: "warning", icon: <AlertTriangle className="h-3 w-3" /> },
    unhealthy: { variant: "destructive", icon: <XCircle className="h-3 w-3" /> },
  }

  const config = variants[health]

  return (
    <Badge variant={config.variant} className="gap-1 capitalize">
      {config.icon}
      {health}
    </Badge>
  )
}

// ============================================================================
// Channel Card Component
// ============================================================================

interface ChannelCardProps {
  channel: ChannelStatus
  isToggling: boolean
  onToggle: () => void
}

function ChannelCard({ channel, isToggling, onToggle }: ChannelCardProps) {
  return (
    <Card className={cn(!channel.enabled && "opacity-75")}>
      <CardContent className="p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div
              className={cn(
                "flex h-10 w-10 items-center justify-center rounded-lg",
                channel.enabled ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground"
              )}
            >
              {CHANNEL_ICONS[channel.type]}
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h4 className="font-medium">{channel.name}</h4>
                <Badge variant="outline" className="text-xs">
                  {CHANNEL_LABELS[channel.type]}
                </Badge>
              </div>
              <div className="flex items-center gap-2 mt-1">
                <HealthBadge health={channel.health} />
                {channel.lastPing && (
                  <span className="text-xs text-muted-foreground">
                    Last ping: {new Date(channel.lastPing).toLocaleTimeString()}
                  </span>
                )}
              </div>
              {channel.error && (
                <p className="text-xs text-destructive mt-1">{channel.error}</p>
              )}
            </div>
          </div>

          <Button
            variant={channel.enabled ? "default" : "outline"}
            size="sm"
            onClick={onToggle}
            disabled={isToggling}
          >
            {isToggling ? (
              <RefreshCw className="h-4 w-4 animate-spin" />
            ) : (
              <>
                <Power className="mr-2 h-3 w-3" />
                {channel.enabled ? "Disable" : "Enable"}
              </>
            )}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Summary Cards Component
// ============================================================================

function SummaryCards() {
  const counts = useChannelCounts()

  return (
    <div className="grid grid-cols-3 gap-3">
      <Card>
        <CardContent className="p-4">
          <div className="text-2xl font-bold">{counts.total}</div>
          <div className="text-sm text-muted-foreground">Total Channels</div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="text-2xl font-bold text-green-600">{counts.enabled}</div>
          <div className="text-sm text-muted-foreground">Enabled</div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <div className="text-2xl font-bold text-green-600">{counts.healthy}</div>
          <div className="text-sm text-muted-foreground">Healthy</div>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Main Channel Panel Component
// ============================================================================

export function ChannelPanel() {
  const channels = useChannels()
  const { isLoading, isToggling, error } = useChannelLoading()
  const fetchChannels = useChannelStore((s) => s.fetchChannels)
  const toggleChannel = useChannelStore((s) => s.toggleChannel)

  const initialized = React.useRef(false)

  React.useEffect(() => {
    if (initialized.current) return
    initialized.current = true
    fetchChannels()
  }, [fetchChannels])

  if (isLoading) {
    return (
      <div className="space-y-4">
        <div className="grid grid-cols-3 gap-3">
          {[1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-20" />
          ))}
        </div>
        {[1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-24" />
        ))}
      </div>
    )
  }

  if (error) {
    return (
      <Card className="border-destructive">
        <CardContent className="p-6 text-center">
          <XCircle className="h-12 w-12 text-destructive mx-auto mb-4" />
          <p className="text-destructive font-medium">Failed to load channels</p>
          <p className="text-sm text-muted-foreground mt-1">{error}</p>
          <Button variant="outline" className="mt-4" onClick={() => fetchChannels()}>
            <RefreshCw className="mr-2 h-4 w-4" />
            Retry
          </Button>
        </CardContent>
      </Card>
    )
  }

  if (channels.length === 0) {
    return (
      <Card>
        <CardContent className="p-12 text-center">
          <MessageSquare className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
          <p className="text-muted-foreground">No channels configured</p>
          <p className="text-sm text-muted-foreground mt-1">
            Configure channels in your zero-bot configuration
          </p>
        </CardContent>
      </Card>
    )
  }

  const enabledChannels = channels.filter((c) => c.enabled)
  const disabledChannels = channels.filter((c) => !c.enabled)

  return (
    <div className="space-y-6">
      <SummaryCards />

      {enabledChannels.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-medium flex items-center gap-2">
            <CheckCircle className="h-4 w-4 text-green-500" />
            Enabled ({enabledChannels.length})
          </h3>
          {enabledChannels.map((channel) => (
            <ChannelCard
              key={channel.name}
              channel={channel}
              isToggling={isToggling === channel.name}
              onToggle={() => toggleChannel(channel.name)}
            />
          ))}
        </div>
      )}

      {disabledChannels.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
            <XCircle className="h-4 w-4" />
            Disabled ({disabledChannels.length})
          </h3>
          {disabledChannels.map((channel) => (
            <ChannelCard
              key={channel.name}
              channel={channel}
              isToggling={isToggling === channel.name}
              onToggle={() => toggleChannel(channel.name)}
            />
          ))}
        </div>
      )}
    </div>
  )
}
