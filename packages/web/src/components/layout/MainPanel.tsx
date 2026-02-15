/**
 * Main Panel Component
 *
 * Displays the main content area with:
 * - Main content area
 * - Breadcrumb navigation
 * - Content children render
 */

import { ChevronRight, Home } from "lucide-react"
import { cn } from "@/lib/utils"
import type { BreadcrumbItem } from "@/lib/types"

export interface MainPanelProps {
  children: React.ReactNode
  breadcrumbs?: BreadcrumbItem[]
  className?: string
  contentClassName?: string
}

/**
 * Breadcrumb component for navigation
 */
function Breadcrumbs({ items }: { items?: BreadcrumbItem[] }) {
  if (!items || items.length === 0) return null

  return (
    <nav className="flex items-center space-x-1 text-sm text-muted-foreground">
      <Home className="h-4 w-4 shrink-0" />
      {items.map((item, index) => (
        <div key={index} className="flex items-center space-x-1">
          <ChevronRight className="h-4 w-4 shrink-0" />
          {item.href ? (
            <a
              href={item.href}
              className="hover:text-foreground transition-colors"
              onClick={item.onClick}
            >
              {item.label}
            </a>
          ) : (
            <span
              className={cn(
                index === items.length - 1
                  ? "text-foreground font-medium"
                  : "hover:text-foreground transition-colors cursor-pointer"
              )}
              onClick={item.onClick}
            >
              {item.label}
            </span>
          )}
        </div>
      ))}
    </nav>
  )
}

/**
 * Main panel component
 */
export function MainPanel({
  children,
  breadcrumbs,
  className,
  contentClassName,
}: MainPanelProps) {
  return (
    <main
      className={cn(
        "flex flex-1 flex-col overflow-hidden bg-background",
        className
      )}
    >
      {/* Breadcrumb header */}
      {(breadcrumbs && breadcrumbs.length > 0) && (
        <div className="border-b px-6 py-3">
          <Breadcrumbs items={breadcrumbs} />
        </div>
      )}

      {/* Content area */}
      <div className={cn("flex-1 overflow-auto", contentClassName)}>
        {children}
      </div>
    </main>
  )
}
