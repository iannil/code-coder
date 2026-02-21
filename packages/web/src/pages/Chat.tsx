/**
 * Chat Page
 *
 * Universal chat interface with:
 * - Auto-routing to appropriate Agent based on intent
 * - Message history
 * - Quick agent selection
 * - Real API integration for chat and agent recommendation
 */

import * as React from "react"
import {
  Bot,
  Send,
  Loader2,
  User,
  Sparkles,
  RotateCcw,
  ChevronDown,
  Copy,
  Check,
  AlertCircle,
} from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Textarea } from "@/components/ui/Textarea"
import { ScrollArea } from "@/components/ui/ScrollArea"
import { Badge } from "@/components/ui/Badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/DropdownMenu"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/Tooltip"
import { useToast } from "@/hooks/use-toast"
import { api } from "@/lib/api"
import type { RegistryAgentMetadata, AgentRecommendation } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface ChatMessage {
  id: string
  role: "user" | "assistant"
  content: string
  timestamp: Date
  agent?: string
  thinking?: boolean
  error?: boolean
}

// ============================================================================
// Message Component
// ============================================================================

interface MessageProps {
  message: ChatMessage
}

function Message({ message }: MessageProps) {
  const [copied, setCopied] = React.useState(false)

  const handleCopy = () => {
    navigator.clipboard.writeText(message.content)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const isUser = message.role === "user"

  return (
    <div className={`flex gap-3 ${isUser ? "flex-row-reverse" : ""}`}>
      <div
        className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-lg ${
          isUser ? "bg-primary" : message.error ? "bg-destructive/10" : "bg-muted"
        }`}
      >
        {isUser ? (
          <User className="h-4 w-4 text-primary-foreground" />
        ) : message.error ? (
          <AlertCircle className="h-4 w-4 text-destructive" />
        ) : (
          <Bot className="h-4 w-4 text-foreground" />
        )}
      </div>
      <div className={`flex flex-col gap-1 max-w-[80%] ${isUser ? "items-end" : "items-start"}`}>
        <div className="flex items-center gap-2">
          {!isUser && message.agent && (
            <Badge variant="outline" className="text-xs">
              @{message.agent}
            </Badge>
          )}
          <span className="text-xs text-muted-foreground">
            {message.timestamp.toLocaleTimeString()}
          </span>
        </div>
        <div
          className={`rounded-lg px-4 py-2 ${
            isUser
              ? "bg-primary text-primary-foreground"
              : message.error
                ? "bg-destructive/10 text-destructive"
                : "bg-muted"
          }`}
        >
          {message.thinking ? (
            <div className="flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span className="text-sm">Thinking...</span>
            </div>
          ) : (
            <div className="prose prose-sm dark:prose-invert max-w-none">
              <p className="whitespace-pre-wrap m-0">{message.content}</p>
            </div>
          )}
        </div>
        {!isUser && !message.thinking && !message.error && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-6 w-6" onClick={handleCopy}>
                {copied ? (
                  <Check className="h-3 w-3" />
                ) : (
                  <Copy className="h-3 w-3" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent>Copy message</TooltipContent>
          </Tooltip>
        )}
      </div>
    </div>
  )
}

// ============================================================================
// Chat Page Component
// ============================================================================

export function Chat() {
  const { toast } = useToast()

  const [messages, setMessages] = React.useState<ChatMessage[]>([])
  const [input, setInput] = React.useState("")
  const [isLoading, setIsLoading] = React.useState(false)
  const [selectedAgent, setSelectedAgent] = React.useState<string | null>(null)
  const [conversationId, setConversationId] = React.useState<string | undefined>(undefined)

  // Agent list and recommendations
  const [agents, setAgents] = React.useState<RegistryAgentMetadata[]>([])
  const [agentRecommendation, setAgentRecommendation] = React.useState<AgentRecommendation | null>(null)
  const [isLoadingAgents, setIsLoadingAgents] = React.useState(true)

  const scrollRef = React.useRef<HTMLDivElement>(null)
  const inputRef = React.useRef<HTMLTextAreaElement>(null)

  // Load agents on mount
  React.useEffect(() => {
    const loadAgents = async () => {
      try {
        const data = await api.getRegistryAgents()
        setAgents(data)
      } catch (error) {
        console.error("Failed to load agents:", error)
      } finally {
        setIsLoadingAgents(false)
      }
    }
    loadAgents()
  }, [])

  // Auto-scroll to bottom on new messages
  React.useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [messages])

  // Debounced intent detection using API
  React.useEffect(() => {
    if (!input.trim() || selectedAgent) {
      setAgentRecommendation(null)
      return
    }

    const timer = setTimeout(async () => {
      try {
        const recommendation = await api.recommendAgent(input)
        setAgentRecommendation(recommendation)
      } catch (error) {
        // Silently fail - intent detection is optional
        console.debug("Intent detection failed:", error)
      }
    }, 300)

    return () => clearTimeout(timer)
  }, [input, selectedAgent])

  const suggestedAgent = agentRecommendation?.recommended?.name ?? null

  const handleSubmit = async () => {
    const trimmedInput = input.trim()
    if (!trimmedInput || isLoading) return

    const targetAgent = selectedAgent || suggestedAgent || "general"

    // Add user message
    const userMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: "user",
      content: trimmedInput,
      timestamp: new Date(),
    }
    setMessages((prev) => [...prev, userMessage])
    setInput("")
    setAgentRecommendation(null)

    // Add thinking placeholder
    const thinkingMessage: ChatMessage = {
      id: crypto.randomUUID(),
      role: "assistant",
      content: "",
      timestamp: new Date(),
      agent: targetAgent,
      thinking: true,
    }
    setMessages((prev) => [...prev, thinkingMessage])
    setIsLoading(true)

    try {
      // Call the chat API
      const response = await api.chat({
        message: trimmedInput,
        conversation_id: conversationId,
        agent: targetAgent,
        user_id: "web-user", // TODO: Get from auth context
        channel: "web",
      })

      // Update conversation ID for follow-up messages
      setConversationId(response.conversation_id)

      // Replace thinking message with actual response
      setMessages((prev) =>
        prev.map((m) =>
          m.id === thinkingMessage.id
            ? {
                ...m,
                content: response.message,
                thinking: false,
                agent: response.agent,
              }
            : m
        )
      )
    } catch (error) {
      console.error("Chat error:", error)

      // Replace thinking message with error message
      setMessages((prev) =>
        prev.map((m) =>
          m.id === thinkingMessage.id
            ? {
                ...m,
                content: error instanceof Error ? error.message : "Failed to get response. Please try again.",
                thinking: false,
                error: true,
              }
            : m
        )
      )

      toast({
        title: "Error",
        description: "Failed to get response. Please try again.",
        variant: "destructive",
      })
    } finally {
      setIsLoading(false)
      inputRef.current?.focus()
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSubmit()
    }
  }

  const handleReset = () => {
    setMessages([])
    setSelectedAgent(null)
    setConversationId(undefined)
    setAgentRecommendation(null)
  }

  // Get agent display info
  const getAgentDisplay = (name: string) => {
    const agent = agents.find((a) => a.name === name)
    return {
      icon: agent?.icon ?? "🤖",
      displayName: agent?.displayName ?? name,
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b bg-muted/30">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
            <Sparkles className="h-5 w-5 text-primary" />
          </div>
          <div>
            <h1 className="text-lg font-semibold">Chat</h1>
            <p className="text-sm text-muted-foreground">
              Ask anything - auto-routes to the best agent
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {messages.length > 0 && (
            <Button variant="outline" size="sm" onClick={handleReset}>
              <RotateCcw className="h-4 w-4 mr-2" />
              New Chat
            </Button>
          )}
        </div>
      </div>

      {/* Messages Area */}
      <ScrollArea className="flex-1 p-4" ref={scrollRef}>
        {messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-6 text-center">
            <div className="flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
              <Sparkles className="h-8 w-8 text-primary" />
            </div>
            <div className="space-y-2 max-w-md">
              <h2 className="text-xl font-semibold">How can I help you today?</h2>
              <p className="text-sm text-muted-foreground">
                Start typing your request. I'll automatically route it to the most appropriate agent
                based on your intent.
              </p>
            </div>
            <div className="flex flex-wrap gap-2 justify-center max-w-lg">
              {["Review my code", "Help me decide", "Write a PRD", "Analyze the market"].map(
                (suggestion) => (
                  <Button
                    key={suggestion}
                    variant="outline"
                    size="sm"
                    onClick={() => setInput(suggestion)}
                    className="text-xs"
                  >
                    {suggestion}
                  </Button>
                )
              )}
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            {messages.map((message) => (
              <Message key={message.id} message={message} />
            ))}
          </div>
        )}
      </ScrollArea>

      {/* Input Area */}
      <div className="p-4 border-t bg-background">
        <div className="flex items-end gap-2">
          {/* Agent Selector */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="icon" className="shrink-0">
                <Bot className="h-4 w-4" />
                <ChevronDown className="h-3 w-3 ml-1" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="w-56 max-h-[300px] overflow-y-auto">
              <DropdownMenuLabel>Select Agent</DropdownMenuLabel>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => setSelectedAgent(null)}
                className={!selectedAgent ? "bg-accent" : ""}
              >
                <Sparkles className="h-4 w-4 mr-2" />
                Auto-detect
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              {isLoadingAgents ? (
                <div className="px-2 py-1.5 text-sm text-muted-foreground">
                  Loading agents...
                </div>
              ) : (
                agents.slice(0, 15).map((agent) => (
                  <DropdownMenuItem
                    key={agent.name}
                    onClick={() => setSelectedAgent(agent.name)}
                    className={selectedAgent === agent.name ? "bg-accent" : ""}
                  >
                    <span className="mr-2">{agent.icon ?? "🤖"}</span>
                    @{agent.displayName ?? agent.name}
                  </DropdownMenuItem>
                ))
              )}
            </DropdownMenuContent>
          </DropdownMenu>

          {/* Input */}
          <div className="relative flex-1">
            <Textarea
              ref={inputRef}
              placeholder="Type your message..."
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={isLoading}
              className="min-h-[44px] max-h-[200px] resize-none pr-12"
              rows={1}
            />
            {suggestedAgent && input.trim() && !selectedAgent && (
              <div className="absolute left-3 bottom-full mb-1">
                <Badge variant="secondary" className="text-xs">
                  <span className="mr-1">{getAgentDisplay(suggestedAgent).icon}</span>
                  Routing to @{getAgentDisplay(suggestedAgent).displayName}
                </Badge>
              </div>
            )}
          </div>

          {/* Send Button */}
          <Button
            onClick={handleSubmit}
            disabled={!input.trim() || isLoading}
            className="shrink-0"
          >
            {isLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Send className="h-4 w-4" />
            )}
          </Button>
        </div>
        {selectedAgent && (
          <div className="mt-2 flex items-center gap-2">
            <Badge variant="outline">
              <span className="mr-1">{getAgentDisplay(selectedAgent).icon}</span>
              Using @{getAgentDisplay(selectedAgent).displayName}
            </Badge>
            <Button variant="ghost" size="sm" onClick={() => setSelectedAgent(null)}>
              Clear
            </Button>
          </div>
        )}
      </div>
    </div>
  )
}

export default Chat
