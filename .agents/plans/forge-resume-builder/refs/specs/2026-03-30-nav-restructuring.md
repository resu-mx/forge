# Navigation Restructuring

**Date:** 2026-03-30
**Status:** Design
**Builds on:** All existing WebUI routes (Phase 7, 14, 20, 28)

## Purpose

Restructure the flat 10-item sidebar navigation in `+layout.svelte` into a grouped, accordion-style sidebar with 4 top-level sections plus a standalone Dashboard link. The current flat list does not scale — users must scan all items to find what they need, and related views (Sources/Skills, Domains/Archetypes) have no visual grouping.

## Goals

1. Group related views under collapsible top-level sections (Data, Opportunities, Resumes, Config)
2. Accordion-style expand/collapse for sub-sections
3. Highlight the active sub-section
4. New route structure under `/data/`, `/opportunities/`, `/config/` prefixes
5. Redirect old routes to their new locations for bookmark compatibility
6. Remove Chain View from navigation (it becomes a modal in Spec 5)

## Non-Goals

- Responsive/mobile sidebar (future)
- Drag-to-reorder nav items
- User-customizable nav sections
- Keyboard shortcut navigation
- Breadcrumb navigation in the content area
- The Chain View modal itself (Spec 5: Chain View Modal)

---

## 1. Schema Changes

**None.** This is a pure UI/routing change.

---

## 2. API Endpoints

**None.** No new API endpoints required.

---

## 3. UI Changes

### 3.1 Sidebar Structure

Replace the flat `navItems` array in `+layout.svelte` with a grouped navigation model:

```
Dashboard                        /
Data
  |- Sources                     /data/sources        (tabs: Bullets | Skills)
  |- Summaries                   /data/summaries      (tabs: Description | Title | Role | Contact Info)
  |- Domains                     /data/domains        (tabs: Domains | Archetypes)
  |- Notes                       /data/notes
Opportunities
  |- Organizations               /opportunities/organizations
  |- Job Descriptions            /opportunities/job-descriptions
Resumes                          /resumes
Config
  |- Profile                     /config/profile
  |- Templates                   /config/templates
  |- Export                       /config/export
  |- Debug (Logs)                /config/debug
```

### 3.2 Navigation Data Model

Replace the flat `navItems` array with a typed structure:

```typescript
interface NavItem {
  href: string
  label: string
}

interface NavGroup {
  label: string
  prefix: string           // route prefix for active-state matching
  children: NavItem[]
}

type NavEntry = NavItem | NavGroup

// Runtime check: use `'children' in entry` to distinguish NavGroup from NavItem.
// TypeScript narrows correctly with this property check.

const navigation: NavEntry[] = [
  { href: '/', label: 'Dashboard' },
  {
    label: 'Data',
    prefix: '/data',
    children: [
      { href: '/data/sources', label: 'Sources' },
      { href: '/data/summaries', label: 'Summaries' },
      { href: '/data/domains', label: 'Domains' },
      { href: '/data/notes', label: 'Notes' },
    ],
  },
  {
    label: 'Opportunities',
    prefix: '/opportunities',
    children: [
      { href: '/opportunities/organizations', label: 'Organizations' },
      { href: '/opportunities/job-descriptions', label: 'Job Descriptions' },
    ],
  },
  { href: '/resumes', label: 'Resumes' },
  {
    label: 'Config',
    prefix: '/config',
    children: [
      { href: '/config/profile', label: 'Profile' },
      { href: '/config/templates', label: 'Templates' },
      { href: '/config/export', label: 'Export' },
      { href: '/config/debug', label: 'Debug (Logs)' },
    ],
  },
]
```

### 3.3 Accordion Behavior

- Top-level groups are always visible in the sidebar
- Clicking a group label toggles its expanded state (shows/hides children)
- A group auto-expands when the current route matches its `prefix`
- The accordion's expanded state must be initialized from `page.url.pathname` on mount, not just from clicks. Use:
```typescript
// NOTE: Do NOT use a plain Set here. Svelte 5 runes track reactivity via
// property assignment on $state objects. Set.add() / Set.delete() are method
// calls that mutate without assignment, so $effect and {#if} blocks won't
// re-trigger. Use a Record<string, boolean> instead, where toggling a key
// (expanded[key] = !expanded[key]) is a tracked property write.
// (Alternatively, import { SvelteSet } from 'svelte/reactivity' which wraps
// Set with reactive semantics, but the Record pattern is simpler here.)
let expanded = $state<Record<string, boolean>>({})
$effect(() => {
  for (const group of navigation.filter(n => 'children' in n)) {
    if (page.url.pathname.startsWith(group.prefix)) {
      expanded[group.prefix] = true
    }
  }
})
// Toggle on click:  expanded[group.prefix] = !expanded[group.prefix]
```
- Only one group needs to be open at a time (optional: allow multiple)
- Expanded state persists via Svelte `$state` (no localStorage needed for MVP)

