/**
 * EmptyState Component
 *
 * A reusable empty state component for displaying:
 * - "No data" scenarios
 * - "No results" from searches
 * - "Getting started" states
 * - Error states
 */

import * as React from "react"
import {
  FileQuestion,
  SearchX,
  Inbox,
  AlertCircle,
  Loader2,
  type LucideIcon,
} from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/Button"

// ============================================================================
// Types
// ============================================================================

export type EmptyStateVariant =
  | "no-data"
  | "no-results"
  | "no-messages"
  | "no-files"
  | "error"
  | "loading"
  | "custom"

export interface EmptyStateProps {
  /** Icon to display (LucideIcon or custom element) */
  icon?: LucideIcon | React.ReactNode
  /** Title of the empty state */
  title: string
  /** Description providing more context */
  description?: string
  /** Optional action button */
  action?: {
    label: string
    onClick: () => void
    variant?: "default" | "outline" | "ghost" | "secondary"
  }
  /** Predefined variant for quick setup */
  variant?: EmptyStateVariant
  /** Size of the component */
  size?: "sm" | "md" | "lg"
  /** Optional className */
  className?: string
}

// ============================================================================
// Default Icons by Variant
// ============================================================================

const DEFAULT_ICONS: Record<EmptyStateVariant, LucideIcon> = {
  "no-data": Inbox,
  "no-results": SearchX,
  "no-messages": FileQuestion,
  "no-files": Inbox,
  "error": AlertCircle,
  "loading": Loader2,
  "custom": Inbox,
}

const DEFAULT_TITLES: Record<EmptyStateVariant, string> = {
  "no-data": "No data found",
  "no-results": "No results found",
  "no-messages": "No messages yet",
  "no-files": "No files found",
  "error": "Something went wrong",
  "loading": "Loading...",
  "custom": "",
}

// ============================================================================
// Size Classes
// ============================================================================

const SIZE_CLASSES = {
  sm: {
    icon: "h-8 w-8",
    title: "text-sm",
    description: "text-xs",
    container: "py-6",
  },
  md: {
    icon: "h-12 w-12",
    title: "text-base",
    description: "text-sm",
    container: "py-10",
  },
  lg: {
    icon: "h-16 w-16",
    title: "text-lg",
    description: "text-sm",
    container: "py-16",
  },
}

// ============================================================================
// EmptyState Component
// ============================================================================

export function EmptyState({
  icon,
  title,
  description,
  action,
  variant = "no-data",
  size = "md",
  className,
}: EmptyStateProps) {
  // Use variant defaults if no custom values provided
  const IconComponent = icon
    ? undefined
    : DEFAULT_ICONS[variant]

  const displayTitle = title || DEFAULT_TITLES[variant]

  const sizeClasses = SIZE_CLASSES[size]

  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center text-center",
        "w-full",
        sizeClasses.container,
        className
      )}
    >
      {/* Icon */}
      <div
        className={cn(
          "flex items-center justify-center rounded-full bg-muted text-muted-foreground mb-4",
          size === "sm" && "p-2",
          size === "md" && "p-3",
          size === "lg" && "p-4"
        )}
      >
        {icon ? (
          typeof icon === "function" && "type" in icon ? (
            React.createElement(icon as LucideIcon, {
              className: cn(sizeClasses.icon, variant === "loading" && "animate-spin"),
            })
          ) : (
            <span className={sizeClasses.icon}>{icon as React.ReactNode}</span>
          )
        ) : IconComponent ? (
          <IconComponent
            className={cn(sizeClasses.icon, variant === "loading" && "animate-spin")}
          />
        ) : null}
      </div>

      {/* Title */}
      <h3 className={cn("font-medium text-foreground", sizeClasses.title)}>
        {displayTitle}
      </h3>

      {/* Description */}
      {description && (
        <p className={cn("text-muted-foreground mt-1 max-w-sm", sizeClasses.description)}>
          {description}
        </p>
      )}

      {/* Action Button */}
      {action && (
        <Button
          variant={action.variant ?? "default"}
          onClick={action.onClick}
          className="mt-4"
        >
          {action.label}
        </Button>
      )}
    </div>
  )
}

// ============================================================================
// Preset Empty State Components
// ============================================================================

export interface NoDataProps extends Omit<EmptyStateProps, "icon" | "variant"> {
  /** Optional custom title */
  title: string
  /** Optional custom description */
  description?: string
}

export function NoData({ title = "No data found", description, ...props }: NoDataProps) {
  return (
    <EmptyState
      variant="no-data"
      title={title}
      description={description ?? "There is no data to display at the moment."}
      {...props}
    />
  )
}

export function NoResults({
  title = "No results found",
  description,
  ...props
}: NoDataProps) {
  return (
    <EmptyState
      variant="no-results"
      title={title}
      description={description ?? "Try adjusting your search or filter criteria."}
      {...props}
    />
  )
}

export function NoMessages({
  title = "No messages yet",
  description,
  action,
  ...props
}: NoDataProps) {
  return (
    <EmptyState
      variant="no-messages"
      title={title}
      description={description ?? "Start a conversation to see messages here."}
      action={
        action ?? {
          label: "Send a message",
          onClick: () => {},
        }
      }
      {...props}
    />
  )
}

export function NoFiles({
  title = "No files found",
  description,
  ...props
}: NoDataProps) {
  return (
    <EmptyState
      variant="no-files"
      title={title}
      description={description ?? "Upload or create files to get started."}
      {...props}
    />
  )
}

export function EmptyStateError({
  title = "Something went wrong",
  description,
  action,
  ...props
}: NoDataProps) {
  return (
    <EmptyState
      variant="error"
      title={title}
      description={description ?? "An error occurred while loading data."}
      action={
        action ?? {
          label: "Try again",
          onClick: () => window.location.reload(),
          variant: "outline",
        }
      }
      {...props}
    />
  )
}

export function LoadingState({
  title = "Loading...",
  description,
  ...props
}: NoDataProps) {
  return (
    <EmptyState
      variant="loading"
      title={title}
      description={description}
      {...props}
    />
  )
}
