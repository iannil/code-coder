/**
 * GitHub CLI Mock for Testing
 *
 * Provides mocking utilities for GitHub CLI (gh) commands
 * Used by lifecycle tests that need to simulate PR workflows
 */

import { mock } from "bun:test"

export interface MockPR {
  number: number
  title: string
  body: string
  state: "open" | "closed" | "merged"
  url: string
  head: {
    ref: string
    sha: string
  }
  base: {
    ref: string
  }
  labels: string[]
  assignees: string[]
  reviewers: string[]
  draft: boolean
  mergeable: boolean
  files: Array<{
    filename: string
    status: "added" | "modified" | "removed"
    additions: number
    deletions: number
  }>
  comments: Array<{
    id: number
    body: string
    user: string
    createdAt: string
  }>
}

export interface MockIssue {
  number: number
  title: string
  body: string
  state: "open" | "closed"
  labels: string[]
  assignees: string[]
}

export interface GitHubMockState {
  prs: Map<number, MockPR>
  issues: Map<number, MockIssue>
  currentUser: string
  currentRepo: { owner: string; name: string }
  shouldError: boolean
  errorMessage: string
  nextPrNumber: number
}

const state: GitHubMockState = {
  prs: new Map(),
  issues: new Map(),
  currentUser: "testuser",
  currentRepo: { owner: "testowner", name: "testrepo" },
  shouldError: false,
  errorMessage: "GitHub CLI error",
  nextPrNumber: 1,
}

/**
 * Parse gh CLI arguments
 */
function parseGhArgs(args: string[]): { command: string; subcommand: string; flags: Record<string, string> } {
  const [command, subcommand, ...rest] = args.slice(1) // Skip 'gh'
  const flags: Record<string, string> = {}

  for (let i = 0; i < rest.length; i++) {
    const arg = rest[i]
    if (arg.startsWith("--")) {
      const key = arg.slice(2)
      const value = rest[i + 1]?.startsWith("--") ? "true" : rest[i + 1] ?? "true"
      flags[key] = value
      if (!rest[i + 1]?.startsWith("--")) i++
    } else if (arg.startsWith("-")) {
      const key = arg.slice(1)
      const value = rest[i + 1]?.startsWith("-") ? "true" : rest[i + 1] ?? "true"
      flags[key] = value
      if (!rest[i + 1]?.startsWith("-")) i++
    } else {
      flags["_positional"] = (flags["_positional"] ?? "") + " " + arg
    }
  }

  return { command: command ?? "", subcommand: subcommand ?? "", flags }
}

/**
 * Mock gh CLI command execution
 */
async function mockGhCommand(args: string[]): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  if (state.shouldError) {
    return { stdout: "", stderr: state.errorMessage, exitCode: 1 }
  }

  const { command, subcommand, flags } = parseGhArgs(args)

  if (command === "pr") {
    return handlePrCommand(subcommand, flags)
  }

  if (command === "issue") {
    return handleIssueCommand(subcommand, flags)
  }

  if (command === "api") {
    return handleApiCommand(flags)
  }

  if (command === "auth") {
    return handleAuthCommand(subcommand)
  }

  return { stdout: "", stderr: `Unknown command: ${command}`, exitCode: 1 }
}

function handlePrCommand(
  subcommand: string,
  flags: Record<string, string>,
): { stdout: string; stderr: string; exitCode: number } {
  switch (subcommand) {
    case "create": {
      const pr: MockPR = {
        number: state.nextPrNumber++,
        title: flags["title"] ?? "Untitled PR",
        body: flags["body"] ?? "",
        state: "open",
        url: `https://github.com/${state.currentRepo.owner}/${state.currentRepo.name}/pull/${state.nextPrNumber - 1}`,
        head: { ref: flags["head"] ?? "feature-branch", sha: "abc123" },
        base: { ref: flags["base"] ?? "main" },
        labels: flags["label"]?.split(",") ?? [],
        assignees: flags["assignee"]?.split(",") ?? [],
        reviewers: flags["reviewer"]?.split(",") ?? [],
        draft: flags["draft"] === "true",
        mergeable: true,
        files: [],
        comments: [],
      }
      state.prs.set(pr.number, pr)
      return { stdout: pr.url, stderr: "", exitCode: 0 }
    }

    case "list": {
      const prs = Array.from(state.prs.values())
        .filter((pr) => pr.state === "open")
        .map((pr) => `${pr.number}\t${pr.title}\t${pr.head.ref}`)
        .join("\n")
      return { stdout: prs || "No open pull requests", stderr: "", exitCode: 0 }
    }

    case "view": {
      const prNumber = parseInt(flags["_positional"]?.trim() ?? "0")
      const pr = state.prs.get(prNumber)
      if (!pr) {
        return { stdout: "", stderr: `PR #${prNumber} not found`, exitCode: 1 }
      }
      if (flags["json"]) {
        return { stdout: JSON.stringify(pr), stderr: "", exitCode: 0 }
      }
      return {
        stdout: `#${pr.number} ${pr.title}\n${pr.body}\n\nState: ${pr.state}\nURL: ${pr.url}`,
        stderr: "",
        exitCode: 0,
      }
    }

    case "merge": {
      const prNumber = parseInt(flags["_positional"]?.trim() ?? "0")
      const pr = state.prs.get(prNumber)
      if (!pr) {
        return { stdout: "", stderr: `PR #${prNumber} not found`, exitCode: 1 }
      }
      pr.state = "merged"
      return { stdout: `PR #${prNumber} merged`, stderr: "", exitCode: 0 }
    }

    case "close": {
      const prNumber = parseInt(flags["_positional"]?.trim() ?? "0")
      const pr = state.prs.get(prNumber)
      if (!pr) {
        return { stdout: "", stderr: `PR #${prNumber} not found`, exitCode: 1 }
      }
      pr.state = "closed"
      return { stdout: `PR #${prNumber} closed`, stderr: "", exitCode: 0 }
    }

    case "review": {
      const prNumber = parseInt(flags["_positional"]?.trim() ?? "0")
      const pr = state.prs.get(prNumber)
      if (!pr) {
        return { stdout: "", stderr: `PR #${prNumber} not found`, exitCode: 1 }
      }
      return { stdout: `Review submitted for PR #${prNumber}`, stderr: "", exitCode: 0 }
    }

    default:
      return { stdout: "", stderr: `Unknown pr subcommand: ${subcommand}`, exitCode: 1 }
  }
}

