# Phase 32: Nav Restructuring

**Status:** Planning
**Date:** 2026-03-31
**Spec:** [2026-03-30-nav-restructuring.md](../refs/specs/2026-03-30-nav-restructuring.md)
**Depends on:** Phase 28 (all existing routes stable)
**Blocks:** Phase 33 (Chain View Modal)
**Parallelizable with:** Phases 29, 30, 31, 36

## Goal

Restructure the flat 10-item sidebar navigation in `+layout.svelte` into a grouped, accordion-style sidebar with 4 top-level sections (Data, Opportunities, Resumes, Config) plus a standalone Dashboard link. Create new route paths under `/data/`, `/opportunities/`, and `/config/` prefixes, migrate existing page content to those paths, set up client-side redirects from all old routes, and update every internal link across the codebase. This is a pure UI/routing change -- no schema, API, or SDK changes.

## Non-Goals

- Responsive/mobile sidebar (future)
- Drag-to-reorder nav items
- User-customizable nav sections
- Keyboard shortcut navigation
- Breadcrumb navigation in the content area
- The Chain View modal itself (Spec 5 / Phase 33)
- Content for Summaries, Job Descriptions, Profile, Templates, Export pages (those depend on Specs 2, 4, 6, 8)
- Updating DragNDropView `/chain?highlight=...` links (Spec 5 will handle those)

## Context

The current `+layout.svelte` renders a flat `navItems` array of 11 items. As the number of views grows, the flat list does not scale -- users must scan all items to find what they need, and related views (Sources/Skills, Domains/Archetypes) have no visual grouping. The spec defines a grouped accordion pattern with route prefix reorganization. Existing pages at `/sources`, `/bullets`, `/skills`, `/domains`, `/archetypes`, `/notes`, `/organizations`, and `/logs` must be moved to new paths, with redirect stubs at the old paths for bookmark compatibility.

The `/chain` route is intentionally kept alive (not redirected) because DragNDropView links still point to it. Spec 5 (Phase 33) will convert Chain View to a modal and update those links. The `/derivation` route currently redirects to `/bullets`; this phase changes it to redirect to `/chain` per spec.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 3.1 Sidebar structure | Yes |
| 3.2 Navigation data model | Yes |
| 3.3 Accordion behavior | Yes |
| 3.4 Active state | Yes |
| 3.5 Visual design | Yes |
| 3.6 Route changes | Yes |
| 3.7 Preserved `/chain` links | Yes (by not touching them) |
| 3.8 Tab views within pages | Yes (Sources, Domains) |
| 3.9 File structure | Yes |
| 1. Schema changes | N/A (none) |
| 2. API endpoints | N/A (none) |
| 4. Type changes | Yes (NavItem/NavGroup/NavEntry) |

## Files to Create

- `packages/webui/src/lib/nav.ts` -- navigation data model and types
- `packages/webui/src/routes/data/sources/+page.svelte` -- Sources with Bullets/Skills tabs
- `packages/webui/src/routes/data/summaries/+page.svelte` -- Summaries placeholder
- `packages/webui/src/routes/data/domains/+page.svelte` -- Domains + Archetypes tabs
- `packages/webui/src/routes/data/notes/+page.svelte` -- Notes (migrated)
- `packages/webui/src/routes/opportunities/organizations/+page.svelte` -- Organizations (migrated)
- `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` -- JDs placeholder
- `packages/webui/src/routes/config/profile/+page.svelte` -- Profile placeholder
- `packages/webui/src/routes/config/templates/+page.svelte` -- Templates placeholder
- `packages/webui/src/routes/config/export/+page.svelte` -- Export placeholder
- `packages/webui/src/routes/config/debug/+page.svelte` -- Logs (migrated)
- `packages/webui/src/routes/sources/+page.ts` -- redirect to `/data/sources`
- `packages/webui/src/routes/bullets/+page.ts` -- redirect to `/data/sources?tab=bullets`
- `packages/webui/src/routes/skills/+page.ts` -- redirect to `/data/sources?tab=skills`
- `packages/webui/src/routes/domains/+page.ts` -- redirect to `/data/domains`
- `packages/webui/src/routes/archetypes/+page.ts` -- redirect to `/data/domains?tab=archetypes`
- `packages/webui/src/routes/notes/+page.ts` -- redirect to `/data/notes`
- `packages/webui/src/routes/organizations/+page.ts` -- redirect to `/opportunities/organizations`
- `packages/webui/src/routes/logs/+page.ts` -- redirect to `/config/debug`
- `packages/webui/src/routes/derivation/+page.ts` -- redirect to `/chain`

## Files to Modify

- `packages/webui/src/routes/+layout.svelte` -- replace flat nav with grouped accordion sidebar
- `packages/webui/src/routes/+page.svelte` -- update `href="/bullets"` and `goto('/sources')` to new routes

## Files to Delete

