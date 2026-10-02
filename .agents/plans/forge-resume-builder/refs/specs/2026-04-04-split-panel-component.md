# SplitPanel — UI Component Spec

**Date:** 2026-04-04
**Status:** Draft
**Package:** `@forge/webui`
**Category:** structural
**Replaces:** 7 per-page split-panel CSS implementations (`.split-panel`, `.list-panel`, `.editor-panel` patterns)

---

## Overview

**What:** A two-column layout component that renders a fixed-width list panel on the left and a flexible detail panel on the right, separated by a border.

**Why:** Seven pages independently implement the same split-panel layout with slight variations in class names (`.list-panel` vs `.split-list`), widths (all 320px), and overflow handling. The duplication means inconsistencies creep in -- some panels have `min-width`, some do not; some set `overflow: hidden`, others `overflow-y: auto`. A single component standardizes the pattern.

**Pages affected:**

- `/data/notes/+page.svelte` -- `.list-panel` / `.editor-panel`
- `/data/contacts/+page.svelte` -- `.split-panel > .list-panel` / `.editor-panel`
- `/data/organizations/+page.svelte` -- `.list-panel` / `.editor-panel`
- `/opportunities/job-descriptions/+page.svelte` -- `.list-panel` / `.editor-panel` (list mode)
- `/data/sources/SourcesView.svelte` -- `.list-panel` / `.detail-panel`
- `/data/sources/SkillsView.svelte` -- `.list-panel` / `.detail-panel`
- `/resumes/+page.svelte` -- split layout in list mode

---

## Component API

### Props

| Prop | Type | Default | Required | Description |
|------|------|---------|----------|-------------|
| `listWidth` | `number` | `320` | No | Width of the left list panel in pixels. Applied as `width` with `flex-shrink: 0`. |

### Snippets (Slots)

| Snippet | Description | When to use |
|---------|-------------|-------------|
| `list` | Content for the left (list) panel | Always -- renders the header, search, and item list |
| `detail` | Content for the right (detail/editor) panel | Always -- renders the selected item editor or empty state |

### Events / Callbacks

None. SplitPanel is a pure layout component.

### TypeScript Types

```typescript
// Export from $lib/components/SplitPanel.svelte
interface SplitPanelProps {
  listWidth?: number;
  list: import('svelte').Snippet;
  detail: import('svelte').Snippet;
}
```

---

## Styling

### Concrete CSS (copy-paste)

```css
.split-panel {
  display: flex;
  flex: 1;
  min-height: 0; /* allow flex children to shrink below content size */
}

.split-list {
  width: var(--_list-width, 320px);
  flex-shrink: 0;
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--color-surface);
}

.split-detail {
  flex: 1;
  overflow-y: auto;
  min-width: 0; /* prevent flex blowout from long content */
  background: var(--color-surface);
}
```

### Token Consumption

| Token | Usage | Fallback |
|-------|-------|----------|
| `--color-border` | Divider between list and detail panels | `#e5e7eb` |
| `--color-surface` | Background of both panels | `#ffffff` |

### Branding Strategy

SplitPanel uses only two tokens: `--color-border` for the divider line and `--color-surface` for the panel backgrounds. Rebranding the border or surface color automatically updates all split-panel pages. No hardcoded colors.

### Rendered Markup

```html
<div class="split-panel">
  <div class="split-list" style="width: 320px">
    <!-- list snippet renders here -->
  </div>
  <div class="split-detail">
    <!-- detail snippet renders here -->
  </div>
</div>
```

---

## Behavior

### State Management

No internal state. `listWidth` is passed as a prop and applied as an inline style variable. The component does not track which panel is active, selected items, or scroll position -- all of that is the caller's responsibility.

### Accessibility

- No keyboard navigation requirements at the SplitPanel level -- it is a layout container, not an interactive widget.
- No ARIA attributes needed. The two panels are generic `<div>` elements. Pages using landmarks (`<nav>`, `<main>`) should place them inside the snippets.
- Focus management is the caller's responsibility (e.g. focusing the editor when an item is selected).

### Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| Detail panel content overflows | `.split-detail` has `overflow-y: auto`, so it scrolls independently |
| List panel content overflows | `.split-list` has `overflow: hidden`; the list snippet is expected to have its own scrolling container (e.g. `<ul>` with `overflow-y: auto; flex: 1`) |
| Window narrower than `listWidth + 200px` | List panel retains its width via `flex-shrink: 0`; detail panel shrinks. Extremely narrow viewports are not a target (desktop-first app). |
| Custom `listWidth` provided | Applied via inline style; CSS variable `--_list-width` is set for the list panel width |
| Detail snippet empty (no item selected) | Detail panel renders the empty-state content from the caller's snippet |

---

## Examples

### Explicit Examples (DO THIS)

```svelte
<!-- Example 1: Basic split panel with default 320px list width -->
<script lang="ts">
  import SplitPanel from '$lib/components/SplitPanel.svelte';
</script>

<SplitPanel>
  {#snippet list()}
    <div class="list-header">
      <h2>Items</h2>
    </div>
    <ul class="item-list">
      <li>Item 1</li>
      <li>Item 2</li>
    </ul>
  {/snippet}
  {#snippet detail()}
    <div class="editor">
      <p>Select an item to edit</p>
    </div>
  {/snippet}
</SplitPanel>
```

```svelte
<!-- Example 2: Custom list width for a wider sidebar -->
<SplitPanel listWidth={400}>
  {#snippet list()}
    <!-- wider list with more info per card -->
  {/snippet}
  {#snippet detail()}
    <!-- detail editor -->
  {/snippet}
</SplitPanel>
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
      <ListPanelHeader title="Contacts" onNew={startCreate} newLabel="+ New Contact" />
      <div class="list-filters">
        <input type="text" placeholder="Search..." bind:value={searchText} />
      </div>
      <div class="card-list">
        {#each filteredContacts as contact}
          <ContactCard {contact} />
        {/each}
      </div>
    {/snippet}
    {#snippet detail()}
      {#if selectedContact}
        <ContactEditor contact={selectedContact} />
      {:else}
        <EmptyState title="No contact selected" />
      {/if}
    {/snippet}
  </SplitPanel>
</PageWrapper>
```

### Anti-Examples (DON'T DO THIS)

```svelte
<!-- WRONG: Inline split-panel CSS instead of using SplitPanel component -->
<div class="my-page">
  <div class="list-panel">...</div>
  <div class="editor-panel">...</div>
</div>
```

```css
/* WRONG: Per-page split-panel CSS that duplicates the shared layout */
.list-panel {
  width: 320px;
  flex-shrink: 0;
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.editor-panel {
  flex: 1;
  overflow-y: auto;
}
```

---

## Goals

- [ ] Eliminate all 7 inline `.list-panel` / `.editor-panel` CSS blocks
- [ ] Standardize list panel width, border, overflow, and background across all pages
- [ ] Provide snippet-based composition so pages control their own content without modifying the layout

## Non-Goals

- SplitPanel does not handle responsive/mobile layouts (this is a desktop-first app)
- SplitPanel does not include drag-to-resize functionality
- SplitPanel does not set scroll position or manage focus -- those are caller responsibilities

---

## Allowed / Not Allowed

### ALLOWED after adoption

- Using `<SplitPanel>` in all pages with a list + detail two-column layout
- Customizing `listWidth` when the default 320px is not appropriate
- Putting any content inside the `list` and `detail` snippets (headers, filters, empty states, editors)

### NOT ALLOWED after adoption

> These rules are enforced by the adoption grep test in CI.

- `.list-panel {` in any page-scoped `<style>` block (indicates an inline split-panel layout)
- Duplicating `width: 320px; flex-shrink: 0; border-right:` in page CSS
- Using `.editor-panel` or `.detail-panel` class with `flex: 1; overflow-y: auto` in page CSS when SplitPanel provides it

---

## Adoption Strategy

### Progressive Adoption

