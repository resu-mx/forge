# Forge Design System: Layout & Containers

**Date:** 2026-04-04
**Doc:** 2 of 6 (Design System Series)
**Status:** Reference specification
**Prerequisite:** [Doc 1 -- Foundation](2026-04-04-design-system-foundation.md) (vocabulary, tokens, architecture)

This document defines the viewport-level containers, the three page types, and the scroll/padding contracts that govern how every page in Forge is laid out. It is the most critical spec for debugging layout issues (sticky headers not sticking, double scrollbars, content overflow).

---

## 1. Viewport Architecture

The root layout (`+layout.svelte`) establishes a three-region viewport. The left sidebar is always visible. The content area fills remaining space. The right sidebar is a contextual overlay that appears on demand.

```
+------------------------------------------------------------------+
|                         VIEWPORT (100vw x 100vh)                 |
|                                                                  |
|  +----------+  +--------------------------------------+  +-----+ |
|  |          |  |                                      |  |     | |
|  |   LEFT   |  |            CONTENT AREA              |  | RT  | |
|  | SIDEBAR  |  |                                      |  | SB  | |
|  |          |  |   +------------------------------+   |  |     | |
|  |  220px   |  |   |                              |   |  | 320 | |
|  |  sticky  |  |   |    Page Content               |   |  | px  | |
|  |  full-h  |  |   |    (FlowPage / AppPage /     |   |  |     | |
|  |          |  |   |     DualModePage)             |   |  | opt | |
|  |  +----+  |  |   |                              |   |  |     | |
|  |  |logo|  |  |   |                              |   |  |     | |
|  |  +----+  |  |   +------------------------------+   |  |     | |
|  |  | nav|  |  |                                      |  |     | |
|  |  | ...|  |  |   padding: var(--content-padding)    |  |     | |
|  |  | ...|  |  |   overflow-y: auto                   |  |     | |
|  |  +----+  |  |                                      |  |     | |
|  |  |user|  |  +--------------------------------------+  +-----+ |
|  +----------+                                                    |
+------------------------------------------------------------------+

Left Sidebar: fixed 220px, sticky, full viewport height
Content Area:  flex: 1, overflow-y: auto, padding: 2rem
Right Sidebar: optional, 320px, overlays on mobile, pushes on desktop
```

### 1.1 Root Container (`.app`)

The outermost `<div class="app">` is a flex row spanning the full viewport.

```css
.app {
  display: flex;
  min-height: 100vh;
}
```

**Rules:**
- The `.app` container NEVER has `overflow` set. It is a passthrough flex container.
- It has exactly three persistent children: `.sidebar`, `.content`, and (optionally) `.right-sidebar`. The right sidebar is INSIDE `.app` as a flex sibling (see Section 4.4). Toast container and modals are mounted as portals outside `.app`.

---

## 2. Left Sidebar

### 2.1 Specification

| Property | Value | Token |
|----------|-------|-------|
| Width | 220px | `--sidebar-width` (gap -- currently hardcoded) |
| Background | sidebar background | `var(--color-sidebar-bg)` |
| Text color | sidebar text | `var(--color-sidebar-text)` |
| Position | sticky, top: 0 | -- |
| Height | 100vh | -- |
| Flex | `flex-shrink: 0` | -- |
| Internal layout | flex column | -- |

### 2.2 Structure

```
.sidebar (sticky, full height, flex column)
  +-- .logo (h2 "Forge", border-bottom)
  +-- .nav-list (flex: 1, overflow-y: auto)
  |     +-- NavItem (top-level link, e.g., "Dashboard")
  |     +-- NavGroup (accordion)
  |           +-- .group-label (toggle button)
  |           +-- .group-children (collapsible child links)
  +-- .profile-button-area (border-top, flex-shrink: 0)
        +-- ProfileMenu (popover)
```

### 2.3 Scroll Behavior

The `.nav-list` is the scroll container for navigation items. It has `flex: 1` and `overflow-y: auto`, so when the nav grows taller than the viewport (many groups expanded), the nav scrolls independently while the logo above and profile button below remain pinned.

