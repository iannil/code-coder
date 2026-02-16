/**
 * Command Palette Component
 *
 * VS Code-style command palette for quick actions:
 * - Keyboard shortcut (Cmd/Ctrl + K)
 * - Fuzzy search
 * - Navigation commands
 * - Action commands
 */

import * as React from "react"
import { useNavigate } from "@tanstack/react-router"
import {
  Home,
  FileText,
  Settings,
  BookOpen,
  MessageSquare,
  Plus,
  Moon,
  Sun,
  Monitor,
  RefreshCw,
} from "lucide-react"
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from "@/components/ui/Command"
import { useTheme } from "@/hooks/use-theme"
import { useSessionStore, useSessions } from "@/stores"

// ============================================================================
// Types
// ============================================================================

interface Command {
  id: string
  label: string
  description?: string
  icon: React.ElementType
  category: "navigation" | "session" | "theme" | "action"
  shortcut?: string
  action: () => void | Promise<void>
}

// ============================================================================
// Command Palette Context
// ============================================================================

interface CommandPaletteContextValue {
  open: boolean
  setOpen: (open: boolean) => void
  toggle: () => void
}

const CommandPaletteContext = React.createContext<CommandPaletteContextValue | null>(null)

export function useCommandPalette() {
  const context = React.useContext(CommandPaletteContext)
  if (!context) {
    throw new Error("useCommandPalette must be used within CommandPaletteProvider")
  }
  return context
}

export function CommandPaletteProvider({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = React.useState(false)

  const toggle = React.useCallback(() => setOpen((o) => !o), [])

  // Global keyboard shortcut
  React.useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Cmd/Ctrl + K
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault()
        toggle()
      }
    }

    window.addEventListener("keydown", handler)
    return () => window.removeEventListener("keydown", handler)
  }, [toggle])

  const value = React.useMemo(() => ({ open, setOpen, toggle }), [open, toggle])

  return (
    <CommandPaletteContext.Provider value={value}>
      {children}
      <CommandPaletteDialog open={open} onOpenChange={setOpen} />
    </CommandPaletteContext.Provider>
  )
}

// ============================================================================
// Command Palette Dialog
// ============================================================================

interface CommandPaletteDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

function CommandPaletteDialog({ open, onOpenChange }: CommandPaletteDialogProps) {
  const navigate = useNavigate()
  const { setTheme } = useTheme()
  const { createSession, loadSessions } = useSessionStore()
  const sessions = useSessions()

  // Build commands list
  const commands: Command[] = React.useMemo(() => {
    const cmds: Command[] = [
      // Navigation
      {
        id: "nav-home",
        label: "Go to Dashboard",
        icon: Home,
        category: "navigation",
        shortcut: "G H",
        action: () => navigate({ to: "/" }),
      },
      {
        id: "nav-documents",
        label: "Go to Documents",
        icon: BookOpen,
        category: "navigation",
        shortcut: "G D",
        action: () => navigate({ to: "/documents" }),
      },
      {
        id: "nav-files",
        label: "Go to Files",
        icon: FileText,
        category: "navigation",
        shortcut: "G F",
        action: () => navigate({ to: "/files" }),
      },
      {
        id: "nav-settings",
        label: "Go to Settings",
        icon: Settings,
        shortcut: "G S",
        category: "navigation",
        action: () => navigate({ to: "/settings" }),
      },
      // Session
      {
        id: "session-new",
        label: "New Session",
        description: "Create a new chat session",
        icon: Plus,
        category: "session",
        shortcut: "N",
        action: async () => {
          const session = await createSession({})
          navigate({ to: "/sessions/$sessionId", params: { sessionId: session.id } })
        },
      },
      {
        id: "session-refresh",
        label: "Refresh Sessions",
        description: "Reload session list",
        icon: RefreshCw,
        category: "session",
        action: () => loadSessions(),
      },
      // Theme
      {
        id: "theme-light",
        label: "Switch to Light Theme",
        icon: Sun,
        category: "theme",
        action: () => setTheme("light"),
      },
      {
        id: "theme-dark",
        label: "Switch to Dark Theme",
        icon: Moon,
        category: "theme",
        action: () => setTheme("dark"),
      },
      {
        id: "theme-system",
        label: "Use System Theme",
        icon: Monitor,
        category: "theme",
        action: () => setTheme("system"),
      },
    ]

    // Add recent sessions as navigation commands
    const recentSessions = sessions.slice(0, 5)
    for (const session of recentSessions) {
      cmds.push({
        id: `session-${session.id}`,
        label: `Open: ${session.title || "Untitled Session"}`,
        icon: MessageSquare,
        category: "session",
        action: () => navigate({ to: "/sessions/$sessionId", params: { sessionId: session.id } }),
      })
    }

    return cmds
  }, [navigate, setTheme, createSession, loadSessions, sessions])

  // Group commands by category
  const groupedCommands = React.useMemo(() => {
    const groups: Record<string, Command[]> = {}
    for (const cmd of commands) {
      if (!groups[cmd.category]) groups[cmd.category] = []
      groups[cmd.category].push(cmd)
    }
    return groups
  }, [commands])

  const categoryLabels: Record<string, string> = {
    navigation: "Navigation",
    session: "Sessions",
    theme: "Theme",
    action: "Actions",
  }

  const executeCommand = async (cmd: Command) => {
    onOpenChange(false)
    await cmd.action()
  }

  return (
    <CommandDialog open={open} onOpenChange={onOpenChange}>
      <CommandInput placeholder="Type a command or search..." />
      <CommandList>
        <CommandEmpty>No commands found</CommandEmpty>
        {Object.entries(groupedCommands).map(([category, cmds]) => (
          <CommandGroup key={category} heading={categoryLabels[category] ?? category}>
            {cmds.map((cmd) => (
              <CommandItem key={cmd.id} onSelect={() => executeCommand(cmd)} className="gap-3">
                <cmd.icon className="h-4 w-4 text-muted-foreground" />
                <div className="flex-1">
                  <div>{cmd.label}</div>
                  {cmd.description && (
                    <div className="text-xs text-muted-foreground">{cmd.description}</div>
                  )}
                </div>
                {cmd.shortcut && <CommandShortcut>{cmd.shortcut}</CommandShortcut>}
              </CommandItem>
            ))}
          </CommandGroup>
        ))}
      </CommandList>
    </CommandDialog>
  )
}
