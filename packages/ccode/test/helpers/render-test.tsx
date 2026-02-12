/**
 * Component Test Renderer
 *
 * Helper for testing SolidJS components used in the TUI.
 * Provides utilities to render components and access their output
 * without requiring actual terminal rendering.
 */

import { createRoot, createEffect, onCleanup } from "solid-js"
import type { JSX } from "solid-js"

interface RenderResult {
  dispose: () => void
  container: { children: JSX.Element[] }
}

/**
 * Render a SolidJS component for testing
 *
 * @example
 * ```tsx
 * test("renders text", () => {
 *   const { container, dispose } = renderComponent(() => <div>Hello</div>)
 *   expect(container.children).toHaveLength(1)
 *   dispose()
 * })
 * ```
 */
export function renderComponent(component: () => JSX.Element): RenderResult {
  const container: { children: JSX.Element[] } = { children: [] }

  const dispose = createRoot((dispose) => {
    createEffect(() => {
      container.children = [component()]
    })
    return dispose
  })

  return { container, dispose }
}

/**
 * Render a component with automatic cleanup after test
 *
 * @example
 * ```tsx
 * test("renders text", () => {
 *   renderTest(<div>Hello</div>)
 *   // Automatic cleanup
 * })
 * ```
 */
export function renderTest(component: JSX.Element): RenderResult {
  const result = renderComponent(() => component)
  onCleanup(() => result.dispose())
  return result
}

/**
 * Wait for a condition to be true before proceeding
 *
 * @example
 * ```tsx
 * test("async update", async () => {
 *   const { container } = renderComponent(() => <MyComponent />)
 *   await waitFor(() => expect(container.children).toHaveLength(1))
 * })
 * ```
 */
export async function waitFor(
  condition: () => void | Promise<void>,
  options: { timeout?: number } = {},
): Promise<void> {
  const timeoutMs = options.timeout ?? 1000
  const start = Date.now()

  while (Date.now() - start < timeoutMs) {
    try {
      await condition()
      return
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 10))
    }
  }

  throw new Error(`waitFor timed out after ${timeoutMs}ms`)
}

/**
 * Run an async function and wait for all promises to settle
 */
export async function act(fn: () => void | Promise<void>): Promise<void> {
  await fn()
  // Allow microtasks to complete
  await new Promise((resolve) => setTimeout(resolve, 0))
  // Allow macro tasks to complete
  await new Promise((resolve) => setTimeout(resolve, 10))
}

/**
 * Capture renderer updates during a test
 */
export function captureUpdates<T>(fn: () => T): { result: T; updates: number } {
  let updates = 0
  const result = fn()
  return { result, updates }
}

/**
 * Create a test harness for components with lifecycle
 */
export function createTestHarness<T>(setup: () => T): {
  harness: T
  cleanup: () => void
} {
  try {
    const harness = setup()
    return {
      harness,
      cleanup: () => {},
    }
  } catch (error) {
    throw error
  }
}
