# P2 — API Read-Only Sanity

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a background service worker to the extension that instantiates `@forge/sdk`'s `ForgeClient`, calls `GET /api/health` on popup open to show a status dot, and calls `sdk.organizations.list()` behind a "List Organizations" button in the popup. Includes a smoke test that proves the extension can complete a full popup → background → SDK → Forge API roundtrip.

**Architecture:** New `src/background/index.ts` service worker owns a singleton `ForgeClient`. Popup uses `chrome.runtime.sendMessage({ cmd })` to call the background, which forwards to the SDK. Config (just `baseUrl` and `devMode`) lives in `chrome.storage.local` behind a thin wrapper. Errors from the SDK are wrapped in `ExtensionError` taxonomy and sent back to the popup as toast-friendly messages.

**Tech Stack:** Everything from P1, plus `@forge/sdk` workspace dep, `chrome.storage.local` API, Chrome service worker lifecycle, Bun's `bun:test` for a smoke test against a running Forge server.

**Worktree:** `.claude/worktrees/forge-ext-p2-api-read/` on branch `feat/forge-ext/p2-api-read`.

**Depends on:**
- P0 — `forge-core-extension-cors` must be merged to main first
- Can run in parallel with P1 (starts from the same base as P1, so if P1 hasn't merged, this worktree branches from main and later merges P1's package scaffold)

**Blocks:** P3 (API write simple).

---

## Context You Need

### How P2 relates to P1

If P1 is already merged to main: this worktree builds on top of the `packages/extension/` package that P1 created. Just add files.

**If P1 is running in parallel and not yet merged**: branch from main (which lacks `packages/extension/`) and create a minimal scaffold matching P1's Tasks 1–3 (package.json, tsconfig, errors.ts, popup shell, manifest, vite config). Do NOT duplicate the plugin work — just enough to have a buildable extension where we can add the background worker and health check.

**In the plan below, assume P1 is merged.** If it isn't, apply the "minimal scaffold" note at the start of Task 1.

### Background service worker in MV3

MV3 background scripts are **service workers**: short-lived, event-driven. They can be terminated at any time and re-instantiated on demand. Implications:
- No persistent in-memory state across invocations (use `chrome.storage` for anything you need later)
- No DOM, no `window`, no `localStorage`
- `self` instead of `window`; `chrome.runtime.onMessage` for message routing
- Fetch works normally

Bundle size matters — the worker is re-parsed on every wake. Keep it lean.

### SDK facts (from research)

File: `packages/sdk/src/client.ts`

```typescript
import { ForgeClient } from '@forge/sdk'
const client = new ForgeClient({ baseUrl: 'http://localhost:3000' })

// Health check
const health = await client.health()
// Returns: { ok: true, data: { server: 'ok', version: '0.0.1' } }
//       or { ok: false, error: { code: string, message: string } }

// Organizations list
const orgs = await client.organizations.list({ limit: 20, offset: 0 })
// Returns: { ok: true, data: Organization[], pagination: { total, offset, limit } }
//       or { ok: false, error: { code, message } }
```

The SDK is `devDeps`-only (`typescript`, `@types/bun`). It has zero runtime deps — safe to bundle into the extension.

### Smoke test requirements

A "smoke test" here means a Bun test that:
1. Starts a test Forge server (in-process, via the same helpers the core uses)
2. Invokes the extension's background worker logic directly (not through Chrome — we can't run the extension outside Chrome)
3. Asserts the background code can successfully call `client.health()` and `client.organizations.list()` against the running server

This isn't end-to-end through Chrome — it's proving the background code wires up correctly. True end-to-end testing (loading the extension in headless Chrome via Puppeteer) is MVP scope.

---

## File Structure

```
packages/extension/
├── package.json                         # modified: add @forge/sdk dep
├── manifest.json                        # modified: add background service_worker
├── src/
│   ├── background/
│   │   ├── index.ts                     # NEW: service worker entry, message router
│   │   ├── client.ts                    # NEW: ForgeClient singleton
│   │   └── handlers/
│   │       ├── health.ts                # NEW: health check handler
│   │       └── orgs.ts                  # NEW: orgs.list handler
│   ├── storage/
│   │   └── config.ts                    # NEW: chrome.storage.local wrapper
│   ├── lib/
│   │   ├── errors.ts                    # modified: add SDK error mapper
│   │   └── messaging.ts                 # NEW: typed message contract
│   └── popup/
│       └── Popup.svelte                 # modified: add health dot + org list button
└── tests/
    └── background/
        ├── client.test.ts               # NEW: client singleton tests (mocked)
        └── smoke.test.ts                # NEW: live-server roundtrip smoke test
```