```css
.nav-list {
  list-style: none;
  flex: 1;
  overflow-y: auto;
}
```

The `.profile-button-area` has `flex-shrink: 0` to prevent it from being squeezed out when nav overflows.

### 2.4 Navigation Route Mapping

Navigation is defined in `$lib/nav.ts` as an array of `NavEntry` items. Each entry is either a `NavItem` (standalone link) or a `NavGroup` (collapsible accordion with children).

| Nav Entry | Type | Routes |
|-----------|------|--------|
| Dashboard | NavItem | `/` |
| Experience | NavGroup | `/experience/roles`, `/experience/projects`, `/experience/education`, `/experience/clearances`, `/experience/general` |
| Data | NavGroup | `/data/bullets`, `/data/skills`, `/data/contacts`, `/data/organizations`, `/data/domains`, `/data/notes` |
| Opportunities | NavGroup | `/opportunities/organizations`, `/opportunities/job-descriptions` |
| Resumes | NavGroup | `/resumes`, `/resumes/summaries`, `/resumes/templates` |

**Active state logic:**
- Root routes (`/`, `/resumes`): exact match (`pathname === href`)
- All other routes: prefix match (`pathname.startsWith(href)`)
- Group headers: highlighted when any child is active (`pathname.startsWith(group.prefix)`)

**Accordion behavior:**
- Groups auto-expand when the current route matches their prefix (via `$effect`)
- Clicking a group label toggles expansion (manual override)
- Expanded state is tracked as `Record<string, boolean>` (not `Set` -- see ADR-005 reactivity note)

### 2.5 Sidebar CSS Reference

```css
.sidebar {
  width: 220px;                         /* TODO: var(--sidebar-width) */
  background: var(--color-sidebar-bg);
  color: var(--color-sidebar-text);
  padding: 1.5rem 0 0;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  position: sticky;
  top: 0;
  height: 100vh;
  overflow-y: auto;
}
```

---

## 3. Content Area

### 3.1 Specification

| Property | Value | Token |
|----------|-------|-------|
| Flex | `flex: 1` | -- |
| Padding | 2rem (all sides) | `--content-padding` (gap -- currently hardcoded as `--space-8`) |
| Overflow | `overflow-y: auto` | -- |
| Background | inherited from body | `var(--color-bg)` |

### 3.2 The Scroll Container

The `.content` element is THE scroll container for FlowPages. This is the single most important fact in the layout system:

```css
.content {
  flex: 1;
  padding: 2rem;
  overflow-y: auto;
}
```

**Implications:**
- `overflow-y: auto` on `.content` creates a scroll context. Any `position: sticky` child will stick relative to `.content`'s scroll position.
- FlowPages render directly inside `.content` and scroll naturally within it.
- AppPages must cancel `.content`'s padding and constrain their own height (see Section 5.2).
- `.content` MUST NOT have `overflow: hidden` -- that would break FlowPage scrolling.

### 3.3 Content Area Children

The `.content` element renders `{@render children()}`, which is the current route's `+page.svelte` output. There is nothing else inside `.content` -- no additional wrappers, headers, or chrome. The page itself is responsible for its internal structure.

---

## 4. Right Sidebar (NEW)

### 4.1 Purpose

A contextual slide-out panel for supplementary data that does not warrant a full page or a modal. Examples:
- Gap Analysis results on the resumes page
- Graph filter controls on the chain view
- Metadata inspector for selected entities

### 4.2 Specification

| Property | Value | Token |
|----------|-------|-------|
| Width | 320px | `--right-sidebar-width` (new token) |
| Background | surface | `var(--color-surface)` |
| Border | left border | `1px solid var(--color-border)` |
| Shadow | medium | `var(--shadow-md)` |
| Z-index | above content, below modals | `var(--z-sidebar)` (new token, value ~500) |
| Animation | slide from right, 200ms ease | `var(--transition-normal)` (new token) |

### 4.3 Behavior

**Opening:** Triggered by a page action (button click, entity selection). The right sidebar slides in from the right edge.

