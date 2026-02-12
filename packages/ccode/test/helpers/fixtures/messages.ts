/**
 * Message Test Fixtures
 *
 * Provides mock message data for testing TUI components.
 */

export interface Message {
  id: string
  role: "user" | "assistant" | "tool"
  content: string
  timestamp: number
  status: "streaming" | "complete" | "error"
  error?: {
    type: string
    message: string
    code?: number
  }
  toolCalls?: Array<{
    id: string
    type: string
    params: Record<string, unknown>
  }>
  toolCallId?: string
  files?: Array<{
    name: string
    path: string
    size: number
  }>
  images?: Array<{
    name: string
    path: string
    size: number
  }>
}

/**
 * Message with markdown content
 */
export function createMarkdownMessage(): Message {
  return {
    id: "msg-markdown",
    role: "assistant",
    content: `# Heading 1

Here's a paragraph with **bold** and *italic* text.

## Code Block

\`\`\`typescript
function greet(name: string): string {
  return \`Hello, \${name}!\`;
}
\`\`\`

## List

- Item 1
- Item 2
  - Nested item

## Links

[Visit GitHub](https://github.com)`,
    timestamp: Date.now(),
    status: "complete",
  }
}

/**
 * Message with diff content
 */
export function createDiffMessage(): Message {
  return {
    id: "msg-diff",
    role: "assistant",
    content: `I'll update the function to add error handling:

\`\`\`diff
 function calculateTotal(items) {
-  return items.reduce((sum, item) => sum + item.price, 0);
+  if (!Array.isArray(items)) {
+    throw new Error('Items must be an array');
+  }
+  return items.reduce((sum, item) => sum + (item?.price ?? 0), 0);
 }`,
    timestamp: Date.now(),
    status: "complete",
  }
}

/**
 * Message with multiple code blocks
 */
export function createMultiCodeBlockMessage(): Message {
  return {
    id: "msg-multi-code",
    role: "assistant",
    content: `Here are examples in multiple languages:

**TypeScript:**
\`\`\`typescript
interface User {
  id: number;
  name: string;
}
\`\`\`

**Python:**
\`\`\`python
@dataclass
class User:
    id: int
    name: str
\`\`\`

**Go:**
\`\`\`go
type User struct {
    ID   int
    Name string
}
\`\`\``,
    timestamp: Date.now(),
    status: "complete",
  }
}

/**
 * Streaming message (in progress)
 */
export function createStreamingMessage(): Message {
  return {
    id: "msg-streaming",
    role: "assistant",
    content: "Here's the answer to your question",
    timestamp: Date.now(),
    status: "streaming",
  }
}

/**
 * Error message
 */
export function createErrorMessage(): Message {
  return {
    id: "msg-error",
    role: "assistant",
    content: "I encountered an error while processing your request.",
    timestamp: Date.now(),
    status: "error",
    error: {
      type: "APIError",
      message: "Rate limit exceeded",
      code: 429,
    },
  }
}

/**
 * Message with tool calls
 */
export function createToolCallMessage(): Message {
  return {
    id: "msg-tool-call",
    role: "assistant",
    content: "I'll read the file and check the configuration.",
    timestamp: Date.now(),
    status: "complete",
    toolCalls: [
      {
        id: "call-read-1",
        type: "read",
        params: { filePath: "src/config.ts" },
      },
      {
        id: "call-list-1",
        type: "list",
        params: { filePath: "src" },
      },
    ],
  }
}

/**
 * Tool result message
 */
export function createToolResultMessage(): Message {
  return {
    id: "msg-tool-result",
    role: "tool",
    content: `export const config = {
  apiUrl: "https://api.example.com",
  timeout: 5000,
};`,
    timestamp: Date.now(),
    status: "complete",
    toolCallId: "call-read-1",
  }
}

/**
 * Long message (for testing scrolling)
 */