---

## Task 1: Worktree Setup

- [ ] **Step 1.1: Verify P0 has merged**

```bash
cd /Users/adam/notes/job-hunting
git log --oneline main | grep -i "extension CORS" | head -5
```

If nothing appears, P0 has not merged. Stop and complete P0 first.

- [ ] **Step 1.2: Create worktree from main**

```bash
git worktree add .claude/worktrees/forge-ext-p2-api-read -b feat/forge-ext/p2-api-read
cd .claude/worktrees/forge-ext-p2-api-read
```

- [ ] **Step 1.3: Verify P1 scaffold exists**

```bash
ls packages/extension/src/plugin/ 2>/dev/null && echo "P1 merged" || echo "P1 NOT merged"
```

If P1 is not merged: branch off `feat/forge-ext/p1-extraction` instead:

```bash
cd /Users/adam/notes/job-hunting
git worktree remove .claude/worktrees/forge-ext-p2-api-read
git worktree add .claude/worktrees/forge-ext-p2-api-read -b feat/forge-ext/p2-api-read feat/forge-ext/p1-extraction
cd .claude/worktrees/forge-ext-p2-api-read
```

This creates P2 as a descendant of the P1 branch. When both merge, P2 brings along P1's commits.

- [ ] **Step 1.4: Install deps**

```bash
bun install
```

---

## Task 2: Add @forge/sdk Dependency

**Files:**
- Modify: `packages/extension/package.json`

- [ ] **Step 2.1: Add `@forge/sdk` as a workspace dep**

In `packages/extension/package.json`, add to the `dependencies` section:

```json
"dependencies": {
  "svelte": "^5.0.0",
  "@forge/sdk": "workspace:*"
}
```

- [ ] **Step 2.2: Install**

```bash
bun install
```

Expected: bun resolves `@forge/sdk` to the local `packages/sdk/` workspace.

- [ ] **Step 2.3: Verify the import resolves**

```bash
cd packages/extension
bun -e "import { ForgeClient } from '@forge/sdk'; console.log(typeof ForgeClient)"
```

Expected: prints `function`.

- [ ] **Step 2.4: Commit**

```bash
git add packages/extension/package.json bun.lock
git commit -m "chore(ext): add @forge/sdk workspace dependency"
```

---

## Task 3: Typed Message Contract

**Files:**
- Create: `packages/extension/src/lib/messaging.ts`

- [ ] **Step 3.1: Write the messaging module**

```typescript
// packages/extension/src/lib/messaging.ts

import type { ExtensionError } from './errors'

/**
 * Commands the popup sends to the background worker.
 * Extend with new cmd types as later phases add capability.
 */
export type Command =
  | { cmd: 'health' }
  | { cmd: 'orgs.list'; limit?: number }

/**
 * Responses from the background worker to the popup.
 * Generic over the data type. Use discriminated union for typed consumers.
 */
export type Response<T> =
  | { ok: true; data: T }
  | { ok: false; error: ExtensionError }

/** Typed wrapper around chrome.runtime.sendMessage for the popup side. */
export async function sendCommand<T = unknown>(cmd: Command): Promise<Response<T>> {
  try {
    const response = (await chrome.runtime.sendMessage(cmd)) as Response<T> | undefined
    if (!response) {
      return {
        ok: false,
        error: {
          code: 'UNKNOWN_ERROR',
          message: 'No response from background worker',
          layer: 'popup',
          timestamp: new Date().toISOString(),
        },
      }
    }
    return response
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    return {
      ok: false,
      error: {
        code: 'UNKNOWN_ERROR',
        message,
        layer: 'popup',
        timestamp: new Date().toISOString(),
      },
    }
  }
}
```

- [ ] **Step 3.2: Commit**

```bash
git add packages/extension/src/lib/messaging.ts
git commit -m "feat(ext): typed message contract for popup<->background"
```

---

## Task 4: Config Storage Wrapper

