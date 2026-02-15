/**
 * Header Component
 *
 * Displays the application header with:
 * - App title/logo
 * - Connection status indicator
 * - Settings button
 * - User info display
 */

import { Settings, Wifi, WifiOff, Loader2, User } from "lucide-react"
import { Button } from "../ui/Button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../ui/DropdownMenu"
import { useSSEConnectionState, useSSEConnecting } from "@/stores/sse"
import { useConfig } from "@/stores/config"
import { cn } from "@/lib/utils"

export interface HeaderProps {
  onSettingsClick?: () => void
  className?: string
}

/**
 * Connection status indicator component
 */
function ConnectionStatus() {
  const connectionState = useSSEConnectionState()
  const isConnecting = useSSEConnecting()

  const getStatusColor = () => {
    switch (connectionState) {
      case "connected":
        return "bg-green-500"
      case "connecting":
      case "reconnecting":
        return "bg-yellow-500"
      case "error":
        return "bg-red-500"
      default:
        return "bg-muted-foreground"
    }
  }

  const getStatusText = () => {
    switch (connectionState) {
      case "connected":
        return "Connected"
      case "connecting":
      case "reconnecting":
        return "Connecting..."
      case "error":
        return "Connection Error"
      default:
        return "Disconnected"
    }
  }

  return (
    <div className="flex items-center gap-2 text-sm">
      <div className="flex items-center gap-1.5">
        {isConnecting ? (
          <Loader2 className="h-4 w-4 animate-spin text-yellow-500" />
        ) : connectionState === "connected" ? (
          <Wifi className="h-4 w-4 text-green-500" />
        ) : (
          <WifiOff className={cn("h-4 w-4", connectionState === "error" ? "text-red-500" : "text-muted-foreground")} />
        )}
        <span className="text-muted-foreground">{getStatusText()}</span>
      </div>
      <div className={cn("h-2 w-2 rounded-full", getStatusColor())} />
    </div>
  )
}

/**
 * User info display component
 */
function UserInfo() {
  const config = useConfig()

  // Extract user info from config if available
  const userName = config?.userName as string | undefined
  const userEmail = config?.userEmail as string | undefined

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="gap-2">
          <User className="h-4 w-4" />
          <span className="hidden sm:inline">{userName || "User"}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>My Account</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <div className="px-2 py-1.5 text-sm">
          <div className="font-medium">{userName || "Anonymous User"}</div>
          {userEmail && <div className="text-muted-foreground">{userEmail}</div>}
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem>Profile</DropdownMenuItem>
        <DropdownMenuItem>Settings</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem className="text-destructive">Sign out</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

/**
 * Main header component
 */
export function Header({ onSettingsClick, className }: HeaderProps) {
  const config = useConfig()
  const appName = (config?.appName as string | undefined) ?? "CodeCoder"

  return (
    <header
      className={cn(
        "flex h-14 items-center justify-between border-b bg-background px-4 sm:px-6",
        className
      )}
    >
      {/* Logo and App Title */}
      <div className="flex items-center gap-3">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <span className="text-sm font-bold">CC</span>
        </div>
        <h1 className="text-lg font-semibold">{appName}</h1>
      </div>

      {/* Right side actions */}
      <div className="flex items-center gap-4">
        <ConnectionStatus />
        <div className="hidden md:block">
          <UserInfo />
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={onSettingsClick}
          aria-label="Open settings"
        >
          <Settings className="h-5 w-5" />
        </Button>
      </div>
    </header>
  )
}
