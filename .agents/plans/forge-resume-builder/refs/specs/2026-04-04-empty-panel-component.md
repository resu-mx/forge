# EmptyPanel -- UI Component Spec

**Date:** 2026-04-04
**Status:** Draft
**Package:** `@forge/webui`
**Category:** polish
**Replaces:** 4 different "nothing selected" right-panel implementations (2 raw `<p>` tags, 2 using `<EmptyState>` which is too heavy for this pattern)

---

## Overview

**What:** A lightweight empty-state placeholder for detail/editor panels in list-detail layouts, showing a centered muted message when no item is selected.

**Why:** Four list-detail pages implement their own "nothing selected" state with inconsistent markup: notes and organizations use raw `<div class="editor-empty"><p>...</p></div>` with page-scoped CSS; contacts and JD pages use `<EmptyState>` which renders a heavier card with title/description/action that is visually too prominent for a simple "select something" hint. EmptyPanel provides a purpose-built, lightweight alternative that is visually consistent and semantically correct for the "no selection" state.

**Pages affected:**
- `/data/notes/+page.svelte` (`.editor-empty` with raw `<p>`)
- `/data/organizations/+page.svelte` (`.editor-empty` or `.empty-editor` with raw `<p>`)
- `/data/contacts/+page.svelte` (`.editor-empty` or using `<EmptyState>`)
- `/opportunities/job-descriptions/+page.svelte` (`.editor-empty` or using `<EmptyState>`)

---

## Component API

### Props

| Prop | Type | Default | Required | Description |
|------|------|---------|----------|-------------|
| `message` | `string` | `'Select an item to view details.'` | No | The placeholder message shown in the empty panel |
| `actionLabel` | `string` | `undefined` | No | Optional button label shown below the message |
| `onAction` | `() => void` | `undefined` | No | Callback fired when the action button is clicked |

### Snippets (Slots)

None. EmptyPanel is intentionally minimal with no slot-based extensibility.

### Events / Callbacks

| Callback | Signature | Description |
|----------|-----------|-------------|
| `onAction` | `() => void` | Called when the optional action button is clicked |

### TypeScript Types

```typescript
// Export from $lib/components/EmptyPanel.svelte
interface EmptyPanelProps {
  message?: string;
  actionLabel?: string;
  onAction?: () => void;
}
```

---

## Styling

### Concrete CSS (copy-paste)

```css
.empty-panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: var(--space-8);
  gap: var(--space-3);
}

.empty-panel-message {
  font-size: var(--text-sm);
  color: var(--text-muted);
  text-align: center;
  line-height: var(--leading-normal);
}

.empty-panel-action {
  padding: var(--space-2) var(--space-4);
  background: none;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--text-secondary);
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}

.empty-panel-action:hover {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.empty-panel-action:focus-visible {
  outline: 2px solid var(--color-border-focus);
  outline-offset: 2px;
}
```

### Token Consumption

| Token | Usage | Fallback |
|-------|-------|----------|
| `--text-sm` | Message and button font size (12.8px) | `0.8rem` |
| `--text-muted` | Message text color | `#6b7280` |
| `--text-secondary` | Button text color | `#374151` |
| `--color-border` | Button border color | `#e5e7eb` |
| `--color-primary` | Button hover border and text color | `#6c63ff` |
| `--color-border-focus` | Focus-visible outline color | `#6c63ff` |
| `--font-medium` | Button font weight | `500` |
| `--radius-md` | Button border radius | `6px` |
| `--space-2` | Button vertical padding | `0.5rem` |
| `--space-3` | Gap between message and action | `0.75rem` |
| `--space-4` | Button horizontal padding | `1rem` |
| `--space-8` | Container padding | `2rem` |
| `--leading-normal` | Message line-height | `1.5` |

### Branding Strategy

All visual properties come from design tokens. The panel is intentionally minimal -- just centered text in `--text-muted`. Rebranding means changing token values only. The optional action button uses a ghost-border style that adapts to `--color-primary` on hover. No component CSS changes needed for rebranding.

### Rendered Markup

