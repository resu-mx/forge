# P3: Extension API Write (Simple) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove the extension write path works end-to-end by adding a "Create Test Org" button that creates an organization in Forge via the background worker + SDK.

**Architecture:** Add `orgs.create` command to the messaging contract, handler in the background worker delegating to `sdk.organizations.create()`, button in the popup with toast feedback. Follows the exact same pattern as the existing `orgs.list` command.

**Tech Stack:** Chrome MV3 extension, Svelte 5 popup, `@forge/sdk`, Bun test runner

**Worktree:** `.claude/worktrees/forge-ext-p3-api-write/` on branch `feat/forge-ext/p3-api-write`

**SPEC reference:** SPEC.md §4 P3

---

### File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Modify | `src/lib/messaging.ts` | Add `orgs.create` to Command union |
| Modify | `src/background/handlers/orgs.ts` | Add `handleOrgsCreate` handler + payload type |
| Modify | `src/background/index.ts` | Add `orgs.create` case to switch |
| Modify | `src/popup/Popup.svelte` | Add "Create Test Org" button + toast |
| Modify | `tests/background/smoke.test.ts` | Add smoke test for write path |

All paths relative to `packages/extension/`.

---

### Task 1: Messaging contract — add `orgs.create` command

**Files:**
- Modify: `packages/extension/src/lib/messaging.ts`

- [ ] **Step 1: Add the command variant**

In `packages/extension/src/lib/messaging.ts`, extend the `Command` type union:

```typescript
export type Command =
  | { cmd: 'health' }
  | { cmd: 'orgs.list'; limit?: number }
  | { cmd: 'orgs.create'; payload: { name: string } }
```

- [ ] **Step 2: Commit**

```bash
git add packages/extension/src/lib/messaging.ts
git commit -m "feat(ext): add orgs.create command to messaging contract"
```

---

### Task 2: Background handler — `handleOrgsCreate`

**Files:**
- Modify: `packages/extension/src/background/handlers/orgs.ts` (add handler + export payload type)
- Modify: `packages/extension/src/background/index.ts` (add switch case)

- [ ] **Step 1: Add handler to orgs.ts**

In `packages/extension/src/background/handlers/orgs.ts`, add the handler and payload type after the existing `handleOrgsList`:

```typescript
export interface OrgsCreatePayload {
  id: string
  name: string
}

export async function handleOrgsCreate(payload: { name: string }): Promise<Response<OrgsCreatePayload>> {
  try {
    const client = await getClient()
    const result = await client.organizations.create({ name: payload.name })
    if (!result.ok) {
      return { ok: false, error: mapSdkError(result.error, { url: '/api/organizations' }) }
    }
    return {
      ok: true,
      data: { id: result.data.id, name: result.data.name },
    }
  } catch (err) {
    return { ok: false, error: mapNetworkError(err, { url: '/api/organizations' }) }
  }
}
```

- [ ] **Step 2: Add switch case to background/index.ts**

In `packages/extension/src/background/index.ts`, add the import and case. Add `handleOrgsCreate` to the import from `./handlers/orgs`:

```typescript
import { handleOrgsList, handleOrgsCreate } from './handlers/orgs'
```

Add the case in the switch before `default`:

```typescript
case 'orgs.create':
  response = await handleOrgsCreate(msg.payload)
  break
```

- [ ] **Step 3: Verify TypeScript compiles**

The exhaustive switch check (`const _exhaustive: never = msg`) will fail if the case is missing. Adding the case ensures compilation. Verify with:

Run: `cd packages/extension && npx tsc --noEmit` (or `bun tsc --noEmit`)

- [ ] **Step 4: Commit**

```bash
git add packages/extension/src/background/handlers/orgs.ts packages/extension/src/background/index.ts
git commit -m "feat(ext): orgs.create background handler with SDK delegation"
```

---

### Task 3: Smoke test — write path

**Files:**
- Modify: `packages/extension/tests/background/smoke.test.ts`

- [ ] **Step 1: Write the smoke test**

Add to `packages/extension/tests/background/smoke.test.ts`, import `handleOrgsCreate` and add a test inside the existing describe block:

Update import:
```typescript
import { handleOrgsList, handleOrgsCreate } from '../../src/background/handlers/orgs'
```

Add test:
```typescript
test('handleOrgsCreate creates an org against running server', async () => {
  if (!serverReachable) return
  const name = `Test Org ${Date.now()}`
  const response = await handleOrgsCreate({ name })
  expect(response.ok).toBe(true)
  if (response.ok) {
    expect(response.data.name).toBe(name)
    expect(typeof response.data.id).toBe('string')
    expect(response.data.id).toHaveLength(36)
  }
})
```

- [ ] **Step 2: Run smoke tests (requires Forge server running)**

Run: `cd packages/extension && bun test tests/background/smoke.test.ts`
Expected: All 3 smoke tests pass (health, orgs.list, orgs.create) if server is running, or skip gracefully if not.

- [ ] **Step 3: Commit**

```bash
git add packages/extension/tests/background/smoke.test.ts
git commit -m "test(ext): smoke test for orgs.create write path"
```

---

### Task 4: Popup UI — "Create Test Org" button

**Files:**
- Modify: `packages/extension/src/popup/Popup.svelte`

- [ ] **Step 1: Add the createTestOrg function**

In the `<script>` block, add import for `OrgsCreatePayload`:

```typescript
import type { OrgsCreatePayload } from '../background/handlers/orgs'
```

Update the existing import line to include it (it already imports `OrgsListPayload` from the same module).

Add the function after `listOrgs()`:

```typescript
async function createTestOrg() {
  const name = `Test Org ${Date.now()}`
  status = `Creating "${name}"...`
  statusKind = 'info'
  const response = await sendCommand<OrgsCreatePayload>({ cmd: 'orgs.create', payload: { name } })
  if (response.ok) {
    status = `Created org ${response.data.id.slice(0, 8)}...`
    statusKind = 'ok'
  } else {
    const code = response.error.code
    if (code === 'API_UNREACHABLE') {
      status = 'Forge is not running'
    } else if (code === 'API_VALIDATION_FAILED') {
      status = 'Validation error — check payload'
    } else {
      status = `Error: ${response.error.message}`
    }
    statusKind = 'err'
  }
}
```

- [ ] **Step 2: Add the button**

In the `<div class="buttons">` block, add after the "List Organizations" button:

```svelte
<button onclick={createTestOrg} class="secondary">Create Test Org</button>
```

- [ ] **Step 3: Build and verify**

Run: `cd packages/extension && bun run build`
Expected: Build succeeds, `dist/` contains updated popup.

- [ ] **Step 4: Commit**

```bash
git add packages/extension/src/popup/Popup.svelte
git commit -m "feat(ext): Create Test Org button in popup with toast feedback"
```

---

### Task 5: Build verification + manual checklist

- [ ] **Step 1: Full build**

Run: `cd packages/extension && bun run build`
Expected: Clean build, no errors.

- [ ] **Step 2: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: All plugin tests + client tests pass. Smoke tests pass if server running.

- [ ] **Step 3: Verification checklist (human sign-off required)**

These require manual testing with the installed extension:

- [ ] `bun run build` succeeds
- [ ] Extension loads in Chrome via "Load unpacked" → `dist/` (no manifest/SW errors)
- [ ] Click "Create Test Org" → toast shows created org ID
- [ ] Created org visible in Forge webui at `/organizations`
- [ ] Stop Forge server → click "Create Test Org" → toast shows "Forge is not running"
- [ ] Health dot correctly reflects server state (green/red)

Branch `feat/forge-ext/p3-api-write` is ready for merge after manual verification passes.
