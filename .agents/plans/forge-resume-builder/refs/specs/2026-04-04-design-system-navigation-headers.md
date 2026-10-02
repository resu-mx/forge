# Forge Design System: Navigation & Headers

**Date:** 2026-04-04
**Doc:** 3 of 6 (Design System Series)
**Status:** Reference specification
**Depends on:** Doc 1 (Design System Foundation)

This document specifies every navigation and header component in the Forge UI, including their APIs, CSS, composition patterns, and sticky behavior. Components are organized by layer (Component or Atom per Doc 1 Section 1) and cross-referenced to their source files.

---

## 1. Header System Overview

Headers in Forge are composable. Rather than a single monolithic header component, the system provides a set of focused pieces that compose into different header layouts depending on context.

**Layer classification:** Headers are **Components** (layer 3). They are domain-aware, know about page titles and actions, and compose atoms (buttons, inputs) for their interactive elements.

**Composition principle:** A header is always a flex row with content on the left (title + subtitle) and actions on the right. The specific pieces vary by context:

```
+--------------------------------------------------+
| [Title]  [Subtitle]              [Actions]       |
+--------------------------------------------------+
```

Two concrete header components exist: `PageHeader` (full-page headings) and `ListPanelHeader` (split-panel list pane headings). Both follow the same structural pattern but differ in sizing, padding, and border treatment.

---

## 2. PageHeader

**File:** `$lib/components/PageHeader.svelte`
**Layer:** Component

The primary page-level header. Renders an `<h1>` title with optional subtitle text and an actions slot for buttons and toggles.

### 2.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `title` | `string` | (required) | Page heading text |
| `subtitle` | `string` | `undefined` | Muted descriptive text below the title |
| `actions` | `Snippet` | `undefined` | Render slot for action buttons, toggles, etc. |

### 2.2 Usage

```svelte
<script lang="ts">
  import { PageHeader, ViewToggle } from '$lib/components'
</script>

<PageHeader title="Content Atoms" subtitle="Unified view of bullets and perspectives">
  {#snippet actions()}
    <ViewToggle mode={viewMode} onchange={handleViewChange} />
    <button class="btn btn-primary" onclick={create}>+ New</button>
  {/snippet}
</PageHeader>
```

### 2.3 CSS Specification

```css
.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
  margin-bottom: var(--space-6);
}

.page-header-text {
  flex: 1;
  min-width: 0;                          /* prevent text from pushing actions off-screen */
}

.page-header h1 {
  font-size: var(--text-2xl);
  font-weight: var(--font-bold);
  color: var(--text-primary);
  line-height: var(--leading-tight);
  margin: 0;
}

.page-header .page-header-subtitle {
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-top: var(--space-1);
  line-height: var(--leading-normal);
}

.page-header-actions {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
```

**Token usage:**
- Title: `--text-2xl` size, `--font-bold` weight, `--text-primary` color
- Subtitle: `--text-sm` size, `--text-muted` color
- Spacing: `--space-4` gap between title and actions, `--space-6` bottom margin, `--space-1` between title and subtitle
- Actions: `--space-2` gap between action elements

### 2.4 Do / Don't

| Do | Don't |
|----|-------|
| Use `PageHeader` for top-of-page headings in FlowPage and AppPage layouts | Don't use `PageHeader` inside a split-panel list pane (use `ListPanelHeader`) |
| Pass `ViewToggle` and buttons via the `actions` snippet | Don't put filter bars inside `PageHeader` actions; use a separate `.filter-bar` row below |
| Keep subtitle text to one line | Don't use subtitle for multi-paragraph descriptions |

### 2.5 Accessibility

- Renders a semantic `<header>` element wrapping the content
- Title uses `<h1>`, establishing the page's heading hierarchy
- Actions area has no explicit ARIA role; individual buttons inside should have descriptive labels

---

## 3. ListPanelHeader

**File:** `$lib/components/ListPanelHeader.svelte`
**Layer:** Component

Header for the list pane of a `SplitPanel` layout. Renders an `<h2>` title with a primary-colored "New" button and an optional actions slot.

