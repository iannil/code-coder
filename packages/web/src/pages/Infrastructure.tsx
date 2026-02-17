/**
 * Infrastructure Page
 *
 * Management page for zero-bot infrastructure:
 * - Channels (messaging platforms)
 * - Gateway (HTTP webhook server)
 * - Cron (scheduled tasks)
 * - Tunnel (public URL proxy)
 */

import {
  Server,
  MessageSquare,
  Clock,
  Globe,
} from "lucide-react"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/Tabs"
import { ChannelPanel } from "@/components/channel"
import { GatewayPanel } from "@/components/gateway"
import { CronPanel } from "@/components/cron"
import { TunnelPanel } from "@/components/tunnel"

// ============================================================================
// Main Infrastructure Component
// ============================================================================

export function Infrastructure() {
  return (
    <div className="flex-1 overflow-auto">
      <div className="container max-w-5xl mx-auto p-6 space-y-8">
        {/* Header */}
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <Server className="h-6 w-6 text-primary" />
            <h1 className="text-3xl font-bold tracking-tight">Infrastructure</h1>
          </div>
          <p className="text-muted-foreground">
            Manage zero-bot messaging channels, gateway, scheduled tasks, and tunnel connections
          </p>
        </div>

        {/* Infrastructure Tabs */}
        <Tabs defaultValue="channels" className="space-y-6">
          <TabsList className="grid grid-cols-4 w-full max-w-[600px]">
            <TabsTrigger value="channels" className="flex items-center gap-2">
              <MessageSquare className="h-4 w-4" />
              <span className="hidden sm:inline">Channels</span>
            </TabsTrigger>
            <TabsTrigger value="gateway" className="flex items-center gap-2">
              <Server className="h-4 w-4" />
              <span className="hidden sm:inline">Gateway</span>
            </TabsTrigger>
            <TabsTrigger value="cron" className="flex items-center gap-2">
              <Clock className="h-4 w-4" />
              <span className="hidden sm:inline">Cron</span>
            </TabsTrigger>
            <TabsTrigger value="tunnel" className="flex items-center gap-2">
              <Globe className="h-4 w-4" />
              <span className="hidden sm:inline">Tunnel</span>
            </TabsTrigger>
          </TabsList>

          {/* Channels Tab */}
          <TabsContent value="channels" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <MessageSquare className="h-5 w-5 text-primary" />
                  Messaging Channels
                </CardTitle>
                <CardDescription>
                  Manage connections to messaging platforms like Telegram, Discord, Slack, and more
                </CardDescription>
              </CardHeader>
              <CardContent>
                <ChannelPanel />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Gateway Tab */}
          <TabsContent value="gateway" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Server className="h-5 w-5 text-primary" />
                  HTTP Gateway
                </CardTitle>
                <CardDescription>
                  Manage the HTTP webhook server for receiving external messages
                </CardDescription>
              </CardHeader>
              <CardContent>
                <GatewayPanel />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Cron Tab */}
          <TabsContent value="cron" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Clock className="h-5 w-5 text-primary" />
                  Scheduled Tasks
                </CardTitle>
                <CardDescription>
                  Create and manage cron jobs for automated task execution
                </CardDescription>
              </CardHeader>
              <CardContent>
                <CronPanel />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Tunnel Tab */}
          <TabsContent value="tunnel" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Globe className="h-5 w-5 text-primary" />
                  Tunnel Connection
                </CardTitle>
                <CardDescription>
                  Expose your local zero-bot instance to the internet via secure tunnels
                </CardDescription>
              </CardHeader>
              <CardContent>
                <TunnelPanel />
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      </div>
    </div>
  )
}

export default Infrastructure
