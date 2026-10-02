# Forge Design System: Interactive Systems

**Date:** 2026-04-04
**Doc:** 5 of 6 (Design System Series)
**Status:** Reference specification
**Depends on:** Doc 1 (Foundation), Doc 2 (Layout & Containers), Doc 4 (Content Patterns)

This document specifies the interactive system components: kanban boards, modals, slide-out drawers, drag-and-drop integration, and the contextual inline/modal rendering pattern. These components handle complex user interactions -- focus management, drag gestures, overlay stacking, and keyboard navigation -- and have strict accessibility requirements.

---

## 1. KanbanBoard

**Layer:** View
**File:** `$lib/components/kanban/GenericKanban.svelte` (current implementation)

The KanbanBoard is a horizontal scrolling layout of status columns. It receives a flat list of items and groups them into columns based on status mapping. The board owns drag-and-drop coordination between columns and provides filter/sort extensibility through snippet slots.

### 1.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `columns` | `ColumnDef[]` | required | Status column definitions (key, label, statuses, accent) |
| `items` | `T[]` | required | Flat array of items; `T extends { id: string; status: string }` |
| `onDrop` | `(itemId: string, newStatus: string) => Promise<void>` | required | Callback when a card is dropped into a new column |
| `loading` | `boolean` | `false` | Show loading spinner instead of columns |
| `loadingMessage` | `string` | `'Loading...'` | Message displayed during loading state |
| `emptyMessage` | `string` | `'No items yet.'` | Message when items array is empty |
| `filterBar` | `Snippet` | `undefined` | Optional filter bar rendered above columns |
| `cardContent` | `Snippet<[T]>` | required | Render function for card body content |
| `defaultCollapsed` | `string` | `''` | Column key to collapse by default on mount |
| `sortItems` | `(a: T, b: T) => number` | alphabetical | Custom sort for items within each column |

### 1.2 ColumnDef Interface

```typescript
interface ColumnDef {
  /** Unique key for this column (used as drop target identifier) */
  key: string
  /** Display label shown in column header */
  label: string
  /** Status values that map to this column (multiple statuses can map to one column) */
  statuses: string[]
  /** Status to set when dropping into this column (defaults to first entry in statuses[]) */
  dropStatus?: string
  /** Accent color (CSS color value) for column header top border */
  accent: string
}
```

### 1.3 Layout CSS

```css
.board-columns {
  display: flex;
  gap: var(--space-3);
  padding: var(--space-4);
  flex: 1;
  align-items: flex-start;
  overflow-x: auto;            /* horizontal scroll when columns exceed viewport */
}

.board-filter-bar {
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--color-border);
}
```

The board fills its parent container's height. When used inside a `PageWrapper` with `overflow="hidden"`, the board stretches to the available viewport height and columns scroll independently.

### 1.4 Optimistic Update Pattern

Drops use optimistic UI: the card moves immediately to the target column, and the `onDrop` callback fires asynchronously. On failure, the card reverts to its original column. This prevents perceptible lag on successful drops.

```
User drags card -> Card appears in new column immediately
                -> onDrop(itemId, newStatus) fires
                -> Success: server confirms, no visual change needed
                -> Failure: card reverts to original column, toast shows error
```

### 1.5 Usage Example

```svelte
<PageWrapper overflow="hidden">
  <GenericKanban
    columns={statusColumns}
    items={bullets}
    onDrop={handleStatusChange}
    defaultCollapsed="archived"
    sortItems={(a, b) => a.content.localeCompare(b.content)}
  >
    {#snippet filterBar()}
      <BulletFilterBar bind:search bind:domain />
    {/snippet}
    {#snippet cardContent(item)}
      <BulletKanbanCard bullet={item} onclick={() => openDetail(item)} />
    {/snippet}
  </GenericKanban>
</PageWrapper>
```

### 1.6 Accessibility

- Each column is a landmark region with `role="list"` semantics
- Cards within columns have implicit `role="listitem"` via the dnd-zone
- Column collapse toggle is a `role="button"` with `tabindex="0"` and Enter/Space activation
- Drop targets receive a visible outline (`2px dashed {accent}`) when a card is dragged over them
- Screen reader announcements for drag start, drag over, and drop complete are handled by `svelte-dnd-action`'s built-in ARIA live region

---

## 2. KanbanColumn

**Layer:** Component
**File:** `$lib/components/kanban/GenericKanbanColumn.svelte` (current implementation)

A single status column with a sticky header and scrollable card body. The column handles its own drag-and-drop zone and communicates drops to the parent board.

