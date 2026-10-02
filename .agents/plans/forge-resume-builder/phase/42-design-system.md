# Phase 42: Design System & CSS Variables

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-design-system-css-variables.md](../refs/specs/2026-04-03-design-system-css-variables.md)
**Depends on:** None (foundational)
**Blocks:** Spec B (page-level CSS sweep), Spec C (profile menu/theme toggle UI)
**Parallelizable with:** Phase 43 (Generic Kanban), any phase that does not modify `$lib/components/` or `+layout.svelte`

## Goal

Introduce a centralized design system for the Forge webui by creating two CSS files (`tokens.css` for semantic CSS custom properties, `base.css` for shared component classes) and a theme store (`theme.svelte.ts`), then migrate all 24 shared components under `$lib/components/` from hardcoded hex colors and duplicated CSS patterns to token-based styling. Support light and dark themes via `prefers-color-scheme` with a localStorage-driven override. No visual regression in light mode; dark mode renders with correct contrast and readable text.

## Non-Goals

- Page-level CSS sweep (`+page.svelte` route files) -- deferred to Spec B.
- Theme toggle UI (button in profile menu) -- deferred to Spec C.
- Sidebar flex-column restructuring -- owned by Spec C.
- Component library / Storybook / isolated component viewer.
- CSS preprocessor (Sass, PostCSS, Tailwind) -- plain CSS custom properties only.
- Animation tokens (transition durations/easing remain hardcoded per-component).
- Responsive breakpoints (no media query tokens; the app has no responsive behavior).
- Color palette generation (dark theme values are hand-picked, not algorithmic).

## Context

The Forge webui currently has 40+ `.svelte` files with hardcoded hex colors, duplicated font sizes, and repeated CSS patterns. Every component re-declares its own `.btn`, `.overlay`, `.field-input`, etc. There is no `$lib/styles/` directory and no central theme file. The design system spec (Spec A) addresses this by defining semantic tokens organized by purpose, extracting shared utility classes, and providing a theme store that supports light/dark/system preference with localStorage persistence.

The existing component inventory under `$lib/components/` includes: `StatusBadge`, `ConfirmDialog`, `LoadingSpinner`, `EmptyState`, `SourcesView`, `SummaryPicker`, `ChainViewModal`, `DriftBanner`, `Toast`, `ToastContainer`, 5 kanban components (`KanbanCard`, `KanbanColumn`, `KanbanBoard`, `OrgPickerModal`, `OrgDetailModal`), and 8 resume components (`OverrideBanner`, `PdfView`, `MarkdownView`, `LatexView`, `AddSectionDropdown`, `SkillsPicker`, `SourcePicker`, `DragNDropView`, `HeaderEditor`).

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Scope (shared components only) | Yes |
| 2. Technical approach (theme architecture, token naming, import order, migration strategy) | Yes |
| 3. File structure | Yes |
| 4. Token definitions (colors, dark overrides, typography, spacing, radii/shadows/z-index) | Yes |
| 5. base.css patterns (buttons, forms, modals, badges, split-panel, cards, banners, section headers) | Yes |
| 6. Theme store | Yes |
| 7. Files to create | Yes |
| 8. Files to modify | Yes |
| 9. Testing | Yes |
| 10. Acceptance criteria | Yes |
| 11. Non-goals | Acknowledged |
| 12. Dependencies | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/styles/tokens.css` | CSS custom properties: colors (light defaults + dark overrides + `data-theme` selectors), typography scale, spacing scale, radii, shadows, z-index layers |
| `packages/webui/src/lib/styles/base.css` | Shared component classes: `.btn` variants, `.form-field` / `.field-input` / `.field-select`, `.modal-overlay` / `.modal-dialog`, `.badge` / `.pill` / `.count-badge`, `.split-panel`, `.card`, `.banner-warning` / `.banner-info`, `.section-title`, `.page-title` |
| `packages/webui/src/lib/stores/theme.svelte.ts` | Theme preference store using Svelte 5 module-level `$state` runes. Reads `localStorage('forge-theme')`, applies `data-theme` on `<html>`, exports `setTheme()` and `getTheme()` |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/+layout.svelte` | Import `tokens.css`, `base.css`, and theme store. Replace hardcoded sidebar hex values with `var(--token)` references. Move `:global(body)` reset and font-family to `tokens.css`. |
| `packages/webui/src/lib/components/StatusBadge.svelte` | Replace hardcoded status color map hex values with token vars. Use `.badge` class from base.css. |
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

## Fallback Strategies