export function createLongMessage(): Message {
  const lines = []
  for (let i = 1; i <= 50; i++) {
    lines.push(`Line ${i}: This is a long message to test scrolling functionality.`)
  }
  return {
    id: "msg-long",
    role: "assistant",
    content: lines.join("\n"),
    timestamp: Date.now(),
    status: "complete",
  }
}

/**
 * Message with mentions (@file, @command)
 */
export function createMessageWithMentions(): Message {
  return {
    id: "msg-mentions",
    role: "user",
    content: "Please review @src/components/Button.tsx and use the /refactor command",
    timestamp: Date.now(),
    status: "complete",
  }
}

/**
 * Message with file attachment
 */
export function createMessageWithFile(): Message {
  return {
    id: "msg-file",
    role: "user",
    content: "@docs/spec.pdf Here's the specification for the feature",
    timestamp: Date.now(),
    status: "complete",
    files: [{ name: "spec.pdf", path: "docs/spec.pdf", size: 12345 }],
  }
}

/**
 * Message with image attachment
 */
export function createMessageWithImage(): Message {
  return {
    id: "msg-image",
    role: "user",
    content: "What does this error mean?",
    timestamp: Date.now(),
    status: "complete",
    images: [{ name: "error.png", path: "/tmp/error.png", size: 45678 }],
  }
}

/**
 * Empty message
 */
export function createEmptyMessage(): Message {
  return {
    id: "msg-empty",
    role: "assistant",
    content: "",
    timestamp: Date.now(),
    status: "complete",
  }
}

/**
 * Message with special characters
 */
export function createSpecialCharsMessage(): Message {
  return {
    id: "msg-special",
    role: "assistant",
    content: `Special characters test:

Unicode: émojis 🎉 🚀 𝕌𝕟𝕚𝕔𝕠𝕕𝕖
Quotes: "double" 'single' \`backtick\`
Symbols: @ # $ % ^ & * ( ) [ ] { } | \\ : ; " ' < > , . ? /
Tabs and indents:
\tIndented line
    Another indent`,
    timestamp: Date.now(),
    status: "complete",
  }
}

/**
 * Create a conversation (array of messages)
 */
export function createConversation(): Message[] {
  return [
    {
      id: "msg-1",
      role: "user",
      content: "How do I create a REST API in Node.js?",
      timestamp: Date.now() - 3000,
      status: "complete",
    },
    {
      id: "msg-2",
      role: "assistant",
      content: "Here's a simple example using Express:\n\n```js\nconst express = require('express');\nconst app = express();\n\napp.get('/api/users', (req, res) => {\n  res.json({ users: [] });\n});\n\napp.listen(3000);\n```",
      timestamp: Date.now() - 2000,
      status: "complete",
    },
    {
      id: "msg-3",
      role: "user",
      content: "How do I add authentication?",
      timestamp: Date.now() - 1000,
      status: "complete",
    },
    {
      id: "msg-4",
      role: "assistant",
      content: "You can use middleware like JWT. Here's how:\n\n```js\nconst jwt = require('jsonwebtoken');\n\nfunction authMiddleware(req, res, next) {\n  const token = req.headers.authorization?.split(' ')[1];\n  if (!token) return res.status(401).json({ error: 'No token' });\n  \n  jwt.verify(token, process.env.SECRET, (err, decoded) => {\n    if (err) return res.status(403).json({ error: 'Invalid token' });\n    req.user = decoded;\n    next();\n  });\n}\n```",
      timestamp: Date.now(),
      status: "complete",
    },
  ]
}

/**
 * Message categories for organized testing
 */
export const messageFixtures = {
  markdown: createMarkdownMessage,
  diff: createDiffMessage,
  multiCode: createMultiCodeBlockMessage,
  streaming: createStreamingMessage,
  error: createErrorMessage,
  toolCall: createToolCallMessage,
  toolResult: createToolResultMessage,
  long: createLongMessage,
  mentions: createMessageWithMentions,
  file: createMessageWithFile,
  image: createMessageWithImage,
  empty: createEmptyMessage,
  specialChars: createSpecialCharsMessage,
}
