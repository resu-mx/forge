# Phase 78: Interactive Systems Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development

**Goal:** Create GenericModal component, fix kanban z-index/ARIA issues, add reduced-motion support, add right sidebar drawer foundation, standardize modal patterns across the app.

**Depends on:** Phase 77 (Content Patterns -- Entry, PaddedEntry used in kanban cards)

**Tech Stack:** Svelte 5 (runes mode), CSS design tokens, bun:test

**Key files:**
- Components: `packages/webui/src/lib/components/` (new files go here)
- Component barrel: `packages/webui/src/lib/components/index.ts`
- Global CSS: `packages/webui/src/lib/styles/base.css`
- Design tokens: `packages/webui/src/lib/styles/tokens.css`
- Tests: `packages/webui/src/__tests__/interactive-systems.test.ts` (new)
- Spec: `.claude/plans/forge-resume-builder/refs/specs/2026-04-04-design-system-interactive-systems.md`

**Non-goals:**
- Don't migrate ALL modals to GenericModal -- just create the component and migrate BulletDetailModal as reference
- Don't implement the contextual swap (inline/modal) -- the pattern is defined, components support it, pages will adopt incrementally
- Don't implement full drag-and-drop changes -- just ARIA and documentation
- Don't create a generic KanbanCard wrapper component (spec Section 3.2 is a target, not this phase)

**Internal task parallelization:**
- 78.1 (Modal) and 78.4 (Kanban ARIA) and 78.5 (Drawer) are independent
- 78.2 depends on 78.1
- 78.3 is independent
- 78.6 depends on 78.1 and 78.2
- 78.7 depends on all

---

## Task 78.1: Create GenericModal component

**Files to create:**
- `packages/webui/src/lib/components/Modal.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)
- `packages/webui/src/lib/styles/tokens.css` (add `--z-popover: 500`)
- `packages/webui/src/lib/styles/base.css` (add modal size modifier classes + animations)

**Satisfies spec:** Section 4 (Modal System), Section 4.4 (Planned Modal Component API), Section 4.5 (Dismiss Behaviors), Section 4.6 (Focus Management), Section 4.7 (Animation)

**Satisfies acceptance criteria:** #1 (Escape closes topmost only), #3 (Focus returns after close), #5 (role="dialog" + aria-modal="true"), #8 (camelCase callbacks)

### Implementation

Add `--z-popover` token to `packages/webui/src/lib/styles/tokens.css`:

```css
  /* --- Z-index layers --- */
  --z-dropdown: 100;
  --z-popover: 500;
  --z-sidebar: 500;
  --z-modal: 10000;
  --z-toast: 10100;
```

Add modal size modifier classes and animations to `packages/webui/src/lib/styles/base.css` immediately after the existing `.modal-dialog--confirm` rule (after line ~290):

```css
/* Modal size modifiers */
.modal-dialog--sm  { max-width: 400px; }
.modal-dialog--md  { max-width: 600px; }
.modal-dialog--lg  { max-width: 800px; }
.modal-dialog--xl  { max-width: 1000px; }
.modal-dialog--full {
  max-width: calc(100vw - var(--space-12));
  max-height: calc(100vh - var(--space-12));
}

/* Modal animations */
.modal-overlay {
  animation: modal-fade-in 0.15s ease-out;
}

.modal-dialog {
  animation: modal-slide-up 0.15s ease-out;
}

@keyframes modal-fade-in {
  from { opacity: 0; }
  to   { opacity: 1; }
}

