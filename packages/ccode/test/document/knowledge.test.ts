import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { Knowledge } from "@/document/knowledge"
import { KnowledgeSchema } from "@/document/knowledge/schema"

describe("Knowledge Module", () => {
  describe("KnowledgeNode", () => {
    const mockDocumentID = "test_doc_001"

    test("should create unique IDs", () => {
      const id1 = Knowledge.Node.createID()
      const id2 = Knowledge.Node.createID()

      expect(id1).toBeDefined()
      expect(id2).toBeDefined()
      expect(id1).not.toBe(id2)
      expect(id1).toMatch(/^kn_/)
      expect(id2).toMatch(/^kn_/)
    })

    test("should have all knowledge node types defined", () => {
      const types: KnowledgeSchema.KnowledgeNodeType[] = [
        "principle",
        "concept",
        "argument",
        "evidence",
        "conclusion",
        "character",
        "location",
        "world_rule",
      ]

      for (const type of types) {
        expect(KnowledgeSchema.KnowledgeNodeType.options).toContain(type)
      }
    })
  })

  describe("ArgumentChain", () => {
    test("should create unique argument chain IDs", () => {
      const id1 = Knowledge.ArgumentChain.createID()
      const id2 = Knowledge.ArgumentChain.createID()

      expect(id1).toBeDefined()
      expect(id2).toBeDefined()
      expect(id1).not.toBe(id2)
      expect(id1).toMatch(/^arg_/)
      expect(id2).toMatch(/^arg_/)
    })

    test("should have defined status types", () => {
      // The status enum is defined in the schema
      const statusSchema = KnowledgeSchema.ArgumentChain.shape.status
      expect(statusSchema).toBeDefined()
    })
  })

  describe("StoryElements", () => {
    test("should create unique story arc IDs", () => {
      const id1 = Knowledge.StoryElements.createArcID()
      const id2 = Knowledge.StoryElements.createArcID()

      expect(id1).toBeDefined()
      expect(id2).toBeDefined()
      expect(id1).not.toBe(id2)
      expect(id1).toMatch(/^arc_/)
      expect(id2).toMatch(/^arc_/)
    })

    test("should create unique world framework IDs", () => {
      const id1 = Knowledge.StoryElements.createWorldID()
      const id2 = Knowledge.StoryElements.createWorldID()

      expect(id1).toBeDefined()
      expect(id2).toBeDefined()
      expect(id1).not.toBe(id2)
      expect(id1).toMatch(/^world_/)
      expect(id2).toMatch(/^world_/)
    })

    test("should have all story arc types defined", () => {
      const arcTypes: KnowledgeSchema.StoryArcType[] = ["setup", "rising", "climax", "falling", "resolution"]

      for (const type of arcTypes) {
        expect(KnowledgeSchema.StoryArcType.options).toContain(type)
      }
    })
  })

  describe("Framework", () => {
    test("should create unique framework IDs", () => {
      const id1 = Knowledge.Framework.createID()
      const id2 = Knowledge.Framework.createID()

      expect(id1).toBeDefined()
      expect(id2).toBeDefined()
      expect(id1).not.toBe(id2)
      expect(id1).toMatch(/^theme_/)
      expect(id2).toMatch(/^theme_/)
    })

    test("should analyze core idea", async () => {
      const analysis = await Knowledge.Framework.analyzeCoreIdea({
        idea: "This is a story about a hero who saves the world",
        targetWords: 100000,
        contentType: "fiction",
      })

      expect(analysis).toBeDefined()
      expect(analysis.contentType).toBe("fiction")
      expect(analysis.coreThesis).toBeDefined()
      expect(analysis.suggestedChapterCount).toBeGreaterThan(0)
    })

    test("should detect fiction content type", async () => {
      const analysis = await Knowledge.Framework.analyzeCoreIdea({
        idea: "Once upon a time, there was a magical kingdom with dragons and heroes",
        targetWords: 50000,
      })

      expect(analysis.contentType).toBe("fiction")
    })

    test("should detect nonfiction content type", async () => {
      const analysis = await Knowledge.Framework.analyzeCoreIdea({
        idea: "This paper argues that climate change is caused by human activity and requires immediate action",
        targetWords: 50000,
      })

      expect(analysis.contentType).toBe("nonfiction")
    })
  })

  describe("Knowledge Namespace", () => {
    test("should export all submodules", () => {
      expect(Knowledge.Node).toBeDefined()
      expect(Knowledge.Framework).toBeDefined()
      expect(Knowledge.ArgumentChain).toBeDefined()
      expect(Knowledge.StoryElements).toBeDefined()
    })
  })
})
