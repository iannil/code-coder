# Code-Reverse Mode

## Overview

**code-reverse** is a primary agent mode designed for website reverse engineering. It analyzes websites, identifies technology stacks, extracts design systems, maps API endpoints, and generates comprehensive development plans for pixel-perfect recreation.

## Features

### 1. Technology Stack Detection

Automatically detects:
- **Frontend Frameworks**: React, Vue, Angular, Svelte, Solid, Next.js, Nuxt, Remix, Astro, and more
- **UI Libraries**: Tailwind CSS, Material-UI, Ant Design, Chakra UI, shadcn/ui, Radix UI, Element Plus, Bootstrap, Bulma, and more
- **State Management**: Redux, Zustand, Pinia, MobX, Recoil, Jotai, XState, Valtio, Apollo Client, TanStack Query
- **Build Tools**: Vite, Webpack, Rollup, esbuild, Parcel, Turbopack, Rspack, SWC
- **Styling**: CSS Modules, Emotion, Styled Components, Linaria, SCSS/Less, Panda CSS

### 2. Infrastructure Detection

Identifies:
- **Hosting**: Vercel, Netlify, Cloudflare, AWS, Azure, Google Cloud, Deno Deploy, Railway, Fly.io
- **CDN**: Cloudflare, AWS CloudFront, etc.
- **Analytics**: Google Analytics, Plausible, PostHog, Segment, Hotjar, FullStory, Mixpanel, Amplitude, Heap
- **Monitoring**: Sentry, LogRocket, Bugsnag, Datadog, Rollbar
- **Authentication**: Auth0, Firebase, Clerk, NextAuth, Supabase, Lucia

### 3. Design System Extraction

Extracts:
- **Colors**: Primary, secondary, accent, background, text colors (hex codes)
- **Typography**: Font families, sizes, weights, line heights
- **Layout**: Container widths, spacing scales, breakpoints
- **Design Tokens**: Border radius, shadows, transitions

### 4. API Mapping

Documents:
- REST API endpoints
- GraphQL queries/mutations
- Request methods and paths
- Authentication requirements
- Response structures

### 5. Development Plan Generation

Generates:
- Technology recommendations
- File structure
- Component breakdown
- Implementation phases
- Time estimates

## Usage

### Basic Usage

```bash
# Analyze a website with the code-reverse agent
codecoder --agent code-reverse "Analyze https://example.com and generate a pixel-perfect recreation plan"
```

### Using the reverse command

```bash
# Quick analysis
codecoder reverse analyze https://example.com

# Specify output directory
codecoder reverse analyze https://example.com --output ./reports

# Interactive mode with TUI
codecoder reverse analyze https://example.com --interactive

# Deep analysis
codecoder reverse analyze https://example.com --depth deep

# JSON output
codecoder reverse analyze https://example.com --format json
```

### List available technology fingerprints

```bash
# List all categories
codecoder reverse list

# List technologies in a specific category
codecoder reverse list --category frontend
codecoder reverse list --category ui
codecoder reverse list --category hosting
```

## Configuration

### Agent Permissions

The `code-reverse` agent has the following permissions:
- **read**: Read files in the project
- **webfetch**: Fetch web content
- **websearch**: Search the web
- **bash**: Run commands for network analysis
- **write**: Write reports (restricted to reports directory)
- **question**: Ask for user input
- **plan_enter/plan_exit**: Plan mode capabilities

### MCP Integration (Optional)

For enhanced browser automation capabilities, configure the Playwright MCP server:

**Step 1**: Add to `~/.ccode/config.jsonc`:

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

**Step 2**: Verify MCP connection:

```bash
codecoder mcp list
```

## Output

### Markdown Report

The default output is a comprehensive Markdown report saved to:
- `~/.ccode/data/reports/reverse/<hostname>-<date>.md`

Report structure:
```markdown
# Website Reverse Analysis: <URL>

## Executive Summary
[Brief overview]

## Technology Stack
### Frontend
- Framework: [...]
- UI Library: [...]
- State Management: [...]
- Build Tool: [...]
- Styling: [...]

### Backend (Inferred)
- Framework: [...]
- API Style: [...]
- API Base URL: [...]

### Infrastructure
- Hosting: [...]
- CDN: [...]
- Analytics: [...]
- Monitoring: [...]

## Design System
### Colors
- Primary: [...]
- Secondary: [...]
- [...]

### Typography
- Headings: [...]
- Body: [...]

### Layout & Spacing
- Container max-width: [...]
- Spacing scale: [...]
- Breakpoints: [...]

## Component Structure
[Component breakdown]

## API Endpoints
[Endpoint documentation]

## Development Plan
### Phase 1: Project Setup
[Implementation steps]

### Phase 2: Core Components
[Implementation steps]

[...]

## File Structure
[Recommended directory structure]

## Estimated Effort
[Time estimates]
```

### Interactive TUI Output

With `--interactive`, you get a visual summary:
- Technology stack display
- Color palette preview
- Component list with complexity
- API endpoint overview
- Phase breakdown with time estimates

### JSON Output

For programmatic access, use `--format json`:

```json
{
  "url": "https://example.com",
  "executiveSummary": "...",
  "techStack": { ... },
  "designSystem": { ... },
  "components": [ ... ],
  "apiEndpoints": [ ... ],
  "phases": [ ... ],
  "fileStructure": "...",
  "totalEstimatedTime": "..."
}
```