### 3.4 Active State

- Top-level items (Dashboard, Resumes): active when `page.url.pathname` matches exactly or starts with the href
- Sub-items: active when `page.url.pathname` starts with the item's href
- Parent group: visually indicated (e.g., bold label, subtle background) when any child is active

### 3.5 Visual Design

- Top-level items and group labels: same font size, padding as current nav links
- Group labels: show a chevron indicator (right-pointing when collapsed, down when expanded)
- Sub-items: indented 16px from group label, slightly smaller font
- Active sub-item: existing highlight style (left border + background) preserved
- Group dividers: thin horizontal rule between groups (optional)

### 3.6 Route Changes

#### New Routes (SvelteKit pages to create)

| New Route | Content | Migrated From |
|-----------|---------|---------------|
| `/data/sources` | Sources list + Bullets/Skills tabs | `/sources` + `/bullets` + `/skills` |
| `/data/summaries` | Summaries CRUD (new, requires Spec 2) | New view |
| `/data/domains` | Domains + Archetypes tabs | `/domains` + `/archetypes` |
| `/data/notes` | Notes list | `/notes` |
| `/opportunities/organizations` | Organizations list | `/organizations` |
| `/opportunities/job-descriptions` | Job Descriptions CRUD (new, requires Spec 4) | New view |
| `/config/profile` | Profile settings (new, placeholder) | New view |
| `/config/templates` | Resume templates management (new, requires Spec 8) | New view |
| `/config/export` | Export tools (new, placeholder) | New view |
| `/config/debug` | Prompt logs | `/logs` |

#### Redirects (old routes -> new routes)

Implement redirects using `+page.ts` files (not `.server.ts` -- this is an SPA with `ssr = false`). Each old route gets a `+page.ts` that calls `redirect(302, '/new/path')` from `@sveltejs/kit`:

```typescript
// e.g., packages/webui/src/routes/sources/+page.ts
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/data/sources') }
```

| Old Route | Redirects To |
|-----------|-------------|
| `/sources` | `/data/sources` |
| `/bullets` | `/data/sources?tab=bullets` |
| `/skills` | `/data/sources?tab=skills` |
| `/domains` | `/data/domains` |
| `/archetypes` | `/data/domains?tab=archetypes` |
| `/notes` | `/data/notes` |
| `/organizations` | `/opportunities/organizations` |
| `/logs` | `/config/debug` |
| `/chain` | No redirect (removed from nav, becomes modal -- see Spec 5) |
| `/derivation` | `/chain` (alias; renders the same Chain View page until Spec 5 converts it to a modal) |

### 3.7 Intentionally Preserved `/chain` Links

The `DragNDropView` component contains provenance tooltip links of the form `/chain?highlight=...`. These links are **intentionally preserved** by this spec -- they should NOT be redirected or removed here. Spec 5 (Chain View Modal) will update these links to open the chain view modal via a modal store instead of navigating to a route.

### 3.8 Tab Views Within Pages

Tabs are handled within the page component, NOT in the sidebar. Sidebar only navigates to the top-level view.

**Sources page** (`/data/sources`):
- Tab bar: Bullets | Skills
- Default tab: Bullets
- Tab state stored in URL query param `?tab=bullets` or `?tab=skills`

**Summaries page** (`/data/summaries`):
- Tab bar: Description | Title | Role | Contact Info
- Default tab: Description
- Contact Info tab is a read-only view of the Profile data

**Domains page** (`/data/domains`):
- Tab bar: Domains | Archetypes
- Default tab: Domains

### 3.9 File Structure

```
packages/webui/src/routes/
  +layout.svelte              -- updated with grouped nav
  +page.svelte                -- Dashboard (unchanged)
  data/
    sources/+page.svelte      -- Sources with Bullets/Skills tabs
    summaries/+page.svelte    -- Summaries CRUD (placeholder until Spec 2)
    domains/+page.svelte      -- Domains + Archetypes tabs
    notes/+page.svelte        -- Notes (migrated)
  opportunities/
    organizations/+page.svelte -- Organizations (migrated)
    job-descriptions/+page.svelte -- JDs (placeholder until Spec 4)
  resumes/
    +page.svelte              -- Resumes (unchanged path)
  config/
    profile/+page.svelte      -- Profile settings (placeholder)
    templates/+page.svelte    -- Resume templates (placeholder until Spec 8)
    export/+page.svelte       -- Export tools (placeholder)
    debug/+page.svelte        -- Logs (migrated from /logs)
```

---

## 4. Type Changes