### 2.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `column` | `ColumnDef` | required | Column definition (key, label, accent) |
| `items` | `(T & { id: string })[]` | required | Items to display in this column |
| `cardContent` | `Snippet<[T]>` | required | Render function for each card |
| `collapsed` | `boolean` | `false` | Whether column is in collapsed state |
| `onToggleCollapse` | `() => void` | `undefined` | Callback to toggle collapse state |
| `onDrop` | `(itemId: string) => void` | required | Callback when a card is dropped into this column |

### 2.2 Column States

**Expanded (default):**
- Width: `280px` (flex: `0 0 auto`, min: `220px`, max: `320px`)
- Header: sticky at top with label, count badge, collapse button
- Body: vertical card stack with `min-height: 60px` for empty drop target
- Accent: 3px top border in `column.accent` color

**Collapsed:**
- Width: `48px`
- Vertical label with `writing-mode: vertical-rl`
- Count badge centered
- Click or Enter to expand
- Transition: `background 0.12s`

### 2.3 Header CSS

```css
.column-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3);
  background: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
  position: sticky;
  top: 0;
  z-index: 10;
  border-radius: var(--radius-md) var(--radius-md) 0 0;
}

.column-label {
  font-size: var(--text-sm);
  font-weight: var(--font-bold);
  color: var(--text-primary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  flex: 1;
  margin: 0;
}

.column-count {
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

### 2.4 svelte-dnd-action Integration

The column body uses `svelte-dnd-action`'s `dndzone` action:

```svelte
<div
  class="column-body"
  use:dndzone={{
    items: localItems,
    flipDurationMs: 200,
    dropTargetStyle: { outline: `2px dashed ${column.accent}` }
  }}
  onconsider={handleConsider}
  onfinalize={handleFinalize}
>
```

**Events:**
- `onconsider`: Fires during drag hover. Updates `localItems` to show the placeholder card position. This is a preview -- no persistence occurs.
- `onfinalize`: Fires on drop. Updates `localItems` and calls `onDrop(itemId)` for any item not originally in this column.

**Local item copy:** The column maintains a `localItems` array synced from props via `$effect`. This is required because `svelte-dnd-action` mutates the array during drag operations. The source-of-truth `items` prop is never mutated directly.

---

## 3. KanbanCard

**Layer:** Component
**Planned file:** `$lib/components/kanban/KanbanCard.svelte` (to be extracted from domain-specific cards)

Individual card rendered inside a KanbanColumn. Uses the `.kanban-card-wrapper` container provided by the column and renders domain-specific content via the `cardContent` snippet.

### 3.1 Current Pattern

Today, kanban cards are domain-specific components (`BulletKanbanCard`, `JDKanbanCard`, `SourceKanbanCard`, etc.) rendered directly through the `cardContent` snippet. There is no generic `KanbanCard` wrapper component yet.

The card wrapper is provided by the column:

```css
.kanban-card-wrapper {
  margin-bottom: var(--space-1);
}
```

### 3.2 Target API (Generic KanbanCard)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `item` | `T` | required | The data item this card represents |
| `onclick` | `() => void` | `undefined` | Click handler (opens detail modal or navigates) |
| `children` | `Snippet` | required | Card body content |

### 3.3 Card Interaction Pattern

```
User clicks card body -> onclick fires
                      -> If on entity's own page: parent shows inline detail in SplitPanel
                      -> If on another page: parent opens Modal with detail component

User drags card -> svelte-dnd-action handles drag visual
               -> On drop in new column: onDrop fires with new status
               -> Card animates to new position (flipDurationMs: 200)
```

### 3.4 Card CSS

Cards should use the existing `.card` global class from `base.css` as their base, with kanban-specific overrides in scoped styles:

```css
.kanban-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  cursor: pointer;
  transition: box-shadow 0.12s, border-color 0.12s;
}

.kanban-card:hover {
  border-color: var(--color-border-strong);
  box-shadow: var(--shadow-sm);
}

