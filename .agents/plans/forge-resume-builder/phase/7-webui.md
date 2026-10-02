# Phase 7: WebUI

**Goal:** Implement the Svelte 5 SPA with all MVP views.

**Non-Goals:** No pixel-perfect design. Functional first, polish later.

**Depends on:** Phase 5 (SDK)
**Can parallelize with:** Phase 6 (CLI) — both are SDK consumers

**Reference:** `refs/uiux/mockups/webui-views.md`

---

## Task 7.1: Svelte 5 + Vite Scaffold

**Steps:**
1. Initialize Svelte 5 project in `packages/webui/`. Note: Phase 0 already created the directory structure — initialize Svelte INTO the existing directory, merging with the pre-existing `package.json` (update name to `@forge/webui`, add workspace dep on `@forge/sdk`).
2. Configure Vite proxy: `/api/*` → `http://localhost:3000/api/*`
3. Install dependencies: `@forge/sdk` (workspace), SvelteKit in SPA mode (`adapter-static` with `fallback: 'index.html'`). SvelteKit provides file-based routing, SSR-off SPA mode, and a mature ecosystem.
4. Set up folder structure:
   ```
   src/
   ├── lib/
   │   ├── sdk.ts          # ForgeClient singleton
   │   ├── stores/         # Svelte stores wrapping SDK calls
   │   └── components/     # Reusable components
   ├── routes/             # Page components
   │   ├── +page.svelte    # Review queue (dashboard)
   │   ├── sources/
   │   ├── derivation/
   │   ├── chain/
   │   └── resumes/
   └── app.html
   ```
5. Create SDK singleton: `const forge = new ForgeClient({ baseUrl: '/api' })` (proxied in dev, direct in prod)
6. Create base layout: navigation sidebar + content area

**Acceptance Criteria:**
- [ ] `bun run --filter '@forge/webui' dev` starts Vite dev server on 5173
- [ ] API proxy forwards `/api/*` to core server
- [ ] Navigation between views works
- [ ] SDK client initialized and importable from any component

**Testing:**
- Smoke: Dev server starts, index page loads
- Smoke: API proxy works (fetch /api/health from browser)

---

## Task 7.2: Review Queue Dashboard (View 5)

**Goal:** Landing page showing pending review counts and recent activity.

**Components:**
- `PendingCard` — shows count for bullets/perspectives, clickable
- `RecentActivity` — last 10 approved/rejected items
- `QuickStats` — total sources, bullets, perspectives

**Implementation:**
- Call `forge.review.pending()` on mount
- Call `forge.sources.list()`, `forge.bullets.list()`, `forge.perspectives.list()` for stats
- Cards link to derivation view filtered by pending status

**Acceptance Criteria:**
- [ ] Shows correct pending counts
- [ ] Cards navigate to relevant views
- [ ] Loading skeleton while data fetches
- [ ] Empty state when nothing is pending

**Testing:**
- Component: Renders correctly with mock SDK data
- Component: Empty state renders when no pending items
- Visual: Verify layout matches mockup description

---

## Task 7.3: Sources List/Editor (View 1)

**Goal:** Two-panel view for managing sources.

**Components:**
- `SourceList` — filterable list with search, status tabs
- `SourceEditor` — form for creating/editing sources
- `SourceCard` — list item showing title, bullet count, status

**Implementation:**
- Left panel: list with filter bar (employer dropdown, status tabs)
- Right panel: editor opens on card click or "New Source" button
- "Derive Bullets" button triggers derivation with spinner
- Save/cancel/delete buttons

**State management:**
- Svelte 5 `$state` rune for selected source
- `$derived` for filtered list
- Loading state during API calls

**Acceptance Criteria:**
- [ ] Create, edit, delete sources through the UI
- [ ] Filter by status (all/draft/approved)
- [ ] Derive Bullets button disabled if not approved or if deriving
- [ ] Spinner shown during derivation (up to 60s)
- [ ] Error toast on failure
- [ ] Bullet count updates after derivation

**Testing:**
- Component: Source list renders with mock data
- Component: Source editor creates/updates correctly
- Component: Derive button state management
- E2E: Create source → approve → derive bullets → verify bullets appear

---

## Task 7.4: Derivation View (View 2)

**Goal:** Three-column view for the Source → Bullet → Perspective workflow.

**Components:**
- `DerivationLayout` — three-column container
- `SourcePanel` — read-only source display
- `BulletPanel` — list of bullets with approve/reject/derive controls
- `PerspectivePanel` — list of perspectives with approve/reject controls
- `RejectModal` — dialog for entering rejection reason
- `DeriveModal` — dialog for selecting archetype/domain/framing

**Implementation:**
- Column 1: source content (read-only)
- Column 2: bullets for this source, each with status badge, approve/reject buttons
- Column 3: perspectives for selected bullet
- Approve/reject are optimistic updates (revert on error)
- Reject opens modal for reason input
- "Derive Perspectives" opens modal with archetype/domain/framing selectors
- Snapshot match indicator on each bullet/perspective

