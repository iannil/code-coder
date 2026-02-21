/**
 * Mock session data for evaluation tests
 *
 * These fixtures simulate session history with tool calls for testing
 * skill extraction and crystallization.
 */

import type { BootstrapTypes } from "@/bootstrap/types"
import { generateTestId } from "./mock-candidates"

/**
 * Create a mock tool call record
 */
export function createMockToolCall(
  overrides: Partial<BootstrapTypes.ToolCallRecord> = {},
): BootstrapTypes.ToolCallRecord {
  return {
    id: generateTestId("tc"),
    tool: "bash",
    input: { command: "echo test" },
    output: "test",
    duration: 100,
    timestamp: Date.now(),
    ...overrides,
  }
}

/**
 * Create a mock session with tool calls
 */
export interface MockSession {
  sessionId: string
  problem: string
  solution: string
  toolCalls: BootstrapTypes.ToolCallRecord[]
  timestamp: number
}

export function createMockSession(overrides: Partial<MockSession> = {}): MockSession {
  return {
    sessionId: generateTestId("sess"),
    problem: "How to process data efficiently",
    solution: "Use a three-step workflow for optimal processing",
    toolCalls: [
      createMockToolCall({ tool: "read", input: { path: "data.json" } }),
      createMockToolCall({ tool: "bash", input: { command: "node process.js" } }),
      createMockToolCall({ tool: "edit", input: { path: "output.json", content: "{}" } }),
    ],
    timestamp: Date.now(),
    ...overrides,
  }
}

/**
 * Pre-defined session fixtures for specific test scenarios
 */
