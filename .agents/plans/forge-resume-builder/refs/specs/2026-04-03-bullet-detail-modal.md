# Bullet Detail Modal

**Date:** 2026-04-03
**Spec:** 12 (Bullet Detail Modal)
**Phase:** TBD (next available)
**Builds on:** Bullets API (Phase 4), bullet_skills (migration 001), source_skills pattern

## Overview

Add a detail modal for bullets on `/data/bullets`. Clicking a bullet card opens a modal overlay showing all bullet data — content, status, domain, skills, technologies, sources, notes, and related perspectives. All fields are editable (except sources, which are read-only). The modal includes a "Derive Perspectives" button that opens a separate derivation dialog.

**No new migrations.** The `bullet_skills` and `bullet_technologies` tables already exist. Backend changes: (1) add bullet skill API routes (matching the `source_skills` pattern), (2) extend `UpdateBulletInput` to accept `notes`, `domain`, and `technologies`, and (3) add a bullet sources endpoint.

## 1. API Changes

### 1.1 New Endpoints

Add to `packages/core/src/routes/bullets.ts`, matching the source skills pattern:

```
GET    /api/bullets/:id/skills         — list skills linked to this bullet
POST   /api/bullets/:id/skills         — link existing skill or create+link
DELETE /api/bullets/:bulletId/skills/:skillId — unlink a skill
GET    /api/bullets/:id/sources        — list sources linked to this bullet (with is_primary flag)
```

**POST /api/bullets/:id/skills body:**
- `{ skill_id: string }` — link an existing skill
- `{ name: string, category?: string }` — create a new skill (capitalizeFirst on name, case-insensitive dedup) and link it

**Response:** `{ data: Skill }` (the linked/created skill)

**GET /api/bullets/:id/sources response:**
- Queries `bullet_sources` JOIN `sources` for the given bullet ID
- Returns `{ data: Array<Source & { is_primary: boolean }> }` — each source includes all source fields plus the `is_primary` flag from `bullet_sources`

### 1.2 Backend Changes Required

**`packages/core/src/db/repositories/bullet-repository.ts`:**

Extend `UpdateBulletInput` to include:
```typescript
export interface UpdateBulletInput {
  content?: string
  metrics?: string | null
  notes?: string | null
  domain?: string | null
  technologies?: string[]
}
```

In `update()`:
- Handle `notes` and `domain` as additional SET clauses (same pattern as `content` and `metrics`)
- For `technologies`: within the same transaction, DELETE all rows from `bullet_technologies WHERE bullet_id = ?` then INSERT the new list. Each technology is stored as `tech.toLowerCase().trim()`.

**`packages/core/src/services/bullet-service.ts`:**

No changes to `updateBullet()` logic needed beyond what the repository handles. The service already forwards `input` to `BulletRepository.update()`.

**Gap: `draft` to `pending_review` transition.** There is no dedicated "submit for review" endpoint. The service has `VALID_TRANSITIONS: { draft: ['pending_review'], ... }` and a private `transition()` method, but it is only called by `approveBullet`, `rejectBullet`, and `reopenBullet` — none of which handle the `draft -> pending_review` case. Two options:
1. Add a `submitBullet(id)` method to `BulletService` that calls `this.transition(id, 'pending_review')`, plus a `PATCH /api/bullets/:id/submit` route.
2. Allow `PATCH /api/bullets/:id` to accept `{ status: 'pending_review' }` and validate the transition there.

**Recommended:** Option 1 (explicit endpoint) for consistency with the approve/reject/reopen pattern. The modal's "Submit for Review" button calls `PATCH /api/bullets/:id/submit`. Add this endpoint to `bullets.ts` routes.

### 1.3 Existing Endpoints Used

| Endpoint | Purpose in modal |
|----------|-----------------|
| `GET /api/bullets/:id` | Load bullet data (content, status, domain, notes, metrics, technologies). Returns `Bullet`, not `BulletWithRelations`. Sources and perspective counts are fetched separately. |
| `PATCH /api/bullets/:id` | Update content, notes, domain, technologies, metrics |
| `PATCH /api/bullets/:id/approve` | Status transition → approved |
| `PATCH /api/bullets/:id/reject` | Status transition → rejected (with `{ rejection_reason: string }` body) |
| `PATCH /api/bullets/:id/reopen` | Status transition → pending_review |
| `PATCH /api/bullets/:id/submit` | Status transition draft → pending_review *(new — see 1.2)* |
| `DELETE /api/bullets/:id` | Delete bullet (returns CONFLICT if perspectives exist) |
| `GET /api/perspectives?bullet_id=:id` | List perspectives derived from this bullet |
| `POST /api/bullets/:id/derive-perspectives` | Trigger AI derivation (used by derivation dialog) |

