# Phase 71: MCP Server Foundation + Tier 0-1 Tools

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-mcp-server-design.md](../refs/specs/2026-04-03-mcp-server-design.md)
**Depends on:** Phase 70 (alignment API in SDK — `AlignmentResource` with `score()` and `matchRequirements()` methods), existing SDK (Phase 5+), HTTP health route (`GET /api/health`)
**Blocks:** Phase 72 (Tier 2+3 combined: data management + refinement tools)
**Parallelizable with:** Independent of all UI phases. Independent of Phase 60-68 (visualization/JD phases).
**Duration:** Medium-Long

## Goal

Replace the stub in `packages/mcp/src/index.ts` with a fully functional MCP server using STDIO transport. Register all 7 MCP resources (profile, archetypes, domains, templates, resume by ID, resume IR by ID, job description by ID). Implement all 21 Tier 0 + Tier 1 tools: 1 health check, 3 search tools, 3 get-by-ID tools, 1 list tool, 2 derivation tools, 4 review tools, 3 resume assembly tools, 3 analysis tools (gap analysis, alignment, requirement matching), and 1 export tool. Every tool delegates to `@forge/sdk` via a shared `ForgeClient` instance. Error mapping converts SDK `Result<T>` failures into MCP `isError: true` responses with human-readable messages.

## Non-Goals

- Tier 2 tools (create entities, JD skills, profile updates, review queue, drift) -- Phase 72
- Tier 3 tools (update, reorder, trace, clone, notes) -- Phase 72 (Tier 2+3 combined)
- SSE or Streamable HTTP transport -- future transport phase
- Resource subscriptions (`resources/subscribe`) -- deferred to SSE transport
- Direct database access from the MCP server (all calls go through SDK)
- Delete tools (intentionally omitted from MCP surface per spec design principle 5)
- Authentication or authorization on the MCP server (STDIO runs as local process)
- Custom prompts or sampling (MCP prompts feature) -- AI reasoning stays with the client
- `forge_search_job_descriptions` is Tier 2 (Phase 72). Cold-start agents can read `forge://job/{id}` if they know the ID.

## Context

The MCP server is the machine-to-machine interface that allows AI assistants (Claude Code, Claude Desktop, Cursor, Windsurf) to interact with the Forge resume builder programmatically. It sits between the AI client and the existing `@forge/sdk`, which in turn calls the Forge HTTP API (`@forge/core`).

The current `packages/mcp/src/index.ts` is a stub with only an `export {}` statement and comments listing intended tools. The `package.json` already declares the `@forge/sdk` workspace dependency and has `dev` and `build` scripts.

The `@forge/sdk` `ForgeClient` class provides all necessary resource sub-clients (`sources`, `bullets`, `perspectives`, `resumes`, `templates`, `profile`, `jobDescriptions`, `export`, `archetypes`, `domains`) with typed `Result<T>` and `PaginatedResult<T>` return values. The MCP server's job is to:
1. Accept MCP tool calls via STDIO
2. Parse parameters using Zod schemas
3. Delegate to the appropriate SDK method
4. Map `Result<T>` success/failure to MCP response format
5. Return structured JSON data as text content

**STDIO polling pattern:** After any mutating tool call that affects a resume (add entry, create section, etc.), the AI client should re-read the `forge://resume/{id}` resource to get updated state. The MCP server does not push updates.

## Task Dependency DAG

`T71.2 ∥ T71.3 → T71.4 → T71.5-T71.12 (parallel) → T71.13 → T71.1 → T71.14 → T71.15 ∥ T71.16`

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| MCP Resources (7 resources with URI patterns) | Yes |
| Tier 0: Diagnostics (forge_health) | Yes |
| Tier 1: Core Workflow (20 tools: 19 from spec + 1 forge_list_resumes) | Yes |
| Tier 2: Data Management (18 tools) | No -- Phase 72 |
| Tier 3: Refinement (16 tools) | No -- Phase 72 (combined) |
| Implementation Notes (STDIO transport, error handling) | Yes |
| Embedding Service Design | No -- prerequisite, assumed available |
| JD Ingestion Pipeline Design | No -- Tier 2 |

## Files to Create

| File | Description |
|------|-------------|
| `packages/mcp/src/server.ts` | MCP server factory: creates `McpServer`, registers resources and tools, returns server instance |
| `packages/mcp/src/utils/error-mapper.ts` | Maps SDK `Result<T>` to MCP tool response format |
| `packages/mcp/src/utils/register-tool.ts` | Helper to reduce tool registration boilerplate |
| `packages/mcp/src/resources.ts` | All 7 MCP resource registrations |
| `packages/mcp/src/tools/tier0.ts` | `forge_health` tool registration |
| `packages/mcp/src/tools/search.ts` | 3 search tools (sources, bullets, perspectives) |
| `packages/mcp/src/tools/get.ts` | 3 get-by-ID tools (source, bullet, perspective) |
| `packages/mcp/src/tools/list.ts` | 1 list tool (forge_list_resumes) |
| `packages/mcp/src/tools/derive.ts` | 2 derivation tools (bullets, perspective) |
| `packages/mcp/src/tools/review.ts` | 4 review tools (approve/reject bullet, approve/reject perspective) |
| `packages/mcp/src/tools/assembly.ts` | 3 resume assembly tools (create resume, add entry, create section) |
| `packages/mcp/src/tools/analysis.ts` | 3 analysis tools (gap_analysis, align_resume, match_requirements) |
| `packages/mcp/src/tools/export.ts` | 1 export tool (forge_export_resume) |
| `packages/mcp/claude_desktop_config.example.json` | Example Claude Desktop / Claude Code MCP config |
| `packages/mcp/README.md` | What the MCP server is, how to start it, transports supported, env vars, prerequisites |
| `packages/mcp/src/__tests__/error-mapper.test.ts` | Unit tests for error mapping |
| `packages/mcp/src/__tests__/integration.test.ts` | Integration tests: tool calls via StdioClientTransport |

## Files to Modify

| File | Change |
|------|--------|
| `packages/mcp/src/index.ts` | Replace stub with entry point: ForgeClient init, server creation, STDIO transport connect, graceful shutdown |
| `packages/mcp/package.json` | Add `@modelcontextprotocol/sdk` and `zod` dependencies |

## Prerequisites

- [ ] Confirm `packages/mcp` is listed in workspace root's `package.json` workspaces array. If not, add it before starting tasks.

## Fallback Strategies

- **Forge server not running:** `forge_health` returns `isError: true` with "Cannot reach Forge server -- is it running? Start it with `bun run packages/core/src/index.ts`". All other tools will also fail with NETWORK_ERROR but health check provides the diagnostic path.
- **SDK method not yet implemented (alignment):** If `sdk.alignment` is undefined (Phase 70 not landed), `forge_align_resume` and `forge_match_requirements` return `isError: true` with "Alignment API not available -- Phase 70 required". The server still starts and all other tools work.
- **MCP client doesn't support resources:** Resources are optional in the MCP spec. The server works fine with tools only -- the client just won't have ambient context loaded.
- **Resource read fails (server down during resource fetch):** Return the error message as the resource text content so the AI client can see what went wrong. Resource error content uses JSON format for machine readability. Tool errors use human-readable format. This is intentional -- resources are read by code, tools are read by AI.
- **PDF export temp file write fails:** Return `isError: true` with the OS error message. The AI client should suggest the user check disk space or permissions. Temp files are the user's/OS's responsibility. Known limitation. Files match pattern `forge-resume-*.pdf` in $TMPDIR.
- **Tool call with invalid parameters:** Zod validation rejects the input before the SDK call. Returns `isError: true` with field-level validation details from Zod.
- **STDIO transport disconnects:** The `McpServer` handles transport lifecycle. On disconnect, the process exits cleanly via the shutdown handler.

---

## Tasks

### T71.1: Set Up MCP Server Entry Point with STDIO Transport

**Files:**
- Replace: `packages/mcp/src/index.ts`
- Modify: `packages/mcp/package.json`

[CRITICAL] The entry point must initialize the `ForgeClient` with a configurable base URL from `FORGE_API_URL` environment variable (defaulting to `http://localhost:3000`). The server must handle graceful shutdown on SIGINT/SIGTERM.

[IMPORTANT] The `@modelcontextprotocol/sdk` package exports from subpath modules. Use exact import paths: `@modelcontextprotocol/sdk/server/mcp.js` and `@modelcontextprotocol/sdk/server/stdio.js`.

**package.json additions:**

```json
{
  "dependencies": {
    "@forge/sdk": "workspace:*",
    "@modelcontextprotocol/sdk": "^1.12.0",
    "zod": "^3.24.0"
  }
}
```

**Entry point (`packages/mcp/src/index.ts`):**

```typescript
import { ForgeClient } from '@forge/sdk'
import { createForgeServer } from './server'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'

const baseUrl = process.env.FORGE_API_URL ?? 'http://localhost:3000'
const sdk = new ForgeClient({ baseUrl })

const server = createForgeServer(sdk)
const transport = new StdioServerTransport()

// Graceful shutdown
const shutdown = async () => {
  await server.close()
  process.exit(0)
}
process.on('SIGINT', shutdown)
process.on('SIGTERM', shutdown)

await server.connect(transport)
```

**Acceptance criteria:**
- `bun run packages/mcp/src/index.ts` starts the MCP server on STDIO without errors.
- `FORGE_API_URL` env var overrides the default base URL.
- SIGINT and SIGTERM trigger graceful shutdown (server.close() called, process exits 0).
- The process does not print anything to stdout (STDIO transport uses stdout for MCP protocol messages).
- Diagnostic/debug output goes to stderr only.

**Failure criteria:**
- Importing from `@modelcontextprotocol/sdk` without subpath (e.g., `@modelcontextprotocol/sdk/server`) fails at runtime.
- Writing log messages to stdout corrupts the STDIO transport protocol.
- Missing `await` on `server.connect(transport)` causes the process to exit immediately.

**Test kind:** Smoke -- start the process, verify it stays alive, send a SIGTERM, verify exit code 0.

---

### T71.2: Create Error Mapping Utility

**File:** `packages/mcp/src/utils/error-mapper.ts`

[CRITICAL] This is the single point of truth for converting SDK `Result<T>` into MCP tool responses. Every tool handler uses this function. The error messages must be human-readable because AI clients display them verbatim.

