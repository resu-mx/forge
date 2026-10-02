# Summary Templates

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Summaries as Standalone Entities (Spec 2, 2026-03-30)

**IMPORTANT: Implement AFTER Spec 2 (Summaries Entity).** This spec is a behavioral extension of the summaries table created in Spec 2. It cannot be started until Spec 2's core implementation (table, CRUD, SDK resource) is complete.

## Purpose

Summary templates allow users to create reusable blueprints for professional summaries. When starting a new resume, the user picks a template, and the system clones it into a fresh summary instance linked to that resume. This avoids repetitive data entry while keeping each resume's summary independently editable.

Templates are not a separate entity type. They use the `is_template` flag on the existing `summaries` table (defined in Spec 2). This spec details the behavioral layer around that flag.

## Goals

1. Template summaries (`is_template = 1`) appear in a dedicated "Templates" section on the Summaries view
2. Any summary can be promoted to or demoted from template status
3. Resume creation flow surfaces templates prominently in the summary picker
4. Cloning a template creates a new summary with `is_template = 0`, linked to the resume
5. Editing a template does NOT affect previously cloned instances (no live binding)

## Non-Goals

- Template categories or tagging
- Template versioning (track changes over time)
- Template inheritance (child templates extending parent)
- Template import/export (future, mentioned but not implemented)
- AI-assisted template generation
- Archetype-to-template auto-mapping
- Template usage analytics (how many resumes cloned from this template)

---

## 1. Schema Changes

**None.** This spec uses the `is_template` column on the `summaries` table defined in Spec 2. No additional tables, columns, or migrations are required.

The relevant column:

```sql
is_template INTEGER NOT NULL DEFAULT 0
```

And the index:

```sql
CREATE INDEX idx_summaries_template ON summaries(is_template);
```

---

## 2. API Endpoints

All endpoints are part of the Summaries API defined in Spec 2. This spec adds behavioral semantics and one additional endpoint.

### 2.1 Existing Endpoints (from Spec 2) with Template Behavior

| Method | Path | Template-Relevant Behavior |
|--------|------|---------------------------|
| `GET` | `/api/summaries?is_template=1` | Filter to only templates |
| `GET` | `/api/summaries?is_template=0` | Filter to only instances |
| `PATCH` | `/api/summaries/:id` | Can set `is_template` to promote/demote |
| `POST` | `/api/summaries/:id/clone` | Clones any summary; result has `is_template = 0` |
| `DELETE` | `/api/summaries/:id` | Deleting a template does NOT affect cloned instances |

### 2.2 New Endpoint: Promote/Demote Template

While `PATCH` can set `is_template`, a dedicated toggle endpoint provides clearer semantics:

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `POST` | `/api/summaries/:id/toggle-template` | Toggle `is_template` flag | - | `Result<Summary>` |

**Behavior:**
- If `is_template = 0`, sets to `1`
- If `is_template = 1`, sets to `0`
- Returns the updated summary

> **Toggle vs PATCH:** The `POST /api/summaries/:id/toggle-template` is a convenience endpoint. The authoritative way to set `is_template` is via `PATCH /api/summaries/:id` with `{ is_template: true/false }`.
>
> **Implementation note:** Use `UPDATE summaries SET is_template = ((is_template + 1) % 2) WHERE id = ?` to perform the flip atomically in a single statement, avoiding any read-modify-write race.

### 2.3 Clone Behavior

Clone behavior is fully defined in Spec 2 (Summaries Entity).

### 2.4 Linked Resumes

| Method | Path | Description | Response |
|--------|------|-------------|----------|
| `GET` | `/api/summaries/:id/linked-resumes` | List resumes linked to this summary | `PaginatedResult<Resume>` |

**Route handler:** `summaries.routes.ts` — register `GET /api/summaries/:id/linked-resumes` delegating to the service method.

**Service method:** `SummaryService.getLinkedResumes(id: string, pagination: PaginationParams): Promise<PaginatedResult<Resume>>` — queries `SELECT * FROM resumes WHERE summary_id = :id` with standard pagination.

**SDK method:** Add to `SummariesResource`:

```typescript
linkedResumes(id: string, params?: PaginationParams): Promise<PaginatedResult<Resume>> {
  return this.requestList<Resume>('GET', `/api/summaries/${id}/linked-resumes`, params)
}
```

**Acceptance criterion:** `GET /api/summaries/:id/linked-resumes` returns a paginated list of resumes whose `summary_id` matches the given summary.

---

## 3. UI Changes

### 3.1 Summaries View — Templates Section

On the `/data/summaries` page, the list is divided into two sections:

```
Templates
  [star icon] Security Engineer — Cloud Focus        [Edit] [Clone] [Demote]
  [star icon] Infrastructure Lead                     [Edit] [Clone] [Demote]

Summaries
  Platform Engineer — AWS                             [Edit] [Clone] [Promote]
  DevSecOps Specialist — DoD                          [Edit] [Clone] [Promote]
  Security Architect — FAANG                          [Edit] [Clone] [Promote]
```

- Templates section appears above regular summaries
- Templates have a visual indicator (star badge, different background, or label)
- "Promote" button on regular summaries → sets `is_template = 1`
- "Demote" button on templates → sets `is_template = 0`
- "Clone" button → creates a copy as a regular summary

### 3.2 Resume Creation Flow — Summary Picker

When creating a new resume, the summary picker step shows:

```
Pick a Summary

Templates                              [Create New]
  [star] Security Engineer — Cloud Focus     [Use]
  [star] Infrastructure Lead                  [Use]

Existing Summaries
  Platform Engineer — AWS                     [Link]
  DevSecOps Specialist — DoD                  [Link]

                                        [Skip]
```

**Behaviors:**

| Action | What happens |
|--------|-------------|
| **[Use]** on a template | Clone the template → link clone to resume |
| **[Link]** on an existing summary | Link that summary directly to resume (shared reference) |
| **[Create New]** | Open inline form → create summary → link to resume |
| **[Skip]** | Create resume with `summary_id = NULL` |

**Warning on [Link]:** If the user links an existing (non-template) summary that is already linked to another resume, show a warning: "This summary is also used by [other resume name]. Changes will affect both resumes. Clone instead?" with [Clone] and [Link Anyway] buttons.

> **Linked-resumes query:** Requires `GET /api/summaries/:id/linked-resumes` (see Section 2.4) and `linked_resume_count` in the summary response. Without these, the 'also used by' warning cannot be displayed.

### 3.3 Summary Detail/Edit View

When editing a template summary, show a banner:

> "This is a template. Changes here will NOT affect resumes that were previously created from this template."

When editing a non-template summary that is linked to multiple resumes, show a banner:

> "This summary is linked to [N] resumes. Changes will be reflected in all of them."

---

## 4. Type Changes

The `Summary` type (defined in Spec 2) gains one computed field:

```typescript
interface Summary {
  // ... existing fields from Spec 2 ...
  linked_resume_count: number  // computed, not stored
}
```

`linked_resume_count` is populated via a subquery in the repository's SELECT:

```sql
(SELECT COUNT(*) FROM resumes WHERE summary_id = summaries.id) AS linked_resume_count
```

This field is included in `GET /api/summaries/:id` and `GET /api/summaries` list responses. It powers the Section 3.3 edit-time banner ("linked to N resumes").

### 4.1 SDK Resource Method Addition

The `SummariesResource` class (defined in Spec 2 as `packages/sdk/src/resources/summaries.ts`) is extended with `toggleTemplate()` and `linkedResumes()`.

> **`ForgeClient` wiring:** `ForgeClient` needs a `public summaries: SummariesResource` property, instantiated as `this.summaries = new SummariesResource(req, reqList)` in the constructor, following the pattern of other resources in `client.ts`.

Add to `SummariesResource` class:

```typescript
toggleTemplate(id: string): Promise<Result<Summary>> {
  return this.request<Summary>('POST', `/api/summaries/${id}/toggle-template`)
}

linkedResumes(id: string, params?: PaginationParams): Promise<PaginatedResult<Resume>> {
  return this.requestList<Resume>('GET', `/api/summaries/${id}/linked-resumes`, params)
}
```

> **`toParams` boolean conversion:** `toParams(boolean)` produces `"true"` not `"1"`, which SQLite will not match against INTEGER columns. When building query params for `is_template`, convert explicitly: `is_template: filter.is_template !== undefined ? (filter.is_template ? '1' : '0') : undefined`.

