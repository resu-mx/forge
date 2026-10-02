# ListPanelHeader — UI Component Spec

**Date:** 2026-04-04
**Status:** Draft
**Package:** `@forge/webui`
**Category:** polish
**Replaces:** 6+ inline `.list-header` + `.btn-new` implementations across split-panel pages

---

## Overview

**What:** A standardized header bar for the list panel in split-panel layouts, containing a title, an optional "new" action button, and an optional actions snippet for additional controls.

**Why:** Six pages independently implement the same `.list-header` flex row with an `<h2>` and a `.btn-new` button. The implementations have two classes of inconsistency:
1. **Button color mismatch:** Notes uses `--color-primary` for `.btn-new`, but Contacts uses `--color-info`. Both should use `--color-primary` (brand action color).
2. **Typography variance:** Notes uses `var(--text-xl)` for the heading, Contacts hardcodes `1rem` with `font-weight: 700` instead of using tokens.

A single component standardizes color, typography, and spacing.

**Pages affected:**

- `/data/notes/+page.svelte` -- `.list-header` + `.btn-new`
- `/data/contacts/+page.svelte` -- `.list-header` + `.btn-new`
- `/data/organizations/+page.svelte` -- `.list-header` + `.btn-new`
- `/opportunities/job-descriptions/+page.svelte` -- `.list-header` + `.btn-new`
- `/data/sources/SkillsView.svelte` -- `.list-header` + `.btn-new`
- `/data/sources/SourcesView.svelte` -- `.list-header` + `.btn-new`

---

## Component API

### Props

| Prop | Type | Default | Required | Description |
|------|------|---------|----------|-------------|
| `title` | `string` | -- | Yes | The heading text displayed in the left side of the header |
| `onNew` | `() => void` | `undefined` | No | Callback for the "new" button. If omitted, the button is not rendered. |
| `newLabel` | `string` | `'+ New'` | No | Label text for the "new" button. Override for specificity (e.g. `'+ New Contact'`). |
| `actions` | `Snippet` | `undefined` | No | Additional action buttons or controls rendered to the right of the title (before the "new" button). |

### Snippets (Slots)

| Snippet | Description | When to use |
|---------|-------------|-------------|
| `actions` | Extra controls (filters, toggles, dropdowns) placed in the header row | When the page needs more than just a title + new button (e.g. a filter dropdown) |

### Events / Callbacks

| Callback | Signature | Description |
|----------|-----------|-------------|
| `onNew` | `() => void` | Fired when the "new" button is clicked. Only relevant when `onNew` is provided. |

### TypeScript Types

```typescript
// Export from $lib/components/ListPanelHeader.svelte
interface ListPanelHeaderProps {
  title: string;
  onNew?: () => void;
  newLabel?: string;
  actions?: import('svelte').Snippet;
}
```

---

## Styling

### Concrete CSS (copy-paste)

```css
.list-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-5) var(--space-4);  /* 1.25rem 1rem */
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
  padding: 0.35rem var(--space-3);  /* match existing btn-new padding */
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

### Token Consumption

| Token | Usage | Fallback |
|-------|-------|----------|
| `--space-5` | Header vertical padding (`1.25rem`) | `1.25rem` |
| `--space-4` | Header horizontal padding (`1rem`) | `1rem` |
| `--space-2` | Gap between actions | `0.5rem` |
| `--space-3` | Button horizontal padding (`0.75rem`) | `0.75rem` |
| `--color-border` | Bottom border of the header | `#e5e7eb` |
| `--color-primary` | New button background | `#6c63ff` |
| `--color-primary-hover` | New button hover background | `#5a52e0` |
| `--text-inverse` | New button text color | `#ffffff` |
| `--text-primary` | Title text color | `#1a1a2e` |
| `--text-xl` | Title font size (`1.125rem`) | `1.125rem` |
| `--font-semibold` | Title font weight (`600`) | `600` |
| `--text-sm` | Button font size (`0.8rem`) | `0.8rem` |
| `--font-medium` | Button font weight (`500`) | `500` |
| `--radius-md` | Button border radius (`6px`) | `6px` |
| `--color-border-focus` | Button focus ring color | `#6c63ff` |

### Branding Strategy

The "new" button uses `--color-primary` (the brand action color), fixing the inconsistency where Contacts used `--color-info` (blue/informational). All visual properties come from tokens. Rebranding means changing token values only.

### Rendered Markup

```html
<div class="list-panel-header">
  <h2 class="list-panel-header__title">Notes</h2>
  <div class="list-panel-header__actions">
    <!-- actions snippet renders here (if provided) -->
    <button class="list-panel-header__new-btn" type="button">+ New</button>
  </div>
</div>
```