### 3.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `title` | `string` | (required) | Panel heading text |
| `onNew` | `() => void` | `undefined` | Callback for the "New" button; button is hidden when omitted |
| `newLabel` | `string` | `'+ New'` | Label text for the "New" button |
| `actions` | `Snippet` | `undefined` | Render slot for additional action elements |

### 3.2 Usage

```svelte
<script lang="ts">
  import { ListPanelHeader, SplitPanel } from '$lib/components'
</script>

<SplitPanel {listWidth}>
  {#snippet list()}
    <ListPanelHeader title="Sources" onNew={createSource} newLabel="+ New Source" />
    <!-- list entries below -->
  {/snippet}
  {#snippet detail()}
    <!-- detail pane -->
  {/snippet}
</SplitPanel>
```

### 3.3 CSS Specification

```css
.list-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-5) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  gap: var(--space-2);
}

.list-panel-header__title {
  font-size: var(--text-xl);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
  margin: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.list-panel-header__actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-shrink: 0;
}

.list-panel-header__new-btn {
  padding: 0.35rem var(--space-3);
  background: var(--color-primary);
  color: var(--text-inverse);
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  cursor: pointer;
  transition: background 0.15s;
  white-space: nowrap;
}

.list-panel-header__new-btn:hover {
  background: var(--color-primary-hover);
}

.list-panel-header__new-btn:focus-visible {
  outline: 2px solid var(--color-border-focus);
  outline-offset: 2px;
}
```