export const MOCK_SESSIONS = {
  /**
   * JSON formatting session - should crystallize as a workflow
   */
  jsonFormatting: createMockSession({
    sessionId: "sess_json_format_001",
    problem: "How to format JSON in a TypeScript file",
    solution: "Use prettier with JSON parser option to format the file content",
    toolCalls: [
      createMockToolCall({
        id: "tc_json_1",
        tool: "read",
        input: { path: "src/config.ts" },
        output: 'const config = {foo: "bar",baz: 123}',
      }),
      createMockToolCall({
        id: "tc_json_2",
        tool: "bash",
        input: { command: 'prettier --parser json <<< \'{"foo":"bar","baz":123}\'' },
        output: '{\n  "foo": "bar",\n  "baz": 123\n}',
      }),
      createMockToolCall({
        id: "tc_json_3",
        tool: "edit",
        input: {
          path: "src/config.ts",
          content: 'const config = {\n  "foo": "bar",\n  "baz": 123\n}',
        },
      }),
    ],
  }),

  /**
   * Test execution session - should crystallize as a workflow
   */
  testExecution: createMockSession({
    sessionId: "sess_test_exec_001",
    problem: "Run tests and check coverage for the user module",
    solution: "Execute bun test with coverage flag and report results",
    toolCalls: [
      createMockToolCall({
        id: "tc_test_1",
        tool: "bash",
        input: { command: "bun test src/user --coverage" },
        output: "Tests: 15 passed, Coverage: 85%",
        duration: 5000,
      }),
      createMockToolCall({
        id: "tc_test_2",
        tool: "read",
        input: { path: "coverage/lcov.info" },
        output: "SF:src/user/index.ts\nLF:100\nLH:85",
      }),
    ],
  }),

  /**
   * Database migration session - should crystallize as a workflow
   */
  databaseMigration: createMockSession({
    sessionId: "sess_db_migrate_001",
    problem: "Run database migration with backup",
    solution: "Create backup first, then run migration, verify success",
    toolCalls: [
      createMockToolCall({
        id: "tc_db_1",
        tool: "bash",
        input: { command: "pg_dump mydb > backup.sql" },
        output: "",
        duration: 2000,
      }),
      createMockToolCall({
        id: "tc_db_2",
        tool: "bash",
        input: { command: "psql mydb < migrations/001.sql" },
        output: "CREATE TABLE",
        duration: 500,
      }),
      createMockToolCall({
        id: "tc_db_3",
        tool: "bash",
        input: { command: "psql mydb -c 'SELECT count(*) FROM migrations'" },
        output: " count \n-------\n     1",
      }),
    ],
  }),

  /**
   * Code review session with task delegation - should crystallize as agent
   */
  codeReviewWithDelegation: createMockSession({
    sessionId: "sess_code_review_001",
    problem: "Review pull request #42 for security issues",
    solution: "Delegate to security reviewer agent, analyze findings",
    toolCalls: [
      createMockToolCall({
        id: "tc_review_1",
        tool: "task",
        input: {
          subagent: "security-reviewer",
          prompt: "Review PR #42 for security vulnerabilities",
        },
        output: "Found 2 potential issues: SQL injection in user.ts:45, XSS in render.ts:102",
        duration: 10000,
      }),
      createMockToolCall({
        id: "tc_review_2",
        tool: "read",
        input: { path: "src/user.ts" },
        output: "const query = `SELECT * FROM users WHERE id = ${userId}`",
      }),
      createMockToolCall({
        id: "tc_review_3",
        tool: "edit",
        input: {
          path: "src/user.ts",
          content: "const query = `SELECT * FROM users WHERE id = $1`",
        },
      }),
    ],
  }),

  /**
   * Simple file read - too simple to crystallize
   */
  simpleRead: createMockSession({
    sessionId: "sess_simple_001",
    problem: "Read the package.json file",
    solution: "Display the contents of package.json",
    toolCalls: [
      createMockToolCall({
        id: "tc_simple_1",
        tool: "read",
        input: { path: "package.json" },
        output: '{"name": "my-app"}',
      }),
    ],
  }),

  /**
   * Complex multi-tool session - should crystallize as workflow
   */
  complexWorkflow: createMockSession({
    sessionId: "sess_complex_001",
    problem: "Set up a new React component with tests",
    solution: "Create component file, add styles, write tests, update exports",
    toolCalls: [
      createMockToolCall({
        id: "tc_complex_1",
        tool: "read",
        input: { path: "src/components/index.ts" },
        output: "export * from './Button'",
      }),
      createMockToolCall({
        id: "tc_complex_2",
        tool: "write",
        input: {
          path: "src/components/Card/Card.tsx",
          content: "export function Card() { return <div>Card</div> }",
        },
      }),
      createMockToolCall({
        id: "tc_complex_3",
        tool: "write",
        input: {
          path: "src/components/Card/Card.module.css",
          content: ".card { padding: 16px; }",
        },
      }),
      createMockToolCall({
        id: "tc_complex_4",
        tool: "write",
        input: {
          path: "src/components/Card/Card.test.tsx",
          content: "test('renders', () => {})",
        },
      }),
      createMockToolCall({
        id: "tc_complex_5",
        tool: "edit",
        input: {
          path: "src/components/index.ts",
          content: "export * from './Button'\nexport * from './Card'",
        },
      }),
      createMockToolCall({
        id: "tc_complex_6",
        tool: "bash",
        input: { command: "bun test src/components/Card" },
        output: "Tests: 1 passed",
      }),
    ],
  }),

  /**
   * API integration session
   */
  apiIntegration: createMockSession({
    sessionId: "sess_api_001",
    problem: "Integrate with the GitHub API to fetch repository info",
    solution: "Use the gh CLI to fetch repository data and parse the response",
    toolCalls: [
      createMockToolCall({
        id: "tc_api_1",
        tool: "bash",
        input: { command: "gh api repos/owner/repo" },
        output: '{"name": "repo", "stars": 100}',
        duration: 1500,
      }),
      createMockToolCall({
        id: "tc_api_2",
        tool: "write",
        input: {
          path: "repo-info.json",
          content: '{"name": "repo", "stars": 100}',
        },
      }),
    ],
  }),
} as const

/**
 * Create session with specific number of tool calls
 */
export function createSessionWithToolCount(count: number): MockSession {
  const toolCalls: BootstrapTypes.ToolCallRecord[] = []
  const tools = ["read", "edit", "bash", "write", "grep", "glob"]

  for (let i = 0; i < count; i++) {
    toolCalls.push(
      createMockToolCall({
        id: `tc_gen_${i}`,
        tool: tools[i % tools.length],
        input: { index: i },
      }),
    )
  }

  return createMockSession({
    sessionId: generateTestId("sess_multi"),
    problem: `Multi-step task with ${count} operations`,
    solution: `Successfully completed ${count} operations`,
    toolCalls,
  })
}

/**
 * Create session from skill candidate (reverse operation for testing)
 */
export function createSessionFromCandidate(
  candidate: import("./mock-candidates").MOCK_CANDIDATES["verified"],
): MockSession {
  return createMockSession({
    sessionId: candidate.source.sessionId,
    problem: candidate.source.problem,
    solution: candidate.source.solution,
    toolCalls: candidate.source.toolCalls.map((id, i) =>
      createMockToolCall({
        id,
        tool: candidate.content.steps?.[i]?.split(":")[0].replace(/\d+\.\s*/, "") ?? "bash",
      }),
    ),
  })
}
