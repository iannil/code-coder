import { describe, expect, it } from "bun:test"
import { findLast } from "../src/array"

describe("findLast", () => {
  it("should find the last element matching the predicate", () => {
    const items = [1, 2, 3, 4, 5]
    const result = findLast(items, (x) => x < 4)
    expect(result).toBe(3)
  })

  it("should return undefined when no element matches", () => {
    const items = [1, 2, 3]
    const result = findLast(items, (x) => x > 10)
    expect(result).toBeUndefined()
  })

  it("should return undefined for empty array", () => {
    const items: number[] = []
    const result = findLast(items, () => true)
    expect(result).toBeUndefined()
  })

  it("should provide correct index to predicate", () => {
    const items = ["a", "b", "c"]
    const indices: number[] = []
    findLast(items, (_, index) => {
      indices.push(index)
      return false
    })
    expect(indices).toEqual([2, 1, 0])
  })

  it("should work with objects", () => {
    const items = [
      { id: 1, active: false },
      { id: 2, active: true },
      { id: 3, active: true },
    ]
    const result = findLast(items, (x) => x.active)
    expect(result).toEqual({ id: 3, active: true })
  })
})
