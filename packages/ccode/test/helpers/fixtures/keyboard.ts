/**
 * Keyboard Test Fixtures
 *
 * Provides mock keyboard events for testing TUI components.
 */

import type { ParsedKey } from "@opentui/core"

/**
 * Create a mock ParsedKey event
 */
export function createKey(overrides: Partial<ParsedKey> = {}): ParsedKey {
  return {
    name: "a",
    ctrl: false,
    meta: false,
    shift: false,
    option: false,
    sequence: "",
    number: false,
    raw: "",
    eventType: "press",
    source: "raw",
    ...overrides,
  }
}

/**
 * Character keys
 */
export const charKeys = {
  a: createKey({ name: "a", sequence: "a" }),
  b: createKey({ name: "b", sequence: "b" }),
  z: createKey({ name: "z", sequence: "z" }),
  "0": createKey({ name: "0", sequence: "0", number: true }),
  "@": createKey({ name: "@", sequence: "@" }),
  "/": createKey({ name: "/", sequence: "/" }),
}

/**
 * Control keys
 */
export const ctrlKeys = {
  a: createKey({ name: "a", sequence: "\x01", ctrl: true }),
  c: createKey({ name: "c", sequence: "\x03", ctrl: true }),
  n: createKey({ name: "n", sequence: "\x0e", ctrl: true }),
  p: createKey({ name: "p", sequence: "\x10", ctrl: true }),
  r: createKey({ name: "r", sequence: "\x12", ctrl: true }),
  u: createKey({ name: "u", sequence: "\x15", ctrl: true }),
  w: createKey({ name: "w", sequence: "\x17", ctrl: true }),
}

/**
 * Special keys
 */
export const specialKeys = {
  enter: createKey({ name: "enter", sequence: "\r", raw: "\r" }),
  escape: createKey({ name: "escape", sequence: "\x1b", raw: "\x1b" }),
  tab: createKey({ name: "tab", sequence: "\t", raw: "\t" }),
  space: createKey({ name: "space", sequence: " ", raw: " " }),
  backspace: createKey({ name: "backspace", sequence: "\x7f", raw: "\x7f" }),
  delete: createKey({ name: "delete", sequence: "\x1b[3~", raw: "\x1b[3~" }),
}

/**
 * Arrow keys
 */
export const arrowKeys = {
  up: createKey({ name: "up", sequence: "\x1b[A", raw: "\x1b[A" }),
  down: createKey({ name: "down", sequence: "\x1b[B", raw: "\x1b[B" }),
  left: createKey({ name: "left", sequence: "\x1b[D", raw: "\x1b[D" }),
  right: createKey({ name: "right", sequence: "\x1b[C", raw: "\x1b[C" }),
}

/**
 * Function keys
 */
export const functionKeys = {
  f1: createKey({ name: "f1", sequence: "\x1bOP", raw: "\x1bOP" }),
  f5: createKey({ name: "f5", sequence: "\x1b[15~", raw: "\x1b[15~" }),
  f10: createKey({ name: "f10", sequence: "\x1b[21~", raw: "\x1b[21~" }),
}

/**
 * Key sequences for testing workflows
 */
export const keySequences = {
  /** Ctrl+C to cancel/exit */
  cancel: [ctrlKeys.c],

  /** Ctrl+N for new session */
  newSession: [ctrlKeys.n],

  /** Ctrl+P for previous session */
  prevSession: [ctrlKeys.p],

  /** Escape to close dialog */
  closeDialog: [specialKeys.escape],

  /** Enter to confirm/select */
  confirm: [specialKeys.enter],

  /** Tab for autocomplete/next */
  next: [specialKeys.tab],

  /** Arrow down for navigation */
  navigateDown: [arrowKeys.down],

  /** Arrow up for navigation */
  navigateUp: [arrowKeys.up],

  /** Leader key sequence (space + key) */
  leaderThenKey: [specialKeys.space, charKeys.a],

  /** Type text and submit */
  typeAndSubmit: [
    createKey({ name: "h", sequence: "h" }),
    createKey({ name: "e", sequence: "e" }),
    createKey({ name: "l", sequence: "l" }),
    createKey({ name: "l", sequence: "l" }),
    createKey({ name: "o", sequence: "o" }),
    createKey({ name: "enter", sequence: "\r" }),
  ],
}

/**
 * Create a key sequence from a string
 */
export function createKeySequence(text: string): ParsedKey[] {
  return text.split("").map((char) => {
    if (char === "\n") return specialKeys.enter
    if (char === "\t") return specialKeys.tab
    if (char === "\x1b") return specialKeys.escape
    return createKey({ name: char, sequence: char })
  })
}

/**
 * Create Ctrl+Key sequence
 */
export function createCtrlSequence(key: string): ParsedKey {
  const code = key.charCodeAt(0) - 96 // Convert 'a' to 1, 'b' to 2, etc.
  return createKey({
    name: key.toLowerCase(),
    sequence: String.fromCharCode(code),
    ctrl: true,
  })
}

/**
 * All commonly used keys grouped by category
 */
export const keyboardFixtures = {
  char: charKeys,
  ctrl: ctrlKeys,
  special: specialKeys,
  arrow: arrowKeys,
  function: functionKeys,
  sequences: keySequences,
}
