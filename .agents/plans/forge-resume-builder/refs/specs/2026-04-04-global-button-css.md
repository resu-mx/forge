# Global Button CSS — UI Component Spec

**Date:** 2026-04-04
**Status:** Draft
**Package:** `@forge/webui`
**Category:** polish
**Replaces:** ~200 lines of per-page button CSS across 11+ page-scoped `<style>` blocks

---

## Overview

**What:** A set of global CSS utility classes (`.btn`, `.btn-primary`, `.btn-danger`, `.btn-ghost`, `.btn-sm`, `.btn-lg`) added to `packages/webui/src/lib/styles/base.css`, using design tokens exclusively. Not a Svelte component.

**Why:** Every page that has buttons re-defines `.btn`, `.btn-primary`, `.btn-ghost`, `.btn-sm`, and related classes in its own `<style>` block. The definitions are nearly identical but have small drift: some use `border-radius: 6px` while others use `var(--radius-md)`, some use `font-weight: 500` while others use `var(--font-medium)`, and the `padding` / `font-size` values vary slightly. This produces ~200 lines of duplicated CSS and makes it impossible to change button styling globally. Moving to a single set of global classes eliminates the duplication and ensures consistent button rendering.

**Pages affected (files with `.btn-primary {` in page-scoped `<style>`):**

- `/resumes/summaries/+page.svelte`
- `/data/sources/BulletsView.svelte`
- `/config/profile/+page.svelte`
- `/resumes/+page.svelte`
- `/resumes/templates/+page.svelte`
- `/config/export/+page.svelte`
- `/data/domains/ArchetypesView.svelte`
- `/data/domains/DomainsView.svelte`
- `/data/notes/+page.svelte`
- `/data/summaries/_old_page.svelte.bak` (legacy)
- `/config/templates/_old_page.svelte.bak` (legacy)

---

## Component API

This is NOT a Svelte component. It is a set of CSS classes added to the global stylesheet.

### CSS Classes

| Class | Purpose | Can combine with |
|-------|---------|-----------------|
| `.btn` | Base button reset: `display`, `align-items`, `border`, `border-radius`, `cursor`, `transition`, `font-family` | Always required as base class |
| `.btn-primary` | Brand action: `--color-primary` background, `--text-inverse` text | `.btn` |
| `.btn-danger` | Destructive action: `--color-danger` background, `--text-inverse` text | `.btn` |
| `.btn-ghost` | Subtle/secondary action: transparent background, `--text-secondary` text | `.btn` |
| `.btn-sm` | Small size modifier: tighter padding, `--text-sm` font | `.btn` + any variant |
| `.btn-lg` | Large size modifier: roomier padding, `--text-lg` font | `.btn` + any variant |

### Usage Pattern

```html
<button class="btn btn-primary">Save</button>
<button class="btn btn-danger btn-sm">Delete</button>
<button class="btn btn-ghost">Cancel</button>
<button class="btn btn-primary btn-lg">Create Resume</button>
```

### TypeScript Types

Not applicable -- this is pure CSS.

---

## Styling

### Concrete CSS (copy-paste)

```css
/* ================================================================
   Global Button Classes
   Added to packages/webui/src/lib/styles/base.css
   ================================================================ */

/* --- Base --- */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-4);       /* 0.5rem 1rem */
  border: none;
  border-radius: var(--radius-md);
  font-family: inherit;
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  line-height: var(--leading-tight);
  cursor: pointer;
  transition: background 0.15s, color 0.15s, opacity 0.15s;
  white-space: nowrap;
  text-decoration: none;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn:focus-visible {
  outline: 2px solid var(--color-border-focus);
  outline-offset: 2px;
}

/* --- Variants --- */
.btn-primary {
  background: var(--color-primary);
  color: var(--text-inverse);
}
.btn-primary:hover:not(:disabled) {
  background: var(--color-primary-hover);
}

.btn-danger {
  background: var(--color-danger);
  color: var(--text-inverse);
}
.btn-danger:hover:not(:disabled) {
  background: var(--color-danger-hover);
}

.btn-ghost {
  background: transparent;
  color: var(--text-secondary);
}
.btn-ghost:hover:not(:disabled) {
  background: var(--color-ghost);
  color: var(--color-ghost-text);
}

/* --- Sizes --- */
.btn-sm {
  padding: var(--space-1) var(--space-3);       /* 0.25rem 0.75rem */
  font-size: var(--text-sm);
}

.btn-lg {
  padding: var(--space-3) var(--space-6);       /* 0.75rem 1.5rem */
  font-size: var(--text-lg);
}
```

