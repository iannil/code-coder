import { describe, expect, test, beforeEach } from "bun:test"
import path from "path"
import { Session } from "../../src/session"
import { Bus } from "../../src/bus"
import { Log } from "../../src/util/log"
import { Instance } from "../../src/project/instance"
import { tmpdir } from "../fixture/fixture"

const projectRoot = path.join(__dirname, "../..")
Log.init({ print: false })

describe("session.started event", () => {
  beforeEach(async () => {
    // Clear any pending events by waiting for event loop to drain
    await new Promise((resolve) => setImmediate(resolve))
  })

  test("should emit session.started event when session is created", async () => {
    await using tmp = await tmpdir({ git: true })

    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        let eventReceived = false
        let receivedInfo: Session.Info | undefined
        let receivedSessionId: string | undefined

        const unsub = Bus.subscribe(Session.Event.Created, (event) => {
          // Only count events from this test run
          const info = event.properties.info as Session.Info
          receivedInfo = info
          receivedSessionId = info.id
          eventReceived = true
        })

        const session = await Session.create({})

        await new Promise((resolve) => setTimeout(resolve, 100))

        unsub()

        expect(eventReceived).toBe(true)
        expect(receivedInfo).toBeDefined()
        expect(receivedSessionId).toBe(session.id)
        expect(receivedInfo?.projectID).toBe(session.projectID)
        expect(receivedInfo?.directory).toBe(session.directory)
        expect(receivedInfo?.title).toBe(session.title)

        await Session.remove(session.id)
      },
    })
  })

  test("session.started event should be emitted before session.updated", async () => {
    await using tmp = await tmpdir({ git: true })

    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const events: { type: string; sessionId: string }[] = []

        const unsubStarted = Bus.subscribe(Session.Event.Created, (event) => {
          const info = event.properties.info as Session.Info
          events.push({ type: "started", sessionId: info.id })
        })

        const unsubUpdated = Bus.subscribe(Session.Event.Updated, (event) => {
          const info = event.properties.info as Session.Info
          events.push({ type: "updated", sessionId: info.id })
        })

        const session = await Session.create({})

        // Also trigger an update to ensure Updated event is emitted
        await Session.update(session.id, (s) => { s.title = "Updated Title" })

        await new Promise((resolve) => setTimeout(resolve, 100))

        unsubStarted()
        unsubUpdated()

        // Filter events to only those from this session
        const sessionEvents = events.filter((e) => e.sessionId === session.id)
        const startedEvent = sessionEvents.find((e) => e.type === "started")
        const updatedEvent = sessionEvents.find((e) => e.type === "updated")

        expect(startedEvent).toBeDefined()
        expect(updatedEvent).toBeDefined()

        // Find positions in the full events array
        const startedIndex = events.findIndex((e) => e.type === "started" && e.sessionId === session.id)
        const updatedIndex = events.findIndex((e) => e.type === "updated" && e.sessionId === session.id)
        expect(startedIndex).toBeGreaterThanOrEqual(0)
        expect(updatedIndex).toBeGreaterThanOrEqual(0)
        expect(startedIndex).toBeLessThan(updatedIndex)

        await Session.remove(session.id)
      },
    })
  })
})