```typescript
import type { Result, PaginatedResult } from '@forge/sdk'

/** MCP tool response content block. */
interface McpToolContent {
  type: 'text'
  text: string
}

/** MCP tool response shape. */
interface McpToolResponse {
  content: McpToolContent[]
  isError?: boolean
}

/**
 * Map an SDK Result<T> to an MCP tool response.
 *
 * Success: JSON-serialized data as text content.
 * Failure: Human-readable error message with isError: true.
 */
export function mapResult<T>(result: Result<T>): McpToolResponse {
  if (result.ok) {
    return {
      content: [{ type: 'text', text: JSON.stringify(result.data, null, 2) }],
    }
  }

  const message = formatError(result.error.code, result.error.message, result.error.details)
  return {
    content: [{ type: 'text', text: message }],
    isError: true,
  }
}

/**
 * Map an SDK PaginatedResult<T> to an MCP tool response.
 *
 * Includes pagination metadata in the response.
 */
export function mapPaginatedResult<T>(result: PaginatedResult<T>): McpToolResponse {
  if (result.ok) {
    return {
      content: [{
        type: 'text',
        text: JSON.stringify({
          data: result.data,
          pagination: result.pagination,
        }, null, 2),
      }],
    }
  }

  const message = formatError(result.error.code, result.error.message, result.error.details)
  return {
    content: [{ type: 'text', text: message }],
    isError: true,
  }
}

/**
 * Format an error code + message into a human-readable string.
 *
 * Error code mapping (from spec):
 *   NOT_FOUND           → "Entity not found: {message}"
 *   VALIDATION_ERROR    → "Validation failed: {message}" + field details if available
 *   CONFLICT            → "Conflict: {message}" (e.g., "Source is locked for derivation")
 *   AI_ERROR            → "AI derivation failed -- retry or check server logs"
 *   GATEWAY_TIMEOUT     → "AI call timed out -- retry"
 *   NETWORK_ERROR       → "Cannot reach Forge server -- is it running?"
 *   SERVICE_UNAVAILABLE → "Service unavailable: {message}. Check that the embedding service is running."
 *   *                   → "Error [{code}]: {message}"
 */
function formatError(code: string, message: string, details?: unknown): string {
  switch (code) {
    case 'NOT_FOUND':
      return `Entity not found: ${message}`
    case 'VALIDATION_ERROR': {
      let text = `Validation failed: ${message}`
      if (details && typeof details === 'object') {
        text += '\n' + JSON.stringify(details, null, 2)
      }
      return text
    }
    case 'CONFLICT':
      return `Conflict: ${message}`
    case 'AI_ERROR':
      return `AI derivation failed -- retry or check server logs. Details: ${message}`
    case 'GATEWAY_TIMEOUT':
      return `AI call timed out -- retry. Details: ${message}`
    case 'NETWORK_ERROR':
      return `Cannot reach Forge server -- is it running? Start with: bun run packages/core/src/index.ts\nDetails: ${message}`
    case 'SERVICE_UNAVAILABLE':
      return `Service unavailable: ${message}. Check that the embedding service is running.`
    default:
      return `Error [${code}]: ${message}`
  }
}
```

**Acceptance criteria:**
- `mapResult({ ok: true, data: { id: '123' } })` returns `{ content: [{ type: 'text', text: '{\n  "id": "123"\n}' }] }` (no `isError`).
- `mapResult({ ok: false, error: { code: 'NOT_FOUND', message: 'bullet abc' } })` returns `{ content: [{ type: 'text', text: 'Entity not found: bullet abc' }], isError: true }`.
- `mapPaginatedResult` includes both `data` and `pagination` in the success response JSON.
- All 7 error codes produce distinct, human-readable messages.
- NETWORK_ERROR message includes the hint about starting the Forge server.
- SERVICE_UNAVAILABLE message mentions the embedding service.

