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
| `@executeautomation/playwright-mcp-server` | Browser automation | `npx -y @executeautomation/playwright-mcp-server` |
| `@modelcontextprotocol/server-filesystem` | Filesystem access | `npx -y @modelcontextprotocol/server-filesystem /path` |
| `@modelcontextprotocol/server-github` | GitHub integration | `npx -y @modelcontextprotocol/server-github` |
| `@modelcontextprotocol/server-sqlite` | SQLite database | `npx -y @modelcontextprotocol/server-sqlite --db-path ./data.db` |
| `@modelcontextprotocol/server-postgres` | PostgreSQL database | `npx -y @modelcontextprotocol/server-postgres` |
| `@modelcontextprotocol/server-brave-search` | Web search | `npx -y @modelcontextprotocol/server-brave-search` |

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