**Token usage:**
- Title: `--text-xl` size (one step down from PageHeader's `--text-2xl`), `--font-semibold` weight
- Padding: `--space-5` vertical, `--space-4` horizontal
- Border: `1px solid var(--color-border)` on bottom edge
- New button: `--color-primary` bg, `--text-inverse` text, `--radius-md`, `--text-sm` size

### 3.4 Structural Differences from PageHeader

| Aspect | PageHeader | ListPanelHeader |
|--------|-----------|-----------------|
| Semantic element | `<header>` | `<div>` |
| Heading level | `<h1>` | `<h2>` |
| Title size | `--text-2xl` | `--text-xl` |
| Title weight | `--font-bold` | `--font-semibold` |
| Bottom border | None | `1px solid var(--color-border)` |
| Bottom margin | `var(--space-6)` | None (border provides visual separation) |
| Built-in button | None (use actions snippet) | `onNew` renders a primary button |

### 3.5 Accessibility

- Title is `<h2>`, maintaining heading hierarchy under the page's `<h1>`
- The "New" button has a `type="button"` attribute to prevent form submission
- Focus-visible styling uses `--color-border-focus` with `2px` offset

---

## 4. TabbedHeader Pattern

**Layer:** Page-level composition (not a standalone component)

A TabbedHeader is a `TabBar` placed immediately below or alongside a page heading. This is a composition pattern, not a separate component. The pattern is used when a page has sub-sections navigated by tabs.

### 4.1 Usage Pattern

```svelte
<script lang="ts">
  import { TabBar } from '$lib/components'

  const TABS = [
    { value: 'domains', label: 'Domains' },
    { value: 'archetypes', label: 'Archetypes' },
  ]

  let activeTab = $state('domains')
</script>

<div class="page-container">
  <TabBar tabs={TABS} active={activeTab} onchange={(tab) => activeTab = tab} />
  <div class="tab-content">
    {#if activeTab === 'archetypes'}
      <ArchetypesView />
    {:else}
      <DomainsView />
    {/if}
  </div>
</div>
```

**Real-world example:** `/data/domains` uses this pattern. The `TabBar` sits at the top of the page and switches between `DomainsView` and `ArchetypesView`. Each sub-view renders its own `PageHeader` internally.

### 4.2 With PageHeader + TabBar

When a page needs both a heading and tabs:

```svelte
<PageHeader title="Data Management" subtitle="Domains, archetypes, and skills" />
<TabBar tabs={TABS} active={activeTab} onchange={switchTab} />
<div class="tab-content">
  <!-- tab panels -->
</div>
```

The `TabBar`'s own `margin-bottom: var(--space-6)` provides spacing between tabs and content. The `PageHeader`'s `margin-bottom: var(--space-6)` provides spacing between heading and tabs.

### 4.3 URL-Synced Tabs

For deep-linkable tab state, sync the active tab to URL search params:

```svelte
<script lang="ts">
  import { page } from '$app/state'
  import { goto } from '$app/navigation'

  let activeTab = $derived(page.url.searchParams.get('tab') ?? 'domains')

  function switchTab(tab: string) {
    goto(`/data/domains?tab=${tab}`, { replaceState: true })
  }
</script>
```

This preserves tab state across navigation and enables direct linking to a specific tab.

---

## 5. TabBar

**File:** `$lib/components/TabBar.svelte`
**Layer:** Component

Horizontal tab strip with keyboard navigation and ARIA roles. Used both standalone (as the sole navigation on a page) and inside TabbedHeader compositions.

### 5.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `tabs` | `TabItem[]` | (required) | Array of tab definitions |
| `active` | `string` | (required) | `value` of the currently active tab |
| `onchange` | `(value: string) => void` | (required) | Callback when user selects a different tab |
| `tab` | `Snippet<[{ tab: TabItem; active: boolean }]>` | `undefined` | Custom render snippet for tab content |

**Exported type:**

```typescript
export interface TabItem {
  value: string
  label: string
}
```

Import the type alongside the component:

```typescript
import TabBar, { type TabItem } from '$lib/components/TabBar.svelte'
```

### 5.2 Usage

**Basic usage:**

```svelte
<TabBar
  tabs={[
    { value: 'all', label: 'All' },
    { value: 'role', label: 'Roles' },
    { value: 'project', label: 'Projects' },
  ]}
  active={activeTab}
  onchange={(tab) => activeTab = tab}
/>
```

**Custom tab content (e.g., with count badges):**

```svelte
<TabBar tabs={SOURCE_TABS} active={activeTab} onchange={switchTab}>
  {#snippet tab({ tab: t, active })}
    {t.label}
    <span class="count-badge">{getCount(t.value)}</span>
  {/snippet}
</TabBar>
```

### 5.3 CSS Specification

```css
.tab-bar {
  display: flex;
  border-bottom: 1px solid var(--color-border);
  margin-bottom: var(--space-6);
}

.tab-bar-btn {
  padding: var(--space-3) var(--space-4);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--text-muted);
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
  white-space: nowrap;
}

.tab-bar-btn:hover {
  color: var(--text-secondary);
}

.tab-bar-btn:focus-visible {
  outline: 2px solid var(--color-border-focus);
  outline-offset: -2px;
  border-radius: var(--radius-sm);
}

.tab-bar-btn.active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
}
```

**Token usage:**
- Inactive tab: `--text-muted` color, `--font-medium` weight, `--text-sm` size
- Active tab: `--color-primary` color and border-bottom
- Hover: `--text-secondary` color
- Focus: `--color-border-focus` outline, `--radius-sm` rounding
- Container border: `1px solid var(--color-border)` on bottom
- Spacing: `--space-3` vertical padding, `--space-4` horizontal padding, `--space-6` bottom margin

### 5.4 Keyboard Navigation

The TabBar implements the [WAI-ARIA Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/):

| Key | Action |
|-----|--------|
| `ArrowRight` | Focus next tab (wraps to first) |
| `ArrowLeft` | Focus previous tab (wraps to last) |
| `Home` | Focus first tab |
| `End` | Focus last tab |
| `Enter` / `Space` | Activate focused tab (native button behavior) |

**ARIA attributes:**
- Container: `<div role="tablist">` (not `<nav role="tablist">` -- a `<nav>` with `role="tablist"` is semantically redundant and confusing to screen readers)
- Container: `aria-label="Tabs"`
- Each tab: `role="tab"`, `aria-selected={active === t.value}`
- Roving tabindex: active tab gets `tabindex="0"`, others get `tabindex="-1"`

### 5.5 Standalone vs. TabbedHeader

| Context | When to Use |
|---------|-------------|
| **Standalone TabBar** | When tabs are the primary navigation and each tab switches between different view components (e.g., `/data/domains` page switching between DomainsView and ArchetypesView) |
| **TabBar below PageHeader** | When the page has its own heading and the tabs control sub-sections within a single conceptual page |
| **Inline filter tabs** | When tab-like buttons filter a list within a panel (these are typically custom button rows styled to look like tabs, not the TabBar component) |

---

## 6. ViewToggle

**File:** `$lib/components/ViewToggle.svelte`
**Layer:** Component

Segmented toggle for switching between `list` and `board` view modes. Appears in `PageHeader` actions or standalone above a view region.

### 6.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `mode` | `'list' \| 'board'` | (required) | Currently active view mode |
| `onchange` | `(mode: 'list' \| 'board') => void` | (required) | Callback when user selects a different mode |

### 6.2 Usage

```svelte
<script lang="ts">
  import { ViewToggle } from '$lib/components'
  import { getViewMode, setViewMode } from '$lib/stores/viewMode.svelte'

  let viewMode = $state<'list' | 'board'>(getViewMode('sources'))

  function handleViewChange(mode: 'list' | 'board') {
    viewMode = mode
    setViewMode('sources', mode)
  }
</script>

<ViewToggle mode={viewMode} onchange={handleViewChange} />
```

### 6.3 CSS Specification

```css
.view-toggle {
  display: inline-flex;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.toggle-btn {
  padding: var(--space-1) var(--space-3);
  border: none;
  background: var(--color-surface);
  color: var(--text-muted);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.toggle-btn:not(:last-child) {
  border-right: 1px solid var(--color-border-strong);
}

.toggle-btn.active {
  background: var(--color-primary);
  color: var(--text-inverse);
}

.toggle-btn:hover:not(.active) {
  background: var(--color-ghost);
}
```

**Token usage:**
- Container: `--color-border-strong` border, `--radius-md` rounding
- Inactive: `--color-surface` bg, `--text-muted` text
- Active: `--color-primary` bg, `--text-inverse` text
- Hover (inactive): `--color-ghost` bg
- Sizing: `--space-1` vertical padding, `--space-3` horizontal padding, `--text-sm` font

### 6.4 Accessibility

The ViewToggle should use `role="radiogroup"` with `aria-label="View mode"` on the container, and each button should have `role="radio"` with `aria-checked` reflecting the active state:

```svelte
<div class="view-toggle" role="radiogroup" aria-label="View mode">
  <button
    class="toggle-btn"
    class:active={mode === 'list'}
    role="radio"
    aria-checked={mode === 'list'}
    onclick={() => onchange('list')}
  >List</button>
  <button
    class="toggle-btn"
    class:active={mode === 'board'}
    role="radio"
    aria-checked={mode === 'board'}
    onclick={() => onchange('board')}
  >Board</button>
</div>
```

### 6.5 Persistence via viewMode Store

View mode preferences are persisted to `localStorage` via the viewMode store:

**File:** `$lib/stores/viewMode.svelte.ts`

```typescript
import { browser } from '$app/environment'

const STORAGE_KEY_PREFIX = 'forge:viewMode:'

export type ViewMode = 'list' | 'board'

export function getViewMode(entity: string): ViewMode {
  if (!browser) return 'list'
  try {
    const stored = localStorage.getItem(`${STORAGE_KEY_PREFIX}${entity}`)
    return stored === 'board' ? 'board' : 'list'
  } catch {
    return 'list'
  }
}

export function setViewMode(entity: string, mode: ViewMode) {
  if (!browser) return
  try {
    localStorage.setItem(`${STORAGE_KEY_PREFIX}${entity}`, mode)
  } catch {
    // localStorage unavailable, silently ignore
  }
}
```

**Entity keys in use:** `'sources'`, `'bullets'`, `'perspectives'`, `'resumes'`, `'jds'`

**Behavior:**
- Defaults to `'list'` on SSR and when no stored preference exists
- Stores under the key `forge:viewMode:{entity}` in localStorage
- Each page/view is responsible for calling `getViewMode` on init and `setViewMode` on change

### 6.6 Integration with DualModePage

The `ViewToggle` is the control mechanism for the DualModePage pattern (Doc 2, Section 5.3):

```svelte
<PageHeader title="Sources" subtitle="Experience entries and their bullets">
  {#snippet actions()}
    <ViewToggle mode={viewMode} onchange={handleViewChange} />
  {/snippet}
</PageHeader>

{#if viewMode === 'board'}
  <PageWrapper overflow="hidden">
    <GenericKanban columns={COLUMNS} items={filteredItems} {onDrop}>
      {#snippet filterBar()}
        <SourceFilterBar bind:filters={boardFilters} onchange={() => {}} />
      {/snippet}
      {#snippet cardContent(item)}
        <SourceKanbanCard source={item} onclick={() => selectSource(item.id)} />
      {/snippet}
    </GenericKanban>
  </PageWrapper>
{:else}
  <!-- list/split-panel view -->
{/if}
```

---

## 7. SearchBar (ListSearchInput)

**File:** `$lib/components/ListSearchInput.svelte`
**Layer:** Atom

Compact text input for search/filter operations within list panels. Uses `$bindable()` for two-way value binding.

### 7.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `string` (bindable) | `''` | Current search text; supports `bind:value` |
| `placeholder` | `string` | `'Search...'` | Placeholder text |
| `...rest` | `Record<string, unknown>` | -- | Spread to the underlying `<input>` for native attributes |

### 7.2 Usage

```svelte
<script lang="ts">
  import { ListSearchInput } from '$lib/components'

  let search = $state('')
</script>

<ListSearchInput bind:value={search} placeholder="Filter sources..." />
```

### 7.3 CSS Specification

```css
.list-search-input {
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-family: inherit;
  color: var(--text-primary);
  background: var(--color-surface);
  transition: border-color 0.15s, box-shadow 0.15s;
}

.list-search-input::placeholder {
  color: var(--text-faint);
}

.list-search-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-subtle);
}
```

**Token usage:**
- Text: `--text-primary` color, `--text-sm` size
- Placeholder: `--text-faint`
- Border: `--color-border-strong` default, `--color-primary` on focus
- Focus ring: `--color-primary-subtle` (2px spread box-shadow)
- Background: `--color-surface`
- Padding: `--space-2` vertical, `--space-3` horizontal
- Radius: `--radius-md`

### 7.4 Do / Don't

| Do | Don't |
|----|-------|
| Use in split-panel list panes below `ListPanelHeader` | Don't use as a standalone page-level search (use a `.field-input` in a `.filter-bar`) |
| Bind value with `bind:value` for reactive filtering | Don't debounce inside the component; add debounce in the consuming page if needed |
| Spread native attributes via `...rest` for `aria-label`, `id`, etc. | Don't add icons or buttons inside the component; it is a plain input atom |

---

## 8. DropDownSelect (field-select)

**Layer:** Atom (global CSS class)

The DropDownSelect is implemented as a global CSS class (`.field-select`) in `base.css`, following ADR-003's pattern of using CSS classes for stateless styling. There is no `<DropDownSelect>` Svelte component.

### 8.1 CSS Specification (from base.css)

```css
.field-select {
  padding: 0.375rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-family: inherit;
  color: var(--text-primary);
  background: var(--color-surface);
  cursor: pointer;
}

.field-select:focus {
  outline: none;
  border-color: var(--color-border-focus);
  box-shadow: 0 0 0 2px var(--color-primary-subtle);
}
```

### 8.2 Usage

```svelte
<select class="field-select" bind:value={statusFilter} onchange={applyFilter}>
  <option value="">All statuses</option>
  <option value="draft">Draft</option>
  <option value="approved">Approved</option>
  <option value="rejected">Rejected</option>
</select>
```

### 8.3 Token Usage

Identical to `.field-input`:
- Text: `--text-primary` color, `--text-sm` size
- Border: `--color-border-strong` default, `--color-border-focus` on focus
- Focus ring: `--color-primary-subtle`
- Background: `--color-surface`
- Radius: `--radius-md`

### 8.4 Consistency with field-input

The `.field-select` and `.field-input` classes are designed to be visually identical at rest. This ensures they align properly when placed side-by-side in filter bars. Both share the same padding, font-size, border, border-radius, and focus treatment.

---

## 9. Filter Bar Composition

**Layer:** Page-level pattern (domain-specific filter bar components exist as Components)

A filter bar is a horizontal row of search inputs and select dropdowns that filter a data view. The pattern appears in two contexts: as domain-specific FilterBar components and as inline `.filter-bar` styled rows.

### 9.1 Domain-Specific Filter Bar Components

**Files:**
- `$lib/components/filters/BulletFilterBar.svelte`
- `$lib/components/filters/PerspectiveFilterBar.svelte`
- `$lib/components/filters/ResumeFilterBar.svelte`
- `$lib/components/filters/SourceFilterBar.svelte`
- `$lib/components/filters/JDFilterBar.svelte`

Each filter bar is a Svelte component that:
1. Accepts a `filters` object prop (bindable fields for each filter dimension)
2. Accepts an `onchange` callback
3. Loads its own filter options (e.g., fetching organizations or domains from the API)
4. Renders a `.filter-bar` row of `.field-select` and `.field-input` elements

### 9.2 Standard Filter Bar API Pattern

```typescript
let { filters, onchange }: {
  filters: {
    organization?: string
    source_type?: string
    search?: string
  }
  onchange: () => void
} = $props()
```

The `filters` object is bound from the parent. When a filter value changes, the component calls `onchange()` to notify the parent to re-derive filtered data.

### 9.3 Filter Bar CSS

The canonical `.filter-bar` styles (used across all filter bar components):

```css
.filter-bar {
  display: flex;
  gap: var(--space-2);
  flex-wrap: wrap;
  align-items: center;
}

.filter-bar .field-select,
.filter-bar .field-input {
  font-size: var(--text-sm);
  padding: var(--space-1) var(--space-2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  color: var(--text-primary);
}

.filter-bar .field-input {
  min-width: 150px;
}
```

**Note:** Filter bar inputs use slightly smaller padding (`--space-1` / `--space-2`) than standalone `.field-input` / `.field-select` (`0.375rem` / `0.625rem`) to keep the bar compact.

### 9.4 Filter Bar Composition Example

```svelte
<div class="filter-bar">
  <select class="field-select" bind:value={filters.organization} onchange={handleChange}>
    <option value="">All Organizations</option>
    {#each organizations as org}
      <option value={org.id}>{org.name}</option>
    {/each}
  </select>

  <select class="field-select" bind:value={filters.source_type} onchange={handleChange}>
    <option value="">All Types</option>
    <option value="role">Role</option>
    <option value="project">Project</option>
  </select>

  <input
    type="text"
    class="field-input"
    placeholder="Search sources..."
    bind:value={filters.search}
    oninput={handleChange}
  />
</div>
```

### 9.5 Filter Bar Placement

Filter bars appear in two locations:

**Inside GenericKanban (board view):** Passed as a `filterBar` snippet prop. Renders above the column headers:

```svelte
<GenericKanban columns={COLUMNS} items={filteredItems} {onDrop}>
  {#snippet filterBar()}
    <SourceFilterBar bind:filters={boardFilters} onchange={() => {}} />
  {/snippet}
  {#snippet cardContent(item)}
    <SourceKanbanCard source={item} />
  {/snippet}
</GenericKanban>
```

**Below the header in list/split-panel view:** Rendered directly in the page template between the header and the list content:

```svelte
<!-- Filter bar placement in list mode (inside SplitPanel list pane) -->
<SplitPanel {listWidth}>
  {#snippet list()}
    <ListPanelHeader title="Sources" onNew={create} />
    <div style="padding: var(--space-2) var(--space-3);">
      <SourceFilterBar bind:filters={listFilters} onchange={applyFilters} />
    </div>
    <div style="overflow-y: auto; flex: 1;">
      {#each filteredItems as item}
        <PaddedEntry onclick={() => select(item.id)}>{item.name}</PaddedEntry>
      {/each}
    </div>
  {/snippet}
  {#snippet detail()}
    <!-- detail content -->
  {/snippet}
</SplitPanel>
```

### 9.6 Known Divergences

**JDFilterBar API divergence:** `JDFilterBar` accepts individual filter props (`status`, `search`, `organization`) instead of the standard `filters` object pattern used by other filter bars. This should be migrated to the standard `{ filters, onchange }` pattern for consistency.

**BulletFilterBar 500-item fetch:** `BulletFilterBar` fetches up to 500 items to populate its dropdown options on mount. This is an anti-pattern for large data sets. A better approach would be to accept pre-loaded filter options via props or use a paginated/search-as-you-type combobox.

### 9.7 Filter Bar Token Reference

| Element | Token |
|---------|-------|
| Row gap | `var(--space-2)` |
| Input/select font | `var(--text-sm)` |
| Input/select padding | `var(--space-1) var(--space-2)` |
| Border | `1px solid var(--color-border)` |
| Radius | `var(--radius-sm)` |
| Background | `var(--color-surface)` |
| Text color | `var(--text-primary)` |
| Search input min-width | `150px` |

---

## 10. Sticky Header Behavior

Headers in Forge behave differently depending on the page type they appear in. The stickiness is a natural consequence of the page layout, not explicit `position: sticky` CSS.

### 10.1 FlowPage Headers

**Behavior:** Headers scroll with content. There is no sticky pinning.

**Why:** In a FlowPage, the entire content area scrolls as a single document. The header is part of the document flow and scrolls off-screen when the user scrolls down.

```
+--[ Sidebar ]--+--[ Content Area (overflow: auto) ]-------+
|               | <PageHeader />        <-- scrolls away    |
|               | <content>             <-- scrolls         |
|               | ...                                       |
+---------------+-------------------------------------------+
```

**Pages:** `/config/profile`, `/config/export`, `/data/domains`

**Implementation:** No `PageWrapper` needed. `PageHeader` has `margin-bottom: var(--space-6)` and sits in the normal document flow.

### 10.2 AppPage Headers (SplitPanel)

**Behavior:** Headers are naturally pinned above scroll regions. The list and detail panes scroll independently; the header sits above the scroll container.

**Why:** `PageWrapper` with `overflow="hidden"` constrains the page height to the viewport. The `SplitPanel` children fill the remaining height. Each pane manages its own overflow.

```
+--[ Sidebar ]--+--[ PageWrapper (overflow: hidden) ]------+
|               | <ListPanelHeader />   <-- pinned (above   |
|               |                           scroll)         |
|               | +--[List (overflow-y)]--+--[Detail]-----+ |
|               | | scrolling entries     | scrolling     | |
|               | |                       | content       | |
|               | +-----------------------+---------------+ |
+---------------+-------------------------------------------+
```

**Pages:** `/data/sources` (list mode), `/data/bullets` (list mode)

**Implementation:**

```svelte
<PageWrapper overflow="hidden">
  <SplitPanel {listWidth}>
    {#snippet list()}
      <ListPanelHeader title="Sources" onNew={create} />
      <!-- ListSearchInput for filtering -->
      <div style="overflow-y: auto; flex: 1;">
        <!-- entries -->
      </div>
    {/snippet}
    {#snippet detail()}
      <!-- detail content scrolls via .split-detail overflow-y: auto -->
    {/snippet}
  </SplitPanel>
</PageWrapper>
```

**Key CSS chain:**
- `PageWrapper`: `height: calc(100vh - var(--space-8) * 2)`, `overflow: hidden`, `display: flex; flex-direction: column`
- `SplitPanel`: `display: flex; flex: 1; min-height: 0`
- `.split-list`: `flex-shrink: 0; display: flex; flex-direction: column; overflow: hidden`
- `.split-detail`: `flex: 1; overflow-y: auto`

The `ListPanelHeader` sits at the top of `.split-list` (before the scrolling entry area) and never scrolls because the scroll container is a child `<div>` below it.

### 10.3 AppPage Headers (KanbanBoard)

**Behavior:** The page header (with ViewToggle) and any filter bar sit above the horizontally scrolling column area.

```
+--[ Sidebar ]--+--[ PageWrapper (overflow: hidden) ]------+
|               | <PageHeader + ViewToggle />  <-- pinned   |
|               | <FilterBar />                <-- pinned   |
|               | +--[Column]--+--[Column]--+--[Column]--+ |
|               | | scrolling  | scrolling  | scrolling   | |
|               | | cards      | cards      | cards       | |
|               | +------------+-----------+-------------+  |
+---------------+-------------------------------------------+
```

**Pages:** `/opportunities/organizations`, `/data/sources` (board mode), `/data/bullets` (board mode)

**Implementation:** The `GenericKanban` component manages the column layout. Filter bars are passed via the `filterBar` snippet prop and render above the column row.

### 10.4 Summary Table

| Page Type | Header Scrolls? | Mechanism |
|-----------|-----------------|-----------|
| FlowPage | Yes | Header is in normal document flow; entire content area scrolls |
| AppPage + SplitPanel | No (pinned) | Header sits above the scroll container in a flex column |
| AppPage + KanbanBoard | No (pinned) | Header sits above the kanban columns in a flex column |
| DualModePage | Depends on active mode | List mode may scroll; board mode is pinned |

---

## 11. Page Header Inline Pattern (Legacy)

Several pages construct their header inline rather than using the `PageHeader` component. This pattern uses the global `.page-title` and `.page-subtitle` classes from `base.css`:

```svelte
<!-- Legacy inline header (pre-component pattern) -->
<div class="page-header">
  <div>
    <h1 class="page-title">Content Atoms</h1>
    <p class="page-subtitle">Unified view of bullets and perspectives</p>
  </div>
  <ViewToggle mode={viewMode} onchange={handleViewChange} />
</div>
```

```css
/* From base.css */
.page-title {
  font-size: var(--text-2xl);
  font-weight: var(--font-bold);
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}

.page-subtitle {
  font-size: var(--text-base);
  color: var(--text-muted);
}
```

**Migration path:** These inline headers should be replaced with `<PageHeader>` as pages are touched. The `PageHeader` component provides the same visual output with a standardized API and the `actions` snippet for buttons and toggles.

---

## 12. Component Relationship Diagram

```
PageHeader
  |-- title (h1, --text-2xl)
  |-- subtitle (p, --text-sm, --text-muted)
  |-- actions (Snippet)
       |-- ViewToggle (list/board toggle)
       |-- <button class="btn btn-primary"> (action buttons)

ListPanelHeader
  |-- title (h2, --text-xl)
  |-- actions (Snippet)
  |-- new-btn (built-in, --color-primary)

TabBar
  |-- tabs[] (TabItem)
  |-- tab (optional Snippet for custom rendering)

Filter Bars (domain-specific components)
  |-- .filter-bar (flex row)
       |-- <select class="field-select"> (dropdowns)
       |-- <input class="field-input"> (search input)

ViewToggle
  |-- two buttons: List | Board
  |-- persisted via viewMode store
```

---

## 13. Cross-References

| Component | Defined Here | Also Referenced In |
|-----------|--------------|-------------------|
| PageHeader | Section 2 | Doc 2 (as content within ContentArea) |
| ListPanelHeader | Section 3 | Doc 5 (SplitPanel composition patterns) |
| TabBar | Section 5 | Doc 1 (type export example) |
| ViewToggle | Section 6 | Doc 2 (DualModePage pattern) |
| ListSearchInput | Section 7 | Doc 5 (list panel filter patterns) |
| .field-select | Section 8 | Doc 1 (global CSS classes in base.css) |
| .field-input | Section 8 | Doc 1 (global CSS classes in base.css) |
| Filter Bars | Section 9 | Doc 5 (GenericKanban filterBar snippet) |
| PageWrapper | Section 10 | Doc 2 (container specification) |
| SplitPanel | Section 10 | Doc 2 (view specification) |

---

## Acceptance Criteria

1. All headers (PageHeader, ListPanelHeader) use design tokens exclusively -- no hardcoded colors, font sizes, or spacing.
2. TabBar keyboard navigation wraps at boundaries: ArrowRight from last tab focuses first tab, ArrowLeft from first tab focuses last tab.
3. ViewToggle persists selected mode across page reloads via localStorage.
4. ViewToggle uses `role="radiogroup"` and `aria-label` for screen reader accessibility.
5. TabBar uses `<div role="tablist">` (not `<nav role="tablist">`).
6. Filter bars follow the standard `{ filters, onchange }` API pattern (JDFilterBar divergence documented as known gap).
7. ListSearchInput supports `bind:value` for two-way reactive filtering.
8. PageHeader renders a semantic `<header>` element with `<h1>` for correct heading hierarchy.
