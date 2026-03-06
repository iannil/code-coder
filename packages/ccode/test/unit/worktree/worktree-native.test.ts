import { describe, test, expect, beforeAll, afterAll } from "bun:test"
import { tmpdir } from "os"
import { join } from "path"
import { mkdir, rm, writeFile } from "fs/promises"
import { isNative, openGitRepo, initGitRepo, isGitRepo } from "@codecoder-ai/core"

describe("Worktree Native Operations", () => {
  let testDir: string
  let repoPath: string

  beforeAll(async () => {
    testDir = join(tmpdir(), `worktree-test-${Date.now()}`)
    repoPath = join(testDir, "repo")
    await mkdir(repoPath, { recursive: true })
  })

  afterAll(async () => {
    await rm(testDir, { recursive: true, force: true })
  })

  test("native bindings are available", () => {
    // The test will run regardless of native availability
    // but we want to track this for debugging
    console.log("Native bindings available:", isNative)
  })

  test("can init and open git repository", async () => {
    if (!initGitRepo || !openGitRepo || !isGitRepo) {
      console.log("Skipping: native git bindings not available")
      return
    }

    // Init a new repo
    const handle = initGitRepo(repoPath)
    expect(handle).toBeDefined()
    expect(handle.path()).toBe(repoPath)

    // Verify it's a git repo
    expect(isGitRepo(repoPath)).toBe(true)

    // Open the existing repo
    const opened = openGitRepo(repoPath)
    expect(opened.path()).toBe(repoPath)
  })

  test("can commit and verify branches", async () => {
    if (!openGitRepo) {
      console.log("Skipping: native git bindings not available")
      return
    }

    const handle = openGitRepo(repoPath)

    // Create a test file and commit
    await writeFile(join(repoPath, "test.txt"), "Hello world")
    const commitResult = handle.commit("Test commit", true, false)
    expect(commitResult.success).toBe(true)
    expect(commitResult.commitHash).toBeDefined()

    // List branches
    const branches = handle.branches(false)
    expect(branches.length).toBeGreaterThan(0)
  })

  test("can use ref_exists", async () => {
    if (!openGitRepo) {
      console.log("Skipping: native git bindings not available")
      return
    }

    const handle = openGitRepo(repoPath)

    // Check for existing branch
    const branches = handle.branches(false)
    if (branches.length > 0) {
      const exists = handle.refExists(`refs/heads/${branches[0]}`)
      expect(exists).toBe(true)
    }

    // Check for non-existing branch
    const notExists = handle.refExists("refs/heads/nonexistent-branch-xyz")
    expect(notExists).toBe(false)
  })

  test("worktree operations (add, list, remove)", async () => {
    if (!openGitRepo) {
      console.log("Skipping: native git bindings not available")
      return
    }

    const handle = openGitRepo(repoPath)
    const wtPath = join(testDir, "worktree-1")

    // Initially no worktrees
    const initialWorktrees = handle.listWorktrees()
    const initialCount = initialWorktrees.length

    // Add a worktree
    const wtInfo = handle.addWorktree("test-wt", wtPath, "test-branch")
    expect(wtInfo.name).toBe("test-wt")
    expect(wtInfo.branch).toBe("test-branch")
    expect(wtInfo.locked).toBe(false)

    // List worktrees - should have one more
    const afterAdd = handle.listWorktrees()
    expect(afterAdd.length).toBe(initialCount + 1)
    expect(afterAdd.some(w => w.name === "test-wt")).toBe(true)

    // Get specific worktree
    const got = handle.getWorktree("test-wt")
    expect(got).not.toBeNull()
    expect(got!.name).toBe("test-wt")

    // Remove the worktree
    const removeResult = handle.removeWorktree("test-wt", true)
    expect(removeResult.success).toBe(true)

    // Verify it's gone
    const afterRemove = handle.listWorktrees()
    expect(afterRemove.length).toBe(initialCount)
    expect(afterRemove.some(w => w.name === "test-wt")).toBe(false)
  })

  test("worktree lock and unlock", async () => {
    if (!openGitRepo) {
      console.log("Skipping: native git bindings not available")
      return
    }

    const handle = openGitRepo(repoPath)
    const wtPath = join(testDir, "worktree-lock")

    // Add a worktree
    handle.addWorktree("lock-test-wt", wtPath, "lock-test-branch")

    // Lock it
    const lockResult = handle.lockWorktree("lock-test-wt", "Testing")
    expect(lockResult.success).toBe(true)

    // Verify it's locked
    const locked = handle.getWorktree("lock-test-wt")
    expect(locked!.locked).toBe(true)

    // Unlock it
    const unlockResult = handle.unlockWorktree("lock-test-wt")
    expect(unlockResult.success).toBe(true)

    // Verify it's unlocked
    const unlocked = handle.getWorktree("lock-test-wt")
    expect(unlocked!.locked).toBe(false)

    // Cleanup
    handle.removeWorktree("lock-test-wt", true)
  })
})
