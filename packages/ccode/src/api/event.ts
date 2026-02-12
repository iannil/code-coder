import { Bus } from "@/bus"
import { BusEvent } from "@/bus/bus-event"
import type { Event } from "@/types"

export type LocalEventSource = {
  stream: ReadableStream<Event>
}

export namespace LocalEvent {
  export function subscribe(): LocalEventSource {
    let controller: ReadableStreamDefaultController<Event> | undefined
    const stream = new ReadableStream<Event>({
      start(c) {
        controller = c
      },
      cancel() {
        unsub()
      },
    })

    const unsub = Bus.subscribeAll(async (event) => {
      controller?.enqueue(event as Event)
    })

    return {
      stream,
    }
  }
}
