# Presentations Entity — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `presentation` as a first-class source type with CRUD, a /experience/presentations page, and proper resume picker routing.

**Architecture:** New `source_presentations` extension table + `presentation` added to source_type CHECK via table rebuild migration. Repository/service/routes follow the existing polymorphic source pattern. WebUI page delegates to the existing `<SourcesView>` component. Resume picker uses the SourcePicker pattern. Compiler gets the LEFT JOIN + COALESCE refactor.

**Tech Stack:** SQLite, TypeScript, Svelte 5, Hono

**Spec:** `.claude/plans/forge-resume-builder/refs/specs/2026-04-05-presentations-entity.md`

---

## CRITICAL: All work in the worktree

```
/Users/adam/notes/job-hunting/.claude/worktrees/presentations-entity
```

Branch: `phase/presentations-entity`. ALL commands run from this directory. ALL file paths are relative to this worktree root. Do NOT work from `/Users/adam/notes/job-hunting/`.

## Files to Create

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/039_presentations.sql` | Rebuild sources CHECK + create source_presentations |
| `packages/webui/src/routes/experience/presentations/+page.svelte` | Thin page delegating to SourcesView |

## Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/types/index.ts` | SourceType, SourcePresentation, PresentationType, CreateSource, UpdateSource, PresentationItem |
| `packages/sdk/src/types.ts` | Mirror all core type changes |
| `packages/core/src/db/repositories/source-repository.ts` | getExtension, updateExtension, create for presentation |
| `packages/core/src/routes/sources.ts` | mapExtension presentation case |
| `packages/core/src/services/resume-compiler.ts` | buildPresentationItems LEFT JOIN + new fields |
| `packages/webui/src/lib/nav.ts` | Add Presentations to Experience group |
| `packages/webui/src/lib/components/SourcesView.svelte` | Presentation form fields + card display |
| `packages/webui/src/routes/resumes/+page.svelte` | openPicker presentations case |
| `packages/webui/src/lib/components/resume/DragNDropView.svelte` | Presentations template update |
| `packages/core/src/db/__tests__/migrate.test.ts` | Migration count |
| `packages/core/src/db/repositories/__tests__/source-repository.test.ts` | Presentation CRUD tests |
| `packages/core/src/services/__tests__/resume-compiler.test.ts` | Presentation compiler test |

---

## Tasks

### Task 1: Migration 039 — presentations

**Files:**
- Create: `packages/core/src/db/migrations/039_presentations.sql`
- Modify: `packages/core/src/db/__tests__/migrate.test.ts`

**Before starting:** verify migration high-water mark: `ls packages/core/src/db/migrations/ | sort -V | tail -3`. Expected: highest is `038_resume_summary_override.sql`. If another agent landed 039, bump this to 040.

- [ ] **Step 1: Create the migration file**

Write to `packages/core/src/db/migrations/039_presentations.sql`:

```sql
-- Forge Resume Builder — Presentations Entity
-- Migration: 039_presentations
-- Date: 2026-04-05
--
-- Adds 'presentation' to the sources.source_type CHECK constraint and
-- creates the source_presentations extension table. Requires a table
-- rebuild on sources since SQLite doesn't support ALTER CHECK.
--
-- PRAGMA foreign_keys = OFF

-- Step 1: Rebuild sources table with updated CHECK constraint
CREATE TABLE sources_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  source_type TEXT NOT NULL DEFAULT 'general'
    CHECK (source_type IN ('role', 'project', 'education', 'general', 'presentation')),
  start_date TEXT,
  end_date TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN (
    'draft', 'in_review', 'approved', 'rejected', 'archived', 'deriving'
  )),
  updated_by TEXT NOT NULL DEFAULT 'human' CHECK(updated_by IN ('human', 'ai')),
  last_derived_at TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO sources_new SELECT * FROM sources;
DROP TABLE sources;
ALTER TABLE sources_new RENAME TO sources;

-- Re-create indexes that were on the original table
CREATE INDEX idx_sources_status ON sources(status);
CREATE INDEX idx_sources_source_type ON sources(source_type);

-- Step 2: Create source_presentations extension table
CREATE TABLE source_presentations (
  source_id TEXT PRIMARY KEY
    CHECK(typeof(source_id) = 'text' AND length(source_id) = 36)
    REFERENCES sources(id) ON DELETE CASCADE,
  venue TEXT,
  presentation_type TEXT NOT NULL DEFAULT 'conference_talk'
    CHECK (presentation_type IN (
      'conference_talk', 'workshop', 'poster', 'webinar',
      'lightning_talk', 'panel', 'internal'
    )),
  url TEXT,
  coauthors TEXT
) STRICT;
```