**Files:**
- Create: `packages/extension/src/storage/config.ts`

- [ ] **Step 4.1: Write the config module**

```typescript
// packages/extension/src/storage/config.ts

export interface ExtensionConfig {
  baseUrl: string
  devMode: boolean
  enabledPlugins: string[]
}

const DEFAULTS: ExtensionConfig = {
  baseUrl: 'http://localhost:3000',
  devMode: true,
  enabledPlugins: ['linkedin'],
}

const STORAGE_KEY = 'forge_ext_config'

/**
 * Load the extension config from chrome.storage.local, merged over defaults.
 * Returns defaults if no config is stored.
 */
export async function loadConfig(): Promise<ExtensionConfig> {
  const stored = await chrome.storage.local.get(STORAGE_KEY)
  const partial = (stored[STORAGE_KEY] ?? {}) as Partial<ExtensionConfig>
  return { ...DEFAULTS, ...partial }
}

export async function saveConfig(config: Partial<ExtensionConfig>): Promise<void> {
  const current = await loadConfig()
  const next = { ...current, ...config }
  await chrome.storage.local.set({ [STORAGE_KEY]: next })
}
```

- [ ] **Step 4.2: Commit**

```bash
git add packages/extension/src/storage/config.ts
git commit -m "feat(ext): chrome.storage.local config wrapper with defaults"
```

---

## Task 5: ForgeClient Singleton in Background

**Files:**
- Create: `packages/extension/src/background/client.ts`

- [ ] **Step 5.1: Write the client singleton**

```typescript
// packages/extension/src/background/client.ts

import { ForgeClient } from '@forge/sdk'
import { loadConfig } from '../storage/config'

let instance: ForgeClient | null = null
let instanceBaseUrl: string | null = null

/**
 * Get (or create) the ForgeClient singleton.
 * Re-instantiates if the baseUrl in config has changed since the last call.
 */
export async function getClient(): Promise<ForgeClient> {
  const config = await loadConfig()
  if (!instance || instanceBaseUrl !== config.baseUrl) {
    instance = new ForgeClient({ baseUrl: config.baseUrl })
    instanceBaseUrl = config.baseUrl
  }
  return instance
}

/** Reset the singleton. Only used by tests. */
export function resetClient(): void {
  instance = null
  instanceBaseUrl = null
}
```

- [ ] **Step 5.2: Commit**

```bash
git add packages/extension/src/background/client.ts
git commit -m "feat(ext): ForgeClient singleton keyed on config baseUrl"
```

---

## Task 6: SDK Error Mapping

**Files:**
- Modify: `packages/extension/src/lib/errors.ts`

- [ ] **Step 6.1: Add an SDK error mapper to errors.ts**

Append to `packages/extension/src/lib/errors.ts`:

```typescript
import type { ForgeError } from '@forge/sdk'

/**
 * Map an SDK ForgeError to an ExtensionError with a sensible code.
 * The SDK's `code` strings come from the Forge server; map the known ones
 * and default the rest to API_INTERNAL_ERROR.
 */
export function mapSdkError(
  err: ForgeError,
  opts: { url?: string; context?: Record<string, unknown> } = {},
): ExtensionError {
  const code: ExtensionErrorCode = (() => {
    switch (err.code) {
      case 'VALIDATION_FAILED':
      case 'INVALID_INPUT':
        return 'API_VALIDATION_FAILED'
      case 'NOT_FOUND':
        return 'API_NOT_FOUND'
      case 'DUPLICATE':
      case 'DUPLICATE_URL':
        return 'API_DUPLICATE'
      default:
        return 'API_INTERNAL_ERROR'
    }
  })()

  return extError(code, err.message, {
    layer: 'sdk',
    url: opts.url,
    context: { sdk_code: err.code, ...opts.context, details: err.details },
  })
}

/**
 * Map a network-level error (SDK request never reached the server) to an
 * ExtensionError. These have no ForgeError; they're raw exceptions from fetch.
 */
export function mapNetworkError(err: unknown, opts: { url?: string } = {}): ExtensionError {
  const message = err instanceof Error ? err.message : String(err)
  // Chrome surfaces CORS blocks as TypeError: Failed to fetch
  const code: ExtensionErrorCode = message.toLowerCase().includes('cors')
    ? 'API_CORS_BLOCKED'
    : 'API_UNREACHABLE'
  return extError(code, message, { layer: 'sdk', url: opts.url })
}
```