function handleIssueCommand(
  subcommand: string,
  flags: Record<string, string>,
): { stdout: string; stderr: string; exitCode: number } {
  switch (subcommand) {
    case "create": {
      const issue: MockIssue = {
        number: state.issues.size + 1,
        title: flags["title"] ?? "Untitled Issue",
        body: flags["body"] ?? "",
        state: "open",
        labels: flags["label"]?.split(",") ?? [],
        assignees: flags["assignee"]?.split(",") ?? [],
      }
      state.issues.set(issue.number, issue)
      return {
        stdout: `https://github.com/${state.currentRepo.owner}/${state.currentRepo.name}/issues/${issue.number}`,
        stderr: "",
        exitCode: 0,
      }
    }

    case "list": {
      const issues = Array.from(state.issues.values())
        .filter((issue) => issue.state === "open")
        .map((issue) => `${issue.number}\t${issue.title}`)
        .join("\n")
      return { stdout: issues || "No open issues", stderr: "", exitCode: 0 }
    }

    case "view": {
      const issueNumber = parseInt(flags["_positional"]?.trim() ?? "0")
      const issue = state.issues.get(issueNumber)
      if (!issue) {
        return { stdout: "", stderr: `Issue #${issueNumber} not found`, exitCode: 1 }
      }
      if (flags["json"]) {
        return { stdout: JSON.stringify(issue), stderr: "", exitCode: 0 }
      }
      return { stdout: `#${issue.number} ${issue.title}\n${issue.body}`, stderr: "", exitCode: 0 }
    }

    default:
      return { stdout: "", stderr: `Unknown issue subcommand: ${subcommand}`, exitCode: 1 }
  }
}

function handleApiCommand(flags: Record<string, string>): { stdout: string; stderr: string; exitCode: number } {
  const endpoint = flags["_positional"]?.trim() ?? ""

  if (endpoint.includes("/pulls/")) {
    const prNumber = parseInt(endpoint.split("/pulls/")[1])
    const pr = state.prs.get(prNumber)
    if (pr) {
      return { stdout: JSON.stringify(pr), stderr: "", exitCode: 0 }
    }
  }

  if (endpoint.includes("/comments")) {
    const prNumber = parseInt(endpoint.match(/pulls\/(\d+)/)?.[1] ?? "0")
    const pr = state.prs.get(prNumber)
    return { stdout: JSON.stringify(pr?.comments ?? []), stderr: "", exitCode: 0 }
  }

  return { stdout: "{}", stderr: "", exitCode: 0 }
}

function handleAuthCommand(subcommand: string): { stdout: string; stderr: string; exitCode: number } {
  switch (subcommand) {
    case "status":
      return { stdout: `Logged in to github.com as ${state.currentUser}`, stderr: "", exitCode: 0 }
    case "token":
      return { stdout: "gho_mocktoken123456789", stderr: "", exitCode: 0 }
    default:
      return { stdout: "", stderr: `Unknown auth subcommand: ${subcommand}`, exitCode: 1 }
  }
}

/**
 * Setup GitHub CLI mock
 */
export function setupGitHubMock() {
  // Mock the BunProc module used for running commands
  mock.module("../../src/bun/index", () => ({
    BunProc: {
      run: async (cmd: string[]) => {
        if (cmd[0] === "gh") {
          return mockGhCommand(cmd)
        }
        throw new Error(`Command not mocked: ${cmd.join(" ")}`)
      },
    },
  }))

  return createGitHubMockController()
}