- `packages/webui/src/routes/derivation/+page.svelte` -- replaced by `routes/derivation/+page.ts` redirect to `/chain`
- `packages/webui/src/routes/sources/+page.svelte` -- content moved to `routes/data/sources/SourcesView.svelte`
- `packages/webui/src/routes/bullets/+page.svelte` -- content moved to `routes/data/sources/BulletsView.svelte`
- `packages/webui/src/routes/skills/+page.svelte` -- content moved to `routes/data/sources/SkillsView.svelte`
- `packages/webui/src/routes/domains/+page.svelte` -- content moved to `routes/data/domains/DomainsView.svelte`
- `packages/webui/src/routes/archetypes/+page.svelte` -- content moved to `routes/data/domains/ArchetypesView.svelte`
- `packages/webui/src/routes/notes/+page.svelte` -- content moved to `routes/data/notes/+page.svelte`
- `packages/webui/src/routes/organizations/+page.svelte` -- content moved to `routes/opportunities/organizations/+page.svelte`
- `packages/webui/src/routes/logs/+page.svelte` -- content moved to `routes/config/debug/+page.svelte`

## Files Intentionally NOT Modified

- `packages/webui/src/lib/components/resume/DragNDropView.svelte` -- `/chain?highlight=...` links are preserved for Spec 5

## Fallback Strategies

| Risk | Fallback |
|------|----------|
| Accordion state not reactive | Fall back to `SvelteSet` from `svelte/reactivity` instead of `Record<string, boolean>` |
| Tab query params lost on navigation | Store active tab in a Svelte `$state` alongside query param sync; degrade to state-only |
| Old route redirect loops | If a redirect file conflicts with its directory, rename old `+page.svelte` before adding `+page.ts` |
| SvelteKit refuses colocated `+page.svelte` and `+page.ts` redirect | Use `+page.server.ts` with `redirect()` instead, or use `load` function in `+page.ts` that returns redirect |
| Auto-expand breaks on direct URL | Add a fallback `onMount` that also sets expanded state from `window.location.pathname` |

---

## Tasks

### T32.1: Create Navigation Data Model (`$lib/nav.ts`)

**File:** `packages/webui/src/lib/nav.ts`

Define the typed navigation structure and export it for use by the layout.

```typescript
/** A single navigation link (no children). */
export interface NavItem {
  href: string
  label: string
}

/** A collapsible group of navigation links. */
export interface NavGroup {
  label: string
  prefix: string
  children: NavItem[]
}

/** A top-level navigation entry: either a standalone link or a group. */
export type NavEntry = NavItem | NavGroup

/** Type guard: returns true if the entry is a NavGroup (has children). */
export function isNavGroup(entry: NavEntry): entry is NavGroup {
  return 'children' in entry
}

export const navigation: NavEntry[] = [
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

**Acceptance criteria:**
- `isNavGroup` correctly narrows `NavEntry` to `NavGroup` for entries with `children`
- `isNavGroup` returns `false` for plain `NavItem` entries
- `navigation` array contains exactly 5 top-level entries (Dashboard, Data, Opportunities, Resumes, Config)
- All `href` values start with `/`

**Failure criteria:**
- TypeScript compile errors on `NavEntry` discrimination
- Missing navigation entries compared to spec

---

### T32.2: Update Layout with Grouped Accordion Sidebar

**File:** `packages/webui/src/routes/+layout.svelte`

Replace the entire `<script>` block, `<nav>` markup, and add new styles for the accordion sidebar. The key implementation detail: use `$state<Record<string, boolean>>({})` for expanded group tracking, NOT `Set.add()`.

```svelte
<script>
  import { page } from '$app/state'
  import { ToastContainer } from '$lib/components'
  import { navigation, isNavGroup } from '$lib/nav'

  let { children } = $props()

  // Accordion expanded state: keyed by group prefix.
  // NOTE: Must use Record<string, boolean> with $state, NOT a Set.
  // Set.add() / Set.delete() are method calls that mutate without assignment,
  // so $effect and {#if} blocks won't re-trigger. Record property writes
  // (expanded[key] = value) ARE tracked by Svelte 5 runes.
  let expanded = $state<Record<string, boolean>>({})

  // Auto-expand group matching current route on navigation
  $effect(() => {
    for (const entry of navigation) {
      if (isNavGroup(entry) && page.url.pathname.startsWith(entry.prefix)) {
        expanded[entry.prefix] = true
      }
    }
  })

  function toggleGroup(prefix: string) {
    expanded[prefix] = !expanded[prefix]
  }

  function isActive(href: string): boolean {
    if (href === '/') return page.url.pathname === '/'
    return page.url.pathname.startsWith(href)
  }

  function isGroupActive(group: { prefix: string }): boolean {
    return page.url.pathname.startsWith(group.prefix)
  }
</script>

