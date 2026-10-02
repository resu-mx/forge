# M7 Infrastructure Design — Config Migration + Server Logging

**Status**: Approved
**Date**: 2026-04-21
**Phase**: M7 (final MVP phase)
**Version**: 0.1.6 → 1.0.0
**Beads**: 3bp.29 (config migration), 3bp.31 (logging)

## Overview

Migrate extension configuration from `chrome.storage.local` to Forge DB with graceful offline fallback. Add server-side error logging gated by config flag. WebUI pages for config editing and log viewing. Version bump to 1.0.0 marks MVP complete.

## Database — Migration 051

### extension_config

Key-value store. Values JSON-serialized (booleans as `true`/`false`, arrays as `["a","b"]`, strings as `"value"`).

```sql
CREATE TABLE IF NOT EXISTS extension_config (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Seeded with defaults:

| Key | Default Value | Type |
|-----|--------------|------|
| `baseUrl` | `"http://localhost:3000"` | string |
| `devMode` | `false` | boolean |
| `enabledPlugins` | `["linkedin"]` | string[] |
| `enableServerLogging` | `true` | boolean |

### extension_logs

```sql
CREATE TABLE IF NOT EXISTS extension_logs (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6)))),
  error_code TEXT NOT NULL,
  message TEXT NOT NULL,
  layer TEXT NOT NULL,
  plugin TEXT,
  url TEXT,
  context TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

No foreign keys. `context` is a JSON blob (stringified `Record<string, unknown>`). `layer` is one of: `plugin`, `content`, `background`, `popup`, `sdk`.

## Core Services

### ExtensionConfigService

Does NOT use ELM — key-value tables with non-UUID primary keys don't fit the ELM pattern. Uses raw `db` parameter directly.

Hardcoded `DEFAULTS` map in the service (same values as migration seed). `getAll()` reads all rows, merges over defaults, returns typed `ExtensionConfig` object. Missing keys get default values without requiring a migration.

```typescript
class ExtensionConfigService {
  constructor(private db: Database) {}

  getAll(): Result<ExtensionConfig>
  get(key: string): Result<string>
  set(key: string, value: unknown): Result<ExtensionConfig>
  setMany(updates: Record<string, unknown>): Result<ExtensionConfig>
}
```

Type coercion: `getAll()` parses each value with `JSON.parse()`. Booleans stored as `true`/`false`, arrays as JSON arrays, strings as JSON strings (quoted).

### ExtensionLogService

Uses ELM for standard CRUD operations.

```typescript
class ExtensionLogService {
  constructor(private elm: EntityLifecycleManager) {}

  append(entry: CreateExtensionLog): Result<ExtensionLog>
  list(opts?: { limit?: number; offset?: number; error_code?: string; layer?: string }): PaginatedResult<ExtensionLog>
  clear(): Result<{ deleted: number }>
}
```

`list()` defaults to `limit: 50`, ordered by `created_at DESC` (most recent first). `clear()` deletes all rows.

## API Routes

Mounted under `/extension` prefix on the Hono app.

### Config

**GET /extension/config** — Returns full config merged over defaults.
```json
{ "data": { "baseUrl": "http://localhost:3000", "devMode": false, "enabledPlugins": ["linkedin"], "enableServerLogging": true } }
```

**PUT /extension/config** — Update one or more config keys. Body: `{ "updates": { "devMode": true, "enableServerLogging": false } }`. Returns full config after update.
```json
{ "data": { "baseUrl": "http://localhost:3000", "devMode": true, "enabledPlugins": ["linkedin"], "enableServerLogging": false } }
```

Validation: rejects unknown keys (not in DEFAULTS map). Returns 400 with error.

### Logs

**POST /extension/log** — Append a log entry. Body matches `ExtensionError` shape from extension:
```json
{
  "error_code": "API_UNREACHABLE",
  "message": "Failed to fetch",
  "layer": "sdk",
  "plugin": null,
  "url": "https://example.com/jobs/123",
  "context": { "sdk_code": "NETWORK_ERROR" }
}
```
Returns 201.

**GET /extension/logs** — Paginated list. Query params: `limit`, `offset`, `error_code`, `layer`. Returns standard `{ data, pagination }` envelope.

**DELETE /extension/logs** — Clear all logs. Returns 204.

## SDK Resources

### ExtensionConfigResource

