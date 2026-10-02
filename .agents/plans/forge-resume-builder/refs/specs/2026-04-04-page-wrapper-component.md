# PageWrapper — UI Component Spec

**Date:** 2026-04-04
**Status:** Draft
**Package:** `@forge/webui`
**Category:** structural
**Replaces:** 8 per-page viewport-escape CSS blocks (`height: calc(100vh - 4rem); margin: -2rem`)

---

## Overview

**What:** A layout wrapper component that escapes the padded `+layout.svelte` content area so a page can fill the full viewport below the top nav.

**Why:** Eight pages independently re-implement the same `height: calc(100vh - 4rem); margin: -2rem` escape hatch. The magic values are fragile -- if the layout padding changes, every page breaks individually. A single component encapsulates this math and gives us one place to update.

**Pages affected:**

- `/data/notes/+page.svelte`
- `/data/contacts/+page.svelte`
- `/data/organizations/+page.svelte`
- `/opportunities/job-descriptions/+page.svelte`
- `/data/skills/+page.svelte`
- `/data/bullets/+page.svelte`
- `/resumes/+page.svelte`
- `/data/sources/_old_page.svelte.bak` (legacy, low priority)

---

## Component API

### Props

| Prop | Type | Default | Required | Description |
|------|------|---------|----------|-------------|
| `overflow` | `'auto' \| 'hidden' \| 'visible'` | `'hidden'` | No | Controls `overflow` on the wrapper div. Most split-panel pages want `hidden` so each child panel scrolls independently. Single-panel pages may want `auto`. |

### Snippets (Slots)

| Snippet | Description | When to use |
|---------|-------------|-------------|
| `children` | Default slot for all page content | Always -- the entire page body goes inside |

### Events / Callbacks

None. This is a pure layout component with no interactive behavior.

### TypeScript Types

```typescript
// Export from $lib/components/PageWrapper.svelte
interface PageWrapperProps {
  overflow?: 'auto' | 'hidden' | 'visible';
  children: import('svelte').Snippet;
}
```

---

## Styling

### Concrete CSS (copy-paste)

```css
.page-wrapper {
  height: calc(100vh - var(--space-8) * 2);  /* 100vh - 4rem (layout padding top+bottom) */
  margin: calc(-1 * var(--space-8));          /* escape the layout padding */
  display: flex;
  flex-direction: column;
}

.page-wrapper--overflow-auto    { overflow: auto; }
.page-wrapper--overflow-hidden  { overflow: hidden; }
.page-wrapper--overflow-visible { overflow: visible; }
```

### Token Consumption

| Token | Usage | Fallback |
|-------|-------|----------|
| `--space-8` | Height calc offset (`2rem * 2 = 4rem`) and negative margin escape | `2rem` |

### Branding Strategy

PageWrapper is almost invisible -- it only uses `--space-8` for the layout padding offset. If layout padding changes (e.g. from `2rem` to `1.5rem`), only this one component needs an update instead of 8 pages. No colors, no typography -- purely structural.

### Rendered Markup

```html
<div class="page-wrapper page-wrapper--overflow-hidden">
  <!-- children snippet renders here -->
</div>
```

---

## Behavior

### State Management

No internal state. The `overflow` prop is passed through to a CSS modifier class. This is a stateless presentational wrapper.

### Accessibility

- No keyboard navigation requirements -- this is a structural container with no interactive elements.
- No ARIA attributes needed. The component is a generic `<div>` with no semantic role.
- Focus management is not applicable.

### Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| Layout padding changes from `2rem` to a different value | Update `--space-8` references in PageWrapper; all 8 pages inherit the fix automatically |
| Page has a loading state that appears before the split panel | Loading spinner renders inside PageWrapper and is centered via the page's own CSS (PageWrapper provides the height) |
| `overflow='visible'` used when page needs visible overflow (e.g. dropdowns) | Wrapper sets `overflow: visible`, child content is responsible for its own scroll containers |

---

## Examples

### Explicit Examples (DO THIS)

```svelte
<!-- Example 1: Basic usage — split-panel page with hidden overflow (default) -->
<script lang="ts">
  import PageWrapper from '$lib/components/PageWrapper.svelte';
</script>

<PageWrapper>
  <div class="split-panel">
    <div class="list-panel">...</div>
    <div class="detail-panel">...</div>
  </div>
</PageWrapper>
```

```svelte
<!-- Example 2: Single-panel page needing its own scroll -->
<script lang="ts">
  import PageWrapper from '$lib/components/PageWrapper.svelte';
</script>

<PageWrapper overflow="auto">
  <div class="single-panel-content">
    <!-- Long scrollable content -->
  </div>
</PageWrapper>
```

### Implicit Examples (THIS IS THE PATTERN)

> These examples show how PageWrapper composes with SplitPanel and ListPanelHeader
> to form a complete page.

```svelte
<!-- Typical full page composition -->
<script lang="ts">
  import PageWrapper from '$lib/components/PageWrapper.svelte';
  import SplitPanel from '$lib/components/SplitPanel.svelte';
  import ListPanelHeader from '$lib/components/ListPanelHeader.svelte';
</script>

<PageWrapper>
  <SplitPanel>
    {#snippet list()}
      <ListPanelHeader title="Notes" onNew={startNew} />
      <div class="filter-bar">...</div>
      <ul class="note-list">...</ul>
    {/snippet}
    {#snippet detail()}
      <div class="editor-panel">...</div>
    {/snippet}
  </SplitPanel>
</PageWrapper>
```

