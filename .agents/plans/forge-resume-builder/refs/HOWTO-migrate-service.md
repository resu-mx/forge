# HOWTO: migrate a service to EntityLifecycleManager

Phase 1 of the storage abstraction rewires every Forge service from raw
`SomeRepo.*(this.db, ...)` calls to `this.elm.*('entity', ...)`. Phase 1.1
established the pattern on three simple services (domain, industry,
role_type). This document captures the recipe so Phase 1.2+ can be
mechanical.

## TL;DR checklist

Per service:

1. Change every method signature to `async` and wrap the return type in
   `Promise<...>`.
2. Replace `SomeRepo.create` / `get` / `list` / `update` / `del` calls
   with `this.elm.create/get/list/update/delete`.
3. Use `storageErrorToForgeError` from `../storage/error-mapper` on every
   ELM failure path.
4. For `create` and `update`, the ELM returns only `{ id }` on success —
   call `this.elm.get(...)` afterwards to rehydrate the typed row the
   service contract expects.
5. For `list`, reshape `Result<ListResult>` (which has `{ rows, total }`)
   into the service's `PaginatedResult<T>` (which has
   `{ data, pagination }`). Remember to pass `orderBy` to preserve sort
   order.
6. Drop pre-delete reference counts **unless** one of these applies:
   - The reference is via a text column (not an FK) — integrity layer
     cannot see it.
   - The entity map declares `cascade` where the historical service
     blocked. Keeping the pre-check preserves the old "block delete"
     behavior. Deliberately re-declaring a `restrict` rule in the
     entity map is the alternative but scopes a larger change.
7. Drop error-message string matching (`message.includes('UNIQUE constraint')`).
   The error mapper translates `UNIQUE_VIOLATION → CONFLICT`
   automatically. The mapped `StorageError.message` contains the
   substring `already exists`, which preserves existing test assertions.
8. Update the corresponding `routes/<entity>.ts`: every handler must be
   `async` and every service call must be `await`ed.
9. Update the corresponding `services/__tests__/<service>.test.ts`:
   every `test('...', () => {})` becomes `test('...', async () => {})`
   and every service call is `await`ed. Assertions stay the same.
10. Commit with message
    `feat(storage): Phase 1.X.Y — migrate <service-name> to EntityLifecycleManager`.

## Canonical skeleton

```typescript
import type { Database } from 'bun:sqlite'
import { buildDefaultElm } from '../storage/build-elm'
import { storageErrorToForgeError } from '../storage/error-mapper'
import type { EntityLifecycleManager } from '../storage/lifecycle-manager'
import type { Foo, Result, PaginatedResult } from '../types'
import type { CreateFooInput } from '../db/repositories/foo-repository'

export class FooService {
  protected readonly elm: EntityLifecycleManager
  constructor(private db: Database, elm?: EntityLifecycleManager) {
    this.elm = elm ?? buildDefaultElm(db)
  }

  async create(input: CreateFooInput): Promise<Result<Foo>> {
    // 1. Service-level validation that the ELM does NOT handle
    //    (regex, enum semantics beyond the map, derived fields, ...)
    if (!input.name || input.name.trim().length === 0) {
      return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Name must not be empty' } }
    }

    // 2. ELM create — returns { id } only
    const createResult = await this.elm.create('foos', { ...input })
    if (!createResult.ok) {
      return { ok: false, error: storageErrorToForgeError(createResult.error) }
    }

    // 3. Fetch the full row to match the service's return type
    const fetched = await this.elm.get('foos', createResult.value.id)
    if (!fetched.ok) {
      return { ok: false, error: storageErrorToForgeError(fetched.error) }
    }
    return { ok: true, data: fetched.value as unknown as Foo }
  }

  async get(id: string): Promise<Result<Foo>> {
    const result = await this.elm.get('foos', id)
    if (!result.ok) {
      return { ok: false, error: storageErrorToForgeError(result.error) }
    }
    return { ok: true, data: result.value as unknown as Foo }
  }

  async list(offset?: number, limit?: number): Promise<PaginatedResult<Foo>> {
    const listResult = await this.elm.list('foos', {
      offset,
      limit,
      orderBy: [{ field: 'name', direction: 'asc' }],
    })
    if (!listResult.ok) {
      return { ok: false, error: storageErrorToForgeError(listResult.error) }
    }
    return {
      ok: true,
      data: listResult.value.rows as unknown as Foo[],
      pagination: { total: listResult.value.total, offset: offset ?? 0, limit: limit ?? 50 },
    }
  }

  async update(id: string, input: Partial<CreateFooInput>): Promise<Result<Foo>> {
    // Service-level validation again here
    const updateResult = await this.elm.update('foos', id, { ...input })
    if (!updateResult.ok) {
      return { ok: false, error: storageErrorToForgeError(updateResult.error) }
    }
    const fetched = await this.elm.get('foos', id)
    if (!fetched.ok) {
      return { ok: false, error: storageErrorToForgeError(fetched.error) }
    }
    return { ok: true, data: fetched.value as unknown as Foo }
  }

  async delete(id: string): Promise<Result<void>> {
    // Only add pre-delete checks if:
    //   (a) the reference is via a text column, OR
    //   (b) the entity map cascades where you want to block.
    // Otherwise rely on the entity map's cascade/restrict/setNull.
    const delResult = await this.elm.delete('foos', id)
    if (!delResult.ok) {
      return { ok: false, error: storageErrorToForgeError(delResult.error) }
    }
    return { ok: true, data: undefined }
  }
}
```

