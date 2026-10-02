# Phase 24: Provenance Tooltip Enhancement

**Goal:** Replace UUID-only provenance tooltips in the DragNDrop resume view with human-readable names (source title, bullet preview, perspective preview) and clickable links that navigate to the Chain View with the relevant node highlighted.

**Non-Goals:** Full inline chain visualization in the tooltip. Editing source/bullet/perspective from the tooltip. Tooltips for non-experience items (skills, education sections don't have `source_chain`). Mobile/touch tooltip support. Tooltip viewport boundary clamping.

**Depends on:** Phase 19 (IR compiler), Phase 25 (both modify chain/+page.svelte -- Phase 25's restructuring must land first)
**Blocks:** Nothing

**Internal task parallelization:** T24.1 must complete before T24.2. T24.2 must complete before T24.3. T24.3 and T24.4 are parallelizable (both depend on T24.2 but not on each other). T24.5 depends on T24.2 and T24.4.

**Tech Stack:** TypeScript, Svelte 5 runes, SvelteKit, SQLite (via `bun:sqlite`), Sigma.js

**Reference:** `refs/specs/2026-03-30-provenance-tooltip-enhancement.md`

**Architecture:**
- Type changes in `packages/sdk/src/types.ts` and `packages/core/src/types/index.ts` (must stay in sync)
- IR compiler enrichment in `packages/core/src/services/resume-compiler.ts` (single-source change -- all views that consume `source_chain` benefit)
- Tooltip UI in `packages/webui/src/lib/components/resume/DragNDropView.svelte`
- Highlight support in `packages/webui/src/routes/chain/+page.svelte`

**Fallback Strategies:**
- If the `bullets` table JOIN causes a performance regression on large datasets, add an index on `bullets.id` (should already be the PK, so unlikely)
- If tooltip hover-delay logic causes flicker on fast mouse movements, increase the grace period from 150ms to 250ms
- If Sigma's `getNodeDisplayData()` returns null (node not yet rendered), fall back to raw graph attributes with a console warning

---

## Context

Currently, hovering over a resume bullet in the DragNDrop view shows raw UUIDs for the provenance chain:

```
Source: 550e8400-e29b-41d4-a716-446655440000
Bullet: 661f9511-f30c-42e5-b827-557766554411
Perspective: 772g0622-g41d-53f6-c938-668877665522
Content has been manually edited
```

This is meaningless to users. The IR compiler (`resume-compiler.ts`) already joins `perspectives` and `bullet_sources` but does NOT join the `bullets` table, so the original bullet content is unavailable. The `source_chain` field on `ExperienceBullet` currently stores only IDs.

After this phase, the tooltip will show:

```
Source: Principal Cloud Forensics Engineer    ->
Bullet: Led 4-engineer team migrating clo... ->
Perspective: Led cloud platform migration... ->
```

Where each `->` is a clickable link to `/chain?highlight={nodeType}-{id}`.

---

## Tasks

### Task 24.1: Update ExperienceBullet source_chain types

**Files to modify:**
- `packages/sdk/src/types.ts`
- `packages/core/src/types/index.ts`

**Goal:** Add `source_title`, `bullet_preview`, and `perspective_preview` fields to the `source_chain` type on `ExperienceBullet`. Both files must have identical `source_chain` field definitions.

#### Steps

- [ ] **Modify `packages/sdk/src/types.ts`** -- update the `ExperienceBullet` interface's `source_chain` field:

```typescript
export interface ExperienceBullet {
  content: string
  entry_id: string | null
  source_chain?: {
    source_id: string
    source_title: string              // e.g., "Principal Cloud Forensics Engineer"
    bullet_id: string
    bullet_preview: string            // first 60 chars of ORIGINAL bullet content
    perspective_id: string
    perspective_preview: string       // first 60 chars of perspective content
  }
  is_cloned: boolean
}
```

- [ ] **Modify `packages/core/src/types/index.ts`** -- apply the exact same change to the `ExperienceBullet` interface:

```typescript
export interface ExperienceBullet {
  content: string
  entry_id: string | null
  source_chain?: {
    source_id: string
    source_title: string
    bullet_id: string
    bullet_preview: string
    perspective_id: string
    perspective_preview: string
  }
  is_cloned: boolean
}
```

#### Acceptance Criteria
- [ ] Both `ExperienceBullet.source_chain` definitions include `source_title`, `bullet_preview`, `perspective_preview`
- [ ] Field order is identical in both files: `source_id`, `source_title`, `bullet_id`, `bullet_preview`, `perspective_id`, `perspective_preview`
- [ ] Existing fields (`source_id`, `bullet_id`, `perspective_id`) are preserved
- [ ] TypeScript compiles in both packages

---

**IMPORTANT: T24.1 and T24.2 must be implemented and committed as a single atomic change.** Updating the `source_chain` type (T24.1) to add required fields (`source_title`, `bullet_preview`, `perspective_preview`) will cause TypeScript errors in `resume-compiler.ts` (which still constructs the old shape) until T24.2 updates the compiler. Do NOT commit T24.1 separately -- the build will be broken between T24.1 and T24.2.

### Task 24.2: Enrich source_chain in IR compiler

**File to modify:** `packages/core/src/services/resume-compiler.ts`

**Goal:** Add `JOIN bullets b ON b.id = p.bullet_id` to all three builder queries (experience, projects, presentations), extend row types with `bullet_content`, and populate enriched `source_chain` with truncated previews.

#### Steps

- [ ] **Add a `truncate` helper function** near the top of the file (after imports, before the compiler function):

```typescript
/** Truncate text to `len` characters, appending '...' if truncated. */
function truncate(text: string | null, len = 60): string {
  if (!text) return ''
  if (text.length <= len) return text
  return text.slice(0, len) + '...'
}
```

- [ ] **Extend `ExperienceEntryRow`** -- add `bullet_content`:

```typescript
interface ExperienceEntryRow {
  entry_id: string
  entry_content: string | null
  perspective_id: string
  perspective_content: string
  bullet_id: string
  bullet_content: string              // NEW: original bullet text from bullets table
  source_id: string
  source_title: string
  organization_id: string | null
  org_name: string | null
  start_date: string | null
  end_date: string | null
  is_current: number
  position: number
}
```

- [ ] **Update `buildExperienceSection` query** -- add the JOIN and SELECT:

```sql
SELECT
  re.id AS entry_id,
  re.content AS entry_content,
  re.perspective_id,
  re.position,
  p.content AS perspective_content,
  p.bullet_id,
  b.content AS bullet_content,
  bs.source_id,
  s.title AS source_title,
  sr.organization_id,
  sr.start_date,
  sr.end_date,
  sr.is_current,
  o.name AS org_name
FROM resume_entries re
JOIN perspectives p ON p.id = re.perspective_id
JOIN bullets b ON b.id = p.bullet_id
JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
JOIN sources s ON s.id = bs.source_id
LEFT JOIN source_roles sr ON sr.source_id = s.id
LEFT JOIN organizations o ON o.id = sr.organization_id
WHERE re.resume_id = ? AND re.section IN ('experience', 'work_history')
ORDER BY sr.is_current DESC, sr.start_date DESC, re.position ASC
```

The key change is `JOIN bullets b ON b.id = p.bullet_id` and `b.content AS bullet_content` in the SELECT.

- [ ] **Update experience `source_chain` population** in `buildExperienceSection`:

```typescript
const bullets: ExperienceBullet[] = entries.map(e => ({
  content: e.entry_content ?? e.perspective_content,
  entry_id: e.entry_id,
  source_chain: {
    source_id: e.source_id,
    source_title: truncate(e.source_title, 60),
    bullet_id: e.bullet_id,
    bullet_preview: truncate(e.bullet_content, 60),
    perspective_id: e.perspective_id,
    perspective_preview: truncate(e.perspective_content, 60),
  },
  is_cloned: e.entry_content !== null,
}))
```

- [ ] **Update `buildProjectsSection` query** -- add `JOIN bullets b ON b.id = p.bullet_id` and `b.content AS bullet_content`:

```sql
SELECT
  re.id AS entry_id,
  re.content AS entry_content,
  re.perspective_id,
  p.content AS perspective_content,
  p.bullet_id,
  b.content AS bullet_content,
  bs.source_id,
  s.title AS source_title,
  sp.start_date,
  sp.end_date
FROM resume_entries re
JOIN perspectives p ON p.id = re.perspective_id
JOIN bullets b ON b.id = p.bullet_id
JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
JOIN sources s ON s.id = bs.source_id
LEFT JOIN source_projects sp ON sp.source_id = s.id
WHERE re.resume_id = ? AND re.section = 'projects'
ORDER BY re.position ASC
```

- [ ] **Update projects anonymous row type** to include `bullet_content: string`

- [ ] **Update projects `source_chain` population:**

```typescript
bullets: entries.map(e => ({
  content: e.entry_content ?? e.perspective_content,
  entry_id: e.entry_id,
  source_chain: {
    source_id: e.source_id,
    source_title: truncate(e.source_title, 60),
    bullet_id: e.bullet_id,
    bullet_preview: truncate(e.bullet_content, 60),
    perspective_id: e.perspective_id,
    perspective_preview: truncate(e.perspective_content, 60),
  },
  is_cloned: e.entry_content !== null,
})),
```

- [ ] **Update `buildPresentationsSection` query** -- add `JOIN bullets b ON b.id = p.bullet_id` and `b.content AS bullet_content`:

```sql
SELECT
  re.id AS entry_id,
  re.content AS entry_content,
  re.perspective_id,
  p.content AS perspective_content,
  p.bullet_id,
  b.content AS bullet_content,
  bs.source_id,
  s.title AS source_title,
  s.end_date
FROM resume_entries re
JOIN perspectives p ON p.id = re.perspective_id
JOIN bullets b ON b.id = p.bullet_id
JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
JOIN sources s ON s.id = bs.source_id
WHERE re.resume_id = ? AND re.section = 'presentations'
ORDER BY re.position ASC
```

- [ ] **Update presentations anonymous row type** to include `bullet_content: string`

- [ ] **Update presentations `source_chain` population** (same pattern as projects above)

#### Acceptance Criteria
- [ ] `ExperienceEntryRow` includes `bullet_content: string`
- [ ] All three builder queries (`buildExperienceSection`, `buildProjectsSection`, `buildPresentationsSection`) JOIN the `bullets` table
- [ ] All three `source_chain` populations include `source_title`, `bullet_preview`, `perspective_preview`
- [ ] `source_title` is truncated to 60 chars
- [ ] `bullet_preview` uses `e.bullet_content` (ORIGINAL bullet text from bullets table), NOT `e.perspective_content`
- [ ] `perspective_preview` uses `e.perspective_content` (perspective reframing)
- [ ] `truncate()` helper appends `...` when text exceeds the limit
- [ ] `GET /api/resumes/:id/ir` returns enriched `source_chain` fields
- [ ] TypeScript compiles

---

### Task 24.3: Update DragNDropView tooltip

**File to modify:** `packages/webui/src/lib/components/resume/DragNDropView.svelte`

**Goal:** Replace the UUID display with human-readable names and clickable links. Add hover-delay dismissal logic so users can click links. Add tooltip handlers to project and presentation bullets.

#### Steps

- [ ] **Add hover-delay state and functions** (in the `<script>` block, after the existing `tooltipPosition` state):

```typescript
let tooltipHovered = $state(false)
let dismissTimer: ReturnType<typeof setTimeout> | null = null

function showTooltip(bullet: ExperienceBullet, e: MouseEvent) {
  if (!bullet.source_chain) return
  if (dismissTimer) { clearTimeout(dismissTimer); dismissTimer = null }
  tooltipEntry = bullet
  tooltipPosition = { x: e.clientX + 10, y: e.clientY + 10 }
}

function scheduleHideTooltip() {
  dismissTimer = setTimeout(() => {
    if (!tooltipHovered) {
      tooltipEntry = null
    }
  }, 150)  // 150ms grace period to reach the tooltip
}
```

Note: this replaces the existing inline `showTooltip` function (lines 113-117 of the current file). Remove the old one.

- [ ] **Update experience bullet `onmouseleave`** -- change from `() => tooltipEntry = null` to `{scheduleHideTooltip}`:

```svelte
<div
  class="bullet-item"
  class:cloned={bullet.is_cloned}
  role="listitem"
  onmouseenter={(e) => showTooltip(bullet, e)}
  onmouseleave={scheduleHideTooltip}
>
```

- [ ] **Add tooltip handlers to project bullets** (currently around lines 254-258, inside the `{#each proj.bullets}` loop):

```svelte
{#each proj.bullets as bullet (bullet.entry_id)}
  <div
    class="bullet-item"
    class:cloned={bullet.is_cloned}
    onmouseenter={(e) => showTooltip(bullet, e)}
    onmouseleave={scheduleHideTooltip}
  >
    <span class="bullet-content">{bullet.content}</span>
  </div>
{/each}
```

- [ ] **Add tooltip handlers to presentation bullets** (in the generic fallback section, currently around lines 278-283):

```svelte
{#if 'bullets' in item && Array.isArray((item as PresentationItem).bullets)}
  {#each (item as PresentationItem).bullets as bullet}
    <div
      class="bullet-item"
      onmouseenter={(e) => showTooltip(bullet, e)}
      onmouseleave={scheduleHideTooltip}
    >
      <span class="bullet-content">{bullet.content}</span>
    </div>
  {/each}
{/if}
```

- [ ] **Replace the tooltip template** (lines 298-308) with the enriched version:

```svelte
<!-- Provenance Tooltip -->
{#if tooltipEntry?.source_chain}
  <div
    class="provenance-tooltip"
    style="left: {tooltipPosition.x}px; top: {tooltipPosition.y}px;"
    onmouseenter={() => { tooltipHovered = true; if (dismissTimer) clearTimeout(dismissTimer) }}
    onmouseleave={() => { tooltipHovered = false; tooltipEntry = null }}
  >
    <div class="tooltip-row">
      <strong>Source:</strong>
      <span class="tooltip-label">{tooltipEntry.source_chain.source_title}</span>
      <a
        class="tooltip-link"
        href="/chain?highlight=source-{tooltipEntry.source_chain.source_id}"
      >&rarr;</a>
    </div>
    <div class="tooltip-row">
      <strong>Bullet:</strong>
      <span class="tooltip-label">{tooltipEntry.source_chain.bullet_preview}</span>
      <a
        class="tooltip-link"
        href="/chain?highlight=bullet-{tooltipEntry.source_chain.bullet_id}"
      >&rarr;</a>
    </div>
    <div class="tooltip-row">
      <strong>Perspective:</strong>
      <span class="tooltip-label">{tooltipEntry.source_chain.perspective_preview}</span>
      <a
        class="tooltip-link"
        href="/chain?highlight=perspective-{tooltipEntry.source_chain.perspective_id}"
      >&rarr;</a>
    </div>
    {#if tooltipEntry.is_cloned}
      <div class="tooltip-row tooltip-cloned">Content has been manually edited</div>
    {/if}
  </div>
{/if}
```

- [ ] **Update tooltip CSS** -- change `pointer-events: none` to `pointer-events: auto` and update `max-width`, add link styles:

```css
.provenance-tooltip {
  position: fixed;
  z-index: 10000;
  background: #1a1a2e;
  color: #e0e0e0;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  font-size: 0.7rem;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  pointer-events: auto;          /* was: none */
  max-width: 350px;              /* was: 320px */
}

.tooltip-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tooltip-link {
  color: #6c63ff;
  text-decoration: none;
  margin-left: 0.35rem;
  flex-shrink: 0;
}

.tooltip-link:hover {
  text-decoration: underline;
}
```

- [ ] **Update `.tooltip-row` CSS** to use flex layout for link alignment:

```css
.tooltip-row {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  margin-bottom: 0.15rem;
}
```

Remove `word-break: break-all;` from `.tooltip-row` (no longer needed since text is truncated at the IR level and CSS ellipsis handles overflow).

#### Acceptance Criteria
- [ ] Tooltip shows source title instead of UUID
- [ ] Tooltip shows bullet content preview instead of UUID
- [ ] Tooltip shows perspective content preview instead of UUID
- [ ] Each row has a clickable `->` link to `/chain?highlight=...`
- [ ] Links use accent color (`#6c63ff`), underline on hover
- [ ] Edited indicator still shown when `is_cloned` is true
- [ ] Tooltip max width 350px
- [ ] `pointer-events: auto` on `.provenance-tooltip`
- [ ] Hover-delay logic: 150ms grace period with `tooltipHovered` state
- [ ] Project bullets show tooltip with enriched `source_chain`
- [ ] Presentation bullets show tooltip with enriched `source_chain`

---

### Task 24.4: Chain View highlight support

**File to modify:** `packages/webui/src/routes/chain/+page.svelte`

**Goal:** Read the `?highlight=source-{uuid}` query param on mount and center/highlight the specified node in the graph.

#### Steps

- [ ] **Add highlight logic** inside the `initSigma()` function, after `sigmaInstance = instance` is set:

```typescript
// Handle ?highlight= query param (e.g., from provenance tooltip links)
const highlight = new URLSearchParams(window.location.search).get('highlight')
if (highlight && graph!.hasNode(highlight)) {
  // Use the full highlight value as the graph node key -- NO split on hyphen.
  // UUIDs contain hyphens and will be truncated if split.
  const nodeDisplayData = instance.getNodeDisplayData(highlight)
  if (nodeDisplayData) {
    instance.getCamera().animate(
      { x: nodeDisplayData.x, y: nodeDisplayData.y, ratio: 0.3 },
      { duration: 500 }
    )
  }

  // Increase node size for visual emphasis
  const attrs = graph!.getNodeAttributes(highlight)
  graph!.setNodeAttribute(highlight, 'size', attrs.size * 2)
  // Sigma's built-in highlight ring depends on this attribute.
  // Without it, only the size change is visible.
  graph!.setNodeAttribute(highlight, 'highlighted', true)

  // Select the node to open the detail panel
  selectedNodeId = highlight
  selectedNodeData = {
    id: attrs.entityId,
    label: attrs.label,
    type: attrs.entityType,
    content: attrs.content,
    status: attrs.status,
    sourceType: attrs.sourceType,
    archetype: attrs.archetype,
    domain: attrs.domain,
  }
}
```

Important implementation notes:
- The `highlight` value (e.g., `source-550e8400-e29b-41d4-a716-446655440000`) is already the graph node key as-is. Do NOT use `split('-')` -- UUIDs contain hyphens.
- For extracting the node type (if needed for UI), use `highlight.slice(0, highlight.indexOf('-'))` which yields `'source'`, `'bullet'`, or `'perspective'`.
- Use `renderer.getNodeDisplayData(highlight)` for camera positioning, NOT raw graph attributes (`attrs.x`, `attrs.y`) -- those are in graph-space, not camera-space.
- If the node is not found in the graph (filtered out, or invalid ID), ignore silently.

#### Acceptance Criteria
- [ ] Reads `?highlight=source-{id}` query param on mount
- [ ] Camera centers on highlighted node using `getNodeDisplayData()`
- [ ] Highlighted node has increased size (2x)
- [ ] Highlighted node has `highlighted: true` attribute set (for Sigma's highlight ring)
- [ ] Detail panel opens for the highlighted node
- [ ] Graceful handling when highlight param references non-existent node (silent ignore)
- [ ] UUIDs with hyphens are NOT truncated (full key used for lookup)

---

### Task 24.5: Tests

**Files to create/modify:**
- `packages/core/src/services/__tests__/resume-compiler.test.ts` (create or extend)

**Goal:** Write tests verifying the enriched `source_chain` fields.

#### Steps

- [ ] **IR compiler test: enriched source_chain has title and previews**

```typescript
test('source_chain includes source_title, bullet_preview, perspective_preview', () => {
  // Setup: insert source, bullet, perspective, resume, resume_entry
  // ...
  const ir = compileResumeIR(db, resumeId)
  const section = ir!.sections.find(s => s.type === 'experience')!
  const group = section.items[0] as ExperienceGroup
  const bullet = group.subheadings[0].bullets[0]

  expect(bullet.source_chain).toBeDefined()
  expect(bullet.source_chain!.source_title).toBeDefined()
  expect(bullet.source_chain!.source_title.length).toBeGreaterThan(0)
  expect(bullet.source_chain!.bullet_preview).toBeDefined()
  expect(bullet.source_chain!.bullet_preview.length).toBeGreaterThan(0)
  expect(bullet.source_chain!.perspective_preview).toBeDefined()
  expect(bullet.source_chain!.perspective_preview.length).toBeGreaterThan(0)
})
```

- [ ] **IR compiler test: bullet_preview != perspective_preview when contents differ**

```typescript
test('bullet_preview uses original bullet content, perspective_preview uses reframing', () => {
  // Setup: create bullet with content "Original bullet about cloud migration"
  //        create perspective with content "Led cloud platform migration initiative"
  // ...
  const ir = compileResumeIR(db, resumeId)
  const section = ir!.sections.find(s => s.type === 'experience')!
  const group = section.items[0] as ExperienceGroup
  const bullet = group.subheadings[0].bullets[0]

  expect(bullet.source_chain!.bullet_preview).not.toEqual(
    bullet.source_chain!.perspective_preview
  )
  expect(bullet.source_chain!.bullet_preview).toContain('Original bullet')
  expect(bullet.source_chain!.perspective_preview).toContain('Led cloud')
})
```

- [ ] **IR compiler test: source_title truncated for long titles**

```typescript
test('source_title truncated to 60 characters for long titles', () => {
  // Setup: create source with title longer than 60 chars
  const longTitle = 'Principal Cloud Forensics Engineer and Platform Security Lead for Enterprise Division'
  // ... insert with longTitle ...
  const ir = compileResumeIR(db, resumeId)
  const section = ir!.sections.find(s => s.type === 'experience')!
  const group = section.items[0] as ExperienceGroup
  const bullet = group.subheadings[0].bullets[0]

  expect(bullet.source_chain!.source_title.length).toBeLessThanOrEqual(63) // 60 + '...'
  expect(bullet.source_chain!.source_title).toEndWith('...')
})
```

- [ ] **IR compiler test: buildProjectsSection enriches source_chain with previews**

```typescript
test('buildProjectsSection enriches source_chain with previews', () => {
  // Seed source + bullet + perspective + resume + resume_entry with section='projects'
  // Compile IR
  // Find projects section, verify source_chain has source_title, bullet_preview, perspective_preview
})
```

- [ ] **IR compiler test: buildPresentationsSection enriches source_chain with previews**

```typescript
test('buildPresentationsSection enriches source_chain with previews', () => {
  // Same pattern for presentations section
})
```

- [ ] **Chain View test: highlight param with UUID-containing-hyphens resolves correctly**

This is a smoke/manual test since it requires a running Sigma.js instance:

```
Manual Test:
1. Navigate to /chain?highlight=source-550e8400-e29b-41d4-a716-446655440000
2. Verify: the graph centers on the source node (if it exists)
3. Verify: the full UUID is used as the node key (not truncated at the first hyphen)
4. Navigate to /chain?highlight=bullet-661f9511-f30c-42e5-b827-557766554411
5. Verify: bullet node centered and selected
6. Navigate to /chain?highlight=nonexistent-node
7. Verify: no error, graph loads normally
```

#### Acceptance Criteria
- [ ] Test: enriched source_chain has title and previews
- [ ] Test: `bullet_preview != perspective_preview` when contents differ
- [ ] Test: `source_title` truncated to 60 chars for long titles
- [ ] Test: `buildProjectsSection` enriches source_chain with previews
- [ ] Test: `buildPresentationsSection` enriches source_chain with previews
- [ ] Test: Chain View highlight with UUID-containing-hyphens resolves correctly (manual)

---

### Task 24.6: Documentation

**Files:** None

**Goal:** No new ADRs or documentation files needed. This phase is a UI enhancement with no architectural decisions.

#### Acceptance Criteria
- [ ] JSDoc on the `truncate` helper in `resume-compiler.ts`
- [ ] Code comments explaining the hover-delay dismissal logic in `DragNDropView.svelte`
- [ ] Code comments explaining why `split('-')` is NOT used for highlight param parsing
- [ ] Update `PLAN.md` status line to mark this phase as complete

---

## Testing Requirements

| Category | Test | Location |
|----------|------|----------|
| Unit | Enriched source_chain has title and previews | `packages/core/src/services/__tests__/resume-compiler.test.ts` |
| Unit | `bullet_preview != perspective_preview` | `packages/core/src/services/__tests__/resume-compiler.test.ts` |
| Unit | `source_title` truncated for long titles | `packages/core/src/services/__tests__/resume-compiler.test.ts` |
| Unit | `buildProjectsSection` enriches source_chain | `packages/core/src/services/__tests__/resume-compiler.test.ts` |
| Unit | `buildPresentationsSection` enriches source_chain | `packages/core/src/services/__tests__/resume-compiler.test.ts` |
| Smoke | Tooltip shows source title, not UUID | Manual |
| Smoke | Tooltip shows bullet preview, not UUID | Manual |
| Smoke | Tooltip shows perspective preview, not UUID | Manual |
| Smoke | Tooltip links navigate to Chain View | Manual |
| Smoke | Project bullet tooltip works | Manual |
| Smoke | Presentation bullet tooltip works | Manual |
| Smoke | Tooltip hover-delay allows clicking links | Manual |
| Smoke | Chain View highlight param centers and selects node | Manual |
| Regression | Chain View highlight with UUID hyphens not truncated | Manual |

---

## Documentation Requirements

- [ ] JSDoc on `truncate()` helper in `resume-compiler.ts`
- [ ] Inline comments on hover-delay dismissal logic
- [ ] Inline comments on highlight param UUID parsing
- [ ] Update `PLAN.md` status line to mark this phase as complete
- [ ] No new documentation files needed
- [ ] No new ADRs needed
