/**
 * API Handler Test Helper Utilities
 *
 * Provides mock implementations and utilities for testing API handlers.
 * This includes:
 * - Request/response mock factories
 * - Handler test context
 * - Authentication mocks
 * - Middleware testing utilities
 */

import { vi, type Mock } from "bun:test"
import type {
  HttpRequest,
  HttpResponse,
  RouteParams,
  RouteHandler,
  ApiResponse,
  HttpMethod,
} from "../../src/api/server/types"

// ===== Request Factories =====

/**
 * Create a mock HttpRequest
 */
export function createMockRequest(overrides: Partial<MockRequestOptions> = {}): HttpRequest {
  const options = {
    method: "GET",
    path: "/api/test",
    query: {},
    headers: {},
    body: undefined,
    ...overrides,
  } as MockRequestOptions

  const url = new URL(`http://localhost:4400${options.path}`)
  for (const [key, value] of Object.entries(options.query ?? {})) {
    url.searchParams.set(key, value)
  }

  const headers = new Headers(options.headers)

  return {
    method: options.method,
    url,
    headers,
    body: options.body,
  }
}

export interface MockRequestOptions {
  method: HttpMethod | string
  path: string
  query?: Record<string, string>
  headers?: Record<string, string>
  body?: ReadableStream | null
}

/**
 * Create a GET request
 */
export function createGetRequest(
  path: string,
  query: Record<string, string> = {},
  headers: Record<string, string> = {},
): HttpRequest {
  return createMockRequest({
    method: "GET",
    path,
    query,
    headers,
  })
}

/**
 * Create a POST request with JSON body
 */
export function createPostRequest(
  path: string,
  body: unknown,
  headers: Record<string, string> = {},
): HttpRequest {
  const jsonBody = JSON.stringify(body)
  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(new TextEncoder().encode(jsonBody))
      controller.close()
    },
  })

  return createMockRequest({
    method: "POST",
    path,
    headers: {
      "content-type": "application/json",
      ...headers,
    },
    body: stream,
  })
}

/**
 * Create a PUT request with JSON body
 */
export function createPutRequest(
  path: string,
  body: unknown,
  headers: Record<string, string> = {},
): HttpRequest {
  const jsonBody = JSON.stringify(body)
  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(new TextEncoder().encode(jsonBody))
      controller.close()
    },
  })

  return createMockRequest({
    method: "PUT",
    path,
    headers: {
      "content-type": "application/json",
      ...headers,
    },
    body: stream,
  })
}

/**
 * Create a DELETE request
 */
export function createDeleteRequest(
  path: string,
  headers: Record<string, string> = {},
): HttpRequest {
  return createMockRequest({
    method: "DELETE",
    path,
    headers,
  })
}

/**
 * Create a PATCH request with JSON body
 */
export function createPatchRequest(
  path: string,
  body: unknown,
  headers: Record<string, string> = {},
): HttpRequest {
  const jsonBody = JSON.stringify(body)
  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(new TextEncoder().encode(jsonBody))
      controller.close()
    },
  })

  return createMockRequest({
    method: "PATCH",
    path,
    headers: {
      "content-type": "application/json",
      ...headers,
    },
    body: stream,
  })
}

// ===== Response Utilities =====

/**
 * Parse JSON from HttpResponse body
 */
export async function parseResponseBody<T = unknown>(response: HttpResponse): Promise<T> {
  if (!response.body) {
    throw new Error("Response body is empty")
  }

  if (response.body instanceof ReadableStream) {
    const text = await new Response(response.body).text()
    return JSON.parse(text) as T
  }

  if (typeof response.body === "string") {
    return JSON.parse(response.body) as T
  }

  throw new Error("Unsupported response body type")
}

/**
 * Assert response status
 */
export function assertStatus(response: HttpResponse, expected: number): void {
  if (response.status !== expected) {
    throw new Error(`Expected status ${expected}, got ${response.status}`)
  }
}

/**
 * Assert response is successful (2xx)
 */
export function assertSuccess(response: HttpResponse): void {
  if (response.status < 200 || response.status >= 300) {
    throw new Error(`Expected success status, got ${response.status}`)
  }
}

/**
 * Assert response is an error (4xx or 5xx)
 */
export function assertError(response: HttpResponse): void {
  if (response.status < 400) {
    throw new Error(`Expected error status, got ${response.status}`)
  }
}

