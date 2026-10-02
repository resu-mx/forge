# JD Lookup-by-URL Endpoint — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `POST /api/job-descriptions/lookup-by-url` — given an exact URL, return the matching JD (hydrated with org name) or 404. This enables the browser extension (P5) to check if a JD already exists before ingesting it.

**Architecture:** New service method `lookupByUrl(url)` uses `elm.list('job_descriptions', { where: { url } })` with limit 1. Route delegates to service, returns `{ data }` or `{ error }` with 404. SDK adds `jobDescriptions.lookupByUrl(url)` method.

**Tech Stack:** Bun, Hono, ELM (EntityLifecycleManager), `@forge/sdk`

**Worktree:** `.claude/worktrees/forge-core-jd-lookup-by-url/` on branch `feat/forge-core/jd-lookup-by-url`

---

### Task 1: Service — `lookupByUrl` method

**Files:**
- Modify: `packages/core/src/services/job-description-service.ts` (add method after `delete`)

- [ ] **Step 1: Write the failing test**

File: `packages/core/src/services/__tests__/job-description-service.test.ts` (or add to existing route tests — see Task 2 for route-level tests which are sufficient here since the service is thin)

Skip isolated service unit test — the method is a 5-line ELM delegation. Route-level integration tests (Task 2) cover it fully.

- [ ] **Step 2: Add `lookupByUrl` to the service**

In `packages/core/src/services/job-description-service.ts`, add after the `delete` method:

```typescript
async lookupByUrl(url: string): Promise<Result<JobDescriptionWithOrg>> {
  if (!url || url.trim().length === 0) {
    return {
      ok: false,
      error: { code: 'VALIDATION_ERROR', message: 'URL must not be empty' },
    }
  }

  const listResult = await this.elm.list('job_descriptions', {
    where: { url },
    limit: 1,
  })

  if (!listResult.ok) {
    return { ok: false, error: storageErrorToForgeError(listResult.error) }
  }

  if (listResult.value.rows.length === 0) {
    return {
      ok: false,
      error: { code: 'NOT_FOUND', message: `No job description found for URL: ${url}` },
    }
  }

  const jd = listResult.value.rows[0] as unknown as JobDescription
  const hydrated = await this.toJDWithOrg(jd)
  return { ok: true, data: hydrated }
}
```

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/services/job-description-service.ts
git commit -m "feat(core): add lookupByUrl method to JobDescriptionService"
```

---

### Task 2: Route — `POST /job-descriptions/lookup-by-url`

**Files:**
- Modify: `packages/core/src/routes/job-descriptions.ts` (add route)
- Test: `packages/core/src/routes/__tests__/job-descriptions.test.ts` (add tests)

- [ ] **Step 1: Write failing tests**

Add to `packages/core/src/routes/__tests__/job-descriptions.test.ts` inside the `describe('Job Description Routes', ...)` block:

```typescript
// ── POST /job-descriptions/lookup-by-url ──────────────────────────────

test('POST lookup-by-url returns JD when URL matches', async () => {
  seedJobDescription(ctx.db, {
    title: 'ML Engineer',
    url: 'https://example.com/jobs/ml-eng',
    rawText: 'Build ML systems',
  })

  const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {
    url: 'https://example.com/jobs/ml-eng',
  })
  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data.title).toBe('ML Engineer')
  expect(body.data.url).toBe('https://example.com/jobs/ml-eng')
})

test('POST lookup-by-url returns 404 when URL not found', async () => {
  const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {
    url: 'https://example.com/jobs/nonexistent',
  })
  expect(res.status).toBe(404)
  const body = await res.json()
  expect(body.error.code).toBe('NOT_FOUND')
})

test('POST lookup-by-url returns 400 for empty URL', async () => {
  const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {
    url: '',
  })
  expect(res.status).toBe(400)
  const body = await res.json()
  expect(body.error.code).toBe('VALIDATION_ERROR')
})

test('POST lookup-by-url returns 400 for missing URL', async () => {
  const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {})
  expect(res.status).toBe(400)
})

test('POST lookup-by-url hydrates organization_name', async () => {
  const orgId = seedOrganization(ctx.db, { name: 'Anthropic' })
  seedJobDescription(ctx.db, {
    title: 'SWE',
    url: 'https://anthropic.com/careers/swe',
    organizationId: orgId,
  })

  const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/lookup-by-url', {
    url: 'https://anthropic.com/careers/swe',
  })
  expect(res.status).toBe(200)
  const body = await res.json()
  expect(body.data.organization_name).toBe('Anthropic')
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd .claude/worktrees/forge-core-jd-lookup-by-url && bun test packages/core/src/routes/__tests__/job-descriptions.test.ts`
Expected: 5 new tests fail (route doesn't exist yet)

- [ ] **Step 3: Add the route**

In `packages/core/src/routes/job-descriptions.ts`, add **before** the `app.get('/job-descriptions/:id', ...)` route (ordering matters — `:id` would swallow `lookup-by-url` if it came first):

```typescript
// Lookup by URL — must come before :id route
app.post('/job-descriptions/lookup-by-url', async (c) => {
  const body = await c.req.json()
  const url = body?.url
  if (!url || typeof url !== 'string') {
    return c.json(
      { error: { code: 'VALIDATION_ERROR', message: 'URL must be a non-empty string' } },
      400,
    )
  }
  const result = await services.jobDescriptions.lookupByUrl(url)
  if (!result.ok) {
    return c.json({ error: result.error }, mapStatusCode(result.error.code))
  }
  return c.json({ data: result.data })
})
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd .claude/worktrees/forge-core-jd-lookup-by-url && bun test packages/core/src/routes/__tests__/job-descriptions.test.ts`
Expected: All tests pass (existing + 5 new)

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/routes/job-descriptions.ts packages/core/src/routes/__tests__/job-descriptions.test.ts
git commit -m "feat(core): POST /job-descriptions/lookup-by-url route + tests"
```

---

### Task 3: SDK — `jobDescriptions.lookupByUrl(url)`

**Files:**
- Modify: `packages/sdk/src/resources/job-descriptions.ts` (add method)

- [ ] **Step 1: Add the SDK method**

In `packages/sdk/src/resources/job-descriptions.ts`, add after the `delete` method:

```typescript
lookupByUrl(url: string): Promise<Result<JobDescriptionWithOrg>> {
  return this.request<JobDescriptionWithOrg>(
    'POST',
    '/api/job-descriptions/lookup-by-url',
    { url },
  )
}
```

- [ ] **Step 2: Verify SDK builds**

Run: `cd .claude/worktrees/forge-core-jd-lookup-by-url && cd packages/sdk && bun run build` (or `bun tsc --noEmit` if no build script)
Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add packages/sdk/src/resources/job-descriptions.ts
git commit -m "feat(sdk): add jobDescriptions.lookupByUrl(url) method"
```

---

### Task 4: Full integration verification

- [ ] **Step 1: Run all core tests**

Run: `cd .claude/worktrees/forge-core-jd-lookup-by-url && bun test packages/core/`
Expected: All tests pass, no regressions

- [ ] **Step 2: Run all SDK tests (if any)**

Run: `cd .claude/worktrees/forge-core-jd-lookup-by-url && bun test packages/sdk/` (if test files exist)
Expected: Pass or no test files

- [ ] **Step 3: Final commit (if any cleanup needed), otherwise done**

Branch `feat/forge-core/jd-lookup-by-url` is ready to merge.
