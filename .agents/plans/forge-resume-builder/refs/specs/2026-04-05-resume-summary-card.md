# Resume Summary Card — Design Spec

**Date:** 2026-04-05
**Beads:** `job-hunting-x03.2` (T95.2: Summary import into resume, P1)
**Parent epic:** Phase 95 — Resume Builder Polish
**Related plan:** `.claude/plans/forge-resume-builder/phase/95-resume-builder-polish.md` (T95.2)
**Status:** Approved, ready for implementation plan

## Summary

Wire the existing `summaries` entity into the resume editor so users can attach a summary to a resume via a picker or inline freeform input. A new top-of-resume **Summary Card** renders above the sections list in the drag-and-drop editor. The card supports four user journeys: pick a template, tweak a template locally without affecting other resumes (override), write a freeform summary from scratch, and promote a freeform summary into a reusable (non-template) summary row.

The data model reuses the existing `resumes.summary_id` FK (currently vestigial) and adds one new nullable column `resumes.summary_override TEXT` plus a companion timestamp. No new tables are introduced.

## Goals

1. Ship the T95.2 "Summary import into resume" P1 task that's been open on Phase 95's epic.
2. Make `resumes.summary_id` load-bearing instead of a dead FK.
3. Support local-to-this-resume edits that don't propagate to other resumes using the same template.
4. Let the user freeform-write a summary inline when no existing template fits.
5. Give an explicit path to "promote" a freeform summary into a reusable row.

## Non-goals

- Multiple summaries per resume (the convention is one summary at the top; if we ever need variants, a future epic can add a `resume_summary_variants` table).
- A dedicated "Summary" section type in `resume_sections.entry_type`. The summary lives in a card attached to the resume header, not in the sections list.
- LaTeX/markdown template rewrites. Exporters will be updated to read from `ir.summary` but that's a small downstream change, tracked as part of the implementation plan rather than as a separate blocker.
- Search/filter in the picker. With ~30 summaries today, a flat scrollable list is sufficient; search is a follow-up if the count grows.
- Deletion or archival of summaries from the picker UI — that remains on the `/resumes/summaries` page.

## Context

The `summaries` table already exists as a first-class entity (29 rows on the current DB, 6 flagged `is_template=1`). The `resumes.summary_id` FK column exists but is **never populated** on any resume and the IR compiler **reads it but doesn't use it** — it's vestigial from an earlier design.

The `resume_sections.entry_type` CHECK constraint does **not** include `'summary'`, so there's no "Summary" section type. Users today put summary-shaped text into `'freeform'` sections, which lose the `summaries` table provenance entirely. The compiler's `buildFreeformItems` function returns `SummaryItem[]` confusingly, but it's wired to `'freeform'` and `'awards'` sections.

This spec resolves that confusion by making the summary a first-class top-of-resume concept backed by the existing `summaries` table linkage.

## Data model

### Existing columns (untouched)

- `resumes.summary_id TEXT REFERENCES summaries(id) ON DELETE SET NULL` — becomes load-bearing. Points at the chosen summary row (template, non-template, or null for no summary / freeform-only case).

### New columns (one migration)

- `resumes.summary_override TEXT` — nullable. When non-null, its value replaces `summaries.description` when rendering this resume. When null, the linked summary's description is used as-is. This is the "local edit" layer that keeps per-resume tweaks from propagating to other resumes sharing the same template.
- `resumes.summary_override_updated_at TEXT` — nullable. Mirrors the pattern of `markdown_override_updated_at` and `latex_override_updated_at` already on the resumes table. Written whenever `summary_override` changes.

Migration number will be selected at implementation time to avoid collision with any other in-flight worktrees.

### Resolution order (mirrors Phase 92 tagline resolution)

When the IR compiler builds the summary card content:

1. `resumes.summary_override` (highest priority, user-authored local edit)
2. `summaries.description` via the linked `summary_id` (template reference)
3. `null` — card renders the empty state with a `[+ Select Summary]` button

### Title handling

- `summaries.title` is rendered as a small label above the body text on the card (e.g. "Senior Staff Engineer Summary") to help the user recognize which template they picked. It's **context for the editor**, not for the exported resume.
- `summaries.role` is not surfaced on the card. It remains metadata used by the summaries page's own filtering/grouping.
- When the user is in the freeform-only state (`summary_id = null`, `summary_override` set), there is no title label on the card — just the override text. This visually signals "this summary isn't linked to a template row".

## IR compiler output

### New top-level field: `ResumeDocument.summary: ResumeSummary | null`

```ts
interface ResumeSummary {
  summary_id: string | null     // FK provenance (null in freeform-only state)
  title: string | null          // summaries.title (null in freeform-only state)
  content: string               // resolved text: override ?? summaries.description ?? ''
  is_override: boolean          // true when resumes.summary_override took precedence
}
```

