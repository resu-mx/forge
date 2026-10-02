# M7 Infrastructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate extension config to Forge DB with offline fallback, add server-side error logging, build WebUI config editor + log viewer, and bump version to 1.0.0 (MVP complete).

**Architecture:** Key-value config table in SQLite with JSON-serialized values, fetched by extension on startup with chrome.storage.local as offline cache. Error logs stored via ELM, gated by `enableServerLogging` config flag. Two new WebUI settings pages accessible from the profile menu.

**Tech Stack:** Bun/Hono (API), SQLite (storage), TypeScript SDK, Svelte 5 (WebUI), Chrome Extension MV3

**Spec:** `.claude/plans/forge-resume-browser-extension/refs/specs/2026-04-21-m7-infrastructure-design.md`

---

## File Map

### Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/051_extension_infra.sql` | Migration: extension_config + extension_logs tables |
| `packages/core/src/services/extension-config-service.ts` | Config service (raw SQL, key-value) |
| `packages/core/src/services/extension-log-service.ts` | Log service (ELM-based) |
| `packages/core/src/services/__tests__/extension-config-service.test.ts` | Config service unit tests |
| `packages/core/src/services/__tests__/extension-log-service.test.ts` | Log service unit tests |
| `packages/core/src/routes/extension.ts` | HTTP routes for config + logs |
| `packages/core/src/routes/__tests__/extension.test.ts` | Route integration tests |
| `packages/sdk/src/resources/extension-config.ts` | SDK config resource |
| `packages/sdk/src/resources/extension-logs.ts` | SDK logs resource |
| `packages/webui/src/routes/settings/extension/+page.svelte` | Config editor page |
| `packages/webui/src/routes/settings/extension-logs/+page.svelte` | Log viewer page |

### Modify

| File | Lines/Section | Change |
|------|---------------|--------|
| `packages/core/src/storage/entity-map.data.ts` | ~1017 (after answer_bank) | Add extension_logs entity |
| `packages/core/src/services/index.ts` | Services interface + createServices | Add extensionConfig + extensionLogs |
| `packages/core/src/routes/server.ts` | Route mounts | Add extensionRoutes |
| `packages/sdk/src/types.ts` | Bottom of file | Add ExtensionConfig, ExtensionLog, CreateExtensionLog, ExtensionLogFilter types |
| `packages/sdk/src/client.ts` | Constructor + properties | Add extensionConfig + extensionLogs resources |
| `packages/sdk/src/index.ts` | Exports | Add new types + resource classes |
| `packages/extension/src/storage/config.ts` | Full rewrite | API-first with chrome.storage.local fallback |
| `packages/extension/src/lib/errors.ts` | Add reportError | Fire-and-forget server logging |
| `packages/extension/src/background/client.ts` | Import path | Adapt to new loadConfig |
| `packages/webui/src/lib/components/ProfileMenu.svelte` | Settings section | Add Extension Config + Extension Logs links |
| `packages/extension/manifest.json` | version | 0.1.6 → 1.0.0 |
| `packages/extension/manifest.firefox.json` | version | 0.1.6 → 1.0.0 |

---

## Task 1: Migration — extension_config + extension_logs tables

**Files:**
- Create: `packages/core/src/db/migrations/051_extension_infra.sql`
- Test: `packages/core/src/db/__tests__/schema.test.ts` (existing, auto-validates)

- [ ] **Step 1: Write the migration SQL**

Create `packages/core/src/db/migrations/051_extension_infra.sql`:

```sql
-- Extension infrastructure (M7): config key-value store + error log sink

CREATE TABLE IF NOT EXISTS extension_config (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed defaults
INSERT OR IGNORE INTO extension_config (key, value) VALUES ('baseUrl', '"http://localhost:3000"');
INSERT OR IGNORE INTO extension_config (key, value) VALUES ('devMode', 'false');
INSERT OR IGNORE INTO extension_config (key, value) VALUES ('enabledPlugins', '["linkedin"]');
INSERT OR IGNORE INTO extension_config (key, value) VALUES ('enableServerLogging', 'true');

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

CREATE INDEX IF NOT EXISTS idx_extension_logs_created_at ON extension_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_extension_logs_error_code ON extension_logs(error_code);
```

- [ ] **Step 2: Run migration test to verify schema applies cleanly**

Run: `cd packages/core && bun test src/db/__tests__/schema.test.ts`
Expected: PASS (schema test creates in-memory DB with all migrations)

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/db/migrations/051_extension_infra.sql
git commit -m "feat(core): add migration 051 — extension_config + extension_logs tables (M7)"
```

---

## Task 2: Entity Map — register extension_logs

**Files:**
- Modify: `packages/core/src/storage/entity-map.data.ts`

- [ ] **Step 1: Add extension_logs entity definition**

After the `answer_bank` entry (~line 1017), add:

```typescript
  extension_logs: {
    fields: {
      id: ID_FIELD,
      error_code: { type: 'text', required: true },
      message: { type: 'text', required: true },
      layer: { type: 'text', required: true },
      plugin: { type: 'text' },
      url: { type: 'text' },
      context: { type: 'text' },
      created_at: CREATED_AT,
    },
    cascade: [],
    restrict: [],
    setNull: [],
  },
```

Note: No `updated_at` on extension_logs — logs are append-only. Do NOT add it to the `beforeUpdate` hook loop.

- [ ] **Step 2: Run entity map tests**

Run: `cd packages/core && bun test src/storage/__tests__/entity-map.test.ts`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/storage/entity-map.data.ts
git commit -m "feat(core): register extension_logs in entity map (M7)"
```

---

## Task 3: ExtensionConfigService — raw SQL key-value service

**Files:**
- Create: `packages/core/src/services/extension-config-service.ts`
- Create: `packages/core/src/services/__tests__/extension-config-service.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/core/src/services/__tests__/extension-config-service.test.ts`:

```typescript
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestDb } from '../../db/__tests__/helpers'
import { ExtensionConfigService } from '../extension-config-service'

describe('ExtensionConfigService', () => {
  let db: Database
  let svc: ExtensionConfigService

  beforeEach(() => {
    db = createTestDb()
    svc = new ExtensionConfigService(db)
  })

  afterEach(() => {
    db.close()
  })

  test('getAll() returns seeded defaults', () => {
    const result = svc.getAll()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.baseUrl).toBe('http://localhost:3000')
    expect(result.data.devMode).toBe(false)
    expect(result.data.enabledPlugins).toEqual(['linkedin'])
    expect(result.data.enableServerLogging).toBe(true)
  })

  test('get() returns a single parsed value', () => {
    const result = svc.get('devMode')
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data).toBe(false)
  })

  test('get() returns NOT_FOUND for unknown key', () => {
    const result = svc.get('nonexistent')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('NOT_FOUND')
  })

  test('set() updates a boolean key', () => {
    const result = svc.set('devMode', true)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.devMode).toBe(true)
  })

  test('set() updates a string key', () => {
    const result = svc.set('baseUrl', 'http://192.168.1.100:3000')
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.baseUrl).toBe('http://192.168.1.100:3000')
  })

  test('set() updates an array key', () => {
    const result = svc.set('enabledPlugins', ['linkedin', 'workday'])
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.enabledPlugins).toEqual(['linkedin', 'workday'])
  })

  test('set() rejects unknown keys', () => {
    const result = svc.set('unknownKey', 'value')
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('setMany() updates multiple keys at once', () => {
    const result = svc.setMany({ devMode: true, enableServerLogging: false })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.devMode).toBe(true)
    expect(result.data.enableServerLogging).toBe(false)
    // Unchanged keys stay default
    expect(result.data.baseUrl).toBe('http://localhost:3000')
  })

  test('setMany() rejects if any key is unknown', () => {
    const result = svc.setMany({ devMode: true, badKey: 'x' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })

  test('getAll() merges missing keys over defaults', () => {
    // Delete a seeded row to simulate a key added after initial migration
    db.run("DELETE FROM extension_config WHERE key = 'enableServerLogging'")
    const result = svc.getAll()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    // Should still return the default
    expect(result.data.enableServerLogging).toBe(true)
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/services/__tests__/extension-config-service.test.ts`
Expected: FAIL (module not found)

- [ ] **Step 3: Write the service implementation**

Create `packages/core/src/services/extension-config-service.ts`:

```typescript
/**
 * ExtensionConfigService — key-value config for the browser extension.
 *
 * Uses raw SQL (not ELM) because extension_config uses TEXT primary key
 * `key` instead of UUID `id`, which doesn't fit the ELM pattern.
 */

import type { Database } from 'bun:sqlite'
import type { Result } from '../types'

export interface ExtensionConfig {
  baseUrl: string
  devMode: boolean
  enabledPlugins: string[]
  enableServerLogging: boolean
}

const DEFAULTS: ExtensionConfig = {
  baseUrl: 'http://localhost:3000',
  devMode: false,
  enabledPlugins: ['linkedin'],
  enableServerLogging: true,
}

const VALID_KEYS = new Set(Object.keys(DEFAULTS))

export class ExtensionConfigService {
  constructor(private db: Database) {}

  /** Get all config keys merged over defaults. */
  getAll(): Result<ExtensionConfig> {
    const rows = this.db.query('SELECT key, value FROM extension_config').all() as Array<{ key: string; value: string }>
    const config = { ...DEFAULTS }
    for (const row of rows) {
      if (row.key in config) {
        (config as Record<string, unknown>)[row.key] = JSON.parse(row.value)
      }
    }
    return { ok: true, data: config }
  }

  /** Get a single config value by key. */
  get(key: string): Result<unknown> {
    if (!VALID_KEYS.has(key)) {
      return { ok: false, error: { code: 'NOT_FOUND', message: `Unknown config key: ${key}` } }
    }
    const row = this.db.query('SELECT value FROM extension_config WHERE key = ?').get(key) as { value: string } | null
    if (!row) {
      // Return the default
      return { ok: true, data: (DEFAULTS as Record<string, unknown>)[key] }
    }
    return { ok: true, data: JSON.parse(row.value) }
  }

  /** Set a single config key. Returns full config after update. */
  set(key: string, value: unknown): Result<ExtensionConfig> {
    if (!VALID_KEYS.has(key)) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Unknown config key: ${key}` } }
    }
    this.db.run(
      `INSERT INTO extension_config (key, value, updated_at) VALUES (?, ?, datetime('now'))
       ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at`,
      [key, JSON.stringify(value)],
    )
    return this.getAll()
  }

  /** Set multiple config keys at once. Returns full config after update. */
  setMany(updates: Record<string, unknown>): Result<ExtensionConfig> {
    for (const key of Object.keys(updates)) {
      if (!VALID_KEYS.has(key)) {
        return { ok: false, error: { code: 'VALIDATION_ERROR', message: `Unknown config key: ${key}` } }
      }
    }
    const stmt = this.db.prepare(
      `INSERT INTO extension_config (key, value, updated_at) VALUES (?, ?, datetime('now'))
       ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at`,
    )
    for (const [key, value] of Object.entries(updates)) {
      stmt.run(key, JSON.stringify(value))
    }
    return this.getAll()
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/core && bun test src/services/__tests__/extension-config-service.test.ts`
Expected: PASS (all 9 tests)

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/services/extension-config-service.ts packages/core/src/services/__tests__/extension-config-service.test.ts
git commit -m "feat(core): add ExtensionConfigService with key-value config (M7)"
```

---

## Task 4: ExtensionLogService — ELM-based log service

**Files:**
- Create: `packages/core/src/services/extension-log-service.ts`
- Create: `packages/core/src/services/__tests__/extension-log-service.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/core/src/services/__tests__/extension-log-service.test.ts`:

```typescript
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestDb } from '../../db/__tests__/helpers'
import { buildDefaultElm } from '../../storage/build-elm'
import { ExtensionLogService } from '../extension-log-service'

describe('ExtensionLogService', () => {
  let db: Database
  let svc: ExtensionLogService

  beforeEach(() => {
    db = createTestDb()
    const elm = buildDefaultElm(db)
    svc = new ExtensionLogService(elm)
  })

  afterEach(() => {
    db.close()
  })

  test('list() returns empty array initially', async () => {
    const result = await svc.list()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data).toEqual([])
  })

  test('append() creates a log entry', async () => {
    const result = await svc.append({
      error_code: 'API_UNREACHABLE',
      message: 'Failed to fetch',
      layer: 'sdk',
    })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.error_code).toBe('API_UNREACHABLE')
    expect(result.data.message).toBe('Failed to fetch')
    expect(result.data.layer).toBe('sdk')
    expect(result.data.id).toBeDefined()
    expect(result.data.created_at).toBeDefined()
  })

  test('append() stores optional fields', async () => {
    const result = await svc.append({
      error_code: 'PLUGIN_THREW',
      message: 'LinkedIn extractor failed',
      layer: 'plugin',
      plugin: 'linkedin',
      url: 'https://linkedin.com/jobs/123',
      context: { selector: '.jobs-description' },
    })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data.plugin).toBe('linkedin')
    expect(result.data.url).toBe('https://linkedin.com/jobs/123')
    expect(result.data.context).toEqual({ selector: '.jobs-description' })
  })

  test('list() returns entries in descending order', async () => {
    await svc.append({ error_code: 'A', message: 'first', layer: 'sdk' })
    await svc.append({ error_code: 'B', message: 'second', layer: 'sdk' })
    await svc.append({ error_code: 'C', message: 'third', layer: 'sdk' })

    const result = await svc.list()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    // Most recent first
    expect(result.data[0].error_code).toBe('C')
    expect(result.data[2].error_code).toBe('A')
  })

  test('list() supports limit and offset', async () => {
    for (let i = 0; i < 5; i++) {
      await svc.append({ error_code: `E${i}`, message: `msg ${i}`, layer: 'sdk' })
    }
    const result = await svc.list({ limit: 2, offset: 1 })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data).toHaveLength(2)
    expect(result.data[0].error_code).toBe('E3') // second most recent
  })

  test('list() filters by error_code', async () => {
    await svc.append({ error_code: 'API_UNREACHABLE', message: 'a', layer: 'sdk' })
    await svc.append({ error_code: 'PLUGIN_THREW', message: 'b', layer: 'plugin' })
    await svc.append({ error_code: 'API_UNREACHABLE', message: 'c', layer: 'sdk' })

    const result = await svc.list({ error_code: 'API_UNREACHABLE' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data).toHaveLength(2)
    expect(result.data.every(e => e.error_code === 'API_UNREACHABLE')).toBe(true)
  })

  test('list() filters by layer', async () => {
    await svc.append({ error_code: 'A', message: 'a', layer: 'sdk' })
    await svc.append({ error_code: 'B', message: 'b', layer: 'plugin' })

    const result = await svc.list({ layer: 'plugin' })
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.data).toHaveLength(1)
    expect(result.data[0].layer).toBe('plugin')
  })

  test('clear() deletes all logs', async () => {
    await svc.append({ error_code: 'A', message: 'a', layer: 'sdk' })
    await svc.append({ error_code: 'B', message: 'b', layer: 'sdk' })

    const clearResult = await svc.clear()
    expect(clearResult.ok).toBe(true)
    if (!clearResult.ok) return
    expect(clearResult.data.deleted).toBe(2)

    const listResult = await svc.list()
    expect(listResult.ok).toBe(true)
    if (!listResult.ok) return
    expect(listResult.data).toHaveLength(0)
  })

  test('append() validates required fields', async () => {
    const result = await svc.append({ error_code: '', message: 'a', layer: 'sdk' })
    expect(result.ok).toBe(false)
    if (result.ok) return
    expect(result.error.code).toBe('VALIDATION_ERROR')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/services/__tests__/extension-log-service.test.ts`
Expected: FAIL (module not found)

- [ ] **Step 3: Write the service implementation**

Create `packages/core/src/services/extension-log-service.ts`:

```typescript
/**
 * ExtensionLogService — server-side storage for browser extension errors.
 *
 * Uses ELM for standard CRUD. Logs are append-only (no update).
 * The `context` field is stored as a JSON string in SQLite but
 * parsed to an object when returned.
 */

import { storageErrorToForgeError } from '../storage/error-mapper'
import type { EntityLifecycleManager } from '../storage/lifecycle-manager'
import type { Result } from '../types'

export interface ExtensionLog {
  id: string
  error_code: string
  message: string
  layer: string
  plugin: string | null
  url: string | null
  context: Record<string, unknown> | null
  created_at: string
}

export interface CreateExtensionLog {
  error_code: string
  message: string
  layer: string
  plugin?: string
  url?: string
  context?: Record<string, unknown>
}

export interface ExtensionLogFilter {
  limit?: number
  offset?: number
  error_code?: string
  layer?: string
}

export class ExtensionLogService {
  constructor(private elm: EntityLifecycleManager) {}

  /** Append a new log entry. */
  async append(input: CreateExtensionLog): Promise<Result<ExtensionLog>> {
    if (!input.error_code || input.error_code.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'error_code is required' } }
    }
    if (!input.message || input.message.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'message is required' } }
    }
    if (!input.layer || input.layer.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'layer is required' } }
    }

    const createResult = await this.elm.create('extension_logs', {
      error_code: input.error_code,
      message: input.message,
      layer: input.layer,
      plugin: input.plugin ?? null,
      url: input.url ?? null,
      context: input.context ? JSON.stringify(input.context) : null,
    })
    if (!createResult.ok) {
      return { ok: false, error: storageErrorToForgeError(createResult.error) }
    }

    const fetched = await this.elm.get('extension_logs', createResult.value.id)
    if (!fetched.ok) {
      return { ok: false, error: storageErrorToForgeError(fetched.error) }
    }
    return { ok: true, data: this.parseRow(fetched.value) }
  }

  /** List log entries with optional filters, ordered by created_at DESC. */
  async list(opts?: ExtensionLogFilter): Promise<Result<ExtensionLog[]>> {
    const where: Record<string, unknown> = {}
    if (opts?.error_code) where.error_code = opts.error_code
    if (opts?.layer) where.layer = opts.layer

    const listResult = await this.elm.list('extension_logs', {
      where: Object.keys(where).length > 0 ? where : undefined,
      orderBy: [{ field: 'created_at', direction: 'desc' }],
      limit: opts?.limit ?? 50,
      offset: opts?.offset ?? 0,
    })
    if (!listResult.ok) {
      return { ok: false, error: storageErrorToForgeError(listResult.error) }
    }
    return {
      ok: true,
      data: listResult.value.rows.map(row => this.parseRow(row)),
    }
  }

  /** Delete all log entries. Returns count of deleted rows. */
  async clear(): Promise<Result<{ deleted: number }>> {
    // ELM doesn't have a "delete all" — use list + delete loop,
    // but that's inefficient. For bulk delete, reach through ELM's
    // underlying db. However, ELM doesn't expose db directly.
    // Instead, list all IDs and delete them.
    const allResult = await this.elm.list('extension_logs', { limit: 10000 })
    if (!allResult.ok) {
      return { ok: false, error: storageErrorToForgeError(allResult.error) }
    }
    const rows = allResult.value.rows as unknown as Array<{ id: string }>
    for (const row of rows) {
      await this.elm.delete('extension_logs', row.id)
    }
    return { ok: true, data: { deleted: rows.length } }
  }

  /** Parse a raw DB row, deserializing the context JSON. */
  private parseRow(row: unknown): ExtensionLog {
    const r = row as Record<string, unknown>
    return {
      id: r.id as string,
      error_code: r.error_code as string,
      message: r.message as string,
      layer: r.layer as string,
      plugin: (r.plugin as string) ?? null,
      url: (r.url as string) ?? null,
      context: r.context ? JSON.parse(r.context as string) : null,
      created_at: r.created_at as string,
    }
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd packages/core && bun test src/services/__tests__/extension-log-service.test.ts`
Expected: PASS (all 9 tests)

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/services/extension-log-service.ts packages/core/src/services/__tests__/extension-log-service.test.ts
git commit -m "feat(core): add ExtensionLogService for server-side error logging (M7)"
```

---

## Task 5: Wire services into Services index

**Files:**
- Modify: `packages/core/src/services/index.ts`

- [ ] **Step 1: Add imports, interface fields, and constructor calls**

Add imports at the top (after existing imports):

```typescript
import { ExtensionConfigService } from './extension-config-service'
import { ExtensionLogService } from './extension-log-service'
```

Add to the `Services` interface (after `answerBank`):

```typescript
  extensionConfig: ExtensionConfigService
  extensionLogs: ExtensionLogService
```

Add to the `createServices()` return object (after `answerBank`):

```typescript
    extensionConfig: new ExtensionConfigService(db),
    extensionLogs: new ExtensionLogService(elm),
```

Add to the re-exports at the bottom:

```typescript
export { ExtensionConfigService } from './extension-config-service'
export { ExtensionLogService } from './extension-log-service'
```

- [ ] **Step 2: Run services wiring test**

Run: `cd packages/core && bun test src/services/__tests__/services-wiring.test.ts`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/services/index.ts
git commit -m "feat(core): wire ExtensionConfig + ExtensionLog services into Services (M7)"
```

---

## Task 6: HTTP Routes — extension config + logs

**Files:**
- Create: `packages/core/src/routes/extension.ts`
- Create: `packages/core/src/routes/__tests__/extension.test.ts`
- Modify: `packages/core/src/routes/server.ts`

- [ ] **Step 1: Write the failing route tests**

Create `packages/core/src/routes/__tests__/extension.test.ts`:

```typescript
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'

describe('Extension routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  // -- Config routes -------------------------------------------------------

  describe('GET /extension/config', () => {
    test('returns seeded defaults', async () => {
      const res = await apiRequest(ctx.app, 'GET', '/extension/config')
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data.baseUrl).toBe('http://localhost:3000')
      expect(body.data.devMode).toBe(false)
      expect(body.data.enabledPlugins).toEqual(['linkedin'])
      expect(body.data.enableServerLogging).toBe(true)
    })
  })

  describe('PUT /extension/config', () => {
    test('updates config keys', async () => {
      const res = await apiRequest(ctx.app, 'PUT', '/extension/config', {
        updates: { devMode: true, enableServerLogging: false },
      })
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data.devMode).toBe(true)
      expect(body.data.enableServerLogging).toBe(false)
      expect(body.data.baseUrl).toBe('http://localhost:3000') // unchanged
    })

    test('returns 400 for unknown keys', async () => {
      const res = await apiRequest(ctx.app, 'PUT', '/extension/config', {
        updates: { badKey: 'value' },
      })
      expect(res.status).toBe(400)
    })

    test('returns 400 for missing updates field', async () => {
      const res = await apiRequest(ctx.app, 'PUT', '/extension/config', {})
      expect(res.status).toBe(400)
    })
  })

  // -- Log routes ----------------------------------------------------------

  describe('POST /extension/log', () => {
    test('creates a log entry and returns 201', async () => {
      const res = await apiRequest(ctx.app, 'POST', '/extension/log', {
        error_code: 'API_UNREACHABLE',
        message: 'Failed to fetch',
        layer: 'sdk',
      })
      expect(res.status).toBe(201)
      const body = await res.json()
      expect(body.data.error_code).toBe('API_UNREACHABLE')
      expect(body.data.id).toBeDefined()
    })

    test('stores optional fields', async () => {
      const res = await apiRequest(ctx.app, 'POST', '/extension/log', {
        error_code: 'PLUGIN_THREW',
        message: 'parse error',
        layer: 'plugin',
        plugin: 'linkedin',
        url: 'https://linkedin.com/jobs/123',
        context: { step: 'extract_title' },
      })
      expect(res.status).toBe(201)
      const body = await res.json()
      expect(body.data.plugin).toBe('linkedin')
      expect(body.data.context).toEqual({ step: 'extract_title' })
    })

    test('returns 400 for missing error_code', async () => {
      const res = await apiRequest(ctx.app, 'POST', '/extension/log', {
        message: 'test', layer: 'sdk',
      })
      expect(res.status).toBe(400)
    })
  })

  describe('GET /extension/logs', () => {
    test('returns empty array initially', async () => {
      const res = await apiRequest(ctx.app, 'GET', '/extension/logs')
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data).toEqual([])
    })

    test('returns logs with pagination', async () => {
      // Create 3 logs
      for (const code of ['A', 'B', 'C']) {
        await apiRequest(ctx.app, 'POST', '/extension/log', {
          error_code: code, message: `msg ${code}`, layer: 'sdk',
        })
      }
      const res = await apiRequest(ctx.app, 'GET', '/extension/logs?limit=2')
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data).toHaveLength(2)
    })

    test('filters by error_code', async () => {
      await apiRequest(ctx.app, 'POST', '/extension/log', {
        error_code: 'A', message: 'm', layer: 'sdk',
      })
      await apiRequest(ctx.app, 'POST', '/extension/log', {
        error_code: 'B', message: 'm', layer: 'plugin',
      })
      const res = await apiRequest(ctx.app, 'GET', '/extension/logs?error_code=A')
      expect(res.status).toBe(200)
      const body = await res.json()
      expect(body.data).toHaveLength(1)
      expect(body.data[0].error_code).toBe('A')
    })
  })

  describe('DELETE /extension/logs', () => {
    test('clears all logs and returns 204', async () => {
      await apiRequest(ctx.app, 'POST', '/extension/log', {
        error_code: 'A', message: 'm', layer: 'sdk',
      })
      const res = await apiRequest(ctx.app, 'DELETE', '/extension/logs')
      expect(res.status).toBe(204)

      // Verify empty
      const listRes = await apiRequest(ctx.app, 'GET', '/extension/logs')
      const body = await listRes.json()
      expect(body.data).toHaveLength(0)
    })
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/routes/__tests__/extension.test.ts`
Expected: FAIL (module not found)

- [ ] **Step 3: Write the routes**

Create `packages/core/src/routes/extension.ts`:

```typescript
/**
 * Extension routes — config + error logging for the browser extension.
 *
 * GET    /extension/config  — get full config
 * PUT    /extension/config  — update config keys
 * POST   /extension/log     — append error log
 * GET    /extension/logs    — list logs (paginated, filterable)
 * DELETE /extension/logs    — clear all logs
 */

import { Hono } from 'hono'
import type { Services } from '../services'
import { mapStatusCode } from './status-codes'

export function extensionRoutes(services: Services) {
  const app = new Hono()

  // -- Config ---------------------------------------------------------------

  app.get('/extension/config', (c) => {
    const result = services.extensionConfig.getAll()
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.put('/extension/config', async (c) => {
    const body = await c.req.json()
    if (!body.updates || typeof body.updates !== 'object') {
      return c.json(
        { error: { code: 'VALIDATION_ERROR', message: 'Body must contain an "updates" object' } },
        400,
      )
    }
    const result = services.extensionConfig.setMany(body.updates)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  // -- Logs -----------------------------------------------------------------

  app.post('/extension/log', async (c) => {
    const body = await c.req.json()
    const result = await services.extensionLogs.append(body)
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data }, 201)
  })

  app.get('/extension/logs', async (c) => {
    const limit = c.req.query('limit') ? parseInt(c.req.query('limit')!, 10) : undefined
    const offset = c.req.query('offset') ? parseInt(c.req.query('offset')!, 10) : undefined
    const error_code = c.req.query('error_code') ?? undefined
    const layer = c.req.query('layer') ?? undefined

    const result = await services.extensionLogs.list({ limit, offset, error_code, layer })
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.json({ data: result.data })
  })

  app.delete('/extension/logs', async (c) => {
    const result = await services.extensionLogs.clear()
    if (!result.ok) return c.json({ error: result.error }, mapStatusCode(result.error.code))
    return c.body(null, 204)
  })

  return app
}
```

- [ ] **Step 4: Mount routes in server.ts**

Add import at the top of `packages/core/src/routes/server.ts` (after `answerBankRoutes`):

```typescript
import { extensionRoutes } from './extension'
```

Add route mount (after the `answerBankRoutes` line):

```typescript
  app.route('/', extensionRoutes(services))
```

- [ ] **Step 5: Run route tests to verify they pass**

Run: `cd packages/core && bun test src/routes/__tests__/extension.test.ts`
Expected: PASS (all tests)

- [ ] **Step 6: Run full core test suite**

Run: `cd packages/core && bun test`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add packages/core/src/routes/extension.ts packages/core/src/routes/__tests__/extension.test.ts packages/core/src/routes/server.ts
git commit -m "feat(core): add extension config + log HTTP routes (M7)"
```

---

## Task 7: SDK Types + Resources

**Files:**
- Modify: `packages/sdk/src/types.ts`
- Create: `packages/sdk/src/resources/extension-config.ts`
- Create: `packages/sdk/src/resources/extension-logs.ts`
- Modify: `packages/sdk/src/client.ts`
- Modify: `packages/sdk/src/index.ts`

- [ ] **Step 1: Add SDK types**

Append to the bottom of `packages/sdk/src/types.ts` (before the closing comment or at EOF):

```typescript
// ---------------------------------------------------------------------------
// Extension Infrastructure (M7)
// ---------------------------------------------------------------------------

/** Full extension config — all keys merged over defaults. */
export interface ExtensionConfig {
  baseUrl: string
  devMode: boolean
  enabledPlugins: string[]
  enableServerLogging: boolean
}

/** A stored extension error log entry. */
export interface ExtensionLog {
  id: string
  error_code: string
  message: string
  layer: string
  plugin: string | null
  url: string | null
  context: Record<string, unknown> | null
  created_at: string
}

/** Input for appending an extension log entry. */
export interface CreateExtensionLog {
  error_code: string
  message: string
  layer: string
  plugin?: string
  url?: string
  context?: Record<string, unknown>
}

/** Filter options for listing extension logs. */
export interface ExtensionLogFilter {
  limit?: number
  offset?: number
  error_code?: string
  layer?: string
}
```

- [ ] **Step 2: Create ExtensionConfigResource**

Create `packages/sdk/src/resources/extension-config.ts`:

```typescript
import type { ExtensionConfig, RequestFn, Result } from '../types'

export class ExtensionConfigResource {
  constructor(private request: RequestFn) {}

  /** Get full extension config (merged over defaults). */
  get(): Promise<Result<ExtensionConfig>> {
    return this.request<ExtensionConfig>('GET', '/api/extension/config')
  }

  /** Update one or more config keys. Returns full config after update. */
  update(updates: Record<string, unknown>): Promise<Result<ExtensionConfig>> {
    return this.request<ExtensionConfig>('PUT', '/api/extension/config', { updates })
  }
}
```

- [ ] **Step 3: Create ExtensionLogsResource**

Create `packages/sdk/src/resources/extension-logs.ts`:

```typescript
import type { ExtensionLog, CreateExtensionLog, ExtensionLogFilter, RequestFn, Result } from '../types'

export class ExtensionLogsResource {
  constructor(private request: RequestFn) {}

  /** Append an error log entry. */
  append(entry: CreateExtensionLog): Promise<Result<ExtensionLog>> {
    return this.request<ExtensionLog>('POST', '/api/extension/log', entry)
  }

  /** List log entries with optional filters. */
  async list(opts?: ExtensionLogFilter): Promise<Result<ExtensionLog[]>> {
    const params = new URLSearchParams()
    if (opts?.limit !== undefined) params.set('limit', String(opts.limit))
    if (opts?.offset !== undefined) params.set('offset', String(opts.offset))
    if (opts?.error_code) params.set('error_code', opts.error_code)
    if (opts?.layer) params.set('layer', opts.layer)
    const qs = params.toString()
    const path = qs ? `/api/extension/logs?${qs}` : '/api/extension/logs'
    return this.request<ExtensionLog[]>('GET', path)
  }

  /** Clear all log entries. */
  clear(): Promise<Result<void>> {
    return this.request<void>('DELETE', '/api/extension/logs')
  }
}
```

- [ ] **Step 4: Wire resources into ForgeClient**

In `packages/sdk/src/client.ts`, add imports (after `AnswerBankResource`):

```typescript
import { ExtensionConfigResource } from './resources/extension-config'
import { ExtensionLogsResource } from './resources/extension-logs'
```

Add property declarations (after `public answerBank`):

```typescript
  /** Extension config — key-value config stored in Forge DB (M7). */
  public extensionConfig: ExtensionConfigResource
  /** Extension logs — server-side error logging (M7). */
  public extensionLogs: ExtensionLogsResource
```

Add initialization in constructor (after `this.answerBank`):

```typescript
    this.extensionConfig = new ExtensionConfigResource(req)
    this.extensionLogs = new ExtensionLogsResource(req)
```

- [ ] **Step 5: Update SDK barrel exports**

In `packages/sdk/src/index.ts`, add type exports (after the `AnswerBankEntry` section):

```typescript
// Extension infrastructure types (M7)
export type { ExtensionConfig, ExtensionLog, CreateExtensionLog, ExtensionLogFilter } from './types'
```

Add resource class exports (after `AnswerBankResource`):

```typescript
export { ExtensionConfigResource } from './resources/extension-config'
export { ExtensionLogsResource } from './resources/extension-logs'
```

- [ ] **Step 6: Verify SDK builds**

Run: `cd packages/sdk && bun run build 2>/dev/null || echo "no build step"`
Then: `cd packages/core && bun test`
Expected: PASS (SDK types used by core tests should still compile)

- [ ] **Step 7: Commit**

```bash
git add packages/sdk/src/types.ts packages/sdk/src/resources/extension-config.ts packages/sdk/src/resources/extension-logs.ts packages/sdk/src/client.ts packages/sdk/src/index.ts
git commit -m "feat(sdk): add ExtensionConfig + ExtensionLogs resources (M7)"
```

---

## Task 8: Extension config.ts rewrite — API-first with fallback

**Files:**
- Modify: `packages/extension/src/storage/config.ts`
- Modify: `packages/extension/src/background/client.ts`

- [ ] **Step 1: Rewrite config.ts**

Replace the contents of `packages/extension/src/storage/config.ts`:

```typescript
// packages/extension/src/storage/config.ts

export interface ExtensionConfig {
  baseUrl: string
  devMode: boolean
  enabledPlugins: string[]
  enableServerLogging: boolean
}

const DEFAULTS: ExtensionConfig = {
  baseUrl: 'http://localhost:3000',
  devMode: false,
  enabledPlugins: ['linkedin'],
  enableServerLogging: true,
}

const STORAGE_KEY = 'forge_ext_config'

/**
 * Module-level flag for fast synchronous checks (used by reportError).
 * Updated whenever loadConfig() succeeds.
 */
export let serverLoggingEnabled = true

/**
 * Load config: try Forge API first, fall back to chrome.storage.local cache.
 *
 * On API success, caches the result to chrome.storage.local for offline use.
 * On API failure, reads cached config from chrome.storage.local.
 * If both fail, returns DEFAULTS.
 */
export async function loadConfig(): Promise<ExtensionConfig> {
  // Try API first — need to read baseUrl from local storage to know where API is
  const stored = await chrome.storage.local.get(STORAGE_KEY)
  const cached = (stored[STORAGE_KEY] ?? {}) as Partial<ExtensionConfig>
  const baseUrl = cached.baseUrl ?? DEFAULTS.baseUrl

  try {
    const response = await fetch(`${baseUrl}/api/extension/config`)
    if (response.ok) {
      const json = await response.json()
      const config: ExtensionConfig = { ...DEFAULTS, ...json.data }
      // Cache for offline fallback
      await chrome.storage.local.set({ [STORAGE_KEY]: config })
      serverLoggingEnabled = config.enableServerLogging
      return config
    }
  } catch {
    // API unreachable — fall through to cache
  }

  // Fallback: cached config merged over defaults
  const config = { ...DEFAULTS, ...cached }
  serverLoggingEnabled = config.enableServerLogging
  return config
}
```

- [ ] **Step 2: Update background/client.ts import**

The `getClient()` function in `packages/extension/src/background/client.ts` already imports `loadConfig` from `../storage/config` and uses `config.baseUrl`. No changes needed — the interface is the same. Verify the import still works.

- [ ] **Step 3: Run extension tests**

Run: `cd packages/extension && bun test`
Expected: PASS (existing tests should still work — `loadConfig` signature unchanged)

- [ ] **Step 4: Commit**

```bash
git add packages/extension/src/storage/config.ts
git commit -m "feat(ext): rewrite config to API-first with chrome.storage.local fallback (M7)"
```

---

## Task 9: Extension error reporting — reportError()

**Files:**
- Modify: `packages/extension/src/lib/errors.ts`

- [ ] **Step 1: Add reportError function and wire into existing error mappers**

At the top of `packages/extension/src/lib/errors.ts`, add the import for the config flag:

```typescript
import { serverLoggingEnabled } from '../storage/config'
```

Add the `reportError` function after the existing `mapNetworkError` function:

```typescript
/**
 * Report an error to the Forge server for logging.
 *
 * Fire-and-forget: does not await the response, catches and swallows
 * errors to prevent reporting from causing cascading failures.
 *
 * Reads `serverLoggingEnabled` synchronously from the module-level flag
 * (set by loadConfig on startup) to avoid async config reads on every error.
 */
export function reportError(err: ExtensionError): void {
  if (!serverLoggingEnabled) return

  // Read baseUrl from chrome.storage.local (sync not available, so use cached approach)
  // Instead, use a module-level baseUrl that gets set alongside serverLoggingEnabled
  const body = {
    error_code: err.code,
    message: err.message,
    layer: err.layer,
    plugin: err.plugin ?? undefined,
    url: err.url ?? undefined,
    context: err.context ?? undefined,
  }

  // Fire and forget — we need the baseUrl from config
  chrome.storage.local.get('forge_ext_config').then((stored) => {
    const config = (stored.forge_ext_config ?? {}) as { baseUrl?: string }
    const baseUrl = config.baseUrl ?? 'http://localhost:3000'
    fetch(`${baseUrl}/api/extension/log`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }).catch(() => {
      // Swallow — logging failures must not cascade
    })
  }).catch(() => {
    // Swallow
  })
}
```

Wire `reportError` into `mapSdkError` and `mapNetworkError` — add a call at the end of each function before the return:

In `mapSdkError`, before `return extError(...)`:

```typescript
  const result = extError(code, err.message, {
    layer: 'sdk',
    url: opts.url,
    context: { sdk_code: err.code, ...opts.context, details: err.details },
  })
  reportError(result)
  return result