/**
 * Assert API response data
 */
export async function assertApiResponse<T>(
  response: HttpResponse,
  expected: Partial<ApiResponse<T>>,
): Promise<ApiResponse<T>> {
  const body = await parseResponseBody<ApiResponse<T>>(response)

  if (expected.success !== undefined && body.success !== expected.success) {
    throw new Error(`Expected success=${expected.success}, got ${body.success}`)
  }

  if (expected.error !== undefined && body.error !== expected.error) {
    throw new Error(`Expected error="${expected.error}", got "${body.error}"`)
  }

  return body
}

// ===== Handler Test Context =====

/**
 * Handler test context for tracking calls and responses
 */
export interface HandlerTestContext {
  handler: RouteHandler
  calls: HandlerCall[]
  lastCall: HandlerCall | null
  onCall: Mock<(call: HandlerCall) => void>
  invoke: (req: HttpRequest, params?: RouteParams) => Promise<HttpResponse>
}

export interface HandlerCall {
  request: HttpRequest
  params: RouteParams
  response: HttpResponse
  duration: number
  error?: Error
}

/**
 * Create a handler test context
 */
export function createHandlerTestContext(handler: RouteHandler): HandlerTestContext {
  const calls: HandlerCall[] = []
  const onCall = vi.fn()

  const ctx: HandlerTestContext = {
    handler,
    calls,
    get lastCall() {
      return calls.at(-1) ?? null
    },
    onCall,
    async invoke(req: HttpRequest, params: RouteParams = {}): Promise<HttpResponse> {
      const start = performance.now()
      let response: HttpResponse
      let error: Error | undefined

      try {
        response = await handler(req, params)
      } catch (e) {
        error = e as Error
        response = {
          status: 500,
          body: JSON.stringify({ success: false, error: error.message }),
        }
      }

      const call: HandlerCall = {
        request: req,
        params,
        response,
        duration: performance.now() - start,
        error,
      }

      calls.push(call)
      onCall(call)

      if (error) throw error
      return response
    },
  }

  return ctx
}

// ===== Authentication Mocks =====

/**
 * Mock authentication state
 */
export interface MockAuthState {
  apiKey?: string
  userId?: string
  roles: string[]
  isAuthenticated: boolean
}

/**
 * Create mock authentication headers
 */
export function createAuthHeaders(apiKey: string): Record<string, string> {
  return {
    Authorization: `Bearer ${apiKey}`,
    "X-API-Key": apiKey,
  }
}

/**
 * Create mock auth state
 */
export function createMockAuthState(overrides: Partial<MockAuthState> = {}): MockAuthState {
  return {
    apiKey: "test-api-key",
    userId: "test-user-id",
    roles: ["user"],
    isAuthenticated: true,
    ...overrides,
  }
}

/**
 * Mock auth middleware
 */
export function createMockAuthMiddleware(
  authState: MockAuthState = createMockAuthState(),
): (req: HttpRequest) => MockAuthState | null {
  return (req: HttpRequest) => {
    const authHeader = req.headers.get("Authorization")
    const apiKeyHeader = req.headers.get("X-API-Key")

    if (!authHeader && !apiKeyHeader) {
      return null
    }

    // Extract key from Bearer token or X-API-Key header
    const key = authHeader?.replace("Bearer ", "") ?? apiKeyHeader

    if (key !== authState.apiKey) {
      return null
    }

    return authState
  }
}

// ===== Middleware Testing =====

/**
 * Middleware function type
 */
export type MiddlewareFunction = (
  req: HttpRequest,
  next: () => Promise<HttpResponse>,
) => Promise<HttpResponse>

/**
 * Create a middleware test context
 */
export interface MiddlewareTestContext {
  middleware: MiddlewareFunction
  nextCalled: boolean
  nextResponse: HttpResponse | null
  invoke: (req: HttpRequest, nextResponse?: HttpResponse) => Promise<HttpResponse>
}

export function createMiddlewareTestContext(
  middleware: MiddlewareFunction,
): MiddlewareTestContext {
  let nextCalled = false
  let nextResponse: HttpResponse | null = null

  return {
    middleware,
    get nextCalled() {
      return nextCalled
    },
    get nextResponse() {
      return nextResponse
    },
    async invoke(
      req: HttpRequest,
      response: HttpResponse = { status: 200, body: "OK" },
    ): Promise<HttpResponse> {
      nextCalled = false
      nextResponse = null

      return middleware(req, async () => {
        nextCalled = true
        nextResponse = response
        return response
      })
    },
  }
}