When `onNew` is not provided:

```html
<div class="list-panel-header">
  <h2 class="list-panel-header__title">Notes</h2>
  <!-- no actions div if neither actions snippet nor onNew is provided -->
</div>
```

---

## Behavior

### State Management

No internal state. The `title` and `newLabel` are reactive props rendered directly. The `onNew` callback is invoked on button click without any internal debouncing or state tracking.

### Accessibility

- The title is rendered as `<h2>` for document outline semantics within the panel.
- The "new" button has `type="button"` to prevent accidental form submissions.
- Focus ring via `:focus-visible` with `--color-border-focus` outline for keyboard users.
- No ARIA attributes needed beyond native `<h2>` and `<button>` semantics.

### Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| `onNew` not provided | The "new" button is not rendered. If `actions` snippet is also absent, the actions container is not rendered. |
| Very long `title` | Title truncates with ellipsis via `text-overflow: ellipsis` and `overflow: hidden` |
| Both `actions` snippet and `onNew` provided | Actions snippet renders first (left), then the "new" button renders after (right) |
| `newLabel` customized | Button text reflects the custom label (e.g. `'+ New Contact'` instead of `'+ New'`) |

---

## Examples

### Explicit Examples (DO THIS)

```svelte
<!-- Example 1: Basic usage — title + new button -->
<script lang="ts">
  import ListPanelHeader from '$lib/components/ListPanelHeader.svelte';
</script>

<ListPanelHeader title="Notes" onNew={startNew} />
```

```svelte
<!-- Example 2: Custom label and additional actions -->
<ListPanelHeader title="Contacts" onNew={startCreate} newLabel="+ New Contact">
  {#snippet actions()}
    <select bind:value={filter}>
      <option value="all">All</option>
      <option value="active">Active</option>
    </select>
  {/snippet}
</ListPanelHeader>
```

```svelte
<!-- Example 3: Read-only list with no new button -->
<ListPanelHeader title="Archived Items" />
```

### Implicit Examples (THIS IS THE PATTERN)

> Complete page composition showing PageWrapper + SplitPanel + ListPanelHeader together.

```svelte
<script lang="ts">
  import PageWrapper from '$lib/components/PageWrapper.svelte';
  import SplitPanel from '$lib/components/SplitPanel.svelte';
  import ListPanelHeader from '$lib/components/ListPanelHeader.svelte';
</script>

<PageWrapper>
  <SplitPanel>
    {#snippet list()}
      <ListPanelHeader title="Organizations" onNew={startCreate} newLabel="+ New Org" />
      <div class="filter-bar">
        <input type="text" placeholder="Search..." bind:value={searchQuery} />
      </div>
      <ul class="org-list">
        {#each filteredOrgs as org}
          <li>...</li>
        {/each}
      </ul>
    {/snippet}
    {#snippet detail()}
      <div class="editor">...</div>
    {/snippet}
  </SplitPanel>
</PageWrapper>
```

### Anti-Examples (DON'T DO THIS)

```svelte
<!-- WRONG: Inline header markup instead of using ListPanelHeader -->
<div class="list-header">
  <h2>Contacts</h2>
  <button class="btn-new" onclick={startCreate}>+ New Contact</button>
</div>
```

```css
/* WRONG: Per-page .btn-new with --color-info instead of --color-primary */
.btn-new {
  padding: 0.35rem 0.75rem;
  background: var(--color-info);    /* BUG: should be --color-primary */
  color: var(--text-inverse);
  border: none;
  border-radius: 0.375rem;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
}
```

```css
/* WRONG: Hardcoded typography values instead of tokens */
.panel-title {
  font-size: 1rem;        /* should be var(--text-xl) */
  font-weight: 700;       /* should be var(--font-semibold) */
}
```

---

## Goals

- [ ] Eliminate all 6+ inline `.list-header` + `.btn-new` CSS blocks
- [ ] Fix the `--color-info` vs `--color-primary` button color inconsistency (all buttons now use `--color-primary`)
- [ ] Standardize heading typography to `--text-xl` + `--font-semibold` across all list headers
- [ ] Support additional actions via the `actions` snippet without requiring component changes

## Non-Goals

- ListPanelHeader does not include the search/filter bar -- that varies too much between pages and sits below the header
- ListPanelHeader does not manage the "new" action's state (loading, disabled) -- the parent page owns that
- ListPanelHeader does not handle the list items or their scroll container

---

## Allowed / Not Allowed

### ALLOWED after adoption