`summary` is a top-level field on `ResumeDocument`, not a field on `header`. Rationale:

- `header` is contact info + tagline — short single-line data. Summary is a paragraph and semantically distinct.
- The DragNDropView renders the summary card in a distinct slot above the sections list; the IR shape should match the UI's structural split.
- LaTeX/markdown exporters render the summary as its own block between the header and the first section, which is cleaner when it's a dedicated top-level field rather than buried in `header`.

### Compiler implementation

A new function `buildSummary` is added to `packages/core/src/services/resume-compiler.ts`:

```ts
function buildSummary(db: Database, resume: ResumeRow): ResumeSummary | null {
  const summaryId = resume.summary_id
  const override = resume.summary_override
  if (!summaryId && !override) return null

  let title: string | null = null
  let templateDescription: string | null = null
  if (summaryId) {
    const row = db
      .query('SELECT id, title, description FROM summaries WHERE id = ?')
      .get(summaryId) as { id: string; title: string; description: string | null } | null
    if (row) {
      title = row.title
      templateDescription = row.description
    }
  }

  const content = override ?? templateDescription ?? ''
  if (!content) return null  // nothing to display

  return {
    summary_id: summaryId,
    title,
    content,
    is_override: override !== null,
  }
}
```

Called once in `compileResumeIR` after `parseHeader`, and the result is assigned to `ResumeDocument.summary`. The base `SELECT` in `compileResumeIR` gains the new columns: `summary_override, summary_override_updated_at`. The `ResumeRow` interface in the same file is updated accordingly.

### Dangling FK handling

If `summary_id` points at a deleted summary, `ON DELETE SET NULL` on the FK clears it automatically. If a race or import path leaves a dangling reference, `buildSummary` returns `null` (the row lookup returns null) and the card renders the empty state. The user then picks a new summary, which overwrites the stale id.

## UI components

### `<ResumeSummaryCard>` — new component in `$lib/components/resume/`

Renders at the top of `DragNDropView`, between the existing `<HeaderEditor>` (if present) and the sections `{#each}` loop. Five visual states driven by `ir.summary` and a local edit-mode flag:

**State 1 — Empty (`ir.summary === null`):**
```
┌──────────────────────────────────────────────┐
│ Summary                                      │
│                                              │
│ No summary selected.                         │
│                                              │
│ [+ Select Summary]                           │
└──────────────────────────────────────────────┘
```

**State 2 — Template only (`summary_id` set, no override):**
```
┌──────────────────────────────────────────────┐
│ Summary                    [Change] [Unlink] │
│ ─ Senior Staff Engineer Template ─           │
│                                              │
│ 10+ years of experience architecting...      │
│ (summaries.description)                      │
│                                       [Edit] │
└──────────────────────────────────────────────┘
```

**State 3 — Template overridden (`is_override: true`):**
```
┌──────────────────────────────────────────────┐
│ Summary                [Change] [Unlink]     │
│ ─ Senior Staff Engineer Template ·  EDITED ─ │
│                                              │
│ (override text, takes precedence)            │
│                     [Reset to template] [Edit]│
└──────────────────────────────────────────────┘
```

**State 4 — Freeform only (`summary_id === null`, override set):**
```
┌──────────────────────────────────────────────┐
│ Summary                [Change] [Unlink]     │
│                                              │
│ (freeform override text, no title)           │
│                     [Promote to Template…] [Edit]│
└──────────────────────────────────────────────┘
```

**State 5 — Edit mode (sub-state of 2/3/4):**
```
┌──────────────────────────────────────────────┐
│ Summary                                      │
│ ─ (template label if applicable) ─           │
│                                              │
│ ┌──────────────────────────────────────────┐ │
│ │ <textarea bound to editing override>     │ │
│ │                                          │ │
│ └──────────────────────────────────────────┘ │
│                            [Cancel] [Save]   │
└──────────────────────────────────────────────┘
```

### Action semantics

