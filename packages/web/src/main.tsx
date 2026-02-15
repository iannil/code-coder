/**
 * Main Entry Point
 *
 * Initializes the React application with TanStack Router and providers
 */

import { StrictMode } from "react"
import ReactDOM from "react-dom/client"
import { RouterProvider } from "@tanstack/react-router"
import { Toaster } from "@/components/ui/Toaster"
import { router } from "./router"
import "./styles.css"

// ============================================================================
// Root Component
// ============================================================================

function Root() {
  return (
    <StrictMode>
      <RouterProvider router={router} />
      <Toaster />
    </StrictMode>
  )
}

// ============================================================================
// Mount Application
// ============================================================================

const rootElement = document.getElementById("root")

if (rootElement) {
  ReactDOM.createRoot(rootElement).render(<Root />)
}

// ============================================================================
// Hot Module Replacement (for development)
// ============================================================================

if (import.meta.hot) {
  import.meta.hot.accept()
}
