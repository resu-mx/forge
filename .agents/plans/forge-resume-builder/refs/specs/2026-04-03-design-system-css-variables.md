# Design System & CSS Variables

**Date:** 2026-04-03
**Spec:** A (Design System)
**Phase:** TBD (next available)
**Dependencies:** None — this is foundational. Many other specs depend on it.

## Overview

The Forge webui currently has 40+ `.svelte` files with hardcoded hex colors, duplicated font sizes, and repeated CSS patterns (buttons, modals, form inputs, badges). There is no central theme file and no `$lib/styles/` directory. Every component re-declares its own `.btn`, `.overlay`, `.field-input`, etc. from scratch.

This spec introduces a two-file design system:

1. **`tokens.css`** — semantic CSS custom properties for colors, typography, spacing, radii, and shadows. Includes light and dark theme via `prefers-color-scheme`, with a localStorage-driven override via a `data-theme` attribute on `<html>`.
2. **`base.css`** — shared utility classes for buttons, form inputs, modals, badges/pills, split-panel layouts, and cards. These replace the per-component class re-declarations.

Both files are imported in the root `+layout.svelte`. Shared components under `$lib/components/` are then updated to consume tokens and base classes instead of hardcoded values. Page-level CSS sweeps are deferred to Spec B.

## 1. Scope

### In scope (Phase 1 — shared components only)

- Create `packages/webui/src/lib/styles/tokens.css`
- Create `packages/webui/src/lib/styles/base.css`
- Import both in `packages/webui/src/routes/+layout.svelte`
- Update `+layout.svelte` sidebar styles to use tokens
- Update all `$lib/components/*` to use tokens and base classes:
  - `StatusBadge.svelte`
  - `ConfirmDialog.svelte`
  - `LoadingSpinner.svelte`
  - `EmptyState.svelte`
  - `SourcesView.svelte`
  - `SummaryPicker.svelte`
  - `ChainViewModal.svelte`
  - `DriftBanner.svelte`
  - `Toast.svelte`
  - `ToastContainer.svelte`
  - `kanban/KanbanCard.svelte`
  - `kanban/KanbanColumn.svelte`
  - `kanban/KanbanBoard.svelte`
  - `kanban/OrgPickerModal.svelte`
  - `kanban/OrgDetailModal.svelte`
  - `resume/OverrideBanner.svelte`
  - `resume/PdfView.svelte`
  - `resume/MarkdownView.svelte`
  - `resume/LatexView.svelte`
  - `resume/AddSectionDropdown.svelte`
  - `resume/SkillsPicker.svelte`
  - `resume/SourcePicker.svelte`
  - `resume/DragNDropView.svelte`
  - `resume/HeaderEditor.svelte`
- Add a `themeStore` Svelte store (`$lib/stores/theme.svelte`) that reads localStorage, applies `data-theme` on `<html>`, and exports a toggle function (the toggle UI itself lives in Spec C Profile Menu)

### Not in scope

- Page-level CSS sweep (`+page.svelte` files under `routes/`) — deferred to Spec B
- Theme toggle UI (button in profile menu) — deferred to Spec C
- Component library documentation / Storybook
- CSS-in-JS or preprocessor tooling (Sass, PostCSS)

## 2. Technical Approach

### 2.1 Theme architecture

CSS custom properties are defined on `:root` (light defaults). Dark values are set in two ways:

1. **OS preference:** `@media (prefers-color-scheme: dark) { :root { ... } }` — automatic, zero-JS.
2. **User override:** `html[data-theme="dark"] { ... }` — set by the theme store from localStorage. This selector has higher specificity than the media query, so it wins when present.

The theme store (`$lib/stores/theme.svelte`) manages three states: `"light"`, `"dark"`, or `"system"` (the default — follows OS). On page load it reads `localStorage.getItem('forge-theme')`, applies `data-theme` on `<html>` if the value is `"light"` or `"dark"`, and removes the attribute for `"system"`.

### 2.2 Token naming convention

Semantic names organized by purpose:

```
--color-{purpose}          Surface/background colors
--color-{purpose}-hover    Hover variant
--text-{purpose}           Text colors
--border-{purpose}         Border colors
--shadow-{level}           Box shadows
--radius-{size}            Border radii
--space-{size}             Spacing scale
--text-{size}              Typography scale (font-size)
--font-{property}          Font family, weight
--z-{layer}                Z-index layers
```