### Token Consumption

| Token | Usage | Fallback |
|-------|-------|----------|
| `--space-1` | `.btn-sm` vertical padding | `0.25rem` |
| `--space-2` | `.btn` vertical padding, gap between icon and text | `0.5rem` |
| `--space-3` | `.btn-sm` horizontal padding, `.btn-lg` vertical padding | `0.75rem` |
| `--space-4` | `.btn` horizontal padding | `1rem` |
| `--space-6` | `.btn-lg` horizontal padding | `1.5rem` |
| `--radius-md` | Button border radius | `6px` |
| `--font-medium` | Button font weight | `500` |
| `--text-sm` | Default and `.btn-sm` font size | `0.8rem` |
| `--text-lg` | `.btn-lg` font size | `1rem` |
| `--leading-tight` | Button line height | `1.25` |
| `--color-primary` | `.btn-primary` background | `#6c63ff` |
| `--color-primary-hover` | `.btn-primary` hover background | `#5a52e0` |
| `--color-danger` | `.btn-danger` background | `#ef4444` |
| `--color-danger-hover` | `.btn-danger` hover background | `#dc2626` |
| `--text-inverse` | `.btn-primary` and `.btn-danger` text color | `#ffffff` |
| `--text-secondary` | `.btn-ghost` text color | `#374151` |
| `--color-ghost` | `.btn-ghost` hover background | `#f3f4f6` |
| `--color-ghost-text` | `.btn-ghost` hover text color | `#374151` |
| `--color-border-focus` | Focus ring color | `#6c63ff` |

### Branding Strategy

Every visual property of every button variant comes from design tokens. Rebranding the entire button system means changing token values in `tokens.css` only -- zero changes to `base.css`. Dark mode support is automatic via the existing token overrides in `tokens.css`.

### Rendered Markup

```html
<!-- Primary button -->
<button class="btn btn-primary">Save Changes</button>

<!-- Small danger button -->
<button class="btn btn-danger btn-sm">Delete</button>

<!-- Ghost button (default size) -->
<button class="btn btn-ghost">Cancel</button>

<!-- Large primary button -->
<button class="btn btn-primary btn-lg">Create Resume</button>

<!-- Disabled state -->
<button class="btn btn-primary" disabled>Saving...</button>
```

---

## Behavior

### State Management

Not applicable -- CSS classes have no state. The `:disabled` pseudo-class is handled natively by the browser. The `:hover` and `:focus-visible` states are CSS-only.

### Accessibility

- `:focus-visible` provides a visible focus ring (`2px solid --color-border-focus`) for keyboard navigation.
- `:disabled` reduces opacity and changes cursor, providing visual feedback.
- `.btn` does not add any ARIA attributes -- those are the responsibility of the markup using the classes.
- Ghost buttons maintain a minimum 4.5:1 contrast ratio between `--text-secondary` and `--color-surface` in both light and dark themes.

### Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| Button with icon + text | `.btn` `gap: var(--space-2)` spaces the icon and text evenly |
| Button used on `<a>` tag | Works -- `.btn` uses `display: inline-flex` and `text-decoration: none` |
| Multiple size classes combined (e.g. `.btn-sm.btn-lg`) | Last class in source order wins. Do not combine sizes. |
| Page-scoped `.btn` still exists during migration | Page-scoped style takes precedence over global due to Svelte's higher specificity. Migration removes the page-scoped version. |
| Dark mode | Token values switch automatically via `@media (prefers-color-scheme: dark)` and `html[data-theme]` overrides in `tokens.css` |

---

## Examples

### Explicit Examples (DO THIS)

```svelte
<!-- Example 1: Basic button group in a form -->
<div class="form-actions">
  <button class="btn btn-primary" onclick={save} disabled={saving}>
    {saving ? 'Saving...' : 'Save'}
  </button>
  <button class="btn btn-ghost" onclick={cancel}>Cancel</button>
  <button class="btn btn-danger btn-sm" onclick={confirmDelete}>Delete</button>
</div>
```

```svelte
<!-- Example 2: Page header with large primary action -->
<div class="page-header">
  <h1>Summaries</h1>
  <button class="btn btn-primary" onclick={startCreate}>+ New Summary</button>
</div>
```

### Implicit Examples (THIS IS THE PATTERN)

> Shows how global button classes replace per-page button CSS in a typical page.

