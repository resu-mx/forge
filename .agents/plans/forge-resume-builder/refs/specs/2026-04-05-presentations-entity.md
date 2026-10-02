# Presentations Entity — Design Spec

**Date:** 2026-04-05
**Status:** Approved, ready for implementation plan

## Summary

Add `presentation` as a first-class source type with a `source_presentations` extension table, CRUD through the existing polymorphic sources API, a new `/experience/presentations` page in the webui, and proper resume picker routing so the Presentations section on a resume pulls from presentation sources instead of falling through to the generic perspective picker.

## Goals

1. Ship a `presentation` source type so users can track conference talks, workshops, webinars, and other speaking engagements as structured data.
2. Wire the resume editor's Presentations section to the new source type via the existing SourcePicker pattern.
3. Update the IR compiler's `buildPresentationItems` to support direct-source entries (LEFT JOIN + COALESCE, same refactor done for education/cert/clearance/project).
4. Add a `/experience/presentations` page following the established SplitPanel + editor pattern.

## Non-goals

- A dedicated PresentationPicker component (SourcePicker is sufficient for v1).
- Audience size, event_name separate from venue, or organization_id on presentations — YAGNI for v1.
- Publications, patents, or other "outputs" entities — separate future work if needed.
- LaTeX/markdown exporter updates for richer presentation rendering (the existing `buildPresentationItems` already emits `PresentationItem` that exporters can render).

## Context

The resume editor already supports a `'presentations'` section type (`resume_sections.entry_type` includes it, and one such section exists on the current DB). The IR compiler has a `buildPresentationItems` function that chains through perspectives → bullets → sources. But there's no `'presentation'` source type — the enum is `role | project | education | general`. So the picker falls through to the generic perspective picker, showing unrelated bullets instead of presentation sources.

Phase 84 (migration 037) removed `clearance` from the source_type enum (clearances moved to the `credentials` entity). The current valid source types are `role | project | education | general`. This spec adds `presentation` as the fifth.

## Schema

### Migration (one file, number chosen at implementation time)

**1. Rebuild `sources` table to add `'presentation'` to the CHECK constraint.**

SQLite doesn't support `ALTER TABLE ... ALTER CONSTRAINT`, so this requires the standard table-rebuild pattern:
1. `CREATE TABLE sources_new (...)` with the updated CHECK
2. `INSERT INTO sources_new SELECT * FROM sources`
3. `DROP TABLE sources`
4. `ALTER TABLE sources_new RENAME TO sources`
5. Re-create indexes

The `PRAGMA foreign_keys = OFF` comment at the top signals the migration runner to disable FK checks during the rebuild (the existing `migrate.ts` runner auto-detects this).

**New CHECK constraint:**
```sql
CHECK (source_type IN ('role', 'project', 'education', 'general', 'presentation'))
```

**2. Create `source_presentations` extension table:**

```sql
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

One-to-one with `sources`, same pattern as `source_roles`, `source_projects`, `source_education`. The `sources` base row carries `title` (presentation name), `description` (abstract), `start_date` (when given), `end_date` (nullable), and `status`.

### Field mapping

| Resume display | DB field | Table |
|---|---|---|
| Presentation title | `title` | `sources` |
| Abstract / description | `description` | `sources` |
| Date | `start_date` | `sources` |
| Venue | `venue` | `source_presentations` |
| Type (conference, workshop...) | `presentation_type` | `source_presentations` |
| Slides / recording link | `url` | `source_presentations` |
| Co-authors | `coauthors` | `source_presentations` |

## Core types

### New types

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

### Modified types

- **`SourceType`** union: add `| 'presentation'`
- **`CreateSource`**: add optional fields `venue?: string`, `presentation_type?: PresentationType`, `url?: string` (as URL field for presentations), `coauthors?: string`
- **`UpdateSource`**: same optional fields
- **`SourceWithExtension.extension`**: union gains `| SourcePresentation`
- **`PresentationItem`** (IR type): add `description: string | null`, `venue: string | null`, `presentation_type: string | null`, `url: string | null`, `coauthors: string | null` alongside the existing `title`, `date`, `bullets`

All mirrored in `packages/sdk/src/types.ts`.

## Repository

`packages/core/src/db/repositories/source-repository.ts`:

### `getExtension()`

Add to the switch:

```ts
case 'presentation':
  return db.query('SELECT * FROM source_presentations WHERE source_id = ?')
    .get(sourceId) as SourcePresentation | null