### 2.3 Import order

In `+layout.svelte`:

```svelte
<script>
  import '$lib/styles/tokens.css'
  import '$lib/styles/base.css'
  // ... existing imports
</script>
```

`tokens.css` must come first because `base.css` references the custom properties.

### 2.4 Migration strategy for components

Each component is updated in a single pass:

1. Replace hardcoded hex values with `var(--token-name)` references.
2. Replace locally-declared `.btn`, `.overlay`, `.field-input`, etc. with the corresponding class from `base.css`. Remove the now-redundant `<style>` rules.
3. Keep component-specific layout rules (flex directions, grid definitions, widths) in the component's `<style>` block — only shared visual properties move to base.css.

## 3. File Structure

```
packages/webui/src/lib/
  styles/
    tokens.css          # CSS custom properties (colors, typography, spacing)
    base.css            # Shared component patterns (buttons, forms, modals, etc.)
  stores/
    theme.svelte        # Theme preference store (light/dark/system)
```

## 4. Token Definitions

### 4.1 Colors

```css
:root {
  /* --- Brand --- */
  --color-primary: #6c63ff;
  --color-primary-hover: #5a52e0;
  --color-primary-subtle: rgba(108, 99, 255, 0.15);

  /* --- Surfaces --- */
  --color-bg: #f5f5f5;
  --color-surface: #ffffff;
  --color-surface-raised: #f9fafb;
  --color-surface-sunken: #f3f4f6;

  /* --- Sidebar --- */
  --color-sidebar-bg: #1a1a2e;
  --color-sidebar-text: #b0b0c0;
  --color-sidebar-text-hover: #ffffff;
  --color-sidebar-text-active: #ffffff;
  --color-sidebar-border: rgba(255, 255, 255, 0.1);
  --color-sidebar-hover-bg: rgba(255, 255, 255, 0.05);
  --color-sidebar-active-bg: rgba(255, 255, 255, 0.1);
  --color-sidebar-accent: #6c63ff;

  /* --- Text --- */
  --text-primary: #1a1a2e;
  --text-secondary: #374151;
  --text-muted: #6b7280;
  --text-faint: #9ca3af;
  --text-inverse: #ffffff;

  /* --- Borders --- */
  --color-border: #e5e7eb;
  --color-border-strong: #d1d5db;
  --color-border-focus: #6c63ff;

  /* --- Status --- */
  --color-success: #22c55e;
  --color-success-subtle: #f0fdf4;
  --color-success-text: #065f46;
  --color-success-strong: #16a34a;

  --color-danger: #ef4444;
  --color-danger-hover: #dc2626;
  --color-danger-subtle: #fee2e2;
  --color-danger-text: #dc2626;

  --color-warning: #f59e0b;
  --color-warning-subtle: #fffbeb;
  --color-warning-border: #fcd34d;
  --color-warning-text: #92400e;
  --color-warning-bg: #fef3c7;

  --color-info: #3b82f6;
  --color-info-subtle: #eff6ff;
  --color-info-border: #bfdbfe;
  --color-info-text: #1e40af;

  /* --- Interactive --- */
  --color-ghost: #f3f4f6;
  --color-ghost-hover: #e5e7eb;
  --color-ghost-text: #374151;

  /* --- Overlay --- */
  --color-overlay: rgba(0, 0, 0, 0.5);

  /* --- Tags / Pills --- */
  --color-tag-bg: #e0e7ff;
  --color-tag-text: #3730a3;
  --color-tag-neutral-bg: #f3f4f6;
  --color-tag-neutral-text: #374151;

  /* --- Template highlight (golden) --- */
  --color-template-bg: #fffbeb;
  --color-template-border: #fcd34d;
  --color-template-star: #f59e0b;
}
```

> **Note:** Token values are derived from the most common hex value found in the existing codebase.

### 4.2 Dark theme overrides