- **`$lib/styles/` import resolution:** If SvelteKit's `$lib` alias does not resolve CSS imports from `<script>` blocks, use relative paths (`../../styles/tokens.css`). Verify with `bun run check` before committing.
- **`$state` in `.svelte.ts` files:** The theme store uses module-level `$state` runes in a `.svelte.ts` file. File must be `.svelte.ts` for Svelte 5 module-level runes. If the Svelte compiler does not process `.svelte.ts` files in the project's current config, rename to a plain `.ts` file and use a simple variable with manual reactivity (no runes). Test with `bun run check`.
- **CSS specificity conflicts:** If global `.btn` from `base.css` collides with component-scoped `.btn`, Svelte's scoped styles have higher specificity by default. If not, prefix base classes with `.forge-` (e.g., `.forge-btn`). Check by inspecting rendered elements in DevTools.
- **Dark theme contrast:** If dark theme values produce insufficient contrast (< 4.5:1 WCAG AA), adjust individual token values. The spec values are hand-picked but may need fine-tuning.
- **Import order sensitivity:** `tokens.css` must be imported before `base.css` because `base.css` references custom properties. If Vite reorders imports, use a single combined `design-system.css` that concatenates both.

---

## Tasks

### T42.1: Create `tokens.css` [CRITICAL]

**File:** `packages/webui/src/lib/styles/tokens.css`

Creates the complete CSS custom properties file with light defaults, dark overrides via `prefers-color-scheme`, and explicit `data-theme` attribute selectors for user override. Includes colors, typography, spacing, radii, shadows, and z-index layers.

```css
/* ================================================================
   Forge Design System — Semantic Tokens
   ================================================================
   Light defaults on :root.
   Dark overrides via @media (prefers-color-scheme: dark).
   User override via html[data-theme="light"|"dark"].
   ================================================================ */

/* ---- Global reset (moved from +layout.svelte) ---- */
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family: var(--font-sans);
  color: var(--text-primary);
  background: var(--color-bg);
}

/* ================================================================
   Light theme (default)
   ================================================================ */
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

  /* --- Typography --- */
  --font-sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
    Oxygen, Ubuntu, Cantarell, sans-serif;
  --font-mono: 'SF Mono', 'Fira Code', 'Fira Mono', Menlo, Consolas, monospace;

  --text-xs: 0.7rem;     /* 11.2px -- collapsed labels, tiny badges */
  --text-sm: 0.8rem;     /* 12.8px -- section titles, field labels, small buttons */
  --text-base: 0.875rem; /* 14px   -- body text, standard buttons, inputs */
  --text-lg: 1rem;       /* 16px   -- card titles, dialog body */
  --text-xl: 1.125rem;   /* 18px   -- dialog headings, section headers */
  --text-2xl: 1.5rem;    /* 24px   -- page titles */

  --font-normal: 400;
  --font-medium: 500;
  --font-semibold: 600;
  --font-bold: 700;

  --leading-tight: 1.25;
  --leading-normal: 1.5;

  /* --- Spacing --- */
  --space-1: 0.25rem;   /*  4px */
  --space-2: 0.5rem;    /*  8px */
  --space-3: 0.75rem;   /* 12px */
  --space-4: 1rem;      /* 16px */
  --space-5: 1.25rem;   /* 20px */
  --space-6: 1.5rem;    /* 24px */
  --space-8: 2rem;      /* 32px */
  --space-10: 2.5rem;   /* 40px */
  --space-12: 3rem;     /* 48px */

  /* --- Radii --- */
  --radius-sm: 3px;
  --radius-md: 6px;
  --radius-lg: 8px;
  --radius-full: 999px;

  /* --- Shadows --- */
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.08);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.15);
  --shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.2);

  /* --- Z-index layers --- */
  --z-dropdown: 100;
  --z-modal: 10000;
  --z-toast: 10100;
}

/* ================================================================
   Dark theme (OS preference)
   ================================================================ */
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

/* ================================================================
   User override via data-theme attribute
   Higher specificity than the media query, so it wins when present.
   ================================================================ */
html[data-theme="light"] {
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

**Key points:**
- The global reset (`*, *::before, *::after { box-sizing: border-box }`) and `body` styles are moved here from `+layout.svelte`'s `:global()` blocks. This centralizes all foundational styling.
- The `body` rule uses `var(--font-sans)` and `var(--text-primary)` so it responds to theme changes.
- The `html[data-theme]` selectors repeat all values to ensure they override `@media (prefers-color-scheme: dark)` regardless of OS setting.
- Token naming follows the semantic convention from the spec: `--color-{purpose}`, `--text-{purpose}`, `--border-{purpose}`, etc.

**Acceptance criteria:**
- File exists at `packages/webui/src/lib/styles/tokens.css`.
- All tokens from Spec A sections 4.1--4.5 are present.
- Light values are `:root` defaults.
- Dark values appear in both `@media (prefers-color-scheme: dark)` and `html[data-theme="dark"]`.
- Light values appear in `html[data-theme="light"]`.
- `body` font-family and color use token vars.
- No CSS parsing errors when loaded in a browser.

**Failure criteria:**
- Token names do not match the spec convention.
- `html[data-theme]` selectors missing (user override would not work).
- Dark theme values missing for any token defined in light.

---

### T42.2: Create `base.css` [CRITICAL]

**File:** `packages/webui/src/lib/styles/base.css`

Creates shared component classes extracted from repeated patterns across 24 components. All values reference tokens from `tokens.css`.

```css
/* ================================================================
   Forge Design System -- Base Classes
   ================================================================
   Shared utility classes for buttons, form inputs, modals,
   badges/pills, split-panel layouts, cards, and banners.
   Imported after tokens.css in +layout.svelte.
   ================================================================ */

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