```svelte
<!-- AFTER: Summaries page using global button classes -->
<script lang="ts">
  // ... existing script unchanged ...
</script>

<div class="summaries-page">
  <div class="page-header">
    <div>
      <h1 class="page-title">Summaries</h1>
      <p class="subtitle">Reusable professional summaries and templates</p>
    </div>
    <button class="btn btn-primary" onclick={startCreate}>+ New Summary</button>
  </div>

  <!-- buttons throughout use global classes -->
  <button class="btn btn-primary btn-sm" onclick={() => saveEdit(summary.id)}>Save</button>
  <button class="btn btn-ghost btn-sm" onclick={() => editing = null}>Cancel</button>
</div>

<style>
  /* Page-scoped styles for layout and content ONLY — no button CSS needed */
  .summaries-page { max-width: 800px; }
  .page-header { display: flex; justify-content: space-between; ... }
  /* .btn, .btn-primary, .btn-ghost, .btn-sm — ALL REMOVED */
</style>
```

### Anti-Examples (DON'T DO THIS)

```css
/* WRONG: Page-scoped .btn-primary re-definition after global classes exist */
<style>
  .btn-primary {
    background: var(--color-primary);
    color: var(--text-inverse);
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
  }
  .btn-primary:hover { background: var(--color-primary-hover); }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
```

```css
/* WRONG: Hardcoded values instead of tokens */
<style>
  .btn {
    border: none;
    border-radius: 6px;        /* should be var(--radius-md) */
    font-weight: 500;          /* should be var(--font-medium) */
    cursor: pointer;
    transition: background 0.15s;
  }
</style>
```

```css
/* WRONG: Per-page button variant with non-standard name */
<style>
  .btn-save { background: var(--color-primary); color: var(--color-surface); }
  .btn-delete { background: var(--color-danger-subtle); color: var(--color-danger-text); }
</style>
```

---

## Goals

- [ ] Add global `.btn`, `.btn-primary`, `.btn-danger`, `.btn-ghost`, `.btn-sm`, `.btn-lg` classes to `base.css`
- [ ] Eliminate ~200 lines of per-page button CSS across 11+ files
- [ ] All button visual properties use design tokens (zero hardcoded values)
- [ ] Dark mode works automatically via existing token overrides
- [ ] Focus ring on all buttons for keyboard accessibility

## Non-Goals

- This does not create a Svelte `<Button>` component -- the CSS classes are applied directly to `<button>` and `<a>` elements
- This does not add loading states, icon support, or other behavioral features -- those are the page's responsibility
- This does not cover every one-off button variant (e.g. `.btn-add`, `.btn-save`, `.btn-danger-ghost`) -- only the canonical set. Rare variants remain page-scoped until they prove reusable.

---

## Allowed / Not Allowed

### ALLOWED after adoption

- Using global `.btn` + variant classes on any `<button>` or `<a>` element
- Combining size modifiers with variant classes (e.g. `.btn.btn-primary.btn-sm`)
- Adding page-scoped overrides for truly unique one-off buttons (but these should be rare)

### NOT ALLOWED after adoption

> These rules are enforced by the adoption grep test in CI.

- `.btn-primary {` in any page-scoped `<style>` block (use the global class instead)
- `border-radius: 6px` on buttons (use `var(--radius-md)` via `.btn` class)
- `font-weight: 500` on buttons (use `var(--font-medium)` via `.btn` class)
- Creating new `.btn-save`, `.btn-delete`, or similar non-standard names -- use `.btn-primary` or `.btn-danger` instead

---

## Adoption Strategy

### Progressive Adoption

1. Global button classes are added to `base.css`
2. Reference page is migrated first (removes its page-scoped button CSS)
3. Remaining pages migrate one at a time
4. Each migration is a separate commit (easy to revert)

### Reference Page

**Page:** `/resumes/summaries` (`packages/webui/src/routes/resumes/summaries/+page.svelte`)
**Why this page:** Has the most complete set of button variants (`.btn`, `.btn-primary`, `.btn-ghost`, `.btn-danger-ghost`, `.btn-sm`, `.btn-xs`) -- 30+ lines of button CSS. Migrating this page proves all the global classes work.

### Migration Order

