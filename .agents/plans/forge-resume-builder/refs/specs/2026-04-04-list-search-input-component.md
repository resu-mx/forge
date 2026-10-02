# ListSearchInput -- UI Component Spec

**Date:** 2026-04-04
**Status:** Draft
**Package:** `@forge/webui`
**Category:** polish
**Replaces:** 7+ search input implementations with varying padding, font-size, border-radius, and focus colors across list pages and modals

---

## Overview

**What:** A shared search input component for filtering lists, providing a consistent text input with standardized sizing, padding, border-radius, and focus ring styling.

**Why:** Seven pages and modals implement `.search-input` with inconsistent CSS: padding ranges from `0.35rem` to `0.5rem`, some use `var(--text-sm)` while others use `var(--text-base)`, focus ring implementations vary (some use `box-shadow`, some use `outline`, some have no focus indicator at all), and border-radius values differ. This duplication creates visual inconsistency and makes rebranding require touching every file.

**Pages affected:**
- `/data/notes/+page.svelte`
- `/data/organizations/+page.svelte`
- `/data/contacts/+page.svelte`
- `/opportunities/job-descriptions/+page.svelte`
- `/data/sources/SkillsView.svelte`
- `/data/sources/BulletsView.svelte`
- `$lib/components/ChainViewModal.svelte`
- `$lib/components/graph/GraphSearchBar.svelte`
- `$lib/components/resume/JDPickerModal.svelte`
- `$lib/components/jd/ResumePickerModal.svelte`
- `$lib/components/kanban/OrgPickerModal.svelte`

---

## Component API

### Props

| Prop | Type | Default | Required | Description |
|------|------|---------|----------|-------------|
| `value` | `string` | `''` | No | The current search query (bindable with `bind:value`) |
| `placeholder` | `string` | `'Search...'` | No | Placeholder text shown when input is empty |

### Snippets (Slots)

None. ListSearchInput is a single-purpose input with no extensibility points.

### Events / Callbacks

None explicitly defined. The `value` prop is bindable, so parents use `bind:value={searchQuery}` for two-way binding. Standard DOM events (`oninput`, `onfocus`, `onblur`) pass through to the underlying `<input>` element via `{...$$restProps}`.

### TypeScript Types

```typescript
// Export from $lib/components/ListSearchInput.svelte
interface ListSearchInputProps {
  value?: string;
  placeholder?: string;
}
```

---

## Styling

### Concrete CSS (copy-paste)

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

### Token Consumption

| Token | Usage | Fallback |
|-------|-------|----------|
| `--text-sm` | Input font size (12.8px) | `0.8rem` |
| `--text-primary` | Input text color | `#1a1a2e` |
| `--text-faint` | Placeholder text color | `#9ca3af` |
| `--color-surface` | Input background | `#ffffff` |
| `--color-border-strong` | Default border color | `#d1d5db` |
| `--color-primary` | Focus border color | `#6c63ff` |
| `--color-primary-subtle` | Focus ring (box-shadow) | `rgba(108, 99, 255, 0.15)` |
| `--radius-md` | Border radius (6px) | `6px` |
| `--space-2` | Vertical padding (8px) | `0.5rem` |
| `--space-3` | Horizontal padding (12px) | `0.75rem` |

### Branding Strategy

All visual properties come from design tokens. The focus ring uses `--color-primary` for the border and `--color-primary-subtle` for the glow, making it automatically consistent with the rest of the design system's focus indicators. Rebranding means changing token values only -- no component CSS changes needed. Dark mode is handled entirely by token overrides in `tokens.css`.

### Rendered Markup

```html
<input
  type="text"
  class="list-search-input"
  placeholder="Search..."
  value=""
/>
```

---

## Behavior

### State Management

ListSearchInput exposes `value` as a bindable prop. The parent owns the search query state and binds to it with `bind:value`. No internal state is managed -- the component is a thin styling wrapper around a native `<input>`.

### Accessibility

- Uses native `<input type="text">` for full built-in accessibility
- Placeholder text provides a hint; does not replace a label
- Parents should provide an associated `<label>` or `aria-label` when the input is not visually labeled by adjacent heading text
- Focus indicator is visible and uses the design system's primary color ring
- No custom keyboard handling needed -- native input behavior is correct

### Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| Empty value | Input shows placeholder text in `--text-faint` color |
| Very long search query | Text scrolls horizontally within the input (native behavior) |
| Parent does not bind value | Input works as uncontrolled with default empty string |
| Custom placeholder | Placeholder text is overridden via the `placeholder` prop |
| Input inside a flex container | `width: 100%` fills available space; parent controls width |
| Dark mode | Colors adapt via token overrides; no component changes |

---

## Examples

### Explicit Examples (DO THIS)

```svelte
<!-- Example 1: Basic usage -- default placeholder -->
<script lang="ts">
  import ListSearchInput from '$lib/components/ListSearchInput.svelte'
  let searchQuery = $state('')
</script>

<ListSearchInput bind:value={searchQuery} />
```

```svelte
<!-- Example 2: Custom placeholder -->
<ListSearchInput bind:value={searchQuery} placeholder="Search notes..." />
```

### Implicit Examples (THIS IS THE PATTERN)

```svelte
<!-- Typical list page showing ListSearchInput in a filter bar -->
<script lang="ts">
  import ListSearchInput from '$lib/components/ListSearchInput.svelte'

  let searchQuery = $state('')

  let filteredNotes = $derived.by(() => {
    if (!searchQuery.trim()) return notes
    const q = searchQuery.toLowerCase()
    return notes.filter(n =>
      n.content.toLowerCase().includes(q) ||
      (n.title && n.title.toLowerCase().includes(q))
    )
  })
</script>

<div class="list-panel">
  <div class="list-header">
    <h2>Notes</h2>
    <button class="btn-new" onclick={startNew}>+ New</button>
  </div>

  <div class="filter-bar">
    <ListSearchInput bind:value={searchQuery} placeholder="Search notes..." />
  </div>

  <ul class="note-list">
    {#each filteredNotes as note (note.id)}
      <!-- list items -->
    {/each}
  </ul>
</div>
```

### Anti-Examples (DON'T DO THIS)

```svelte
<!-- WRONG: Raw input with page-scoped search-input class -->
<input
  type="text"
  class="search-input"
  placeholder="Search notes..."
  bind:value={searchQuery}
/>
```

```css
/* WRONG: Page-scoped .search-input styles that duplicate the shared component */
.search-input {
  width: 100%;
  padding: 0.4rem 0.65rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.search-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-subtle);
}
```

---

## Goals

- [x] Eliminate all per-page `.search-input` style blocks
- [x] Guarantee consistent padding (--space-2 / --space-3), font-size (--text-sm), border-radius (--radius-md), and focus ring (--color-primary) across all search inputs
- [x] Provide a bindable `value` prop for seamless two-way data flow
- [x] Zero hardcoded values -- 100% token-driven
- [x] Works in both list pages and modal picker contexts

## Non-Goals

- Search icon or clear button (can be added later as an enhancement)
- Debounced search (parent handles debounce logic if needed)
- Dropdown suggestions or autocomplete (separate component concern)
- Full-text search API integration (parent handles API calls)

---

## Allowed / Not Allowed

### ALLOWED after adoption

- Using `<ListSearchInput>` in all list filter bars and modal search fields
- Binding `value` for two-way data flow
- Customizing `placeholder` text per usage context
- Wrapping in a `<div class="filter-bar">` for layout spacing (parent concern)

### NOT ALLOWED after adoption

> These rules are enforced by the adoption grep test in CI.

- `.search-input {` in any page-scoped `<style>` block (use `<ListSearchInput>` instead)
- Inline `<input>` elements with manual focus ring CSS for search/filter purposes
- Inconsistent padding or font-size on search inputs

---

## Adoption Strategy

### Progressive Adoption

1. Component is created with tests
2. Reference page is migrated first (proves the component works)
3. Remaining pages migrate one at a time
4. Each migration is a separate commit (easy to revert)

### Reference Page

**Page:** `/data/notes` (`packages/webui/src/routes/data/notes/+page.svelte`)
**Why this page:** Clean, isolated search input usage in a filter bar. No extra complexity like icons or debounce. The existing `.search-input` CSS closely matches the target component styling.

### Migration Order

