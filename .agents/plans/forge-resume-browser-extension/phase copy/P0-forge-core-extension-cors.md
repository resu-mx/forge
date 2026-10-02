# P0 — Forge Core: Extension CORS Allowlist

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `chrome-extension://` origin to the Forge CORS allowlist so the browser extension can make authenticated-origin requests to `http://localhost:3000/api/*`.

**Architecture:** Hono's `cors()` middleware currently accepts a fixed array of origins. Change to a function-based origin check that accepts the existing localhost origins plus any `chrome-extension://*` origin in development mode. Production remains unchanged (same-origin).

**Tech Stack:** Hono v4, Bun, `bun:test`, `hono/cors` middleware.

**Worktree:** `.claude/worktrees/forge-core-extension-cors/` on branch `feat/forge-core/extension-cors`.

**Blocks:** P2 (API read-only sanity) — extension cannot make cross-origin requests to Forge without this.

---

## Context You Need

**Current CORS configuration** (`packages/core/src/routes/server.ts:62-69`):

```typescript
app.use(
  '*',
  cors({
    origin: process.env.NODE_ENV === 'production'
      ? '*'  // same-origin in production (served from same server)
      : ['http://localhost:5173', 'http://127.0.0.1:5173'],
  }),
)
```

**Why a function-based check is needed:** Chrome extensions have origins like `chrome-extension://abcdef1234567890abcdef1234567890`. The ID is either (a) auto-generated when loading an unpacked extension or (b) deterministic if the manifest includes a `key` field. We can't enumerate valid extension IDs in an array, so we check the protocol prefix.

**Hono cors `origin` options**: per `hono/cors` docs, `origin` accepts `string | string[] | (origin: string) => string | null | undefined`. The function receives the request's `Origin` header and returns the origin to echo back (or `null`/`undefined` to reject).

**Test helpers**: `packages/core/src/routes/__tests__/helpers.ts` exports `createTestApp()` and `apiRequest()`. Existing test at `packages/core/src/routes/__tests__/server.test.ts` shows the pattern.

---

## File Structure

- **Modify**: `packages/core/src/routes/server.ts` (lines 62-69 — CORS middleware)
- **Create**: `packages/core/src/routes/__tests__/cors.test.ts` (new test file for CORS behavior)

---

## Task 1: Worktree Setup

- [ ] **Step 1.1: Create worktree**

```bash
cd /Users/adam/notes/job-hunting
git worktree add .claude/worktrees/forge-core-extension-cors -b feat/forge-core/extension-cors
cd .claude/worktrees/forge-core-extension-cors
```

- [ ] **Step 1.2: Install deps (workspace-level)**

Run from worktree root:

```bash
bun install
```

Expected: `Done` with no errors. If dependencies are missing, investigate before continuing.

---

## Task 2: Write Failing CORS Test

**Files:**
- Create: `packages/core/src/routes/__tests__/cors.test.ts`

- [ ] **Step 2.1: Create test file with four test cases**

```typescript
// packages/core/src/routes/__tests__/cors.test.ts

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, type TestContext } from './helpers'

describe('CORS', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  test('allows localhost:5173 origin (webui)', async () => {
    const res = await ctx.app.request('/api/health', {
      method: 'GET',
      headers: { Origin: 'http://localhost:5173' },
    })
    expect(res.status).toBe(200)
    expect(res.headers.get('access-control-allow-origin')).toBe('http://localhost:5173')
  })

  test('allows chrome-extension://<id> origin', async () => {
    const extOrigin = 'chrome-extension://abcdef1234567890abcdef1234567890'
    const res = await ctx.app.request('/api/health', {
      method: 'GET',
      headers: { Origin: extOrigin },
    })
    expect(res.status).toBe(200)
    expect(res.headers.get('access-control-allow-origin')).toBe(extOrigin)
  })

  test('allows chrome-extension://<id> preflight', async () => {
    const extOrigin = 'chrome-extension://abcdef1234567890abcdef1234567890'
    const res = await ctx.app.request('/api/organizations', {
      method: 'OPTIONS',
      headers: {
        Origin: extOrigin,
        'Access-Control-Request-Method': 'POST',
        'Access-Control-Request-Headers': 'content-type',
      },
    })
    expect(res.status).toBe(204)
    expect(res.headers.get('access-control-allow-origin')).toBe(extOrigin)
    expect(res.headers.get('access-control-allow-methods')).toContain('POST')
  })

  test('rejects unknown cross-origin', async () => {
    const res = await ctx.app.request('/api/health', {
      method: 'GET',
      headers: { Origin: 'http://evil.example.com' },
    })
    // Hono cors() without matching origin sends request through but omits
    // the Allow-Origin header, so browser blocks it client-side.
    expect(res.status).toBe(200)
    expect(res.headers.get('access-control-allow-origin')).toBeNull()
  })
})
```

- [ ] **Step 2.2: Run test to confirm it fails**

From `packages/core/`:

```bash
bun test src/routes/__tests__/cors.test.ts
```

Expected:
- `allows localhost:5173 origin (webui)` — **PASS** (already works)
- `rejects unknown cross-origin` — **PASS** (already works)
- `allows chrome-extension://<id> origin` — **FAIL** (header is null)
- `allows chrome-extension://<id> preflight` — **FAIL** (header is null)

If all four pass on the first run, stop and investigate: CORS already supports extensions somehow, and this task is unnecessary.

---

## Task 3: Implement Function-Based Origin Check

**Files:**
- Modify: `packages/core/src/routes/server.ts` (lines 62-69)

- [ ] **Step 3.1: Replace the CORS middleware configuration**

In `packages/core/src/routes/server.ts`, find:

```typescript
app.use(
  '*',
  cors({
    origin: process.env.NODE_ENV === 'production'
      ? '*'  // same-origin in production (served from same server)
      : ['http://localhost:5173', 'http://127.0.0.1:5173'],
  }),
)
```

Replace with:

```typescript
app.use(
  '*',
  cors({
    origin: (origin) => {
      // Production: same-origin (served from same server)
      if (process.env.NODE_ENV === 'production') {
        return '*'
      }
      // Development: allow webui + Chrome/Firefox extension origins
      if (!origin) return undefined
      const allowedOrigins = ['http://localhost:5173', 'http://127.0.0.1:5173']
      if (allowedOrigins.includes(origin)) return origin
      // Browser extension origins: chrome-extension:// or moz-extension://
      if (origin.startsWith('chrome-extension://') || origin.startsWith('moz-extension://')) {
        return origin
      }
      return undefined
    },
  }),
)
```

**Why this shape:** The function receives the `Origin` header value and returns the value to echo back in `Access-Control-Allow-Origin`. Returning `undefined` omits the header, which causes the browser to reject the response (request still reaches the server but the browser discards the result client-side). `moz-extension://` is included now to reduce churn when Firefox support lands in MVP.

- [ ] **Step 3.2: Run the test suite to confirm all four pass**

From `packages/core/`:

```bash
bun test src/routes/__tests__/cors.test.ts
```

Expected: all 4 tests PASS.

- [ ] **Step 3.3: Run the full core test suite to check for regressions**

```bash
bun test
```

Expected: all tests pass. If any unrelated tests fail, investigate — the CORS change should be backwards-compatible.

---

## Task 4: Manual Verification Against Running Server

- [ ] **Step 4.1: Start the Forge server**

In a separate terminal, from the worktree root:

```bash
cd packages/core
FORGE_DB_PATH=/Users/adam/notes/job-hunting/data/forge.db bun run --watch src/index.ts
```

Expected: server starts on port 3000, logs "Server ready".

- [ ] **Step 4.2: Curl the health endpoint with a chrome-extension origin header**

In another terminal:

```bash
curl -v -H 'Origin: chrome-extension://abcdef1234567890abcdef1234567890' http://localhost:3000/api/health
```

Expected response headers include:
```
< access-control-allow-origin: chrome-extension://abcdef1234567890abcdef1234567890
```

And body:
```json
{"data":{"server":"ok","version":"..."}}
```

- [ ] **Step 4.3: Curl with an unknown origin to verify rejection**

```bash
curl -v -H 'Origin: http://evil.example.com' http://localhost:3000/api/health
```

Expected: response has no `access-control-allow-origin` header. The body still returns the health data (CORS is browser-enforced, not server-enforced) — this is correct behavior.

- [ ] **Step 4.4: Preflight check**

```bash
curl -v -X OPTIONS \
  -H 'Origin: chrome-extension://abcdef1234567890abcdef1234567890' \
  -H 'Access-Control-Request-Method: POST' \
  -H 'Access-Control-Request-Headers: content-type' \
  http://localhost:3000/api/organizations
```

Expected: 204 No Content with headers:
```
< access-control-allow-origin: chrome-extension://abcdef1234567890abcdef1234567890
< access-control-allow-methods: POST, ...
```

- [ ] **Step 4.5: Stop the dev server**

Ctrl-C in the server terminal.

---

## Task 5: Commit with Verification Checklist

- [ ] **Step 5.1: Stage and commit**

From the worktree root:

```bash
git add packages/core/src/routes/server.ts packages/core/src/routes/__tests__/cors.test.ts
git commit -m "$(cat <<'EOF'
feat(core): allow chrome-extension and moz-extension origins in dev CORS

Browser extension (packages/extension, planned) needs to make cross-origin
requests to the Forge API at http://localhost:3000. Extensions have origins
of the form chrome-extension://<id> or moz-extension://<id>, which cannot
be enumerated in a static array, so replace the array-based origin config
with a function that checks the protocol prefix.

Production behavior is unchanged (same-origin).

Verification checklist:
- [x] CORS test suite passes (4 cases: localhost allow, chrome-ext allow,
      chrome-ext preflight, unknown-origin reject)
- [x] Full core test suite passes (no regressions)
- [x] Manual curl with chrome-extension://<id> Origin returns
      access-control-allow-origin header echoing the origin
- [x] Manual curl with http://evil.example.com Origin omits the
      access-control-allow-origin header
- [x] OPTIONS preflight returns 204 with correct allow-methods and
      allow-origin headers

Blocks unblocked: P2 (extension API read-only sanity prototype)

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 5.2: Verify commit was created**

```bash
git log -1 --stat
```

Expected output shows one commit modifying `server.ts` and creating `cors.test.ts`.

---

## Task 6: Merge Back to Main

**When to do this:** After user review and approval of the commit.

- [ ] **Step 6.1: Switch to main worktree and merge**

```bash
cd /Users/adam/notes/job-hunting
git merge --no-ff feat/forge-core/extension-cors -m "Merge forge-core extension CORS support"
```

- [ ] **Step 6.2: Clean up worktree**

```bash
git worktree remove .claude/worktrees/forge-core-extension-cors
```

The branch `feat/forge-core/extension-cors` stays for history.

- [ ] **Step 6.3: Run full test suite from main**

```bash
cd packages/core && bun test
```

Expected: all tests pass.

---

## Done When

- [ ] CORS test file exists with 4 passing tests
- [ ] `server.ts` CORS middleware uses function-based origin check
- [ ] Manual curl verification confirms `chrome-extension://<id>` Origin is accepted
- [ ] Manual curl verification confirms unknown origins are omitted
- [ ] Commit landed on main with filled verification checklist
- [ ] Worktree removed
- [ ] P2 is unblocked
