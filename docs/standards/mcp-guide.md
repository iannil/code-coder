# MCP (Model Context Protocol) Guide

## Overview

CodeCoder supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io), an open standard for connecting AI assistants to external tools and data sources. This enables agents to access additional capabilities beyond the built-in tools.

## Configuration

MCP servers are configured in your `~/.codecoder/config.jsonc` file.

### Configuration Structure

```jsonc
{
  "mcp": {
    "server-name": {
      "type": "remote" | "local",
      "enabled": true | false,
      // For remote servers
      "url": "https://...",
      "oauth": false | true | { "clientId": "..." },
      "headers": { "Authorization": "Bearer ..." },
      // For local servers
      "command": ["node", "path/to/server.js"],
      "environment": { "ENV_VAR": "value" },
      "timeout": 30000,
      // Agent-specific enablement
      "enabledAgents": ["agent-name"]
    }
  }
}
```

### Configuration Options

| Option | Type | Description |
|--------|------|-------------|
| `type` | `"remote"` \| `"local"` | Connection type |
| `enabled` | `boolean` | Whether the server is enabled (default: `true`) |
| `url` | `string` | URL for remote servers |
| `oauth` | `boolean` \| `object` | OAuth configuration for remote servers |
| `headers` | `object` | HTTP headers for remote servers |
| `command` | `string[]` | Command for local servers |
| `environment` | `object` | Environment variables for local servers |
| `timeout` | `number` | Connection timeout in milliseconds (default: `30000`) |
| `enabledAgents` | `string[]` | List of agents that can use this MCP server |

## MCP Server Examples

### Playwright MCP (Browser Automation)

The Playwright MCP server enables browser automation for the `code-reverse` agent.

```jsonc
{
  "mcp": {
    "playwright": {
      "type": "local",
      "command": ["npx", "-y", "@executeautomation/playwright-mcp-server"],
      "enabledAgents": ["code-reverse"],
      "timeout": 60000
    }
  }
}
```

**Features:**

- Screenshot capture
- Page navigation
- Network monitoring
- Form interaction
- Element extraction

**Usage in code-reverse agent:**

```bash
codecoder --agent code-reverse "Analyze https://example.com and generate a pixel-perfect recreation plan"
```

### Filesystem MCP

Access local filesystem through MCP:

```jsonc
{
  "mcp": {
    "filesystem": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-filesystem", "/allowed/path"],
      "enabledAgents": ["build"]
    }
  }
}
```

### GitHub MCP

Access GitHub repositories:

```jsonc
{
  "mcp": {
    "github": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-github"],
      "environment": {
        "GITHUB_TOKEN": "your-github-token"
      },
      "enabledAgents": ["build", "explore"]
    }
  }
}
```

### SQLite MCP

Query SQLite databases:

```jsonc
{
  "mcp": {
    "sqlite": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-sqlite", "--db-path", "./data.db"],
      "enabledAgents": ["build"]
    }
  }
}
```

### Remote MCP Server

Connect to a remote MCP server:

```jsonc
{
  "mcp": {
    "my-remote-server": {
      "type": "remote",
      "url": "https://my-server.com/mcp",
      "oauth": true
    }
  }
}
```

## Agent-Specific MCP Access

You can restrict which agents can access specific MCP servers using the `enabledAgents` option:

```jsonc
{
  "mcp": {
    "playwright": {
      "type": "local",
      "command": ["npx", "-y", "@executeautomation/playwright-mcp-server"],
      "enabledAgents": ["code-reverse", "build"]
    }
  }
}
```

If `enabledAgents` is not specified, all agents with MCP permissions can access the server.

## Authentication

### OAuth Authentication

For remote servers that require OAuth:

```jsonc
{
  "mcp": {
    "my-server": {
      "type": "remote",
      "url": "https://my-server.com/mcp",
      "oauth": {
        "clientId": "your-client-id",
        "clientSecret": "your-client-secret",
        "scope": "read:write"
      }
    }
  }
}
```

### Token-Based Authentication