```html
<!-- Minimal (no action) -->
<div class="empty-panel">
  <p class="empty-panel-message">Select an item to view details.</p>
</div>

<!-- With action -->
<div class="empty-panel">
  <p class="empty-panel-message">Select a note or create a new one.</p>
  <button class="empty-panel-action">New Note</button>
</div>
```

---

## Behavior

### State Management

EmptyPanel is stateless. The message is a prop with a sensible default. The optional action button is purely a passthrough to the parent's callback. No internal state.

### Accessibility

- Message uses a `<p>` element for proper semantic structure
- Action button is a native `<button>` element with full keyboard support
- Focus-visible styling on the action button for keyboard navigation
- No ARIA attributes needed -- the component is simple enough that native semantics suffice
- When used in a panel context, the parent should manage focus (e.g., moving focus to this panel when a list item is deselected)

### Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| No props provided | Renders default message: "Select an item to view details." |
| Custom message, no action | Renders only the message text centered |
| Action label without onAction | Button renders but clicking does nothing (defensive) |
| onAction without actionLabel | No button rendered (actionLabel gates visibility) |
| Panel container has fixed height | EmptyPanel fills 100% height and centers vertically |
| Panel container has no height | EmptyPanel collapses to content height with padding |

---

## Examples

### Explicit Examples (DO THIS)

```svelte
<!-- Example 1: Basic usage -- default message -->
<EmptyPanel />
```

```svelte
<!-- Example 2: Custom message for notes context -->
<EmptyPanel message="Select a note or create a new one." />
```

```svelte
<!-- Example 3: With action button -->
<EmptyPanel
  message="No contact selected."
  actionLabel="New Contact"
  onAction={startNew}
/>
```

### Implicit Examples (THIS IS THE PATTERN)

```svelte
<!-- Typical list-detail page showing EmptyPanel in the right panel -->
<script lang="ts">
  import { EmptyPanel } from '$lib/components'
</script>

<div class="notes-page">
  <div class="list-panel">
    <!-- list content -->
  </div>

  <div class="editor-panel">
    {#if !selectedNote && !editing}
      <EmptyPanel message="Select a note or create a new one." />
    {:else}
      <div class="editor-content">
        <!-- editor form -->
      </div>
    {/if}
  </div>
</div>
```

### Anti-Examples (DON'T DO THIS)

```svelte
<!-- WRONG: Raw markup for empty panel state -->
<div class="editor-empty">
  <p>Select a note or create a new one.</p>
</div>
```

```css
/* WRONG: Page-scoped .editor-empty styles that duplicate the shared component */
.editor-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-faint);
  font-size: var(--text-base);
}
```

```svelte
<!-- WRONG: Using the heavier EmptyState component for a simple "no selection" hint -->
<EmptyState
  title="No item selected"
  description="Select an item from the list to view details."
/>
```

---

## Goals

- [x] Eliminate all per-page `.editor-empty` and `.empty-editor` style blocks
- [x] Provide a lightweight alternative to `<EmptyState>` for the "no selection" pattern
- [x] Guarantee consistent message styling (--text-sm, --text-muted, centered) across all list-detail panels
- [x] Support an optional action button for "create new" shortcuts
- [x] Zero hardcoded values -- 100% token-driven

## Non-Goals

- Replacing `<EmptyState>` for actual empty-list scenarios (that component handles title + description + icon)
- Illustrations or icons in the empty panel (keep it minimal)
- Loading states (use `<LoadingSpinner>` instead)
- Error states (different concern)

---

## Allowed / Not Allowed

### ALLOWED after adoption

- Using `<EmptyPanel>` in all list-detail pages for the "no item selected" state
- Customizing the message text via the `message` prop
- Adding an action button via `actionLabel` + `onAction` for quick-create shortcuts

### NOT ALLOWED after adoption

> These rules are enforced by the adoption grep test in CI.

- `.editor-empty {` in any page-scoped `<style>` block (use `<EmptyPanel>` instead)
- `.empty-editor {` in any page-scoped `<style>` block (use `<EmptyPanel>` instead)
- Raw `<div>` + `<p>` centered placeholder markup in editor panels
- Using `<EmptyState>` for "no item selected" hints in list-detail layouts

---

## Adoption Strategy

