/**
 * Event API Handler
 * Handles /api/events endpoint for Server-Sent Events (SSE)
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse } from "../middleware"

// ============================================================================
// SSE Event Helpers
// ============================================================================

function formatSSEEvent(event: string, data: string, id?: string): string {
  let output = ""

  if (id) {
    output += `id: ${id}\n`
  }

  output += `event: ${event}\n`
  output += `data: ${data}\n`
  output += "\n"

  return output
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/events
 * Stream events using Server-Sent Events
 */
export async function streamEvents(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { LocalEvent } = await import("../../../api")
    const source = await LocalEvent.subscribe()

    if (!source?.stream) {
      return jsonResponse(
        {
          success: false,
          error: "Event stream not available",
        },
        500,
      )
    }

    // Create a transform stream to format SSE events
    const encoder = new TextEncoder()

    const transformStream = new TransformStream<unknown, Uint8Array>({
      transform(chunk, controller) {
        try {
          const data = typeof chunk === "string" ? chunk : JSON.stringify(chunk)
          const sseEvent = formatSSEEvent("message", data)
          controller.enqueue(encoder.encode(sseEvent))
        } catch (error) {
          const errorMsg = formatSSEEvent(
            "error",
            JSON.stringify({
              message: error instanceof Error ? error.message : String(error),
            }),
          )
          controller.enqueue(encoder.encode(errorMsg))
        }
      },
    })

    // Pipe the event stream through the transform
    const readable = source.stream.pipeThrough(transformStream)

    return {
      status: 200,
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
        "X-Accel-Buffering": "no", // Disable nginx buffering
      },
      body: readable,
    }
  } catch (error) {
    return jsonResponse(
      {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      },
      500,
    )
  }
}

/**
 * GET /api/events/channels
 * List available event channels
 */
export async function listEventChannels(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    return jsonResponse({
      success: true,
      data: {
        channels: ["message", "status", "error", "permission", "progress"],
      },
    })
  } catch (error) {
    return jsonResponse(
      {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      },
      500,
    )
  }
}