```

In `mapNetworkError`, before `return extError(...)`:

```typescript
  const result = extError(code, message, { layer: 'sdk', url: opts.url })
  reportError(result)
  return result
```

- [ ] **Step 2: Run extension tests**

Run: `cd packages/extension && bun test`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add packages/extension/src/lib/errors.ts
git commit -m "feat(ext): add reportError() for server-side error logging (M7)"
```

---

## Task 10: WebUI — Config Editor page

**Files:**
- Create: `packages/webui/src/routes/settings/extension/+page.svelte`
- Modify: `packages/webui/src/lib/components/ProfileMenu.svelte`

- [ ] **Step 1: Create the config editor page**

Create `packages/webui/src/routes/settings/extension/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { PageHeader } from '$lib/components'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'

  let loading = $state(true)
  let saving = $state(false)

  let baseUrl = $state('')
  let devMode = $state(false)
  let enabledPlugins = $state('')
  let enableServerLogging = $state(true)

  async function loadConfig() {
    loading = true
    const result = await forge.extensionConfig.get()
    if (result.ok) {
      baseUrl = result.data.baseUrl
      devMode = result.data.devMode
      enabledPlugins = result.data.enabledPlugins.join(', ')
      enableServerLogging = result.data.enableServerLogging
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
    loading = false
  }

  async function saveConfig() {
    saving = true
    const plugins = enabledPlugins
      .split(',')
      .map(s => s.trim())
      .filter(s => s.length > 0)
    const result = await forge.extensionConfig.update({
      baseUrl,
      devMode,
      enabledPlugins: plugins,
      enableServerLogging,
    })
    if (result.ok) {
      addToast({ message: 'Config saved', type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to save config'), type: 'error' })
    }
    saving = false
  }

  onMount(loadConfig)
</script>

<div class="settings-page">
  <PageHeader title="Extension Config" subtitle="Browser extension settings stored in Forge. The extension fetches this config on startup." />

  {#if loading}
    <div class="loading">Loading...</div>
  {:else}
    <form class="settings-form" onsubmit={(e) => { e.preventDefault(); saveConfig() }}>
      <div class="form-field">
        <label for="cfg-baseUrl">API Base URL</label>
        <input id="cfg-baseUrl" type="text" bind:value={baseUrl} />
        <span class="form-hint">The Forge API server URL the extension connects to.</span>
      </div>

      <div class="form-field">
        <label for="cfg-devMode">Dev Mode</label>
        <label class="toggle-label">
          <input id="cfg-devMode" type="checkbox" bind:checked={devMode} />
          <span>{devMode ? 'Enabled' : 'Disabled'}</span>
        </label>
        <span class="form-hint">Enables verbose console logging in the extension.</span>
      </div>

      <div class="form-field">
        <label for="cfg-plugins">Enabled Plugins</label>
        <input id="cfg-plugins" type="text" bind:value={enabledPlugins} />
        <span class="form-hint">Comma-separated list of enabled plugins (e.g. linkedin, workday).</span>
      </div>

      <div class="form-field">
        <label for="cfg-logging">Server Logging</label>
        <label class="toggle-label">
          <input id="cfg-logging" type="checkbox" bind:checked={enableServerLogging} />
          <span>{enableServerLogging ? 'Enabled' : 'Disabled'}</span>
        </label>
        <span class="form-hint">When enabled, extension errors are reported to the Forge server.</span>
      </div>

      <button type="submit" class="btn btn-primary" disabled={saving}>
        {saving ? 'Saving...' : 'Save Config'}
      </button>
    </form>
  {/if}
</div>

<style>
  .settings-page {
    max-width: 640px;
  }

  .settings-form {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .form-field > label:first-child {
    font-size: var(--text-sm);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
  }

  .form-field input[type="text"] {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    color: var(--text-primary);
    background: var(--color-surface);
    transition: border-color 0.15s;
  }

  .form-field input[type="text"]:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-primary);
    cursor: pointer;
  }

  .toggle-label input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }

  .form-hint {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .loading {
    text-align: center;
    padding: var(--space-8);
    color: var(--text-muted);
  }
</style>
```