/* Semantic aliases */
.btn-save { /* alias for primary -- semantic name for form context */ }
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

/* ---- Section header ---- */
.section-title {
  font-size: var(--text-sm);
  font-weight: var(--font-semibold);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
```

**Key points:**
- All values reference `var(--token)` from `tokens.css` -- no hardcoded hex values.
- `.btn-danger-ghost:hover` has one intentional one-off hex (`#fee2e2`) for a missing `danger-subtle-hover` token. The spec notes this explicitly.
- `.page-title` / `.page-subtitle` are defined for reference but applied in Spec B (page-level sweep).
- Semantic aliases (`.btn-save`, `.btn-delete`, etc.) are empty rulesets that serve as documentation markers.

**Acceptance criteria:**
- File exists at `packages/webui/src/lib/styles/base.css`.
- All patterns from Spec A sections 5.1--5.9 are present.
- All values (except the one noted one-off) reference CSS custom properties.
- No CSS parsing errors.

**Failure criteria:**
- Hardcoded hex values beyond the one documented exception.
- Missing class from spec (e.g., `.modal-dialog--confirm` not defined).
- Class names conflict with Svelte scoped styles in unexpected ways.

---

### T42.3: Create Theme Store [IMPORTANT]

**File:** `packages/webui/src/lib/stores/theme.svelte.ts`

Creates the theme preference store using Svelte 5 module-level `$state` runes. Manages three states (`light`, `dark`, `system`), reads from localStorage on init, and applies `data-theme` attribute on `<html>`.

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

**Key points:**
- Uses Svelte 5 module-level `$state` rune, which is valid in `.svelte.ts` files processed by the Svelte compiler.
- `init()` runs on import. When `+layout.svelte` imports this file, the theme is applied immediately.
- `system` mode removes the `data-theme` attribute, letting `@media (prefers-color-scheme)` take over.
- `setTheme('system')` removes the localStorage key rather than storing `"system"`.
- SSR-safe: all DOM operations are gated behind `if (!browser)`.

**Acceptance criteria:**
- File exists at `packages/webui/src/lib/stores/theme.svelte.ts`.
- `getTheme()` returns `'system'` by default.
- `setTheme('dark')` sets `localStorage('forge-theme', 'dark')` and `document.documentElement.dataset.theme = 'dark'`.
- `setTheme('system')` removes `forge-theme` from localStorage and removes `data-theme` from `<html>`.
- No errors during SSR (no `document` access without `browser` guard).

**Failure criteria:**
- `init()` accesses `document` or `localStorage` during SSR.
- Setting `system` leaves stale `data-theme` attribute on `<html>`.
- `.svelte.ts` extension not recognized by the build system.

---

### T42.4: Update `+layout.svelte` [CRITICAL]

**File:** `packages/webui/src/routes/+layout.svelte`

Import the design system files and theme store. Remove the `:global()` reset and body styles (moved to `tokens.css`). Replace all hardcoded hex colors in the sidebar `<style>` block with token references.

**Changes to `<script>` block -- add imports at the top:**

```svelte
<script lang="ts">
  import '$lib/styles/tokens.css'
  import '$lib/styles/base.css'
  import '$lib/stores/theme.svelte.ts'  // initializes theme on import
  import { page } from '$app/state'
  import { ToastContainer } from '$lib/components'
  // ... rest of existing imports unchanged
</script>
```

**Changes to `<style>` block -- remove global reset and replace hex values:**

Remove:
```css
  :global(*, *::before, *::after) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
      Oxygen, Ubuntu, Cantarell, sans-serif;
    color: #1a1a1a;
    background: #f5f5f5;
  }
```

Replace sidebar hex values:

| Old value | Token replacement |
|-----------|-------------------|
| `background: #1a1a2e` | `background: var(--color-sidebar-bg)` |
| `color: #e0e0e0` | `color: var(--color-sidebar-text)` |
| `border-bottom: 1px solid rgba(255, 255, 255, 0.1)` | `border-bottom: 1px solid var(--color-sidebar-border)` |
| `color: #fff` (logo h2) | `color: var(--color-sidebar-text-active)` |
| `color: #b0b0c0` (top-link, group-label) | `color: var(--color-sidebar-text)` |
| `background: rgba(255, 255, 255, 0.05)` (hover) | `background: var(--color-sidebar-hover-bg)` |
| `color: #fff` (hover) | `color: var(--color-sidebar-text-hover)` |
| `background: rgba(255, 255, 255, 0.1)` (active) | `background: var(--color-sidebar-active-bg)` |
| `border-left: 3px solid #6c63ff` (active indicator) | `border-left: 3px solid var(--color-sidebar-accent)` |
| `border-top: 1px solid rgba(255, 255, 255, 0.05)` (nav-group) | `border-top: 1px solid var(--color-sidebar-border)` |
| `color: #9090a8` (group-children a) | `color: var(--color-sidebar-text)` |
| `color: #d0d0e0` (group-children a:hover) | `color: var(--color-sidebar-text-hover)` |

**Acceptance criteria:**
- `tokens.css` is imported before `base.css` in the `<script>` block.
- Theme store is imported (initializes on import).
- No `:global(*)` or `:global(body)` rules remain in this file.
- Zero hardcoded hex colors remain in the `<style>` block.
- Sidebar appearance is visually identical in light mode.

**Failure criteria:**
- Import order wrong (base.css before tokens.css).
- `:global()` reset still present (would duplicate the tokens.css version).
- Hardcoded hex colors remain in sidebar styles.

---

### T42.5: Update `StatusBadge.svelte` [IMPORTANT]

**File:** `packages/webui/src/lib/components/StatusBadge.svelte`

Replace the hardcoded color map with token-derived values and use the `.badge` base class. The `colorMap` still uses direct hex values for status-specific colors because these are dynamic per-status (set via inline `style:background`), but the surrounding badge styles (padding, border-radius, font-size, font-weight) come from the base `.badge` class.

**Updated component:**

```svelte
<script lang="ts">
  let { status }: { status: string } = $props()

  const colorMap: Record<string, string> = {
    draft: 'var(--text-muted)',
    approved: 'var(--color-success)',
    in_review: 'var(--color-warning)',
    pending_review: 'var(--color-warning)',
    rejected: 'var(--color-danger)',
    deriving: 'var(--color-info)',
    final: '#8b5cf6',
    archived: 'var(--text-faint)',
  }

  const labelMap: Record<string, string> = {
    draft: 'Draft',
    approved: 'Approved',
    in_review: 'In Review',
    pending_review: 'Pending Review',
    rejected: 'Rejected',
    deriving: 'Deriving',
    final: 'Final',
    archived: 'Archived',
  }

  let color = $derived(colorMap[status] ?? 'var(--text-muted)')
  let label = $derived(labelMap[status] ?? status)
  let pulsing = $derived(status === 'deriving')
</script>

<span
  class="badge"
  class:pulsing
  style:background={color}
>
  {label}
</span>

<style>
  .pulsing {
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }
</style>
```

**Key points [IMPORTANT]:**
- The `.badge` class from `base.css` provides all structural styles (padding, border-radius, font-size, font-weight, color). The component's `<style>` block only retains the `.pulsing` animation which is unique to this component.
- The `colorMap` values use `var()` references where a matching token exists. `final` status uses `#8b5cf6` because there is no purple token in the system (it is a legacy status being removed in Phase 43's migration).
- Added entries for `in_review` and `archived` statuses. [GAP] The existing component has no entry for these statuses; they would fall through to the default `#6b7280`. Adding them now prepares for Phase 43's unified status model.
- `pending_review` is kept for backward compatibility until Phase 43 migration renames it.

**Acceptance criteria:**
- Badge renders with correct colors for all status values.
- `.badge` base class provides structural styling (no duplicated padding/radius/font rules in component).
- `in_review` and `archived` entries added to both maps.
- Pulsing animation still works for `deriving` status.
- No hardcoded hex values in `<style>` block.

**Failure criteria:**
- `.badge` class from base.css not applied (global class not in scope).
- Status colors do not match the token values.
- Pulsing animation removed or broken.

---

### T42.6: Update `ConfirmDialog.svelte` [IMPORTANT]

**File:** `packages/webui/src/lib/components/ConfirmDialog.svelte`

Replace local `.overlay`, `.dialog`, and `.btn` styles with base classes from `base.css` and token references. The `<script>` block is unchanged.

**Changes to markup:**

```svelte
<!-- Replace class="overlay" with class="modal-overlay" -->
<div class="modal-overlay" onclick={oncancel} role="presentation">
  <!-- Replace class="dialog" with class="modal-dialog modal-dialog--confirm" -->
  <div class="modal-dialog modal-dialog--confirm" onclick={(e) => e.stopPropagation()} role="alertdialog" aria-modal="true" aria-labelledby="confirm-title" aria-describedby="confirm-message">
    <h3 id="confirm-title" class="dialog-title">{title}</h3>
    <p id="confirm-message" class="dialog-message">{message}</p>
    <div class="form-actions" style="justify-content: flex-end;">
      <button
        bind:this={cancelBtn}
        class="btn btn-ghost"
        onclick={oncancel}
      >
        {cancelLabel}
      </button>
      <button
        bind:this={confirmBtn}
        class="btn"
        class:btn-danger={destructive}
        class:btn-primary={!destructive}
        onclick={onconfirm}
      >
        {confirmLabel}
      </button>
    </div>
  </div>
</div>
```

**Changes to `<style>` block -- remove all local `.overlay`, `.dialog`, `.btn`, `.btn-cancel`, `.btn-confirm` rules. Keep only component-specific rules:**

```css
<style>
  .dialog-title {
    font-size: var(--text-xl);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    margin-bottom: var(--space-2);
  }

  .dialog-message {
    font-size: var(--text-base);
    color: var(--text-secondary);
    line-height: var(--leading-normal);
    margin-bottom: var(--space-6);
  }
</style>
```

**Acceptance criteria:**
- Overlay uses `.modal-overlay` (full-screen, centered, semi-transparent background).
- Dialog uses `.modal-dialog--confirm` (narrower, padded).
- Cancel button uses `btn btn-ghost`.
- Confirm button uses `btn btn-danger` (destructive) or `btn btn-primary` (non-destructive).
- No hardcoded hex colors in `<style>` block.
- Focus trap still works (unchanged `<script>` logic).
- Escape key still dismisses.

**Failure criteria:**
- Overlay does not cover full viewport.
- Dialog not centered.
- Button styles do not match previous appearance.
- Focus trap broken.

---

### T42.7: Update `LoadingSpinner.svelte` [MINOR]

**File:** `packages/webui/src/lib/components/LoadingSpinner.svelte`

Replace three hardcoded hex colors with token references:

| Old | New |
|-----|-----|
| `#e5e7eb` (spinner track) | `var(--color-border)` |
| `#6c63ff` (spinner active arc) | `var(--color-primary)` |
| `#6b7280` (message text) | `var(--text-muted)` |

**Acceptance criteria:**
- Spinner track color uses `var(--color-border)`.
- Active arc uses `var(--color-primary)`.
- Message text uses `var(--text-muted)`.
- Spinner animation unchanged.

**Failure criteria:**
- Hardcoded hex values remain.
- Spinner invisible in dark mode.

---

### T42.8: Update `EmptyState.svelte` [MINOR]

**File:** `packages/webui/src/lib/components/EmptyState.svelte`

Replace hardcoded colors with tokens and replace local `.action-btn` with base `btn btn-primary`.

**Acceptance criteria:**
- Heading uses `var(--text-secondary)`.
- Description uses `var(--text-muted)`.
- Action button uses `btn btn-primary` classes.
- No hardcoded hex values in `<style>` block.

**Failure criteria:**
- Action button styling regresses.

---

### T42.9: Update `SourcesView.svelte` [IMPORTANT]

**File:** `packages/webui/src/lib/components/SourcesView.svelte`

This is the largest component migration. Replace locally declared `.btn`, `.field-input`, `.overlay`, `.split-panel` patterns with base classes and token vars.

**Key replacements:**
- Local `.btn` declarations -> base `.btn` + variant classes (`.btn-primary`, `.btn-ghost`, `.btn-danger-ghost`)
- Local `.field-input` -> base `.field-input`
- Local overlay/modal styles -> `.modal-overlay` + `.modal-dialog`
- Hardcoded hex colors -> token vars throughout
- Local `.split-panel` / list-panel / editor-panel layout -> base `.split-panel` classes (if layout matches) or keep component-specific flex rules with tokenized colors

**Acceptance criteria:**
- All locally redeclared button, input, overlay, and modal styles removed from `<style>`.
- Base classes used instead.
- All hex colors replaced with token vars.
- Component renders and functions identically in light mode.

**Failure criteria:**
- Layout breaks because base class widths/padding differ from originals.
- Interactive behavior (create, edit, delete) broken.

---

### T42.10: Update `SummaryPicker.svelte` [IMPORTANT]

**File:** `packages/webui/src/lib/components/SummaryPicker.svelte`

Replace `.overlay`, `.picker-dialog`, local `.btn-*`, `.field-input` styles with base classes. Replace all hex colors with tokens.

**Key replacements:**
- `.overlay` -> `.modal-overlay`
- `.picker-dialog` -> `.modal-dialog` (with component-specific max-width override if needed)
- Local `.btn-*` -> base `.btn` + variant classes
- `.field-input` -> base `.field-input`
- Warning box hex colors -> token vars (`--color-warning-*`)

**Acceptance criteria:**
- Modal overlay and dialog use base classes.
- Button styles from base classes.
- Warning box uses token colors.
- Zero hardcoded hex in `<style>`.

**Failure criteria:**
- Picker dialog layout breaks.
- Warning box not visible.

---

### T42.11: Update `ChainViewModal.svelte` [MINOR]

**File:** `packages/webui/src/lib/components/ChainViewModal.svelte`

Replace overlay/modal styles with base classes. Replace hex colors with tokens. Keep the Sigma.js/WebGL canvas-specific layout rules in the component `<style>`.

**Acceptance criteria:**
- Overlay uses `.modal-overlay`.
- Modal container uses `.modal-dialog` (likely with width override for the graph view).
- Hex colors replaced with token vars.

**Failure criteria:**
- Chain graph canvas sizing breaks.
- Modal not centered.

---

### T42.12: Update `DriftBanner.svelte` [MINOR]

**File:** `packages/webui/src/lib/components/DriftBanner.svelte`

Replace `.drift-banner` hardcoded colors with `.banner-warning` base class + token vars. Replace `.drift-rederive` button with `btn btn-primary btn-sm`.

**Acceptance criteria:**
- Banner uses `.banner-warning` class from base.css.
- Re-derive button uses `btn btn-primary btn-sm`.
- Warning icon color uses `var(--color-warning)`.
- Zero hardcoded hex in `<style>`.

**Failure criteria:**
- Banner not visible.
- Button styling regresses.

---

### T42.13: Update `Toast.svelte` [MINOR]

**File:** `packages/webui/src/lib/components/Toast.svelte`

Replace hardcoded status color map values with token references.

**Key replacements:**
- Success toast color: `var(--color-success)`
- Error toast color: `var(--color-danger)`
- Info toast color: `var(--color-info)`
- Toast background: `var(--color-surface)`
- Toast text: `var(--text-primary)`
- Toast shadow: `var(--shadow-md)`

**Acceptance criteria:**
- Toast colors adapt to light/dark theme.
- Status-specific accent colors match the design system.
- Dismiss button uses token colors.

**Failure criteria:**
- Toasts not visible in dark mode.
- Status colors wrong.

---

### T42.14: Update `ToastContainer.svelte` [MINOR]

**File:** `packages/webui/src/lib/components/ToastContainer.svelte`

Replace hardcoded z-index with `var(--z-toast)`.

**Acceptance criteria:**
- `z-index: var(--z-toast)` used.
- Toasts render above modals.

**Failure criteria:**
- Toasts hidden behind other elements.

---

### T42.15: Update Kanban Components (3 files) [IMPORTANT]

**Files:**
- `packages/webui/src/lib/components/kanban/KanbanCard.svelte`
- `packages/webui/src/lib/components/kanban/KanbanColumn.svelte`
- `packages/webui/src/lib/components/kanban/KanbanBoard.svelte`

**KanbanCard.svelte changes:**
- Replace `border: 1px solid #e5e7eb` -> `border: 1px solid var(--color-border)`
- Replace `color: #1a1a2e` -> `color: var(--text-primary)`
- Replace `color: #6b7280` -> `color: var(--text-muted)`
- Replace `.tag-pill` styles with `.pill` base class
- Replace `.interest-badge` structural styles with `.badge` base class (keep inline `style:background`)
- Replace `.worked-badge` with token values (`background: var(--color-success-subtle)`, `color: var(--color-success-text)`)
- Replace `box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08)` -> `box-shadow: var(--shadow-sm)`
- Replace `border-radius: 6px` -> `border-radius: var(--radius-md)`
- Replace `border-radius: 3px` -> `border-radius: var(--radius-sm)`

**KanbanColumn.svelte changes:**
- Replace `background: #f9fafb` -> `background: var(--color-surface-raised)`
- Replace `background: #fff` -> `background: var(--color-surface)`
- Replace `border-bottom: 1px solid #e5e7eb` -> `border-bottom: 1px solid var(--color-border)`
- Replace `color: #1a1a2e` -> `color: var(--text-primary)`
- Replace `.column-count` styles with `.count-badge` base class
- Replace `background: #e5e7eb` -> `background: var(--color-border)`
- Replace `color: #374151` -> `color: var(--text-secondary)`
- Replace `color: #9ca3af` -> `color: var(--text-faint)`
- Replace `background: #f3f4f6` -> `background: var(--color-surface-sunken)`
- Replace `color: #6b7280` -> `color: var(--text-muted)`
- Replace `background: #d1d5db` -> `background: var(--color-border-strong)`
- Replace `border-radius: 6px` -> `border-radius: var(--radius-md)`
- Replace `border-radius: 999px` -> `border-radius: var(--radius-full)`
- Replace `border-radius: 3px` -> `border-radius: var(--radius-sm)`

**KanbanBoard.svelte changes:**
- Replace any remaining hardcoded hex colors with token vars.
- Replace `color: #6b7280` -> `color: var(--text-muted)` in empty state text.

**Acceptance criteria:**
- All three kanban components have zero hardcoded hex in `<style>` blocks.
- Kanban board renders identically in light mode.
- Dark mode: column backgrounds, card borders, text colors all adapt.
- `.pill` class used for tag pills in KanbanCard.
- `.count-badge` class used for column counts in KanbanColumn.

**Failure criteria:**
- Board layout breaks.
- Cards invisible in dark mode.
- Drag-and-drop styling broken.

---

### T42.16: Update Kanban Modal Components (2 files) [IMPORTANT]

**Files:**
- `packages/webui/src/lib/components/kanban/OrgPickerModal.svelte`
- `packages/webui/src/lib/components/kanban/OrgDetailModal.svelte`

**OrgPickerModal.svelte changes:**
- Replace overlay -> `.modal-overlay`
- Replace dialog container -> `.modal-dialog`
- Replace header -> `.modal-header` + `.modal-title`
- Replace body -> `.modal-body`
- Replace footer -> `.modal-footer`
- Replace local `.btn-*` -> base `.btn` + variant classes
- Replace `.field-input` -> base `.field-input`
- Replace all hex colors -> token vars

**OrgDetailModal.svelte changes:**
- Same pattern: overlay/dialog/header/body/footer -> base modal classes
- Form inputs -> base form classes
- Buttons -> base button classes
- Hex colors -> token vars

**Acceptance criteria:**
- Both modals use base modal classes.
- Form inputs and buttons use base classes.
- Zero hardcoded hex in `<style>` blocks.
- Modals render and function correctly (search, filter, create, update, remove).

**Failure criteria:**
- Modal not centered or not covering viewport.
- Form inputs not styled.
- Interactive behavior broken (search, tag filter, create new org, interest dropdown, notes auto-save).

---

### T42.17: Update Resume Components (8 files) [IMPORTANT]

**Files:**
- `packages/webui/src/lib/components/resume/OverrideBanner.svelte`
- `packages/webui/src/lib/components/resume/PdfView.svelte`
- `packages/webui/src/lib/components/resume/MarkdownView.svelte`
- `packages/webui/src/lib/components/resume/LatexView.svelte`
- `packages/webui/src/lib/components/resume/AddSectionDropdown.svelte`
- `packages/webui/src/lib/components/resume/SkillsPicker.svelte`
- `packages/webui/src/lib/components/resume/SourcePicker.svelte`
- `packages/webui/src/lib/components/resume/DragNDropView.svelte`
- `packages/webui/src/lib/components/resume/HeaderEditor.svelte`

**Per-component approach (apply to each):**

1. **OverrideBanner:** Replace warning colors with `--color-warning-*` tokens. Use `.banner-warning` if layout matches.
2. **PdfView:** Replace background/border hex with `var(--color-surface)`, `var(--color-border)`.
3. **MarkdownView:** Replace code block backgrounds, border colors, text colors with tokens.
4. **LatexView:** Replace background/border hex with tokens.
5. **AddSectionDropdown:** Replace dropdown background/border/shadow hex with `var(--color-surface)`, `var(--color-border)`, `var(--shadow-md)`. Replace z-index with `var(--z-dropdown)`.
6. **SkillsPicker:** Replace form input styles with base `.field-input`. Replace button styles with base `.btn` classes. Replace hex colors with tokens.
7. **SourcePicker:** Same approach as SkillsPicker.
8. **DragNDropView:** Replace drag handle/item border/background hex with tokens. Keep drag-specific cursor and animation styles.
9. **HeaderEditor:** Replace form input/button styles with base classes. Replace hex colors with tokens.

**Acceptance criteria:**
- All 9 resume component files have zero hardcoded hex in `<style>` blocks.
- Resume builder page renders and functions correctly.
- PDF/Markdown/LaTeX views readable in both themes.
- Drag-and-drop in DragNDropView still works.
- Section add dropdown positioned correctly with correct z-index.

**Failure criteria:**
- Resume editing workflow broken.
- Views unreadable in dark mode.
- Dropdown hidden behind other elements.

---

### T42.18: Verification Sweep [CRITICAL]

After all component updates, run a final verification:

1. **Grep for remaining hardcoded hex values:**
   ```bash
   grep -rn '#[0-9a-fA-F]\{3,8\}' packages/webui/src/lib/components/ --include='*.svelte' | grep '<style' -A 1000
   ```
   Expected: zero matches in `<style>` blocks (hex values in `<script>` for dynamic inline styles are acceptable).

2. **Build verification:**
   ```bash
   cd packages/webui && bun run check
   cd packages/webui && bun run build
   ```
   Both must pass with no errors.

3. **Manual dark mode test:**
   - Open DevTools, add `data-theme="dark"` to `<html>`.
   - Navigate every page that uses shared components.
   - Verify text readable, borders visible, contrast adequate.

**Acceptance criteria:**
- Zero hardcoded hex in `<style>` blocks of any `$lib/components/**/*.svelte` file.
- `bun run check` passes.
- `bun run build` passes.
- Dark mode renders correctly across all shared components.
- All token color pairs (text on surface, text on sidebar, badge text on badge bg) meet WCAG AA 4.5:1 minimum contrast ratio.

**Failure criteria:**
- Hardcoded hex values found in component styles.
- Build fails.
- Dark mode has unreadable text or invisible borders.

---

## Testing Support

### Test Fixtures

No test fixtures are needed for this phase. The design system is purely CSS/client-side with no backend or database changes. Testing is manual and checklist-driven.

### Test Cases

#### Visual Regression (Manual Checklist)

Per component, verify:

| Check | Description |
|-------|-------------|
| Light mode identical | Component renders identically to pre-migration appearance |
| Dark mode correct | Text readable, borders visible, adequate contrast |
| Hover states | Buttons, cards, links show correct hover color |
| Focus states | Form inputs show focus ring with `--color-border-focus` |
| Disabled states | Buttons show `opacity: 0.5`, `cursor: not-allowed` |
| Modal overlay | Covers full viewport, semi-transparent |
| Badge colors | Status-specific colors render correctly |

#### Functional Smoke Tests

| Test | Kind | Description |
|------|------|-------------|
| Theme toggle via DevTools | Smoke | Set `data-theme="dark"` on `<html>` -- entire UI switches without reload |
| localStorage dark override | Smoke | Set `forge-theme=dark` in localStorage, reload -- app loads dark |
| localStorage light override | Smoke | Set `forge-theme=light` with dark OS, reload -- app loads light |
| System default | Smoke | Remove `forge-theme` from localStorage, reload -- follows OS |
| No hex in styles | Doc test | `grep -rn '#[0-9a-fA-F]' packages/webui/src/lib/components/ --include='*.svelte'` returns zero `<style>` block matches |

#### Build Verification

| Test | Kind | Description |
|------|------|-------------|
| Type check | Smoke | `bun run check` passes |
| Production build | Smoke | `bun run build` succeeds |
| No CSS errors | Smoke | Browser console shows no CSS parsing errors |

### Visual Tests

No automated visual testing infrastructure exists. For a future spec, Playwright screenshot comparisons could be added. For now, manual verification per the checklist above.

---

## Documentation Requirements

- No new documentation files required (non-goal per spec).
- The spec file (`2026-04-03-design-system-css-variables.md`) serves as the design document.
- This plan file serves as the implementation reference.
- Inline CSS comments in `tokens.css` document token purpose and pixel equivalents.
- Inline CSS comments in `base.css` document which components each pattern was extracted from.
- No TSDoc changes needed (no TypeScript interface changes).

---

## Parallelization Notes

**Within this phase:**
- T42.1 (tokens.css) and T42.2 (base.css) can be developed in parallel but must be committed together -- base.css references tokens.
- T42.3 (theme store) is independent of T42.1/T42.2 and can be developed in parallel.
- T42.4 (+layout.svelte) depends on T42.1, T42.2, and T42.3 (imports all three).
- T42.5--T42.17 (component updates) all depend on T42.1 and T42.2 (reference base classes) but are independent of each other and can be developed in parallel.
- T42.18 (verification sweep) must run last.

**Recommended execution order:**
1. T42.1 + T42.2 + T42.3 (foundational files -- parallel)
2. T42.4 (+layout.svelte -- imports foundational files)
3. T42.5--T42.17 (component updates -- all parallel with each other)
4. T42.18 (final verification)

**Cross-phase:**
- This phase has no dependencies on other phases.
- Phase 43 (Generic Kanban) and any UI phase can run in parallel, but implementers should use tokens and base classes from this phase rather than introducing new hardcoded values.
- StatusBadge.svelte is also modified by Phase 43 (adds `in_review`/`archived` entries). Sequence: Phase 42 tokenizes existing entries first, then Phase 43 adds new ones. Do not parallelize StatusBadge changes.
- Spec B (page-level CSS sweep) depends on this phase completing first.
- Spec C (profile menu/theme toggle) depends on this phase for `setTheme()` from the theme store.
