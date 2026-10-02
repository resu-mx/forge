# TabBar -- UI Component Spec

**Date:** 2026-04-04
**Status:** Draft
**Package:** `@forge/webui`
**Category:** structural
**Replaces:** 3 hand-rolled tab bar implementations (domains `+page.svelte`, SourcesView `.filter-tab`, debug layout nav cards)

---

## Overview

**What:** A shared horizontal tab bar component with an underline-active-tab indicator pattern, rendering a row of tab buttons with a bottom border highlight on the active tab.

**Why:** Three pages implement their own tab bar with inconsistent class names (`.tab-btn`, `.filter-tab`), varying padding values, and different active-state styling. The domains page uses `--color-primary` for the underline, SourcesView uses a different approach with tab counts, and the debug layout uses navigation cards instead of tabs. Consolidating into one component eliminates these inconsistencies and provides a single place to update tab styling.

**Pages affected:**
- `/data/domains/+page.svelte` (`.tab-btn` pattern)
- `/data/sources/SourcesView.svelte` (`.filter-tab` pattern with counts)
- `/config/debug/+page.svelte` (navigation cards acting as tabs -- candidate for migration to TabBar + router)

---

## Component API

### Props

| Prop | Type | Default | Required | Description |
|------|------|---------|----------|-------------|
| `tabs` | `TabItem[]` | -- | Yes | Array of tab definitions with `value` and `label` |
| `active` | `string` | -- | Yes | The `value` of the currently active tab |
| `onchange` | `(value: string) => void` | -- | Yes | Callback fired when a tab is clicked |

### Snippets (Slots)

| Snippet | Description | When to use |
|---------|-------------|-------------|
| `tab` | Custom tab content renderer, receives `{ tab: TabItem, active: boolean }` | When tabs need badges, counts, or icons beyond a plain label |

### Events / Callbacks

| Callback | Signature | Description |
|----------|-----------|-------------|
| `onchange` | `(value: string) => void` | Called with the tab's `value` when the user clicks a non-active tab |

### TypeScript Types

```typescript
// Export from $lib/components/TabBar.svelte or $lib/types.ts
interface TabItem {
  value: string;
  label: string;
}

interface TabBarProps {
  tabs: TabItem[];
  active: string;
  onchange: (value: string) => void;
  tab?: import('svelte').Snippet<[{ tab: TabItem; active: boolean }]>;
}
```

---

## Styling

### Concrete CSS (copy-paste)

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

### Token Consumption

| Token | Usage | Fallback |
|-------|-------|----------|
| `--color-border` | Bottom border of the tab bar | `#e5e7eb` |
| `--color-primary` | Active tab text color and underline indicator | `#6c63ff` |
| `--color-border-focus` | Focus-visible outline color | `#6c63ff` |
| `--text-sm` | Tab label font size (12.8px) | `0.8rem` |
| `--font-medium` | Tab label font weight | `500` |
| `--text-muted` | Inactive tab text color | `#6b7280` |
| `--text-secondary` | Hovered tab text color | `#374151` |
| `--space-3` | Vertical padding (12px) | `0.75rem` |
| `--space-4` | Horizontal padding (16px) | `1rem` |
| `--space-6` | Bottom margin below the tab bar | `1.5rem` |
| `--radius-sm` | Focus-visible border radius | `3px` |

### Branding Strategy

All visual properties come from design tokens. The active indicator color is controlled entirely by `--color-primary`. Spacing, typography, and inactive colors use their respective tokens. Rebranding means changing token values only -- no component CSS changes needed.

### Rendered Markup

```html
<nav class="tab-bar" role="tablist" aria-label="Tabs">
  <!-- repeated for each tab -->
  <button
    class="tab-bar-btn active"
    role="tab"
    aria-selected="true"
    tabindex="0"
  >
    Domains
  </button>
  <button
    class="tab-bar-btn"
    role="tab"
    aria-selected="false"
    tabindex="-1"
  >
    Archetypes
  </button>
</nav>
```

---

## Behavior

### State Management

TabBar is a controlled component. The active tab is determined entirely by the `active` prop. When a user clicks a tab, the component calls `onchange(tab.value)` -- the parent is responsible for updating `active`. No internal state is managed.