- [ ] **Step 6.2: Import types.ts from the SDK**

You may need to add a type import for `ForgeError` at the top:

```typescript
import type { ForgeError } from '@forge/sdk'
```

If the SDK's package.json doesn't export `ForgeError` as a type, add it to the SDK's exports first (this is a small edit to `packages/sdk/src/index.ts`) and commit separately. Run `bun install` to pick up the new export.

- [ ] **Step 6.3: Commit**

```bash
git add packages/extension/src/lib/errors.ts
git commit -m "feat(ext): map SDK errors to ExtensionError taxonomy"
```

---

## Task 7: Health Check Handler

**Files:**
- Create: `packages/extension/src/background/handlers/health.ts`

- [ ] **Step 7.1: Write the handler**

```typescript
// packages/extension/src/background/handlers/health.ts

import { getClient } from '../client'
import { mapSdkError, mapNetworkError } from '../../lib/errors'
import type { Response } from '../../lib/messaging'

export interface HealthPayload {
  server: string
  version: string
}

export async function handleHealth(): Promise<Response<HealthPayload>> {
  try {
    const client = await getClient()
    const result = await client.health()
    if (!result.ok) {
      return { ok: false, error: mapSdkError(result.error, { url: '/api/health' }) }
    }
    return { ok: true, data: result.data }
  } catch (err) {
    return { ok: false, error: mapNetworkError(err, { url: '/api/health' }) }
  }
}
```

- [ ] **Step 7.2: Commit**

```bash
git add packages/extension/src/background/handlers/health.ts
git commit -m "feat(ext): health check handler"
```

---

## Task 8: Orgs List Handler

**Files:**
- Create: `packages/extension/src/background/handlers/orgs.ts`

- [ ] **Step 8.1: Write the handler**

```typescript
// packages/extension/src/background/handlers/orgs.ts

import type { Organization } from '@forge/sdk'
import { getClient } from '../client'
import { mapSdkError, mapNetworkError } from '../../lib/errors'
import type { Response } from '../../lib/messaging'

export interface OrgsListPayload {
  orgs: Organization[]
  total: number
}

export async function handleOrgsList(limit = 20): Promise<Response<OrgsListPayload>> {
  try {
    const client = await getClient()
    const result = await client.organizations.list({ limit, offset: 0 })
    if (!result.ok) {
      return { ok: false, error: mapSdkError(result.error, { url: '/api/organizations' }) }
    }
    return {
      ok: true,
      data: { orgs: result.data, total: result.pagination.total },
    }
  } catch (err) {
    return { ok: false, error: mapNetworkError(err, { url: '/api/organizations' }) }
  }
}
```

- [ ] **Step 8.2: Commit**

```bash
git add packages/extension/src/background/handlers/orgs.ts
git commit -m "feat(ext): orgs.list handler"
```

---

## Task 9: Background Message Router

**Files:**
- Create: `packages/extension/src/background/index.ts`

- [ ] **Step 9.1: Write the router**

```typescript
// packages/extension/src/background/index.ts

import type { Command, Response } from '../lib/messaging'
import { handleHealth } from './handlers/health'
import { handleOrgsList } from './handlers/orgs'

chrome.runtime.onMessage.addListener(
  (msg: Command, _sender, sendResponse: (response: Response<unknown>) => void) => {
    // Route by cmd. Each handler is async, so we return true to keep the
    // channel open until sendResponse is called.
    ;(async () => {
      let response: Response<unknown>
      switch (msg.cmd) {
        case 'health':
          response = await handleHealth()
          break
        case 'orgs.list':
          response = await handleOrgsList(msg.limit)
          break
        default: {
          // Exhaustive check
          const _exhaustive: never = msg
          response = {
            ok: false,
            error: {
              code: 'UNKNOWN_ERROR',
              message: `Unknown command: ${JSON.stringify(_exhaustive)}`,
              layer: 'background',
              timestamp: new Date().toISOString(),
            },
          }
        }
      }
      sendResponse(response)
    })()
    return true  // async response
  },
)
```

- [ ] **Step 9.2: Commit**

```bash
git add packages/extension/src/background/index.ts
git commit -m "feat(ext): background message router with health + orgs.list"
```