- [ ] **Step 2: Update migration test count**

Read `packages/core/src/db/__tests__/migrate.test.ts`. Find both `toHaveLength(N)` assertions and bump by 1. Add `expect(rows[<last_index>].name).toBe("039_presentations")` at the end of both test functions' assertion lists.

- [ ] **Step 3: Run migration tests**

```bash
bun test packages/core/src/db/__tests__/migrate.test.ts 2>&1 | tail -8
```
Expected: all pass, migration 039 appears in "Applied migration:" output.

- [ ] **Step 4: Commit**

```bash
git add packages/core/src/db/migrations/039_presentations.sql packages/core/src/db/__tests__/migrate.test.ts
git commit -m "feat(core): migration 039 — presentations entity (source_type + extension table)"
```

---

### Task 2: Core + SDK types

**Files:**
- Modify: `packages/core/src/types/index.ts`
- Modify: `packages/sdk/src/types.ts`

Both files need identical changes mirrored.

- [ ] **Step 1: Add PresentationType + SourcePresentation to core types**

Add near the other source extension types (`SourceRole`, `SourceProject`, `SourceEducation`):

```ts
export type PresentationType =
  | 'conference_talk'
  | 'workshop'
  | 'poster'
  | 'webinar'
  | 'lightning_talk'
  | 'panel'
  | 'internal'

export interface SourcePresentation {
  source_id: string
  venue: string | null
  presentation_type: PresentationType
  url: string | null
  coauthors: string | null
}
```

- [ ] **Step 2: Update SourceType union**

Find `export type SourceType` and add `| 'presentation'`.

- [ ] **Step 3: Update CreateSource + UpdateSource**

Add to both interfaces (they share the same optional presentation fields):

```ts
  venue?: string
  presentation_type?: PresentationType
  coauthors?: string
```

Note: `url` is already on `CreateSource`/`UpdateSource` (shared with projects + education). No addition needed for url.

- [ ] **Step 4: Update PresentationItem (IR type)**

Find `export interface PresentationItem`. Currently it has `title, venue, date, entry_id, source_id, bullets`. Add new fields:

```ts
  description: string | null
  presentation_type: string | null
  url: string | null
  coauthors: string | null
```

Also change `venue: string` to `venue: string | null` (to handle direct-source entries where venue might not be set).

- [ ] **Step 5: Update SourceWithExtension if needed**

Check if `SourceWithExtension.extension` union needs `SourcePresentation` added. Grep for the type and add it.

- [ ] **Step 6: Mirror all changes in SDK types**

Repeat steps 1-5 in `packages/sdk/src/types.ts`.

- [ ] **Step 7: Run core tests**

```bash
bun test packages/core/src/ 2>&1 | tail -5
```
Expected: all pass. Fix any test that fails due to `PresentationItem` shape change (add `null` for the new fields in test fixtures).

- [ ] **Step 8: Commit**

```bash
git add packages/core/src/types/index.ts packages/sdk/src/types.ts
git commit -m "feat(core): presentation types — SourcePresentation, PresentationType, PresentationItem (T presentations)"
```

---

### Task 3: Repository — presentation extension handling

**Files:**
- Modify: `packages/core/src/db/repositories/source-repository.ts`
- Modify: `packages/core/src/db/repositories/__tests__/source-repository.test.ts`

- [ ] **Step 1: Write failing tests**

Add to the existing test file inside the appropriate `describe` block:

```ts
test('creates a presentation source with extension fields', () => {
  const source = SourceRepo.create(db, {
    title: 'Cloud Forensics at Scale',
    description: 'Lessons from production IR',
    source_type: 'presentation',
    venue: 'BSides DC 2024',
    presentation_type: 'conference_talk',
    url: 'https://slides.example.com/bsides-2024',
    coauthors: 'Jane Smith',
  })
  expect(source.source_type).toBe('presentation')
  expect(source.extension).not.toBeNull()
  const ext = source.extension as SourcePresentation
  expect(ext.venue).toBe('BSides DC 2024')
  expect(ext.presentation_type).toBe('conference_talk')
  expect(ext.url).toBe('https://slides.example.com/bsides-2024')
  expect(ext.coauthors).toBe('Jane Smith')
})

test('updates presentation extension fields', () => {
  const source = SourceRepo.create(db, {
    title: 'Talk',
    description: 'A talk',
    source_type: 'presentation',
    venue: 'Old Venue',
  })
  const updated = SourceRepo.update(db, source.id, {
    venue: 'New Venue',
    presentation_type: 'workshop',
  })
  const ext = updated!.extension as SourcePresentation
  expect(ext.venue).toBe('New Venue')
  expect(ext.presentation_type).toBe('workshop')
})

test('lists presentation sources via source_type filter', () => {
  SourceRepo.create(db, { title: 'Talk', description: 'd', source_type: 'presentation' })
  SourceRepo.create(db, { title: 'Role', description: 'd', source_type: 'role' })
  const result = SourceRepo.list(db, { source_type: 'presentation' }, 0, 50)
  expect(result.total).toBe(1)
  expect(result.data[0].source_type).toBe('presentation')
})
```