## 2. Components

### 2.1 BulletDetailModal.svelte

**Location:** `packages/webui/src/lib/components/BulletDetailModal.svelte`

**Props:**
```typescript
let { bulletId, onclose, onupdate }: {
  bulletId: string
  onclose: () => void
  onupdate: () => void  // called after any mutation to refresh parent list
} = $props()
```

**State:**
```typescript
let bullet = $state<Bullet | null>(null)
let bulletSkills = $state<Skill[]>([])
let bulletSources = $state<Array<Source & { is_primary: boolean }>>([])
let perspectives = $state<Perspective[]>([])
let loading = $state(true)
let saving = $state(false)

// Editable fields (populated from bullet on load)
let editContent = $state('')
let editNotes = $state('')
let editDomain = $state<string | null>(null)

// Skill picker
let allSkills = $state<Skill[]>([])
let skillSearch = $state('')
let showSkillDropdown = $state(false)

// Technology input
let newTechInput = $state('')
let editTechnologies = $state<string[]>([])

// Derivation dialog
let showDeriveDialog = $state(false)

// Delete confirmation — use existing ConfirmDialog component (imported from $lib/components)
// instead of custom state. Follow the ConfirmDialog props pattern used in other pages.
```

**Data loading (on mount):**
1. `GET /api/bullets/:id` → populate `bullet`, `editContent`, `editNotes`, `editDomain`, `editTechnologies`. Note: this returns `Bullet` (not `BulletWithRelations`). Sources are not included in this response.
2. `GET /api/bullets/:id/skills` → populate `bulletSkills`
3. `GET /api/bullets/:id/sources` → populate `bulletSources` (sources with `is_primary` flag, displayed read-only)
4. `GET /api/perspectives?bullet_id=:id` → populate `perspectives` (perspective count is derived from this list's length)
5. `GET /api/skills` → populate `allSkills` (for the skill picker)

**Layout sections (top to bottom):**

1. **Header row:** Bullet title (truncated content) + Status dropdown + Close button
2. **Content:** Editable textarea (auto-height)
3. **Domain:** Dropdown of available domains (from `/api/domains`). The `Bullet.domain` field stores a domain name string (not an ID). The dropdown displays `domain.name` values and stores the selected `name` string directly. `editDomain` is typed as `string | null`.
4. **Skills:** Tag pills with × remove + search input with dropdown (same pattern as source skills)
5. **Technologies:** Tag pills with × remove + text input (Enter to add, free text — no lookup table). Technologies are stored lowercase in the database (`tech.toLowerCase().trim()`). The UI displays them as-is from the API response. New technologies entered by the user are lowercased before sending.
6. **Sources:** Read-only list from `GET /api/bullets/:id/sources`. Primary source marked with star. Each source shows title. Data comes from step 3 of data loading (not from the bullet object).
7. **Notes:** Editable textarea
8. **Perspectives:** List showing archetype / domain / framing + status badge. Read-only summary for now (no dedicated perspective page exists yet — clicking is a future enhancement).
9. **Footer:** "Derive Perspectives" button (opens dialog) + "Save" button + "Delete" button

**Saving behavior (all API calls should use SDK methods, not raw fetch, for consistency):**
- **Save button:** PATCHes `content`, `notes`, `domain`, and `technologies` in a single call via `sdk.bullets.update(id, { content, notes, domain, technologies })`. Calls `onupdate()` after success.
- **Status changes:** Dedicated buttons trigger the appropriate SDK method (`sdk.bullets.approve(id)`, `sdk.bullets.reject(id, { rejection_reason })`, `sdk.bullets.reopen(id)`, or the new submit method). Rejection shows an inline text input for the reason.
- **Skills:** Individual add/remove calls via SDK (instant, no Save needed).
- **Technologies:** Tracked locally in `editTechnologies`, sent as part of the Save PATCH.

### 2.2 DerivePerspectivesDialog.svelte

**Location:** `packages/webui/src/lib/components/DerivePerspectivesDialog.svelte`

**Props:**
```typescript
let { bulletId, onclose, onderive }: {
  bulletId: string
  onclose: () => void
  onderive: () => void  // called after successful derivation to refresh perspectives list
} = $props()
```

**Layout:**
```
┌─ Derive Perspectives ─────────── × ─┐
│                                       │
│ Archetype  [DevSecOps Engineer ▾]     │
│ Domain     [Security ▾]              │
│ Framing    [Technical ▾]             │
│                                       │
│              [Cancel]  [Derive]       │
└───────────────────────────────────────┘
```

**Behavior:**
- Archetype dropdown loads from `GET /api/archetypes`. Displays `archetype.name` and sends `archetype.name` as the string value to the derive endpoint (the AI prompt uses the name, not the ID).
- Domain dropdown loads from `GET /api/domains`. Displays `domain.name` and sends `domain.name`.
- Framing dropdown: hardcoded options (`accomplishment`, `responsibility`, `context`) — matches the `Framing` type in `types/index.ts`
- The Derive button is disabled while archetype/domain dropdowns are loading or if either has no options. Show a loading spinner in the dialog while the dropdown data loads.
- "Derive" button calls `POST /api/bullets/:id/derive-perspectives` with `{ archetype, domain, framing }`
- Shows loading spinner during AI derivation
- On success: shows toast, calls `onderive()`, closes dialog
- On error: shows error toast, stays open

### 2.3 Status Transitions

The status area in the modal header shows the current status badge and valid action buttons. The actual valid transitions from `BulletService.VALID_TRANSITIONS` are:

- `draft` -> `pending_review`
- `pending_review` -> `approved` or `rejected`
- `approved` -> *(no transitions — final state)*
- `rejected` -> `pending_review`

| Current Status | Available Actions |
|---|---|
| `draft` | Submit for Review |
| `pending_review` | Approve, Reject |
| `approved` | *(none — final state)* |
| `rejected` | Reopen (back to pending_review) |

**Submit for Review:** If the bullet is `draft`, show a "Submit for Review" button. This calls `PATCH /api/bullets/:id/submit` (new endpoint — see Section 1.2).

**Approve:** Calls `PATCH /api/bullets/:id/approve`.

**Reject flow:** Clicking "Reject" shows an inline text input for the rejection reason, then calls `PATCH /api/bullets/:id/reject` with `{ rejection_reason: string }` in the request body.

**Reopen:** Only available for `rejected` bullets. Calls `PATCH /api/bullets/:id/reopen`. Transitions to `pending_review` (NOT `draft`).

## 3. Integration with BulletsView

**Trigger:** Add `onclick` handler to each bullet card in `BulletsView.svelte`. Clicking a card opens `BulletDetailModal` with the bullet's ID.

**State in BulletsView:**
```typescript
let detailBulletId = $state<string | null>(null)
```

**Template addition (after the existing list):**
```svelte
{#if detailBulletId}
  <BulletDetailModal
    bulletId={detailBulletId}
    onclose={() => detailBulletId = null}
    onupdate={() => loadItems()}
  />
{/if}
```

**Card click:** `onclick={() => detailBulletId = item.id}` on each `.item-card` div.

## 4. Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/BulletDetailModal.svelte` | Bullet detail/edit modal |
| `packages/webui/src/lib/components/DerivePerspectivesDialog.svelte` | AI derivation parameter dialog |

## 5. Files to Modify

| File | Change |
|------|--------|
| `packages/core/src/db/repositories/bullet-repository.ts` | Extend `UpdateBulletInput` with `notes`, `domain`, `technologies` fields. Update `update()` to handle new fields (SET clauses for notes/domain; DELETE-all + INSERT-all for technologies within same transaction). |
| `packages/core/src/services/bullet-service.ts` | Add `submitBullet(id)` method that calls `this.transition(id, 'pending_review')` for the draft -> pending_review transition. |
| `packages/core/src/routes/bullets.ts` | Add bullet skills endpoints (GET/POST/DELETE), bullet sources endpoint (GET), and `PATCH /api/bullets/:id/submit` route. |
| `packages/sdk/src/resources/bullets.ts` | Add `listSkills(bulletId)`, `addSkill(bulletId, input)`, `removeSkill(bulletId, skillId)`, `listSources(bulletId)`, and `submit(bulletId)` methods to `BulletsResource`. |
| `packages/webui/src/routes/data/sources/BulletsView.svelte` | Add click handler + modal mount. Remove the existing inline derive modal (archetype/domain/framing dropdowns + derive button on each card). The new flow is: click card -> open BulletDetailModal -> click "Derive Perspectives" -> opens DerivePerspectivesDialog. The card-level "Derive Perspective" button is removed. |

## 6. Testing

### Integration tests
- `GET /api/bullets/:id/skills` returns linked skills
- `POST /api/bullets/:id/skills` with `{ skill_id }` links an existing skill
- `POST /api/bullets/:id/skills` with `{ name }` creates and links a new skill (capitalizeFirst applied)
- `POST /api/bullets/:id/skills` with duplicate skill is idempotent (INSERT OR IGNORE)
- `DELETE /api/bullets/:bulletId/skills/:skillId` unlinks a skill
- `POST /api/bullets/:id/skills` with `{ name: 'python' }` when 'Python' exists links the existing one (case-insensitive dedup)
- `GET /api/bullets/:id/sources` returns linked sources with `is_primary` flag
- `PATCH /api/bullets/:id` with `{ notes: 'new note' }` updates the notes field
- `PATCH /api/bullets/:id` with `{ domain: 'security' }` updates the domain field
- `PATCH /api/bullets/:id` with `{ domain: null }` clears the domain field
- `PATCH /api/bullets/:id` with `{ technologies: ['python', 'rust'] }` replaces all technologies
- `PATCH /api/bullets/:id` with `{ technologies: [] }` clears all technologies
- `PATCH /api/bullets/:id/submit` transitions draft bullet to pending_review
- `PATCH /api/bullets/:id/submit` on non-draft bullet returns VALIDATION_ERROR

### Component smoke tests
- Modal opens when bullet card is clicked
- All fields populate correctly from API data
- Content textarea is editable
- Status dropdown shows valid transitions
- Skills can be added/removed/created
- Technologies can be added/removed via text input
- Sources display as read-only with primary marker
- Perspectives list shows and is clickable
- "Derive Perspectives" opens the derivation dialog
- Delete shows ConfirmDialog (from `$lib/components`), then removes bullet on confirm
- Modal closes on ×, Escape, backdrop click

## 7. Non-Goals

- Editing perspectives from within the bullet modal (click navigates away)
- Inline derivation parameters (separate dialog)
- Bulk bullet operations
- Reordering perspectives within the bullet
- Source editing from the bullet modal (sources are read-only context)

## 8. Acceptance Criteria

1. Clicking a bullet card on `/data/bullets` opens the detail modal with all fields populated
2. Modal shows a loading spinner while initial data fetches are in-flight
3. Content and notes are editable and save via PATCH on "Save" click
4. Status area shows valid transition buttons and calls the correct endpoint: Submit for Review (draft), Approve/Reject (pending_review), Reopen (rejected), none (approved)
5. `draft` bullets show "Submit for Review" which transitions to `pending_review`
6. Rejection shows inline reason input before calling the reject endpoint with `{ rejection_reason: string }`
7. Reopen transitions to `pending_review` (not `draft`)
8. Skills can be added (search + select), created inline (type + Enter), and removed (× click)
9. New skills created inline appear in `/data/skills` with capitalizeFirst formatting
10. Technologies can be added (type + Enter) and removed (× click). New technologies are lowercased before sending.
11. Sources are displayed read-only with primary source marked with a star
12. Domain is selectable from a dropdown of existing domains (stores domain name string, not ID)
13. Perspectives list shows archetype/domain/framing + status badge. Clicking a perspective is a future enhancement (no dedicated perspective page exists yet) — for now the list is read-only summary.
14. "Derive Perspectives" button opens the derivation dialog with archetype/domain/framing dropdowns. Derive button is disabled while dropdowns are loading or if archetype/domain have no options.
15. Successful derivation refreshes the perspectives list in the modal
16. "Delete" shows ConfirmDialog (from `$lib/components`), then deletes bullet and closes modal
17. If the bullet has existing perspectives, the delete API returns a CONFLICT error; the modal shows an error toast with "Cannot delete bullet with existing perspectives. Delete its perspectives first." and remains open
18. Modal closes on × click, Escape key, or backdrop click
19. Parent bullet list refreshes after any mutation (save, status change, delete)
20. The existing inline derive modal on bullet cards in `BulletsView.svelte` is removed in favor of the new BulletDetailModal flow