---

## Task 10: Register Background Worker in Manifest

**Files:**
- Modify: `packages/extension/manifest.json`

- [ ] **Step 10.1: Add background service worker**

Update `packages/extension/manifest.json` to add the `background` key and additional permissions needed for SDK calls:

```json
{
  "manifest_version": 3,
  "name": "Forge",
  "version": "0.0.1",
  "description": "Capture job descriptions and autofill applications into Forge.",
  "action": {
    "default_popup": "src/popup/index.html",
    "default_title": "Forge"
  },
  "permissions": ["activeTab", "clipboardWrite", "storage"],
  "host_permissions": ["http://localhost:3000/*"],
  "background": {
    "service_worker": "src/background/index.ts",
    "type": "module"
  },
  "content_scripts": [
    {
      "matches": ["*://*.linkedin.com/*"],
      "js": ["src/content/linkedin.ts"],
      "run_at": "document_idle"
    }
  ]
}
```

Changes from P1:
- `permissions`: added `storage` (for `chrome.storage.local`)
- `host_permissions`: added `http://localhost:3000/*` (for SDK fetch)
- `background`: new section registering the service worker

- [ ] **Step 10.2: Rebuild**

```bash
cd packages/extension
bun run build
```

Expected: build succeeds, `dist/src/background/index.js` exists (or similar hashed name), manifest references it.

- [ ] **Step 10.3: Commit**

```bash
git add packages/extension/manifest.json
git commit -m "build(ext): register background worker and storage+localhost permissions"
```

---

## Task 11: Update Popup with Health Dot + Orgs List Button

**Files:**
- Modify: `packages/extension/src/popup/Popup.svelte`

- [ ] **Step 11.1: Replace `Popup.svelte` with the expanded version**

```svelte
<!-- packages/extension/src/popup/Popup.svelte -->
<script lang="ts">
  import { onMount } from 'svelte'
  import { sendCommand } from '../lib/messaging'
  import type { HealthPayload } from '../background/handlers/health'
  import type { OrgsListPayload } from '../background/handlers/orgs'

  type HealthState = 'unknown' | 'ok' | 'down'

  let healthState = $state<HealthState>('unknown')
  let healthVersion = $state<string | null>(null)

  let status = $state<string | null>(null)
  let statusKind = $state<'info' | 'ok' | 'err'>('info')

  let orgs = $state<Array<{ name: string }>>([])
  let orgsTotal = $state(0)

  onMount(async () => {
    await checkHealth()
  })

  async function checkHealth() {
    const response = await sendCommand<HealthPayload>({ cmd: 'health' })
    if (response.ok) {
      healthState = 'ok'
      healthVersion = response.data.version
    } else {
      healthState = 'down'
      healthVersion = null
    }
  }

  async function extract() {
    status = 'Extracting...'
    statusKind = 'info'
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
    if (!tab?.id) {
      status = 'No active tab'
      statusKind = 'err'
      return
    }
    try {
      const response = await chrome.tabs.sendMessage(tab.id, { cmd: 'extract' })
      if (response?.ok) {
        status = 'Extracted — see modal on page'
        statusKind = 'ok'
        setTimeout(() => window.close(), 800)
      } else if (response?.error?.code === 'EXTRACTION_EMPTY') {
        status = 'No job found on this page'
        statusKind = 'err'
      } else if (response?.error?.code === 'PLUGIN_THREW') {
        status = 'Plugin error — see console'
        statusKind = 'err'
      } else {
        status = 'Unknown response'
        statusKind = 'err'
      }
    } catch (err) {
      status = 'No plugin for this site yet'
      statusKind = 'err'
    }
  }

  async function listOrgs() {
    status = 'Loading orgs...'
    statusKind = 'info'
    const response = await sendCommand<OrgsListPayload>({ cmd: 'orgs.list', limit: 5 })
    if (response.ok) {
      orgs = response.data.orgs.map((o) => ({ name: o.name }))
      orgsTotal = response.data.total
      status = `Loaded ${orgs.length} of ${orgsTotal}`
      statusKind = 'ok'
    } else {
      const code = response.error.code
      if (code === 'API_UNREACHABLE') {
        status = 'Forge is not running'
      } else if (code === 'API_CORS_BLOCKED') {
        status = 'CORS blocked — check Forge config'
      } else {
        status = `Error: ${response.error.message}`
      }
      statusKind = 'err'
    }
  }
</script>

<main>
  <header>
    <h1>Forge</h1>
    <div class="health" class:ok={healthState === 'ok'} class:down={healthState === 'down'}>
      <span class="dot"></span>
      <span class="label">
        {#if healthState === 'ok' && healthVersion}v{healthVersion}
        {:else if healthState === 'down'}Offline
        {:else}…{/if}
      </span>
    </div>
  </header>

  <div class="buttons">
    <button onclick={extract}>Extract Job</button>
    <button onclick={listOrgs} class="secondary">List Organizations</button>
  </div>

  {#if status}
    <p class="status {statusKind}">{status}</p>
  {/if}

  {#if orgs.length > 0}
    <ul class="orgs">
      {#each orgs as org}
        <li>{org.name}</li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  main { padding: 16px; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
  h1 { margin: 0; font-size: 16px; color: #88f; }
  .health { display: flex; align-items: center; gap: 6px; font-size: 11px; color: #888; }
  .health .dot { width: 8px; height: 8px; border-radius: 50%; background: #666; }
  .health.ok .dot { background: #4a4; box-shadow: 0 0 4px #4a4; }
  .health.down .dot { background: #a44; box-shadow: 0 0 4px #a44; }
  .health.ok .label { color: #aca; }
  .health.down .label { color: #caa; }
  .buttons { display: flex; flex-direction: column; gap: 6px; }
  button { width: 100%; background: #335; color: #fff; border: 1px solid #557; padding: 10px 12px; border-radius: 4px; cursor: pointer; font-size: 14px; }
  button:hover { background: #446; }
  button.secondary { background: #233; border-color: #355; font-size: 12px; padding: 8px; }
  button.secondary:hover { background: #344; }
  .status { margin: 12px 0 0 0; padding: 8px; border-radius: 4px; font-size: 12px; }
  .status.info { background: #223; color: #aac; }
  .status.ok { background: #232; color: #afa; }
  .status.err { background: #322; color: #faa; }
  .orgs { margin: 12px 0 0 0; padding: 0; list-style: none; font-size: 11px; }
  .orgs li { padding: 4px 8px; background: #222; border-radius: 3px; margin-bottom: 2px; word-break: break-all; }
</style>
```