- [ ] **Step 2: Add links in ProfileMenu**

In `packages/webui/src/lib/components/ProfileMenu.svelte`, in the Settings section (after the EEO Disclosures button), add:

```svelte
      <button class="menu-item menu-link" onclick={() => navigateTo('/settings/extension')}>
        Extension Config
      </button>
      <button class="menu-item menu-link" onclick={() => navigateTo('/settings/extension-logs')}>
        Extension Logs
      </button>
```

- [ ] **Step 3: Verify WebUI compiles**

Run: `cd packages/webui && bun run build 2>&1 | tail -5`
Expected: Build succeeds

- [ ] **Step 4: Commit**

```bash
git add packages/webui/src/routes/settings/extension/+page.svelte packages/webui/src/lib/components/ProfileMenu.svelte
git commit -m "feat(webui): add Extension Config settings page (M7)"
```

---

## Task 11: WebUI — Log Viewer page

**Files:**
- Create: `packages/webui/src/routes/settings/extension-logs/+page.svelte`

- [ ] **Step 1: Create the log viewer page**

Create `packages/webui/src/routes/settings/extension-logs/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { PageHeader, ConfirmDialog } from '$lib/components'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import type { ExtensionLog } from '@forge/sdk'

  let loading = $state(true)
  let logs = $state<ExtensionLog[]>([])
  let expandedId = $state<string | null>(null)

  // Filters
  let filterCode = $state('')
  let filterLayer = $state('')

  // Pagination
  const PAGE_SIZE = 50
  let offset = $state(0)
  let hasMore = $state(false)

  // Clear confirmation
  let showClearConfirm = $state(false)

  async function loadLogs() {
    loading = true
    const opts: Record<string, unknown> = { limit: PAGE_SIZE + 1, offset }
    if (filterCode) opts.error_code = filterCode
    if (filterLayer) opts.layer = filterLayer

    const result = await forge.extensionLogs.list(opts as any)
    if (result.ok) {
      if (result.data.length > PAGE_SIZE) {
        hasMore = true
        logs = result.data.slice(0, PAGE_SIZE)
      } else {
        hasMore = false
        logs = result.data
      }
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
    loading = false
  }

  function toggleExpanded(id: string) {
    expandedId = expandedId === id ? null : id
  }

  function applyFilters() {
    offset = 0
    loadLogs()
  }

  function clearFilters() {
    filterCode = ''
    filterLayer = ''
    offset = 0
    loadLogs()
  }

  function nextPage() {
    offset += PAGE_SIZE
    loadLogs()
  }

  function prevPage() {
    offset = Math.max(0, offset - PAGE_SIZE)
    loadLogs()
  }

  async function clearAllLogs() {
    const result = await forge.extensionLogs.clear()
    if (result.ok) {
      addToast({ message: 'All logs cleared', type: 'success' })
      logs = []
      offset = 0
      hasMore = false
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to clear logs'), type: 'error' })
    }
    showClearConfirm = false
  }

  function formatTimestamp(ts: string): string {
    try {
      const d = new Date(ts + 'Z')
      return d.toLocaleString()
    } catch {
      return ts
    }
  }

  onMount(loadLogs)
</script>

<div class="logs-page">
  <PageHeader title="Extension Logs" subtitle="Server-side error logs from the browser extension." />

  <div class="toolbar">
    <div class="filters">
      <input
        type="text"
        placeholder="Filter by error code..."
        bind:value={filterCode}
        onkeydown={(e) => { if (e.key === 'Enter') applyFilters() }}
      />
      <select bind:value={filterLayer} onchange={applyFilters}>
        <option value="">All layers</option>
        <option value="plugin">plugin</option>
        <option value="content">content</option>
        <option value="background">background</option>
        <option value="popup">popup</option>
        <option value="sdk">sdk</option>
      </select>
      {#if filterCode || filterLayer}
        <button class="btn btn-ghost" onclick={clearFilters}>Clear filters</button>
      {/if}
    </div>
    <button
      class="btn btn-danger"
      onclick={() => showClearConfirm = true}
      disabled={logs.length === 0}
    >
      Clear All
    </button>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if logs.length === 0}
    <div class="empty">No extension logs recorded.</div>
  {:else}
    <div class="log-table-wrapper">
      <table class="log-table">
        <thead>
          <tr>
            <th>Timestamp</th>
            <th>Error Code</th>
            <th>Message</th>
            <th>Layer</th>
            <th>Plugin</th>
          </tr>
        </thead>
        <tbody>
          {#each logs as log}
            <tr
              class="log-row"
              class:expanded={expandedId === log.id}
              onclick={() => toggleExpanded(log.id)}
            >
              <td class="ts">{formatTimestamp(log.created_at)}</td>
              <td><code>{log.error_code}</code></td>
              <td class="msg">{log.message}</td>
              <td><span class="badge layer-{log.layer}">{log.layer}</span></td>
              <td>{log.plugin ?? '-'}</td>
            </tr>
            {#if expandedId === log.id && (log.url || log.context)}
              <tr class="detail-row">
                <td colspan="5">
                  <div class="detail-content">
                    {#if log.url}
                      <div class="detail-field">
                        <strong>URL:</strong> <span>{log.url}</span>
                      </div>
                    {/if}
                    {#if log.context}
                      <div class="detail-field">
                        <strong>Context:</strong>
                        <pre>{JSON.stringify(log.context, null, 2)}</pre>
                      </div>
                    {/if}
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>

    <div class="pagination">
      <button class="btn btn-ghost" onclick={prevPage} disabled={offset === 0}>Previous</button>
      <span class="page-info">Showing {offset + 1}–{offset + logs.length}</span>
      <button class="btn btn-ghost" onclick={nextPage} disabled={!hasMore}>Next</button>
    </div>
  {/if}
</div>

{#if showClearConfirm}
  <ConfirmDialog
    title="Clear All Logs"
    message="This will permanently delete all extension error logs. This cannot be undone."
    confirmLabel="Clear All"
    onconfirm={clearAllLogs}
    oncancel={() => showClearConfirm = false}
  />
{/if}

<style>
  .logs-page {
    max-width: 1000px;
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }

  .filters {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .filters input[type="text"] {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    color: var(--text-primary);
    background: var(--color-surface);
    width: 200px;
  }

  .filters input[type="text"]:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .filters select {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    color: var(--text-primary);
    background: var(--color-surface);
  }

  .log-table-wrapper {
    overflow-x: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
  }

  .log-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  .log-table th {
    text-align: left;
    padding: var(--space-3);
    background: var(--color-surface-sunken);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
    border-bottom: 1px solid var(--color-border);
  }

  .log-table td {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    vertical-align: top;
  }

  .log-row {
    cursor: pointer;
    transition: background 0.1s;
  }

  .log-row:hover {
    background: var(--color-surface-raised);
  }

  .log-row.expanded {
    background: var(--color-surface-raised);
  }

  .ts {
    white-space: nowrap;
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .msg {
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    display: inline-block;
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    font-weight: var(--font-medium);
  }

  .layer-sdk { background: var(--color-primary-subtle); color: var(--color-primary); }
  .layer-plugin { background: var(--color-success-subtle, #dcfce7); color: var(--color-success, #16a34a); }
  .layer-background { background: var(--color-warning-subtle, #fef9c3); color: var(--color-warning, #ca8a04); }
  .layer-content { background: var(--color-info-subtle, #dbeafe); color: var(--color-info, #2563eb); }
  .layer-popup { background: var(--color-surface-sunken); color: var(--text-secondary); }

  .detail-row td {
    padding: 0;
    border-bottom: 1px solid var(--color-border);
  }

  .detail-content {
    padding: var(--space-3) var(--space-4);
    background: var(--color-surface-sunken);
  }

  .detail-field {
    margin-bottom: var(--space-2);
  }

  .detail-field:last-child {
    margin-bottom: 0;
  }

  .detail-field strong {
    color: var(--text-secondary);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .detail-field pre {
    margin-top: var(--space-1);
    padding: var(--space-2);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    overflow-x: auto;
    white-space: pre-wrap;
  }

  .pagination {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: var(--space-4);
    margin-top: var(--space-4);
  }

  .page-info {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .loading, .empty {
    text-align: center;
    padding: var(--space-8);
    color: var(--text-muted);
  }
</style>
```