## Gotchas observed during Phase 1.2 (junction-heavy services)

- **Junction table composite-PK uniqueness.** `validateUniqueness`
  originally only covered per-field `unique: true` constraints, not
  composite primary keys on junction tables. Duplicate inserts on a
  junction (e.g. re-linking an archetype to the same domain) surfaced
  as `ADAPTER_ERROR / INTERNAL` instead of `UNIQUE_VIOLATION /
  CONFLICT`. Phase 1.2.1 extended `validateUniqueness` to run a
  composite-key `count(where: {...pks})` when
  `def.primaryKey.length > 1`. This benefits ALL junction writes in
  Phase 1.2+ (skill_domains, summary_skills, certification_skills,
  contact_*, note_references, org_tags). Tests cover 2-field and
  3-field composite keys.
- **Junction create via `elm.create(entity, {...keys})`.** No id is
  auto-generated because the entity map has no `id` field. The ELM's
  `applyDefaults` skips id generation when the entity lacks an id.
  `adapter.create` returns `{ id: '' }` from junction creates — the
  service should not rely on it.
- **Junction delete via `elm.deleteWhere(entity, {...keys})`.** Never
  use `elm.delete()` on a junction — there is no single id. The
  `deleteWhere` primitive skips cascade/restrict/setNull, which is
  fine for junction rows because they should not declare cascades
  themselves.
- **Junction read via `elm.list(entity, { where: {...}, limit })`.**
  Default limit is small and junctions can be large — always pass an
  explicit limit (10000 is a safe upper bound for bounded tables).
- **Idempotent junction adds: pre-check with `elm.count` before
  `elm.create`.** Most historical `addSkill` / `addDomain` /
  `addOrganization` helpers used `INSERT OR IGNORE`. To preserve that
  contract, pre-check existence via `elm.count(entity, {...pk})` and
  return ok if `value > 0`. Do NOT rely on the composite-PK uniqueness
  check's CONFLICT — it surfaces as a CONFLICT error, which is a
  different behavior from "silently succeed".
- **Boolean fields with `boolean: true` in the entity map
  round-trip as booleans.** If the legacy type declares the field as
  `number` (e.g. `summary.is_template: number`,
  `organization.worked: number`), the service must normalize
  `typeof === 'boolean'` on every read path via a `toEntity`
  converter. Certifications are the exception: the type declares
  `in_progress: boolean`, so the ELM output already matches without
  conversion.
- **Lazy JSON fields require `includeLazy`.** The entity map declares
  `credentials.details` as `{ type: 'json', lazy: true }`, which
  means the ELM's deserializer skips it by default. Every read path
  in credential-service passes `{ includeLazy: ['details'] }` or
  the `details` field will be missing from the returned row. The same
  gate applies to `resumes.header` and `prompt_logs.prompt_input` —
  any future migration touching those must remember the opt-in.