- Using `<ListPanelHeader>` in all split-panel list headers
- Customizing `newLabel` for page-specific phrasing
- Adding extra controls via the `actions` snippet
- Omitting `onNew` for read-only list panels

### NOT ALLOWED after adoption

> These rules are enforced by the adoption grep test in CI.

- `.btn-new {` in any page-scoped `<style>` block
- `--color-info` used for a "create new" action button background (brand actions use `--color-primary`)
- Inline `.list-header` markup with an `<h2>` + action button instead of using the component

---

## Adoption Strategy

### Progressive Adoption

1. Component is created with tests
2. Reference page is migrated first (proves the component works)
3. Remaining pages migrate one at a time
4. Each migration is a separate commit (easy to revert)

### Reference Page

**Page:** `/data/notes` (`packages/webui/src/routes/data/notes/+page.svelte`)
**Why this page:** Simplest list header -- just a title and a `+ New` button. No additional actions or filters in the header row.

### Migration Order

| Order | Page | File | Complexity | Notes |
|-------|------|------|------------|-------|
| 1 | Notes | `src/routes/data/notes/+page.svelte` | Low | Reference: title + new button |
| 2 | Organizations | `src/routes/data/organizations/+page.svelte` | Low | Title + new button |
| 3 | Contacts | `src/routes/data/contacts/+page.svelte` | Low | Fixes `--color-info` to `--color-primary` |
| 4 | Job Descriptions | `src/routes/opportunities/job-descriptions/+page.svelte` | Medium | May have a view-mode toggle in header |
| 5 | Skills (SkillsView) | `src/routes/data/sources/SkillsView.svelte` | Low | Sub-view header |
| 6 | Sources (SourcesView) | `src/routes/data/sources/SourcesView.svelte` | Low | Sub-view header |

### Migration Checklist (per page)

- [ ] Import `ListPanelHeader` from `$lib/components`
- [ ] Replace inline `.list-header` div with `<ListPanelHeader>` passing title, onNew, newLabel
- [ ] Move any additional header controls into the `actions` snippet
- [ ] Remove page-scoped `.list-header`, `.btn-new`, and `.panel-title` CSS
- [ ] Verify the button color is now `--color-primary` (not `--color-info`)
- [ ] Verify visual match (before/after screenshot or manual check)
- [ ] Verify no regressions in interactive behavior
- [ ] Commit with message: `refactor(webui): migrate [page] to <ListPanelHeader>`

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
  name: 'ListPanelHeader',
  pattern: /\.btn-new\s*\{/,
  allowedIn: ['ListPanelHeader.svelte'],
  message: 'Use <ListPanelHeader> instead of inline .btn-new CSS',
}
```

### CLAUDE.md Rule

```markdown
- Split-panel list headers MUST use `<ListPanelHeader>`, not inline `.list-header` + `.btn-new` markup and CSS
- "Create new" action buttons MUST use `--color-primary`, not `--color-info`
```

---

## Testing

### Unit Tests

| Test | Description |
|------|-------------|
| Renders title | `<h2>` contains the provided `title` text |
| Renders new button when onNew provided | Button with `newLabel` text is visible and clickable |
| Hides new button when onNew omitted | No button element is rendered |
| Custom newLabel | Button text matches the custom label |
| Calls onNew on click | Clicking the button invokes the `onNew` callback |

### Component Tests

| Test | Description |
|------|-------------|
| Button uses primary color | Button has `background: var(--color-primary)`, not `--color-info` |
| Title uses design tokens | `<h2>` has `font-size: var(--text-xl)` and `font-weight: var(--font-semibold)` |
| Actions snippet renders | Extra controls passed via `actions` snippet appear in the header row |
| Long title truncates | Title with 100+ characters shows ellipsis and does not wrap |

### Adoption Enforcement Tests

| Test | Description |
|------|-------------|
| Anti-pattern grep | Fails if `.btn-new {` found outside `ListPanelHeader.svelte` |
| Migration count | Tracks remaining pages to migrate (informational, not blocking) |

---

## Acceptance Criteria

- [ ] Component renders correctly with all prop combinations
- [ ] Reference page (notes) migrated with no visual diff
- [ ] All design tokens used (no hardcoded values)
- [ ] `--color-primary` used for new button (not `--color-info`)
- [ ] Grep test added and passing
- [ ] CLAUDE.md rule added
- [ ] Component exported from `$lib/components/`

## Failure Criteria

- Component uses `--color-info` for the "new" button (perpetuating the existing bug)
- Hardcoded color/spacing values instead of token references
- Migration breaks interactive behavior on any page
- Component API requires page-specific props (should be generic)
