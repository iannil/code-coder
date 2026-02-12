/**
 * Test Context Providers
 *
 * Mock SolidJS context providers for testing TUI components
 * without requiring the full OpenTUI rendering infrastructure.
 */

import { createContext, useContext, type ParentProps, type JSX } from "solid-js"
import { createStore } from "solid-js/store"
import type { ParsedKey } from "@opentui/core"
import { Keybind } from "@/util/keybind"
import type { Route } from "@/cli/cmd/tui/context/route"
import type { KeybindsConfig } from "@/types"
import {
  createMockRenderer,
  createMockTheme,
  createMockSyncData,
  createMockDimensions,
} from "./tui-mock"

// ===== Mock Route Context =====

interface MockRouteContextValue {
  data: Route
  navigate: (route: Route) => void
}

const MockRouteCtx = createContext<MockRouteContextValue>()

export function TestRouteProvider(props: ParentProps<{ initial?: Route }>) {
  const [store, setStore] = createStore<Route>(
    props.initial ?? {
      type: "home",
    },
  )

  const value: MockRouteContextValue = {
    get data() {
      return store
    },
    navigate: (route: Route) => {
      setStore(route)
    },
  }

  return <MockRouteCtx.Provider value={value}>{props.children}</MockRouteCtx.Provider>
}

export function useTestRoute(): MockRouteContextValue {
  const value = useContext(MockRouteCtx)
  if (!value) throw new Error("useTestRoute must be used within TestRouteProvider")
  return value
}

// ===== Mock Keybind Context =====

interface MockKeybindContextValue {
  all: Record<string, Keybind.Info[]>
  leader: boolean
  parse: (evt: ParsedKey) => Keybind.Info
  match: (key: keyof KeybindsConfig, evt: ParsedKey) => boolean
  print: (key: keyof KeybindsConfig) => string
  setLeader: (active: boolean) => void
}

const MockKeybindCtx = createContext<MockKeybindContextValue>()

export function TestKeybindProvider(props: ParentProps<{ config?: KeybindsConfig }>) {
  const config = props.config ?? {
    leader: "space",
    new_session: "ctrl+n",
    previous_session: "ctrl+p",
  }

  const keybinds = Object.fromEntries(
    Object.entries(config).map(([k, v]) => [k, Keybind.parse(v as string)]),
  ) as Record<string, Keybind.Info[]>

  const [store, setStore] = createStore({
    leader: false,
  })

  const value: MockKeybindContextValue = {
    get all() {
      return keybinds
    },
    get leader() {
      return store.leader
    },
    parse: (evt: ParsedKey) => {
      // Handle Ctrl+Underscore special case
      if (evt.name === "\x1F") {
        return Keybind.fromParsedKey({ ...evt, name: "_", ctrl: true }, store.leader)
      }
      return Keybind.fromParsedKey(evt, store.leader)
    },
    match: (key: keyof KeybindsConfig, evt: ParsedKey) => {
      const keybind = keybinds[key]
      if (!keybind) return false
      const parsed: Keybind.Info = value.parse(evt)
      return keybind.some((k) => Keybind.match(k, parsed))
    },
    print: (key: keyof KeybindsConfig) => {
      const first = keybinds[key]?.at(0)
      if (!first) return ""
      const result = Keybind.toString(first)
      const leaderKey = keybinds.leader?.[0]
      return leaderKey ? result.replace("<leader>", Keybind.toString(leaderKey)) : result
    },
    setLeader: (active: boolean) => {
      setStore("leader", active)
    },
  }

  return <MockKeybindCtx.Provider value={value}>{props.children}</MockKeybindCtx.Provider>
}

export function useTestKeybind(): MockKeybindContextValue {
  const value = useContext(MockKeybindCtx)
  if (!value) throw new Error("useTestKeybind must be used within TestKeybindProvider")
  return value
}

// ===== Mock Sync Context =====

interface MockSyncContextValue {
  data: {
    config: {
      keybinds: KeybindsConfig
      theme: string
      editor: string
    }
    session: unknown[]
    cwd: string
    root: string
  }
  update: (key: string, value: unknown) => void
}

const MockSyncCtx = createContext<MockSyncContextValue>()

export function TestSyncProvider(props: ParentProps<{ initial?: Partial<MockSyncContextValue["data"]> }>) {
  const data = { ...createMockSyncData(), ...props.initial }
  const [store, setStore] = createStore(data)

  const value: MockSyncContextValue = {
    get data() {
      return store
    },
    update: (key: string, value: unknown) => {
      setStore(key as never, value as never)
    },
  }

  return <MockSyncCtx.Provider value={value}>{props.children}</MockSyncCtx.Provider>
}

export function useTestSync(): MockSyncContextValue {
  const value = useContext(MockSyncCtx)
  if (!value) throw new Error("useTestSync must be used within TestSyncProvider")
  return value
}

// ===== Mock Theme Context =====

interface MockThemeContextValue {
  theme: ReturnType<typeof createMockTheme>
  setTheme: (theme: string) => void
}