```css
@media (prefers-color-scheme: dark) {
  :root {
    --color-primary: #8b83ff;
    --color-primary-hover: #a29bff;
    --color-primary-subtle: rgba(139, 131, 255, 0.2);

    --color-bg: #0f0f1a;
    --color-surface: #1a1a2e;
    --color-surface-raised: #222240;
    --color-surface-sunken: #141428;

    --color-sidebar-bg: #0d0d1a;
    --color-sidebar-text: #8888a0;
    --color-sidebar-text-hover: #e0e0f0;
    --color-sidebar-text-active: #ffffff;
    --color-sidebar-border: rgba(255, 255, 255, 0.06);
    --color-sidebar-hover-bg: rgba(255, 255, 255, 0.04);
    --color-sidebar-active-bg: rgba(255, 255, 255, 0.08);

    --text-primary: #e0e0f0;
    --text-secondary: #b0b0c8;
    --text-muted: #8888a0;
    --text-faint: #606078;
    --text-inverse: #0f0f1a;

    --color-border: #2a2a44;
    --color-border-strong: #3a3a58;
    --color-border-focus: #8b83ff;

    --color-success: #4ade80;
    --color-success-subtle: #0a2618;
    --color-success-text: #86efac;
    --color-success-strong: #22c55e;

    --color-danger: #f87171;
    --color-danger-hover: #ef4444;
    --color-danger-subtle: #2a0f0f;
    --color-danger-text: #fca5a5;

    --color-warning: #fbbf24;
    --color-warning-subtle: #2a2008;
    --color-warning-border: #854d0e;
    --color-warning-text: #fde68a;
    --color-warning-bg: #422006;

    --color-info: #60a5fa;
    --color-info-subtle: #0a1628;
    --color-info-border: #1e3a5f;
    --color-info-text: #93c5fd;

    --color-ghost: #222240;
    --color-ghost-hover: #2a2a44;
    --color-ghost-text: #b0b0c8;

    --color-overlay: rgba(0, 0, 0, 0.7);

    --color-tag-bg: #1e1e3a;
    --color-tag-text: #a5b4fc;
    --color-tag-neutral-bg: #222240;
    --color-tag-neutral-text: #b0b0c8;

    --color-template-bg: #2a2008;
    --color-template-border: #854d0e;
    --color-template-star: #fbbf24;
  }
}

/* User override via data-theme attribute (higher specificity than media query) */
html[data-theme="light"] {
  /* Light values are already the :root defaults, but we repeat them here
     so the attribute explicitly wins over a dark prefers-color-scheme media query. */
  --color-primary: #6c63ff;
  --color-primary-hover: #5a52e0;
  --color-primary-subtle: rgba(108, 99, 255, 0.15);
  --color-bg: #f5f5f5;
  --color-surface: #ffffff;
  --color-surface-raised: #f9fafb;
  --color-surface-sunken: #f3f4f6;
  --color-sidebar-bg: #1a1a2e;
  --color-sidebar-text: #b0b0c0;
  --color-sidebar-text-hover: #ffffff;
  --color-sidebar-text-active: #ffffff;
  --color-sidebar-border: rgba(255, 255, 255, 0.1);
  --color-sidebar-hover-bg: rgba(255, 255, 255, 0.05);
  --color-sidebar-active-bg: rgba(255, 255, 255, 0.1);
  --color-sidebar-accent: #6c63ff;
  --text-primary: #1a1a2e;
  --text-secondary: #374151;
  --text-muted: #6b7280;
  --text-faint: #9ca3af;
  --text-inverse: #ffffff;
  --color-border: #e5e7eb;
  --color-border-strong: #d1d5db;
  --color-border-focus: #6c63ff;
  --color-success: #22c55e;
  --color-success-subtle: #f0fdf4;
  --color-success-text: #065f46;
  --color-success-strong: #16a34a;
  --color-danger: #ef4444;
  --color-danger-hover: #dc2626;
  --color-danger-subtle: #fee2e2;
  --color-danger-text: #dc2626;
  --color-warning: #f59e0b;
  --color-warning-subtle: #fffbeb;
  --color-warning-border: #fcd34d;
  --color-warning-text: #92400e;
  --color-warning-bg: #fef3c7;
  --color-info: #3b82f6;
  --color-info-subtle: #eff6ff;
  --color-info-border: #bfdbfe;
  --color-info-text: #1e40af;
  --color-ghost: #f3f4f6;
  --color-ghost-hover: #e5e7eb;
  --color-ghost-text: #374151;
  --color-overlay: rgba(0, 0, 0, 0.5);
  --color-tag-bg: #e0e7ff;
  --color-tag-text: #3730a3;
  --color-tag-neutral-bg: #f3f4f6;
  --color-tag-neutral-text: #374151;
  --color-template-bg: #fffbeb;
  --color-template-border: #fcd34d;
  --color-template-star: #f59e0b;
}

html[data-theme="dark"] {
  /* Same values as the prefers-color-scheme: dark block above. */
  --color-primary: #8b83ff;
  --color-primary-hover: #a29bff;
  --color-primary-subtle: rgba(139, 131, 255, 0.2);
  --color-bg: #0f0f1a;
  --color-surface: #1a1a2e;
  --color-surface-raised: #222240;
  --color-surface-sunken: #141428;
  --color-sidebar-bg: #0d0d1a;
  --color-sidebar-text: #8888a0;
  --color-sidebar-text-hover: #e0e0f0;
  --color-sidebar-text-active: #ffffff;
  --color-sidebar-border: rgba(255, 255, 255, 0.06);
  --color-sidebar-hover-bg: rgba(255, 255, 255, 0.04);
  --color-sidebar-active-bg: rgba(255, 255, 255, 0.08);
  --text-primary: #e0e0f0;
  --text-secondary: #b0b0c8;
  --text-muted: #8888a0;
  --text-faint: #606078;
  --text-inverse: #0f0f1a;
  --color-border: #2a2a44;
  --color-border-strong: #3a3a58;
  --color-border-focus: #8b83ff;
  --color-success: #4ade80;
  --color-success-subtle: #0a2618;
  --color-success-text: #86efac;
  --color-success-strong: #22c55e;
  --color-danger: #f87171;
  --color-danger-hover: #ef4444;
  --color-danger-subtle: #2a0f0f;
  --color-danger-text: #fca5a5;
  --color-warning: #fbbf24;
  --color-warning-subtle: #2a2008;
  --color-warning-border: #854d0e;
  --color-warning-text: #fde68a;
  --color-warning-bg: #422006;
  --color-info: #60a5fa;
  --color-info-subtle: #0a1628;
  --color-info-border: #1e3a5f;
  --color-info-text: #93c5fd;
  --color-ghost: #222240;
  --color-ghost-hover: #2a2a44;
  --color-ghost-text: #b0b0c8;
  --color-overlay: rgba(0, 0, 0, 0.7);
  --color-tag-bg: #1e1e3a;
  --color-tag-text: #a5b4fc;
  --color-tag-neutral-bg: #222240;
  --color-tag-neutral-text: #b0b0c8;
  --color-template-bg: #2a2008;
  --color-template-border: #854d0e;
  --color-template-star: #fbbf24;
}
```