**Closing:** Close button in the sidebar header, Escape key, or clicking the backdrop (mobile only).

**Desktop (>1024px):** The right sidebar PUSHES the content area narrower. The `.content` element shrinks via flex to accommodate. No backdrop.

**Mobile/tablet (<1024px):** The right sidebar OVERLAYS the content area with a semi-transparent backdrop (`var(--color-overlay)`). Content does not reflow.

### 4.4 Layout Integration

The right sidebar is a sibling of `.content` inside `.app`, not a child of `.content`:

```svelte
<div class="app">
  <nav class="sidebar">...</nav>
  <main class="content">
    {@render children()}
  </main>
  {#if rightSidebarOpen}
    <aside class="right-sidebar">
      {@render rightSidebarContent()}
    </aside>
  {/if}
</div>
```

### 4.5 CSS Reference

```css
.right-sidebar {
  width: 320px;                          /* var(--right-sidebar-width) */
  flex-shrink: 0;
  background: var(--color-surface);
  border-left: 1px solid var(--color-border);
  box-shadow: var(--shadow-md);
  display: flex;
  flex-direction: column;
  height: 100vh;
  position: relative;                    /* flex sibling, not sticky -- pushed by flex layout */
  overflow-y: auto;
  animation: slide-in-right 0.2s ease;
}

@keyframes slide-in-right {
  from { transform: translateX(100%); }
  to   { transform: translateX(0); }
}

/* Mobile: overlay mode */
@media (max-width: 1024px) {
  .right-sidebar {
    position: fixed;
    top: 0;
    right: 0;
    height: 100vh;
    z-index: var(--z-sidebar, 500);
  }

  .right-sidebar-backdrop {
    position: fixed;
    inset: 0;
    background: var(--color-overlay);
    z-index: calc(var(--z-sidebar, 500) - 1);
  }
}
```

### 4.6 When to Use

| Use right sidebar | Use modal instead | Use a new page instead |
|-------------------|-------------------|------------------------|
| Supplementary data for the current view | Destructive confirmations | Complex multi-step workflows |
| Filters, metadata, analysis results | Entity creation forms | Independent feature areas |
| User may need to reference main content | Quick-edit overlays | Navigation-worthy destinations |
| Content is read-heavy, not form-heavy | Blocking user input required | Content has its own URL/route |

---

## 5. Page Types

Forge pages fall into three categories based on their scroll and layout behavior. Every `+page.svelte` must conform to exactly one of these contracts.

### 5.1 FlowPage

**Definition:** Content scrolls naturally inside the Content Area's `overflow-y: auto` scroll context. The `.content` element's 2rem padding wraps the page content.

**Wrapper component:** None. Just render content directly.

**Scroll owner:** `.content` (the Content Area)

**Use for:** Forms, settings, dashboards, simple lists, template galleries, summaries.

**Current examples:** `/config/profile`, `/config/export`, `/data/notes`, `/data/domains`, `/resumes/templates`, `/resumes/summaries`, `/` (dashboard), `/chain` (note: graph container height is set explicitly to fill available space)

**Template:**
```svelte
<!-- FlowPage: no PageWrapper, no height constraints -->
<PageHeader title="Settings" subtitle="Configure your profile" />

<div class="form-group">
  <TitledDataInput label="Name" bind:value={name} />
  <TitledDataInput label="Email" bind:value={email} />
</div>

<div class="form-actions">
  <button class="btn btn-primary" onclick={save}>Save</button>
</div>
```

**CSS contract:**
- The page NEVER sets `height`, `max-height`, or `overflow` on its root element.
- The page NEVER adds outer padding or margin -- it inherits `.content`'s 2rem padding.
- The page MAY use `position: sticky` on internal headers; they stick relative to `.content`.

### 5.2 AppPage

**Definition:** The page takes over the full viewport height, cancels `.content`'s padding, and manages its own internal scroll regions. No part of the page scrolls via `.content` -- all scrolling happens inside internal panes.

**Wrapper component:** `PageWrapper` (required)

**Scroll owner:** Internal regions (`.split-list`, `.split-detail`, `.column-body`, etc.)