- **Polymorphic JSON merges stay in the service.** credential-service
  fetches the existing `details` (with includeLazy), merges the
  partial patch in-memory, then validates the merged result against
  the type-specific schema before writing back. The ELM does not
  understand polymorphic JSON structures.
- **FK pre-checks for friendly error wording.** The ELM's
  `validateForeignKeys` already catches unknown FK references and
  returns a generic `FK_VIOLATION → VALIDATION_ERROR`. But several
  historical tests assert specific wording like
  `"Organization X not found"` or `"Skill X not found"`. For those
  cases, keep a pre-check in the service (`elm.get(...)`) before the
  write so you can return the friendly message with `code: 'NOT_FOUND'`.
- **Singleton tables.** `user_profile` has at most one row. The ELM
  has no built-in singleton awareness, so the service enforces it by
  always fetching the row with `elm.list({ limit: 1 })` and never
  calling `elm.create('user_profile', ...)` (the row is seeded by
  migration). Updates use `elm.update('user_profile', singletonId,
  patch)`.
- **Junction-based filters in list queries.** When a list method
  supports filtering by a column that lives in a junction table
  (e.g. `list({ tag })` for organizations, `list({ skill_id })` for
  summaries), walk the junction first to collect matching entity ids,
  then combine with the main-table where clause via `$and` +
  `{ id: { $in: [...] } }`. If the junction returns no matches,
  short-circuit to an empty result before querying the main table.
- **`COLLATE NOCASE` substring searches.** The historical repos used
  `LIKE ? COLLATE NOCASE` for case-insensitive substring matching.
  SQLite's default `LIKE` is already case-insensitive for ASCII
  characters (the lowercasing is done by the pattern matcher), so a
  plain `$like` clause on the WhereClause DSL preserves the behavior
  for all practical inputs. Document this when the old code explicitly
  used COLLATE NOCASE so a future reader does not re-add the clause.
- **Computed columns (`linked_resume_count`, `organization_name`,
  `tags`).** JOIN-computed or aggregate columns don't exist in the
  entity map. The service is responsible for populating them on
  every read. Patterns used in Phase 1.2:
  - `summary.linked_resume_count`: per-row `elm.count('resumes', {
    summary_id })` inside `toSummaryWithCount`.
  - `contact.organization_name`: per-row `elm.get('organizations',
    organization_id)` inside `toContactWithOrg`.
  - `organization.tags`: `elm.list('org_tags', { where: {
    organization_id } })` inside `fetchTagsFor`, sorted client-side.
  - `archetype.resume_count / perspective_count / domain_count`: 3x
    per-row `elm.count(...)` during `list()`.
  For tables with <50 rows (taxonomy lookups), the per-row queries are
  acceptable. For hot paths, add a named query instead.
- **Service narrowing vs. entity-map broadening.** The entity map's
  `note_references.entity_type` enum has 11 values, but the historical
  service only accepts 7. The service keeps its own narrower check
  rather than deferring to the entity map, preserving the contract.
  If a future caller needs one of the extras, widen the service's
  list — don't rely on the ELM's enum check alone.
- **Route-level async propagation travels farther than you think.**
  Several cross-route callsites referenced contact/skill/org services
  outside the entity's own route file:
  - `routes/job-descriptions.ts` calls `services.contacts.listByJobDescription`
    and `services.skills.list`
  - `routes/organizations.ts` calls `services.contacts.listByOrganization`
  - `routes/resumes.ts` calls `services.contacts.listByResume`
  All had to be async-converted alongside the service migration.
  Always `grep -rn "services.<name>\."` across `routes/`, `mcp/`,
  and `sdk/` before committing.
- **Test files that aren't the service's own.**
  `services/__tests__/services-wiring.test.ts` and
  `services/__tests__/qualifications-integration.test.ts` both
  directly call credential/certification service methods and had to
  be async-ified. Any test file that imports a service directly (not
  via routes) will break if you miss it.

## Gotchas observed during Phase 1.1

- **Type casts.** The ELM returns `Record<string, unknown>` from
  `get`/`list`. Use `as unknown as Foo` (double cast). TypeScript rejects
  a direct cast because `Record<string, unknown>` is not assignable to a
  specific interface.