```typescript
class ExtensionConfigResource {
  constructor(private request: RequestFn) {}

  get(): Promise<Result<ExtensionConfig>>
  update(updates: Record<string, unknown>): Promise<Result<ExtensionConfig>>
}
```

### ExtensionLogsResource

```typescript
class ExtensionLogsResource {
  constructor(private request: RequestFn, private requestList: RequestListFn) {}

  append(entry: CreateExtensionLog): Promise<Result<void>>
  list(opts?: ExtensionLogFilter): Promise<PaginatedResult<ExtensionLog>>
  clear(): Promise<Result<void>>
}
```

Mounted on `ForgeClient` as `client.extensionConfig` and `client.extensionLogs`.

### SDK Types

```typescript
interface ExtensionConfig {
  baseUrl: string
  devMode: boolean
  enabledPlugins: string[]
  enableServerLogging: boolean
}

interface ExtensionLog {
  id: string
  error_code: string
  message: string
  layer: string
  plugin: string | null
  url: string | null
  context: Record<string, unknown> | null
  created_at: string
}

interface CreateExtensionLog {
  error_code: string
  message: string
  layer: string
  plugin?: string
  url?: string
  context?: Record<string, unknown>
}

interface ExtensionLogFilter {
  limit?: number
  offset?: number
  error_code?: string
  layer?: string
}
```

## Extension Changes

### Config Loading (`src/storage/config.ts`)

Rewrite `loadConfig()`:

1. Try `GET /api/extension/config` via SDK client
2. On success: cache result to `chrome.storage.local`, return typed config
3. On failure (network error): read from `chrome.storage.local` cache, merge over DEFAULTS, return
4. If both fail: return DEFAULTS

`saveConfig()` removed — config is now managed via Forge API/WebUI, not extension-side writes.

The `DEFAULTS` constant stays in the extension as the ultimate fallback. `devMode` default changes from `true` to `false` for production.

### Error Reporting (`src/lib/errors.ts`)

New exported function:

```typescript
export function reportError(err: ExtensionError): void
```

- Reads `enableServerLogging` from cached config (sync read from module-level variable, updated on config load)
- If disabled, no-op
- If enabled, fires `POST /api/extension/log` via the SDK client (fire-and-forget — no await, catch-and-swallow to prevent reporting errors from causing more errors)
- Called from: `mapSdkError()`, `mapNetworkError()`, and the background message handler's default error branch

Module-level `_serverLoggingEnabled: boolean` flag, set by `loadConfig()` on startup. Avoids async config reads on every error.

### Background Client (`src/background/client.ts`)

No structural changes. `getClient()` continues to use `loadConfig()` which now tries API first. The client singleton re-instantiates if baseUrl changes (existing behavior).

## WebUI Pages

Accessed via **profile menu** (matches existing `eeo` and `work-auth` settings pattern).

### /settings/extension — Config Editor

- Fetches config via `forge.extensionConfig.get()`
- Renders each key as an editable row:
  - Boolean keys: toggle switch
  - String keys: text input
  - Array keys: comma-separated tag input
- "Save" button calls `forge.extensionConfig.update(changedKeys)`
- Toast on success/error
- Uses `PageWrapper` + `PageHeader` shared components

### /settings/extension-logs — Log Viewer

- Fetches logs via `forge.extensionLogs.list()` with pagination
- Table columns: Timestamp, Error Code, Message, Layer, Plugin
- Expandable row detail for `context` JSON (formatted)
- Filter dropdowns: error_code (populated from distinct values), layer
- "Clear All" button with confirmation dialog
- Pagination controls at bottom
- Uses `PageWrapper` + `PageHeader` shared components
- Empty state when no logs

### Profile Menu Updates

Add two links to the existing profile menu:
- "Extension Config" → `/settings/extension`
- "Extension Logs" → `/settings/extension-logs`

## Entity Map Registration

`extension_logs` registered in entity-map.data.ts (standard ELM entity). `extension_config` is NOT registered — it uses raw SQL via the service (non-standard primary key).

## Services Index Updates

Add `ExtensionConfigService` (receives `db`) and `ExtensionLogService` (receives `elm`) to the `Services` interface and `createServices()` factory.

## Version Bump

Both `manifest.json` and `manifest.firefox.json`: `"version": "1.0.0"`.

## Not In Scope

- Log retention/pruning (manual clear sufficient for MVP)
- Config change history/audit trail
- Config sync between multiple browsers
- Real-time log streaming (WebSocket)
- Config validation beyond unknown-key rejection