### Accessibility

- Uses `role="tablist"` on the container `<nav>` element
- Each tab button uses `role="tab"` with `aria-selected="true|false"`
- Active tab has `tabindex="0"`, inactive tabs have `tabindex="-1"`
- Arrow key navigation: Left/Right arrows move focus between tabs
- Home/End keys move focus to first/last tab
- When a tab receives focus via keyboard, it does not auto-activate (user must press Enter/Space)
- `aria-label="Tabs"` on the container provides a landmark label

### Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| Single tab | Renders one tab, always active; still useful for visual consistency |
| `active` value not in `tabs` array | No tab is highlighted; no crash |
| Tab clicked that is already active | `onchange` is not called (no-op) |
| Many tabs overflow container | Tabs scroll horizontally with `overflow-x: auto` on the container |
| Tab with very long label | Label does not wrap (`white-space: nowrap`); truncates via container scroll |

---

## Examples

### Explicit Examples (DO THIS)

```svelte
<!-- Example 1: Basic usage -- two tabs -->
<script lang="ts">
  import TabBar from '$lib/components/TabBar.svelte'

  let activeTab = $state('domains')
</script>

<TabBar
  tabs={[
    { value: 'domains', label: 'Domains' },
    { value: 'archetypes', label: 'Archetypes' },
  ]}
  active={activeTab}
  onchange={(v) => activeTab = v}
/>
```

```svelte
<!-- Example 2: With custom tab content (counts) -->
<TabBar
  tabs={SOURCE_TABS}
  active={activeTab}
  onchange={(v) => activeTab = v}
>
  {#snippet tab({ tab, active })}
    {tab.label}
    <span class="tab-count">{getCount(tab.value)}</span>
  {/snippet}
</TabBar>
```

### Implicit Examples (THIS IS THE PATTERN)

```svelte
<!-- Typical page composition showing TabBar with content switching -->
<script lang="ts">
  import { page } from '$app/state'
  import { goto } from '$app/navigation'
  import TabBar from '$lib/components/TabBar.svelte'
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
  <TabBar tabs={TABS} active={activeTab} onchange={switchTab} />
  <div class="tab-content">
    {#if activeTab === 'archetypes'}
      <ArchetypesView />
    {:else}
      <DomainsView />
    {/if}
  </div>
</div>
```

### Anti-Examples (DON'T DO THIS)

```svelte
<!-- WRONG: Hand-rolled tab buttons with per-page styles -->
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
```

```css
/* WRONG: Page-scoped .tab-btn styles that duplicate the shared component */
.tab-btn {
  padding: 0.75rem 1.25rem;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  font-size: var(--text-sm);
  color: var(--text-muted);
  cursor: pointer;
}
```

---

## Goals

- [x] Eliminate all per-page `.tab-btn` and `.filter-tab` style blocks
- [x] Guarantee consistent underline-active-tab visual pattern across all tabbed views
- [x] Provide keyboard navigation (arrow keys, Home/End) and ARIA roles
- [x] Support custom tab content via snippet for badges and counts
- [x] Zero hardcoded values -- 100% token-driven

## Non-Goals

- Vertical tab bars (not used anywhere in the app)
- Tab panels with automatic content switching (parent manages content)
- Closable/draggable tabs (not a browser-tab pattern)
- Routing integration (parent handles `goto` / URL params)

---

## Allowed / Not Allowed

### ALLOWED after adoption

- Using `<TabBar>` in all pages that display a horizontal tab-switching UI
- Extending via `tab` snippet for custom tab content (counts, badges, icons)
- Using `onchange` to drive URL-based or state-based tab switching

### NOT ALLOWED after adoption

> These rules are enforced by the adoption grep test in CI.

- `.tab-btn {` in any page-scoped `<style>` block (use `<TabBar>` instead)
- `.filter-tab {` in any page-scoped `<style>` block (use `<TabBar>` instead)
- Hand-rolled `{#each}` loops producing tab buttons with inline active-state styling

---

## Adoption Strategy

### Progressive Adoption

1. Component is created with tests
2. Reference page is migrated first (proves the component works)
3. Remaining pages migrate one at a time
4. Each migration is a separate commit (easy to revert)