| Button | Visible in | Behavior |
|---|---|---|
| `[+ Select Summary]` | State 1 | Open `<SummaryPickerModal>` |
| `[Change]` | States 2, 3, 4 | Open `<SummaryPickerModal>` (will replace whatever is currently linked; override is cleared on new template pick) |
| `[Unlink]` | States 2, 3, 4 | `PATCH` with `{ summary_id: null, summary_override: null }` — transitions to state 1 |
| `[Edit]` | States 2, 3, 4 | Enter state 5 with textarea prefilled from current `ir.summary.content` |
| `[Reset to template]` | State 3 only | `PATCH` with `{ summary_override: null }` — transitions back to state 2 (template's canonical text visible again) |
| `[Promote to Template…]` | State 4 only | Open title prompt modal → `POST /api/summaries` → `PATCH /api/resumes/:id` with new summary_id + clear override → transitions to state 2 |
| `[Save]` (in state 5) | State 5 | `PATCH` with `{ summary_override: <textarea value> }` — transitions to state 3 if `summary_id` was set, state 4 if it was null |
| `[Cancel]` (in state 5) | State 5 | Discard textarea contents, return to prior state |

### `<SummaryPickerModal>` — new component in `$lib/components/resume/`

Wraps the existing base `$lib/components/Modal.svelte` primitive. Loads summaries via `forge.summaries.list()` on open.

Layout (flat list with template badge, per approved Q1 option B):

```
┌──────────────────────────────────────────────┐
│ Select Summary                         [×]   │
├──────────────────────────────────────────────┤
│ ┌──────────────────────────────────────────┐ │
│ │ ⭐ Senior Staff Engineer Template         │ │
│ │ 10+ years of experience architecting...  │ │
│ └──────────────────────────────────────────┘ │
│ ┌──────────────────────────────────────────┐ │
│ │ ⭐ Security Engineer Template             │ │
│ │ Offensive and defensive security...      │ │
│ └──────────────────────────────────────────┘ │
│ ┌──────────────────────────────────────────┐ │
│ │    Anthropic Agent Infra                 │ │
│ │    (description preview, truncated)      │ │
│ └──────────────────────────────────────────┘ │
│  ...                                         │
├──────────────────────────────────────────────┤
│                    [ + Write my own ]        │
└──────────────────────────────────────────────┘
```

**Sort order:** `is_template DESC, title ASC` — templates pinned to top, alphabetical within each group.

**Row click:** fires `onselect(summary.id)`. Parent PATCHes `{ summary_id: <id>, summary_override: null }` (clearing any prior override).

**`[+ Write my own]` button:** switches the modal into freeform entry mode — a single textarea with `[Cancel]` and `[Save freeform]`. Saving fires `onfreeform(text)` which PATCHes `{ summary_id: null, summary_override: <text> }`. This takes the user into state 4.

**Search:** not included in v1 per approved Q2.

### `DragNDropView.svelte` — modifications

- New prop: `onUpdateSummary?: (update: { summary_id?: string \| null; summary_override?: string \| null }) => Promise<void>`
- Render `<ResumeSummaryCard>` immediately after the header area and before the sections `{#each}` loop.
- The card receives `ir.summary` as a prop and forwards user actions back to `onUpdateSummary`, which the parent wires to the resume update SDK call.

### `+page.svelte` (resumes route) — wiring

- Pass `onUpdateSummary` to `<DragNDropView>`:
  ```ts
  onUpdateSummary={async (update) => {
    const result = await forge.resumes.update(selectedResumeId, update)
    if (result.ok) {
      await loadIR(selectedResumeId)
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  }}
  ```

## API surface

All summary card actions route through the **existing** `PATCH /api/resumes/:id` endpoint, plus the **existing** `POST /api/summaries` endpoint for Promote.

### Type updates

**`packages/core/src/types/index.ts`:**

- `UpdateResume` gains two optional fields:
  ```ts
  interface UpdateResume {
    // ...existing fields
    summary_id?: string | null
    summary_override?: string | null
  }
  ```
- `Resume` (the base entity type) gains `summary_override: string | null` and `summary_override_updated_at: string | null`.
- `ResumeDocument` (IR) gains `summary: ResumeSummary | null`.
- New `ResumeSummary` interface (shape above).

**`packages/sdk/src/types.ts`** mirrors all of the above.

### Repository updates

`packages/core/src/db/repositories/resume-repository.ts` `update` function adds both fields to its set-clause builder, using the project's existing `'in' in input` convention so `null` means "clear" and `undefined` means "leave alone". Writes to `summary_override` also update `summary_override_updated_at`.

### No new routes required

The existing `PATCH /api/resumes/:id` route body-spreads the update payload straight to the repository, so no route changes are needed.

## Migration

A new migration adds two columns:

```sql
ALTER TABLE resumes ADD COLUMN summary_override TEXT;
ALTER TABLE resumes ADD COLUMN summary_override_updated_at TEXT;
```

Both nullable. No backfill needed — every existing resume keeps its current behavior (summary_id is null on all of them today, and the new override column defaults to null).

Migration number chosen at implementation time to avoid collision with any in-flight parallel worktrees. Implementation task MUST check for the highest migration on `main` at the moment of commit and use `<max + 1>`, not pre-allocate.

## Testing

### Repository (Vitest-equivalent, bun:test + in-memory DB)

- `update(resume, { summary_id })` sets the FK and leaves other fields alone
- `update(resume, { summary_override: 'text' })` sets the override AND `summary_override_updated_at`
- `update(resume, { summary_override: null })` clears the override
- `update(resume, { summary_id: null, summary_override: null })` clears both in one call (Unlink behavior)
- Undefined vs null semantics: `undefined` leaves a field alone, `null` clears it

### Compiler (resume-compiler.test.ts)

- `buildSummary` returns `null` when both `summary_id` and `summary_override` are null
- `buildSummary` returns `{ title, content: description, is_override: false }` when only `summary_id` is set (template mode)
- `buildSummary` returns `{ title, content: override, is_override: true }` when both are set (template override mode)
- `buildSummary` returns `{ title: null, content: override, is_override: true }` when only `summary_override` is set (freeform mode)
- `buildSummary` handles dangling `summary_id` (deleted summary) by treating it as if only the override existed; returns `null` if both are effectively empty

### Structural (webui bun:test + readFileSync)

- `<ResumeSummaryCard>` source file contains the expected state-machine branches (`State 1 ... State 5`) keyed on `summary === null`, `is_override`, and the edit-mode local flag
- `<ResumeSummaryCard>` contains the expected action handlers with correct PATCH payload shapes (each of the 7 action buttons in the table)
- `<SummaryPickerModal>` source file wraps base `Modal`, loads summaries via the SDK, sorts by `is_template DESC, title ASC`, and fires `onselect`/`onfreeform` callbacks
- `DragNDropView.svelte` imports and renders `<ResumeSummaryCard>` in the right position, and declares the `onUpdateSummary` prop
- `resumes/+page.svelte` wires `onUpdateSummary` to the resume update SDK + IR reload

No runtime rendering tests — the project's webui test convention is source-string assertion, and runtime verification comes from svelte-check + manual smoke.

## Files affected

### Create
- `packages/webui/src/lib/components/resume/ResumeSummaryCard.svelte`
- `packages/webui/src/lib/components/resume/SummaryPickerModal.svelte`
- `packages/core/src/db/migrations/<N>_resume_summary_override.sql` (N chosen at implementation time)
- `packages/webui/src/__tests__/resume-summary-card.test.ts` (structural tests)

### Modify
- `packages/core/src/types/index.ts` — add `summary_id`/`summary_override` to `UpdateResume`, add fields to `Resume`, add `ResumeSummary` interface, add `summary` field to `ResumeDocument`
- `packages/sdk/src/types.ts` — mirror the above
- `packages/core/src/db/repositories/resume-repository.ts` — add the two new fields to the `update` set-clause builder, bump `summary_override_updated_at` when override changes
- `packages/core/src/db/repositories/__tests__/resume-repository.test.ts` — new tests for summary_id / summary_override persistence
- `packages/core/src/services/resume-compiler.ts` — new `buildSummary` function, wire into `compileResumeIR`, update base SELECT to include new columns
- `packages/core/src/services/__tests__/resume-compiler.test.ts` — new tests for `buildSummary` resolution logic
- `packages/webui/src/lib/components/resume/DragNDropView.svelte` — new `onUpdateSummary` prop, render `<ResumeSummaryCard>` above sections
- `packages/webui/src/routes/resumes/+page.svelte` — wire `onUpdateSummary` handler
- `packages/core/src/db/__tests__/migrate.test.ts` — update migration count + add entry for the new migration file

## Risks & verification points

1. **Migration number collision with parallel worktrees.** The phase-92 → 034/035 renumber earlier this week showed this is a real hazard. Implementation task must look at `main`'s migration files at the moment of committing and pick `<max + 1>`. The migration file itself is additive and won't conflict semantically with anything.
2. **LaTeX/markdown exporters need updates.** Today they may render the freeform section as a summary. Once the summary moves to its own IR field, the exporters need to read from `ir.summary` and emit a summary block between the header and sections. This is a downstream concern the implementation plan will address.
3. **Existing resumes with "summary content in a freeform section".** The user currently uses `freeform` sections for summary-like content. After this feature ships, those existing freeform sections still exist as freeform — they don't auto-migrate to the new summary card. The user can manually copy their existing freeform text into a summary (either freeform entry or a new summary row) if they want the new card pattern. No automated migration is in scope.
4. **`summaries.role` currently required/used?** Check the summaries service/routes to confirm creating a new summary row with only `title + description + is_template` is accepted by the existing POST endpoint. If the endpoint requires `role`, the freeform Promote flow needs to provide a default (e.g. use the resume's `target_role`).

## Out-of-scope follow-ups

- LaTeX/markdown exporter updates to render the summary block (tracked in the implementation plan but shippable separately if needed)
- Picker search input (ship only if the summary list grows past ~50-100 entries and feels unwieldy)
- Multiple summaries per resume (variants) — would require a new junction table
- A dedicated `'summary'` section type in `resume_sections.entry_type` — explicitly rejected in favor of the header-attached card pattern
- Summary card in the resume list/card view on `/resumes` — only the editor gets the card for now
