# Phase 5: SDK

**Goal:** Implement the typed API client (`@forge/sdk`) that all consumers use.

**Non-Goals:** No UI, no CLI logic. Just the HTTP client wrapper.

**Depends on:** Phase 1 (types — can start building against types), Phase 4 (API — for integration testing)
**Blocks:** Phase 6 (CLI), Phase 7 (WebUI)

**Note:** SDK development can start in parallel with Phases 2-4 by building against the type definitions and API spec. Integration testing requires a running API (Phase 4).

---

## Task 5.1: ForgeClient Base

**File:** `packages/sdk/src/client.ts`

**Steps:**
1. Create `ForgeClient` class with configurable `baseUrl`
2. Implement private `request<T>(method, path, body?): Promise<Result<T>>` helper
3. Handle JSON serialization/deserialization
4. Catch network errors → `{ ok: false, error: { code: 'NETWORK_ERROR' } }`
5. Parse error envelope responses into ForgeError
6. Handle pagination responses

```typescript
export class ForgeClient {
  private baseUrl: string

  constructor(options: { baseUrl: string }) {
    // Normalize trailing slash
    this.baseUrl = options.baseUrl.replace(/\/$/, '')
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<Result<T>> {
    try {
      const response = await fetch(`${this.baseUrl}${path}`, {
        method,
        headers: body ? { 'Content-Type': 'application/json' } : {},
        body: body ? JSON.stringify(body) : undefined,
      })

      if (response.status === 204) return { ok: true, data: undefined as T }

      const json = await response.json()

      if (!response.ok) {
        return { ok: false, error: json.error as ForgeError }
      }

      return { ok: true, data: json.data as T }
    } catch (err) {
      return {
        ok: false,
        error: { code: 'NETWORK_ERROR', message: String(err) }
      }
    }
  }
}
```

**Two internal request helpers:**
- `request<T>(method, path, body?)` — for single-entity endpoints. Returns `Result<T>`. Unwraps `json.data`.
- `requestList<T>(method, path, params?)` — for list endpoints. Returns `PaginatedResult<T>`. Unwraps `json.data` AND `json.pagination`.

See `refs/contracts/result-type.md` for the implementation pattern.

**Acceptance Criteria:**
- [ ] Base URL normalization (trailing slash handling)
- [ ] `request<T>` unwraps single-entity responses into `{ ok: true, data: T }`
- [ ] `requestList<T>` unwraps list responses into `{ ok: true, data: T[], pagination }`
- [ ] Error responses unwrapped into `{ ok: false, error: ForgeError }`
- [ ] Network failures caught and returned as NETWORK_ERROR
- [ ] 204 responses handled (no body)
- [ ] Unknown HTTP status codes (e.g., 503) mapped to UNKNOWN_ERROR
- [ ] Uses only `fetch` — no Bun/Node-specific APIs (must work in browser)
- [ ] All API paths prefixed with `/api/` (matching Phase 4 prefix)

**Testing:**
- Unit: Mock fetch, verify request construction (method, headers, body)
- Unit: Mock successful response → ok: true
- Unit: Mock error response → ok: false with correct code
- Unit: Mock network failure → NETWORK_ERROR
- Unit: Trailing slash normalization
- Contract: SDK uses only `fetch` (grep for disallowed imports)

---

## Task 5.2: Resource Clients

**Files:** All within `packages/sdk/src/client.ts` (or split into `resources/` directory)

**Implement resource namespaces on ForgeClient:**

### sources
- `create(input: CreateSource): Promise<Result<Source>>`
- `list(filter?: SourceFilter & PaginationParams): Promise<PaginatedResult<Source>>`
- `get(id: string): Promise<Result<SourceWithBullets>>`
- `update(id: string, input: UpdateSource): Promise<Result<Source>>`
- `delete(id: string): Promise<Result<void>>`
- `deriveBullets(id: string): Promise<Result<Bullet[]>>`

### bullets
- `list(filter?: BulletFilter & PaginationParams): Promise<PaginatedResult<Bullet>>`
- `get(id: string): Promise<Result<BulletWithRelations>>`
- `update(id: string, input: UpdateBullet): Promise<Result<Bullet>>`
- `delete(id: string): Promise<Result<void>>`
- `approve(id: string): Promise<Result<Bullet>>`
- `reject(id: string, input: RejectInput): Promise<Result<Bullet>>`
- `reopen(id: string): Promise<Result<Bullet>>`
- `derivePerspectives(id: string, input: DerivePerspectiveInput): Promise<Result<Perspective>>`

### perspectives
- `list(filter?): Promise<PaginatedResult<Perspective>>`
- `get(id: string): Promise<Result<PerspectiveWithChain>>`
- `update(id, input): Promise<Result<Perspective>>`
- `delete(id): Promise<Result<void>>`
- `approve(id): Promise<Result<Perspective>>`
- `reject(id, input: RejectInput): Promise<Result<Perspective>>`
- `reopen(id): Promise<Result<Perspective>>`

### resumes
- `create(input: CreateResume): Promise<Result<Resume>>`
- `list(pagination?): Promise<PaginatedResult<Resume>>`
- `get(id): Promise<Result<ResumeWithPerspectives>>`
- `update(id, input): Promise<Result<Resume>>`
- `delete(id): Promise<Result<void>>`
- `addPerspective(resumeId, input: AddResumePerspective): Promise<Result<void>>`
- `removePerspective(resumeId, perspectiveId): Promise<Result<void>>`
- `reorderPerspectives(resumeId, input: ReorderPerspectives): Promise<Result<void>>`
- `gaps(id): Promise<Result<GapAnalysis>>`
- `export(id): Promise<Result<never>>` (always returns NOT_IMPLEMENTED for MVP)

### review
- `pending(): Promise<Result<ReviewQueue>>`

**Acceptance Criteria:**
- [ ] Every API endpoint has a corresponding SDK method
- [ ] Method signatures match the type definitions from Phase 1
- [ ] Filter params correctly serialized as query string params
- [ ] Pagination params (offset, limit) passed through
- [ ] Reject methods require RejectInput with rejection_reason

**Testing:**
- Unit (mocked fetch): Every method constructs the correct HTTP request
- Unit (mocked fetch): Every method handles success and error responses correctly
- Integration (against running API): Full CRUD cycle for each resource
- Integration: deriveBullets returns created bullets
- Integration: gaps returns structured gap report

---

## Task 5.3: SDK Package Exports

**File:** `packages/sdk/src/index.ts`

**Steps:**
1. Re-export `ForgeClient`
2. Re-export all types (Result, ForgeError, entity types, input types)
3. Verify `package.json` has correct `main`, `types`, `exports` fields

```json
{
  "name": "@forge/sdk",
  "main": "src/index.ts",
  "types": "src/index.ts",
  "exports": {
    ".": "./src/index.ts"
  }
}
```

**Acceptance Criteria:**
- [ ] `import { ForgeClient, type Source } from '@forge/sdk'` works from CLI and WebUI packages
- [ ] TypeScript strict mode passes
- [ ] No runtime dependencies (only devDependencies for types)

**Testing:**
- Type-check: `tsc --noEmit` passes
- Smoke: Import from another workspace package, verify types resolve

---

## Documentation

- `docs/src/sdk/client.md` — ForgeClient usage, configuration, error handling
- `docs/src/sdk/examples.md` — code examples for each resource (drawn from `refs/examples/error/sdk/error-handling.md`)
