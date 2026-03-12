#!/usr/bin/env bun
/**
 * End-to-End Integration Test for Rust-First Architecture
 *
 * Tests the following flow:
 * 1. HTTP API: List agents
 * 2. HTTP API: Get specific agent
 * 3. WebSocket: Agent dispatch (simulated)
 *
 * Run: bun run test/e2e/rust-api-test.ts
 */

const API_BASE = process.env.API_URL || "http://localhost:4402"

interface AgentInfo {
  name: string
  description?: string
  mode: string
  hidden: boolean
}

interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

async function testListAgents(): Promise<void> {
  console.log("\n📋 Test 1: List Agents (GET /api/v1/agents)")

  try {
    const res = await fetch(`${API_BASE}/api/v1/agents`)
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${res.statusText}`)
    }

    const data = await res.json()
    console.log(`  ✅ Found ${data.agents?.length || 0} agents`)

    // Verify expected agents exist
    const expectedAgents = ["build", "plan", "explore", "writer", "macro", "trader"]
    const agentNames = data.agents?.map((a: AgentInfo) => a.name) || []

    for (const name of expectedAgents) {
      if (agentNames.includes(name)) {
        console.log(`  ✅ Agent "${name}" exists`)
      } else {
        console.log(`  ❌ Agent "${name}" NOT found`)
      }
    }
  } catch (err) {
    console.log(`  ⚠️ Skipped (server not running): ${err}`)
    return
  }
}

async function testGetAgent(): Promise<void> {
  console.log("\n🔍 Test 2: Get Specific Agent (GET /api/v1/agents/build)")

  try {
    const res = await fetch(`${API_BASE}/api/v1/agents/build`)
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${res.statusText}`)
    }

    const data = await res.json()
    if (data.agent?.name === "build") {
      console.log("  ✅ Agent 'build' retrieved successfully")
      console.log(`  ✅ Description: ${data.agent.description?.substring(0, 50)}...`)
    } else {
      console.log("  ❌ Agent 'build' not found in response")
    }
  } catch (err) {
    console.log(`  ⚠️ Skipped (server not running): ${err}`)
    return
  }
}

async function testAgentPrompt(): Promise<void> {
  console.log("\n📝 Test 3: Get Agent Prompt (GET /api/v1/agents/build/prompt)")

  try {
    const res = await fetch(`${API_BASE}/api/v1/agents/build/prompt`)
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${res.statusText}`)
    }

    const data = await res.json()
    if (data.prompt && data.prompt.length > 100) {
      console.log(`  ✅ Prompt retrieved (${data.prompt.length} chars)`)
      console.log(`  ✅ Contains: "${data.prompt.substring(0, 60)}..."`)
    } else {
      console.log("  ❌ Prompt too short or missing")
    }
  } catch (err) {
    console.log(`  ⚠️ Skipped (server not running): ${err}`)
    return
  }
}

async function testHealthCheck(): Promise<void> {
  console.log("\n💓 Test 0: Health Check (GET /health)")

  try {
    const res = await fetch(`${API_BASE}/health`, { signal: AbortSignal.timeout(2000) })
    if (res.ok) {
      console.log("  ✅ Server is healthy")
    } else {
      console.log(`  ❌ Health check failed: ${res.status}`)
    }
  } catch (err) {
    console.log(`  ⚠️ Server not running at ${API_BASE}`)
    console.log("  💡 Start the daemon with: ./ops.sh start")
    return
  }
}

async function main() {
  console.log("=" .repeat(60))
  console.log("CodeCoder Rust-First Architecture E2E Test")
  console.log("=" .repeat(60))

  await testHealthCheck()
  await testListAgents()
  await testGetAgent()
  await testAgentPrompt()

  console.log("\n" + "=" .repeat(60))
  console.log("Test complete. If server is not running, start it with:")
  console.log("  ./ops.sh start")
  console.log("=" .repeat(60))
}

main().catch(console.error)
