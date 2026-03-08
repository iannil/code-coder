/**
 * Tests for the Three Dials system
 */

// Import setup FIRST to mock dependencies
import "../setup"

import { describe, test, expect, beforeEach } from "bun:test"
import {
  Dial,
  ThreeDials,
  GEAR_PRESETS,
  createDialsFromGear,
  createCustomDials,
  parseGear,
  gearAllowsAutonomousDecisions,
  gearAllowsAutonomousActions,
  type GearPreset,
} from "@/observer/dial"

describe("Dial", () => {
  test("creates with default values", () => {
    const dial = new Dial()
    expect(dial.value).toBe(50)
    expect(dial.mode).toBe("manual")
    expect(dial.bounds).toEqual({ min: 0, max: 100 })
  })

  test("creates with custom values (positional args)", () => {
    // Dial constructor: new Dial(value, mode, min, max)
    const dial = new Dial(75, "adaptive", 20, 80)
    expect(dial.value).toBe(75)
    expect(dial.mode).toBe("adaptive")
    expect(dial.bounds).toEqual({ min: 20, max: 80 })
  })

  test("clamps value to bounds in adaptive mode", () => {
    const dial = new Dial(50, "adaptive", 20, 80)
    dial.value = 100
    expect(dial.value).toBe(80) // clamped to max
    dial.value = 0
    expect(dial.value).toBe(20) // clamped to min
  })

  test("adjusts value by delta", () => {
    const dial = new Dial(50)
    dial.adjust(10)
    expect(dial.value).toBe(60)
    dial.adjust(-20)
    expect(dial.value).toBe(40)
  })

  test("converts value to float (0.0-1.0)", () => {
    const dial = new Dial(75)
    expect(dial.asFloat()).toBe(0.75)
  })

  test("checks if active (value > 0)", () => {
    const dialActive = new Dial(50)
    expect(dialActive.isActive()).toBe(true)

    const dialInactive = new Dial(0)
    expect(dialInactive.isActive()).toBe(false)
  })

  test("checks if high (value > threshold)", () => {
    const dial = new Dial(60)
    expect(dial.isHigh(50)).toBe(true)  // 60 > 50
    expect(dial.isHigh(70)).toBe(false) // 60 > 70 = false
  })
})

describe("ThreeDials", () => {
  test("creates with default values", () => {
    const dials = new ThreeDials()
    // Default constructor: (observe=70, decide=60, act=40)
    expect(dials.values()).toEqual({
      observe: 70,
      decide: 60,
      act: 40,
    })
  })

  test("creates from gear preset", () => {
    const dialsPark = ThreeDials.fromGear("P")
    expect(dialsPark.values()).toEqual({ observe: 0, decide: 0, act: 0 })

    const dialsDrive = ThreeDials.fromGear("D")
    expect(dialsDrive.values()).toEqual({ observe: 70, decide: 60, act: 40 })

    const dialsSport = ThreeDials.fromGear("S")
    expect(dialsSport.values()).toEqual({ observe: 90, decide: 80, act: 70 })
  })

  test("creates custom dials", () => {
    const dials = ThreeDials.custom(80, 60, 40)
    expect(dials.values()).toEqual({ observe: 80, decide: 60, act: 40 })
    expect(dials.gear).toBe("M")
  })

  test("sets gear and updates dials", () => {
    const dials = new ThreeDials()
    dials.setGear("S")
    expect(dials.gear).toBe("S")
    expect(dials.values()).toEqual({ observe: 90, decide: 80, act: 70 })
  })

  test("sets individual dial and switches to manual", () => {
    const dials = ThreeDials.fromGear("D")
    dials.setDial("observe", 100)
    expect(dials.values().observe).toBe(100)
    expect(dials.gear).toBe("M") // Switched to manual mode
  })

  test("shouldObserve returns true when observe > 0", () => {
    const dialsP = ThreeDials.fromGear("P")
    expect(dialsP.shouldObserve()).toBe(false)

    const dialsD = ThreeDials.fromGear("D")
    expect(dialsD.shouldObserve()).toBe(true)
  })

  test("shouldDecideAutonomously returns true when decide > 50", () => {
    const dialsN = ThreeDials.fromGear("N")
    expect(dialsN.shouldDecideAutonomously()).toBe(false)

    const dialsD = ThreeDials.fromGear("D")
    expect(dialsD.shouldDecideAutonomously()).toBe(true)
  })

  test("shouldActImmediately returns true when act > 50", () => {
    const dialsD = ThreeDials.fromGear("D")
    expect(dialsD.shouldActImmediately()).toBe(false)

    const dialsS = ThreeDials.fromGear("S")
    expect(dialsS.shouldActImmediately()).toBe(true)
  })

  test("calculates autonomy score", () => {
    const dialsP = ThreeDials.fromGear("P")
    expect(dialsP.autonomyScore()).toBe(0) // (0+0+0)/3 = 0

    const dialsS = ThreeDials.fromGear("S")
    // Sport: { observe: 90, decide: 80, act: 70 } => (90+80+70)/3 = 80
    expect(dialsS.autonomyScore()).toBe(80)
  })
})

describe("GEAR_PRESETS", () => {
  test("has all expected gears", () => {
    expect(Object.keys(GEAR_PRESETS)).toEqual(["P", "N", "D", "S", "M"])
  })

  test("Park has all zeros", () => {
    expect(GEAR_PRESETS.P).toEqual({ observe: 0, decide: 0, act: 0 })
  })

  test("Neutral has observe but no decide/act", () => {
    expect(GEAR_PRESETS.N.observe).toBeGreaterThan(0)
    expect(GEAR_PRESETS.N.decide).toBe(0)
    expect(GEAR_PRESETS.N.act).toBe(0)
  })

  test("Sport has highest values", () => {
    const sport = GEAR_PRESETS.S
    const drive = GEAR_PRESETS.D
    expect(sport.observe).toBeGreaterThan(drive.observe)
    expect(sport.decide).toBeGreaterThan(drive.decide)
    expect(sport.act).toBeGreaterThan(drive.act)
  })
})

describe("Helper functions", () => {
  test("createDialsFromGear creates correct dials", () => {
    const dials = createDialsFromGear("D")
    expect(dials.gear).toBe("D")
    expect(dials.values()).toEqual({ observe: 70, decide: 60, act: 40 })
  })

  test("createCustomDials creates manual mode dials", () => {
    const dials = createCustomDials(80, 60, 40)
    expect(dials.gear).toBe("M")
  })

  test("parseGear handles valid and invalid input", () => {
    expect(parseGear("D")).toBe("D")
    expect(parseGear("S")).toBe("S")
    expect(parseGear("drive")).toBe("D") // Case insensitive
    expect(parseGear("PARK")).toBe("P")
    expect(parseGear("invalid")).toBe(null) // Returns null for invalid
    expect(parseGear("")).toBe(null)
  })

  test("gearAllowsAutonomousDecisions", () => {
    expect(gearAllowsAutonomousDecisions("P")).toBe(false)
    expect(gearAllowsAutonomousDecisions("N")).toBe(false)
    expect(gearAllowsAutonomousDecisions("D")).toBe(true)
    expect(gearAllowsAutonomousDecisions("S")).toBe(true)
  })

  test("gearAllowsAutonomousActions", () => {
    expect(gearAllowsAutonomousActions("P")).toBe(false)
    expect(gearAllowsAutonomousActions("N")).toBe(false)
    expect(gearAllowsAutonomousActions("D")).toBe(false)
    expect(gearAllowsAutonomousActions("S")).toBe(true)
  })
})