```

### `updateExtension()`

Add a `presentation` block following the same set-builder pattern as `role`/`project`/`education`:

```ts
if (sourceType === 'presentation') {
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

### `create()` transaction

Add INSERT for presentation extension:

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

## API routes

`packages/core/src/routes/sources.ts`:

The `mapExtension` helper maps SDK-style nested extension objects. Add the `'presentation'` case:

```ts
case 'presentation':
  return { ...base, presentation: extension }
```

No new routes needed — the existing `POST/GET/PATCH/DELETE /sources` endpoints handle all source types polymorphically through the service layer.

## SDK

`packages/sdk/src/types.ts`: mirror all core type changes.

`packages/sdk/src/resources/sources.ts`: no changes needed — generic CRUD already works.

## WebUI

### New page: `/experience/presentations`

`packages/webui/src/routes/experience/presentations/+page.svelte`

Follows the exact SplitPanel + ListPanelHeader + editor form pattern used by `/experience/roles`, `/experience/projects`, and `/experience/education`:

- **Left panel:** list of presentation sources with title + venue preview. ListPanelHeader with "+ New" button. ListSearchInput for filtering.
- **Right panel:** editor form when a presentation is selected, EmptyPanel when none selected.
- **Editor fields:** Title, Description/Abstract (textarea), Venue, Type (select), Date, URL, Co-authors, Status + action buttons.
- **Shared components:** `PageWrapper`, `SplitPanel`, `ListPanelHeader`, `EmptyPanel`, `ListSearchInput` per `.claude/rules/ui-shared-components.md`.

### Navigation

`packages/webui/src/lib/nav.ts`: add `{ label: 'Presentations', href: '/experience/presentations' }` to the Experience nav group, between Projects and Education.

### Resume picker routing

`packages/webui/src/routes/resumes/+page.svelte` `openPicker()`: change the 'presentations' case from falling through to the perspective picker to using SourcePicker:

```ts
case 'presentations':
  sourcePickerState = { sectionId, sourceType: 'presentation' }
  return
```

### DragNDropView presentations template

`packages/webui/src/lib/components/resume/DragNDropView.svelte`: update the `{:else if section.type === 'presentations'}` template block to render venue + type below the title (same pattern as education's institution + degree type), and show description when present.

## Compiler

`packages/core/src/services/resume-compiler.ts` `buildPresentationItems()`:

Apply the same LEFT JOIN + COALESCE refactor done for education/cert/clearance/project:

```sql
FROM resume_entries re
LEFT JOIN perspectives p ON p.id = re.perspective_id
LEFT JOIN bullets b ON b.id = p.bullet_id
LEFT JOIN bullet_sources bs ON bs.bullet_id = p.bullet_id AND bs.is_primary = 1
LEFT JOIN sources s ON s.id = COALESCE(bs.source_id, re.source_id)
LEFT JOIN source_presentations sp ON sp.source_id = s.id
```

Add `s.description AS source_description`, `sp.venue`, `sp.presentation_type`, `sp.url`, `sp.coauthors` to the SELECT.

The `PresentationItem` return type gains `description`, `venue`, `presentation_type`, `url`, `coauthors` fields.

## Testing

### Repository (bun:test + in-memory DB)

- Create a presentation source with all extension fields, verify round-trip
- Update presentation extension fields individually
- `getExtension()` returns `SourcePresentation` for presentation type
- `list()` with `source_type: 'presentation'` filter works

### Compiler (resume-compiler.test.ts)

- `buildPresentationItems` renders a direct-source presentation entry (no perspective chain) with venue + type data from `source_presentations`
- Perspective-chain presentation entries still work (backward compatibility)

### Structural (webui bun:test + readFileSync)

- `/experience/presentations/+page.svelte` exists and imports SplitPanel, ListPanelHeader, EmptyPanel
- `nav.ts` includes Presentations link
- `openPicker` has `case 'presentations'` routing to SourcePicker with `sourceType: 'presentation'`
- DragNDropView presentations template renders venue

## Files affected

### Create

- `packages/core/src/db/migrations/<N>_presentations.sql` (N at implementation time)
- `packages/webui/src/routes/experience/presentations/+page.svelte`

### Modify

- `packages/core/src/types/index.ts` — SourceType, SourcePresentation, PresentationType, CreateSource, UpdateSource, PresentationItem
- `packages/sdk/src/types.ts` — mirror
- `packages/core/src/db/repositories/source-repository.ts` — getExtension, updateExtension, create
- `packages/core/src/routes/sources.ts` — mapExtension
- `packages/core/src/services/resume-compiler.ts` — buildPresentationItems LEFT JOIN + new fields
- `packages/webui/src/lib/nav.ts` — add Presentations to Experience group
- `packages/webui/src/routes/resumes/+page.svelte` — openPicker case
- `packages/webui/src/lib/components/resume/DragNDropView.svelte` — presentations template
- `packages/core/src/db/__tests__/migrate.test.ts` — migration count
- `packages/core/src/db/repositories/__tests__/source-repository.test.ts` — presentation CRUD tests
- `packages/core/src/services/__tests__/resume-compiler.test.ts` — presentation compiler test

## Risks

1. **Migration table rebuild for sources.** The CHECK constraint change requires a full table rebuild with FK checks disabled. This is the same pattern used in migration 002 and works reliably, but if another agent's in-flight migration also rebuilds `sources`, they'll collide. Check `main` before committing.
2. **`url` field name collision.** `source_presentations.url` is fine on the extension table, but `CreateSource.url` and `UpdateSource.url` may collide if the base `sources` table ever gains a `url` column. Currently it doesn't — the field is unambiguous. If a collision arises, rename to `presentation_url`.
3. **Existing presentation-type resume sections.** One section of type `'presentations'` exists on the current DB. It likely has entries added via the old perspective picker. Those entries will keep working (they have perspective_id set, so the LEFT JOIN still chains through perspectives). New entries will use the direct source_id path.

## Out-of-scope follow-ups

- Richer PresentationPicker with venue/date columns (v2 polish)
- `audience_size` field
- `organization_id` FK for the hosting org
- LaTeX template rendering venue/type in the presentation section (currently just renders title + date + bullets)
- Bulk import of presentations from a CV or LinkedIn export