No changes to `@forge/core` or `@forge/sdk` types. The only new types are the `NavItem`/`NavGroup`/`NavEntry` interfaces, defined locally in the layout component or a `$lib/nav.ts` module.

---

## 5. Acceptance Criteria

1. **Grouped sidebar** renders with Dashboard, Data, Opportunities, Resumes, Config sections
2. **Accordion behavior**: clicking a group label toggles expand/collapse of its children
3. **Auto-expand**: navigating to `/data/sources` auto-expands the Data group
4. **Active highlighting**: active sub-item shows left border highlight; active group label is visually distinct
5. **All old routes redirect** to new locations with 302 status
6. **Tab views** on Sources, Summaries, and Domains pages switch content without navigation
7. **URL query params** reflect active tab (`?tab=skills`, `?tab=archetypes`)
8. **No broken links**: all internal links throughout the app updated to new routes. All internal `<a href>` links and `goto()` calls across Svelte components that reference old routes are updated to new routes. Run both grep commands to find them:
   - `grep -rn 'href="/sources\|href="/organizations\|href="/skills\|href="/domains\|href="/archetypes\|href="/notes\|href="/logs\|href="/chain\|href="/bullets\|href="/derivation' packages/webui/src/`
   - `grep -rn 'goto("/sources\|goto("/organizations\|goto("/skills\|goto("/domains\|goto("/archetypes\|goto("/notes\|goto("/logs\|goto("/chain\|goto("/bullets\|goto("/derivation\|goto(`/sources\|goto(`/organizations\|goto(`/skills\|goto(`/domains\|goto(`/archetypes\|goto(`/notes\|goto(`/logs\|goto(`/chain\|goto(`/bullets\|goto(`/derivation' packages/webui/src/`
9. **Chain View** no longer appears in the sidebar
10. **Placeholder pages** exist for Summaries, Job Descriptions, Profile, Templates, Export with "Coming Soon" messaging
11. **Debug page relocated**: the existing `/logs` page content (at `packages/webui/src/routes/logs/+page.svelte`) is moved to `packages/webui/src/routes/config/debug/+page.svelte`. The redirect from `/logs` to `/config/debug` handles navigation, but the actual page file must be relocated.
12. **Direct URL to `/data/sources`** shows Data accordion pre-expanded
13. **All 9 old routes redirect** to correct new destinations via client-side `+page.ts` (including `/derivation` -> `/chain`)
14. **`/chain` route still renders** when accessed directly (even though removed from nav)
15. **All internal `<a href>` and `goto()` calls** updated to new routes

---

## 6. Dependencies & Parallelization

### Dependencies

- None for the navigation restructuring itself
- Blocks: **Spec 5** (Chain Modal must apply on top of the new layout structure)
- `/data/summaries` content depends on **Spec 2** (Summaries as Standalone Entities)
- `/opportunities/job-descriptions` content depends on **Spec 4** (Job Descriptions Entity)
- `/config/profile` content depends on **Spec 6** (Config -- Profile)
- `/config/templates` content depends on **Spec 8** (Resume Templates)

### Parallelization

This spec can be split into independent work streams:

| Stream | Description | Can run in parallel |
|--------|-------------|-------------------|
| A | Layout component + accordion logic | Start first |
| B | Route migration (move existing pages to new paths) | After A |
| C | Redirect hooks for old routes | After B |
| D | Tab view components (Sources, Domains) | Parallel with B |
| E | Placeholder pages (Summaries, JDs, Profile, Export) | Parallel with B |

---

## 7. Testing

- Verify grouped sidebar renders with all 4 sections plus Dashboard
- Verify clicking a group label toggles expand/collapse
- Direct navigation to `/data/sources` shows Data group pre-expanded
- All old routes redirect correctly (test each of the 8 redirects)
- `/chain` still renders standalone (not redirected, just removed from nav)
- Tab switching on Sources, Summaries, and Domains pages works without navigation
- URL query params update when switching tabs

---

## 8. Known Limitations

1. **No mobile responsive sidebar** — the sidebar is desktop-only for now; a hamburger menu or drawer pattern is deferred
2. **No deep-linking to tabs** — tab state is in query params which may not survive external link shares gracefully
3. **Chain View removal** — the Chain View page at `/chain` still exists but is unreachable from nav; Spec 5 (Chain View Modal) will convert it to a modal
4. **No route groups in SvelteKit** — we use flat directories under `/data/`, `/opportunities/`, `/config/` rather than SvelteKit route groups `(data)` to keep URLs clean
5. **Redirect persistence** — old bookmarks will work via redirects, but browser history entries are not cleaned up
6. **Combined Sources page** — merging Sources, Bullets, and Skills into one tabbed view is a UI density concern; may need to revisit if the page becomes too complex