<div class="app">
  <nav class="sidebar">
    <div class="logo">
      <h2>Forge</h2>
    </div>
    <ul class="nav-list">
      {#each navigation as entry}
        {#if isNavGroup(entry)}
          <li class="nav-group">
            <button
              class="group-label"
              class:group-active={isGroupActive(entry)}
              onclick={() => toggleGroup(entry.prefix)}
            >
              <span>{entry.label}</span>
              <span class="chevron" class:open={expanded[entry.prefix]}>
                &#9656;
              </span>
            </button>
            {#if expanded[entry.prefix]}
              <ul class="group-children">
                {#each entry.children as child}
                  <li>
                    <a
                      href={child.href}
                      class:active={isActive(child.href)}
                    >
                      {child.label}
                    </a>
                  </li>
                {/each}
              </ul>
            {/if}
          </li>
        {:else}
          <li>
            <a
              href={entry.href}
              class="top-link"
              class:active={isActive(entry.href)}
            >
              {entry.label}
            </a>
          </li>
        {/if}
      {/each}
    </ul>
  </nav>
  <main class="content">
    {@render children()}
  </main>
</div>

<ToastContainer />

<style>
  :global(*, *::before, *::after) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
      Oxygen, Ubuntu, Cantarell, sans-serif;
    color: #1a1a1a;
    background: #f5f5f5;
  }

  .app {
    display: flex;
    min-height: 100vh;
  }

  .sidebar {
    width: 220px;
    background: #1a1a2e;
    color: #e0e0e0;
    padding: 1.5rem 0;
    flex-shrink: 0;
    overflow-y: auto;
  }

  .logo {
    padding: 0 1.5rem 1.5rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    margin-bottom: 1rem;
  }

  .logo h2 {
    font-size: 1.25rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: #fff;
  }

  .nav-list {
    list-style: none;
  }

  /* Top-level standalone links (Dashboard, Resumes) */
  .top-link {
    display: block;
    padding: 0.625rem 1.5rem;
    color: #b0b0c0;
    text-decoration: none;
    font-size: 0.9rem;
    transition: background 0.15s, color 0.15s;
  }

  .top-link:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #fff;
  }

  .top-link.active {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
    border-left: 3px solid #6c63ff;
    padding-left: calc(1.5rem - 3px);
  }

  /* Group label (accordion toggle) */
  .nav-group {
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  .nav-group:first-child {
    border-top: none;
  }

  .group-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.625rem 1.5rem;
    background: none;
    border: none;
    color: #b0b0c0;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    text-align: left;
  }

  .group-label:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #fff;
  }

  .group-label.group-active {
    color: #fff;
    font-weight: 600;
  }

  .chevron {
    font-size: 0.7rem;
    transition: transform 0.15s;
    display: inline-block;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  /* Child links within a group */
  .group-children {
    list-style: none;
  }

  .group-children li a {
    display: block;
    padding: 0.45rem 1.5rem 0.45rem 2.5rem;
    color: #9090a8;
    text-decoration: none;
    font-size: 0.82rem;
    transition: background 0.15s, color 0.15s;
  }

  .group-children li a:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #d0d0e0;
  }

  .group-children li a.active {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
    border-left: 3px solid #6c63ff;
    padding-left: calc(2.5rem - 3px);
  }

  .content {
    flex: 1;
    padding: 2rem;
    overflow-y: auto;
  }
</style>
```

**Key decisions:**
- The `isActive()` helper uses `pathname.startsWith(prefix)`. Be aware that `/resumes` would match `/resumes-archive` if such a route existed. This is acceptable for the current route set but should be reviewed if new routes are added under similar prefixes.
- `$state<Record<string, boolean>>({})` instead of `Set` -- Svelte 5 runes track property assignment, not method calls
- `$effect` watches `page.url.pathname` to auto-expand groups on route change (handles direct URL, browser back/forward)
- Chevron uses `&#9656;` (right-pointing triangle, U+25B8) rotated 90deg when open
- Sub-items indented with `padding-left: 2.5rem` (16px indent from group label's 1.5rem)
- Group dividers via `border-top` on `.nav-group`

**Acceptance criteria:**
- Sidebar renders 5 top-level entries: Dashboard, Data, Opportunities, Resumes, Config
- Clicking Data expands its 4 children; clicking again collapses
- Navigating to `/data/sources` auto-expands the Data group
- Active child link shows left border highlight
- Active group label is bold/white
- Chain View does NOT appear in sidebar

**Failure criteria:**
- Accordion does not expand/collapse (likely `Set` was used instead of `Record`)
- Active state not updating on navigation (missing `$effect` on `page.url.pathname`)
- Styles from old nav leaking through (old `li a` selectors too broad)

---

### T32.3: Create Sources Tabbed Page (Bullets + Skills tabs)

**File:** `packages/webui/src/routes/data/sources/+page.svelte`

This page combines three previously separate views: Sources (`/sources`), Bullets (`/bullets`), and Skills (`/skills`). The Sources content is always visible in the left panel. The right panel switches between Bullets and Skills via tab query params.

The implementation uses `page.url.searchParams` for tab state and `goto()` with `replaceState` for tab switching without adding history entries.

```svelte
<script lang="ts">
  import { page } from '$app/state'
  import { goto } from '$app/navigation'
  import SourcesView from './SourcesView.svelte'
  import BulletsView from './BulletsView.svelte'
  import SkillsView from './SkillsView.svelte'

  const TABS = [
    { value: 'bullets', label: 'Bullets' },
    { value: 'skills', label: 'Skills' },
  ]

  let activeTab = $derived(page.url.searchParams.get('tab') ?? 'bullets')

  function switchTab(tab: string) {
    goto(`/data/sources?tab=${tab}`, { replaceState: true })
  }
</script>

<div class="sources-page-container">
  <div class="sources-panel">
    <SourcesView />
  </div>
  <div class="tab-panel">
    <div class="tab-bar">
      {#each TABS as tab}
        <button
          class="tab-btn"
          class:active={activeTab === tab.value}
          onclick={() => switchTab(tab.value)}
        >
          {tab.label}
        </button>
      {/each}
    </div>
    <div class="tab-content">
      {#if activeTab === 'skills'}
        <SkillsView />
      {:else}
        <BulletsView />
      {/if}
    </div>
  </div>
</div>

<style>
  .sources-page-container {
    display: flex;
    height: calc(100vh - 4rem);
    margin: -2rem;
  }

  .sources-panel {
    width: 380px;
    flex-shrink: 0;
    border-right: 1px solid #e5e7eb;
    overflow-y: auto;
  }

  .tab-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .tab-bar {
    display: flex;
    border-bottom: 1px solid #e5e7eb;
    background: #fff;
    padding: 0 1rem;
  }

  .tab-btn {
    padding: 0.75rem 1.25rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    font-size: 0.85rem;
    font-weight: 500;
    color: #6b7280;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab-btn:hover {
    color: #374151;
  }

  .tab-btn.active {
    color: #6c63ff;
    border-bottom-color: #6c63ff;
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
  }
</style>
```

**Variable collision warning:** When extracting SourcesView, BulletsView, SkillsView as child components, ensure each uses props for tab selection (e.g., `export let activeTab`) rather than reading from the URL directly -- the parent page manages URL state and passes it down. If `SourcesView.svelte` internally uses `activeTab` and the parent page also has `activeTab`, the extracted component must use its own local state or accept the tab as a prop.

**Refactoring approach:** Rather than duplicating the 970-line Sources page and the 677-line Bullets page inline, extract the existing page contents into sub-components:

- `packages/webui/src/routes/data/sources/SourcesView.svelte` -- extracted from `routes/sources/+page.svelte` (the list panel + editor)
- `packages/webui/src/routes/data/sources/BulletsView.svelte` -- extracted from `routes/bullets/+page.svelte` (the content atoms view)
- `packages/webui/src/routes/data/sources/SkillsView.svelte` -- extracted from `routes/skills/+page.svelte` (the skills list + editor)

Each `*View.svelte` file is a copy of its original `+page.svelte` content with the outermost wrapper class adjusted so it fills its container rather than setting full-page height. Specifically:

- `SourcesView.svelte`: Copy `routes/sources/+page.svelte` verbatim. Change `.sources-page { height: calc(100vh - 4rem); margin: -2rem; }` to `.sources-page { height: 100%; }`.
- `BulletsView.svelte`: Copy `routes/bullets/+page.svelte` verbatim. Add `padding: 1.5rem;` wrapper.
- `SkillsView.svelte`: Copy `routes/skills/+page.svelte` verbatim. Change `.skills-page { height: calc(100vh - 4rem); margin: -2rem; }` to `.skills-page { height: 100%; }`.

**Files to create for this task:**
- `packages/webui/src/routes/data/sources/+page.svelte` (tab container above)
- `packages/webui/src/routes/data/sources/SourcesView.svelte` (from `/sources`)
- `packages/webui/src/routes/data/sources/BulletsView.svelte` (from `/bullets`)
- `packages/webui/src/routes/data/sources/SkillsView.svelte` (from `/skills`)

**Dual tab state:** The Sources page has two independent selection states: (1) the source-type filter (experience/project/education/clearance) which is internal to SourcesView, and (2) the top-level tab (Sources/Bullets/Skills) which is URL-driven via `?tab=`. These are intentionally separate -- the source-type filter does not persist in the URL.

**Acceptance criteria:**
- `/data/sources` renders Sources list on left, Bullets view on right (default tab)
- `/data/sources?tab=skills` renders Sources list on left, Skills view on right
- `/data/sources?tab=bullets` renders Sources list on left, Bullets view on right
- Tab switching updates URL query param without full navigation
- All existing Sources/Bullets/Skills functionality preserved

**Failure criteria:**
- Tab switch causes full page reload (missing `replaceState: true`)
- `$derived` on `page.url.searchParams` not reactive (verify `page` is from `$app/state`, not `$app/stores`)
- Extracted views have broken full-page height styles (negative margin from original pages)

---

### T32.4: Create Domains Tabbed Page (Domains + Archetypes tabs)

**File:** `packages/webui/src/routes/data/domains/+page.svelte`

Same tab pattern as Sources. Combines Domains and Archetypes views.

```svelte
<script lang="ts">
  import { page } from '$app/state'
  import { goto } from '$app/navigation'
  import DomainsView from './DomainsView.svelte'
  import ArchetypesView from './ArchetypesView.svelte'

  const TABS = [
    { value: 'domains', label: 'Domains' },
    { value: 'archetypes', label: 'Archetypes' },
  ]

  let activeTab = $derived(page.url.searchParams.get('tab') ?? 'domains')

  function switchTab(tab: string) {
    goto(`/data/domains?tab=${tab}`, { replaceState: true })
  }
</script>

<div class="domains-container">
  <div class="tab-bar">
    {#each TABS as tab}
      <button
        class="tab-btn"
        class:active={activeTab === tab.value}
        onclick={() => switchTab(tab.value)}
      >
        {tab.label}
      </button>
    {/each}
  </div>
  <div class="tab-content">
    {#if activeTab === 'archetypes'}
      <ArchetypesView />
    {:else}
      <DomainsView />
    {/if}
  </div>
</div>

<style>
  .domains-container {
    max-width: 1000px;
  }

  .tab-bar {
    display: flex;
    border-bottom: 1px solid #e5e7eb;
    margin-bottom: 1.5rem;
  }

  .tab-btn {
    padding: 0.75rem 1.25rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    font-size: 0.85rem;
    font-weight: 500;
    color: #6b7280;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab-btn:hover {
    color: #374151;
  }

  .tab-btn.active {
    color: #6c63ff;
    border-bottom-color: #6c63ff;
  }

  .tab-content {
    min-height: 0;
  }
</style>
```

**Files to create for this task:**
- `packages/webui/src/routes/data/domains/+page.svelte` (tab container above)
- `packages/webui/src/routes/data/domains/DomainsView.svelte` (copy from `routes/domains/+page.svelte`, remove outer wrapper sizing)
- `packages/webui/src/routes/data/domains/ArchetypesView.svelte` (copy from `routes/archetypes/+page.svelte`, remove outer wrapper sizing)

**Acceptance criteria:**
- `/data/domains` shows Domains tab by default
- `/data/domains?tab=archetypes` shows Archetypes tab
- Tab switching updates URL without full navigation
- All existing Domains/Archetypes functionality preserved

**Failure criteria:**
- Archetype domain picker broken after extraction (ensure `$lib/sdk` imports still resolve)

---

### T32.5: Migrate Notes Page

**File:** `packages/webui/src/routes/data/notes/+page.svelte`

Copy `packages/webui/src/routes/notes/+page.svelte` contents to the new path. No content changes needed -- the Notes view is a standalone page.

**Acceptance criteria:**
- `/data/notes` renders the full Notes UI
- All CRUD operations work at the new path

**Failure criteria:**
- Import paths broken (all should use `$lib/` which is path-independent)

---

### T32.6: Migrate Organizations Page

**File:** `packages/webui/src/routes/opportunities/organizations/+page.svelte`

Copy `packages/webui/src/routes/organizations/+page.svelte` contents to the new path. No content changes needed.

**Acceptance criteria:**
- `/opportunities/organizations` renders the full Organizations UI
- All CRUD operations work at the new path

---

### T32.7: Migrate Logs Page to Config/Debug

**File:** `packages/webui/src/routes/config/debug/+page.svelte`

Copy `packages/webui/src/routes/logs/+page.svelte` contents to the new path.

The current logs page is a placeholder with a "Coming soon" EmptyState. Copy it verbatim:

```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="debug-page">
  <h1 class="page-title">Prompt Logs</h1>
  <p class="subtitle">AI derivation audit trail</p>

  <EmptyState
    title="Coming soon"
    description="Prompt logs will appear here after AI derivation runs. A dedicated logs endpoint is in progress."
  />
</div>

<style>
  .debug-page {
    max-width: 800px;
  }

  .page-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: #1a1a2e;
    margin-bottom: 0.25rem;
  }

  .subtitle {
    font-size: 0.85rem;
    color: #6b7280;
    margin-bottom: 1.5rem;
  }
</style>
```

**Acceptance criteria:**
- `/config/debug` renders the Prompt Logs placeholder
- CSS class renamed from `.logs-page` to `.debug-page` to match new context

---

### T32.8: Create Placeholder Pages

Create 5 placeholder pages for views that depend on future specs. Each follows the same pattern.

#### T32.8a: Summaries Placeholder

**File:** `packages/webui/src/routes/data/summaries/+page.svelte`

```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="summaries-page">
  <h1 class="page-title">Summaries</h1>
  <p class="subtitle">Professional summaries, titles, roles, and contact info</p>

  <EmptyState
    title="Coming soon"
    description="Summary management will be available after Summaries as Standalone Entities is implemented."
  />
</div>

<style>
  .summaries-page { max-width: 800px; }
  .page-title { font-size: 1.5rem; font-weight: 700; color: #1a1a2e; margin-bottom: 0.25rem; }
  .subtitle { font-size: 0.85rem; color: #6b7280; margin-bottom: 1.5rem; }
</style>
```

#### T32.8b: Job Descriptions Placeholder

**File:** `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte`

```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="jd-page">
  <h1 class="page-title">Job Descriptions</h1>
  <p class="subtitle">Track and analyze job postings</p>

  <EmptyState
    title="Coming soon"
    description="Job description management will be available after the Job Descriptions Entity spec is implemented."
  />
</div>

<style>
  .jd-page { max-width: 800px; }
  .page-title { font-size: 1.5rem; font-weight: 700; color: #1a1a2e; margin-bottom: 0.25rem; }
  .subtitle { font-size: 0.85rem; color: #6b7280; margin-bottom: 1.5rem; }
</style>
```

#### T32.8c: Profile Placeholder

**File:** `packages/webui/src/routes/config/profile/+page.svelte`

```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="profile-page">
  <h1 class="page-title">Profile</h1>
  <p class="subtitle">Your personal and contact information</p>

  <EmptyState
    title="Coming soon"
    description="Profile settings will be available after the Config Profile spec is implemented."
  />
</div>

<style>
  .profile-page { max-width: 800px; }
  .page-title { font-size: 1.5rem; font-weight: 700; color: #1a1a2e; margin-bottom: 0.25rem; }
  .subtitle { font-size: 0.85rem; color: #6b7280; margin-bottom: 1.5rem; }
</style>
```

#### T32.8d: Templates Placeholder

**File:** `packages/webui/src/routes/config/templates/+page.svelte`

```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="templates-page">
  <h1 class="page-title">Templates</h1>
  <p class="subtitle">Resume templates and layout configuration</p>

  <EmptyState
    title="Coming soon"
    description="Template management will be available after the Resume Templates spec is implemented."
  />
</div>

<style>
  .templates-page { max-width: 800px; }
  .page-title { font-size: 1.5rem; font-weight: 700; color: #1a1a2e; margin-bottom: 0.25rem; }
  .subtitle { font-size: 0.85rem; color: #6b7280; margin-bottom: 1.5rem; }
</style>
```

#### T32.8e: Export Placeholder

**File:** `packages/webui/src/routes/config/export/+page.svelte`

```svelte
<script lang="ts">
  import { EmptyState } from '$lib/components'
</script>

<div class="export-page">
  <h1 class="page-title">Export</h1>
  <p class="subtitle">Export resumes and data</p>

  <EmptyState
    title="Coming soon"
    description="Export tools will be available in a future release."
  />
</div>

<style>
  .export-page { max-width: 800px; }
  .page-title { font-size: 1.5rem; font-weight: 700; color: #1a1a2e; margin-bottom: 0.25rem; }
  .subtitle { font-size: 0.85rem; color: #6b7280; margin-bottom: 1.5rem; }
</style>
```

**Acceptance criteria (all 5):**
- Each placeholder page renders with title, subtitle, and "Coming soon" EmptyState
- No broken imports
- Pages accessible at their new routes

---

### T32.9: Create Redirect Stubs for Old Routes

Each old route gets a `+page.ts` file that throws a `redirect(302, ...)`. The old `+page.svelte` files are then deleted (or, if SvelteKit allows both files, the `+page.ts` redirect takes precedence in the load function).

**Atomic delete/create:** For each redirect, the old `+page.svelte` must be deleted in the same commit as the new `+page.ts` redirect is created. Do not delete the old page without creating the redirect -- this leaves the route broken.

**Important:** In SvelteKit, when both `+page.ts` and `+page.svelte` exist, the `load` function in `+page.ts` runs first. If `load` throws a redirect, the `+page.svelte` never renders. However, to keep things clean, delete the old `+page.svelte` after creating the `+page.ts` redirect -- the old content has been moved to new locations.

#### Redirect Files

Each file follows this exact pattern:

**`packages/webui/src/routes/sources/+page.ts`**
```typescript
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/data/sources') }
```

**`packages/webui/src/routes/bullets/+page.ts`**
```typescript
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/data/sources?tab=bullets') }
```

**`packages/webui/src/routes/skills/+page.ts`**
```typescript
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/data/sources?tab=skills') }
```

**`packages/webui/src/routes/domains/+page.ts`**
```typescript
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/data/domains') }
```

**`packages/webui/src/routes/archetypes/+page.ts`**
```typescript
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/data/domains?tab=archetypes') }
```

**`packages/webui/src/routes/notes/+page.ts`**
```typescript
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/data/notes') }
```

**`packages/webui/src/routes/organizations/+page.ts`**
```typescript
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/opportunities/organizations') }
```

**`packages/webui/src/routes/logs/+page.ts`**
```typescript
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/config/debug') }
```

**`packages/webui/src/routes/derivation/+page.ts`**
```typescript
import { redirect } from '@sveltejs/kit'
export const load = () => { throw redirect(302, '/chain') }
```

After creating each `+page.ts`, delete the corresponding old `+page.svelte` for that route (the content has been moved to its new location):

- Delete `packages/webui/src/routes/sources/+page.svelte`
- Delete `packages/webui/src/routes/bullets/+page.svelte`
- Delete `packages/webui/src/routes/skills/+page.svelte`
- Delete `packages/webui/src/routes/domains/+page.svelte`
- Delete `packages/webui/src/routes/archetypes/+page.svelte`
- Delete `packages/webui/src/routes/notes/+page.svelte`
- Delete `packages/webui/src/routes/organizations/+page.svelte`
- Delete `packages/webui/src/routes/logs/+page.svelte`
- Delete `packages/webui/src/routes/derivation/+page.svelte`

**Note on derivation:** The current `derivation/+page.svelte` uses `onMount` + `goto('/bullets')`. This is fully replaced by the `derivation/+page.ts` redirect above. Delete the old `+page.svelte` as part of this task.

**Do NOT delete or redirect:**
- `packages/webui/src/routes/chain/+page.svelte` -- Chain View stays at `/chain` until Phase 33 converts it to a modal

**Acceptance criteria:**
- Navigating to `/sources` redirects to `/data/sources`
- Navigating to `/bullets` redirects to `/data/sources?tab=bullets`
- Navigating to `/skills` redirects to `/data/sources?tab=skills`
- Navigating to `/domains` redirects to `/data/domains`
- Navigating to `/archetypes` redirects to `/data/domains?tab=archetypes`
- Navigating to `/notes` redirects to `/data/notes`
- Navigating to `/organizations` redirects to `/opportunities/organizations`
- Navigating to `/logs` redirects to `/config/debug`
- Navigating to `/derivation` redirects to `/chain`
- Navigating to `/chain` still renders the Chain View page (no redirect)
- All 9 redirects use 302 status

**Failure criteria:**
- Redirect loop (e.g., if old `+page.svelte` somehow takes precedence over `+page.ts`)
- `/chain` redirected or broken
- Old `+page.svelte` files left behind causing confusion (delete them after confirming new routes work)

---

### T32.10: Update Dashboard Links

**File:** `packages/webui/src/routes/+page.svelte`

Update all internal links in the dashboard to use new routes. Based on grep results, there are 3 links and 1 `goto()` call to update:

1. `goto('/sources')` (line 108) -> `goto('/data/sources')`
2. `href="/bullets"` (line 112, Pending Bullets card) -> `href="/data/sources?tab=bullets"`
3. `href="/bullets"` (line 117, Pending Perspectives card) -> `href="/data/sources?tab=bullets"`
4. `href="/chain"` (line 135, Integrity Alerts card) -> keep as `href="/chain"` (Chain View is intentionally preserved at this path)

Specific edits:

```diff
-          onaction={() => goto('/sources')}
+          onaction={() => goto('/data/sources')}
```

```diff
-          <a href="/bullets" class="pending-card">
+          <a href="/data/sources?tab=bullets" class="pending-card">
             <div class="pending-count">{pendingBullets}</div>
```

```diff
-          <a href="/bullets" class="pending-card">
+          <a href="/data/sources?tab=bullets" class="pending-card">
             <div class="pending-count">{pendingPerspectives}</div>
```

The `href="/chain"` on line 135 is intentionally kept -- Chain View still lives at `/chain` and Spec 5 will update this.

**Acceptance criteria:**
- "View Sources" button navigates to `/data/sources`
- Pending Bullets card navigates to `/data/sources?tab=bullets`
- Pending Perspectives card navigates to `/data/sources?tab=bullets`
- Integrity Alerts card still navigates to `/chain`
- No `goto()` or `href` references to old routes remain in the dashboard (except `/chain`)

**Failure criteria:**
- Dashboard links still point to `/sources` or `/bullets`
- `goto` import missing after update

---

### T32.11: Verify No Remaining Old Route References

Run both grep commands from the spec acceptance criteria to verify all internal links have been updated:

```bash
# Check <a href> links
grep -rn 'href="/sources\|href="/organizations\|href="/skills\|href="/domains\|href="/archetypes\|href="/notes\|href="/logs\|href="/bullets\|href="/derivation' packages/webui/src/

# Check goto() calls
grep -rn 'goto("/sources\|goto("/organizations\|goto("/skills\|goto("/domains\|goto("/archetypes\|goto("/notes\|goto("/logs\|goto("/bullets\|goto("/derivation\|goto(`/sources\|goto(`/organizations\|goto(`/skills\|goto(`/domains\|goto(`/archetypes\|goto(`/notes\|goto(`/logs\|goto(`/bullets\|goto(`/derivation' packages/webui/src/
```

**Note:** `href="/chain"` references are intentionally allowed. Only `/chain` links in DragNDropView are preserved. The dashboard `/chain` link is also preserved per spec.

**Positive assertion for `/chain` references:** After cleanup, grep should find exactly 5 references to `/chain` in `.svelte` files: 3 in DragNDropView (provenance tooltip links with `?highlight=...`), 1 in the Dashboard (`href="/chain"` on the Integrity Alerts card), and 1 in `chain/+page.svelte` (the page itself). The `derivation/+page.ts` redirect stub adds 1 more reference in a `.ts` file. If the count differs, investigate.

**Acceptance criteria:**
- Both grep commands return zero results (excluding any files inside `routes/*/+page.ts` redirect stubs which don't contain href/goto)
- `/chain` references are only in: DragNDropView (3 occurrences), Dashboard (1 occurrence), and `chain/+page.svelte` (the page itself)

**Failure criteria:**
- grep finds unreplaced old-route references in any `.svelte` component

---

## Testing Support

### Test Kinds

| Kind | What to test | Tool |
|------|-------------|------|
| Unit | `isNavGroup` type guard, `navigation` structure | Vitest |
| Component | Sidebar rendering, accordion expand/collapse, active state | Vitest + @testing-library/svelte (if configured) |
| Smoke | Each new route loads without error | Manual or Playwright |
| Integration | Redirect stubs send correct 302 responses | SvelteKit test harness or manual |
| Visual | Sidebar appearance, tab styles, placeholder pages | Manual |

### Unit Tests

**File:** `packages/webui/src/lib/nav.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { navigation, isNavGroup } from './nav'
import type { NavEntry, NavGroup, NavItem } from './nav'

describe('nav', () => {
  describe('isNavGroup', () => {
    it('returns true for entries with children', () => {
      const group: NavGroup = { label: 'Data', prefix: '/data', children: [{ href: '/data/x', label: 'X' }] }
      expect(isNavGroup(group)).toBe(true)
    })

    it('returns false for plain NavItem', () => {
      const item: NavItem = { href: '/', label: 'Dashboard' }
      expect(isNavGroup(item)).toBe(false)
    })
  })

  describe('navigation', () => {
    it('has 5 top-level entries', () => {
      expect(navigation).toHaveLength(5)
    })

    it('starts with Dashboard as a plain link', () => {
      expect(isNavGroup(navigation[0])).toBe(false)
      expect((navigation[0] as NavItem).href).toBe('/')
      expect((navigation[0] as NavItem).label).toBe('Dashboard')
    })

    it('has Data group with 4 children', () => {
      const data = navigation[1]
      expect(isNavGroup(data)).toBe(true)
      expect((data as NavGroup).label).toBe('Data')
      expect((data as NavGroup).children).toHaveLength(4)
    })

    it('has Opportunities group with 2 children', () => {
      const opp = navigation[2]
      expect(isNavGroup(opp)).toBe(true)
      expect((opp as NavGroup).label).toBe('Opportunities')
      expect((opp as NavGroup).children).toHaveLength(2)
    })

    it('has Resumes as a plain link', () => {
      expect(isNavGroup(navigation[3])).toBe(false)
      expect((navigation[3] as NavItem).href).toBe('/resumes')
    })

    it('has Config group with 4 children', () => {
      const config = navigation[4]
      expect(isNavGroup(config)).toBe(true)
      expect((config as NavGroup).label).toBe('Config')
      expect((config as NavGroup).children).toHaveLength(4)
    })

    it('all hrefs start with /', () => {
      for (const entry of navigation) {
        if (isNavGroup(entry)) {
          for (const child of entry.children) {
            expect(child.href).toMatch(/^\//)
          }
        } else {
          expect(entry.href).toMatch(/^\//)
        }
      }
    })

    it('does not contain Chain View', () => {
      const labels: string[] = []
      for (const entry of navigation) {
        if (isNavGroup(entry)) {
          for (const child of entry.children) labels.push(child.label)
        } else {
          labels.push(entry.label)
        }
      }
      expect(labels).not.toContain('Chain View')
    })
  })
})
```

### Smoke Tests (Manual Checklist)

1. Load `/` -- Dashboard renders, Data group collapsed by default
2. Click "Data" in sidebar -- expands to show Sources, Summaries, Domains, Notes
3. Click "Sources" -- navigates to `/data/sources`, Data group stays expanded
4. Verify Bullets tab is default, click Skills tab -- URL changes to `/data/sources?tab=skills`
5. Click "Domains" -- navigates to `/data/domains`, Domains tab is default
6. Click Archetypes tab -- URL changes to `/data/domains?tab=archetypes`
7. Navigate directly to `/data/notes` -- Data group auto-expanded, Notes link active
8. Click "Opportunities" -- expands Organizations, Job Descriptions
9. Click "Resumes" -- navigates to `/resumes` (unchanged path)
10. Click "Config" -- expands Profile, Templates, Export, Debug (Logs)
11. Navigate to `/config/debug` -- Config group expanded, Debug link active
12. Test each redirect: `/sources`, `/bullets`, `/skills`, `/domains`, `/archetypes`, `/notes`, `/organizations`, `/logs`, `/derivation`
13. Navigate to `/chain` directly -- Chain View renders (not redirected)
14. Check DragNDropView provenance tooltip links -- they still point to `/chain?highlight=...` (Requires: a resume loaded in the resume builder with provenance data. If no such data exists in dev, skip -- the grep in T32.11 is the primary verification.)

### Redirect Test Matrix

**Note:** 9 total redirects: 8 content routes (/sources, /bullets, /skills, /domains, /archetypes, /notes, /organizations, /logs) + 1 alias (/derivation -> /chain).

| Old URL | Expected Redirect | Expected Query Params |
|---------|------------------|-----------------------|
| `/sources` | `/data/sources` | none |
| `/bullets` | `/data/sources` | `?tab=bullets` |
| `/skills` | `/data/sources` | `?tab=skills` |
| `/domains` | `/data/domains` | none |
| `/archetypes` | `/data/domains` | `?tab=archetypes` |
| `/notes` | `/data/notes` | none |
| `/organizations` | `/opportunities/organizations` | none |
| `/logs` | `/config/debug` | none |
| `/derivation` | `/chain` | none |

---

## Documentation Requirements

No external documentation files needed. The navigation structure is self-documenting through the typed `$lib/nav.ts` module. If the project has a component storybook or design docs, the sidebar changes should be noted there, but that is not in scope for this phase.

Internal code comments are included in `+layout.svelte` explaining the `Record<string, boolean>` vs `Set` decision for Svelte 5 reactivity.

---

## Parallelization Notes

This phase can be split into 4 independent work streams after T32.1 and T32.2 are complete:

| Stream | Tasks | Dependencies |
|--------|-------|-------------|
| A: Nav Model + Layout | T32.1, T32.2 | Start first -- all other streams depend on this |
| B: Page Migration | T32.3, T32.4, T32.5, T32.6, T32.7 | After A (pages must exist at new paths before redirects) |
| C: Placeholders | T32.8a-e | Parallel with B (no dependencies on existing pages) |
| D: Redirects + Link Updates | T32.9, T32.10 | After B (old pages must be moved before deletion) |
| E: Verification | T32.11 | After D (all link updates must be complete before grep verification) |

Stream C (placeholders) can run fully in parallel with B since placeholder pages are new files with no content dependencies. Stream D must run after B because the redirect stubs delete the old `+page.svelte` files that B migrates content from.

This phase can run in parallel with Phases 29-31 and 36 because it only touches WebUI routing and layout -- it does not modify core, SDK, or API code.