/**
 * Create controller for GitHub mock
 */
export function createGitHubMockController() {
  // Local state for this controller instance
  let localPRs: Array<{
    number: number
    title: string
    body?: string
    state: string
    url: string
    headRefName: string
    baseRefName: string
    author: { login: string }
    additions?: number
    deletions?: number
    changedFiles?: number
    mergeable?: boolean
  }> = []

  let localChecks: Array<{
    name: string
    status: string
    conclusion: string | null
  }> = []

  let localComments: Array<{
    id: number
    body: string
    author: { login: string }
    path?: string
    line?: number
  }> = []

  return {
    /**
     * Set current user
     */
    setCurrentUser: (username: string) => {
      state.currentUser = username
    },

    /**
     * Set current repository
     */
    setCurrentRepo: (owner: string, name: string) => {
      state.currentRepo = { owner, name }
    },

    /**
     * Set PRs (replaces all PRs)
     */
    setPRs: (prs: typeof localPRs) => {
      localPRs = prs
      // Also sync to global state
      state.prs.clear()
      for (const pr of prs) {
        state.prs.set(pr.number, {
          number: pr.number,
          title: pr.title,
          body: pr.body ?? "",
          state: pr.state as "open" | "closed" | "merged",
          url: pr.url,
          head: { ref: pr.headRefName, sha: "abc123" },
          base: { ref: pr.baseRefName },
          labels: [],
          assignees: [],
          reviewers: [],
          draft: false,
          mergeable: pr.mergeable ?? true,
          files: [],
          comments: [],
        })
      }
    },

    /**
     * Get PRs
     */
    getPRs: () => localPRs,

    /**
     * Set checks
     */
    setChecks: (checks: typeof localChecks) => {
      localChecks = checks
    },

    /**
     * Get checks
     */
    getChecks: () => localChecks,

    /**
     * Set comments
     */
    setComments: (comments: typeof localComments) => {
      localComments = comments
    },

    /**
     * Get comments
     */
    getComments: () => localComments,

    /**
     * Add a mock PR
     */
    addPR: (pr: Partial<MockPR>): MockPR => {
      const fullPR: MockPR = {
        number: state.nextPrNumber++,
        title: pr.title ?? "Test PR",
        body: pr.body ?? "",
        state: pr.state ?? "open",
        url:
          pr.url ??
          `https://github.com/${state.currentRepo.owner}/${state.currentRepo.name}/pull/${state.nextPrNumber - 1}`,
        head: pr.head ?? { ref: "feature-branch", sha: "abc123" },
        base: pr.base ?? { ref: "main" },
        labels: pr.labels ?? [],
        assignees: pr.assignees ?? [],
        reviewers: pr.reviewers ?? [],
        draft: pr.draft ?? false,
        mergeable: pr.mergeable ?? true,
        files: pr.files ?? [],
        comments: pr.comments ?? [],
      }
      state.prs.set(fullPR.number, fullPR)
      return fullPR
    },

    /**
     * Add a mock issue
     */
    addIssue: (issue: Partial<MockIssue>): MockIssue => {
      const fullIssue: MockIssue = {
        number: state.issues.size + 1,
        title: issue.title ?? "Test Issue",
        body: issue.body ?? "",
        state: issue.state ?? "open",
        labels: issue.labels ?? [],
        assignees: issue.assignees ?? [],
      }
      state.issues.set(fullIssue.number, fullIssue)
      return fullIssue
    },

    /**
     * Get a PR by number
     */
    getPR: (number: number) => state.prs.get(number),

    /**
     * Get all PRs
     */
    getAllPRs: () => Array.from(state.prs.values()),

    /**
     * Set error mode
     */
    setError: (shouldError: boolean, message?: string) => {
      state.shouldError = shouldError
      if (message) state.errorMessage = message
    },

    /**
     * Add a comment to a PR
     */
    addComment: (prNumber: number, body: string, user?: string) => {
      const pr = state.prs.get(prNumber)
      if (pr) {
        pr.comments.push({
          id: pr.comments.length + 1,
          body,
          user: user ?? state.currentUser,
          createdAt: new Date().toISOString(),
        })
      }
    },

    /**
     * Add files to a PR
     */
    addFiles: (prNumber: number, files: MockPR["files"]) => {
      const pr = state.prs.get(prNumber)
      if (pr) {
        pr.files.push(...files)
      }
    },

    /**
     * Reset all state
     */
    reset: () => {
      state.prs.clear()
      state.issues.clear()
      state.currentUser = "testuser"
      state.currentRepo = { owner: "testowner", name: "testrepo" }
      state.shouldError = false
      state.errorMessage = "GitHub CLI error"
      state.nextPrNumber = 1
      localPRs = []
      localChecks = []
      localComments = []
    },
  }
}