// ===== Route Testing =====

/**
 * Extract route params from path pattern
 */
export function matchRoute(
  pattern: string,
  path: string,
): RouteParams | null {
  const patternParts = pattern.split("/")
  const pathParts = path.split("/")

  if (patternParts.length !== pathParts.length) {
    return null
  }

  const params: RouteParams = {}

  for (let i = 0; i < patternParts.length; i++) {
    const patternPart = patternParts[i]
    const pathPart = pathParts[i]

    if (patternPart.startsWith(":")) {
      // Parameter
      params[patternPart.slice(1)] = pathPart
    } else if (patternPart !== pathPart) {
      // Mismatch
      return null
    }
  }

  return params
}

/**
 * Create route params for a given path
 */
export function createRouteParams(params: Record<string, string>): RouteParams {
  return params
}

// ===== SSE Testing =====

/**
 * Parse SSE events from response body
 */
export async function parseSSEEvents(response: HttpResponse): Promise<SSEEvent[]> {
  if (!response.body) {
    return []
  }

  let text: string
  if (response.body instanceof ReadableStream) {
    text = await new Response(response.body).text()
  } else if (typeof response.body === "string") {
    text = response.body
  } else {
    throw new Error("Unsupported response body type for SSE")
  }

  const events: SSEEvent[] = []
  const lines = text.split("\n")
  let currentEvent: Partial<SSEEvent> = {}

  for (const line of lines) {
    if (line.startsWith("event:")) {
      currentEvent.event = line.slice(6).trim()
    } else if (line.startsWith("data:")) {
      currentEvent.data = line.slice(5).trim()
    } else if (line.startsWith("id:")) {
      currentEvent.id = line.slice(3).trim()
    } else if (line === "") {
      if (currentEvent.data !== undefined) {
        events.push(currentEvent as SSEEvent)
      }
      currentEvent = {}
    }
  }

  return events
}

export interface SSEEvent {
  event?: string
  data: string
  id?: string
}

// ===== Test Data Factories =====

/**
 * Create mock session data for API responses
 */
export function createMockSessionData(overrides: Partial<MockSessionApiData> = {}): MockSessionApiData {
  const now = Date.now()
  return {
    id: `session-${now}`,
    title: "Test Session",
    createdAt: now,
    updatedAt: now,
    messageCount: 0,
    agent: "build",
    ...overrides,
  }
}

export interface MockSessionApiData {
  id: string
  title: string
  createdAt: number
  updatedAt: number
  messageCount: number
  agent?: string
}

/**
 * Create mock message data for API responses
 */
export function createMockMessageData(overrides: Partial<MockMessageApiData> = {}): MockMessageApiData {
  return {
    id: `msg-${Date.now()}`,
    role: "user",
    content: "Test message",
    timestamp: Date.now(),
    parts: [{ type: "text", value: "Test message" }],
    ...overrides,
  }
}

export interface MockMessageApiData {
  id: string
  role: "user" | "assistant" | "system"
  content: string
  timestamp: number
  parts: Array<{ type: string; value?: string }>
}

// ===== Error Testing =====

/**
 * Create a mock error response
 */
export function createErrorResponse(
  message: string,
  status: number = 500,
): HttpResponse {
  return {
    status,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      success: false,
      error: message,
    }),
  }
}

/**
 * Create a mock success response
 */
export function createSuccessResponse<T>(
  data: T,
  status: number = 200,
): HttpResponse {
  return {
    status,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      success: true,
      data,
    }),
  }
}

// ===== Export all =====
export {
  createMockRequest,
  createGetRequest,
  createPostRequest,
  createPutRequest,
  createDeleteRequest,
  createPatchRequest,
  parseResponseBody,
  assertStatus,
  assertSuccess,
  assertError,
  assertApiResponse,
  createHandlerTestContext,
  createAuthHeaders,
  createMockAuthState,
  createMockAuthMiddleware,
  createMiddlewareTestContext,
  matchRoute,
  createRouteParams,
  parseSSEEvents,
  createMockSessionData,
  createMockMessageData,
  createErrorResponse,
  createSuccessResponse,
}