### 4.3 Typography scale

```css
:root {
  --font-sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
    Oxygen, Ubuntu, Cantarell, sans-serif;
  --font-mono: 'SF Mono', 'Fira Code', 'Fira Mono', Menlo, Consolas, monospace;

  --text-xs: 0.7rem;     /* 11.2px — collapsed labels, tiny badges */
  --text-sm: 0.8rem;     /* 12.8px — section titles, field labels, small buttons */
  --text-base: 0.875rem; /* 14px   — body text, standard buttons, inputs */
  --text-lg: 1rem;       /* 16px   — card titles, dialog body */
  --text-xl: 1.125rem;   /* 18px   — dialog headings, section headers */
  --text-2xl: 1.5rem;    /* 24px   — page titles */

  --font-normal: 400;
  --font-medium: 500;
  --font-semibold: 600;
  --font-bold: 700;

  --leading-tight: 1.25;
  --leading-normal: 1.5;
}
```

> **Note:** Typography tokens are intentional roundings to a clean scale. `1.1rem` in the codebase maps to `--text-xl: 1.125rem` — a 0.4px difference that is visually indistinguishable.

### 4.4 Spacing scale

```css
:root {
  --space-1: 0.25rem;   /*  4px */
  --space-2: 0.5rem;    /*  8px */
  --space-3: 0.75rem;   /* 12px */
  --space-4: 1rem;      /* 16px */
  --space-5: 1.25rem;   /* 20px */
  --space-6: 1.5rem;    /* 24px */
  --space-8: 2rem;      /* 32px */
  --space-10: 2.5rem;   /* 40px */
  --space-12: 3rem;     /* 48px */
}
```

### 4.5 Radii, shadows, z-index

```css
:root {
  --radius-sm: 3px;
  --radius-md: 6px;
  --radius-lg: 8px;
  --radius-full: 999px;

  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.08);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.15);
  --shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.2);

  --z-dropdown: 100;
  --z-modal: 10000;
  --z-toast: 10100;
}
```