### Anti-Examples (DON'T DO THIS)

```svelte
<!-- WRONG: Inline viewport-escape CSS instead of using PageWrapper -->
<div class="my-page">
  ...
</div>

<style>
  .my-page {
    height: calc(100vh - 4rem);
    margin: -2rem;
    display: flex;
    flex-direction: column;
  }
</style>
```

```css
/* WRONG: Hardcoded magic values duplicated across pages */
.contacts-page {
  height: calc(100vh - 4rem);
  margin: -2rem;
  display: flex;
  flex-direction: column;
}
```

---

## Goals

- [ ] Eliminate all 8 inline `calc(100vh - 4rem)` / `margin: -2rem` blocks
- [ ] Provide a single point of change if layout padding ever changes
- [ ] Keep the component API minimal (one optional prop)

## Non-Goals

- PageWrapper does not handle sidebar width or top-nav height -- those are the layout's responsibility
- PageWrapper does not set `background` or `color` -- pages own their own surface colors
- PageWrapper does not include any scroll behavior for children -- each child panel manages its own overflow

---

## Allowed / Not Allowed

### ALLOWED after adoption

- Using `<PageWrapper>` in all pages that need to escape layout padding
- Extending with the `overflow` prop when the default `hidden` is not appropriate
- Nesting `<SplitPanel>` or any other layout component inside

### NOT ALLOWED after adoption

> These rules are enforced by the adoption grep test in CI.

- `height: calc(100vh - 4rem)` in any page-scoped `<style>` block
- `margin: -2rem` paired with the height calc as a layout escape pattern
- Hardcoded viewport-escape math in any new page

---

## Adoption Strategy

### Progressive Adoption

1. Component is created with tests
2. Reference page is migrated first (proves the component works)
3. Remaining pages migrate one at a time
4. Each migration is a separate commit (easy to revert)

### Reference Page

**Page:** `/data/notes` (`packages/webui/src/routes/data/notes/+page.svelte`)
**Why this page:** Simplest split-panel layout, fewest edge cases (no loading-state wrapper, no conditional layout modes).

### Migration Order

| Order | Page | File | Complexity | Notes |
|-------|------|------|------------|-------|
| 1 | Notes | `src/routes/data/notes/+page.svelte` | Low | Reference implementation |
| 2 | Contacts | `src/routes/data/contacts/+page.svelte` | Low | Has loading-container that wraps split panel |
| 3 | Job Descriptions | `src/routes/opportunities/job-descriptions/+page.svelte` | Medium | Has list/detail mode toggle |
| 4 | Organizations | `src/routes/data/organizations/+page.svelte` | Low | Standard split-panel |
| 5 | Skills | `src/routes/data/skills/+page.svelte` | Low | Wrapper page for sub-views |
| 6 | Bullets | `src/routes/data/bullets/+page.svelte` | Low | Wrapper page for sub-views |
| 7 | Resumes | `src/routes/resumes/+page.svelte` | Medium | Has list mode and detail mode |

### Migration Checklist (per page)

- [ ] Import `PageWrapper` from `$lib/components`
- [ ] Wrap page content in `<PageWrapper>` (with overflow prop if needed)
- [ ] Remove the outer page div class that had the viewport-escape CSS
- [ ] Remove `height: calc(100vh - 4rem)`, `margin: -2rem`, `display: flex`, `flex-direction: column` from the page-scoped `<style>` block
- [ ] Verify visual match (before/after screenshot or manual check)
- [ ] Verify no regressions in interactive behavior
- [ ] Commit with message: `refactor(webui): migrate [page] to <PageWrapper>`

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
  name: 'PageWrapper',
  pattern: /height:\s*calc\(100vh\s*-\s*4rem\)/,
  allowedIn: ['PageWrapper.svelte'],
  message: 'Use <PageWrapper> instead of inline viewport-escape CSS (height: calc(100vh - 4rem))',
}
```

### CLAUDE.md Rule

```markdown
- Full-viewport pages MUST use `<PageWrapper>`, not inline `height: calc(100vh - 4rem); margin: -2rem`
```

---

## Testing

### Unit Tests

| Test | Description |
|------|-------------|
| Renders children | Confirms the default slot content appears inside the wrapper |
| Default overflow is hidden | Wrapper div has `overflow: hidden` when no prop is passed |
| Overflow prop applies | Each overflow value (`auto`, `hidden`, `visible`) results in the correct CSS class |

### Component Tests

| Test | Description |
|------|-------------|
| Fills viewport height | Wrapper has `height: calc(100vh - 4rem)` computed style |
| Negative margin escapes padding | Wrapper has `margin: -2rem` computed style |
| Composes with SplitPanel | SplitPanel renders inside PageWrapper with correct dimensions |

### Adoption Enforcement Tests

| Test | Description |
|------|-------------|
| Anti-pattern grep | Fails if `height: calc(100vh - 4rem)` found outside `PageWrapper.svelte` |
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

- Component introduces new layout math instead of centralizing the existing pattern
- Hardcoded `4rem` or `2rem` values instead of token references
- Migration breaks interactive behavior on any page
- Component API requires page-specific props (should be generic)
