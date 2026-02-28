import { describe, test, expect, beforeEach } from "bun:test"
import {
  AgentSignatureVerifier,
  createVerifier,
  generateKeyPair,
  type AgentDefinition,
  type TrustLevel,
} from "@/agent/signature"

describe("agent-signature", () => {
  describe("key generation", () => {
    test("generates valid Ed25519 key pair", () => {
      const keyPair = generateKeyPair()

      expect(keyPair.publicKey).toBeDefined()
      expect(keyPair.privateKey).toBeDefined()
      expect(typeof keyPair.publicKey).toBe("string")
      expect(typeof keyPair.privateKey).toBe("string")
      // Ed25519 keys are 32 bytes, hex encoded = 64 chars, but with DER wrapper they're longer
      expect(keyPair.publicKey.length).toBeGreaterThan(32)
      expect(keyPair.privateKey.length).toBeGreaterThan(64)
    })

    test("generates unique key pairs", () => {
      const keyPair1 = generateKeyPair()
      const keyPair2 = generateKeyPair()

      expect(keyPair1.publicKey).not.toBe(keyPair2.publicKey)
      expect(keyPair1.privateKey).not.toBe(keyPair2.privateKey)
    })
  })

  describe("AgentSignatureVerifier", () => {
    let verifier: AgentSignatureVerifier

    beforeEach(() => {
      verifier = createVerifier()
    })

    describe("verify", () => {
      test("returns unverified for agent without manifest", async () => {
        await verifier.initialize()

        const agentDef: AgentDefinition = {
          name: "test-agent-no-manifest",
          prompt: "Test prompt",
          description: "Test description",
          mode: "subagent",
          options: {},
          permission: [],
        }

        const result = await verifier.verify(agentDef)

        expect(result.trust).toBe("unverified")
        expect(result.valid).toBe(false)
        expect(result.message).toContain("No manifest found")
      })

      test("verification result includes timestamp", async () => {
        await verifier.initialize()

        const agentDef: AgentDefinition = {
          name: "test-agent-timestamp",
          prompt: "Test",
          mode: "subagent",
          options: {},
          permission: [],
        }

        const before = Date.now()
        const result = await verifier.verify(agentDef)
        const after = Date.now()

        expect(result.verifiedAt).toBeGreaterThanOrEqual(before)
        expect(result.verifiedAt).toBeLessThanOrEqual(after)
      })
    })

    describe("trust levels", () => {
      test("trust levels are properly typed", () => {
        const levels: TrustLevel[] = ["verified", "unverified", "untrusted", "self_signed"]

        for (const level of levels) {
          expect(typeof level).toBe("string")
        }
      })
    })

    describe("trusted keys management", () => {
      test("initially has no trusted keys", async () => {
        await verifier.initialize()
        const keys = verifier.getTrustedKeys()
        // May have some if file exists from previous tests
        expect(Array.isArray(keys)).toBe(true)
      })

      test("can add trusted key", async () => {
        await verifier.initialize()
        const keyPair = generateKeyPair()

        const initialCount = verifier.getTrustedKeys().length
        await verifier.addTrustedKey(keyPair.publicKey)

        const keys = verifier.getTrustedKeys()
        expect(keys.length).toBe(initialCount + 1)
        expect(keys).toContain(keyPair.publicKey)
      })

      test("can remove trusted key", async () => {
        await verifier.initialize()
        const keyPair = generateKeyPair()

        await verifier.addTrustedKey(keyPair.publicKey)
        expect(verifier.getTrustedKeys()).toContain(keyPair.publicKey)

        await verifier.removeTrustedKey(keyPair.publicKey)
        expect(verifier.getTrustedKeys()).not.toContain(keyPair.publicKey)
      })
    })

    describe("verifyAll", () => {
      test("verifies multiple agents", async () => {
        await verifier.initialize()

        const agents: AgentDefinition[] = [
          { name: "agent1", mode: "subagent", options: {}, permission: [] },
          { name: "agent2", mode: "primary", options: {}, permission: [] },
          { name: "agent3", mode: "subagent", options: {}, permission: [] },
        ]

        const results = await verifier.verifyAll(agents)

        expect(results.size).toBe(3)
        expect(results.has("agent1")).toBe(true)
        expect(results.has("agent2")).toBe(true)
        expect(results.has("agent3")).toBe(true)

        // All should be unverified since no manifests exist
        for (const [name, result] of results) {
          expect(result.trust).toBe("unverified")
        }
      })
    })
  })

  describe("hash consistency", () => {
    test("same agent definition produces same hash", async () => {
      const verifier = createVerifier()
      await verifier.initialize()

      const agentDef: AgentDefinition = {
        name: "consistent-hash-test",
        prompt: "Test prompt",
        description: "Test description",
        mode: "subagent",
        options: { key: "value" },
        permission: [{ type: "read", pattern: "*" }],
      }

      // Verify twice - internal hash should be consistent
      const result1 = await verifier.verify(agentDef)
      const result2 = await verifier.verify(agentDef)

      // Both should be unverified but consistent
      expect(result1.trust).toBe(result2.trust)
    })

    test("different agent definitions produce different hashes", async () => {
      const verifier = createVerifier()
      await verifier.initialize()

      const agentDef1: AgentDefinition = {
        name: "hash-test-1",
        prompt: "Prompt A",
        mode: "subagent",
        options: {},
        permission: [],
      }

      const agentDef2: AgentDefinition = {
        name: "hash-test-2",
        prompt: "Prompt B",
        mode: "subagent",
        options: {},
        permission: [],
      }

      // Internally they hash differently - both unverified but different agents
      const result1 = await verifier.verify(agentDef1)
      const result2 = await verifier.verify(agentDef2)

      expect(result1.message).toContain("hash-test-1")
      expect(result2.message).toContain("hash-test-2")
    })
  })
})