## 5. base.css Patterns

### 5.1 Buttons

Extracted from: ConfirmDialog, SummaryPicker, EmptyState, DriftBanner, SourcesView, BulletsView, resumes/+page.svelte.

```css
/* ---- Buttons ---- */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-4);
  border: none;
  border-radius: var(--radius-md);
  font-family: inherit;
  font-size: var(--text-base);
  font-weight: var(--font-medium);
  cursor: pointer;
  transition: background 0.15s, color 0.15s, opacity 0.15s;
  white-space: nowrap;
  line-height: var(--leading-tight);
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Variants */
.btn-primary {
  background: var(--color-primary);
  color: var(--text-inverse);
}
.btn-primary:hover:not(:disabled) {
  background: var(--color-primary-hover);
}

.btn-ghost {
  background: var(--color-ghost);
  color: var(--color-ghost-text);
}
.btn-ghost:hover:not(:disabled) {
  background: var(--color-ghost-hover);
}

.btn-danger {
  background: var(--color-danger);
  color: var(--text-inverse);
}
.btn-danger:hover:not(:disabled) {
  background: var(--color-danger-hover);
}

.btn-danger-ghost {
  background: var(--color-danger-subtle);
  color: var(--color-danger-text);
}
.btn-danger-ghost:hover:not(:disabled) {
  background: #fee2e2; /* intentional one-off, danger-subtle-hover */
}

.btn-secondary {
  background: var(--color-border);
  color: var(--text-secondary);
}
.btn-secondary:hover:not(:disabled) {
  background: var(--color-border-strong);
}

.btn-save { /* alias for primary — semantic name for form context */ }
.btn-delete { /* alias for btn-danger-ghost */ }
.btn-new, .btn-edit { /* alias for btn-primary */ }

/* Sizes */
.btn-xs {
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-xs);
}
.btn-sm {
  padding: 0.375rem var(--space-3);
  font-size: var(--text-sm);
}
.btn-lg {
  padding: var(--space-3) var(--space-6);
  font-size: var(--text-lg);
}

/* Icon-only button */
.btn-icon {
  background: none;
  border: none;
  cursor: pointer;
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-muted);
  border-radius: var(--radius-sm);
}
.btn-icon:hover {
  color: var(--text-primary);
  background: var(--color-surface-sunken);
}
.btn-icon.btn-danger {
  color: var(--color-danger-text);
  background: none;
}
```

### 5.2 Form inputs

Extracted from: SummaryPicker, SourcesView, BulletsView, templates/+page.svelte, summaries/+page.svelte.

```css
/* ---- Form inputs ---- */
.form-field {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.field-label {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--text-secondary);
}

.field-input {
  padding: 0.375rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: var(--text-base);
  font-family: inherit;
  color: var(--text-primary);
  background: var(--color-surface);
}
.field-input:focus {
  outline: none;
  border-color: var(--color-border-focus);
  box-shadow: 0 0 0 2px var(--color-primary-subtle);
}

/* Select */
.field-select {
  padding: 0.375rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: var(--text-base);
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

/* Textarea */
textarea.field-input {
  resize: vertical;
  line-height: var(--leading-normal);
}

/* Form layout */
.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.form-actions {
  display: flex;
  gap: var(--space-2);
}
```

### 5.3 Modal overlay + dialog

Extracted from: ConfirmDialog, SummaryPicker, OrgDetailModal, OrgPickerModal, ChainViewModal.

```css
/* ---- Modal ---- */
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  background: var(--color-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-dialog {
  background: var(--color-surface);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  max-width: 480px;
  width: 90%;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--color-border);
}

.modal-title {
  font-size: var(--text-xl);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
}

.modal-body {
  overflow-y: auto;
  flex: 1;
  padding: var(--space-4) var(--space-5);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-5);
  border-top: 1px solid var(--color-border);
  background: var(--color-surface-raised);
  border-radius: 0 0 var(--radius-lg) var(--radius-lg);
}

/* Confirm dialog variant (narrower, centered actions) */
.modal-dialog--confirm {
  max-width: 420px;
  padding: var(--space-6);
}
```

### 5.4 Badges / Pills

Extracted from: StatusBadge, KanbanCard (tag pills, interest badges, worked badge), templates page (section tags, built-in badge).

