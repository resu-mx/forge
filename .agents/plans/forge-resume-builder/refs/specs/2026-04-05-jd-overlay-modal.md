# JD Overlay Modal — Design Spec

**Date:** 2026-04-05
**Ticket:** `job-hunting-x03.5` (T95.5: JD Overlay Modal from Resume, P1)
**Parent epic:** Phase 95 — Resume Builder Polish
**Status:** Approved, ready for implementation plan

## Summary

A reusable, read-only JD detail overlay that any part of the app can open with a single function call. First concrete consumer is `ResumeLinkedJDs` (the "Targeted Job Descriptions" section on the resume page, per T95.5). The design is generalized so future consumers — kanban cards, Gantt bars, search results, summary entries, notes — can drop in with zero prop plumbing.

This spec also **establishes a project-wide convention** for entity-overlay modals. Future entity overlays (Org, Skill, Bullet, Summary, etc.) should follow the same three-piece hybrid pattern when they need "drop in anywhere" behavior. Existing modals (`OrgDetailModal`, `BulletDetailModal`, `ChainViewModal`) stay as-is — they migrate opportunistically when touched for other reasons.

## Goals

1. Ship the JD overlay modal as spec'd in T95.5 (read-only detail view, title/org/status/raw_text/skills).
2. Introduce a reusable three-piece API (primitive + host + store) that other consumer sites can adopt with minimal code.
3. Document the pattern as the convention for future entity overlays.
4. Wire up T95.5's consumer site (`ResumeLinkedJDs`) to use the new modal.

## Non-goals

- Full CRUD on JDs from the modal — stays on the JD page.
- Markdown rendering of `raw_text` — plain text with `white-space: pre-wrap` for v1; markdown is future work.
- Copy-to-clipboard, status change, or any other interactive action beyond "close" and "open full page".
- Deep-linking modal open state to URL query params.
- Refactoring existing modals (`OrgDetailModal`, `BulletDetailModal`, `ChainViewModal`) to the new pattern.
- Skill chip click-to-filter behavior.

## Architecture

Three-piece hybrid API under `packages/webui/src/lib/components/overlays/`:

```
overlays/
├── JDOverlayModal.svelte      ← prop-driven primitive
├── JDOverlayHost.svelte       ← mounts the singleton once in root layout
├── jdOverlay.svelte.ts        ← store: open() / close() / state
├── index.ts                   ← barrel export
├── README.md                  ← pattern convention documentation
└── __tests__/
    ├── jdOverlay.test.ts
    └── JDOverlayModal.test.ts
```

### Component contracts

#### `JDOverlayModal.svelte` (primitive)

Prop-driven Svelte component. Can be used directly by any consumer that wants local-state control.

```ts
interface JDOverlayModalProps {
  open: boolean
  jdId: string | null
  initialData?: Partial<JobDescriptionWithOrg> & { skills?: Skill[] }
  onClose: () => void
  size?: 'lg' | 'xl'  // default: 'lg'
}
```

Behavior:
- Wraps the existing `$lib/components/Modal.svelte` primitive (reuses focus trap, ESC, backdrop click, a11y).
- On `open` transitioning `false → true`:
  - If `initialData` provided, render immediately using those fields (state: `placeholder`).
  - Fire parallel fetches: `forge.jobDescriptions.get(jdId)` and `forge.jobDescriptions.listSkills(jdId)`.
  - On success, transition to state `loaded` and render canonical data.
  - On failure, transition to state `error` and render an error banner with a retry button. Header area still shows `initialData` if available.
- On `open` transitioning `true → false`: base Modal unmounts, no extra cleanup needed.
- `jdId` changing while open (e.g., user clicks a different JD while modal is already open): treat as a fresh fetch cycle (reset state, re-fetch).

#### `JDOverlayHost.svelte` (singleton mount)

Zero-prop component mounted **exactly once** in `packages/webui/src/routes/+layout.svelte`. Subscribes to the `jdOverlay` store and renders a single `<JDOverlayModal>` driven by store state.

```svelte
<script>
  import { jdOverlay } from './jdOverlay.svelte'
  import JDOverlayModal from './JDOverlayModal.svelte'
</script>

<JDOverlayModal
  open={jdOverlay.state.jdId !== null}
  jdId={jdOverlay.state.jdId}
  initialData={jdOverlay.state.initialData}
  onClose={jdOverlay.close}
/>
```