| Order | Page | File | Complexity | Notes |
|-------|------|------|------------|-------|
| 1 | /data/notes | `routes/data/notes/+page.svelte` | Low | Reference implementation; clean filter bar |
| 2 | /data/organizations | `routes/data/organizations/+page.svelte` | Low | Same filter bar pattern |
| 3 | /data/contacts | `routes/data/contacts/+page.svelte` | Low | Same filter bar pattern |
| 4 | /opportunities/job-descriptions | `routes/opportunities/job-descriptions/+page.svelte` | Low | Same filter bar pattern |
| 5 | /data/sources (skills) | `routes/data/sources/SkillsView.svelte` | Low | Search within skills tab |
| 6 | /data/sources (bullets) | `routes/data/sources/BulletsView.svelte` | Low | Search within bullets tab |
| 7 | Modals | `$lib/components/ChainViewModal.svelte` | Medium | Modal context; verify width behavior |
| 8 | Modals | `$lib/components/resume/JDPickerModal.svelte` | Medium | Modal context |
| 9 | Modals | `$lib/components/jd/ResumePickerModal.svelte` | Medium | Modal context |
| 10 | Modals | `$lib/components/kanban/OrgPickerModal.svelte` | Medium | Modal context |
| 11 | Graph | `$lib/components/graph/GraphSearchBar.svelte` | Medium | May have additional styling concerns |

### Migration Checklist (per page)

- [ ] Import `ListSearchInput` from `$lib/components`
- [ ] Replace inline `<input class="search-input" ...>` with `<ListSearchInput bind:value={searchQuery} placeholder="..." />`
- [ ] Remove page-scoped `.search-input` and `.search-input:focus` CSS rules
- [ ] Verify visual match (before/after screenshot or manual check)
- [ ] Verify search filtering still works correctly
- [ ] Verify focus ring appears on keyboard focus
- [ ] Commit with message: `refactor(webui): migrate [page] to <ListSearchInput>`

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
  name: 'ListSearchInput',
  pattern: /\.search-input\s*\{/,
  allowedIn: ['ListSearchInput.svelte'],
  message: 'Use <ListSearchInput> instead of page-scoped .search-input styles',
}
```

### CLAUDE.md Rule

```markdown
- List search/filter inputs MUST use `<ListSearchInput>`, not inline `.search-input` CSS
```

---

## Testing

### Unit Tests

| Test | Description |
|------|-------------|
| Renders with default placeholder | `<ListSearchInput />` renders input with placeholder "Search..." |
| Renders custom placeholder | `<ListSearchInput placeholder="Filter..." />` uses custom placeholder text |
| Binds value | `bind:value={query}` updates the parent's reactive variable on input |
| Renders with initial value | `<ListSearchInput value="test" />` shows "test" in the input |
| Applies correct CSS class | Input element has class `list-search-input` |
| Full width | Input has `width: 100%` computed style |

### Component Tests

| Test | Description |
|------|-------------|
| Focus ring | Clicking the input shows `--color-primary` border and `--color-primary-subtle` box-shadow |
| Placeholder color | Placeholder text uses `--text-faint` color |
| Visual regression | Screenshot comparison of input in default, focused, and filled states |
| Dark mode | Input adapts correctly via token overrides |

### Adoption Enforcement Tests

| Test | Description |
|------|-------------|
| Anti-pattern grep | Fails if `.search-input {` found outside `ListSearchInput.svelte` |
| Migration count | Tracks remaining pages to migrate (informational, not blocking) |

---

## Acceptance Criteria

- [ ] Component renders correctly with all prop combinations
- [ ] Reference page (`/data/notes`) migrated with no visual diff
- [ ] All design tokens used (no hardcoded values)
- [ ] Focus ring is visible and uses `--color-primary`
- [ ] `bind:value` works for two-way data flow
- [ ] Grep test added and passing
- [ ] CLAUDE.md rule added
- [ ] Component exported from `$lib/components/`

## Failure Criteria

- Component introduces a new visual variant instead of matching the canonical search input from notes
- Hardcoded color/spacing/border values instead of token references
- Migration breaks search filtering on any page
- Component API requires page-specific props (should be generic)
- Focus ring does not appear or uses a different color than `--color-primary`