```jsonc
{
  "mcp": {
    "my-server": {
      "type": "remote",
      "url": "https://my-server.com/mcp",
      "headers": {
        "Authorization": "Bearer your-token",
        "X-API-Key": "your-api-key"
      }
    }
  }
}
```

## Managing MCP Servers

### List MCP Servers

```bash
codecoder mcp list
```

### Connect/Disconnect MCP Server

```bash
codecoder mcp connect server-name
codecoder mcp disconnect server-name
```

### Authenticate MCP Server

For OAuth-enabled servers:

```bash
codecoder mcp auth server-name
```

## Troubleshooting

### MCP Server Not Connecting

1. Check the server status:

   ```bash
   codecoder mcp list
   ```

2. Verify the command or URL is correct

3. Check for authentication issues:

   ```bash
   codecoder mcp auth server-name
   ```

### MCP Tools Not Available to Agent

1. Verify the agent has MCP permissions enabled
2. Check if `enabledAgents` includes your agent
3. Verify the MCP server is connected

### Timeout Issues

Increase the timeout value:

```jsonc
{
  "mcp": {
    "server-name": {
      "timeout": 60000  // 60 seconds
    }
  }
}
```

## Available MCP Servers

Here are some popular MCP servers you can use:

| Server | Description | Installation |
|--------|-------------|--------------|
| `codecoder` (built-in) | CodeCoder's 20+ development tools | `ccode mcp serve` |
| `@executeautomation/playwright-mcp-server` | Browser automation | `npx -y @executeautomation/playwright-mcp-server` |
| `@modelcontextprotocol/server-filesystem` | Filesystem access | `npx -y @modelcontextprotocol/server-filesystem /path` |
| `@modelcontextprotocol/server-github` | GitHub integration | `npx -y @modelcontextprotocol/server-github` |
| `@modelcontextprotocol/server-sqlite` | SQLite database | `npx -y @modelcontextprotocol/server-sqlite --db-path ./data.db` |
| `@modelcontextprotocol/server-postgres` | PostgreSQL database | `npx -y @modelcontextprotocol/server-postgres` |
| `@modelcontextprotocol/server-brave-search` | Web search | `npx -y @modelcontextprotocol/server-brave-search` |

## CodeCoder MCP Server

CodeCoder includes a built-in MCP server that exposes all of its 20+ development tools, agent prompts, and project resources via MCP protocol. This allows external clients like ZeroBot to access CodeCoder's powerful capabilities.

### Features

- **Tools**: 20+ development tools (read, write, edit, bash, etc.)
- **Prompts**: 27 agent prompts (build, plan, code-reviewer, etc.)
- **Resources**: Project files (CLAUDE.md, README.md, package.json)
- **Transports**: stdio and HTTP (Streamable HTTP)
- **Authentication**: API key authentication for HTTP transport

### Starting the MCP Server

```bash
# Start in stdio mode (default)
ccode mcp serve

# Start with HTTP transport
ccode mcp serve --transport http --port 4420

# Start with API key authentication
ccode mcp serve --transport http --port 4420 --api-key your-secret-key

# Filter tools by agent
ccode mcp serve --agent code-reviewer

# Enable only specific tools
ccode mcp serve --tools "read,write,edit,glob,grep"
```

### CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `--transport` | Transport mode: `stdio` or `http` | `stdio` |
| `--port` | Port for HTTP transport | `4420` |
| `--api-key` | API key for HTTP authentication | none |
| `--agent` | Filter tools by agent name | all |
| `--tools` | Comma-separated list of enabled tools | all |

### Server Configuration (config.json)

You can configure the MCP server in `~/.codecoder/config.json` to set defaults:

```jsonc
{
  "mcp": {
    "server": {
      "apiKey": "your-secret-key",
      "port": 4420,
      "defaultTransport": "http",
      "resources": ["src/**/*.ts", "docs/**/*.md"]
    }
  }
}
```

**Configuration Options:**

| Option | Type | Description |
|--------|------|-------------|
| `apiKey` | `string` | Default API key for HTTP authentication |
| `port` | `number` | Default port for HTTP transport |
| `defaultTransport` | `"stdio"` \| `"http"` | Default transport mode |
| `resources` | `string[]` | Glob patterns for additional resources to expose |