- [ ] **Step 11.2: Rebuild**

```bash
bun run build
```

Expected: build succeeds.

- [ ] **Step 11.3: Commit**

```bash
git add packages/extension/src/popup/Popup.svelte
git commit -m "feat(ext): popup with health dot and orgs list button"
```

---

## Task 12: Unit Test for Client Singleton

**Files:**
- Create: `packages/extension/tests/background/client.test.ts`

- [ ] **Step 12.1: Write the test**

```typescript
// packages/extension/tests/background/client.test.ts

import { describe, test, expect, beforeEach } from 'bun:test'
import { getClient, resetClient } from '../../src/background/client'

// Mock chrome.storage.local for this unit test
declare global {
  interface Window {}
}

const storage = new Map<string, unknown>()
;(globalThis as any).chrome = {
  storage: {
    local: {
      get: async (key: string) => {
        const value = storage.get(key)
        return value !== undefined ? { [key]: value } : {}
      },
      set: async (obj: Record<string, unknown>) => {
        for (const [k, v] of Object.entries(obj)) storage.set(k, v)
      },
    },
  },
}

describe('ForgeClient singleton', () => {
  beforeEach(() => {
    storage.clear()
    resetClient()
  })

  test('returns a ForgeClient instance', async () => {
    const client = await getClient()
    expect(client).toBeDefined()
    expect(typeof client.health).toBe('function')
  })

  test('returns the same instance on repeat calls with same config', async () => {
    const a = await getClient()
    const b = await getClient()
    expect(a).toBe(b)
  })

  test('re-instantiates when baseUrl changes', async () => {
    const a = await getClient()
    await (globalThis as any).chrome.storage.local.set({
      forge_ext_config: { baseUrl: 'http://localhost:9999', devMode: true, enabledPlugins: [] },
    })
    const b = await getClient()
    expect(a).not.toBe(b)
  })
})
```

- [ ] **Step 12.2: Run**