1. Component is created with tests
2. Reference page is migrated first (proves the component works)
3. Remaining pages migrate one at a time
4. Each migration is a separate commit (easy to revert)

### Reference Page

**Page:** `/data/notes` (`packages/webui/src/routes/data/notes/+page.svelte`)
**Why this page:** Most straightforward split-panel implementation. No conditional layout modes, no nested sub-views. The `.list-panel` and `.editor-panel` pattern maps 1:1 to the `list` and `detail` snippets.

### Migration Order

| Order | Page | File | Complexity | Notes |
|-------|------|------|------------|-------|
| 1 | Notes | `src/routes/data/notes/+page.svelte` | Low | Reference implementation |
| 2 | Contacts | `src/routes/data/contacts/+page.svelte` | Low | Already uses `.split-panel` wrapper class name |
| 3 | Organizations | `src/routes/data/organizations/+page.svelte` | Low | Standard split-panel |
| 4 | Job Descriptions | `src/routes/opportunities/job-descriptions/+page.svelte` | Medium | Has list mode / detail mode toggle; SplitPanel used only in list mode |
| 5 | Sources (SkillsView) | `src/routes/data/sources/SkillsView.svelte` | Low | Sub-view inside sources page |
| 6 | Sources (SourcesView) | `src/routes/data/sources/SourcesView.svelte` | Low | Sub-view inside sources page |
| 7 | Resumes | `src/routes/resumes/+page.svelte` | Medium | Has list mode that uses split layout |

### Migration Checklist (per page)

- [ ] Import `SplitPanel` from `$lib/components`
- [ ] Replace the outer split-panel div + list/detail divs with `<SplitPanel>`
- [ ] Move list content into `{#snippet list()}` block
- [ ] Move detail content into `{#snippet detail()}` block
- [ ] Remove page-scoped `.list-panel`, `.editor-panel`, `.split-panel` CSS
- [ ] Verify visual match (before/after screenshot or manual check)
- [ ] Verify no regressions in interactive behavior
- [ ] Commit with message: `refactor(webui): migrate [page] to <SplitPanel>`

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
  name: 'SplitPanel',
  pattern: /\.list-panel\s*\{/,
  allowedIn: ['SplitPanel.svelte'],
  message: 'Use <SplitPanel> instead of inline .list-panel CSS',
}
```

### CLAUDE.md Rule

```markdown
- Two-column list+detail layouts MUST use `<SplitPanel>`, not inline `.list-panel` / `.editor-panel` CSS
```

---

## Testing

### Unit Tests

| Test | Description |
|------|-------------|
| Renders list snippet | Content passed to `list` snippet appears in the left panel |
| Renders detail snippet | Content passed to `detail` snippet appears in the right panel |
| Default list width | Left panel has `width: 320px` when no `listWidth` prop is passed |
| Custom list width | Left panel width matches the `listWidth` prop value |

### Component Tests

| Test | Description |
|------|-------------|
| Border divider present | A `border-right` exists on the list panel |
| List panel does not scroll | List panel has `overflow: hidden` |
| Detail panel scrolls | Detail panel has `overflow-y: auto` |
| Flex layout | Wrapper has `display: flex` and detail panel has `flex: 1` |

### Adoption Enforcement Tests

| Test | Description |
|------|-------------|
| Anti-pattern grep | Fails if `.list-panel {` found outside `SplitPanel.svelte` |
| Migration count | Tracks remaining pages to migrate (informational, not blocking) |

---

## Acceptance Criteria

- [ ] Component renders correctly with all prop combinations
- [ ] Reference page (notes) migrated with no visual diff
- [ ] All design tokens used (no hardcoded values)
- [ ] Grep test added and passing
- [ ] CLAUDE.md rule added
- [ ] Component exported from `$lib/components/`

## Failure Criteria

- Component introduces a different split-panel visual variant instead of matching the canonical pattern
- Hardcoded color/spacing values instead of token references
- Migration breaks interactive behavior on any page
- Component API requires page-specific props (should be generic)