Add `import type { SourcePresentation } from '../../../types'` if not already imported.

- [ ] **Step 2: Run tests to verify they fail**

```bash
bun test packages/core/src/db/repositories/__tests__/source-repository.test.ts 2>&1 | tail -15
```

- [ ] **Step 3: Add presentation handling to the repository**

In `source-repository.ts`:

**`getExtension()`** — add case:
```ts
case 'presentation':
  return db.query('SELECT * FROM source_presentations WHERE source_id = ?')
    .get(sourceId) as SourcePresentation | null
```

**`updateExtension()`** — add block:
```ts
} else if (sourceType === 'presentation') {
  const sets: string[] = []
  const params: unknown[] = []
  if ('venue' in input) { sets.push('venue = ?'); params.push(input.venue ?? null) }
  if ('presentation_type' in input) { sets.push('presentation_type = ?'); params.push(input.presentation_type) }
  if ('url' in input) { sets.push('url = ?'); params.push(input.url ?? null) }
  if ('coauthors' in input) { sets.push('coauthors = ?'); params.push(input.coauthors ?? null) }
  if (sets.length > 0) {
    params.push(sourceId)
    db.run(`UPDATE source_presentations SET ${sets.join(', ')} WHERE source_id = ?`, params)
  }
}
```

**`create()` transaction** — add INSERT block:
```ts
if (input.source_type === 'presentation') {
  db.run(
    `INSERT INTO source_presentations (source_id, venue, presentation_type, url, coauthors)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      input.venue ?? null,
      input.presentation_type ?? 'conference_talk',
      input.url ?? null,
      input.coauthors ?? null,
    ],
  )
}
```

Also update the `SourceWithExtension` type reference in the function signature if needed (add `SourcePresentation` to the union).

- [ ] **Step 4: Run tests**

```bash
bun test packages/core/src/db/repositories/__tests__/source-repository.test.ts 2>&1 | tail -8
```
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/db/repositories/source-repository.ts packages/core/src/db/repositories/__tests__/source-repository.test.ts
git commit -m "feat(core): repository CRUD for presentation sources"
```

---

### Task 4: Route mapExtension + SDK resource

**Files:**
- Modify: `packages/core/src/routes/sources.ts`

- [ ] **Step 1: Add presentation case to mapExtension**

In `packages/core/src/routes/sources.ts`, find the `mapExtension` function (line ~12). Update the ternary chain:

```ts
function mapExtension(source: SourceWithExtension): Record<string, unknown> {
  const { extension, ...rest } = source
  if (!extension) return rest
  const key = source.source_type === 'role' ? 'role'
    : source.source_type === 'project' ? 'project'
    : source.source_type === 'education' ? 'education'
    : source.source_type === 'presentation' ? 'presentation'
    : null
  if (!key) return rest
  return { ...rest, [key]: extension }
}
```

Note: also remove the stale `clearance` case if present (Phase 84 removed clearance from source_type but the ternary may still reference it).

- [ ] **Step 2: Run core route tests**

```bash
bun test packages/core/src/routes/__tests__/sources.test.ts 2>&1 | tail -8
```
Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/routes/sources.ts
git commit -m "feat(core): route mapExtension for presentation sources"
```

---

### Task 5: WebUI — nav + page + SourcesView form

**Files:**
- Modify: `packages/webui/src/lib/nav.ts`
- Create: `packages/webui/src/routes/experience/presentations/+page.svelte`
- Modify: `packages/webui/src/lib/components/SourcesView.svelte`

- [ ] **Step 1: Add Presentations to nav**

In `packages/webui/src/lib/nav.ts`, find the Experience group's `children` array. Add a new entry between Projects and Education:

```ts
{ href: '/experience/presentations', label: 'Presentations' },
```

- [ ] **Step 2: Create the page**

Write to `packages/webui/src/routes/experience/presentations/+page.svelte`:

```svelte
<script lang="ts">
  import SourcesView from '$lib/components/SourcesView.svelte'