## Technology Fingerprint Database

The code-reverse agent uses a comprehensive fingerprint database with 500+ patterns across 15+ categories:

| Category | Technologies |
|----------|--------------|
| Frontend Frameworks | React, Vue, Angular, Svelte, Solid, Next.js, Nuxt, Remix, Astro, Umi, SvelteKit, Gatsby, Qwik |
| UI Libraries | Tailwind CSS, Material-UI, Ant Design, Chakra UI, shadcn/ui, Radix UI, Element Plus, Arco Design, Bootstrap, Bulma, Foundation, Semantic UI, PrimeVue, Vuetify, Quasar |
| State Management | Redux, Zustand, Pinia, MobX, Recoil, Jotai, XState, Valtio, Apollo Client, TanStack Query, SWR |
| Build Tools | Vite, Webpack, Rollup, esbuild, Parcel, Turbopack, Rspack, SWC |
| Styling | CSS Modules, Emotion, Styled Components, Linaria, Vanilla Extract, SCSS/Sass, Less, Panda CSS |
| Backend (Inferred) | Express, Fastify, NestJS, Django, Rails, Laravel, Spring Boot, FastAPI, Next.js API, Nuxt Server |
| Hosting | Vercel, Netlify, Cloudflare, AWS, Azure, Google Cloud, Deno Deploy, Railway, Fly.io, Render, Heroku |
| Analytics | Google Analytics, Plausible, Fathom, PostHog, Segment, Hotjar, FullStory, Mixpanel, Amplitude, Heap, Umami |
| Monitoring | Sentry, LogRocket, Bugsnag, Datadog, Rollbar, Airbrake |
| Authentication | Auth0, Firebase, Clerk, NextAuth, Supabase, Lucia |
| Payment | Stripe, PayPal, Shopify |

## Examples

### Example 1: Analyze a Vercel-hosted Next.js site

```bash
codecoder reverse analyze https://vercel.com --interactive
```

Output:
```
🔍 WEBSITE REVERSE ANALYSIS: vercel.com
============================================================

📊 TECHNOLOGY STACK
  Frontend:
    • Framework: Next.js
    • UI Library: Custom
    • Styling: Tailwind CSS

  Infrastructure:
    • Hosting: Vercel
    • CDN: Vercel Edge

🎨 DESIGN SYSTEM
  Primary Colors: #0070f3, #7928ca
  Font Family: Geist Sans, Geist Mono

🔧 COMPONENTS DETECTED: 12
🌐 API ENDPOINTS: 5

⏱️  ESTIMATED EFFORT
  Phase 1: Project Setup: 2-3 hours
  Phase 2: Core Layout Components: 4-6 hours
  Phase 3: UI Components: 6-8 hours
  Phase 4: Page Implementation: 8-12 hours
  Phase 5: Polish & Deploy: 4-6 hours
  **TOTAL: 3-5 days**

============================================================
```

### Example 2: Analyze a Vue 3 site

```bash
codecoder reverse analyze https://vuejs.org
```

Generates a comprehensive report with:
- Vue 3 framework detection
- Vite build tool identification
- Design system extraction
- API endpoint documentation
- Step-by-step recreation plan

## Implementation Details

### Files Added

| File | Purpose |
|------|---------|
| `packages/ccode/src/agent/prompt/code-reverse.txt` | System prompt for the agent |
| `packages/ccode/src/agent/agent.ts` | Agent definition (modified) |
| `packages/ccode/src/util/tech-fingerprints.ts` | Technology fingerprint database |
| `packages/ccode/src/util/report-generator.ts` | Report generation utilities |
| `packages/ccode/src/tool/network-analyzer.ts` | Network analysis tool |
| `packages/ccode/src/cli/cmd/reverse.ts` | CLI command implementation |
| `packages/ccode/src/index.ts` | CLI registration (modified) |

### Architecture

```
code-reverse Agent
    ├─ System Prompt (code-reverse.txt)
    │      └─ Fingerprint database instructions
    │
    ├─ Tools
    │      ├─ webfetch (built-in)
    │      ├─ bash (built-in)
    │      └─ network-analyzer (new)
    │
    ├─ Utilities
    │      ├─ tech-fingerprints.ts
    │      └─ report-generator.ts
    │
    └─ CLI
           └─ reverse command
                  ├─ analyze <url>
                  └─ list
```

## Troubleshooting

### "Agent not found" error

Ensure the agent is properly registered:
```bash
codecoder agent list | grep code-reverse
```

### MCP connection issues

Verify Playwright MCP is configured and connected:
```bash
codecoder mcp list
codecoder mcp connect playwright
```

### Report not generated

Check the reports directory:
```bash
ls -la ~/.ccode/data/reports/reverse/
```

## Future Enhancements

1. **Screenshot Capture**: Integrate Playwright MCP for visual analysis
2. **Component Tree Extraction**: Deep component hierarchy analysis
3. **Network Traffic Recording**: Complete XHR/fetch capture
4. **Interactive Diff**: Side-by-side comparison tool
5. **Template Generation**: Auto-generate starter project
6. **Multi-page Analysis**: Analyze entire site structure
7. **Performance Metrics**: Lighthouse-style scoring
8. **Accessibility Audit**: WCAG compliance checking

## See Also

- [MCP Guide](../../standards/mcp-guide.md)
- [Agent Configuration](../agent-guide.md)
- [Developer Guide](../developer-guide.md)
