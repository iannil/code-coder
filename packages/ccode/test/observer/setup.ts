/**
 * Observer Test Setup
 *
 * This file must be imported FIRST (before any @/observer imports)
 * to properly mock dependencies before observer modules try to use them.
 *
 * @module test/observer/setup
 */

import { mock } from "bun:test"

// Create a mock logger factory
const createLogger = (tags?: Record<string, unknown>) => ({
  debug: () => {},
  info: () => {},
  warn: () => {},
  error: () => {},
  tag: () => createLogger(tags),
  clone: () => createLogger(tags),
  time: () => ({ stop: () => {}, [Symbol.dispose]: () => {} }),
  structured: {
    configureObservability: () => {},
    initObservability: async () => {},
    createSpan: () => ({ end: () => {} }),
    point: () => {},
  },
})

// Mock the Log module before any observer modules import it
mock.module("@/util/log", () => ({
  Log: {
    Level: {
      parse: (v: string) => v,
      safeParse: (v: string) => ({ success: true, data: v }),
    },
    Default: createLogger({ service: "default" }),
    init: async () => {},
    create: createLogger,
    file: () => "",
  },
}))

// Mock the Bus module
mock.module("@/bus", () => ({
  Bus: {
    publish: async () => {},
    subscribe: () => () => {},
    unsubscribe: () => {},
  },
  BusEvent: {
    define: () => ({}),
    create: () => ({}),
    parse: () => ({}),
    extend: () => ({}),
  },
}))

// Mock bus-event separately
mock.module("@/bus/bus-event", () => ({
  BusEvent: {
    define: () => ({}),
    create: () => ({}),
    parse: () => ({}),
    extend: () => ({}),
  },
}))

// Mock observability
mock.module("@/observability", () => ({
  configureObservability: () => {},
  initObservability: async () => {},
  createSpan: () => ({ end: () => {} }),
  point: () => {},
}))

// Mock Storage module to avoid SQLite dependency in tests
mock.module("@/infrastructure/storage/storage", () => ({
  Storage: {
    NotFoundError: {
      create: () => ({}),
    },
    get: async () => null,
    set: async () => {},
    delete: async () => {},
    list: async () => [],
  },
}))

// Mock the Hands bridge for executor tests
mock.module("@/autonomous/hands/bridge", () => ({
  getBridge: () => ({
    health: async () => false,
    trigger: async () => ({ success: false, error: "Not available in tests" }),
    list: async () => [],
  }),
}))