- [ ] **Step 2: Verify WebUI compiles**

Run: `cd packages/webui && bun run build 2>&1 | tail -5`
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add packages/webui/src/routes/settings/extension-logs/+page.svelte
git commit -m "feat(webui): add Extension Logs viewer page (M7)"
```

---

## Task 12: Version bump + dist rebuild

**Files:**
- Modify: `packages/extension/manifest.json`
- Modify: `packages/extension/manifest.firefox.json`

- [ ] **Step 1: Bump version to 1.0.0 in both manifests**

In `packages/extension/manifest.json`, change:
```json
"version": "0.1.6"
```
to:
```json
"version": "1.0.0"
```

In `packages/extension/manifest.firefox.json`, change:
```json
"version": "0.1.6"
```
to:
```json
"version": "1.0.0"
```

- [ ] **Step 2: Build extension for both browsers**

Run: `cd packages/extension && bun run build`
Expected: Produces `dist/chrome/` and `dist/firefox/` with no errors

- [ ] **Step 3: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: All tests pass

- [ ] **Step 4: Run full test suite**

Run: `cd /Users/adam/notes/job-hunting && just test`
Expected: All tests pass across core/sdk/extension

- [ ] **Step 5: Commit**

```bash
git add packages/extension/manifest.json packages/extension/manifest.firefox.json packages/extension/dist/
git commit -m "chore(ext): bump version to 1.0.0 — MVP complete (M7)"
```

---

## Summary

| Task | Description | Tests |
|------|-------------|-------|
| 1 | Migration 051 — extension_config + extension_logs | Schema test |
| 2 | Entity map — extension_logs registration | Entity map test |
| 3 | ExtensionConfigService — key-value CRUD | 9 unit tests |
| 4 | ExtensionLogService — append/list/clear | 9 unit tests |
| 5 | Wire services into Services index | Wiring test |
| 6 | HTTP routes — config + logs | 9 route tests |
| 7 | SDK types + resources | Build verification |
| 8 | Extension config.ts — API-first + fallback | Extension tests |
| 9 | Extension reportError() | Extension tests |
| 10 | WebUI config editor + profile menu links | Build verification |
| 11 | WebUI log viewer | Build verification |
| 12 | Version bump to 1.0.0 + dist rebuild | Full test suite |