### Reference Page

**Page:** `/data/domains` (`packages/webui/src/routes/data/domains/+page.svelte`)
**Why this page:** Simplest tab bar usage -- two tabs, URL-based switching, no custom tab content. Matches the component API exactly.

### Migration Order

| Order | Page | File | Complexity | Notes |
|-------|------|------|------------|-------|
| 1 | /data/domains | `routes/data/domains/+page.svelte` | Low | Reference implementation; 2 tabs, URL-based |
| 2 | /data/sources | `routes/data/sources/SourcesView.svelte` | Medium | 6 tabs with counts via snippet; uses `.filter-tab` class |
| 3 | /config/debug | `routes/config/debug/+page.svelte` | Medium | Currently uses card-based nav; convert to TabBar + nested routes or content switch |

### Migration Checklist (per page)

- [ ] Import `TabBar` from `$lib/components`
- [ ] Replace inline tab markup (`{#each}` over tabs with buttons) with `<TabBar>` usage
- [ ] Remove page-scoped `.tab-btn`, `.tab-bar`, or `.filter-tab` CSS rules
- [ ] Verify visual match (before/after screenshot or manual check)
- [ ] Verify keyboard navigation works (arrow keys, Enter/Space)
- [ ] Verify no regressions in tab switching behavior
- [ ] Commit with message: `refactor(webui): migrate [page] to <TabBar>`

### Coexistence Rules

During migration, old and new implementations coexist. Rules:

- New pages MUST use the shared component (enforced by CLAUDE.md rule)
- Existing pages keep their inline CSS until migrated (no rush)
- The grep test tracks remaining violations -- count decreases toward zero
- Component API is frozen after reference page ships (no breaking changes during migration)

---

## Adoption Enforcement

### CI Grep Test

```typescript
// In packages/webui/src/__tests__/component-adoption.test.ts
{
  name: 'TabBar',
  pattern: /\.tab-btn\s*\{/,
  allowedIn: ['TabBar.svelte'],
  message: 'Use <TabBar> instead of page-scoped .tab-btn styles',
}
```

### CLAUDE.md Rule

```markdown
- Tab navigation MUST use `<TabBar>`, not hand-rolled `.tab-btn` or `.filter-tab` CSS
```

---

## Testing

### Unit Tests

| Test | Description |
|------|-------------|
| Renders all tabs | `<TabBar tabs={[...]} active="a" onchange={fn} />` renders a button for each tab |
| Highlights active tab | The button matching `active` has `.active` class and `aria-selected="true"` |
| Calls onchange on click | Clicking an inactive tab calls `onchange` with that tab's `value` |
| Does not call onchange for active tab | Clicking the already-active tab does not trigger `onchange` |
| Renders custom tab content via snippet | When `tab` snippet is provided, it renders custom content instead of plain label |
| ARIA roles | Container has `role="tablist"`, buttons have `role="tab"` |

### Component Tests

| Test | Description |
|------|-------------|
| Keyboard arrow navigation | Left/Right arrows move focus between tabs |
| Home/End keys | Home moves to first tab, End moves to last |
| Visual regression | Screenshot comparison of TabBar with 2-6 tabs, active states |

### Adoption Enforcement Tests

| Test | Description |
|------|-------------|
| Anti-pattern grep | Fails if `.tab-btn {` found outside `TabBar.svelte` |
| Migration count | Tracks remaining pages to migrate (informational, not blocking) |

---

## Acceptance Criteria

- [ ] Component renders correctly with all prop combinations
- [ ] Reference page (`/data/domains`) migrated with no visual diff
- [ ] All design tokens used (no hardcoded values)
- [ ] Keyboard navigation works (arrow keys, Home/End)
- [ ] ARIA attributes present and correct
- [ ] Grep test added and passing
- [ ] CLAUDE.md rule added
- [ ] Component exported from `$lib/components/`

## Failure Criteria

- Component introduces a new tab visual variant instead of matching the canonical underline pattern from domains
- Hardcoded color/spacing values instead of token references
- Migration breaks tab switching behavior on any page
- Component API requires page-specific props (should be generic)
- Active state indicator differs from the existing `--color-primary` underline pattern