**Acceptance Criteria:**
- [ ] Three-column layout with source → bullets → perspectives flow
- [ ] Approve/reject work inline with optimistic updates
- [ ] Reject modal requires non-empty reason
- [ ] Derive Perspectives modal has archetype/domain/framing selectors
- [ ] Spinner during derivation
- [ ] Snapshot match indicator (green = match, yellow = diverged)
- [ ] Reopen button on rejected items

**Testing:**
- Component: Bullet card with approve/reject buttons
- Component: Reject modal captures reason
- Component: Derive modal validates required fields
- E2E: Full derivation workflow through UI

---

## Task 7.5: Chain View (View 3)

**Goal:** Tree visualization of provenance chain.

**Components:**
- `ChainTree` — tree layout (source at top, bullets branching, perspectives as leaves)
- `ChainNode` — expandable node showing content, status, snapshot comparison
- `SnapshotDiff` — side-by-side comparison when snapshot diverges from current

**Implementation:**
- Fetch perspective with full chain via `forge.perspectives.get(id)`
- Or: fetch all perspectives for a source and build the tree client-side
- Nodes are expandable/collapsible
- Click node to see full content
- Border color: green (snapshots match), yellow (diverged)
- "View Diff" link shows snapshot vs current side-by-side

**Acceptance Criteria:**
- [ ] Tree renders Source → Bullet → Perspective hierarchy
- [ ] Nodes expand/collapse on click
- [ ] Snapshot match shown with color coding
- [ ] Diff view shows what changed when snapshot diverges
- [ ] Click node to edit (navigates to source editor or derivation view)

**Testing:**
- Component: Tree renders with mock chain data
- Component: Diff view shows differences
- Visual: Tree layout is readable with 5+ bullets and 10+ perspectives

---

## Task 7.6: Resume Builder (View 4)

**Goal:** Drag-and-drop resume assembly with gap analysis.

**Components:**
- `ResumeHeader` — metadata form (name, target_role, employer, archetype)
- `SectionGroup` — collapsible section container (work_history, projects, etc.)
- `PerspectiveCard` — draggable card showing perspective content
- `PerspectivePicker` — modal to add approved perspectives to a section
- `GapAnalysisPanel` — sidebar showing gap analysis results

**Implementation:**
- Left panel: sections with draggable perspective cards
- Drag to reorder within section, drag between sections
- "Add Perspective" button per section opens picker filtered by archetype
- Right panel: gap analysis, auto-refreshes when perspectives change
- "Export" button shows 501 message

**Drag-and-drop:** Use `svelte-dnd-action` — well-maintained, Svelte-native, supports keyboard accessibility. Decision made here to avoid deferring during implementation.

**Acceptance Criteria:**
- [ ] Create resume with metadata
- [ ] Add perspectives to sections via picker
- [ ] Drag to reorder within and between sections
- [ ] Remove perspectives from resume
- [ ] Gap analysis panel updates on changes
- [ ] Export button shows 501 with instructions
- [ ] Only approved perspectives shown in picker

**Testing:**
- Component: Resume metadata form
- Component: Perspective picker filters correctly
- Component: Gap analysis renders with mock data
- E2E: Assemble resume → reorder → check gaps

---

## Task 7.7: Shared Components & Polish

**Goal:** Error handling, loading states, toast notifications.

**Components:**
- `Toast` — notification component for success/error messages
- `LoadingSpinner` — used during derivation
- `EmptyState` — placeholder when no data
- `StatusBadge` — colored badge for entity statuses
- `ConfirmDialog` — for destructive actions (delete)

**Acceptance Criteria:**
- [ ] All API errors surface as toast notifications
- [ ] Destructive actions require confirmation
- [ ] Loading states for all async operations
- [ ] Empty states for all list views

**Testing:**
- Component: Toast renders and auto-dismisses
- Component: Confirm dialog blocks action until confirmed

---

## Parallelization

Views can be developed independently after scaffold (Task 7.1):

```
Task 7.1 (scaffold) ──┬──► Task 7.2 (review queue)
                       ├──► Task 7.3 (sources)
                       ├──► Task 7.4 (derivation)
                       ├──► Task 7.5 (chain)
                       ├──► Task 7.6 (resume builder)
                       └──► Task 7.7 (shared components)
```

Task 7.7 should start first — it provides shared components (`Toast`, `StatusBadge`, `LoadingSpinner`, `ConfirmDialog`) consumed by Tasks 7.2-7.6. Tasks 7.2-7.6 can proceed in parallel, importing shared components as they become available.

**Component testing strategy:** Use Vitest + `@testing-library/svelte` for component tests. Svelte 5 component testing setup: configure Vitest with `@sveltejs/vite-plugin-svelte` and `svelte({ compilerOptions: { runes: true } })`. Install in Task 7.1 scaffold.

**Bullet count update mechanism (Task 7.3):** After derivation completes, re-fetch the source via `forge.sources.get(id)` to update the bullet count. No polling or WebSocket — simple re-fetch on action completion.

## Documentation

- `docs/src/webui/views.md` — view descriptions and navigation
- `docs/src/webui/components.md` — component catalog