---

## 5. Acceptance Criteria

1. **Template flag**: summaries with `is_template = 1` are visually distinct in the list view
2. **Promote/Demote**: clicking Promote sets `is_template = 1`; clicking Demote sets it to `0`
3. **Toggle endpoint**: `POST /api/summaries/:id/toggle-template` toggles the flag and returns the updated summary
4. **Clone from template**: using a template in resume creation clones it (new row, `is_template = 0`) and links the clone
5. **No live binding**: editing a template after cloning does NOT change the clone
6. **Link warning**: linking an existing non-template summary shared by another resume shows a warning
7. **Template banner**: editing a template shows an informational banner
8. **Resume creation flow**: summary picker shows Templates section above Existing Summaries
9. **Filter API**: `GET /api/summaries?is_template=1` returns only templates
10. **Delete safety**: deleting a template does not affect cloned summaries (they are independent rows)
11. **Linked resumes endpoint**: `GET /api/summaries/:id/linked-resumes` returns a paginated list of resumes using this summary
12. **[Create New] inline form**: clicking [Create New] in the summary picker opens an inline form; submitting creates a summary and links it to the resume
13. **[Skip] sets null**: clicking [Skip] in the summary picker creates the resume with `summary_id = NULL`
14. **Edit-time multi-resume banner**: editing a non-template summary linked to N > 1 resumes shows "This summary is linked to N resumes. Changes will be reflected in all of them."

---

## 6. Dependencies & Parallelization

### Dependencies

| Dependency | Required For | Blocking? |
|-----------|-------------|-----------|
| Spec 2 (Summaries Entity) | `summaries` table, CRUD endpoints, `SummaryRepository` | Yes — must be implemented first |
| Spec 1 (Nav Restructuring) | `/data/summaries` route in sidebar | Soft — view can be developed independently |

### Parallelization

This spec is an extension of Spec 2 and CANNOT be started until Spec 2's core implementation (table, repository, API) is complete. However, parts can be parallelized:

| Stream | Description | Can run in parallel |
|--------|-------------|-------------------|
| A | Toggle-template API endpoint + repository method | After Spec 2 core |
| B | Summaries view — template section UI | After Spec 2 core; soft dep: Spec 1 (Nav Restructuring) |
| C | Resume creation flow — summary picker | After A + B |
| D | Warning banners and shared-summary detection | After C |

**Recommended approach:** Implement Spec 2 and Spec 3 as a single work unit. The `is_template` column is defined in Spec 2's migration, and the behavioral layer in Spec 3 is thin enough to build in the same phase.

---

## 7. Testing

- Verify toggle template on/off (0 -> 1 -> 0)
- Verify `toggleTemplate` on a non-existent id returns 404
- Verify clone template produces `is_template = 0`
- Verify clone template produces title prefixed with 'Copy of '
- Verify deleting a template does NOT cascade-delete cloned summaries
- Verify filter `?is_template=1` returns only templates
- Verify filter `?is_template=0` returns only instances
- Verify `GET /api/summaries/:id/linked-resumes` returns resumes linked to the summary
- Verify [Link] on a summary linked to >=1 other resumes shows the shared-summary warning
- Verify [Use] on a template clones it and links the clone (not the template) to the resume
- Verify edit-time banner shows correct N for a summary linked to N resumes

---

## 8. Known Limitations

1. **No template lineage tracking** — once cloned, there is no record of which template a summary was cloned from. If lineage tracking is needed later, a `cloned_from_id` column can be added.
2. **No bulk operations** — no way to update all summaries cloned from a template. This is by design (no live binding), but a "push template changes" feature could be added later.
3. **No template categories** — templates are a flat list. If the user accumulates many templates, filtering or categorization will be needed.
4. **Shared summary risk** — the [Link] action creates a shared reference. If the user edits the shared summary, all linked resumes are affected. The warning mitigates this but does not prevent it.
5. **No template export/import** — templates cannot be exported as JSON or shared with other Forge instances. This is noted as a future feature.
6. **Clone naming** — "Copy of [title]" is a simple naming convention. The user must rename manually if they want something more descriptive.