- **`Result.value` vs `Result.data`.** The storage layer's Result uses
  `.value`; the service layer's Result uses `.data`. `liftStorageResult`
  renames automatically, but if you're doing error-path passthrough, use
  `storageErrorToForgeError(result.error)` directly — it's clearer and
  avoids the "what type is T on the failure branch" question.
- **`PaginatedResult<T>` shape.** The ELM returns
  `{ rows, total }`; the service returns `{ data, pagination: { total,
  offset, limit } }`. Always reshape manually.
- **`orderBy` matters for WebUI stability.** Always pass an explicit
  `orderBy` or your list order becomes SQLite row insertion order,
  breaking UI stability. Pass the same column the old repository's
  `ORDER BY` used.
- **Per-row enrichment ("`XxxWithUsage`" types).** Some old repositories
  enriched list rows with JOIN-computed counts. The ELM has no JOIN.
  Options: (a) compute per-row via `elm.count(...)` (cheap if the list
  is bounded), (b) add a named query in
  `storage/adapters/sqlite-named-queries.ts`. For Phase 1.1, option (a)
  is fine for taxonomy-size tables.
- **Case-insensitive `getByName`.** Old repositories used
  `lower(name) = lower(?)`. The ELM has no `$ilike`. For small lookup
  tables, re-implement as an in-memory filter over a
  bounded `list({ limit: 1000 })`. For large tables, add a named query.
- **Error-mapper `UNIQUE_VIOLATION` message substring.** The storage
  layer's `uniqueViolation` factory produces a message like
  `"domains.name must be unique: \"foo\" already exists"`. Tests that
  assert `.toContain('already exists')` still pass unchanged. Tests that
  assert the exact pre-migration message (e.g., `"Domain 'foo' already
  exists"`) would need a rewrite.
- **Behavior changes from `setNull`.** Deleting an industry previously
  blocked when any org referenced it. Post-migration, deletes silently
  NULL the org's `industry_id`. This is a deliberate behavior change
  authorized by the entity map. If a service has existing tests that
  assert the blocking behavior, either update the test OR re-declare a
  `restrict` rule in the entity map.

## Gotchas observed during Phase 1.3 (core content services)

Phase 1.3 migrated the five services that own Forge's core domain
model: `job-description-service`, `bullet-service`,
`perspective-service`, `source-service`, and `resume-service`. These
services coordinate writes across multiple tables and interact with
hooks that were already wired in Phase 0. The following gotchas
expand on the Phase 1.2 list:

- **Embedding hooks are not always drop-in replacements.**
  `createEmbedHook(embeddingService, entityType, contentField)` works
  for simple "single row → single content field" embeddings
  (`source.description`, `bullet.content`, `perspective.content`).
  For **JD embedding** the semantics are different: raw_text gets
  parsed into multiple requirement strings, each embedded under
  `entity_id = '{jd_id}:{i}'`. The generic hook cannot express this,
  so the JD service still owns the `queueMicrotask(() =>
  onJDCreated(jd, requirementTexts))` pattern AND the entity-map's
  incorrect `createEmbedHook(..., 'jd_requirement', 'raw_text')` wiring
  was deleted in Phase 1.3.1. Before migrating any service with an
  embedding hook, verify the hook's shape actually matches the
  service's current behavior — do not trust a prior "already wired"
  comment without reading the hook factory.