```css
/* ---- Badge (status indicator) ---- */
.badge {
  display: inline-block;
  padding: 0.2em 0.6em;
  border-radius: var(--radius-full);
  font-size: var(--text-xs);
  font-weight: var(--font-semibold);
  color: var(--text-inverse);
  line-height: 1.4;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

/* ---- Pill (tag / label) ---- */
.pill {
  display: inline-block;
  padding: 0.05em 0.3em;
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  font-weight: var(--font-medium);
  background: var(--color-tag-bg);
  color: var(--color-tag-text);
}

.pill-neutral {
  background: var(--color-tag-neutral-bg);
  color: var(--color-tag-neutral-text);
}

/* Count badge (e.g. column count in kanban) */
.count-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.4rem;
  height: 1.4rem;
  padding: 0 var(--space-1);
  background: var(--color-border);
  color: var(--text-secondary);
  border-radius: var(--radius-full);
  font-size: var(--text-xs);
  font-weight: var(--font-semibold);
}
```

### 5.5 Split-panel layout

Extracted from: SourcesView (list-panel + editor-panel pattern), used across data pages.

```css
/* ---- Split panel ---- */
.split-panel {
  display: flex;
  gap: 0;
  min-height: 0;
}

.list-panel {
  width: 340px;
  flex-shrink: 0;
  border-right: 1px solid var(--color-border);
  overflow-y: auto;
}

.editor-panel {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-6);
}
```

### 5.6 Card

Extracted from: KanbanCard, resume list cards, template cards, summary cards.

```css
/* ---- Card ---- */
.card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: var(--space-4);
}

.card:hover {
  box-shadow: var(--shadow-sm);
}

.card-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-1);
}

.card-title {
  font-size: var(--text-base);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}

.card-meta {
  display: flex;
  gap: var(--space-2);
  flex-wrap: wrap;
  font-size: var(--text-xs);
  color: var(--text-muted);
}

/* Template-highlighted card */
.card--template {
  border-color: var(--color-template-border);
  background: var(--color-template-bg);
}
```

### 5.7 Warning / info banners

Extracted from: DriftBanner, SummaryPicker warning box, summaries page info/warning badges.

```css
/* ---- Banner ---- */
.banner-warning {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  background: var(--color-warning-subtle);
  border: 1px solid var(--color-warning-border);
  border-radius: var(--radius-md);
  color: var(--color-warning-text);
  font-size: var(--text-base);
}

.banner-info {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  background: var(--color-info-subtle);
  border: 1px solid var(--color-info-border);
  border-radius: var(--radius-md);
  color: var(--color-info-text);
  font-size: var(--text-base);
}
```

### 5.8 Page-level header (for reference — applied in Spec B)