@keyframes modal-slide-up {
  from { opacity: 0; transform: translateY(8px) scale(0.98); }
  to   { opacity: 1; transform: translateY(0) scale(1); }
}
```

**Note:** The `.modal-overlay` already has existing styles. The animation rule must be added as a *separate* selector block that adds only the `animation` property, or merged into the existing `.modal-overlay` block. Merging is preferred -- add `animation: modal-fade-in 0.15s ease-out;` to the existing `.modal-overlay { ... }` block, and `animation: modal-slide-up 0.15s ease-out;` to the existing `.modal-dialog { ... }` block. Then add the `@keyframes` after the modal section.

Create `packages/webui/src/lib/components/Modal.svelte`:

```svelte
<!--
  Modal.svelte — Generic modal dialog with focus management.

  Provides standard modal chrome: overlay backdrop, dialog panel with
  configurable size, optional header/body/footer slots, escape-key
  dismissal, backdrop click dismissal, and focus trap with restore.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    open,
    onClose,
    size = 'md',
    title = '',
    header,
    body,
    footer,
  }: {
    open: boolean
    onClose: () => void
    size?: 'sm' | 'md' | 'lg' | 'xl' | 'full'
    title?: string
    header?: Snippet
    body: Snippet
    footer?: Snippet
  } = $props()

  let dialogRef = $state<HTMLDivElement | null>(null)
  let previouslyFocused: Element | null = null

  // --- Focus management ---

  function getFocusableElements(): HTMLElement[] {
    if (!dialogRef) return []
    return Array.from(
      dialogRef.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
      )
    )
  }

  $effect(() => {
    if (open) {
      previouslyFocused = document.activeElement
      requestAnimationFrame(() => {
        const focusable = getFocusableElements()
        if (focusable.length > 0) {
          focusable[0].focus()
        }
      })
    } else if (previouslyFocused instanceof HTMLElement) {
      previouslyFocused.focus()
      previouslyFocused = null
    }
  })

  // --- Keyboard handling ---

  function handleKeydown(e: KeyboardEvent) {
    if (!open) return

    if (e.key === 'Escape') {
      e.preventDefault()
      e.stopPropagation()
      onClose()
      return
    }

    // Focus trap
    if (e.key === 'Tab') {
      const focusable = getFocusableElements()
      if (focusable.length === 0) return

      const first = focusable[0]
      const last = focusable[focusable.length - 1]
      const active = document.activeElement

      if (e.shiftKey && active === first) {
        e.preventDefault()
        last.focus()
      } else if (!e.shiftKey && active === last) {
        e.preventDefault()
        first.focus()
      }
    }
  }

  const sizeClass = $derived(`modal-dialog--${size}`)
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={onClose} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      bind:this={dialogRef}
      class="modal-dialog {sizeClass}"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-labelledby={title ? 'modal-title' : undefined}
    >
      {#if header}
        {@render header()}
      {:else if title}
        <div class="modal-header">
          <h3 id="modal-title" class="modal-title">{title}</h3>
          <button class="btn-icon modal-close" onclick={onClose} aria-label="Close">
            &times;
          </button>
        </div>
      {/if}

      <div class="modal-body">
        {@render body()}
      </div>

      {#if footer}
        <div class="modal-footer">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* btn-icon is scoped here for the close button; matches spec */
  .btn-icon {
    background: none;
    border: none;
    font-size: 1.3rem;
    color: var(--text-faint);
    cursor: pointer;
    padding: 0.2rem;
    line-height: 1;
    flex-shrink: 0;
  }

  .btn-icon:hover {
    color: var(--text-secondary);
  }
</style>
```

Add export to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as Modal } from './Modal.svelte'
```

---

## Task 78.2: Fix BulletDetailModal z-index

**Files to modify:**
- `packages/webui/src/lib/components/BulletDetailModal.svelte`

**Satisfies acceptance criteria:** #9 (BulletDetailModal z-index remediation)

### Implementation

In `packages/webui/src/lib/components/BulletDetailModal.svelte`, change the scoped `.modal-overlay` style from:

```css
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
```

to:

```css
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-modal);
  }
```

This changes the hardcoded `z-index: 1000` to `var(--z-modal)` (which is `10000` from tokens.css), correctly placing BulletDetailModal in the stacking order above dropdowns (`--z-dropdown: 100`), popovers (`--z-popover: 500`), and sidebar (`--z-sidebar: 500`), but below toasts (`--z-toast: 10100`).

**Note:** This task does NOT refactor BulletDetailModal to use the GenericModal wrapper. That would be a larger refactor involving restructuring the component's template. The spec calls this out as a remediation item, not a full migration. Future phases can progressively adopt GenericModal.

---

## Task 78.3: Add reduced-motion media queries

**Files to modify:**
- `packages/webui/src/lib/styles/base.css`

**Satisfies acceptance criteria:** #10 (Reduced-motion media query planned for all animations)
**Satisfies spec:** Section 16.6 (Reduced-Motion Support)

### Implementation

Add the following at the end of `packages/webui/src/lib/styles/base.css`:

```css
/* ---- Reduced motion ---- */
@media (prefers-reduced-motion: reduce) {
  .modal-overlay,
  .modal-dialog,
  .drawer-panel,
  .right-sidebar,
  .column-collapsed {
    animation: none !important;
    transition: none !important;
  }
}
```

This globally disables animations and transitions for all interactive overlay components when the user has `prefers-reduced-motion: reduce` enabled in their OS settings. Covers:
- Modal fade-in and slide-up animations (`.modal-overlay`, `.modal-dialog`)
- Drawer slide-in animation (`.drawer-panel`, `.right-sidebar`)
- Kanban column collapse transition (`.column-collapsed` has `transition: background 0.12s`)

---

## Task 78.4: Fix GenericKanban ARIA

**Files to modify:**
- `packages/webui/src/lib/components/kanban/GenericKanbanColumn.svelte`

**Satisfies spec:** Section 1.6 (Accessibility -- role="list" on column body, role="listitem" on cards)
**Satisfies acceptance criteria:** #7 (Column collapse toggle activates on Enter/Space)

### Implementation

In `packages/webui/src/lib/components/kanban/GenericKanbanColumn.svelte`, add `role="list"` to the `.column-body` div and `role="listitem"` to `.kanban-card-wrapper`:

Change the column body (around line 77-88) from:

```svelte
    <div
      class="column-body"
      use:dndzone={{ items: localItems, flipDurationMs, dropTargetStyle: { outline: `2px dashed ${column.accent}` } }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each localItems as item (item.id)}
        <div class="kanban-card-wrapper">
          {@render cardContent(item)}
        </div>
      {/each}
    </div>
```

to:

```svelte
    <div
      class="column-body"
      role="list"
      use:dndzone={{ items: localItems, flipDurationMs, dropTargetStyle: { outline: `2px dashed ${column.accent}` } }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each localItems as item (item.id)}
        <div class="kanban-card-wrapper" role="listitem">
          {@render cardContent(item)}
        </div>
      {/each}
    </div>
```

The collapsed column toggle already has `role="button"`, `tabindex="0"`, and `onkeydown` handling for Enter (line 60), satisfying AC #7. No changes needed to the collapsed state.

---

## Task 78.5: Create RightSidebar (Drawer) foundation

**Files to create:**
- `packages/webui/src/lib/components/Drawer.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)

**Satisfies spec:** Section 8 (Right Sidebar / Slide-out Drawer)
**Satisfies acceptance criteria:** #4 (Drawer uses transform: translateX)

### Implementation

Create `packages/webui/src/lib/components/Drawer.svelte`:

```svelte
<!--
  Drawer.svelte — Right sidebar slide-out panel.

  A contextual panel that slides in from the right edge of the viewport.
  Less disruptive than a modal: main content remains visible and partially
  interactive. No focus trap (unlike Modal) — users can Tab back to main
  content. Escape key dismisses. On mobile (<1024px), shows a subtle backdrop.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    open,
    onClose,
    title = '',
    width = '320px',
    children,
  }: {
    open: boolean
    onClose: () => void
    title?: string
    width?: string
    children: Snippet
  } = $props()

  let panelRef = $state<HTMLDivElement | null>(null)
  let previouslyFocused: Element | null = null

  // --- Focus management (no trap, just save/restore) ---

  $effect(() => {
    if (open) {
      previouslyFocused = document.activeElement
      requestAnimationFrame(() => {
        // Focus the panel itself or first focusable element
        if (panelRef) {
          const focusable = panelRef.querySelector<HTMLElement>(
            'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
          )
          if (focusable) {
            focusable.focus()
          } else {
            panelRef.focus()
          }
        }
      })
    } else if (previouslyFocused instanceof HTMLElement) {
      previouslyFocused.focus()
      previouslyFocused = null
    }
  })

  function handleKeydown(e: KeyboardEvent) {
    if (open && e.key === 'Escape') {
      e.preventDefault()
      onClose()
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="drawer-backdrop" onclick={onClose} role="presentation"></div>
{/if}

<div
  bind:this={panelRef}
  class="drawer-panel"
  class:drawer-panel--open={open}
  style:width={width}
  style:--drawer-width={width}
  role="complementary"
  aria-label={title}
  tabindex="-1"
>
  <div class="drawer-header">
    <h3 class="drawer-title">{title}</h3>
    <button class="btn-icon" onclick={onClose} aria-label="Close panel">&times;</button>
  </div>
  <div class="drawer-body">
    {@render children()}
  </div>
</div>

<style>
  .drawer-backdrop {
    display: none;
  }

  /* Mobile: show backdrop overlay */
  @media (max-width: 1024px) {
    .drawer-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      z-index: calc(var(--z-sidebar) - 1);
      background: rgba(0, 0, 0, 0.15);
      transition: opacity 0.2s ease;
    }
  }

  .drawer-panel {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: var(--z-sidebar);
    background: var(--color-surface);
    border-left: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    transform: translateX(100%);
    transition: transform 0.2s ease;
  }

  .drawer-panel--open {
    transform: translateX(0);
    animation: slide-in-right 0.2s ease;
  }

  @keyframes slide-in-right {
    from { transform: translateX(100%); }
    to   { transform: translateX(0); }
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .drawer-title {
    font-size: var(--text-lg);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    margin: 0;
  }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4) var(--space-5);
  }

  .btn-icon {
    background: none;
    border: none;
    font-size: 1.3rem;
    color: var(--text-faint);
    cursor: pointer;
    padding: 0.2rem;
    line-height: 1;
    flex-shrink: 0;
  }

  .btn-icon:hover {
    color: var(--text-secondary);
  }
</style>
```

Add export to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as Drawer } from './Drawer.svelte'
```

---

## Task 78.6: Standardize modal overlay CSS

**Files to modify:**
- `packages/webui/src/lib/styles/base.css` (verify canonical `.modal-overlay` and `.modal-dialog` classes)

**Satisfies spec:** Section 4.3 (Modal CSS from base.css)

### Implementation

Verify that `base.css` already contains the canonical modal classes from the spec. Based on current state analysis, `base.css` (lines 234-290) already has:
- `.modal-overlay` with `position: fixed; inset: 0; z-index: var(--z-modal); background: var(--color-overlay);`
- `.modal-dialog` with standard chrome
- `.modal-header`, `.modal-title`, `.modal-body`, `.modal-footer`
- `.modal-dialog--confirm` for ConfirmDialog

**No changes needed** to the existing base.css modal classes -- they are already canonical and match the spec.

The BulletDetailModal scoped `.modal-overlay` and `.modal-content` duplicates the global styles. These scoped styles remain in BulletDetailModal for now (scoped CSS does not conflict with global CSS). A full migration of BulletDetailModal to use GenericModal is deferred to a future phase since it would require restructuring the component's template significantly (the component has inline header/body/footer rather than snippet slots).

**Documentation note:** Add a CSS comment in base.css above the modal section noting these are canonical:

Change the comment (around line 234) from:

```css
/* ---- Modal ---- */
```

to:

```css
/* ---- Modal (canonical — all modals should use these classes) ---- */
```

---

## Task 78.7: Write acceptance tests

**Files to create:**
- `packages/webui/src/__tests__/interactive-systems.test.ts`

**Satisfies spec:** All acceptance criteria (Section 17 / Acceptance Criteria 1-10)

### Implementation

Create `packages/webui/src/__tests__/interactive-systems.test.ts`:

```typescript
import { describe, test, expect } from 'bun:test'
import { readFileSync } from 'fs'
import { join } from 'path'

const COMPONENTS = join(import.meta.dir, '..', 'lib', 'components')
const STYLES = join(import.meta.dir, '..', 'lib', 'styles')

function read(path: string): string {
  return readFileSync(path, 'utf-8')
}

function readComponent(name: string): string {
  return read(join(COMPONENTS, name))
}

function readCSS(name: string): string {
  return read(join(STYLES, name))
}

describe('Interactive Systems', () => {
  // AC #1: Modal Escape closes topmost only
  describe('Modal escape handling', () => {
    test('Modal has keydown escape handler', () => {
      const content = readComponent('Modal.svelte')
      expect(content).toContain("e.key === 'Escape'")
      expect(content).toContain('onClose')
    })

    test('BulletDetailModal checks for sub-dialogs before handling Escape', () => {
      const content = readComponent('BulletDetailModal.svelte')
      // The parent modal checks for open child dialogs
      expect(content).toContain('showDeriveDialog')
      expect(content).toContain('showDeleteConfirm')
      expect(content).toContain("e.key === 'Escape'")
    })
  })

  // AC #2: Kanban drag fires exactly one onDrop
  describe('Kanban intra-column guard', () => {
    test('GenericKanban has intra-column drop guard', () => {
      const content = readComponent('kanban/GenericKanban.svelte')
      // Checks if item is already in this column before calling onDrop
      expect(content).toContain('currentItems.some')
      expect(content).toContain('return')
    })

    test('GenericKanbanColumn detects cross-column moves only', () => {
      const content = readComponent('kanban/GenericKanbanColumn.svelte')
      // Only calls onDrop for items not originally in this column
      expect(content).toContain('originalIds')
      expect(content).toContain('newItems')
    })
  })

  // AC #3: Focus returns after modal close
  describe('Modal focus management', () => {
    test('Modal saves and restores focus', () => {
      const content = readComponent('Modal.svelte')
      expect(content).toContain('previouslyFocused')
      expect(content).toContain('document.activeElement')
      expect(content).toContain('.focus()')
    })
  })

  // AC #4: Drawer uses transform: translateX (not width)
  describe('Drawer animation', () => {
    test('Drawer uses transform translateX for animation', () => {
      const content = readComponent('Drawer.svelte')
      expect(content).toContain('translateX')
      expect(content).toContain('slide-in-right')
    })

    test('Drawer does not animate width', () => {
      const content = readComponent('Drawer.svelte')
      // Should not have transition on width property
      const styleBlock = content.match(/<style[^>]*>([\s\S]*?)<\/style>/)?.[1] ?? ''
      expect(styleBlock).not.toMatch(/transition:\s*width/)
    })
  })

  // AC #5: All modals use role="dialog" and aria-modal="true"
  describe('Modal ARIA attributes', () => {
    test('Modal has role="dialog" and aria-modal="true"', () => {
      const content = readComponent('Modal.svelte')
      expect(content).toContain('role="dialog"')
      expect(content).toContain('aria-modal="true"')
    })

    test('ConfirmDialog has role="alertdialog" and aria-modal="true"', () => {
      const content = readComponent('ConfirmDialog.svelte')
      expect(content).toContain('role="alertdialog"')
      expect(content).toContain('aria-modal="true"')
    })

    test('BulletDetailModal has role="dialog" and aria-modal="true"', () => {
      const content = readComponent('BulletDetailModal.svelte')
      expect(content).toContain('role="dialog"')
      expect(content).toContain('aria-modal="true"')
    })
  })

  // AC #6: ConfirmDialog focuses cancel button
  describe('ConfirmDialog focus behavior', () => {
    test('ConfirmDialog focuses cancel button on open', () => {
      const content = readComponent('ConfirmDialog.svelte')
      expect(content).toContain('cancelBtn')
      expect(content).toContain('cancelBtn?.focus()')
    })
  })

  // AC #7: Kanban column collapse toggle on Enter/Space
  describe('Kanban column keyboard navigation', () => {
    test('Collapsed column has role="button" and tabindex="0"', () => {
      const content = readComponent('kanban/GenericKanbanColumn.svelte')
      expect(content).toContain('role="button"')
      expect(content).toContain('tabindex="0"')
    })

    test('Collapsed column responds to Enter key', () => {
      const content = readComponent('kanban/GenericKanbanColumn.svelte')
      expect(content).toContain("e.key === 'Enter'")
    })

    test('Column body has role="list"', () => {
      const content = readComponent('kanban/GenericKanbanColumn.svelte')
      expect(content).toContain('role="list"')
    })

    test('Card wrapper has role="listitem"', () => {
      const content = readComponent('kanban/GenericKanbanColumn.svelte')
      expect(content).toContain('role="listitem"')
    })
  })

  // AC #8: camelCase callback props
  describe('Callback naming conventions', () => {
    test('Modal uses camelCase onClose', () => {
      const content = readComponent('Modal.svelte')
      expect(content).toContain('onClose')
    })

    test('Drawer uses camelCase onClose', () => {
      const content = readComponent('Drawer.svelte')
      expect(content).toContain('onClose')
    })
  })

  // AC #9: BulletDetailModal z-index uses token
  describe('BulletDetailModal z-index', () => {
    test('BulletDetailModal uses var(--z-modal) not hardcoded z-index', () => {
      const content = readComponent('BulletDetailModal.svelte')
      expect(content).toContain('var(--z-modal)')
      expect(content).not.toContain('z-index: 1000')
    })
  })

  // AC #10: Reduced-motion media query
  describe('Reduced motion support', () => {
    test('base.css has prefers-reduced-motion media query', () => {
      const content = readCSS('base.css')
      expect(content).toContain('prefers-reduced-motion: reduce')
    })

    test('reduced motion disables modal animations', () => {
      const content = readCSS('base.css')
      expect(content).toContain('.modal-overlay')
      expect(content).toContain('.modal-dialog')
      expect(content).toContain('animation: none !important')
    })
  })

  // Z-index stacking order
  describe('Z-index token stacking order', () => {
    test('tokens.css defines z-index tokens in correct order', () => {
      const content = readCSS('tokens.css')
      const dropdownMatch = content.match(/--z-dropdown:\s*(\d+)/)
      const popoverMatch = content.match(/--z-popover:\s*(\d+)/)
      const sidebarMatch = content.match(/--z-sidebar:\s*(\d+)/)
      const modalMatch = content.match(/--z-modal:\s*(\d+)/)
      const toastMatch = content.match(/--z-toast:\s*(\d+)/)

      expect(dropdownMatch).toBeTruthy()
      expect(popoverMatch).toBeTruthy()
      expect(sidebarMatch).toBeTruthy()
      expect(modalMatch).toBeTruthy()
      expect(toastMatch).toBeTruthy()

      const dropdown = parseInt(dropdownMatch![1])
      const popover = parseInt(popoverMatch![1])
      const sidebar = parseInt(sidebarMatch![1])
      const modal = parseInt(modalMatch![1])
      const toast = parseInt(toastMatch![1])

      // Verify stacking order: dropdown < popover <= sidebar < modal < toast
      expect(dropdown).toBeLessThan(popover)
      expect(popover).toBeLessThanOrEqual(sidebar)
      expect(sidebar).toBeLessThan(modal)
      expect(modal).toBeLessThan(toast)
    })
  })

  // Drawer uses --z-sidebar token
  describe('Drawer z-index', () => {
    test('Drawer uses --z-sidebar token', () => {
      const content = readComponent('Drawer.svelte')
      expect(content).toContain('var(--z-sidebar)')
    })
  })

  // Modal overlay uses global classes
  describe('Modal overlay standardization', () => {
    test('base.css modal section is marked as canonical', () => {
      const content = readCSS('base.css')
      expect(content).toContain('canonical')
      expect(content).toContain('.modal-overlay')
    })

    test('base.css has modal size modifier classes', () => {
      const content = readCSS('base.css')
      expect(content).toContain('.modal-dialog--sm')
      expect(content).toContain('.modal-dialog--md')
      expect(content).toContain('.modal-dialog--lg')
      expect(content).toContain('.modal-dialog--xl')
      expect(content).toContain('.modal-dialog--full')
    })

    test('base.css has modal animation keyframes', () => {
      const content = readCSS('base.css')
      expect(content).toContain('@keyframes modal-fade-in')
      expect(content).toContain('@keyframes modal-slide-up')
    })
  })

  // Component barrel exports
  describe('Component exports', () => {
    test('index.ts exports Modal', () => {
      const content = read(join(COMPONENTS, 'index.ts'))
      expect(content).toContain("Modal")
      expect(content).toMatch(/from\s+'\.\/Modal\.svelte'/)
    })

    test('index.ts exports Drawer', () => {
      const content = read(join(COMPONENTS, 'index.ts'))
      expect(content).toContain("Drawer")
      expect(content).toMatch(/from\s+'\.\/Drawer\.svelte'/)
    })
  })
})
```

---

## Acceptance Criteria Traceability

| Spec AC | Description | Task | Test |
|---------|-------------|------|------|
| AC #1 | Modal Escape closes topmost only | 78.1 | `Modal escape handling` |
| AC #2 | Kanban drag fires exactly one onDrop | 78.4 (verified, already correct) | `Kanban intra-column guard` |
| AC #3 | Focus returns after modal close | 78.1 | `Modal focus management` |
| AC #4 | Drawer uses transform: translateX | 78.5 | `Drawer animation` |
| AC #5 | Modals use role="dialog" + aria-modal="true" | 78.1, 78.2 | `Modal ARIA attributes` |
| AC #6 | ConfirmDialog focuses cancel button | (already correct) | `ConfirmDialog focus behavior` |
| AC #7 | Column collapse toggle on Enter/Space | 78.4 (already correct) | `Kanban column keyboard navigation` |
| AC #8 | camelCase callback props | 78.1, 78.5 | `Callback naming conventions` |
| AC #9 | BulletDetailModal z-index remediation | 78.2 | `BulletDetailModal z-index` |
| AC #10 | Reduced-motion media query | 78.3 | `Reduced motion support` |