- **Junction management inside create/update paths.**
  `bullet-service.createBullet` writes `bullets` + `bullet_sources` +
  `bullet_skills` (with find-or-create on the `skills` table). The
  historical repo bundled this in a single `db.transaction(...)`; the
  migrated service does it as sequential `elm.create` calls without a
  transaction scope, accepting that a mid-flight failure can leave a
  partially-created row (same failure mode as the historical code,
  because `db.transaction` only wraps the immediate callback and does
  not see side effects of the ELM's own internal writes). If a real
  atomicity requirement emerges later, wrap the sequence in
  `elm.transaction(...)` and use raw `tx.create` (no validation);
  pre-validate outside the transaction.

- **Find-or-create skill resolution.** For technology tagging on
  bullets (and anywhere else that accepts free-text skill names),
  match the Phase 1.2 skill-service pattern: bulk-load all skills
  once via `elm.list('skills', { limit: 10000 })`, build a
  `Map<lowerName, id>`, then loop the input normalizing each name.
  One bulk query instead of N lookups. Create missing skills via
  `elm.create('skills', { name: normalized, category: 'other' })`
  and add them to the map as you go.

- **Source polymorphism is NOT via transactions.** The entity map
  declares four extension tables (`source_roles`, `source_projects`,
  `source_education`, `source_presentations`) keyed by `source_id`.
  Create: one `elm.create('sources', ...)` followed by one
  `elm.create('source_{type}')`. Update: one `elm.update('sources',
  id, patch)` followed by `elm.updateWhere('source_{type}', {
  source_id }, extPatch)`. **Critical gotcha**: the extension table
  names are NOT pluralized uniformly. `source_education` is
  singular; the others are plural. Use an explicit
  `extensionEntityFor(sourceType)` dispatch helper; do not
  `source_${type}s` string-concatenate, or education breaks with
  `Unknown entity type: source_educations`.

- **Immutable source_type after create.** The historical
  `SourceRepo.update` never touches `source_type` — it always
  dispatches the extension update using the EXISTING source type.
  The migrated service preserves this: update fetches the row to
  learn its type, then updates the matching extension table. Moving
  between extension tables (changing source_type from `role` to
  `project`) is NOT supported by the current update path and is not
  tested. The prompt's warning about "source_type can change" turned
  out to be historical speculation; verify current behavior before
  implementing cross-table moves.

- **Status FSM validation stays in the service.** Bullets and
  perspectives have FSM rules (draft → in_review → approved/rejected;
  approved → archived; archived → draft). The ELM does not know about
  FSMs. Keep `VALID_TRANSITIONS` in the service, fetch the current
  row via `elm.get`, check the transition, then build a patch that
  includes the new status plus any state-dependent fields
  (`approved_at`, `approved_by`, `rejection_reason`). The historical
  updateStatus had subtle COALESCE semantics (reject keeps
  approved_at/by); mirror those exactly to preserve test
  expectations.

- **captureSnapshotHook only captures, it does not clear.** The
  entity map's `beforeCreate: [captureSnapshotHook]` on
  `resume_entries` and `beforeCreate:
  [captureBulletSnapshotHook]` on `perspectives` fire at CREATE time
  and populate the snapshot column if it would otherwise be null.
  On UPDATE they do NOT re-run (the hook checks `if (!perspectiveId
  || content) return` and `perspectiveId` is only in the patch when
  the caller supplies it). This means:
  - The "create in reference mode → snapshot captured once" path is
    handled by the hook.
  - The "update: switch to clone mode" path does nothing (snapshot
    stays from creation time, which is acceptable).
  - The "update: reset to reference mode (content=null)" path must
    EXPLICITLY set `perspective_content_snapshot: null` in the
    service's patch. The hook will not clear it automatically, and
    the e2e test asserts snapshot is null after the reset.

- **Lazy fields require explicit includeLazy on every read path.**
  The ELM's deserializer skips fields marked `lazy: true` in the
  entity map unless the caller passes `{ includeLazy: [...fields] }`.
  Phase 1.3 touched these lazy fields:
  - `resumes.header` (lazy JSON — object on read, stringified on write)
  - `resumes.markdown_override` (lazy text)
  - `resumes.latex_override` (lazy text)
  - `resumes.summary_override` (lazy text)
  - (Previously in Phase 1.2) `credentials.details` (lazy JSON)
  Build a shared `fetchResume(id)` helper that always includes the
  full set of lazy fields; call it from every read path. Specialized
  paths (like `generatePDF` that only needs `latex_override`) can use
  a narrower include list. If you skip includeLazy, you will get
  "header is undefined" and similar cryptic errors in downstream
  compilers / templates.

- **JSON serialization happens inside the ELM for lazy json fields.**
  For `resumes.header` (type: `json`, lazy: true), the service passes
  a plain object: `elm.update('resumes', id, { header: {...} })`. The
  ELM's write path serializes the object to a JSON string. Do NOT
  manually `JSON.stringify(header)` before the update — that would
  double-encode.

- **UNIQUE SQL constraints outside the entity map must be
  pre-checked.** `resume_skills` has `UNIQUE(section_id, skill_id)`
  and `resume_certifications` has `UNIQUE(resume_id,
  certification_id)` at the SQLite level. The entity map only
  declares primary-key uniqueness and per-field `unique: true`
  uniqueness — it does not know about ad-hoc composite UNIQUE
  clauses. When the ELM's `validateUniqueness` passes, the adapter
  attempts the INSERT, SQLite rejects it, and the adapter wraps the
  error as `ADAPTER_ERROR / INTERNAL`. Tests that assert `CONFLICT`
  will fail.
  Workaround: pre-check with `elm.count(entity, { ...uniqueKeys })`
  before calling `elm.create`. Pattern:
  ```ts
  const existing = await this.elm.count('resume_skills', { section_id, skill_id })
  if (existing.ok && existing.value > 0) {
    return { ok: false, error: { code: 'CONFLICT', message: '...' } }
  }
  ```
  Long-term fix (Phase 1.8 or later): extend the entity map to
  declare composite uniqueness via a new `uniqueTuples` field, and
  have `validateUniqueness` check those too.

- **Multi-table hydration: use named queries only for hot paths.**
  `resume-service.fetchResumeWithEntries` does O(1 + sections +
  entries + entries-with-perspectives) queries per call. For small
  resumes (<50 entries) this is fine; for the hot resume-editor
  path it may become a bottleneck. If it does, add a named query
  that JOINs everything. For Phase 1.3 we leave it as-is because
  neither the tests nor the WebUI exercise enough volume to care.

- **`elm.transaction` takes a TransactionScope — no validation.**
  Inside `elm.transaction(async (tx) => {...})`, the tx object
  exposes `create/get/update/delete/deleteWhere/updateWhere/count`
  but does NOT run entity-map validation. FK/enum/required checks
  must happen BEFORE opening the transaction. Only use transactions
  when multiple writes must all succeed or all fail — most migrated
  services (even ones touching multiple tables) get away with
  sequential ELM calls because the failure mode matches what the
  historical `db.transaction` wrapper allowed.

## Phase 1 complete — post-1.8 state

Phase 1.8 deleted 22 repository files and 21 test files (~12K lines).
Only `campus-repository.ts` survives (campuses route is not yet a service).

Service constructors are now `(elm: EntityLifecycleManager)` except:
- `ResumeService(db, elm)` — raw SQL in `findBulletsForGap`, `getSourceTitleForBullet`
- `ExportService(db, dbPath, elm)` — `compileResumeIR` + sqlite3 dump

Tests use `new Service(buildDefaultElm(db))` for direct construction.

Type definitions extracted from repositories live in `types/index.ts`
(34 types added: input/filter/sort interfaces, embedding row types, etc.).

Test baseline: 1281 pass / 12 fail (pre-existing) / 1293 total.
Dropped from 1790 due to deleted repo tests; zero regressions.

## Gotchas observed during Phase 1.4 (read-only services)

Phase 1.4 migrated 6 read-heavy services: review, tagline, audit,
integrity, export, and resume-compiler. These services primarily read
from the content model (already migrated in Phase 1.3). Key learnings:

- **Per-row enrichment works fine for small result sets.** Review-service
  uses elm.list with status filters, then enriches each row with source
  title via `elm.list('bullet_sources', {...}) → elm.get('sources', id)`.
  For review queues (<20 items), the N+1 pattern is perfectly acceptable
  and much simpler than adding named queries.

- **Named queries are the right tool for cross-table inequality JOINs.**
  Integrity-service's drift detection (`WHERE snapshot != current_value`)
  requires comparing columns across joined tables. This can't be
  expressed with the WhereClause DSL — use a named query. Phase 1.4
  added `listDriftedBullets` and `listDriftedPerspectives`.

- **Named query authoring is a 4-step process.**
  a. Declare params + result types in `named-queries.ts`
  b. Add to the `NamedQueryRegistry` interface
  c. Implement in `sqlite-named-queries.ts`
  d. Register in the `SQLITE_NAMED_QUERIES` handler map
  Always add a test in `named-queries.test.ts`. Phase 1.4 grew the
  storage test suite from 51 to 53.

- **Named query return types are raw — no deserialization.**
  elm.query returns exactly what the SQL implementation returns. No
  boolean conversion, no JSON parse, no lazy field gating. Design the
  SQL and result types to match what the service caller expects. If you
  need booleans, cast in SQL or normalize in the service.

- **Free functions with db parameter: add optional elm parameter.**
  `regenerateResumeTagline(db, resumeId, options, elm?)` and
  `compileResumeIR(db, resumeId, elm?)` follow option 1 from the Phase
  1 plan — add elm as an optional trailing parameter, default to
  `buildDefaultElm(db)` when not provided. This keeps all existing
  callers working without modification. Phase 1.8 cleanup can promote
  these to service classes if needed.

- **Route callers that pass `db` to free functions need attention.**
  `routes/job-descriptions.ts` calls `regenerateResumeTagline(db, ...)`
  directly. When the function becomes async, the route handler must also
  become async. Always grep for the function name across `routes/`,
  `services/`, and `mcp/` before committing.

- **Export route's `db` parameter can be eliminated.** Phase 1.4.5
  replaced `ResumeRepository.get(db, id)` in the export route with
  `services.resumes.getResume(id)`, removing the route's dependency on
  the raw db parameter. The `exportRoutes(services, db)` signature
  simplified to `exportRoutes(services)`.

- **listAll patterns → elm.list with huge limit.** `BulletRepository
  .listAll`, `PerspectiveRepository.listAll`, `SourceRepo.listAll`, etc.
  are all replaced with `elm.list(entity, { limit: 100000, orderBy:
  [{ field: 'created_at', direction: 'desc' }] })`. The limit is a
  safety cap; bounded tables are <10k rows in practice.

- **Complex multi-table JOINs should stay as raw SQL, not N+1 ELM
  queries.** The resume-compiler has 6-8 table JOINs per section builder
  (experience, education, projects, presentations, etc.). Converting
  these to multi-step ELM fetches would be a regression in both
  performance and readability. The correct migration path is to promote
  them to named queries in Phase 2 (when adapter portability matters).
  Phase 1.4.6 added only the elm parameter; internal SQL is unchanged.

- **Test count grows with named queries.** Phase 1.4 added 2 named
  query tests (listDriftedBullets, listDriftedPerspectives). Full suite
  baseline is now 1790 pass / 12 fail / 1802 total. Storage suite is
  53/53.

## After each commit

Run, in Docker, the targeted service test file first:

```bash
docker compose --profile tools run --rm shell -c \
  'cd /app && bun test packages/core/src/services/__tests__/<svc>.test.ts'
```

Then (once per sub-phase) the full core suite:

```bash
docker compose --profile tools run --rm shell -c \
  'cd /app && bun run --filter "@forge/core" test'
```

Baseline from Phase 1.7: **1790 pass / 12 fail / 1802 total**. The 12
failures are pre-existing unrelated tests (sb2nov template,
buildOrgDisplayString, compileResumeIR Phase 44 data quality). Any new
failure is a regression.

## Gotchas observed during Phase 1.5 (derivation service)

Phase 1.5 migrated the split-handshake derivation service with its
pending_derivations lock protocol:

- **Entity-map afterCreate hooks replace explicit embedding calls.**
  The entity map has `afterCreate: [createEmbedHook(...)]` on bullets
  and perspectives. When derivation-service switched from
  `BulletRepository.create` (raw SQL, no hooks) to `elm.create('bullets',
  ...)` (hooks fire), the explicit `queueMicrotask(() =>
  embeddingService.onBulletCreated(...))` calls became redundant.
  Removing them + removing `setEmbeddingService()` from the service and
  from `index.ts` avoids double-embedding.

- **UNIQUE INDEX on (entity_type, entity_id) not in entity map.** The
  `pending_derivations` table has a SQL-level UNIQUE INDEX. The entity
  map only declares the primary key (`id`). When `elm.create` hits the
  UNIQUE constraint, the adapter wraps it as `ADAPTER_ERROR`. Check
  `createResult.error.message.includes('UNIQUE')` and translate to
  `CONFLICT` to preserve the original error contract.

- **Unexpired lock check uses `$gt` on ISO timestamps.** Replaces
  `datetime(expires_at) > datetime('now')` with `{ expires_at: { $gt:
  new Date().toISOString() } }`. ISO 8601 strings sort lexicographically
  in the same timezone (both UTC), so this is equivalent.

- **Case-insensitive archetype/domain lookup.** The derivation service
  validates archetype and domain names. Replaced `getByName(db, name)`
  (which used `lower(name) = lower(?)`) with `elm.list` + in-memory
  `.some(a => a.name.toLowerCase() === target.toLowerCase())` on
  bounded taxonomy tables (<50 rows).

- **Technology resolution pattern reused from Phase 1.2.** Bulk-load
  all skills once via `elm.list('skills', { limit: 10000 })`, build
  a `Map<lowerName, id>`, find-or-create missing ones.

- **`recoverStaleLocks` became async with optional elm parameter.** Uses
  `elm.deleteWhere('pending_derivations', { expires_at: { $lte: now } })`
  for expired lock cleanup and `elm.updateWhere('sources', ...)` for
  legacy 'deriving' status reset. Callers in `index.ts` now `await` the
  call (top-level await in ESM).

- **Lazy fields on pending_derivations.** `prompt` and `snapshot` are
  `lazy: true` in the entity map. The commit methods must pass
  `{ includeLazy: ['prompt', 'snapshot'] }` on every get call.

## Gotchas observed during Phase 1.6 (embedding service)

Phase 1.6 migrated the embedding service, which has a circular
dependency with the entity map (service is injected into hooks):

- **Circular dependency is safe by construction.** `EmbeddingService(db)`
  creates its own private ELM via `buildDefaultElm(db)` — this ELM has
  NO embedding hooks (because `embeddingService` wasn't passed as a
  dep). The shared ELM (in `createServices`) has hooks that call back
  to the EmbeddingService. So: shared ELM → hook → EmbeddingService →
  private ELM → no hook → no loop.

- **Upsert replaced with find + create/update.** The original
  `EmbeddingRepository.upsert` used `INSERT ... ON CONFLICT DO UPDATE`.
  ELM has no upsert. Migration: `elm.list` to find existing by
  `{ entity_type, entity_id }`, then `elm.update` or `elm.create`.

- **Blob fields work through ELM.** On write: `Float32Array` → adapter's
  `serializeValue` converts to `Buffer`. On read: ELM deserializes to
  `Uint8Array`. Use `toFloat32()` helper to convert back to
  `Float32Array` for cosine similarity math.

- **vector field is lazy.** Must pass `includeLazy: ['vector']` when
  reading embeddings for similarity search. For hash-only checks
  (freshness), skip the vector to save memory.

- **Model loading timeout.** The first `computeEmbedding()` call
  downloads/loads the ML model (~30s). Tests need `--timeout 60000`
  on first run. Subsequent calls are fast (<50ms). This is pre-existing,
  not a migration regression.

- **alignResume: JOIN replaced with sequential ELM fetches.** The
  `resume_entries JOIN resume_sections WHERE resume_id = ?` query was
  replaced with `elm.list('resume_sections', { where: { resume_id } })`
  → `elm.list('resume_entries', { where: { section_id } })` per section.
  O(sections) queries instead of 1 JOIN, acceptable for <10 sections.

## Gotchas observed during Phase 1.7 (template service)

Phase 1.7 migrated the template service with its atomic
createResumeFromTemplate transaction:

- **sections field is lazy json.** `resume_templates.sections` is
  `{ type: 'json', lazy: true }`. Every read path must pass
  `includeLazy: ['sections']` or the returned template will have
  `sections: undefined`.

- **is_builtin boolean normalization.** Entity map declares
  `boolean: true`, ELM deserializes to `true`/`false`. The
  `ResumeTemplate` type expects `is_builtin: number` (0|1). Added
  `toTemplate()` converter that normalizes `true → 1, false → 0`.

- **elm.transaction tx.create does NOT run applyDefaults.** This is
  the most critical gotcha. Inside `elm.transaction(async (tx) => {})`,
  `tx.create` bypasses the lifecycle manager entirely — no validation,
  no defaults, no hooks. You MUST provide:
  - `id: crypto.randomUUID()` — no auto-generation
  - `created_at: now` and `updated_at: now` — no default functions
  The SQL-level `DEFAULT (datetime('now'))` MAY fire if the column is
  omitted, but relying on it is fragile across adapters. Be explicit.

- **Route async propagation.** `routes/templates.ts` had 5 handlers
  that needed `async` + `await`. `routes/resumes.ts` had 2 cross-route
  calls to template service that needed `await`.