```bash
bun test tests/background/client.test.ts
```

Expected: all 3 tests PASS.

- [ ] **Step 12.3: Commit**

```bash
git add packages/extension/tests/background/client.test.ts
git commit -m "test(ext): ForgeClient singleton unit tests"
```

---

## Task 13: Smoke Test Against Running Forge Server

**Files:**
- Create: `packages/extension/tests/background/smoke.test.ts`

- [ ] **Step 13.1: Write the smoke test**

```typescript
// packages/extension/tests/background/smoke.test.ts
//
// This test requires a running Forge server at http://localhost:3000.
// Run `bun run --watch src/index.ts` in packages/core/ before running this test.
//
// Skipped automatically if the server isn't reachable.

import { describe, test, expect, beforeAll } from 'bun:test'
import { handleHealth } from '../../src/background/handlers/health'
import { handleOrgsList } from '../../src/background/handlers/orgs'
import { resetClient } from '../../src/background/client'

// Stub chrome.storage.local for the handlers
const storage = new Map<string, unknown>()
;(globalThis as any).chrome = {
  storage: {
    local: {
      get: async (key: string) => {
        const value = storage.get(key)
        return value !== undefined ? { [key]: value } : {}
      },
      set: async (obj: Record<string, unknown>) => {
        for (const [k, v] of Object.entries(obj)) storage.set(k, v)
      },
    },
  },
}

let serverReachable = false

beforeAll(async () => {
  resetClient()
  try {
    const res = await fetch('http://localhost:3000/api/health')
    serverReachable = res.ok
  } catch {
    serverReachable = false
  }
})

describe('smoke: extension <-> Forge roundtrip', () => {
  test('handleHealth returns ok against running server', async () => {
    if (!serverReachable) {
      console.warn('[smoke] Forge server not reachable — skipping. Start with: cd packages/core && bun run --watch src/index.ts')
      return
    }
    const response = await handleHealth()
    expect(response.ok).toBe(true)
    if (response.ok) {
      expect(response.data.server).toBe('ok')
      expect(typeof response.data.version).toBe('string')
    }
  })

  test('handleOrgsList returns a list against running server', async () => {
    if (!serverReachable) return
    const response = await handleOrgsList(5)
    expect(response.ok).toBe(true)
    if (response.ok) {
      expect(Array.isArray(response.data.orgs)).toBe(true)
      expect(typeof response.data.total).toBe('number')
    }
  })
})
```

- [ ] **Step 13.2: Start Forge in a separate terminal**

```bash
cd /Users/adam/notes/job-hunting/packages/core
FORGE_DB_PATH=/Users/adam/notes/job-hunting/data/forge.db bun run --watch src/index.ts
```

Wait until the server logs "Server ready" on port 3000.

- [ ] **Step 13.3: Run the smoke test**

From the worktree's `packages/extension/`:

```bash
bun test tests/background/smoke.test.ts
```

Expected: both tests PASS. If "Forge server not reachable" warning appears, the server isn't on port 3000 — check the dev command.

- [ ] **Step 13.4: Verify skip behavior when server is down**

Stop the Forge server (Ctrl-C). Re-run:

```bash
bun test tests/background/smoke.test.ts
```

Expected: tests **return** (no assertions fail) and the warning is printed. This confirms the skip-when-offline logic works.

- [ ] **Step 13.5: Commit**

```bash
git add packages/extension/tests/background/smoke.test.ts
git commit -m "test(ext): smoke test for extension<->Forge roundtrip (health + orgs.list)"
```

---

## Task 14: Manual Verification in Chrome

**Prerequisites:** Forge server running, extension built (`bun run build`).

- [ ] **Step 14.1: Reload the extension**

Start the Forge server (if not already running):

```bash
cd /Users/adam/notes/job-hunting/packages/core
FORGE_DB_PATH=/Users/adam/notes/job-hunting/data/forge.db bun run --watch src/index.ts
```

In Chrome:
1. Navigate to `chrome://extensions`
2. Find the Forge extension
3. Click the reload icon (circular arrow) — this picks up the new `dist/` output from P2's build

- [ ] **Step 14.2: Verify health dot — green case**

1. Click the Forge extension icon
2. Within 500ms, the health dot should turn **green** and show `v<server-version>`

- [ ] **Step 14.3: Verify health dot — red case**