#### `jdOverlay.svelte.ts` (store)

Svelte 5 runes-based state module. Single global instance.

```ts
interface JDOverlayState {
  jdId: string | null
  initialData?: Partial<JobDescriptionWithOrg> & { skills?: Skill[] }
}

interface JDOverlayStore {
  readonly state: JDOverlayState
  open(jdId: string, initialData?: JDOverlayState['initialData']): void
  close(): void
}

export const jdOverlay: JDOverlayStore
```

Behavior:
- `open(jdId, initialData?)`: set `state.jdId = jdId`, `state.initialData = initialData`. If already open with a different `jdId`, replace it (do not stack — there's only one modal instance).
- `close()`: set `state.jdId = null`, clear `initialData`.
- State is read-only from outside the store (consumers mutate only via `open`/`close`).

### Existing primitive reused

- `packages/webui/src/lib/components/Modal.svelte` — generic modal chrome (overlay, focus trap, ESC, backdrop click, a11y). Provides sizes `sm`/`md`/`lg`/`xl`/`full`, `header`/`body`/`footer` snippets. **No changes required.**

## Data flow & state machine

The modal has three internal states: `fetching`, `loaded`, `error`. The `fetching` state has two rendering variants depending on whether `initialData` was provided — either a **placeholder render** (initialData fields visible, skeleton rows for the rest) or a **spinner render** (no data yet, body shows a centered spinner). Both variants transition to the same `loaded`/`error` states when fetches resolve.

```
                    ┌────────────────────┐
                    │  closed (jdId=null)│
                    └──────────┬─────────┘
                               │ jdOverlay.open(jdId, initialData?)
                               ▼
                    ┌─────────────────────────────┐
           ┌────────┤          fetching           │
           │        │  - placeholder render if    │
           │        │    initialData provided     │
           │        │  - spinner render otherwise │
           │        │  Promise.all in flight      │
           │        └──────────┬──────────────────┘
           │                   │
           │                   │ Promise.all resolves
           │                   ▼
           │        ┌────────────────────┐
           │        │       loaded       │
           │        │  render canonical  │
           │        └──────────┬─────────┘
           │                   │
           └─┐                 │
             │ fetch fails     │
             ▼                 │
    ┌────────────────────┐     │
    │        error       │     │
    │  banner + retry    │     │
    │  (keep initialData │     │
    │   in header)       │     │
    └──────┬─────────────┘     │
           │ retry               │
           └────────────────────┤
                               ▼
                    (back to loaded on success)
                               │
                               │ jdOverlay.close() or ESC or backdrop or [×] or [Open full page →]
                               ▼
                    ┌────────────────────┐
                    │       closed       │
                    └────────────────────┘
```

Fetches fire in parallel:
```ts
const [jdResult, skillsResult] = await Promise.all([
  forge.jobDescriptions.get(jdId),
  forge.jobDescriptions.listSkills(jdId),
])
```

Partial failure handling: if the JD fetch fails, show the error state. If only the skills fetch fails, show the JD with a "Failed to load skills" inline note in the skills row (non-blocking).

## Layout

Single-column stacked. Modal size `lg` (~800px wide) by default.

```
┌──────────────────────────────────────────────┐
│ [status]  Job Title                      [×] │  ← header (base Modal header snippet)
│ Organization Name                            │
├──────────────────────────────────────────────┤
│  📍 Location  ·  💰 $120k–$160k  ·  🔗 url    │  ← metadata strip (hidden if all empty)
│                                              │
│  Skills                                      │  ← hidden if skills list empty
│  [React] [TypeScript] [Node] [AWS] [+3 more] │
│                                              │
│  Description                                 │
│  ┌────────────────────────────────────────┐  │
│  │ (scrollable, max-height ~50vh)         │  │  ← raw_text in plain pre-wrap
│  │ ...                                    │  │
│  └────────────────────────────────────────┘  │
│                                              │
│  Notes                                       │  ← hidden if notes empty
│  ...                                         │
├──────────────────────────────────────────────┤
│  Created Apr 3  ·  Updated Apr 5             │  ← footer (base Modal footer snippet)
│               [Open full page →]   [Close]   │
└──────────────────────────────────────────────┘
```

### Rendering rules

- **Status badge:** reuse status color tokens from `packages/webui/src/lib/components/charts/gantt-utils.ts` (`STATUS_COLORS`). Single source of truth for JD status colors in the app.
- **URL:** clickable `<a>` with `target="_blank" rel="noopener noreferrer"`. Hidden if null.
- **Salary:** if `salary_range` is set, show as-is. Otherwise if `salary_min`/`salary_max` set, format as `$Xk–$Yk`. Otherwise hide.
- **Location:** plain text. Hide if null.
- **Metadata strip:** the whole row hides if all three (location, salary, url) are absent.
- **Skills row:** hide entirely if no skills. Show up to 6 inline; if more, show `[+N more]` chip (non-interactive in v1). Skills are **not clickable** in v1.
- **Description (raw_text):** rendered as `<div style="white-space: pre-wrap">{raw_text}</div>`. No sanitization needed — plain text, not HTML. Max-height ~50vh, internal scroll.
- **Notes section:** hide entirely if `notes` is null/empty.
- **Footer timestamps:** format as `Created MMM D · Updated MMM D` using a consistent date helper (check if one exists in `$lib` first).
- **Empty states**: if canonical fetch returns an unexpectedly-empty JD (all main fields null), render a minimal "No details available" message in the body.

### Footer actions

| Button | Behavior |
|---|---|
| `[Open full page →]` | Calls `goto('/opportunities/job-descriptions?selected=' + jdId)` from `$app/navigation`, **then** calls `onClose()` |
| `[Close]` | Calls `onClose()` |

Plus: header `[×]` button, `Escape` key, backdrop click — all handled by base `Modal`.

## Consumer wiring (T95.5)

Two specific changes for the T95.5 acceptance criteria:

### 1. Mount the host in root layout

`packages/webui/src/routes/+layout.svelte`:

```svelte
<script>
  import { JDOverlayHost } from '$lib/components/overlays'
  // ...existing imports
</script>

<!-- ...existing layout markup... -->
<JDOverlayHost />
```

Verify that `+layout.svelte` does not already have conflicting modal machinery before mounting.

### 2. Make linked JD titles clickable

`packages/webui/src/lib/components/resume/ResumeLinkedJDs.svelte` — change the `.linked-card-name` `<span>` into a `<button>`:

```svelte
<script>
  import { jdOverlay } from '$lib/components/overlays'
  // ...existing imports
</script>

<!-- in the linked-card template -->
<button
  type="button"
  class="linked-card-name-btn"
  onclick={() => jdOverlay.open(jd.job_description_id, {
    title: jd.title,
    status: jd.status,
    organization_name: jd.organization_name,
    location: jd.location,
    salary_range: jd.salary_range,
  })}
>
  {jd.title}
</button>
```

The passed `initialData` is the subset of fields already available on `JDLink`. Modal paints instantly with those, then fills in `raw_text`, `url`, `notes`, and `skills` once the parallel fetch resolves (~100ms typical).

CSS: `.linked-card-name-btn` should be an unstyled button that visually matches the current `.linked-card-name` span (same font-weight, size, color) plus a subtle hover state (underline or color shift) to afford clickability.

## Pattern convention (documented in `overlays/README.md`)

The README in `overlays/` documents this as the convention for future entity-overlay modals. Template for a new entity `X`:

```
overlays/
├── XOverlayModal.svelte   ← prop-driven primitive
├── XOverlayHost.svelte    ← singleton host
├── xOverlay.svelte.ts     ← store
```

Plus export from `overlays/index.ts`, mount host in `+layout.svelte`, and wire consumers via `xOverlay.open(id, initialData?)`.

README sections:
- **When to use this pattern** (entity detail overlays with multiple potential consumer sites)
- **When NOT to use this pattern** (selection dialogs like `JDPickerModal`, one-off context-specific modals)
- **The three pieces** (primitive / host / store) with code skeletons
- **Migration policy** (existing modals migrate opportunistically, not as a batch refactor)
- **Out-of-scope behaviors** (markdown, copy, status change, deep-linking)

## Testing

### Unit tests (Vitest)

**`jdOverlay.test.ts`:**
- `open(id)` sets `state.jdId` and leaves `initialData` undefined
- `open(id, initialData)` sets both fields
- `open(id2)` while already open replaces `jdId` and `initialData` (does not stack)
- `close()` resets `jdId` to `null` and clears `initialData`
- Store is a single instance: re-importing from two call sites returns the same object

**`JDOverlayModal.test.ts`:**
- `fetching` state with `initialData` renders placeholder fields (title, org, status) immediately and shows skeleton rows for description/skills
- `fetching` state without `initialData` renders a spinner in the body
- Transitions to `loaded` after fetch resolves, rendering canonical data
- Handles fetch error: shows error banner, retry button re-fires fetch and transitions back to `loaded` on success
- Hides skills row entirely when skills array is empty
- Hides notes section when `notes` is null/empty
- Hides metadata strip when location, salary, and url are all absent
- `[Open full page →]` calls `goto()` with the expected URL and fires `onClose`
- `[Close]` fires `onClose`
- Skills partial failure: JD renders normally with a "Failed to load skills" inline note in the skills row
- `jdId` prop changing while `open` is true triggers a fresh fetch cycle

### Integration coverage

- `ResumeLinkedJDs` rendering a clickable title that fires `jdOverlay.open(...)` with the correct `initialData` subset. Covered via Svelte component test.

### Out of scope for this phase

- Playwright E2E coverage — existing E2E suite already covers modal-opening patterns generically; adding a JD-specific E2E adds cost without commensurate value for a read-only view.

## Files affected

### Create
- `packages/webui/src/lib/components/overlays/JDOverlayModal.svelte`
- `packages/webui/src/lib/components/overlays/JDOverlayHost.svelte`
- `packages/webui/src/lib/components/overlays/jdOverlay.svelte.ts`
- `packages/webui/src/lib/components/overlays/index.ts` (barrel: re-export all three + types)
- `packages/webui/src/lib/components/overlays/README.md` (pattern convention)
- `packages/webui/src/lib/components/overlays/__tests__/jdOverlay.test.ts`
- `packages/webui/src/lib/components/overlays/__tests__/JDOverlayModal.test.ts`

### Modify
- `packages/webui/src/routes/+layout.svelte` — mount `<JDOverlayHost />` once (verify no conflict first)
- `packages/webui/src/lib/components/resume/ResumeLinkedJDs.svelte` — wire clickable title → `jdOverlay.open(...)`

### Optional (not required for T95.5 acceptance)
- `packages/webui/src/lib/components/index.ts` — decision during implementation: either re-export the overlays barrel from the main components barrel, or keep the overlays barrel isolated. Lean toward keeping isolated so the pattern is discoverable as its own module.

## Risks & verification points for implementation

1. **Root layout modal machinery** — verify `+layout.svelte` does not already have layout-level modal orchestration that would conflict with the host. Read before modifying.
2. **Svelte 5 `.svelte.ts` store pattern** — sanity-check against an existing store in the project (e.g., `lib/stores/toast.svelte.ts`) to match idioms.
3. **Skill type shape** — `forge.jobDescriptions.listSkills()` returns `Skill[]` per the SDK; double-check the type signature and whether it's `Skill` or `SkillWithDomains` after any in-flight Phase 89 changes.
4. **`JDLink` field availability** — confirm the fields passed as `initialData` in `ResumeLinkedJDs` actually exist on the `JDLink` type the SDK returns from `listJobDescriptions(resumeId)`.
5. **Date formatting helper** — check for an existing helper under `$lib` before writing a new one. Use it if found.
6. **Status color import path** — importing from `gantt-utils.ts` couples overlays to charts. If that coupling feels wrong, extract `STATUS_COLORS` to a neutral `$lib/constants/jd-status-colors.ts` as part of this PR (small, mechanical). Decide during implementation.

## Out-of-scope follow-ups (tracked separately after merge)

- Markdown rendering of `raw_text` (new ticket)
- Org overlay modal using the same pattern (new ticket, tied to `OrgDetailModal` opportunistic migration)
- Skill overlay modal (future, tied to Phase 89+ skill taxonomy work)
- Deep-linking modal state to URL query params (future UX polish)