### Progressive Adoption

1. Component is created with tests
2. Reference page is migrated first (proves the component works)
3. Remaining pages migrate one at a time
4. Each migration is a separate commit (easy to revert)

### Reference Page

**Page:** `/data/notes` (`packages/webui/src/routes/data/notes/+page.svelte`)
**Why this page:** Simplest empty-panel implementation -- a single `<div class="editor-empty"><p>` with basic centering CSS. No action button needed for the initial migration.

### Migration Order

| Order | Page | File | Complexity | Notes |
|-------|------|------|------------|-------|
| 1 | /data/notes | `routes/data/notes/+page.svelte` | Low | Reference implementation; raw `<p>` replacement |
| 2 | /data/organizations | `routes/data/organizations/+page.svelte` | Low | Same raw `<p>` pattern |
| 3 | /data/contacts | `routes/data/contacts/+page.svelte` | Medium | May be using `<EmptyState>` -- replace with `<EmptyPanel>` |
| 4 | /opportunities/job-descriptions | `routes/opportunities/job-descriptions/+page.svelte` | Medium | May be using `<EmptyState>` -- replace with `<EmptyPanel>` |

### Migration Checklist (per page)

- [ ] Import `EmptyPanel` from `$lib/components`
- [ ] Replace inline `<div class="editor-empty">` or `<EmptyState>` usage with `<EmptyPanel>`
- [ ] Remove page-scoped `.editor-empty` or `.empty-editor` CSS rules
- [ ] Set appropriate `message` prop for the page context
- [ ] Add `actionLabel` and `onAction` if the page had a "create new" shortcut in the empty state
- [ ] Verify visual match (before/after screenshot or manual check)
- [ ] Verify no regressions in interactive behavior
- [ ] Commit with message: `refactor(webui): migrate [page] to <EmptyPanel>`

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
  name: 'EmptyPanel',
  pattern: /\.editor-empty\s*\{|\.empty-editor\s*\{/,
  allowedIn: ['EmptyPanel.svelte'],
  message: 'Use <EmptyPanel> instead of page-scoped .editor-empty / .empty-editor styles',
}
```

### CLAUDE.md Rule

```markdown
- "No selection" panel states MUST use `<EmptyPanel>`, not inline `.editor-empty` CSS or `<EmptyState>`
```

---

## Testing

### Unit Tests

| Test | Description |
|------|-------------|
| Renders default message | `<EmptyPanel />` renders "Select an item to view details." |
| Renders custom message | `<EmptyPanel message="Pick one" />` renders "Pick one" |
| Renders action button | `<EmptyPanel actionLabel="Create" onAction={fn} />` renders a button labeled "Create" |
| Omits action button without label | `<EmptyPanel onAction={fn} />` does not render a button |
| Calls onAction on click | Clicking the action button calls the `onAction` callback |
| Centers content | Container uses flexbox centering (verified via computed styles or class) |

### Component Tests

| Test | Description |
|------|-------------|
| Visual regression | Screenshot comparison of EmptyPanel with and without action button |
| Full-height centering | EmptyPanel vertically centers when parent has a fixed height |
| Token-only styling | Computed styles use only token values, no hardcoded colors/sizes |

### Adoption Enforcement Tests

| Test | Description |
|------|-------------|
| Anti-pattern grep | Fails if `.editor-empty {` or `.empty-editor {` found outside `EmptyPanel.svelte` |
| Migration count | Tracks remaining pages to migrate (informational, not blocking) |

---

## Acceptance Criteria

- [ ] Component renders correctly with all prop combinations
- [ ] Reference page (`/data/notes`) migrated with no visual diff
- [ ] All design tokens used (no hardcoded values)
- [ ] Visually lighter than `<EmptyState>` (no card border, no title)
- [ ] Grep test added and passing
- [ ] CLAUDE.md rule added
- [ ] Component exported from `$lib/components/`

## Failure Criteria

- Component looks identical to `<EmptyState>` (should be visually lighter)
- Hardcoded color/spacing values instead of token references
- Migration breaks interactive behavior on any page
- Component API requires page-specific props (should be generic)
- Message text is not centered both vertically and horizontally in the panel