**Use for:** Split panels, kanban boards -- any "app-like" interface with fixed header areas and independently scrolling regions.

**Current examples:** `/data/contacts` (SplitPanel), `/data/organizations` (SplitPanel), `/data/skills` (PageWrapper), `/opportunities/organizations` (KanbanBoard)

**Template:**
```svelte
<!-- AppPage: PageWrapper constrains height, panels scroll internally -->
<PageWrapper overflow="hidden">
  <ListPanelHeader title="Sources" onNew={startCreate} />
  <SplitPanel listWidth={320}>
    {#snippet list()}
      <div class="list-scroll-region">
        {#each items as item}
          <Entry onclick={() => select(item.id)}>{item.name}</Entry>
        {/each}
      </div>
    {/snippet}
    {#snippet detail()}
      <DetailEditor entity={selected} />
    {/snippet}
  </SplitPanel>
</PageWrapper>
```

**CSS contract:**
- The page MUST wrap content in `<PageWrapper>`.
- `PageWrapper` cancels `.content`'s padding and constrains height to the viewport.
- Internal scroll regions MUST set `overflow-y: auto` on the appropriate child.
- The page NEVER relies on `.content` for scrolling.

### 5.3 DualModePage

**Definition:** A page that switches between two AppPage-style layouts. A `ViewToggle` controls the active mode. Both modes are rendered inside the same `PageWrapper`.

**Wrapper component:** `PageWrapper` (required, wraps both modes)

**Scroll owner:** Each mode's internal regions

**Use for:** Entities that support both list/detail (SplitPanel) and board (KanbanBoard) views.

**Current examples:** `/opportunities/job-descriptions` (list + board), `/data/bullets` (delegates to a view component that handles both), `/resumes` (list + board)

**Template:**
```svelte
<PageWrapper>
  <ListPanelHeader title="Job Descriptions" onNew={startCreate}>
    {#snippet actions()}
      <ViewToggle mode={viewMode} onchange={(m) => viewMode = m} />
    {/snippet}
  </ListPanelHeader>

  {#if viewMode === 'board'}
    <div class="board-container">
      <GenericKanban columns={columns} items={items} onDrop={handleDrop}>
        <!-- kanban snippets -->
      </GenericKanban>
    </div>
  {:else}
    <SplitPanel>
      {#snippet list()} ... {/snippet}
      {#snippet detail()} ... {/snippet}
    </SplitPanel>
  {/if}
</PageWrapper>
```

**CSS contract:**
- The `PageWrapper` stays constant -- it does not remount when the view switches.
- The `.board-container` must be `flex: 1; min-height: 0; overflow: hidden;` to fill the remaining space and allow the kanban to scroll its columns internally.
- The `SplitPanel` also fills remaining space via `flex: 1; min-height: 0;`.

### 5.4 Page Type Decision Tree

```
Does the page need fixed-height regions that scroll independently?
  NO  -> FlowPage (no PageWrapper)
  YES -> Does it support multiple view modes?
           NO  -> AppPage (PageWrapper)
           YES -> DualModePage (PageWrapper + ViewToggle)
```

---

## 6. PageWrapper Component

### 6.1 Purpose

`PageWrapper` is the bridge between the Content Area's default behavior (padded, scrolling) and an AppPage's requirements (full height, no outer scroll, internal scroll regions).

### 6.2 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `overflow` | `'auto' \| 'hidden' \| 'visible'` | `'hidden'` | Overflow mode for the wrapper |
| `children` | `Snippet` | required | Page content |

### 6.3 What It Does

```css
.page-wrapper {
  /* Cancel .content's 2rem padding by pulling outward */
  margin: calc(-1 * var(--space-8));

  /* Fill the full viewport height, accounting for the padding that was cancelled */
  height: calc(100vh - var(--space-8) * 2);

  /* Column layout so children can flex-fill */
  display: flex;
  flex-direction: column;
}

.page-wrapper--overflow-hidden  { overflow: hidden; }
.page-wrapper--overflow-auto    { overflow: auto; }
.page-wrapper--overflow-visible { overflow: visible; }
```