/* During drag */
.kanban-card:active {
  box-shadow: var(--shadow-md);
  opacity: 0.9;
}
```

### 3.5 Accessibility

- Cards are focusable via keyboard (Tab navigates between cards in a column)
- Enter/Space activates the card's `onclick` handler
- During drag, `svelte-dnd-action` provides ARIA live region announcements
- Card content should include a descriptive `aria-label` summarizing the item

---

## 4. Modal System

**Layer:** Component
**Global CSS:** `base.css` (`.modal-overlay`, `.modal-dialog`, `.modal-header`, `.modal-body`, `.modal-footer`)
**Current implementations:** `BulletDetailModal.svelte`, `ChainViewModal.svelte`, `ConfirmDialog.svelte`, `OrgDetailModal.svelte`, `JDPickerModal.svelte`

The modal system provides overlay dialogs for focused tasks: viewing details, confirming actions, and selecting entities. Modals trap focus, dismiss on Escape, and stack above all page content.

### 4.1 Modal Sizes

| Size | `max-width` | Use Case |
|------|-------------|----------|
| `sm` | `400px` | Confirm dialogs, simple pickers |
| `md` | `600px` | Detail views, forms |
| `lg` | `800px` | Complex editors, multi-section detail views |
| `xl` | `1000px` | Wide content (chain view, comparison views) |
| `full` | `calc(100vw - var(--space-12))` | Full-screen overlays (graph view) |

Size modifiers applied via CSS classes:

```css
.modal-dialog--sm  { max-width: 400px; }
.modal-dialog--md  { max-width: 600px; }
.modal-dialog--lg  { max-width: 800px; }
.modal-dialog--xl  { max-width: 1000px; }
.modal-dialog--full {
  max-width: calc(100vw - var(--space-12));
  max-height: calc(100vh - var(--space-12));
}
```

The default `.modal-dialog` in `base.css` uses `max-width: 480px` (between `sm` and `md`), which serves as the default for modals that do not specify a size.

### 4.2 Modal Structure

```svelte
<!-- Overlay backdrop -->
<div class="modal-overlay" onclick={onClose} role="presentation">
  <!-- Dialog panel -->
  <div
    class="modal-dialog modal-dialog--{size}"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="modal-title"
  >
    <!-- Header -->
    <div class="modal-header">
      <h3 id="modal-title" class="modal-title">{title}</h3>
      <button class="btn-icon modal-close" onclick={onClose} aria-label="Close">
        &times;
      </button>
    </div>

    <!-- Body (scrollable) -->
    <div class="modal-body">
      {@render body()}
    </div>

    <!-- Footer (optional) -->
    {#if footer}
      <div class="modal-footer">
        {@render footer()}
      </div>
    {/if}
  </div>
</div>
```

### 4.3 Modal CSS (from base.css)

```css
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);          /* 10000 */
  background: var(--color-overlay);  /* rgba(0, 0, 0, 0.4) */
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
```

### 4.4 Planned Modal Component API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `open` | `boolean` | required | Controls visibility |
| `onClose` | `() => void` | required | Called on dismiss (Escape, backdrop, close button) |
| `size` | `'sm' \| 'md' \| 'lg' \| 'xl' \| 'full'` | `'md'` | Dialog width |
| `title` | `string` | `''` | Header title text |
| `header` | `Snippet` | `undefined` | Custom header content (replaces default title + close) |
| `body` | `Snippet` | required | Main content |
| `footer` | `Snippet` | `undefined` | Optional footer actions |

### 4.5 Dismiss Behaviors

| Trigger | Behavior |
|---------|----------|
| Escape key | Calls `onClose`. If a nested dialog is open (e.g., ConfirmDialog inside a detail modal), the innermost dialog handles Escape first. |
| Backdrop click | Calls `onClose`. Uses `onclick` on overlay + `e.stopPropagation()` on dialog to distinguish. |
| Close button (X) | Calls `onClose`. |
| Programmatic | Set `open = false` from parent. |

**Nested modal priority:** When a `ConfirmDialog` is open inside a detail modal, the confirm dialog's Escape handler fires first. The parent modal checks for open child dialogs before handling Escape:

```typescript
function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    // If a sub-dialog is open, let it handle Escape
    if (showDeriveDialog || showDeleteConfirm) return
    onClose()
  }
}
```

### 4.6 Focus Management

**On open:**
1. Record the previously focused element
2. Move focus to the first focusable element inside the modal (or the close button)
3. `ConfirmDialog` focuses the cancel button by default (safer for destructive actions)

**Focus trapping:**
Tab cycles within the modal's focusable elements. When the user Tabs past the last element, focus wraps to the first. When they Shift+Tab past the first, focus wraps to the last.

```typescript
// Focus trap between actionable elements
if (e.key === 'Tab') {
  const active = document.activeElement
  if (e.shiftKey && active === firstFocusable) {
    e.preventDefault()
    lastFocusable?.focus()
  } else if (!e.shiftKey && active === lastFocusable) {
    e.preventDefault()
    firstFocusable?.focus()
  }
}
```

**On close:**
Return focus to the element that was focused before the modal opened.

```typescript
let previouslyFocused: Element | null = null

$effect(() => {
  if (open) {
    previouslyFocused = document.activeElement
    requestAnimationFrame(() => firstFocusable?.focus())
  } else if (previouslyFocused instanceof HTMLElement) {
    previouslyFocused.focus()
  }
})
```

### 4.7 Animation

Modals use CSS transitions for enter/exit:

```css
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