CLI arguments override config file values. For example:

```bash
# Uses apiKey and port from config
ccode mcp serve --transport http

# Overrides config values
ccode mcp serve --transport http --port 5000 --api-key different-key
```

### HTTP Transport

The HTTP transport uses Streamable HTTP (MCP spec 2025-03-26), providing:

- **Session management**: Each client gets its own session
- **Resumability**: Sessions can be resumed after disconnection
- **Authentication**: API key via `Authorization: Bearer <key>` or `X-API-Key` header
- **Health check**: `/health` endpoint returns server status

### Configuring ZeroBot to use CodeCoder MCP Server

**Local (stdio) connection:**

```jsonc
{
  "mcp": {
    "codecoder": {
      "type": "local",
      "command": ["ccode", "mcp", "serve"],
      "enabled": true
    }
  }
}
```

**Remote (HTTP) connection:**

```jsonc
{
  "mcp": {
    "codecoder": {
      "type": "remote",
      "url": "http://localhost:4420/mcp",
      "headers": {
        "X-API-Key": "your-secret-key"
      }
    }
  }
}
```

### Available Tools

The CodeCoder MCP Server exposes these tools:

| Category | Tools |
|----------|-------|
| File Operations | `read`, `write`, `edit`, `glob`, `grep` |
| Shell | `bash` |
| Web | `webfetch`, `websearch` |
| Code Search | `codesearch` |
| Task Management | `task`, `todo_write`, `todo_read` |
| Questions | `question` |
| Skills | `skill` |
| Credentials | `get_credential` |
| Network | `network_analyzer` |
| Batch | `batch` (experimental) |
| LSP | `lsp` (experimental) |

### Available Prompts

The MCP server exposes agent prompts for use by clients:

| Category | Prompts |
|----------|---------|
| Primary Modes | `build`, `plan`, `writer`, `autonomous` |
| Reverse Engineering | `code-reverse`, `jar-code-reverse` |
| Code Quality | `code-reviewer`, `security-reviewer`, `tdd-guide`, `architect`, `verifier` |
| Content | `writer`, `proofreader`, `expander`, `expander-fiction`, `expander-nonfiction` |
| Exploration | `general`, `explore` |
| 祝融说 (Zhurong) | `observer`, `decision`, `macro`, `trader`, `picker`, `miniproduct`, `ai-engineer` |
| Utility | `synton-assistant` |

### Available Resources

The MCP server exposes project files as resources:

**Built-in Resources:**

| Resource | Description | URI |
|----------|-------------|-----|
| `CLAUDE.md` | Project instructions | `file://<workdir>/CLAUDE.md` |
| `README.md` | Project documentation | `file://<workdir>/README.md` |
| `package.json` | Package configuration | `file://<workdir>/package.json` |

**Dynamic Resources:**

Configure glob patterns in `~/.codecoder/config.json` to expose additional files:

```jsonc
{
  "mcp": {
    "server": {
      "resources": ["src/**/*.ts", "docs/**/*.md", "*.yaml"]
    }
  }
}
```

The server also provides resource templates for dynamic resource discovery:

```bash
# List resource templates
mcp resources/templates/list
# Returns templates like: file://<workdir>/src/**/*.ts
```

### Integration with ZeroBot

When configured, ZeroBot will automatically receive tools from CodeCoder via MCP, replacing the need for the direct `codecoder` HTTP tool. This provides:

- **Unified tool registry**: All tools available through standard MCP protocol
- **Better performance**: Direct stdio communication instead of HTTP/SSE
- **Consistency**: Same tool definitions used by both CodeCoder and ZeroBot

## Best Practices

1. **Security**: Only enable MCP servers for agents that need them
2. **Scope**: Use `enabledAgents` to limit access
3. **Timeouts**: Set appropriate timeouts for long-running operations
4. **Authentication**: Use environment variables for sensitive data
5. **Testing**: Test MCP server connectivity before using in production

## See Also

- [Agent Configuration Guide](./agent-guide.md)
- [Code-Reverse Mode](../reports/completed/code-reverse-mode.md)
- [MCP Specification](https://spec.modelcontextprotocol.io/)