```css
/* ---- Page header ---- */
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

### 5.9 Section header (uppercase, small, muted)

```css
/* ---- Section header ---- */
.section-title {
  font-size: var(--text-sm);
  font-weight: var(--font-semibold);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
```

## 6. Theme Store

**File:** `packages/webui/src/lib/stores/theme.svelte.ts`

> **Note:** The `theme.svelte.ts` file uses Svelte 5 module-level `$state` runes. This is valid in `.svelte.ts` files which are processed by the Svelte compiler.

```typescript
import { browser } from '$app/environment'

const STORAGE_KEY = 'forge-theme'
type ThemePreference = 'light' | 'dark' | 'system'

let preference = $state<ThemePreference>('system')

function apply(pref: ThemePreference) {
  if (!browser) return
  if (pref === 'system') {
    document.documentElement.removeAttribute('data-theme')
  } else {
    document.documentElement.setAttribute('data-theme', pref)
  }
}

function init() {
  if (!browser) return
  const stored = localStorage.getItem(STORAGE_KEY) as ThemePreference | null
  if (stored === 'light' || stored === 'dark') {
    preference = stored
    apply(stored)
  }
}

export function setTheme(pref: ThemePreference) {
  preference = pref
  if (browser) {
    if (pref === 'system') {
      localStorage.removeItem(STORAGE_KEY)
    } else {
      localStorage.setItem(STORAGE_KEY, pref)
    }
    apply(pref)
  }
}

export function getTheme(): ThemePreference {
  return preference
}

// Call on app startup (from +layout.svelte)
init()
```

The store is imported in `+layout.svelte` alongside the CSS imports:

```svelte
<script>
  import '$lib/styles/tokens.css'
  import '$lib/styles/base.css'
  import '$lib/stores/theme.svelte'  // initializes theme on import
  // ... existing imports
</script>
```

## 7. Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/styles/tokens.css` | CSS custom properties: colors, typography, spacing, radii, shadows, z-index. Light defaults + dark overrides + data-theme selectors. |
| `packages/webui/src/lib/styles/base.css` | Shared component classes: buttons, form inputs, modals, badges, pills, split-panel, cards, banners, section headers. |
| `packages/webui/src/lib/stores/theme.svelte.ts` | Theme preference store: reads localStorage, sets `data-theme` on `<html>`, exports `setTheme()` and `getTheme()`. |

## 8. Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+layout.svelte` | Import `tokens.css`, `base.css`, and theme store. Replace hardcoded sidebar hex values with `var(--token)` references. Move the `:global(body)` reset and font-family to `tokens.css`. |
| `packages/webui/src/lib/components/StatusBadge.svelte` | Replace hardcoded status colors with token-derived values. Use `.badge` class from base.css. |
| `packages/webui/src/lib/components/ConfirmDialog.svelte` | Replace `.overlay` with `.modal-overlay`, `.dialog` with `.modal-dialog--confirm`. Replace local `.btn` / `.btn-cancel` / `.btn-confirm` with base `.btn` + variant classes. Replace all hex colors with token vars. |
| `packages/webui/src/lib/components/LoadingSpinner.svelte` | Replace `#e5e7eb` with `var(--color-border)`, `#6c63ff` with `var(--color-primary)`, `#6b7280` with `var(--text-muted)`. |
| `packages/webui/src/lib/components/EmptyState.svelte` | Replace hardcoded colors with tokens. Replace local `.action-btn` with `btn btn-primary`. |
| `packages/webui/src/lib/components/SourcesView.svelte` | Replace locally declared `.btn`, `.field-input`, `.overlay` styles with base classes and token vars. |
| `packages/webui/src/lib/components/SummaryPicker.svelte` | Replace `.overlay`, `.picker-dialog`, local `.btn-*`, `.field-input` styles with base classes. Replace all hex colors with tokens. |
| `packages/webui/src/lib/components/ChainViewModal.svelte` | Replace overlay/modal styles with base classes. Replace hex colors with tokens. |
| `packages/webui/src/lib/components/DriftBanner.svelte` | Replace `.drift-banner` hardcoded colors with `.banner-warning` base class + token vars. Replace `.drift-rederive` button with `btn btn-primary btn-sm`. |
| `packages/webui/src/lib/components/Toast.svelte` | Replace hardcoded status color map values with token references (`--color-success`, `--color-danger`, `--color-info`). |
| `packages/webui/src/lib/components/ToastContainer.svelte` | Replace hardcoded z-index with `var(--z-toast)`. |
| `packages/webui/src/lib/components/kanban/KanbanCard.svelte` | Replace hex colors with tokens. Use `.pill` for tag pills. Use `.badge` for interest badges. |
| `packages/webui/src/lib/components/kanban/KanbanColumn.svelte` | Replace hex colors with tokens. Use `.count-badge` for column count. |
| `packages/webui/src/lib/components/kanban/KanbanBoard.svelte` | Replace any hardcoded hex colors with tokens. |
| `packages/webui/src/lib/components/kanban/OrgPickerModal.svelte` | Replace overlay/modal/button styles with base classes and tokens. |
| `packages/webui/src/lib/components/kanban/OrgDetailModal.svelte` | Replace overlay/modal/form/button styles with base classes and tokens. |
| `packages/webui/src/lib/components/resume/OverrideBanner.svelte` | Replace hardcoded colors with tokens. |
| `packages/webui/src/lib/components/resume/PdfView.svelte` | Replace hardcoded colors with tokens. |
| `packages/webui/src/lib/components/resume/MarkdownView.svelte` | Replace hardcoded colors with tokens. |
| `packages/webui/src/lib/components/resume/LatexView.svelte` | Replace hardcoded colors with tokens. |
| `packages/webui/src/lib/components/resume/AddSectionDropdown.svelte` | Replace hardcoded colors with tokens. |
| `packages/webui/src/lib/components/resume/SkillsPicker.svelte` | Replace form/button styles with base classes and tokens. |
| `packages/webui/src/lib/components/resume/SourcePicker.svelte` | Replace form/button styles with base classes and tokens. |
| `packages/webui/src/lib/components/resume/DragNDropView.svelte` | Replace hardcoded colors with tokens. |
| `packages/webui/src/lib/components/resume/HeaderEditor.svelte` | Replace form/button styles with base classes and tokens. |

## 9. Testing

### 9.1 Visual regression approach

There is no existing visual testing infrastructure. For this spec, testing is manual and checklist-driven. A future spec may add Playwright screenshot comparisons.

**Manual verification checklist (per component):**

1. Component renders identically in light mode (no visual diff from current hardcoded state).
2. Component renders correctly in dark mode (text is readable, contrast is adequate, borders are visible).
3. Interactive states work: hover, focus, disabled, active.
4. Modal overlays cover the full viewport and dismiss correctly.
5. Form inputs show focus ring with the correct color.
6. Status badges show correct status-specific colors.

### 9.2 Functional smoke tests

1. Toggle `data-theme="dark"` on `<html>` in DevTools — the entire UI switches to dark colors without page reload.
2. Set `localStorage.setItem('forge-theme', 'dark')` and reload — the app loads in dark mode.
3. Set `localStorage.setItem('forge-theme', 'light')` and reload with OS set to dark — the app loads in light mode (override wins).
4. Remove `forge-theme` from localStorage and reload — the app follows OS preference.
5. Verify no hardcoded hex colors remain in any `$lib/components/` `.svelte` file (grep for `#[0-9a-f]{3,8}` in `<style>` blocks should return zero matches).

### 9.3 Build verification

- `bun run check` passes (Svelte type checking).
- `bun run build` produces a working production build.
- No CSS parsing errors in the browser console.

## 10. Acceptance Criteria

1. `packages/webui/src/lib/styles/tokens.css` exists and defines all semantic tokens listed in Section 4 for both light and dark themes.
2. `packages/webui/src/lib/styles/base.css` exists and defines all shared classes listed in Section 5.
3. `packages/webui/src/lib/stores/theme.svelte.ts` exists and correctly applies `data-theme` from localStorage on page load.
4. Both CSS files are imported in `+layout.svelte` before any component code.
5. The theme store is initialized on app startup.
6. All `$lib/components/` files listed in Section 8 have been updated to use tokens and base classes.
7. Zero hardcoded hex color values remain in `<style>` blocks of `$lib/components/**/*.svelte` files.
8. Light mode renders identically to the current UI (no visual regression from the user's perspective).
9. Dark mode renders with readable text, visible borders, and correct contrast.
10. `data-theme="light"` on `<html>` overrides a dark OS preference.
11. `data-theme="dark"` on `<html>` overrides a light OS preference.
12. Removing the `data-theme` attribute falls back to OS preference.
13. `bun run check` and `bun run build` pass without errors.
14. Page-level `+page.svelte` files are NOT modified (deferred to Spec B).
15. No theme toggle UI is added (deferred to Spec C).

## 11. Non-Goals

- **Page-level CSS sweep** — Route `+page.svelte` files will be updated in Spec B. This spec only touches `$lib/components/` and `+layout.svelte`.
- **Profile toggle UI** — The light/dark/system toggle button lives in the profile menu, covered by Spec C.
- **Sidebar flex-column restructuring** — Owned by Spec C (the profile button requires `display: flex; flex-direction: column` on `.sidebar`). Spec A only tokenizes the existing sidebar styles.
- **Component library / Storybook** — No documentation or isolated component viewer.
- **CSS preprocessor** — No Sass, PostCSS, or Tailwind. Plain CSS custom properties only.
- **Animation tokens** — Transition durations and easing functions remain hardcoded per-component for now.
- **Responsive breakpoints** — No media query tokens. The existing app does not have responsive behavior.
- **Color palette generation** — Dark theme values are hand-picked for contrast, not algorithmically derived.

## 12. Dependencies

None. This spec is foundational and has no dependencies on other specs.

**Specs that depend on this one:**
- Spec B (Page-level CSS sweep) — uses tokens.css and base.css to update route pages.
- Spec C (Profile Menu) — uses `setTheme()` from the theme store to build the toggle UI.
- Any future spec adding new components should consume tokens and base classes rather than introducing new hardcoded values.