### 6.4 The Padding Cancellation Trick

This is the core mechanism. Understand it or nothing else makes sense.

1. `.content` has `padding: 2rem` (which is `var(--space-8)` -- note: `--space-8: 2rem` per the spacing scale).
2. FlowPages live happily inside this padding.
3. AppPages need to fill the full viewport width and height, edge to edge within the content area.
4. `PageWrapper` applies `margin: calc(-1 * var(--space-8))` -- a negative margin equal to the padding -- which pulls the wrapper's edges outward to align with `.content`'s border box.
5. The height is set to `calc(100vh - var(--space-8) * 2)` to account for the padding that would have existed on top and bottom (the negative margin cancels the padding visually, but the viewport height calculation must still subtract it because `.content`'s padding still affects its content box size).

**Why not just conditionally remove padding from `.content`?**
- `.content` is defined in `+layout.svelte`, which does not know which page type is currently rendered.
- SvelteKit routes change via `{@render children()}` -- the layout cannot reactively adjust padding per route.
- The negative-margin approach is a well-established CSS pattern that keeps the layout contract simple: `.content` always has padding, pages opt out when needed.

### 6.5 Token Gap

The hardcoded `var(--space-8)` works because `--space-8: 2rem` matches `.content`'s padding. But this coupling is fragile. When `--content-padding` is added as a token (see Foundation Doc, Section 3.2), both `.content` and `PageWrapper` should reference it:

```css
/* Future: tokens.css */
--content-padding: var(--space-8);

/* Future: +layout.svelte */
.content { padding: var(--content-padding); }

/* Future: PageWrapper.svelte */
.page-wrapper {
  margin: calc(-1 * var(--content-padding));
  height: calc(100vh - var(--content-padding) * 2);
}
```

---

## 7. SplitPanel Component

Documented in detail in Doc 5 (Interactive Systems) for drag-and-drop behavior. Summary here for layout context.

### 7.1 Structure

```
.split-panel (flex row, flex: 1, min-height: 0)
  +-- .split-list (fixed width, flex-shrink: 0, border-right)
  |     +-- [header area -- not scrolled]
  |     +-- [list items -- overflow-y: auto on a child]
  +-- .split-detail (flex: 1, overflow-y: auto)
        +-- [detail content -- scrolls within]
```

### 7.2 CSS

```css
.split-panel {
  display: flex;
  flex: 1;
  min-height: 0;           /* Critical: allows flex children to shrink below content size */
}

.split-list {
  flex-shrink: 0;
  width: 320px;             /* configurable via listWidth prop */
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow: hidden;          /* Prevents list from scrolling as a whole */
  background: var(--color-surface);
}

.split-detail {
  flex: 1;
  overflow-y: auto;          /* Detail pane is its own scroll container */
  min-width: 0;
  background: var(--color-surface);
}
```

### 7.3 The `min-height: 0` Rule

By default, flex items have `min-height: auto`, which prevents them from shrinking below their content size. Without `min-height: 0` on `.split-panel`, the panel would grow to fit all list items, pushing below the viewport. This is the most common source of "page is taller than viewport" bugs in AppPages.

---

## 8. Scroll Behavior Contract

This section is the definitive reference for which element scrolls in each page type. If a scroll-related bug occurs, start here.

### 8.1 Scroll Map

```
FlowPage
  .content .............. scrolls (overflow-y: auto)
    page content ........ flows naturally, no height constraint
      sticky headers .... stick relative to .content

AppPage (SplitPanel)
  .content .............. DOES NOT scroll (PageWrapper fills it exactly)
    PageWrapper ......... overflow: hidden
      ListPanelHeader ... fixed at top (not sticky -- it's above scroll regions)
      .split-panel ...... flex row, does not scroll
        .split-list ..... overflow: hidden (container)
          header ........ fixed at top of list
          .card-list .... scrolls (overflow-y: auto on this child)
        .split-detail ... scrolls (overflow-y: auto)

AppPage (KanbanBoard)
  .content .............. DOES NOT scroll
    PageWrapper ......... overflow: hidden
      ListPanelHeader ... fixed at top
      .board-container .. flex: 1, overflow: hidden
        .kanban-board ... flex row, overflow-x: auto (horizontal scroll)
          .column ....... flex column
            .column-header  fixed at top of column
            .column-body .. scrolls (overflow-y: auto)

DualModePage
  .content .............. DOES NOT scroll
    PageWrapper ......... overflow: hidden
      ListPanelHeader ... fixed at top (persists across mode switch)
      [mode === 'list']
        SplitPanel ...... (same as AppPage/SplitPanel above)
      [mode === 'board']
        .board-container  (same as AppPage/KanbanBoard above)
```

### 8.2 The Golden Rule

**Headers in AppPages NEVER scroll.** They are placed ABOVE the scroll regions in the flex layout. They do not use `position: sticky` -- they are simply not inside a scrolling container.

**Headers in FlowPages MAY use `position: sticky`.** They stick relative to `.content`'s scroll position. This requires that no ancestor between the sticky element and `.content` has `overflow` set (see Section 9).

### 8.3 Double-Scrollbar Prevention

A double scrollbar occurs when both `.content` and an internal region scroll. This happens when an AppPage fails to fill `.content` exactly, leaving `.content` with overflow.

**Prevention checklist:**
1. AppPage uses `PageWrapper` (cancels padding, constrains height).
2. `PageWrapper` has `overflow: hidden` (default).
3. Every flex child in the chain from `PageWrapper` down to the scroll region has `min-height: 0` or a fixed height.
4. Exactly one element in each vertical chain has `overflow-y: auto`.

---

## 9. Sticky Headers: How They Work

### 9.1 CSS `position: sticky` Requirements

For `position: sticky` to work, ALL of the following must be true:

1. **The sticky element has `position: sticky` and a `top` value** (e.g., `top: 0`).
2. **No ancestor between the sticky element and the scroll container has `overflow: hidden`, `overflow: auto`, or `overflow: scroll`.** Any of these create a new scroll context that traps the sticky element.
3. **The sticky element's containing block (its parent) is taller than the sticky element.** The element sticks while its parent is in view and unsticks when its parent scrolls out.

### 9.2 FlowPage Sticky Headers

In a FlowPage, `.content` is the scroll container. A sticky header inside a FlowPage sticks at the top of `.content` as the user scrolls.

```svelte
<!-- FlowPage with sticky header -->
<div class="page-section">
  <h2 class="section-header">Section Title</h2>
  <div class="section-content">
    <!-- long content -->
  </div>
</div>

<style>
  .section-header {
    position: sticky;
    top: 0;
    background: var(--color-bg);
    padding: var(--space-3) 0;
    z-index: var(--z-dropdown);
    border-bottom: 1px solid var(--color-border);
  }
</style>
```

**Critical:** The `.page-section` parent MUST NOT have `overflow` set. If it does, the sticky element sticks to `.page-section` instead of `.content`, and since `.page-section` scrolls with `.content`, the sticky effect is invisible.

**Ancestry chain check:**
```
.content (overflow-y: auto)  <-- scroll container
  .page-section              <-- NO overflow! (default: visible)
    .section-header          <-- position: sticky; top: 0
    .section-content
```

### 9.3 AppPage Sticky Headers (Don't Use Sticky)

In an AppPage, headers do NOT use `position: sticky`. Instead, they are placed above the scroll regions in the flex layout:

```svelte
<PageWrapper overflow="hidden">
  <!-- This header is NOT inside any scroll container -->
  <ListPanelHeader title="Sources" onNew={startCreate} />

  <!-- The SplitPanel is below the header and scrolls internally -->
  <SplitPanel>
    {#snippet list()}
      <!-- The list panel header is also above the scroll region -->
      <div class="list-header">Search / Filter</div>
      <div class="card-list" style="overflow-y: auto; flex: 1;">
        <!-- items scroll here -->
      </div>
    {/snippet}
    {#snippet detail()}
      <!-- detail scrolls via .split-detail's overflow-y: auto -->
    {/snippet}
  </SplitPanel>
</PageWrapper>
```

**Why not sticky in AppPages?** Because `PageWrapper` has `overflow: hidden`, which creates a scroll context that traps sticky elements. And even if it didn't, AppPage headers should be in fixed positions by design -- they're part of the chrome, not the content.

### 9.4 Sticky Inside AppPage Scroll Regions

Within an AppPage's scroll regions (`.split-detail`, `.column-body`), sticky elements DO work because the scroll region itself is the nearest ancestor with overflow:

```svelte
<!-- Inside .split-detail (overflow-y: auto) -->
<div class="detail-section">
  <h3 class="detail-section-header">Skills</h3>
  <div class="detail-section-content">
    <!-- long list of skills -->
  </div>
</div>

<style>
  .detail-section-header {
    position: sticky;
    top: 0;
    background: var(--color-surface);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    z-index: 1;
  }
</style>
```

This works because the ancestry chain is:
```
.split-detail (overflow-y: auto)  <-- scroll container
  .detail-section                 <-- no overflow
    .detail-section-header        <-- sticky: sticks to .split-detail
    .detail-section-content
```

---

## 10. Padding Strategy

### 10.1 The Single Source Rule

**`.content`'s `padding: 2rem` is the ONLY source of outer page padding in the entire app.**

- FlowPages inherit this padding. They render inside it. They NEVER add their own outer margin or padding.
- AppPages cancel this padding via `PageWrapper`'s negative margin and manage their own internal padding (or have none -- split panels and kanban boards go edge-to-edge).

### 10.2 Internal Padding

Components inside pages manage their own internal padding:

| Component | Internal Padding | Source |
|-----------|-----------------|--------|
| `ListPanelHeader` | `var(--space-5) var(--space-4)` | Component scoped style |
| `.split-detail` | None (detail components add their own) | -- |
| KanbanColumn header | `var(--space-3) var(--space-4)` | Component scoped style |
| KanbanColumn body | `var(--space-2)` | Component scoped style |
| Form groups | `var(--space-6)` gap | `.form-group` in `base.css` |

### 10.3 What Goes Wrong

**Bug pattern: Double padding**
```svelte
<!-- WRONG: FlowPage adds its own outer padding -->
<div style="padding: 2rem;">
  <PageHeader title="Settings" />
</div>
```
Result: 4rem total padding (2rem from `.content` + 2rem from the div). Content appears indented too far.

**Fix:** Remove the div's padding. `.content` already provides it.

**Bug pattern: AppPage content not edge-to-edge**
```svelte
<!-- WRONG: AppPage without PageWrapper -->
<SplitPanel>
  {#snippet list()} ... {/snippet}
  {#snippet detail()} ... {/snippet}
</SplitPanel>
```
Result: SplitPanel is inset 2rem from all edges due to `.content`'s padding. It also overflows vertically because there's no height constraint.

**Fix:** Wrap in `<PageWrapper>`.

---

## 11. Tokens Required (Gaps to Fill)

The following tokens are referenced in this spec but not yet defined in `tokens.css`. They should be added as part of the layout system stabilization:

| Token | Value | Purpose |
|-------|-------|---------|
| `--sidebar-width` | `220px` | Left sidebar width; used in sidebar CSS and any content-area width calculations |
| `--content-padding` | `var(--space-8)` | Content area padding; consumed by `.content` and `PageWrapper` for consistent cancellation math |
| `--right-sidebar-width` | `320px` | Right sidebar width |
| `--z-sidebar` | `500` | Z-index for right sidebar overlay (between dropdown and modal) |
| `--transition-fast` | `0.15s ease` | Standard fast transition (hover states, accordion open/close) |
| `--transition-normal` | `0.2s ease` | Standard normal transition (sidebar slide, panel open/close) |

---

## 12. Implementation Checklist

When building a new page, follow this checklist:

1. **Decide the page type** using the decision tree (Section 5.4).
2. **FlowPage:** Render content directly. Use `PageHeader` for title. Do not add outer padding.
3. **AppPage:** Wrap in `<PageWrapper>`. Set `overflow="hidden"` (default). Ensure every flex child in the vertical chain has `min-height: 0`. Verify only internal regions scroll.
4. **DualModePage:** Single `<PageWrapper>` wrapping both modes. `ViewToggle` in the header persists across modes. Each mode manages its own scroll.
5. **Test:** Resize browser to various heights. Verify no double scrollbar. Verify headers stay fixed. Verify scroll regions scroll independently.
6. **Sticky headers:** Only use in FlowPages (relative to `.content`) or inside AppPage scroll regions. Never on AppPage top-level headers.

---

## 13. Known Patterns and Anti-Patterns

### 13.1 Sidebar Double `overflow-y: auto`

Both `.sidebar` and `.nav-list` have `overflow-y: auto`. This is intentional: `.sidebar`'s overflow catches any edge case where the entire sidebar container overflows (e.g., due to dynamic content), while `.nav-list`'s overflow is the primary scroll region for navigation items. In practice, only `.nav-list` scrolls because it has `flex: 1` and its sibling elements (logo, profile button) have fixed sizes.

### 13.2 Self-Wrapping Component Anti-Pattern

`KanbanBoard.svelte` (the legacy version) wraps itself in a `PageWrapper` internally, which means the page-level orchestrator cannot control the wrapper's overflow mode or share the wrapper with sibling components. This violates the principle that containers are composed by pages, not by views.

**Correct pattern:** The page wraps the kanban in `<PageWrapper>`:
```svelte
<PageWrapper overflow="hidden">
  <GenericKanban columns={columns} items={items} onDrop={handleDrop}>
    ...
  </GenericKanban>
</PageWrapper>
```

**Anti-pattern:** The kanban component wraps itself:
```svelte
<!-- KanbanBoard.svelte -- WRONG -->
<PageWrapper overflow="hidden">
  <div class="board-columns">...</div>
</PageWrapper>
```

`GenericKanban` does NOT have this problem -- it is a pure view component that expects an external `PageWrapper`.

### 13.3 Mobile Layout

**Minimum viewport width:** The sidebar-plus-content layout assumes a minimum viewport of approximately 768px. Below this, the left sidebar should collapse to an icon-only rail or a hamburger-triggered overlay. This behavior is not yet implemented.

**Sidebar collapse on mobile:** When the viewport is below 768px:
- The left sidebar should hide and be accessible via a hamburger menu button
- The right sidebar should use fixed overlay mode (already specified in Section 4.3)
- PageWrapper height calculation may need adjustment for mobile viewports without persistent sidebars

**Status:** Mobile layout is planned but not yet implemented. The current layout is desktop-only.

---

## 14. Cross-References

| Doc | Relevance |
|-----|-----------|
| **Doc 1** (Foundation) | Token definitions, CSS architecture rules, component taxonomy |
| **Doc 3** (Navigation & Headers) | PageHeader, ListPanelHeader, ViewToggle, TabBar |
| **Doc 4** (Content Patterns) | Entry, PaddedEntry, forms, detail panels |
| **Doc 5** (Interactive Systems) | KanbanBoard, modals, drawers, drag-and-drop |
| **Doc 6** (Data Viz) | Graph viewport and chart container layout requirements |

---

## Acceptance Criteria

1. FlowPage shows exactly one scrollbar (`.content`'s `overflow-y: auto`). No double scrollbars appear at any browser height.
2. AppPage headers (ListPanelHeader, PageHeader inside PageWrapper) never scroll -- they remain pinned above scroll regions.
3. Right sidebar pushes content narrower on desktop (>1024px) via flex layout, not overlay.
4. Right sidebar overlays content on mobile (<1024px) with backdrop.
5. PageWrapper negative margin math correctly cancels `.content`'s 2rem padding at all viewport sizes.
6. SplitPanel's `.split-list` and `.split-detail` scroll independently -- resizing the browser height does not produce double scrollbars.
7. All three page types (FlowPage, AppPage, DualModePage) are correctly classified in the route mapping and decision tree.
8. No `overflow` property is set on `.app` -- it is a passthrough flex container only.