---

## 5. ConfirmDialog

**Layer:** Component
**File:** `$lib/components/ConfirmDialog.svelte`

A narrow modal variant specialized for destructive confirmation prompts. Uses `role="alertdialog"` instead of `role="dialog"` to signal urgency to screen readers.

### 5.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `open` | `boolean` | required | Controls visibility |
| `title` | `string` | required | Dialog title |
| `message` | `string` | required | Explanatory message |
| `confirmLabel` | `string` | `'Delete'` | Confirm button text |
| `cancelLabel` | `string` | `'Cancel'` | Cancel button text |
| `onconfirm` | `() => void` | required | Called when confirm is clicked |
| `oncancel` | `() => void` | required | Called on cancel/dismiss |
| `destructive` | `boolean` | `true` | When true, confirm button uses `.btn-danger`; otherwise `.btn-primary` |

### 5.2 CSS

Uses the `.modal-dialog--confirm` variant from `base.css`:

```css
.modal-dialog--confirm {
  max-width: 420px;
  padding: var(--space-6);
}

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
```

### 5.3 Focus Behavior

On open, focus moves to the **cancel button** (not the confirm button). This is a deliberate safety choice: for destructive operations, the safe action should be the default keyboard target.

Focus traps between cancel and confirm buttons only:

```typescript
if (e.key === 'Tab') {
  const active = document.activeElement
  if (e.shiftKey && active === cancelBtn) {
    e.preventDefault()
    confirmBtn?.focus()
  } else if (!e.shiftKey && active === confirmBtn) {
    e.preventDefault()
    cancelBtn?.focus()
  }
}
```

### 5.4 Usage

```svelte
<ConfirmDialog
  open={showDeleteConfirm}
  title="Delete Bullet"
  message="Are you sure you want to delete this bullet? This cannot be undone."
  confirmLabel="Delete"
  onconfirm={handleDelete}
  oncancel={() => showDeleteConfirm = false}
/>
```

---

## 6. KanbanCard Detail Modal

**Layer:** Composition pattern (not a standalone component)

When a user clicks a kanban card, a detail view opens. The context determines whether this appears as a modal or inline in a split panel (see Section 7).

### 6.1 From KanbanBoard Context

When on the kanban board view, clicking a card opens a **modal** containing the entity's detail component. The modal wraps the detail component -- the detail component itself is unaware that it's inside a modal.

```svelte
<!-- In the kanban page -->
<GenericKanban {columns} {items} {onDrop}>
  {#snippet cardContent(item)}
    <BulletKanbanCard
      bullet={item}
      onclick={() => { selectedBulletId = item.id; showBulletDetail = true }}
    />
  {/snippet}
</GenericKanban>

{#if showBulletDetail && selectedBulletId}
  <BulletDetailModal
    bulletId={selectedBulletId}
    onClose={() => { showBulletDetail = false; selectedBulletId = null }}
    onUpdate={refreshData}
  />
{/if}
```

### 6.2 Detail Component Design

Detail components (e.g., `BulletDetailModal`) follow a consistent structure:

1. **Data loading:** Fetch entity data and related records on mount
2. **Editable fields:** Bound to local `$state` variables
3. **Status transitions:** Action buttons that call API and update state
4. **Save/Delete:** Footer actions with optimistic feedback
5. **Sub-dialogs:** Nested dialogs (derive, confirm delete) that stack above the detail modal

The detail component manages its own data lifecycle and does not require the parent to pass the full entity object -- only the entity ID. This enables lazy loading and ensures the component always has fresh data.

---

## 7. Inline / Modal Contextual Swap

**Layer:** Architecture pattern (ADR-006)

The same detail/editor component renders in two contexts depending on which page the user is on. This is the most important composition pattern in the Forge UI.

### 7.1 The Rule

| User is on... | Clicking an entity of the same type | Clicking an entity of a different type |
|---------------|-------------------------------------|----------------------------------------|
| Entity's own page | Shows detail **inline** in SplitPanel right pane | Opens **modal** with that entity's detail component |
| Any other page | Opens **modal** with that entity's detail component | Opens **modal** with that entity's detail component |

### 7.2 Implementation Pattern

The detail component is identical in both contexts. The parent page decides which wrapping to use:

**Inline context (entity's own page):**

```svelte
<!-- /data/bullets page -->
<SplitPanel {listWidth}>
  {#snippet list()}
    <BulletList items={bullets} onselect={(id) => selectedId = id} />
  {/snippet}
  {#snippet detail()}
    {#if selectedId}
      <BulletEditor id={selectedId} />
    {:else}
      <EmptyPanel message="Select a bullet" />
    {/if}
  {/snippet}
</SplitPanel>
```

**Modal context (from another page):**

```svelte
<!-- /resumes page, user clicks a linked bullet -->
{#if showBulletModal}
  <div class="modal-overlay" onclick={() => showBulletModal = false} role="presentation">
    <div class="modal-dialog modal-dialog--lg" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3 class="modal-title">Bullet Details</h3>
        <button class="btn-icon" onclick={() => showBulletModal = false}>&times;</button>
      </div>
      <div class="modal-body">
        <BulletEditor id={bulletId} />
      </div>
    </div>
  </div>
{/if}
```

### 7.3 Component Requirements for Dual Context

A detail component that supports dual context MUST:

1. **Not hardcode width or height.** Use `flex: 1`, `width: 100%`, or `min-width: 0` so it adapts to its container.
2. **Not assume scroll behavior.** The parent container (SplitPanel pane or modal-body) provides the scroll context.
3. **Accept `onClose` as optional.** In inline context, there is no close action. In modal context, the parent passes a close handler.
4. **Manage its own data loading.** Accept an entity ID prop, not the full entity object. This avoids stale data when switching between inline selections.

### 7.4 ChainViewModal: isModal Pattern

`ChainViewModal` demonstrates an alternative approach where the component itself handles both contexts via an `isModal` prop:

```svelte
let { isModal = true, onClose }: { isModal?: boolean; onClose?: () => void } = $props()
```

When `isModal=true`, the component renders its own overlay, backdrop, and close button. When `isModal=false`, it renders inline without modal chrome. This pattern is appropriate when the component has unique layout requirements (e.g., full-viewport graph canvas) that make external wrapping impractical.

**Preferred approach:** External wrapping (Section 7.2) is the default. Use `isModal` only when the component needs to control its own overlay behavior.

---

## 8. Right Sidebar (Slide-out Drawer)

**Layer:** Container
**Status:** Planned (not yet implemented)

A contextual panel that slides in from the right edge of the viewport. Less disruptive than a modal -- the main content remains visible and partially interactive.

**Layout architecture:** See **Doc 2 (Layout & Containers) Section 4** for the canonical layout architecture. On desktop, the right sidebar is a flex sibling inside `.app` that pushes content narrower. On mobile, it is a fixed overlay with backdrop. This section specifies only the component chrome (header, body, animation, close behavior).

### 8.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `open` | `boolean` | required | Controls visibility |
| `onClose` | `() => void` | required | Called on dismiss |
| `title` | `string` | `''` | Header title |
| `width` | `string` | `'320px'` | Panel width (matches `--right-sidebar-width` from Doc 2) |
| `children` | `Snippet` | required | Panel body content |

### 8.2 Use Cases

- **Gap Analysis panel:** Shows skill gap results alongside a resume or JD page
- **Supplementary metadata:** Additional context that doesn't warrant a full modal
- **Quick actions:** Batch operations or settings that relate to the current view

### 8.3 Component Chrome CSS

The layout positioning (flex sibling vs. fixed overlay) is handled by the root layout per Doc 2 Section 4. This CSS covers only the internal structure:

```css
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
}

.drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4) var(--space-5);
}
```

**Mobile overlay backdrop** (only on viewports < 1024px):

```css
@media (max-width: 1024px) {
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    z-index: calc(var(--z-modal) - 1);
    background: rgba(0, 0, 0, 0.15);       /* subtler than modal overlay */
    transition: opacity 0.2s ease;
  }
}
```

**Animation:**

```css
.right-sidebar {
  animation: slide-in-right 0.2s ease;
}

@keyframes slide-in-right {
  from { transform: translateX(100%); }
  to   { transform: translateX(0); }
}
```

### 8.4 Drawer vs. Modal Decision Guide

| Criterion | Use Drawer | Use Modal |
|-----------|------------|-----------|
| User needs to reference main content | Yes | No |
| Content is supplementary/contextual | Yes | No |
| Content requires full user attention | No | Yes |
| Content has form submission / confirmation | No | Yes |
| Content is a detail view for an entity | No | Yes |
| Content is a quick reference panel | Yes | No |

### 8.5 Dismiss Behaviors

Same as Modal: Escape key, backdrop click, close button. The drawer animates out (`transform: translateX(100%)`) on dismiss.

### 8.6 Accessibility

- `role="complementary"` on the drawer panel (it supplements the main content)
- `aria-label` with the drawer title for screen readers
- Focus moves to the drawer on open, returns to previous element on close
- Focus trapping is NOT used (unlike Modal). The drawer is supplementary, so users can Tab back to main content. Escape dismisses the drawer.

### 8.7 Implementation Skeleton

```svelte
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

  function handleKeydown(e: KeyboardEvent) {
    if (open && e.key === 'Escape') onClose()
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="drawer-backdrop" onclick={onClose} role="presentation"></div>
{/if}

<div
  class="drawer-panel"
  class:drawer-panel--open={open}
  style:width={width}
  role="complementary"
  aria-label={title}
>
  <div class="drawer-header">
    <h3 class="drawer-title">{title}</h3>
    <button class="btn-icon" onclick={onClose} aria-label="Close panel">&times;</button>
  </div>
  <div class="drawer-body">
    {@render children()}
  </div>
</div>
```

---

## 9. Drag-and-Drop Integration

**Library:** `svelte-dnd-action` (npm package)
**Current consumers:** `GenericKanbanColumn`, `KanbanColumn`, `DragNDropView` (resume bullet reordering)

### 9.1 Integration Pattern

`svelte-dnd-action` works via a Svelte action (`use:dndzone`) applied to a container element. It requires items to have a string `id` property at the top level.

**Setup:**

```svelte
<script>
  import { dndzone } from 'svelte-dnd-action'

  let localItems = $state([...items])

  // Sync from props
  $effect(() => { localItems = [...items] })

  function handleConsider(e: CustomEvent) {
    localItems = e.detail.items  // preview position during drag
  }

  function handleFinalize(e: CustomEvent) {
    localItems = e.detail.items  // commit position on drop
    persistNewOrder(localItems)
  }
</script>

<div
  use:dndzone={{ items: localItems, flipDurationMs: 200 }}
  onconsider={handleConsider}
  onfinalize={handleFinalize}
>
  {#each localItems as item (item.id)}
    <div>{item.name}</div>
  {/each}
</div>
```

### 9.2 Key Requirements

1. **Items must have `id: string`.** If the source data uses a different ID field (e.g., `uuid`), map it:
   ```typescript
   const dndItems = items.map(o => ({ ...o, id: o.uuid }))
   ```

2. **Local mutable copy.** The `dndzone` action mutates the array during drag. Never pass reactive props directly. Always use a `$state` copy synced via `$effect`.

3. **`flipDurationMs`.** Controls the card animation duration when items reorder. Use `200` for standard cards, `150` for compact lists.

4. **`dropTargetStyle`.** Visual indicator when a drop zone is active:
   ```typescript
   dropTargetStyle: { outline: `2px dashed ${accentColor}` }
   ```

5. **Deferred persistence.** After `onfinalize`, persist the new order. For resume bullet reordering, use `setTimeout(() => onUpdate(), 50)` to let `svelte-dnd-action` finish DOM cleanup before triggering a re-render.

### 9.3 Kanban Cross-Column Drag

In the kanban context, each column is an independent `dndzone`. When a card is dragged from Column A to Column B:

1. Column A's `onconsider` fires -- the card is removed from A's `localItems`
2. Column B's `onconsider` fires -- the card is added to B's `localItems`
3. Column B's `onfinalize` fires -- detects the card is new (not in original items), calls `onDrop(itemId)`
4. The board-level `handleDrop` sets the new status and calls the API

Detection of cross-column moves:

```typescript
function handleFinalize(e: CustomEvent) {
  localItems = e.detail.items
  const originalIds = new Set(items.map(i => i.id))
  const newItems = localItems.filter(i => !originalIds.has(i.id))
  for (const item of newItems) {
    onDrop(item.id)
  }
}
```

### 9.4 Resume Bullet Reordering

In the resume `DragNDropView`, drag-and-drop operates within a single section's subheading. Each subheading maintains its own `dndzone` for bullet ordering.

After reorder:
1. Extract the new positions from array index
2. Fire `forge.resumes.updateEntry(resumeId, entryId, { position: index })` for each moved bullet
3. Defer `onUpdate()` with `setTimeout` to avoid race conditions with DOM cleanup

### 9.5 Drag Visual Feedback

During an active drag:
- The dragged item has `box-shadow: var(--shadow-md)` and slight opacity reduction
- Drop targets show a dashed outline in the column's accent color
- Items in the target zone animate apart to show the insertion point (handled by `svelte-dnd-action`'s flip animation)
- The cursor changes to `grabbing`

### 9.6 Accessibility for Drag-and-Drop

`svelte-dnd-action` provides built-in keyboard DnD support:

- **Space/Enter** on a focused item starts drag mode
- **Arrow keys** move the item within the zone
- **Tab** moves between zones
- **Space/Enter** drops the item
- **Escape** cancels the drag

An ARIA live region announces drag state changes:
- "Picked up item [label]. Current position: [n] of [total]"
- "Moved item [label] to position [n]"
- "Dropped item [label] at position [n]"
- "Drag cancelled"

---

## 10. Entry and PaddedEntry

See **Doc 4 (Content Patterns) Sections 1-2** for Entry and PaddedEntry specifications. These components are content patterns, not interactive systems. Doc 4 is the canonical home for their API, CSS, and usage guidance.

---

## 11. SectionedList

See **Doc 4 (Content Patterns) Section 3** for SectionedList specification.

---

## 12. EmptyState and EmptyPanel

**Layer:** Component
**Files:** `$lib/components/EmptyState.svelte`, `$lib/components/EmptyPanel.svelte`

### 12.1 EmptyState

Centered message with an optional call-to-action button. Used in main content areas when no data exists.

```css
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-12);
  text-align: center;
  color: var(--text-muted);
  gap: var(--space-4);
}

.empty-state__message {
  font-size: var(--text-base);
  line-height: var(--leading-normal);
}

.empty-state__cta {
  margin-top: var(--space-2);
}
```

### 12.2 EmptyPanel

Specialized empty state for the detail pane of a SplitPanel. Lighter styling, no CTA. Displays a message like "Select an item to view details."

```css
.empty-panel {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-faint);
  font-size: var(--text-sm);
  font-style: italic;
}
```

---

## 13. Keyboard Navigation Reference

### 13.1 Global Shortcuts

| Key | Context | Action |
|-----|---------|--------|
| `Escape` | Modal open | Close innermost modal |
| `Escape` | Drawer open | Close drawer |
| `Escape` | Dropdown open | Close dropdown |
| `Tab` | Modal open | Cycle within modal focus trap |
| `Tab` | Drawer open | Move between drawer and main content (no trap) |

### 13.2 Kanban Navigation

| Key | Context | Action |
|-----|---------|--------|
| `Tab` | Board | Move focus between cards |
| `Enter` / `Space` | Card focused | Open card detail |
| `Enter` / `Space` | Collapsed column | Expand column |
| `Space` | Card focused (with dnd) | Start drag mode |
| `Arrow Up/Down` | Drag mode | Move card within column |
| `Tab` | Drag mode | Move card to adjacent column |
| `Escape` | Drag mode | Cancel drag |

### 13.3 Modal Navigation

| Key | Context | Action |
|-----|---------|--------|
| `Tab` | Modal body | Cycle forward through focusable elements |
| `Shift+Tab` | Modal body | Cycle backward through focusable elements |
| `Escape` | Modal | Close modal (innermost first) |
| `Enter` | ConfirmDialog | Activate focused button |

---

## 14. Z-Index Stacking Order

All interactive overlay components participate in a defined stacking order:

```
var(--z-toast)                 10100    Toast notifications (always on top)
var(--z-modal)                 10000    Modal dialogs, drawer panels
calc(var(--z-modal) - 1)        9999    Drawer backdrop
var(--z-popover)                  500   Popovers, combobox dropdowns (between z-dropdown and z-modal)
var(--z-dropdown)                 100   Dropdowns, select popover
Column header (sticky)             10   KanbanColumn header
Page content                        0   Base layer
```

Nested modals (e.g., ConfirmDialog inside a detail modal) both render at `z-index: var(--z-modal)`. DOM order ensures the later-rendered element stacks on top.

---

## 15. Do / Don't Reference

### Modals

| Do | Don't |
|----|-------|
| Use `role="dialog"` and `aria-modal="true"` | Omit ARIA attributes on modal containers |
| Trap focus inside modal content | Let Tab escape to page content behind the modal |
| Return focus to trigger element on close | Leave focus on the now-hidden modal |
| Use `.modal-overlay` from `base.css` | Create one-off overlay styles per component |
| Use size modifier classes (`--sm`, `--lg`) | Hardcode `max-width` in component styles |
| Check for nested dialogs before handling Escape | Blindly close on any Escape press |

### Kanban

| Do | Don't |
|----|-------|
| Use a local `$state` copy for dnd items | Pass reactive `$props` directly to `dndzone` |
| Sync local items from props via `$effect` | Manually track item changes outside the effect system |
| Use optimistic updates with revert on failure | Wait for API response before moving the card |
| Set `flipDurationMs` for smooth animations | Set `flipDurationMs: 0` (causes jarring snaps) |
| Provide `dropTargetStyle` for visual feedback | Leave drop targets visually indistinguishable |

### Drag-and-Drop

| Do | Don't |
|----|-------|
| Ensure items have `id: string` at top level | Rely on nested ID fields or numeric IDs |
| Defer `onUpdate` after finalize with `setTimeout` | Call `onUpdate` synchronously in `onfinalize` |
| Use `svelte-dnd-action`'s built-in ARIA | Build custom ARIA live regions for drag state |

### Drawer

| Do | Don't |
|----|-------|
| Use subtler backdrop (`rgba(0,0,0,0.15)`) than modals | Use the same heavy overlay as modals |
| Allow Tab to escape the drawer | Trap focus inside the drawer |
| Use `role="complementary"` | Use `role="dialog"` (drawers are supplementary, not blocking) |
| Animate with `transform: translateX` | Animate with `width` (causes reflow) |

---

## 16. Known Gaps and Debt

### 16.1 GenericKanban Missing `overflow-x: auto`

The `.board-columns` container has `overflow-x: auto` in the spec (Section 1.3), but `GenericKanban.svelte` may not consistently apply this when nested inside certain PageWrapper configurations. Verify that horizontal scrolling works when column count exceeds the viewport width.

### 16.2 BulletDetailModal `z-index: 1000`

`BulletDetailModal.svelte` currently uses a hardcoded `z-index: 1000` instead of `var(--z-modal)` (which is `10000`). This is a remediation item -- the component should use the token to participate in the stacking order correctly.

### 16.3 BulletDetailModal `onClose` Required

`BulletDetailModal.onClose` is currently required (the component always renders a close button), which violates ADR-006's guidance that `onClose` should be optional for inline context. This is a known debt item -- the component should be refactored to make `onClose` optional, hiding the close button when not provided.

### 16.4 Focus Management

Focus management (trapping, restoration) as described in Section 4.6 is planned but not yet fully implemented across all modal components. `ConfirmDialog` has basic focus trapping. `BulletDetailModal` and `OrgDetailModal` rely on the browser's default focus behavior. Full focus trap and restoration should be implemented as part of the GenericModal extraction.

### 16.5 GenericModal (Planned)

A `GenericModal.svelte` component should be extracted to `$lib/components/GenericModal.svelte` to provide the standard modal chrome (overlay, dialog, header with close button, scrollable body, optional footer) as a reusable wrapper. Current modals duplicate the overlay/dialog structure inline. This extraction is a prerequisite for consistent focus management and animation.

### 16.6 Reduced-Motion Support

All animations (modal slide-up, drawer slide-in, kanban card flip, column collapse) should respect the user's `prefers-reduced-motion` preference:

```css
@media (prefers-reduced-motion: reduce) {
  .modal-overlay,
  .modal-dialog,
  .drawer-panel,
  .right-sidebar {
    animation: none !important;
    transition: none !important;
  }
}
```

This is not yet implemented and should be added to `base.css` or `tokens.css` as a global rule.

---

## 17. Cross-References

| Reference | Location |
|-----------|----------|
| Design token definitions | Doc 1, Section 3 |
| Token consumption rules | Doc 1, Section 3.3 |
| CSS class naming conventions | Doc 1, Section 4.4 |
| Button strategy (global CSS classes) | Doc 1, ADR-003 |
| Dual-context detail components | Doc 1, ADR-006 |
| Container components (PageWrapper, ContentArea) | Doc 2 |
| Right sidebar layout architecture | Doc 2, Section 4 |
| Navigation & Headers (PageHeader, ListPanelHeader) | Doc 3 |
| Content patterns (Entry, PaddedEntry, forms) | Doc 4 |
| Data visualization (Charts, Graphs) | Doc 6 |

---

## Acceptance Criteria

1. Modal Escape key closes the topmost (innermost) modal only -- parent modal remains open when a nested ConfirmDialog is dismissed.
2. Kanban drag fires exactly one `onDrop` callback per card drop. No duplicate API calls on column transitions.
3. Focus returns to the previously focused element after a modal closes.
4. Drawer slide-in animation uses `transform: translateX` (not `width`) to avoid layout reflow.
5. All modals use `role="dialog"` and `aria-modal="true"` for screen reader accessibility.
6. ConfirmDialog focuses the cancel button by default (not the destructive confirm button).
7. Kanban column collapse toggle activates on Enter/Space for keyboard users.
8. All custom callback props use camelCase (`onClose`, `onUpdate`, `onDrop`) -- DOM events stay lowercase (`onclick`, `onchange`).
9. BulletDetailModal z-index is documented as a remediation item until migrated to `var(--z-modal)`.
10. Reduced-motion media query is planned for all animations.
