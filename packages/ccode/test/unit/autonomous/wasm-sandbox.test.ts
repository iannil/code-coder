/**
 * WASM Sandbox Tests
 *
 * Tests for the WebAssembly-based code execution sandbox.
 */

import { describe, test, expect, beforeAll } from "bun:test"
import {
  WasmSandboxExecutor,
  createWasmSandboxExecutor,
  validateCodeForWasm,
  recommendSandboxBackend,
} from "../../../src/autonomous/execution/wasm-sandbox"

describe("WasmSandboxExecutor", () => {
  let executor: WasmSandboxExecutor

  beforeAll(async () => {
    executor = await createWasmSandboxExecutor()
  })

  test("should initialize successfully", () => {
    // QuickJS WASM should be available
    expect(executor.isAvailable()).toBe(true)
  })

  test("should execute simple JavaScript code", async () => {
    const result = await executor.execute({
      language: "javascript",
      code: `
        const x = 1 + 2;
        console.log("Result:", x);
        x;
      `,
    })

    expect(result.exitCode).toBe(0)
    expect(result.stdout).toContain("Result: 3")
    expect(result.timedOut).toBe(false)
  })

  test("should capture console.log output", async () => {
    const result = await executor.execute({
      language: "javascript",
      code: `
        console.log("Hello");
        console.log("World");
      `,
    })

    expect(result.exitCode).toBe(0)
    expect(result.stdout).toContain("Hello")
    expect(result.stdout).toContain("World")
  })

  test("should capture console.error output", async () => {
    const result = await executor.execute({
      language: "javascript",
      code: `
        console.error("Error message");
      `,
    })

    expect(result.exitCode).toBe(0)
    expect(result.stderr).toContain("Error message")
  })

  test("should handle syntax errors", async () => {
    const result = await executor.execute({
      language: "javascript",
      code: `
        const x = {
      `,
    })

    expect(result.exitCode).toBe(1)
    expect(result.stderr.length).toBeGreaterThan(0)
  })

  test("should handle runtime errors", async () => {
    const result = await executor.execute({
      language: "javascript",
      code: `
        throw new Error("Test error");
      `,
    })

    expect(result.exitCode).toBe(1)
    expect(result.stderr).toContain("Test error")
  })

  test("should timeout on infinite loops", async () => {
    const result = await executor.execute({
      language: "javascript",
      code: `
        while (true) {}
      `,
      config: {
        maxTimeMs: 100, // 100ms timeout
      },
    })

    expect(result.timedOut).toBe(true)
    expect(result.exitCode).toBe(124) // Standard timeout exit code
  })

  test("should inject global variables", async () => {
    const result = await executor.execute({
      language: "javascript",
      code: `
        console.log("Name:", userName);
        console.log("Count:", itemCount);
      `,
      globals: {
        userName: "Alice",
        itemCount: 42,
      },
    })

    expect(result.exitCode).toBe(0)
    expect(result.stdout).toContain("Name: Alice")
    expect(result.stdout).toContain("Count: 42")
  })

  test("should handle complex computations", async () => {
    const result = await executor.execute({
      language: "javascript",
      code: `
        function fibonacci(n) {
          if (n <= 1) return n;
          return fibonacci(n - 1) + fibonacci(n - 2);
        }
        const result = fibonacci(10);
        console.log("Fib(10) =", result);
        result;
      `,
    })

    expect(result.exitCode).toBe(0)
    expect(result.stdout).toContain("Fib(10) = 55")
  })

  test("should handle JSON operations", async () => {
    const result = await executor.execute({
      language: "javascript",
      code: `
        const data = { name: "Test", values: [1, 2, 3] };
        const json = JSON.stringify(data);
        const parsed = JSON.parse(json);
        console.log("Parsed:", parsed.name);
      `,
    })

    expect(result.exitCode).toBe(0)
    expect(result.stdout).toContain("Parsed: Test")
  })

  test("should reject unsupported languages", async () => {
    const result = await executor.execute({
      language: "python" as any,
      code: `print("Hello")`,
    })

    expect(result.exitCode).toBe(1)
    expect(result.stderr).toContain("Unsupported language")
  })

  test("should track execution count", async () => {
    const initialStats = executor.getStats()
    const initialCount = initialStats.executionCount

    await executor.execute({
      language: "javascript",
      code: `1 + 1`,
    })

    const newStats = executor.getStats()
    expect(newStats.executionCount).toBe(initialCount + 1)
  })
})

describe("validateCodeForWasm", () => {
  test("should accept simple code", () => {
    const result = validateCodeForWasm(`
      const x = 1 + 2;
      console.log(x);
    `)

    expect(result.valid).toBe(true)
  })

  test("should reject require() calls", () => {
    const result = validateCodeForWasm(`
      const fs = require('fs');
    `)

    expect(result.valid).toBe(false)
    expect(result.reason).toContain("require")
  })

  test("should reject ES module imports", () => {
    const result = validateCodeForWasm(`
      import fs from 'fs';
    `)

    expect(result.valid).toBe(false)
    expect(result.reason).toContain("modules")
  })

  test("should reject process access", () => {
    const result = validateCodeForWasm(`
      console.log(process.env.PATH);
    `)

    expect(result.valid).toBe(false)
    expect(result.reason).toContain("process")
  })

  test("should reject fetch calls", () => {
    const result = validateCodeForWasm(`
      fetch('https://example.com');
    `)

    expect(result.valid).toBe(false)
    expect(result.reason).toContain("fetch")
  })
})

describe("recommendSandboxBackend", () => {
  test("should recommend wasm for simple JavaScript", () => {
    const code = `
      const x = 1 + 2;
      console.log(x);
    `

    const result = recommendSandboxBackend(code, "javascript")
    expect(result).toBe("wasm")
  })

  test("should recommend docker for Python", () => {
    const code = `print("Hello")`

    const result = recommendSandboxBackend(code, "python")
    expect(result).toBe("docker")
  })

  test("should recommend docker for JS with require", () => {
    const code = `
      const fs = require('fs');
      fs.readFileSync('test.txt');
    `

    const result = recommendSandboxBackend(code, "javascript")
    expect(result).toBe("docker")
  })

  test("should recommend docker for large code", () => {
    // Generate large code (>10000 chars)
    const code = Array(1000).fill("const x = 1234567890;").join("\n")

    const result = recommendSandboxBackend(code, "javascript")
    expect(result).toBe("docker")
  })

  test("should recommend docker for code with many functions", () => {
    const code = Array(15).fill("function foo() {}").join("\n")

    const result = recommendSandboxBackend(code, "javascript")
    expect(result).toBe("docker")
  })
})