**Failure criteria:**
- Returning raw SDK error objects instead of formatted strings (AI clients can't parse them).
- Missing `isError: true` on error responses (MCP clients won't treat them as errors).
- Pretty-printing disabled on success responses (large JSON blobs are unreadable).

**Test kind:** Unit -- no SDK or server needed. Pure input/output mapping.

**Test fixtures:**

```typescript
// Success
mapResult({ ok: true, data: { id: '1', content: 'test' } })
// → { content: [{ type: 'text', text: '{\n  "id": "1",\n  "content": "test"\n}' }] }

// NOT_FOUND
mapResult({ ok: false, error: { code: 'NOT_FOUND', message: 'bullet abc-123' } })
// → { content: [{ type: 'text', text: 'Entity not found: bullet abc-123' }], isError: true }

// VALIDATION_ERROR with details
mapResult({
  ok: false,
  error: {
    code: 'VALIDATION_ERROR',
    message: 'Invalid input',
    details: { source_id: 'required' },
  },
})
// → { content: [{ type: 'text', text: 'Validation failed: Invalid input\n{\n  "source_id": "required"\n}' }], isError: true }

// NETWORK_ERROR
mapResult({ ok: false, error: { code: 'NETWORK_ERROR', message: 'Connection refused' } })
// → { content: [{ type: 'text', text: 'Cannot reach Forge server -- is it running? Start with: ...\nDetails: Connection refused' }], isError: true }

// SERVICE_UNAVAILABLE
mapResult({ ok: false, error: { code: 'SERVICE_UNAVAILABLE', message: 'Embedding service down' } })
// → { content: [{ type: 'text', text: 'Service unavailable: Embedding service down. Check that the embedding service is running.' }], isError: true }

// Paginated success
mapPaginatedResult({
  ok: true,
  data: [{ id: '1' }, { id: '2' }],
  pagination: { total: 50, offset: 0, limit: 20 },
})
// → { content: [{ type: 'text', text: '{\n  "data": [...],\n  "pagination": {...}\n}' }] }
```

---

### T71.3: Create Tool Registration Helper

**File:** `packages/mcp/src/utils/register-tool.ts`

[IMPORTANT] This helper reduces boilerplate by wrapping the `server.tool()` call with try/catch, error mapping, and stderr logging. Without it, every tool handler would duplicate the same error handling pattern.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { z, ZodRawShape } from 'zod'

/**
 * Register an MCP tool with standardized error handling.
 *
 * Wraps the handler in try/catch so unhandled exceptions (e.g., Zod parse
 * failures, unexpected SDK errors) become MCP error responses instead of
 * crashing the server.
 *
 * Success logging is gated behind FORGE_MCP_DEBUG env var to avoid noise.
 * Error logging is unconditional.
 *
 * @param server - The McpServer instance
 * @param name - Tool name (e.g., 'forge_search_sources')
 * @param description - Human-readable tool description for AI clients
 * @param schema - Zod schema shape defining the tool's parameters
 * @param handler - Async function that receives validated params and returns an MCP response
 */
export function registerTool<S extends ZodRawShape>(
  server: McpServer,
  name: string,
  description: string,
  schema: S,
  handler: (params: z.objectOutputType<z.ZodObject<S>, z.ZodTypeAny>) => Promise<{
    content: Array<{ type: 'text'; text: string }>
    isError?: boolean
  }>,
): void {
  server.tool(name, description, schema, async (params) => {
    const start = performance.now()
    try {
      const result = await handler(params)
      const duration = Math.round(performance.now() - start)
      if (result.isError) {
        console.error(`[forge:mcp] ${name} ERROR ${duration}ms`)
      } else if (process.env.FORGE_MCP_DEBUG) {
        console.error(`[forge:mcp] ${name} ok ${duration}ms`)
      }
      return result
    } catch (err) {
      const duration = Math.round(performance.now() - start)
      console.error(`[forge:mcp] ${name} UNHANDLED_ERROR ${duration}ms: ${String(err)}`)
      return {
        content: [{ type: 'text' as const, text: `Internal error in ${name}: ${String(err)}` }],
        isError: true,
      }
    }
  })
}
```

[IMPORTANT] Logging goes to `console.error` (stderr), NOT `console.log` (stdout). STDIO transport uses stdout for protocol messages. Any stdout pollution breaks the transport.

**Acceptance criteria:**
- `registerTool(server, 'test', 'desc', { id: z.string() }, handler)` registers the tool on the server.
- Handler exceptions are caught and returned as `isError: true` responses.
- Success logging only appears when `FORGE_MCP_DEBUG` is set. Error logging is unconditional.
- Each error tool call logs to stderr: `[forge:mcp] tool_name ERROR 5ms`.
- The handler receives the validated (parsed) parameters, not raw input.

**Failure criteria:**
- Using `console.log` instead of `console.error` for logging (breaks STDIO).
- Not catching handler exceptions (crashes the MCP server process).
- Swallowing the error message (AI client gets no diagnostic info).

**Test kind:** Unit -- mock the `McpServer.tool()` method, verify the wrapper behavior.

---

### T71.4: Register All 7 MCP Resources

**File:** `packages/mcp/src/resources.ts`

[CRITICAL] Resources provide ambient context that AI clients load at session start. The 4 static resources (profile, archetypes, domains, templates) are simple URI-to-SDK-call mappings. The 3 parameterized resources (resume by ID, resume IR by ID, job description by ID) use MCP resource templates with URI patterns.

[IMPORTANT] The `@modelcontextprotocol/sdk` McpServer API uses `server.resource()` for static resources and `server.resource()` with a URI template for parameterized resources. The URI template syntax is `forge://resume/{id}` where `{id}` is a placeholder.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'

/**
 * Register all 7 MCP resources on the server.
 *
 * Static resources (polled on session start):
 *   forge://profile     → sdk.profile.get()
 *   forge://archetypes  → sdk.archetypes.list()
 *   forge://domains     → sdk.domains.list()
 *   forge://templates   → sdk.templates.list()
 *
 * Parameterized resources (fetched by ID):
 *   forge://resume/{id}    → sdk.resumes.get(id)
 *   forge://resume/{id}/ir → sdk.resumes.ir(id)
 *   forge://job/{id}       → sdk.jobDescriptions.get(id)
 */
export function registerResources(server: McpServer, sdk: ForgeClient): void {
  // -- Static resources --

  server.resource(
    'forge-profile',
    'forge://profile',
    'User profile: name, contact info, clearance level',
    async (uri) => {
      const result = await sdk.profile.get()
      if (!result.ok) {
        return {
          contents: [{
            uri: uri.href,
            mimeType: 'application/json',
            text: JSON.stringify({ error: result.error }),
          }],
        }
      }
      return {
        contents: [{
          uri: uri.href,
          mimeType: 'application/json',
          text: JSON.stringify(result.data, null, 2),
        }],
      }
    },
  )

  server.resource(
    'forge-archetypes',
    'forge://archetypes',
    'Career archetypes with domain associations (e.g., platform-engineer, security-engineer)',
    async (uri) => {
      const result = await sdk.archetypes.list()
      if (!result.ok) {
        return {
          contents: [{
            uri: uri.href,
            mimeType: 'application/json',
            text: JSON.stringify({ error: result.error }),
          }],
        }
      }
      return {
        contents: [{
          uri: uri.href,
          mimeType: 'application/json',
          text: JSON.stringify(result.data, null, 2),
        }],
      }
    },
  )

  server.resource(
    'forge-domains',
    'forge://domains',
    'Skill domain taxonomy (e.g., infrastructure, security, ai_ml, cloud)',
    async (uri) => {
      const result = await sdk.domains.list()
      if (!result.ok) {
        return {
          contents: [{
            uri: uri.href,
            mimeType: 'application/json',
            text: JSON.stringify({ error: result.error }),
          }],
        }
      }
      return {
        contents: [{
          uri: uri.href,
          mimeType: 'application/json',
          text: JSON.stringify(result.data, null, 2),
        }],
      }
    },
  )

  server.resource(
    'forge-templates',
    'forge://templates',
    'Resume templates with section structures',
    async (uri) => {
      const result = await sdk.templates.list()
      if (!result.ok) {
        return {
          contents: [{
            uri: uri.href,
            mimeType: 'application/json',
            text: JSON.stringify({ error: result.error }),
          }],
        }
      }
      return {
        contents: [{
          uri: uri.href,
          mimeType: 'application/json',
          text: JSON.stringify(result.data, null, 2),
        }],
      }
    },
  )

  // -- Parameterized resources (resource templates) --

  server.resource(
    'forge-resume',
    'forge://resume/{id}',
    'Resume with sections and entries. Re-read after any mutating tool call.',
    async (uri, params) => {
      const id = params.id as string
      const result = await sdk.resumes.get(id)
      if (!result.ok) {
        return {
          contents: [{
            uri: uri.href,
            mimeType: 'application/json',
            text: JSON.stringify({ error: result.error }),
          }],
        }
      }
      return {
        contents: [{
          uri: uri.href,
          mimeType: 'application/json',
          text: JSON.stringify(result.data, null, 2),
        }],
      }
    },
  )

  server.resource(
    'forge-resume-ir',
    'forge://resume/{id}/ir',
    'Compiled resume intermediate representation (IR) for rendering to Markdown/LaTeX/PDF',
    async (uri, params) => {
      const id = params.id as string
      const result = await sdk.resumes.ir(id)
      if (!result.ok) {
        return {
          contents: [{
            uri: uri.href,
            mimeType: 'application/json',
            text: JSON.stringify({ error: result.error }),
          }],
        }
      }
      return {
        contents: [{
          uri: uri.href,
          mimeType: 'application/json',
          text: JSON.stringify(result.data, null, 2),
        }],
      }
    },
  )

  server.resource(
    'forge-job',
    'forge://job/{id}',
    'Job description with organization details, raw text, and status',
    async (uri, params) => {
      const id = params.id as string
      const result = await sdk.jobDescriptions.get(id)
      if (!result.ok) {
        return {
          contents: [{
            uri: uri.href,
            mimeType: 'application/json',
            text: JSON.stringify({ error: result.error }),
          }],
        }
      }
      return {
        contents: [{
          uri: uri.href,
          mimeType: 'application/json',
          text: JSON.stringify(result.data, null, 2),
        }],
      }
    },
  )
}
```

**Acceptance criteria:**
- All 7 resources are registered and respond to `resources/read` requests.
- Static resources (profile, archetypes, domains, templates) return JSON content from the corresponding SDK list/get method.
- Parameterized resources (resume, resume IR, job) extract the `id` from the URI template params and call the correct SDK method.
- All resource responses have `mimeType: 'application/json'`.
- When the SDK returns an error (e.g., Forge server down), the resource returns the error as JSON content (not a transport-level error) so the AI client can read the message.

**Failure criteria:**
- Parameterized resources using hardcoded IDs instead of extracting from `params`.
- Throwing exceptions on SDK errors instead of returning error content (crashes the MCP server).
- Missing `mimeType` on content (clients may not parse correctly).
- Registering resource templates with incorrect URI patterns (e.g., `forge://resume/:id` instead of `forge://resume/{id}`).

**Test kind:** Integration -- start Forge server + MCP server, read each resource via MCP client, verify JSON content shape.

**Test fixtures:**

```typescript
// Static resource read
// Request: resources/read { uri: 'forge://profile' }
// Expected response shape:
{
  contents: [{
    uri: 'forge://profile',
    mimeType: 'application/json',
    text: '{ "name": "Adam", "email": "...", ... }',
  }]
}

// Parameterized resource read
// Request: resources/read { uri: 'forge://resume/abc-123' }
// Expected: JSON of ResumeWithEntries for resume abc-123

// Error case (Forge server down)
// Request: resources/read { uri: 'forge://profile' }
// Expected:
{
  contents: [{
    uri: 'forge://profile',
    mimeType: 'application/json',
    text: '{ "error": { "code": "NETWORK_ERROR", "message": "..." } }',
  }]
}
```

---

### T71.5: Register Tier 0 Tool (`forge_health`)

**File:** `packages/mcp/src/tools/tier0.ts`

[CRITICAL] `forge_health` is the first tool the AI client should call at session start. It verifies that the Forge HTTP server is reachable. If it fails, the AI should tell the user to start the server before proceeding.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { registerTool } from '../utils/register-tool'
import { mapResult } from '../utils/error-mapper'

export function registerTier0Tools(server: McpServer, sdk: ForgeClient): void {
  registerTool(
    server,
    'forge_health',
    'Check connectivity to the Forge HTTP server. Call this at session start to verify the server is running.',
    {},  // no parameters
    async () => {
      const result = await sdk.health()
      return mapResult(result)
    },
  )
}
```

[IMPORTANT] The preferred approach is to add a `health()` method to `ForgeClient`:

```typescript
// In ForgeClient class:
async health(): Promise<Result<{ server: string; version: string }>> {
  return this.request<{ server: string; version: string }>('GET', '/api/health')
}

// Then in tier0.ts:
const result = await sdk.health()
```

**Acceptance criteria:**
- `forge_health` with no parameters calls `GET /api/health` on the Forge server.
- When Forge is running: returns `{ content: [{ type: 'text', text: '{ "server": "ok", "version": "..." }' }] }`.
- When Forge is down: returns `{ content: [{ type: 'text', text: 'Cannot reach Forge server -- is it running? ...' }], isError: true }`.

**Failure criteria:**
- Returning a generic error message that doesn't mention starting the Forge server.
- Tool requires parameters (spec says none).

**Test kind:** Integration -- start/stop Forge server, call `forge_health`, verify both paths.

**Test fixtures:**

```typescript
// MCP request
{ method: 'tools/call', params: { name: 'forge_health', arguments: {} } }

// Success response (Forge running)
{
  content: [{ type: 'text', text: '{\n  "server": "ok",\n  "version": "1.0.0"\n}' }]
}

// Error response (Forge not running)
{
  content: [{ type: 'text', text: 'Cannot reach Forge server -- is it running? Start with: bun run packages/core/src/index.ts\nDetails: Connection refused' }],
  isError: true
}
```

---

### T71.6: Register Search Tools (3 tools)

**File:** `packages/mcp/src/tools/search.ts`

[CRITICAL] Search tools are the primary discovery mechanism. Each tool has a Zod schema defining optional filter parameters with proper types and defaults. All three support pagination via `offset` and `limit`.

> **Zod note:** `.default()` makes a field optional implicitly. Do not chain `.default().optional()` -- the `.optional()` is redundant when `.default()` is present.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { z } from 'zod'
import { registerTool } from '../utils/register-tool'
import { mapPaginatedResult } from '../utils/error-mapper'

export function registerSearchTools(server: McpServer, sdk: ForgeClient): void {

  // -- forge_search_sources --

  registerTool(
    server,
    'forge_search_sources',
    'Search experience sources by employer, type, date range, status. Sources with status "deriving" are locked by an in-progress AI derivation -- do not call forge_derive_bullets on them. Poll forge_search_sources with status filter to check if derivation has completed, or use forge_review_pending (available in Phase 72) to check the review queue.',
    {
      source_type: z.enum(['role', 'project', 'education', 'clearance', 'general']).optional()
        .describe('Filter by source type'),
      status: z.enum(['draft', 'approved', 'deriving']).optional()
        .describe('Filter by source status'),
      search: z.string().optional()
        .describe('Full-text search on title + description'),
      offset: z.number().int().min(0).default(0)
        .describe('Pagination offset (default 0)'),
      limit: z.number().int().min(1).max(100).default(20)
        .describe('Results per page (default 20, max 100)'),
    },
    async (params) => {
      const result = await sdk.sources.list(params)
      return mapPaginatedResult(result)
    },
  )

  // -- forge_search_bullets --

  registerTool(
    server,
    'forge_search_bullets',
    'Search bullet inventory by domain, status, source, content. Each bullet includes sources[] and technologies[].',
    {
      domain: z.string().optional()
        .describe('Filter by domain slug (e.g., "infrastructure", "security")'),
      status: z.enum(['draft', 'pending_review', 'approved', 'rejected']).optional()
        .describe('Filter by bullet status'),
      source_id: z.string().uuid().optional()
        .describe('Filter by source ID'),
      search: z.string().optional()
        .describe('Full-text search on bullet content'),
      offset: z.number().int().min(0).default(0)
        .describe('Pagination offset (default 0)'),
      limit: z.number().int().min(1).max(100).default(20)
        .describe('Results per page (default 20, max 100)'),
    },
    async (params) => {
      const result = await sdk.bullets.list(params)
      return mapPaginatedResult(result)
    },
  )

  // -- forge_search_perspectives --

  registerTool(
    server,
    'forge_search_perspectives',
    'Search perspectives by archetype, domain, framing, status. Perspectives are the final resume-ready statements derived from bullets.',
    {
      archetype: z.string().optional()
        .describe('Filter by archetype slug (e.g., "platform-engineer")'),
      domain: z.string().optional()
        .describe('Filter by domain slug'),
      framing: z.enum(['accomplishment', 'responsibility', 'context']).optional()
        .describe('Filter by perspective framing type'),
      status: z.enum(['draft', 'pending_review', 'approved', 'rejected']).optional()
        .describe('Filter by perspective status'),
      search: z.string().optional()
        .describe('Full-text search on perspective content'),
      offset: z.number().int().min(0).default(0)
        .describe('Pagination offset (default 0)'),
      limit: z.number().int().min(1).max(100).default(20)
        .describe('Results per page (default 20, max 100)'),
    },
    async (params) => {
      const result = await sdk.perspectives.list(params)
      return mapPaginatedResult(result)
    },
  )
}
```

**Acceptance criteria:**
- All 3 tools accept their documented parameters with proper Zod validation.
- Missing optional parameters are omitted (not sent as `undefined` strings to the SDK).
- `offset` defaults to 0, `limit` defaults to 20.
- `limit` is clamped to max 100 (prevents AI clients from requesting unbounded result sets).
- Responses include both `data` and `pagination` fields.
- `source_type` only accepts the 5 valid enum values.
- `forge_search_sources` status enum includes `'deriving'` in addition to `'draft'` and `'approved'`.
- `status` values match each entity's status lifecycle (bullets/perspectives have `pending_review` and `rejected`; sources have `draft`, `approved`, and `deriving`).

**Failure criteria:**
- Accepting arbitrary strings for `source_type` or `status` (schema must validate enum values).
- Missing pagination in the response (AI clients need total count to decide whether to paginate).
- `source_id` on `forge_search_bullets` not validated as UUID format.
- `forge_search_sources` status enum missing `'deriving'` value.

**Test kind:** Integration -- seed test data in Forge, call each search tool with various filters, verify returned data matches.

**Test fixtures:**

```typescript
// forge_search_sources with filter
{
  method: 'tools/call',
  params: {
    name: 'forge_search_sources',
    arguments: { source_type: 'role', status: 'approved', limit: 5 }
  }
}
// Expected: { data: Source[], pagination: { total: N, offset: 0, limit: 5 } }

// forge_search_sources with deriving status filter
{
  method: 'tools/call',
  params: {
    name: 'forge_search_sources',
    arguments: { status: 'deriving' }
  }
}
// Expected: { data: Source[], pagination: { ... } }

// forge_search_bullets with domain filter
{
  method: 'tools/call',
  params: {
    name: 'forge_search_bullets',
    arguments: { domain: 'security', status: 'approved' }
  }
}

// forge_search_perspectives with framing filter
{
  method: 'tools/call',
  params: {
    name: 'forge_search_perspectives',
    arguments: { framing: 'accomplishment', archetype: 'platform-engineer' }
  }
}

// Empty results (valid, not an error)
{
  method: 'tools/call',
  params: {
    name: 'forge_search_sources',
    arguments: { search: 'nonexistent-query-xyz' }
  }
}
// Expected: { data: [], pagination: { total: 0, offset: 0, limit: 20 } }
```

---

### T71.7: Register Get-by-ID Tools (3 tools)

**File:** `packages/mcp/src/tools/get.ts`

[IMPORTANT] Get-by-ID tools return a single entity with full relations. They are used after search to drill into a specific record.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { z } from 'zod'
import { registerTool } from '../utils/register-tool'
import { mapResult } from '../utils/error-mapper'

export function registerGetTools(server: McpServer, sdk: ForgeClient): void {

  // -- forge_get_source --

  registerTool(
    server,
    'forge_get_source',
    'Get a single source by ID with full extension data (role/project/education/clearance fields) and associated bullets.',
    {
      source_id: z.string().uuid()
        .describe('Source UUID'),
    },
    async (params) => {
      const result = await sdk.sources.get(params.source_id)
      return mapResult(result)
    },
  )

  // -- forge_get_bullet --

  registerTool(
    server,
    'forge_get_bullet',
    'Get a single bullet by ID with full relations (sources[], technologies[]).',
    {
      bullet_id: z.string().uuid()
        .describe('Bullet UUID'),
    },
    async (params) => {
      const result = await sdk.bullets.get(params.bullet_id)
      return mapResult(result)
    },
  )

  // -- forge_get_perspective --

  registerTool(
    server,
    'forge_get_perspective',
    'Get a single perspective by ID with full provenance chain (perspective -> bullet -> source).',
    {
      perspective_id: z.string().uuid()
        .describe('Perspective UUID'),
    },
    async (params) => {
      const result = await sdk.perspectives.get(params.perspective_id)
      return mapResult(result)
    },
  )
}
```

**Acceptance criteria:**
- All 3 tools require a single UUID parameter (validated by Zod).
- `forge_get_source` returns `SourceWithBullets` (source + extension fields + bullets array).
- `forge_get_bullet` returns `BulletWithRelations` (bullet + sources array + technologies array).
- `forge_get_perspective` returns `PerspectiveWithChain` (perspective + bullet + source provenance).
- Invalid UUIDs return a Zod validation error (caught by `registerTool` wrapper).
- Nonexistent IDs return `isError: true` with "Entity not found" message (from error mapper).

**Failure criteria:**
- Accepting non-UUID strings (e.g., integers, slugs) for ID parameters.
- Returning partial data (e.g., source without bullets) due to wrong SDK method.

**Test kind:** Integration -- create test entities, fetch by ID, verify full relation data.

**Test fixtures:**

```typescript
// forge_get_source -- success
{
  method: 'tools/call',
  params: { name: 'forge_get_source', arguments: { source_id: '550e8400-e29b-41d4-a716-446655440000' } }
}
// Expected: SourceWithBullets JSON

// forge_get_bullet -- nonexistent entity
{
  method: 'tools/call',
  params: { name: 'forge_get_bullet', arguments: { bullet_id: '00000000-0000-0000-0000-000000000000' } }
}
// Expected: { content: [{ type: 'text', text: 'Entity not found: ...' }], isError: true }

// Invalid UUID format
{
  method: 'tools/call',
  params: { name: 'forge_get_source', arguments: { source_id: 'not-a-uuid' } }
}
// Expected: Zod validation error wrapped by registerTool
```

---

### T71.7b: Register List Tools (1 tool)

**File:** `packages/mcp/src/tools/list.ts`

[IMPORTANT] `forge_list_resumes` is a Tier 1 tool that lists all resumes with pagination. This is essential for cold-start agents that need to discover existing resumes without knowing IDs.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { z } from 'zod'
import { registerTool } from '../utils/register-tool'
import { mapPaginatedResult } from '../utils/error-mapper'

export function registerListTools(server: McpServer, sdk: ForgeClient): void {

  // -- forge_list_resumes --

  registerTool(
    server,
    'forge_list_resumes',
    'List all resumes with pagination. Returns resume summaries (id, name, target_role, target_employer, archetype, created_at, updated_at). Use forge://resume/{id} resource to get full resume with sections and entries.',
    {
      offset: z.number().int().min(0).default(0)
        .describe('Pagination offset (default 0)'),
      limit: z.number().int().min(1).max(100).default(20)
        .describe('Results per page (default 20, max 100)'),
    },
    async (params) => {
      const result = await sdk.resumes.list(params)
      return mapPaginatedResult(result)
    },
  )
}
```

**Acceptance criteria:**
- `forge_list_resumes` supports `offset` and `limit` parameters with defaults.
- Returns paginated result with `data` (resume summaries) and `pagination` fields.
- Works with zero resumes (returns empty `data` array, not an error).

**Failure criteria:**
- Missing pagination in response.
- Returning full resume data (with sections/entries) instead of summaries.

**Test kind:** Integration -- list resumes, verify pagination shape.

**Test fixtures:**

```typescript
// forge_list_resumes -- default pagination
{
  method: 'tools/call',
  params: { name: 'forge_list_resumes', arguments: {} }
}
// Expected: { data: Resume[], pagination: { total: N, offset: 0, limit: 20 } }

// forge_list_resumes -- custom pagination
{
  method: 'tools/call',
  params: { name: 'forge_list_resumes', arguments: { offset: 10, limit: 5 } }
}
// Expected: { data: Resume[], pagination: { total: N, offset: 10, limit: 5 } }
```

---

### T71.8: Register Derivation Tools (2 tools)

**File:** `packages/mcp/src/tools/derive.ts`

[CRITICAL] Derivation tools trigger AI processing on the Forge server. The MCP client is NOT doing the reasoning -- Forge's AI module handles bullet/perspective derivation. These are the most latency-sensitive tools because they invoke Claude CLI on the server side. This call may take up to 60 seconds as it invokes the AI module.

[IMPORTANT] `forge_derive_perspective` requires `framing` as a REQUIRED parameter. This is the only Tier 1 tool where a field that might seem optional is actually required by the spec.

> **SDK method naming note:** The SDK method name is `derivePerspectives` (plural) but returns a single Perspective per call. The MCP tool name uses singular (`forge_derive_perspective`) to match the spec.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { z } from 'zod'
import { registerTool } from '../utils/register-tool'
import { mapResult } from '../utils/error-mapper'

export function registerDeriveTools(server: McpServer, sdk: ForgeClient): void {

  // -- forge_derive_bullets --

  registerTool(
    server,
    'forge_derive_bullets',
    'Trigger AI bullet derivation from a source. Returns generated bullets in pending_review status. The source must be in "approved" or "draft" status. If the source is "deriving", returns a CONFLICT error -- wait and retry. This calls Forge\'s AI module (not the MCP client). This call may take up to 60 seconds as it invokes the AI module.',
    {
      source_id: z.string().uuid()
        .describe('Source UUID to derive bullets from'),
    },
    async (params) => {
      const result = await sdk.sources.deriveBullets(params.source_id)
      return mapResult(result)
    },
  )

  // -- forge_derive_perspective --

  registerTool(
    server,
    'forge_derive_perspective',
    'Trigger AI perspective derivation from an approved bullet. The bullet must be in "approved" status. Only approved bullets can derive perspectives. Returns a single perspective in pending_review status. This call may take up to 60 seconds as it invokes the AI module.',
    {
      bullet_id: z.string().uuid()
        .describe('Bullet UUID to derive perspective from (must be approved)'),
      archetype: z.string()
        .describe('Target archetype slug (e.g., "platform-engineer")'),
      domain: z.string()
        .describe('Target domain slug (e.g., "infrastructure")'),
      framing: z.enum(['accomplishment', 'responsibility', 'context'])
        .describe('Perspective framing: accomplishment (what you achieved), responsibility (what you owned), or context (background/setup)'),
    },
    async (params) => {
      const result = await sdk.bullets.derivePerspectives(params.bullet_id, {
        archetype: params.archetype,
        domain: params.domain,
        framing: params.framing,
      })
      return mapResult(result)
    },
  )
}
```

**Acceptance criteria:**
- `forge_derive_bullets` requires `source_id` (UUID). Calls `sdk.sources.deriveBullets(source_id)`. Returns `Bullet[]`.
- `forge_derive_perspective` requires all 4 params: `bullet_id` (UUID), `archetype` (string), `domain` (string), `framing` (enum). Calls `sdk.bullets.derivePerspectives(bullet_id, {archetype, domain, framing})`. Returns `Perspective`.
- `framing` is validated as one of the 3 enum values.
- CONFLICT error (source is `deriving`) produces a clear message via the error mapper.
- AI_ERROR or GATEWAY_TIMEOUT from the server-side AI call is mapped to human-readable messages.

**Failure criteria:**
- Making `framing` optional on `forge_derive_perspective` (spec says REQUIRED).
- Making `archetype` or `domain` optional (spec says REQUIRED).
- Returning raw AI error output instead of mapped error messages.
- Setting a timeout that's too short for AI derivation (the SDK/server handle timeouts).

**Test kind:** Integration -- create and approve a source, derive bullets, approve a bullet, derive perspective. Verify status transitions and return shapes.

**Test fixtures:**

```typescript
// forge_derive_bullets -- success
{
  method: 'tools/call',
  params: { name: 'forge_derive_bullets', arguments: { source_id: '<approved-source-uuid>' } }
}
// Expected: { content: [{ type: 'text', text: '[...Bullet[]]' }] }

// forge_derive_bullets -- CONFLICT (source is deriving)
// Expected: { content: [{ type: 'text', text: 'Conflict: Source is locked for derivation' }], isError: true }

// forge_derive_perspective -- success
{
  method: 'tools/call',
  params: {
    name: 'forge_derive_perspective',
    arguments: {
      bullet_id: '<approved-bullet-uuid>',
      archetype: 'platform-engineer',
      domain: 'infrastructure',
      framing: 'accomplishment',
    }
  }
}
// Expected: { content: [{ type: 'text', text: '{ ...Perspective }' }] }

// forge_derive_perspective -- missing framing
{
  method: 'tools/call',
  params: {
    name: 'forge_derive_perspective',
    arguments: { bullet_id: '<uuid>', archetype: 'pe', domain: 'infra' }
  }
}
// Expected: Zod validation error -- framing is required
```

---

### T71.9: Register Review Tools (4 tools)

**File:** `packages/mcp/src/tools/review.ts`

[IMPORTANT] Review tools transition entities between status states. Approve moves `pending_review` to `approved`. Reject moves `pending_review` to `rejected` and requires a `rejection_reason`. These are the human-in-the-loop gatekeepers in the derivation pipeline.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { z } from 'zod'
import { registerTool } from '../utils/register-tool'
import { mapResult } from '../utils/error-mapper'

export function registerReviewTools(server: McpServer, sdk: ForgeClient): void {

  // -- forge_approve_bullet --

  registerTool(
    server,
    'forge_approve_bullet',
    'Approve a bullet (pending_review -> approved). Only pending_review bullets can be approved.',
    {
      bullet_id: z.string().uuid()
        .describe('Bullet UUID to approve'),
    },
    async (params) => {
      const result = await sdk.bullets.approve(params.bullet_id)
      return mapResult(result)
    },
  )

  // -- forge_reject_bullet --

  registerTool(
    server,
    'forge_reject_bullet',
    'Reject a bullet with a reason (pending_review -> rejected). The rejection reason is stored for audit and improvement.',
    {
      bullet_id: z.string().uuid()
        .describe('Bullet UUID to reject'),
      rejection_reason: z.string().min(1)
        .describe('Why this bullet was rejected (required, non-empty)'),
    },
    async (params) => {
      const result = await sdk.bullets.reject(params.bullet_id, {
        rejection_reason: params.rejection_reason,
      })
      return mapResult(result)
    },
  )

  // -- forge_approve_perspective --

  registerTool(
    server,
    'forge_approve_perspective',
    'Approve a perspective (pending_review -> approved). Only approved perspectives can be added to resumes.',
    {
      perspective_id: z.string().uuid()
        .describe('Perspective UUID to approve'),
    },
    async (params) => {
      const result = await sdk.perspectives.approve(params.perspective_id)
      return mapResult(result)
    },
  )

  // -- forge_reject_perspective --

  registerTool(
    server,
    'forge_reject_perspective',
    'Reject a perspective with a reason (pending_review -> rejected).',
    {
      perspective_id: z.string().uuid()
        .describe('Perspective UUID to reject'),
      rejection_reason: z.string().min(1)
        .describe('Why this perspective was rejected (required, non-empty)'),
    },
    async (params) => {
      const result = await sdk.perspectives.reject(params.perspective_id, {
        rejection_reason: params.rejection_reason,
      })
      return mapResult(result)
    },
  )
}
```

**Acceptance criteria:**
- `forge_approve_bullet` and `forge_approve_perspective` require only the entity UUID.
- `forge_reject_bullet` and `forge_reject_perspective` require the entity UUID AND a non-empty `rejection_reason`.
- Approving a non-`pending_review` entity returns a VALIDATION_ERROR or CONFLICT.
- Rejecting with an empty string fails Zod validation (`.min(1)`).
- All 4 tools return the updated entity (Bullet or Perspective) on success.

**Failure criteria:**
- Making `rejection_reason` optional on reject tools (spec says required).
- Allowing empty string rejection reasons (provides no audit value).
- Not returning the updated entity (AI client needs to see the new status).

**Test kind:** Integration -- create a source, derive bullets (creates `pending_review` bullets), approve one, reject another with reason. Verify status transitions.

**Test fixtures:**

```typescript
// forge_approve_bullet
{
  method: 'tools/call',
  params: { name: 'forge_approve_bullet', arguments: { bullet_id: '<pending-bullet-uuid>' } }
}
// Expected: Bullet with status 'approved'

// forge_reject_bullet
{
  method: 'tools/call',
  params: {
    name: 'forge_reject_bullet',
    arguments: {
      bullet_id: '<pending-bullet-uuid>',
      rejection_reason: 'Too vague -- needs specific metrics'
    }
  }
}
// Expected: Bullet with status 'rejected'

// forge_reject_bullet -- empty reason
{
  method: 'tools/call',
  params: {
    name: 'forge_reject_bullet',
    arguments: { bullet_id: '<uuid>', rejection_reason: '' }
  }
}
// Expected: Zod validation error -- min length 1
```

---

### T71.10: Register Resume Assembly Tools (3 tools)

**File:** `packages/mcp/src/tools/assembly.ts`

[CRITICAL] These tools compose the final resume. `forge_create_resume` has template branching logic: if `template_id` is provided, create via `sdk.templates.createResumeFromTemplate()`. Otherwise, create a blank resume via `sdk.resumes.create()`.

[IMPORTANT] `forge_add_resume_entry` requires `section_id` which is a UUID FK to `resume_sections`. The AI client must first get the resume (via `forge://resume/{id}` resource) to find the section IDs, then add entries to specific sections.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { z } from 'zod'
import { registerTool } from '../utils/register-tool'
import { mapResult } from '../utils/error-mapper'

export function registerAssemblyTools(server: McpServer, sdk: ForgeClient): void {

  // -- forge_create_resume --

  registerTool(
    server,
    'forge_create_resume',
    'Create a new resume, optionally from a template. If template_id is provided, the resume is created with predefined sections from the template. Otherwise, a blank resume is created and sections must be added manually with forge_create_resume_section.',
    {
      name: z.string().min(1)
        .describe('Resume name (e.g., "Platform Engineer - Acme Corp")'),
      target_role: z.string().min(1)
        .describe('Target job role (e.g., "Senior Platform Engineer")'),
      target_employer: z.string().min(1)
        .describe('Target employer name'),
      archetype: z.string().min(1).regex(/^[a-z0-9-]+$/)
        .describe('Archetype slug (e.g., "platform-engineer"). Must be lowercase alphanumeric with hyphens. Full validation happens server-side. Use forge://archetypes resource to see available archetypes.'),
      template_id: z.string().uuid().optional()
        .describe('Optional template UUID. Use forge://templates resource to see available templates.'),
    },
    async (params) => {
      // Template branching: use the appropriate SDK call path
      if (params.template_id) {
        const result = await sdk.templates.createResumeFromTemplate({
          template_id: params.template_id,
          name: params.name,
          target_role: params.target_role,
          target_employer: params.target_employer,
          archetype: params.archetype,
        })
        return mapResult(result)
      } else {
        const result = await sdk.resumes.create({
          name: params.name,
          target_role: params.target_role,
          target_employer: params.target_employer,
          archetype: params.archetype,
        })
        return mapResult(result)
      }
    },
  )

  // -- forge_add_resume_entry --

  registerTool(
    server,
    'forge_add_resume_entry',
    'Add an approved perspective to a resume section. Only approved perspectives can be added. Get section IDs from the forge://resume/{id} resource. After adding, re-read the resume resource to see the updated state.',
    {
      resume_id: z.string().uuid()
        .describe('Resume UUID'),
      section_id: z.string().uuid()
        .describe('Section UUID (FK to resume_sections). Get from forge://resume/{id} resource.'),
      perspective_id: z.string().uuid()
        .describe('Perspective UUID (must have status "approved")'),
      position: z.number().int().min(0).optional()
        .describe('Position within the section (0-indexed). Omit to append at end.'),
    },
    async (params) => {
      const result = await sdk.resumes.addEntry(params.resume_id, {
        section_id: params.section_id,
        perspective_id: params.perspective_id,
        position: params.position,
      })
      return mapResult(result)
    },
  )

  // -- forge_create_resume_section --

  registerTool(
    server,
    'forge_create_resume_section',
    'Create a section in a resume (e.g., "Professional Experience", "Technical Skills"). Each section has an entry_type that determines what kind of content it holds.',
    {
      resume_id: z.string().uuid()
        .describe('Resume UUID'),
      title: z.string().min(1)
        .describe('Section title (e.g., "Professional Experience", "Technical Skills", "Education")'),
      entry_type: z.enum([
        'experience',
        'skills',
        'education',
        'projects',
        'certifications',
        'clearance',
        'presentations',
        'awards',
        'freeform',
      ])
        .describe('Section content type. "freeform" is for custom sections that do not fit standard categories.'),
      position: z.number().int().min(0).optional()
        .describe('Position in section ordering (0-indexed). Omit to append at end.'),
    },
    async (params) => {
      const result = await sdk.resumes.createSection(params.resume_id, {
        title: params.title,
        entry_type: params.entry_type,
        position: params.position,
      })
      return mapResult(result)
    },
  )
}
```

**Acceptance criteria:**
- `forge_create_resume` with `template_id` calls `sdk.templates.createResumeFromTemplate()`. Without `template_id` calls `sdk.resumes.create()`.
- `forge_create_resume` requires `name`, `target_role`, `target_employer`, `archetype` (all non-empty strings). `archetype` is validated with `/^[a-z0-9-]+$/` regex. `template_id` is optional UUID.
- `forge_add_resume_entry` requires `resume_id`, `section_id`, `perspective_id` (all UUIDs). `position` is optional integer.
- `forge_add_resume_entry` with a non-approved perspective returns VALIDATION_ERROR from the SDK (not from the MCP layer -- the MCP tool passes through to SDK which validates).
- `forge_create_resume_section` validates `entry_type` against the 9 valid enum values.
- All 3 tools return the created entity on success.

**Failure criteria:**
- Using a single SDK call path instead of branching on `template_id`.
- Missing `archetype` as required on `forge_create_resume` (it's required per spec).
- Accepting arbitrary strings for `entry_type` (must be one of the 9 enum values).
- Not mentioning the `forge://resume/{id}` resource in the `forge_add_resume_entry` description (AI clients need this hint to find section IDs).
- Accepting uppercase or special characters in `archetype` (regex validates slug format).

**Test kind:** Integration -- create a resume with and without template, create sections, add entries (with approved perspectives). Verify returned shapes.

**Test fixtures:**

```typescript
// forge_create_resume -- with template
{
  method: 'tools/call',
  params: {
    name: 'forge_create_resume',
    arguments: {
      name: 'PE Resume - Acme',
      target_role: 'Senior Platform Engineer',
      target_employer: 'Acme Corp',
      archetype: 'platform-engineer',
      template_id: '<template-uuid>',
    }
  }
}
// Expected: Resume with pre-populated sections from template

// forge_create_resume -- without template
{
  method: 'tools/call',
  params: {
    name: 'forge_create_resume',
    arguments: {
      name: 'SE Resume - BigCo',
      target_role: 'Security Engineer',
      target_employer: 'BigCo',
      archetype: 'security-engineer',
    }
  }
}
// Expected: Blank Resume (no sections)

// forge_create_resume_section
{
  method: 'tools/call',
  params: {
    name: 'forge_create_resume_section',
    arguments: {
      resume_id: '<resume-uuid>',
      title: 'Professional Experience',
      entry_type: 'experience',
      position: 0,
    }
  }
}
// Expected: ResumeSectionEntity

// forge_add_resume_entry
{
  method: 'tools/call',
  params: {
    name: 'forge_add_resume_entry',
    arguments: {
      resume_id: '<resume-uuid>',
      section_id: '<section-uuid>',
      perspective_id: '<approved-perspective-uuid>',
    }
  }
}
// Expected: ResumeEntry

// forge_add_resume_entry -- non-approved perspective
{
  method: 'tools/call',
  params: {
    name: 'forge_add_resume_entry',
    arguments: {
      resume_id: '<uuid>',
      section_id: '<uuid>',
      perspective_id: '<draft-perspective-uuid>',
    }
  }
}
// Expected: { ..., isError: true } with VALIDATION_ERROR
```

---

### T71.11: Register Analysis Tools (3 tools)

**File:** `packages/mcp/src/tools/analysis.ts`

[CRITICAL] Analysis tools provide the intelligence layer -- 3 analysis tools (gap_analysis, align_resume, match_requirements). `forge_gap_analysis` is available now (calls `sdk.resumes.gaps()`). `forge_align_resume` and `forge_match_requirements` depend on Phase 70 (alignment API in SDK) and require the embedding service to have vectors computed.

[IMPORTANT] If the alignment SDK resource (`sdk.alignment`) is not available (Phase 70 not yet landed), the alignment tools should return a clear error message rather than crashing. Check for the existence of `sdk.alignment` at call time using typed access.

> **Zod note:** `.default()` makes a field optional implicitly. Do not chain `.default().optional()` -- the `.optional()` is redundant when `.default()` is present.

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { z } from 'zod'
import { registerTool } from '../utils/register-tool'
import { mapResult } from '../utils/error-mapper'

export function registerAnalysisTools(server: McpServer, sdk: ForgeClient): void {

  // -- forge_gap_analysis --

  registerTool(
    server,
    'forge_gap_analysis',
    'Get domain coverage gaps for a resume vs. its archetype\'s expected domains. Returns covered, missing, and thin domains with entry counts. A domain is "thin" if it has fewer than 2 approved perspectives. Use this to identify what content is needed before export.',
    {
      resume_id: z.string().uuid()
        .describe('Resume UUID to analyze'),
    },
    async (params) => {
      const result = await sdk.resumes.gaps(params.resume_id)
      return mapResult(result)
    },
  )

  // -- forge_align_resume --

  registerTool(
    server,
    'forge_align_resume',
    'Programmatic JD-to-Resume alignment using embedding similarity. Returns an AlignmentReport with per-requirement match scores, coverage summary, and unmatched entries. Requires the embedding service to have vectors for both the JD requirements and the resume\'s perspective entries.',
    {
      job_description_id: z.string().uuid()
        .describe('Job description UUID'),
      resume_id: z.string().uuid()
        .describe('Resume UUID'),
      strong_threshold: z.number().min(0).max(1).default(0.75)
        .describe('Similarity threshold for "strong" match (default 0.75)'),
      adjacent_threshold: z.number().min(0).max(1).default(0.50)
        .describe('Similarity threshold for "adjacent" match (default 0.50). Between this and strong = adjacent; below = gap.'),
    },
    async (params) => {
      // Guard: alignment resource may not exist if Phase 70 is not landed
      // TODO: Remove guard once Phase 70 lands AlignmentResource on ForgeClient
      if (!sdk.alignment) {
        return {
          content: [{
            type: 'text' as const,
            text: 'Alignment API not available -- Phase 70 (alignment API in SDK) must be implemented first. forge_gap_analysis is available as an alternative for domain-level coverage analysis.',
          }],
          isError: true,
        }
      }
      const result = await sdk.alignment.score(
        params.job_description_id,
        params.resume_id,
        {
          strong_threshold: params.strong_threshold,
          adjacent_threshold: params.adjacent_threshold,
        },
      )
      return mapResult(result)
    },
  )

  // -- forge_match_requirements --

  registerTool(
    server,
    'forge_match_requirements',
    'Match JD requirements against the full bullet or perspective inventory, independent of any resume. Use this BEFORE creating a resume to discover which approved content best matches a JD. Returns per-requirement candidate matches ranked by embedding similarity.',
    {
      job_description_id: z.string().uuid()
        .describe('Job description UUID'),
      entity_type: z.enum(['bullet', 'perspective'])
        .describe('Search bullets or perspectives'),
      threshold: z.number().min(0).max(1).default(0.50)
        .describe('Minimum similarity threshold (default 0.50)'),
      limit: z.number().int().min(1).max(50).default(10)
        .describe('Max matches per requirement (default 10)'),
    },
    async (params) => {
      // Guard: alignment resource may not exist if Phase 70 is not landed
      // TODO: Remove guard once Phase 70 lands AlignmentResource on ForgeClient
      if (!sdk.alignment) {
        return {
          content: [{
            type: 'text' as const,
            text: 'Alignment API not available -- Phase 70 (alignment API in SDK) must be implemented first.',
          }],
          isError: true,
        }
      }
      const result = await sdk.alignment.matchRequirements(
        params.job_description_id,
        params.entity_type,
        {
          threshold: params.threshold,
          limit: params.limit,
        },
      )
      return mapResult(result)
    },
  )
}
```

[IMPORTANT] The `ForgeClient` type must declare `alignment` as an optional property to avoid `as any` casts:

```typescript
// In @forge/sdk ForgeClient class definition:
alignment?: AlignmentResource
```

This makes `sdk.alignment` a typed optional property rather than requiring `(sdk as any).alignment`. The type guard `if (!sdk.alignment)` narrows correctly after this change.

**Acceptance criteria:**
- `forge_gap_analysis` requires `resume_id` (UUID). Returns `GapAnalysis` with `covered_domains`, `missing_domains`, `thin_domains`, `entry_count_by_domain`.
- `forge_align_resume` requires `job_description_id` and `resume_id`. `strong_threshold` defaults to 0.75, `adjacent_threshold` defaults to 0.50.
- `forge_match_requirements` requires `job_description_id` and `entity_type` (enum: `bullet` | `perspective`). `threshold` defaults to 0.50, `limit` defaults to 10.
- If `sdk.alignment` does not exist, `forge_align_resume` and `forge_match_requirements` return `isError: true` with a message pointing to Phase 70 as a prerequisite.
- Thresholds are validated in range [0, 1].
- No `as any` casts are used -- `sdk.alignment` is typed as optional `AlignmentResource`.

**Failure criteria:**
- Crashing when `sdk.alignment` is undefined (Phase 70 not landed).
- Not providing default values for thresholds (AI clients would need to know the magic numbers).
- `forge_gap_analysis` being blocked on Phase 70 (it uses `sdk.resumes.gaps()` which already exists).
- Using `(sdk as any).alignment` instead of typed access.

**Test kind:** Integration for `forge_gap_analysis` (works now). Smoke test for alignment tools (verify they return the "not available" message when Phase 70 is missing, and work correctly when it's present).

**Test fixtures:**

```typescript
// forge_gap_analysis
{
  method: 'tools/call',
  params: { name: 'forge_gap_analysis', arguments: { resume_id: '<resume-uuid>' } }
}
// Expected:
{
  content: [{
    type: 'text',
    text: JSON.stringify({
      covered_domains: ['infrastructure', 'security'],
      missing_domains: ['data_systems'],
      thin_domains: ['cloud'],
      entry_count_by_domain: { infrastructure: 4, security: 3, cloud: 1, data_systems: 0 }
    }, null, 2)
  }]
}

// forge_align_resume -- Phase 70 not available
{
  method: 'tools/call',
  params: {
    name: 'forge_align_resume',
    arguments: { job_description_id: '<uuid>', resume_id: '<uuid>' }
  }
}
// Expected: { content: [{ type: 'text', text: 'Alignment API not available ...' }], isError: true }

// forge_match_requirements
{
  method: 'tools/call',
  params: {
    name: 'forge_match_requirements',
    arguments: {
      job_description_id: '<uuid>',
      entity_type: 'perspective',
      threshold: 0.60,
      limit: 5,
    }
  }
}
// Expected: RequirementMatchReport JSON (when Phase 70 is available)
```

---

### T71.12: Register Export Tool (1 tool)

**File:** `packages/mcp/src/tools/export.ts`

[CRITICAL] The export tool has format-dependent branching. JSON and markdown/LaTeX return text content directly. PDF writes a binary file to a temp directory and returns the file path for the user to open.

> **Bun compatibility note:** `Blob.text()` is supported in Bun -- verified compatible.

> **Enhancement (future):** Include slugified resume name in temp filename for better UX (e.g., `forge-resume-pe-acme-{id}-{ts}.pdf`).

```typescript
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import { z } from 'zod'
import { registerTool } from '../utils/register-tool'
import { mapResult } from '../utils/error-mapper'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { writeFile } from 'node:fs/promises'

export function registerExportTools(server: McpServer, sdk: ForgeClient): void {

  registerTool(
    server,
    'forge_export_resume',
    'Export a resume in the specified format. JSON returns the full IR document. Markdown and LaTeX return text content. PDF writes to a temp file and returns the file path.',
    {
      resume_id: z.string().uuid()
        .describe('Resume UUID to export'),
      format: z.enum(['json', 'markdown', 'latex', 'pdf'])
        .describe('Export format'),
    },
    async (params) => {
      const { resume_id, format } = params

      // -- JSON: return IR document --
      if (format === 'json') {
        const result = await sdk.export.resumeAsJson(resume_id)
        return mapResult(result)
      }

      // -- Markdown / LaTeX: return text content --
      if (format === 'markdown' || format === 'latex') {
        const result = await sdk.export.downloadResume(resume_id, format)
        if (!result.ok) {
          return mapResult(result)
        }
        const text = await result.data.text()
        return {
          content: [{ type: 'text' as const, text }],
        }
      }

      // -- PDF: write to temp file, return path --
      if (format === 'pdf') {
        const result = await sdk.export.downloadResume(resume_id, 'pdf')
        if (!result.ok) {
          return mapResult(result)
        }
        const buffer = await result.data.arrayBuffer()
        const filename = `forge-resume-${resume_id}-${Date.now()}.pdf`
        const filePath = join(tmpdir(), filename)
        await writeFile(filePath, Buffer.from(buffer))
        return {
          content: [{
            type: 'text' as const,
            text: JSON.stringify({ file_path: filePath, format: 'pdf', size_bytes: buffer.byteLength }, null, 2),
          }],
        }
      }

      // Unreachable (Zod validates format), but TypeScript needs it
      return {
        content: [{ type: 'text' as const, text: `Unsupported format: ${format}` }],
        isError: true,
      }
    },
  )
}
```

**Acceptance criteria:**
- `format: 'json'` calls `sdk.export.resumeAsJson(resume_id)` and returns the ResumeDocument IR as JSON text.
- `format: 'markdown'` calls `sdk.export.downloadResume(resume_id, 'markdown')`, extracts text from the Blob, returns it as MCP text content.
- `format: 'latex'` calls `sdk.export.downloadResume(resume_id, 'latex')`, extracts text from the Blob, returns it as MCP text content.
- `format: 'pdf'` calls `sdk.export.downloadResume(resume_id, 'pdf')`, writes the binary to `$TMPDIR/forge-resume-{id}-{timestamp}.pdf`, returns `{ file_path, format, size_bytes }` as JSON.
- The temp file path is unique per export (includes resume ID + timestamp).
- SDK errors (e.g., resume not found, LaTeX compilation failure) are mapped through `mapResult`.

**Failure criteria:**
- Returning binary PDF data as MCP text content (corrupts the STDIO transport).
- Using a hardcoded temp directory instead of `os.tmpdir()`.
- Not including `size_bytes` in the PDF response (useful diagnostic for the AI client).
- Missing `await` on `writeFile` (file may not be complete when path is returned).

**Test kind:** Integration -- create a resume with content, export in all 4 formats. For PDF, verify the file exists at the returned path and is non-empty.

**Test fixtures:**

```typescript
// forge_export_resume -- JSON
{
  method: 'tools/call',
  params: { name: 'forge_export_resume', arguments: { resume_id: '<uuid>', format: 'json' } }
}
// Expected: ResumeDocument IR as JSON text

// forge_export_resume -- markdown
{
  method: 'tools/call',
  params: { name: 'forge_export_resume', arguments: { resume_id: '<uuid>', format: 'markdown' } }
}
// Expected: { content: [{ type: 'text', text: '# Adam...\n\n## Professional Experience\n...' }] }

// forge_export_resume -- PDF
{
  method: 'tools/call',
  params: { name: 'forge_export_resume', arguments: { resume_id: '<uuid>', format: 'pdf' } }
}
// Expected: { content: [{ type: 'text', text: '{ "file_path": "/tmp/forge-resume-...", "format": "pdf", "size_bytes": 45321 }' }] }
```

---

### T71.13: Create Server Factory and Wire Everything Together

**File:** `packages/mcp/src/server.ts`

[CRITICAL] This is the composition root that creates the MCP server, registers all resources and tools, and returns the server instance. The entry point (`index.ts`) calls this factory.

```typescript
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import type { ForgeClient } from '@forge/sdk'
import pkg from '../package.json'
import { registerResources } from './resources'
import { registerTier0Tools } from './tools/tier0'
import { registerSearchTools } from './tools/search'
import { registerGetTools } from './tools/get'
import { registerListTools } from './tools/list'
import { registerDeriveTools } from './tools/derive'
import { registerReviewTools } from './tools/review'
import { registerAssemblyTools } from './tools/assembly'
import { registerAnalysisTools } from './tools/analysis'
import { registerExportTools } from './tools/export'

/**
 * Create and configure the Forge MCP server.
 *
 * Registers all resources and Tier 0 + Tier 1 tools (21 total).
 * Does NOT connect a transport — the caller handles that.
 */
export function createForgeServer(sdk: ForgeClient): McpServer {
  const server = new McpServer({
    name: 'forge',
    version: pkg.version,
  })

  // Startup-time check for alignment API availability
  if (!sdk.alignment) {
    console.error('[forge:mcp] WARNING: sdk.alignment is not available. forge_align_resume and forge_match_requirements will return errors. Phase 70 must be implemented to enable alignment tools.')
  }

  // Resources (7)
  registerResources(server, sdk)

  // Tier 0: Diagnostics (1 tool)
  registerTier0Tools(server, sdk)

  // Tier 1: Core Workflow (20 tools)
  registerSearchTools(server, sdk)    // 3 tools
  registerGetTools(server, sdk)       // 3 tools
  registerListTools(server, sdk)      // 1 tool
  registerDeriveTools(server, sdk)    // 2 tools
  registerReviewTools(server, sdk)    // 4 tools
  registerAssemblyTools(server, sdk)  // 3 tools
  registerAnalysisTools(server, sdk)  // 3 analysis tools (gap_analysis, align_resume, match_requirements)
  registerExportTools(server, sdk)    // 1 tool

  return server
}
```

**Acceptance criteria:**
- `createForgeServer(sdk)` returns a configured `McpServer` with all 7 resources and 21 tools registered.
- The server `name` is `'forge'` and `version` is read from `package.json` (not hardcoded).
- Each registration function is called exactly once (9 registration calls: resources + 8 tool groups).
- The factory does NOT connect a transport (separation of concerns -- entry point handles transport).
- A warning is logged to stderr at construction time if `sdk.alignment` is undefined.

**Failure criteria:**
- Connecting transport inside the factory (prevents testing without STDIO).
- Missing any of the 9 registration calls (resources + 8 tool groups).
- Hardcoding the SDK client inside the factory instead of accepting it as a parameter (prevents testing with mocks).
- Hardcoding version string instead of reading from package.json.

**Test kind:** Unit -- call `createForgeServer(mockSdk)`, verify no errors. Verify tool count via introspection if the MCP SDK supports it.

---

### T71.14: Create Claude Desktop Configuration Example

**File:** `packages/mcp/claude_desktop_config.example.json`

[IMPORTANT] This file shows users how to register the Forge MCP server in Claude Desktop and Claude Code. The configuration uses `bun` as the command runner since the project uses Bun.

```json
{
  "_comment": "Add this to your Claude Desktop config (~/Library/Application Support/Claude/claude_desktop_config.json) or Claude Code config (.claude/settings.json)",
  "_comment_2": "FORGE_API_URL defaults to http://localhost:3000 if not specified",
  "mcpServers": {
    "forge": {
      "command": "bun",
      "args": ["run", "/ABSOLUTE/PATH/TO/packages/mcp/src/index.ts"],
      "env": {
        "FORGE_API_URL": "http://localhost:3000"
      }
    }
  }
}
```

Additionally, for Claude Code (`.claude/settings.json` at project root):

```json
{
  "mcpServers": {
    "forge": {
      "command": "bun",
      "args": ["run", "packages/mcp/src/index.ts"],
      "env": {
        "FORGE_API_URL": "http://localhost:3000"
      }
    }
  }
}
```

**Acceptance criteria:**
- File contains valid JSON with the MCP server configuration.
- Both Claude Desktop and Claude Code config formats are documented.
- `FORGE_API_URL` environment variable is shown with the default value.
- A comment documents that `FORGE_API_URL` defaults to `http://localhost:3000`.
- Path uses `bun run` (not `node` or `npx`).

**Failure criteria:**
- Using relative paths without explaining they need to be absolute for Claude Desktop.
- Missing `env` block (users won't know how to configure the Forge server URL).

**Test kind:** Manual -- copy the config, start the MCP server, verify it connects.

---

### T71.14b: Create MCP Package README

**File:** `packages/mcp/README.md`

[IMPORTANT] This README provides essential documentation for the MCP server package.

**Content should include:**
- What the MCP server is (machine-to-machine interface between AI clients and Forge)
- How to start it (`bun run packages/mcp/src/index.ts`)
- Transports supported (STDIO only for now; SSE/Streamable HTTP planned)
- Environment variables (`FORGE_API_URL` with default, `FORGE_MCP_DEBUG` for verbose logging)
- Prerequisites (Forge HTTP server running, `@forge/sdk` workspace dependency)
- Quick reference of tool tiers and counts

**Acceptance criteria:**
- README exists and covers all listed topics.
- No stale tool counts or version references.

**Test kind:** Manual review.

---

### T71.14c: Add Justfile Recipe (or document)

[IMPORTANT] If the project justfile is still in use, add an `mcp` recipe:

```just
# Start the MCP server on STDIO
mcp:
  bun run packages/mcp/src/index.ts
```

If the justfile has been replaced by the Forge app's own task runner, add a note in the MCP README instead documenting the start command.

**Test kind:** Manual -- run `just mcp` or verify documentation.

---

### T71.15: Unit Tests for Error Mapper

**File:** `packages/mcp/src/__tests__/error-mapper.test.ts`

[IMPORTANT] These tests verify the pure mapping logic without any SDK or server dependencies.

```typescript
import { describe, test, expect } from 'bun:test'
import { mapResult, mapPaginatedResult } from '../utils/error-mapper'

describe('mapResult', () => {
  test('success: returns JSON-serialized data', () => {
    const result = mapResult({ ok: true, data: { id: '1', name: 'test' } })
    expect(result.isError).toBeUndefined()
    expect(result.content).toHaveLength(1)
    expect(result.content[0].type).toBe('text')
    const parsed = JSON.parse(result.content[0].text)
    expect(parsed.id).toBe('1')
  })

  test('NOT_FOUND: returns entity not found message', () => {
    const result = mapResult({
      ok: false,
      error: { code: 'NOT_FOUND', message: 'bullet abc-123' },
    })
    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('Entity not found')
    expect(result.content[0].text).toContain('abc-123')
  })

  test('VALIDATION_ERROR: includes field details', () => {
    const result = mapResult({
      ok: false,
      error: {
        code: 'VALIDATION_ERROR',
        message: 'Invalid input',
        details: { name: 'required' },
      },
    })
    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('Validation failed')
    expect(result.content[0].text).toContain('"name"')
  })

  test('CONFLICT: returns conflict message', () => {
    const result = mapResult({
      ok: false,
      error: { code: 'CONFLICT', message: 'Source is locked for derivation' },
    })
    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('Conflict')
    expect(result.content[0].text).toContain('locked')
  })

  test('AI_ERROR: returns AI-specific message', () => {
    const result = mapResult({
      ok: false,
      error: { code: 'AI_ERROR', message: 'Claude CLI failed' },
    })
    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('AI derivation failed')
  })

  test('GATEWAY_TIMEOUT: returns timeout message', () => {
    const result = mapResult({
      ok: false,
      error: { code: 'GATEWAY_TIMEOUT', message: '30s exceeded' },
    })
    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('timed out')
  })

  test('NETWORK_ERROR: suggests starting Forge server', () => {
    const result = mapResult({
      ok: false,
      error: { code: 'NETWORK_ERROR', message: 'Connection refused' },
    })
    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('is it running')
    expect(result.content[0].text).toContain('bun run')
  })

  test('SERVICE_UNAVAILABLE: mentions embedding service', () => {
    const result = mapResult({
      ok: false,
      error: { code: 'SERVICE_UNAVAILABLE', message: 'Embedding service unreachable' },
    })
    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('Service unavailable')
    expect(result.content[0].text).toContain('embedding service')
  })

  test('unknown error code: returns generic format', () => {
    const result = mapResult({
      ok: false,
      error: { code: 'WEIRD_ERROR', message: 'something broke' },
    })
    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('[WEIRD_ERROR]')
  })
})

describe('mapPaginatedResult', () => {
  test('success: includes data and pagination', () => {
    const result = mapPaginatedResult({
      ok: true,
      data: [{ id: '1' }, { id: '2' }],
      pagination: { total: 50, offset: 0, limit: 20 },
    })
    expect(result.isError).toBeUndefined()
    const parsed = JSON.parse(result.content[0].text)
    expect(parsed.data).toHaveLength(2)
    expect(parsed.pagination.total).toBe(50)
  })

  test('error: maps through formatError', () => {
    const result = mapPaginatedResult({
      ok: false,
      error: { code: 'NETWORK_ERROR', message: 'timeout' },
    })
    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('is it running')
  })
})
```

**Acceptance criteria:**
- All 7 error codes have dedicated test cases (including SERVICE_UNAVAILABLE).
- Unknown error codes are tested (fallback format).
- Both `mapResult` and `mapPaginatedResult` are tested.
- Success paths verify JSON structure.
- Error paths verify `isError: true` and human-readable message content.

**Test kind:** Unit -- `bun test packages/mcp/src/__tests__/error-mapper.test.ts`

---

### T71.16: Integration Tests

**File:** `packages/mcp/src/__tests__/integration.test.ts`

[IMPORTANT] Integration tests start the Forge HTTP server and MCP server, then send MCP tool calls via `StdioClientTransport` to verify the full chain: MCP client -> STDIO -> MCP server -> SDK -> HTTP API -> response.

[CRITICAL] These tests require the Forge server to be running with a test database. Use the existing test infrastructure from `packages/core` to set up a temporary database.

```typescript
import { describe, test, expect, beforeAll, afterAll } from 'bun:test'
import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js'

// Guard: skip test suite if Forge server is not reachable
const FORGE_API_URL = process.env.FORGE_API_URL ?? 'http://localhost:3000'
let forgeReachable = false
try {
  const res = await fetch(`${FORGE_API_URL}/api/health`)
  forgeReachable = res.ok
} catch {
  forgeReachable = false
}

const describeFn = forgeReachable ? describe : describe.skip

describeFn('MCP Integration', () => {
  let client: Client
  let transport: StdioClientTransport

  beforeAll(async () => {
    // Start MCP server as a child process
    transport = new StdioClientTransport({
      command: 'bun',
      args: ['run', 'packages/mcp/src/index.ts'],
      env: { ...process.env, FORGE_API_URL },
    })

    client = new Client({ name: 'test-client', version: '1.0.0' })
    try {
      await client.connect(transport)
    } catch (err) {
      console.error('[test] Failed to connect MCP client:', err)
      throw err
    }
  })

  afterAll(async () => {
    try {
      await client.close()
    } catch {
      // Best-effort cleanup
    }
    // Transport cleanup: kill MCP child process if still running
    try {
      await transport.close()
    } catch {
      // Best-effort cleanup
    }
  })

  test('forge_health returns server status', async () => {
    const result = await client.callTool({ name: 'forge_health', arguments: {} })
    // If Forge is running: should get ok response
    // If not: should get isError with helpful message
    expect(result.content).toBeDefined()
    expect((result.content as any[]).length).toBeGreaterThan(0)
  })

  test('forge_search_sources returns paginated results', async () => {
    const result = await client.callTool({
      name: 'forge_search_sources',
      arguments: { limit: 5 },
    })
    expect(result.isError).toBeFalsy()
    const parsed = JSON.parse((result.content as any)[0].text)
    expect(parsed.pagination).toBeDefined()
    expect(parsed.pagination.limit).toBe(5)
  })

  test('forge_get_source with nonexistent entity returns error', async () => {
    const result = await client.callTool({
      name: 'forge_get_source',
      arguments: { source_id: '00000000-0000-0000-0000-000000000000' },
    })
    expect(result.isError).toBe(true)
    expect((result.content as any)[0].text).toContain('not found')
  })

  test('forge_list_resumes returns paginated results', async () => {
    const result = await client.callTool({
      name: 'forge_list_resumes',
      arguments: { limit: 5 },
    })
    expect(result.isError).toBeFalsy()
    const parsed = JSON.parse((result.content as any)[0].text)
    expect(parsed.pagination).toBeDefined()
  })

  test('resources can be read', async () => {
    const result = await client.readResource({ uri: 'forge://profile' })
    expect(result.contents).toBeDefined()
    expect(result.contents.length).toBeGreaterThan(0)
    expect(result.contents[0].mimeType).toBe('application/json')
  })

  test('forge_export_resume with nonexistent entity returns error', async () => {
    const result = await client.callTool({
      name: 'forge_export_resume',
      arguments: {
        resume_id: '00000000-0000-0000-0000-000000000000',
        format: 'json',
      },
    })
    expect(result.isError).toBe(true)
  })

  test('listTools returns all 21 registered tools', async () => {
    const tools = await client.listTools()
    expect(tools.tools).toHaveLength(21)
  })

  test('listResources returns registered resources', async () => {
    const resources = await client.listResources()
    expect(resources.resources).toBeDefined()
  })

  test('forge://resume/{id}/ir resource with nonexistent ID returns error JSON', async () => {
    const result = await client.readResource({ uri: 'forge://resume/00000000-0000-0000-0000-000000000000/ir' })
    expect(result.contents).toBeDefined()
    expect(result.contents.length).toBeGreaterThan(0)
    const parsed = JSON.parse(result.contents[0].text as string)
    expect(parsed.error).toBeDefined()
  })

  test('MCP server shuts down gracefully on SIGTERM', async () => {
    // Spawn a separate MCP server process for shutdown testing
    const { spawn } = await import('node:child_process')
    const child = spawn('bun', ['run', 'packages/mcp/src/index.ts'], {
      env: { ...process.env, FORGE_API_URL },
      stdio: 'pipe',
    })

    // Give it time to start
    await new Promise(resolve => setTimeout(resolve, 1000))

    // Send SIGTERM
    child.kill('SIGTERM')

    // Wait for exit with timeout
    const exitCode = await new Promise<number | null>((resolve) => {
      const timeout = setTimeout(() => resolve(null), 5000)
      child.on('exit', (code) => {
        clearTimeout(timeout)
        resolve(code)
      })
    })

    expect(exitCode).toBe(0)
  })

  test('MCP server with invalid FORGE_API_URL returns error on resource read', async () => {
    // Spawn MCP server pointing to a bad URL
    const badTransport = new StdioClientTransport({
      command: 'bun',
      args: ['run', 'packages/mcp/src/index.ts'],
      env: { ...process.env, FORGE_API_URL: 'http://localhost:19999' },
    })
    const badClient = new Client({ name: 'test-bad', version: '1.0.0' })

    try {
      await badClient.connect(badTransport)
      const result = await badClient.readResource({ uri: 'forge://profile' })
      expect(result.contents).toBeDefined()
      const parsed = JSON.parse(result.contents[0].text as string)
      expect(parsed.error).toBeDefined()
    } finally {
      try { await badClient.close() } catch {}
      try { await badTransport.close() } catch {}
    }
  })
})
```

**Acceptance criteria:**
- Tests check if FORGE_API_URL is reachable; if not, the suite is skipped with `describe.skip`.
- Tests start the MCP server as a child process via `StdioClientTransport`.
- At least one tool from each category (health, search, get, list, export) is tested.
- Resource reads are tested (including IR resource and error resource).
- Error paths (nonexistent entities) are tested.
- `listTools()` verifies 21 tools are registered.
- `listResources()` smoke test is included.
- Graceful shutdown test: spawn, SIGTERM, assert exit code 0 within timeout.
- Resource error fallback test: invalid FORGE_API_URL, read resource, verify error JSON.
- Tests clean up (close client, transport, and child processes) in afterAll.
- `beforeAll` wraps `client.connect` in try/catch for better error reporting.

**Failure criteria:**
- Tests that depend on specific data existing in the database (use error paths or seed data in beforeAll).
- Tests that leave orphaned child processes on failure.
- Tests that write to stdout (interferes with STDIO transport in parent process).
- Tests that run when Forge is unavailable (should skip).

**Test kind:** Integration -- `bun test packages/mcp/src/__tests__/integration.test.ts` (requires Forge server running).

---

## Verification Checklist

After all tasks are complete, verify:

- [ ] `packages/mcp` is listed in workspace root's `package.json` workspaces array
- [ ] `bun run packages/mcp/src/index.ts` starts without errors
- [ ] `forge_health` returns `{ server: 'ok', version: '...' }` when Forge is running
- [ ] `forge_health` returns `isError: true` with helpful message when Forge is down
- [ ] All 7 resources respond to `resources/read` requests
- [ ] Parameterized resources (`forge://resume/{id}`) resolve IDs correctly
- [ ] All 21 tools (1 Tier 0 + 20 Tier 1) are registered and respond to `tools/call`
- [ ] `listTools()` returns exactly 21 tools
- [ ] Search tools return paginated results with `data` and `pagination`
- [ ] `forge_search_sources` status filter accepts `'deriving'` value
- [ ] `forge_list_resumes` returns paginated resume summaries
- [ ] Get-by-ID tools return full relation data
- [ ] Derivation tools call SDK derivation methods (require running AI module for full test)
- [ ] Review tools transition entity statuses correctly
- [ ] Resume assembly tools create resumes, sections, and entries
- [ ] `forge_create_resume` branches on `template_id` (SDK call vs template call)
- [ ] `forge_create_resume` validates archetype slug format with regex
- [ ] `forge_gap_analysis` returns domain coverage gaps
- [ ] `forge_align_resume` returns "not available" when Phase 70 is missing
- [ ] `forge_align_resume` uses typed `sdk.alignment` (no `as any` casts)
- [ ] `forge_export_resume` handles all 4 formats correctly
- [ ] PDF export writes to temp file and returns path
- [ ] Error mapper produces human-readable messages for all 7 error codes (including SERVICE_UNAVAILABLE)
- [ ] No stdout output (only stderr for logging)
- [ ] Success logging gated behind `FORGE_MCP_DEBUG` env var
- [ ] Server version read from `package.json` (not hardcoded)
- [ ] Startup warning logged to stderr when `sdk.alignment` is unavailable
- [ ] SIGINT/SIGTERM trigger graceful shutdown (server.close() called, process exits 0)
- [ ] Unit tests pass: `bun test packages/mcp/src/__tests__/error-mapper.test.ts`
- [ ] Integration tests pass: `bun test packages/mcp/src/__tests__/integration.test.ts`
- [ ] Integration tests skip gracefully when Forge is not running
- [ ] Claude Desktop config example is valid JSON
- [ ] `packages/mcp/README.md` exists with startup instructions and env var docs