1. Stop the Forge server (Ctrl-C in the terminal)
2. Close and reopen the popup
3. The health dot should turn **red** and show "Offline"

- [ ] **Step 14.4: Verify List Organizations — success case**

1. Restart the Forge server
2. Reopen the popup
3. Click "List Organizations"
4. Verify the status shows "Loaded N of M"
5. Verify up to 5 org names appear in the list

- [ ] **Step 14.5: Verify List Organizations — failure case**

1. Stop the Forge server
2. Click "List Organizations"
3. Verify status shows "Forge is not running"

- [ ] **Step 14.6: Verify P1 extraction still works**

1. Restart the Forge server
2. Navigate to a LinkedIn job page
3. Click the extension icon
4. Click "Extract Job"
5. Verify the debug modal still appears on the page
6. Verify the health dot is green

This confirms P2 didn't break P1.

- [ ] **Step 14.7: Record results**

Create `packages/extension/docs/p2-verification.md`:

```markdown
# P2 Verification Results

Captured: YYYY-MM-DD
Extension ID: <the-id>
Build: <git sha>

## Health check
- Server up → dot green, version displayed ✓
- Server down → dot red, "Offline" ✓

## List Organizations
- Server up → N orgs loaded, names displayed ✓
- Server down → "Forge is not running" ✓

## P1 regression check
- Extract Job still works on LinkedIn ✓

## Smoke test
- bun test tests/background/smoke.test.ts passes against running server ✓
- Skips gracefully when server is down ✓
```

---

## Task 15: Commit Verification and Merge

- [ ] **Step 15.1: Commit verification**

```bash
git add packages/extension/docs/p2-verification.md
git commit -m "$(cat <<'EOF'
docs(ext): P2 verification — API read-only sanity

Extension successfully reaches Forge via @forge/sdk through the background
service worker. Health check + orgs.list work against the real server.
Smoke test proves the popup -> background -> SDK -> API roundtrip.
CORS allowlist from P0 is verified working (no preflight failures).

Verification checklist (SPEC Section 4 P2):
- [x] Health dot green when Forge running, red when stopped
- [x] Org list returns same data as webui shows
- [x] Smoke test passes: full popup->background->SDK->API roundtrip
- [x] Red-state: stop Forge mid-session -> user-friendly API_UNREACHABLE toast
- [x] P1 extraction still works (no regression)

Blocks unblocked: P3 (API write simple)

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 15.2: Merge to main**

```bash
cd /Users/adam/notes/job-hunting
git merge --no-ff feat/forge-ext/p2-api-read -m "Merge P2 (extension API read-only sanity)"
```

- [ ] **Step 15.3: Run all tests from main**

```bash
cd packages/extension
bun test
```

Expected: all tests pass. Smoke test skips if Forge isn't running.

- [ ] **Step 15.4: Remove worktree**

```bash
cd /Users/adam/notes/job-hunting
git worktree remove .claude/worktrees/forge-ext-p2-api-read
```

---

## Done When

- [ ] Background service worker registered in manifest, owns `ForgeClient` singleton
- [ ] `@forge/sdk` is a workspace dependency of `packages/extension/`
- [ ] Typed message contract (`Command`, `Response<T>`, `sendCommand`) in `src/lib/messaging.ts`
- [ ] `chrome.storage.local` config wrapper in `src/storage/config.ts`
- [ ] Health check handler reachable via `{ cmd: 'health' }` message
- [ ] Orgs list handler reachable via `{ cmd: 'orgs.list' }` message
- [ ] SDK errors mapped to `ExtensionError` taxonomy (`mapSdkError`, `mapNetworkError`)
- [ ] Popup shows health dot (green/red) and List Organizations button
- [ ] Client singleton unit tests pass (3 tests)
- [ ] Smoke test against running Forge server passes (2 tests)
- [ ] **Extension compiles cleanly: `bun run build` succeeds with no errors**
- [ ] **Extension is dev-installable: user can load `packages/extension/dist/` as an unpacked extension in Chrome without errors (fresh install or reload-over-P1)**
- [ ] Manual verification in Chrome: health dot, org list, P1 regression, all pass
- [ ] Verification log committed
- [ ] Merge commit on main with filled checklist
- [ ] Worktree removed; P3 is unblocked