</script>

<SourcesView sourceTypeFilter="presentation" />
```

- [ ] **Step 3: Add presentation form state to SourcesView**

Read `packages/webui/src/lib/components/SourcesView.svelte`. Find the form state variable declarations (around lines 40-100, where `formTitle`, `formDescription`, `formOrgId`, `formProjectUrl`, etc. live).

Add these new variables alongside the existing project form variables:

```ts
  let formVenue = $state('')
  let formPresentationType = $state('conference_talk')
  let formPresentationUrl = $state('')
  let formCoauthors = $state('')
```

- [ ] **Step 4: Add populate logic**

Find `function populateFormFromSource(source: Source)` (around line 312). Find the block that handles `source.source_type === 'project'`. Add a similar block for presentations:

```ts
    } else if (source.source_type === 'presentation' && source.presentation) {
      formVenue = source.presentation.venue ?? ''
      formPresentationType = source.presentation.presentation_type ?? 'conference_talk'
      formPresentationUrl = source.presentation.url ?? ''
      formCoauthors = source.presentation.coauthors ?? ''
```

- [ ] **Step 5: Add reset logic**

Find the reset section (around lines 395-425, where form variables are cleared). Add:

```ts
    formVenue = ''
    formPresentationType = 'conference_talk'
    formPresentationUrl = ''
    formCoauthors = ''
```

- [ ] **Step 6: Add save logic**

Find the save/update function (around line 445). Find where the payload is built for different source types. Add a presentation block:

```ts
      if (source.source_type === 'presentation') {
        payload.venue = formVenue || undefined
        payload.presentation_type = formPresentationType || undefined
        payload.url = formPresentationUrl || undefined
        payload.coauthors = formCoauthors || undefined
      }
```

Also in the CREATE path (for new sources), add similar fields when `source_type === 'presentation'`.

- [ ] **Step 7: Add form template block**

Find the editor form section in the template. There are `{#if source.source_type === 'role'}...{:else if source.source_type === 'project'}...{:else if source.source_type === 'education'}...` blocks for type-specific fields.

Add a new `{:else if source.source_type === 'presentation'}` block:

```svelte
{:else if source.source_type === 'presentation'}
  <!-- Presentation fields -->
  <div class="form-row">
    <div class="form-group">
      <label for="pres-venue">Venue / Event</label>
      <input id="pres-venue" type="text" bind:value={formVenue}
             placeholder="e.g. BSides DC 2024, AWS re:Inforce" />
    </div>
    <div class="form-group">
      <label for="pres-type">Type</label>
      <select id="pres-type" bind:value={formPresentationType}>
        <option value="conference_talk">Conference Talk</option>
        <option value="workshop">Workshop</option>
        <option value="poster">Poster</option>
        <option value="webinar">Webinar</option>
        <option value="lightning_talk">Lightning Talk</option>
        <option value="panel">Panel</option>
        <option value="internal">Internal</option>
      </select>
    </div>
  </div>
  <div class="form-row">
    <div class="form-group">
      <label for="pres-start">Date</label>
      <input id="pres-start" type="date" bind:value={formStartDate} />
    </div>
    <div class="form-group">
      <label for="pres-url">URL (slides/recording)</label>
      <input id="pres-url" type="url" bind:value={formPresentationUrl}
             placeholder="https://..." />
    </div>
  </div>
  <div class="form-group">
    <label for="pres-coauthors">Co-authors</label>
    <input id="pres-coauthors" type="text" bind:value={formCoauthors}
           placeholder="e.g. Jane Smith, Bob Lee" />
  </div>
{/if}
```

- [ ] **Step 8: Add card display for presentations in the source list**

In the left-panel card rendering (find where `source.source_type === 'role'` shows org name), add a venue display for presentations:

```svelte
{#if source.source_type === 'presentation' && source.presentation?.venue}
  <span class="card-org">{source.presentation.venue}</span>
{/if}
```

- [ ] **Step 9: Verify svelte-check**

```bash
cd packages/webui && bunx svelte-check --tsconfig ./tsconfig.json 2>&1 | grep -E "SourcesView|presentations/\+page|nav\.ts" | head -20 && cd ..
```
Expected: no errors.

- [ ] **Step 10: Commit**

```bash
git add packages/webui/src/lib/nav.ts \
        packages/webui/src/routes/experience/presentations/+page.svelte \
        packages/webui/src/lib/components/SourcesView.svelte
git commit -m "feat(webui): /experience/presentations page + SourcesView form fields"
```

---

### Task 6: Resume picker routing + compiler + DragNDropView

**Files:**
- Modify: `packages/webui/src/routes/resumes/+page.svelte`
- Modify: `packages/core/src/services/resume-compiler.ts`
- Modify: `packages/core/src/services/__tests__/resume-compiler.test.ts`
- Modify: `packages/webui/src/lib/components/resume/DragNDropView.svelte`

- [ ] **Step 1: Update openPicker in +page.svelte**

Find the `case 'presentations':` in the `openPicker` switch (currently it falls through to `default:`). Replace with:

```ts
case 'presentations':
  sourcePickerState = { sectionId, sourceType: 'presentation' }
  return
```

Remove `'presentations'` from the fallthrough that currently groups it with `'experience'`.

- [ ] **Step 2: Update buildPresentationItems compiler query**

Read `packages/core/src/services/resume-compiler.ts` and find `buildPresentationItems` (around line 725). Apply the LEFT JOIN + COALESCE refactor:

Replace the query with:

```sql
SELECT
  re.id AS entry_id,
  re.content AS entry_content,
  re.perspective_id,
  p.content AS perspective_content,
  p.bullet_id,
  b.content AS bullet_content,
  COALESCE(bs.source_id, re.source_id) AS source_id,
  s.title AS source_title,
  s.description AS source_description,
  s.end_date,
  sp.venue,
  sp.presentation_type,
  sp.url AS presentation_url,
  sp.coauthors
FROM resume_entries re
LEFT JOIN perspectives p ON p.id = re.perspective_id
LEFT JOIN bullets b ON b.id = p.bullet_id
LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
LEFT JOIN sources s ON s.id = COALESCE(bs.source_id, re.source_id)
LEFT JOIN source_presentations sp ON sp.source_id = s.id
WHERE re.section_id = ?
ORDER BY re.position ASC
```

Update the row type to make perspective/bullet/source fields nullable. Update the result mapping to include `description`, `venue`, `presentation_type`, `url` (from `presentation_url`), `coauthors` on `PresentationItem`. Apply the same `hasChain` guard for source_chain as in the other builders.

- [ ] **Step 3: Write compiler test**

```ts
test('presentation section renders direct-source entry with venue + type', () => {
  const resumeId = seedResume(db)
  const sourceId = seedSource(db, {
    title: 'Cloud Forensics at Scale',
    sourceType: 'presentation',
    description: 'Lessons from production IR',
  })
  db.run(
    `INSERT INTO source_presentations (source_id, venue, presentation_type, url, coauthors)
     VALUES (?, ?, ?, ?, ?)`,
    [sourceId, 'BSides DC 2024', 'conference_talk', 'https://slides.example.com', 'Jane Smith']
  )
  const secId = seedResumeSection(db, resumeId, 'Presentations', 'presentations', 0)
  seedResumeEntry(db, secId, { sourceId, position: 0 })

  const result = compileResumeIR(db, resumeId)!
  const presSection = result.sections.find(s => s.type === 'presentations')!
  expect(presSection.items).toHaveLength(1)
  const item = presSection.items[0] as PresentationItem
  expect(item.kind).toBe('presentation')
  expect(item.title).toBe('Cloud Forensics at Scale')
  expect(item.venue).toBe('BSides DC 2024')
  expect(item.presentation_type).toBe('conference_talk')
  expect(item.url).toBe('https://slides.example.com')
  expect(item.coauthors).toBe('Jane Smith')
  expect(item.description).toBe('Lessons from production IR')
})
```

- [ ] **Step 4: Run compiler tests**

```bash
bun test packages/core/src/services/__tests__/resume-compiler.test.ts 2>&1 | tail -8
```
Expected: all pass.

- [ ] **Step 5: Update DragNDropView presentations template**

Find the `{:else if section.type === 'presentations'}` block in DragNDropView.svelte. Update it to render venue + type below the title:

```svelte
{:else if section.type === 'presentations'}
  {#each section.items as item}
    {#if item.kind === 'presentation'}
      {@const pres = item as PresentationItem}
      <div class="project-item">
        <div class="project-header">
          <span class="project-name">{pres.title}</span>
          <span class="project-meta">
            {#if pres.date}<span class="project-date">{pres.date}</span>{/if}
            {#if onRemoveEntry && pres.entry_id}
              <button
                type="button"
                class="btn btn-xs btn-ghost entry-remove-btn"
                onclick={() => onRemoveEntry?.(pres.entry_id!)}
                aria-label="Remove {pres.title}"
                title="Remove this presentation"
              >&times;</button>
            {/if}
          </span>
        </div>
        {#if pres.venue}
          <span class="edu-degree">{pres.venue}{#if pres.presentation_type} · {pres.presentation_type.replace(/_/g, ' ')}{/if}</span>
        {/if}
        {#if pres.description}
          <p class="project-description">{pres.description}</p>
        {/if}
        {#each pres.bullets as bullet (bullet.entry_id)}
          <div
            class="bullet-item"
            class:cloned={bullet.is_cloned}
            onmouseenter={(e) => showTooltip(bullet, e)}
            onmouseleave={scheduleHideTooltip}
          >
            <span class="bullet-content">{bullet.content}</span>
          </div>
        {/each}
      </div>
    {/if}
  {/each}
```

Import `PresentationItem` from `@forge/sdk` at the top of the script if not already imported.

- [ ] **Step 6: Run full core + webui tests**

```bash
bun test packages/core/src/ 2>&1 | tail -5
bun test packages/webui/src/__tests__/ 2>&1 | tail -5
```
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add packages/webui/src/routes/resumes/+page.svelte \
        packages/core/src/services/resume-compiler.ts \
        packages/core/src/services/__tests__/resume-compiler.test.ts \
        packages/webui/src/lib/components/resume/DragNDropView.svelte
git commit -m "feat: resume picker routing + compiler + DragNDropView for presentations"
```

---

### Task 7: Verification + merge

- [ ] **Step 1: Full test suites**

```bash
bun test packages/core/src/ 2>&1 | tail -5
bun test packages/webui/src/__tests__/ 2>&1 | tail -5
```

- [ ] **Step 2: svelte-check on all touched files**

```bash
cd packages/webui && bunx svelte-check --tsconfig ./tsconfig.json 2>&1 | grep -E "SourcesView|presentations|DragNDropView|resumes/\+page" | head -20 && cd ..
```

- [ ] **Step 3: Check file overlap with main**

From the main repo cwd:
```bash
cd /Users/adam/notes/job-hunting
comm -12 \
  <(git log main..phase/presentations-entity --name-only --pretty=format: | grep -v "^Good\|^$" | sort -u) \
  <(git log phase/presentations-entity..main --name-only --pretty=format: | grep -v "^Good\|^$" | sort -u)
```
Expected: empty (no overlap). If overlap, rebase first.

- [ ] **Step 4: Merge to main**

If no overlap: fast-forward merge. If main drifted: rebase then fast-forward.
```bash
git checkout main
git merge --ff-only phase/presentations-entity
```

- [ ] **Step 5: Verify on merged main**

```bash
bun test packages/core/src/ 2>&1 | tail -3
bun test packages/webui/src/__tests__/ 2>&1 | tail -3
```

- [ ] **Step 6: Cleanup**

```bash
git worktree remove .claude/worktrees/presentations-entity
git branch -d phase/presentations-entity
```

- [ ] **Step 7: Manual smoke test**

In browser at `http://localhost:5173`:
1. Navigate to `/experience/presentations` → page loads with empty state
2. Create a new presentation: title="Cloud Forensics at Scale", venue="BSides DC 2024", type=Conference Talk, date, URL, coauthors
3. Verify it appears in the source list with venue shown
4. Edit it → verify fields round-trip
5. Go to `/resumes`, select a resume with a Presentations section
6. Click "+Add Entry" → SourcePicker should show the presentation source (not bullets/perspectives)
7. Add it → verify it renders in the DragNDropView with venue + type

---

## Acceptance Criteria

- [ ] `presentation` is a valid `source_type` in the DB schema
- [ ] `source_presentations` table exists with `venue`, `presentation_type`, `url`, `coauthors`
- [ ] CRUD for presentation sources works via the existing `/api/sources` endpoints
- [ ] `/experience/presentations` page renders using `<SourcesView>`
- [ ] Nav includes Presentations under the Experience group
- [ ] Resume picker routes 'presentations' to SourcePicker with `sourceType: 'presentation'`
- [ ] Compiler `buildPresentationItems` handles direct-source entries (LEFT JOIN + COALESCE)
- [ ] DragNDropView renders venue + type for presentation entries
- [ ] All core + webui tests pass
- [ ] svelte-check clean on touched files