const MockThemeCtx = createContext<MockThemeContextValue>()

export function TestThemeProvider(props: ParentProps<{ theme?: string }>) {
  const darkTheme = createMockTheme()
  const themeMap: Record<string, typeof darkTheme> = {
    dark: darkTheme,
    light: { name: "light", background: { r: 241, g: 241, b: 241, a: 255 }, foreground: { r: 18, g: 18, b: 18, a: 255 }, backgroundPanel: { r: 230, g: 230, b: 230, a: 255 }, border: { r: 200, g: 200, b: 200, a: 255 }, primary: { r: 97, g: 175, b: 239, a: 255 }, secondary: { r: 207, g: 146, b: 120, a: 255 }, success: { r: 86, g: 182, b: 91, a: 255 }, warning: { r: 227, g: 184, b: 76, a: 255 }, error: { r: 214, g: 79, b: 79, a: 255 }, muted: { r: 119, g: 119, b: 119, a: 255 } },
  }

  const [store, setStore] = createStore({
    name: props.theme ?? "dark",
  })

  const value: MockThemeContextValue = {
    get theme() {
      return themeMap[store.name] ?? themeMap.dark
    },
    setTheme: (name: string) => {
      setStore("name", name)
    },
  }

  return <MockThemeCtx.Provider value={value}>{props.children}</MockThemeCtx.Provider>
}

export function useTestTheme(): MockThemeContextValue {
  const value = useContext(MockThemeCtx)
  if (!value) throw new Error("useTestTheme must be used within TestThemeProvider")
  return value
}

// ===== Mock Dialog Context =====

interface DialogStackItem {
  element: JSX.Element
  onClose?: () => void
}

interface MockDialogContextValue {
  stack: DialogStackItem[]
  size: "medium" | "large"
  push: (element: JSX.Element, onClose?: () => void) => void
  replace: (element: JSX.Element, onClose?: () => void) => void
  clear: () => void
  setSize: (size: "medium" | "large") => void
}

const MockDialogCtx = createContext<MockDialogContextValue>()

export function TestDialogProvider(props: ParentProps) {
  const [store, setStore] = createStore<{
    stack: DialogStackItem[]
    size: "medium" | "large"
  }>({
    stack: [],
    size: "medium",
  })

  const value: MockDialogContextValue = {
    get stack() {
      return store.stack
    },
    get size() {
      return store.size
    },
    push: (element: JSX.Element, onClose?: () => void) => {
      setStore("stack", [...store.stack, { element, onClose }])
    },
    replace: (element: JSX.Element, onClose?: () => void) => {
      setStore("stack", [{ element, onClose }])
    },
    clear: () => {
      for (const item of store.stack) {
        item.onClose?.()
      }
      setStore("stack", [])
    },
    setSize: (size: "medium" | "large") => {
      setStore("size", size)
    },
  }

  return <MockDialogCtx.Provider value={value}>{props.children}</MockDialogCtx.Provider>
}

export function useTestDialog(): MockDialogContextValue {
  const value = useContext(MockDialogCtx)
  if (!value) throw new Error("useTestDialog must be used within TestDialogProvider")
  return value
}

// ===== Mock Renderer Context =====

interface MockRendererContextValue {
  renderer: ReturnType<typeof createMockRenderer>
  dimensions: ReturnType<typeof createMockDimensions>
}

const MockRendererCtx = createContext<MockRendererContextValue>()

export function TestRendererProvider(props: ParentProps<{ width?: number; height?: number }>) {
  const renderer = createMockRenderer()
  const dimensions = createMockDimensions(props.width ?? 120, props.height ?? 30)

  const value: MockRendererContextValue = { renderer, dimensions }

  return <MockRendererCtx.Provider value={value}>{props.children}</MockRendererCtx.Provider>
}

export function useTestRenderer(): MockRendererContextValue {
  const value = useContext(MockRendererCtx)
  if (!value) throw new Error("useTestRenderer must be used within TestRendererProvider")
  return value
}

// ===== Combined Provider =====

interface TestProvidersProps {
  children: JSX.Element
  route?: Route
  keybinds?: KeybindsConfig
  theme?: string
  syncData?: Partial<MockSyncContextValue["data"]>
  rendererWidth?: number
  rendererHeight?: number
}

/**
 * Convenience provider that includes all mock contexts
 */
export function TestProviders(props: TestProvidersProps) {
  return (
    <TestRendererProvider width={props.rendererWidth} height={props.rendererHeight}>
      <TestSyncProvider initial={props.syncData}>
        <TestThemeProvider theme={props.theme}>
          <TestKeybindProvider config={props.keybinds}>
            <TestRouteProvider initial={props.route}>
              <TestDialogProvider>{props.children}</TestDialogProvider>
            </TestRouteProvider>
          </TestKeybindProvider>
        </TestThemeProvider>
      </TestSyncProvider>
    </TestRendererProvider>
  )
}