| Order | Page | File | Complexity | Notes |
|-------|------|------|------------|-------|
| 1 | Summaries | `src/routes/resumes/summaries/+page.svelte` | Low | Reference: most button variants. Remove ~30 lines of button CSS. |
| 2 | Notes | `src/routes/data/notes/+page.svelte` | Low | Remove `.btn`, `.btn-save`, `.btn-delete`, `.btn-primary`, `.btn-ghost`, `.btn-danger`, `.btn-sm` CSS. Rename `.btn-save` to `.btn-primary`, `.btn-delete` to `.btn-danger`. |
| 3 | Profile | `src/routes/config/profile/+page.svelte` | Low | Standard button CSS removal |
| 4 | Resumes | `src/routes/resumes/+page.svelte` | Low | Standard button CSS removal |
| 5 | Templates | `src/routes/resumes/templates/+page.svelte` | Low | Standard button CSS removal |
| 6 | Export | `src/routes/config/export/+page.svelte` | Low | Standard button CSS removal |
| 7 | BulletsView | `src/routes/data/sources/BulletsView.svelte` | Low | Sub-view button CSS removal |
| 8 | ArchetypesView | `src/routes/data/domains/ArchetypesView.svelte` | Low | Sub-view button CSS removal |
| 9 | DomainsView | `src/routes/data/domains/DomainsView.svelte` | Low | Sub-view button CSS removal |

### Migration Checklist (per page)

- [ ] Verify the page's `<style>` block defines `.btn`, `.btn-primary`, or related button classes
- [ ] Replace any non-standard button class names (`.btn-save` -> `.btn-primary`, `.btn-delete` -> `.btn-danger`) in the markup
- [ ] Remove all button-related CSS from the page-scoped `<style>` block
- [ ] Verify buttons still render correctly (colors, sizes, hover, disabled, focus)
- [ ] Verify dark mode button rendering
- [ ] Verify no regressions in interactive behavior
- [ ] Commit with message: `refactor(webui): migrate [page] to global button CSS`

### Coexistence Rules

During migration, old and new implementations coexist. Rules:

- New pages MUST use the global button classes (enforced by CLAUDE.md rule)
- Existing pages keep their page-scoped button CSS until migrated -- Svelte's scoped styles have higher specificity than global styles, so there are no conflicts
- The grep test tracks remaining violations -- count decreases toward zero
- Global class definitions are frozen after reference page ships (no breaking changes during migration)

---

## Adoption Enforcement

### CI Grep Test

```typescript
// In packages/webui/src/__tests__/component-adoption.test.ts
{
  name: 'Global Button CSS',
  pattern: /\.btn-primary\s*\{/,
  allowedIn: ['base.css'],
  message: 'Use global .btn-primary class from base.css instead of page-scoped button CSS',
}
```

### CLAUDE.md Rule

```markdown
- Button styling MUST use global `.btn` + `.btn-primary`/`.btn-danger`/`.btn-ghost` classes from `base.css`, not page-scoped button CSS
```

---

## Testing

### Unit Tests

Not applicable -- these are CSS classes, not a component.

### Visual Regression Tests

| Test | Description |
|------|-------------|
| `.btn-primary` render | Primary button has correct background, text color, padding, and border-radius |
| `.btn-danger` render | Danger button has correct red background and white text |
| `.btn-ghost` render | Ghost button has transparent background and secondary text color |
| `.btn-sm` size | Small button has tighter padding and `--text-sm` font size |
| `.btn-lg` size | Large button has roomier padding and `--text-lg` font size |
| Hover states | Each variant changes background on hover |
| Disabled state | Disabled buttons have 0.5 opacity and `not-allowed` cursor |
| Focus ring | Focus-visible shows `2px solid --color-border-focus` outline |
| Dark mode | All variants render with dark-mode token values |

### Adoption Enforcement Tests

| Test | Description |
|------|-------------|
| Anti-pattern grep | Fails if `.btn-primary {` found outside `base.css` |
| Migration count | Tracks remaining pages with page-scoped button CSS (informational, not blocking) |

---

## Acceptance Criteria

- [ ] Global button classes added to `base.css`
- [ ] Reference page (summaries) migrated with no visual diff
- [ ] All design tokens used (no hardcoded values in button classes)
- [ ] Dark mode works automatically
- [ ] Focus ring present on all button variants
- [ ] Grep test added and passing
- [ ] CLAUDE.md rule added

## Failure Criteria

- Global classes introduce new visual variants that do not match the canonical pattern across pages
- Hardcoded color/spacing values instead of token references
- Migration breaks button rendering on any page (especially during coexistence with page-scoped styles)
- Button classes require page-specific overrides to look correct (should be standalone)
