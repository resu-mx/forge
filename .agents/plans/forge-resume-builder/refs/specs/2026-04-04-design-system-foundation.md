# Forge Design System Foundation

**Date:** 2026-04-04
**Doc:** 1 of 6 (Design System Series)
**Status:** Reference specification -- single source of truth for the Forge UI Design System

This document establishes the vocabulary, architecture, and rules that all other design system docs reference. It is not an implementation plan. It is the canonical reference for anyone building, reviewing, or extending Forge UI components.

---

## 1. Design System Mental Model

The Forge UI is built from four compositional layers. Each layer has a single responsibility. Higher layers compose lower layers; lower layers never reach up.

```
Containers (viewport-level shells)
  -> Views (layout strategies within content areas)
    -> Components (meaningful UI units with domain awareness)
      -> Atoms (primitive, reusable input/display elements)
```

### 1.1 Containers

**Responsibility:** Define the physical regions of the viewport. They own the grid/flex skeleton that divides screen real estate and handle sticky/fixed positioning. Containers know nothing about domain data.

A container answers: *"Where does content go on screen?"*

The root layout (`+layout.svelte`) is itself the outermost container, establishing the sidebar + content split. Within `ContentArea`, pages can further subdivide using `PageWrapper` to control scroll behavior.

### 1.2 Views

**Responsibility:** Define how a collection of items is arranged and interacted with. A view receives data and renders it using a layout strategy -- board columns, split master/detail, scrolling list, or a node graph. Views own scroll containers, drag-and-drop zones, and layout-level interaction patterns.

A view answers: *"How is this collection of items presented?"*

Views are swappable. The same data set (e.g., job descriptions) can render as a `KanbanBoard` or a `ListView` depending on user preference. The `ViewToggle` component controls which view is active.

### 1.3 Components

**Responsibility:** Render a single meaningful unit of UI. Components are domain-aware -- they know what a "resume card" or a "column header" looks like. They compose atoms for inputs and display, and they emit events upward. Components are the primary unit of reuse across pages.

A component answers: *"What does this thing look like and what can the user do with it?"*

Components may be used in multiple contexts. A detail editor component renders identically whether it appears in a `SplitPanel` detail pane or inside a `Modal` overlay (see ADR-006).

### 1.4 Atoms

**Responsibility:** The smallest reusable UI primitives. Atoms have no domain knowledge. They accept generic data (strings, arrays, booleans) and emit generic events. They are styled entirely through design tokens and provide the tactile building blocks: text inputs, dropdowns, badges, toggle switches.

An atom answers: *"How does the user enter/view a single piece of data?"*

Some atoms are implemented as global CSS classes (buttons, form inputs) rather than Svelte components -- see ADR-003.

### 1.5 Composition Rules

1. **Containers never render atoms directly.** Containers compose views; views compose components; components compose atoms.
2. **Data flows down, events flow up.** Props descend through layers. Callbacks (`onclick`, `onchange`, `onNew`) propagate user intent upward.
3. **A layer may skip one level when wrapping.** A view may directly render an atom (e.g., a `SearchBar` inside a `KanbanBoard` header) when the intermediary component adds no value.
4. **Pages are not a layer.** A `+page.svelte` file is a page-level orchestrator. It fetches data, selects which view to render, and wires callbacks. Pages live outside the design system hierarchy.

---

## 2. Full Component Taxonomy

Every component in the Forge design system, organized by layer. The **Spec Doc** column indicates which document in the 6-doc series defines the component.

### 2.1 Containers

| Component | Spec Doc | Status | Description |
|-----------|----------|--------|-------------|
| **Viewport** | Doc 2 | Exists | Root `<div class="app">` shell; flex row splitting sidebar from content |
| **LeftSidebar** | Doc 2 | Exists | Fixed-width nav rail with logo, accordion nav groups, and profile button |
| **RightSidebar** | Doc 2 | Planned | Optional collapsible panel for contextual info (graph filters, metadata) |
| **ContentArea** | Doc 2 | Exists | Flex-grow main region; receives page content via `{@render children()}` |
| **PageWrapper** | Doc 2 | Exists | Height-constraining wrapper for AppPage layouts; controls overflow mode |

### 2.2 Views

| Component | Spec Doc | Status | Description |
|-----------|----------|--------|-------------|
| **KanbanBoard** | Doc 5 | Exists | Horizontal column layout with drag-and-drop card reordering |
| **SplitPanel** | Doc 2 | Exists | Master/detail two-pane layout with configurable list width |
| **ListView / TableView** | Doc 5 | Planned | Vertical scrolling list or tabular view of items |
| **EdgeNodeGraph** | Doc 6 | Exists | Sigma.js WebGL graph renderer for relationship visualization |

### 2.3 Components

| Component | Spec Doc | Status | Description |
|-----------|----------|--------|-------------|
| **Header** | Doc 3 | Exists | Composite page header area containing title, subtitle, and action buttons |
| **PageHeader** | Doc 3 | Exists | Title + subtitle + actions slot for page-level headings |
| **ListPanelHeader** | Doc 3 | Exists | Title + "New" button for split-panel list headers |
| **HeaderTitle** | Doc 3 | Exists | `<h1>` or `<h2>` with token-based typography |
| **HeaderSubTitle** | Doc 3 | Exists | Muted subheading text below a title |
| **HeaderButton** | Doc 3 | Exists | Action button placed in a header's action area |
| **HeaderToggleButton** | Doc 3 | Exists | Toggle button in header for mode switching (e.g., edit/preview) |
| **TabbedHeader** | Doc 3 | Exists | Header variant with integrated TabBar for sub-navigation |
| **TabBar** | Doc 3 | Exists | Horizontal tab strip with keyboard navigation and ARIA roles |
| **ViewToggle** | Doc 3 | Exists | Segmented toggle for switching between view modes (list/board) |
| **Entry** | Doc 4 | Exists | Base list entry row (clickable, selected state, no padding opinion) |
| **PaddedEntry** | Doc 4 | Exists | Entry variant with standard internal padding and border-bottom |
| **SectionedList** | Doc 4 | Exists | List with grouped sections, each with a section title |
| **KanbanColumn** | Doc 5 | Exists | Single column in a KanbanBoard with header, count badge, and card list |
| **KanbanCard** | Doc 5 | Exists | Card rendered inside a KanbanColumn; domain-specific variants exist |
| **Modal** | Doc 5 | Exists | Overlay dialog with header, scrollable body, and footer actions |
| **ConfirmDialog** | Doc 5 | Exists | Narrow modal variant for destructive confirmation prompts |
| **EmptyState** | Doc 4 | Exists | Centered message with optional CTA for empty data scenarios |
| **EmptyPanel** | Doc 4 | Exists | Empty state specific to split-panel detail pane |
| **MetricContainer** | Doc 6 | Exists | Dashboard card wrapper for a single metric or chart |
| **MetricCalloutContainer** | Doc 6 | Exists | Highlighted metric card with accent border for key stats |

### 2.4 Atoms

| Component | Spec Doc | Status | Description |
|-----------|----------|--------|-------------|
| **DataInput** | Doc 4 | Exists | Text input field with token-based styling (`.field-input` CSS class) |
| **TitledDataInput** | Doc 4 | Exists | Label + DataInput composed as a `.form-field` |
| **SearchBar** | Doc 3 | Exists | Text input with search icon and debounced value emission |
| **ListSearchInput** | Doc 3 | Exists | Compact search input for list panel filter areas |
| **DropDownSelect** | Doc 3 | Exists | Native `<select>` with token styling (`.field-select` CSS class) |
| **TagsList** | Doc 4 | Exists | Horizontal wrap of `.pill` elements with optional remove action |
| **LinkButton** | Doc 4 | Exists | Text-styled button that navigates or triggers an action |
| **ToggleButton** | Doc 4 | Exists | Two-state button (on/off) with visual active indicator |
| **StatusBadge** | Doc 4 | Exists | Color-coded status indicator using the `.badge` CSS class |
| **LoadingSpinner** | Doc 4 | Exists | Animated spinner for loading states |
| **Toast / ToastContainer** | Doc 4 | Exists | Transient notification system with auto-dismiss |
| **ConfidenceBar** | Doc 4 | Exists | Horizontal bar fill showing a 0-100 confidence value |
| **DriftBanner** | Doc 4 | Exists | Warning banner for data staleness alerts |
| **RenderViewport** | Doc 6 | Planned | Container for rendered resume preview (PDF/LaTeX/Markdown) |
| **GraphViewport** | Doc 6 | Exists | Container for Sigma.js canvas with toolbar and filter panel |
| **EdgeNodeGraphViewport** | Doc 6 | Exists | Full graph viewport with search, filters, and toolbar composed |

---

## 3. Design Tokens Strategy

### 3.1 Current Token Inventory

All tokens are defined in `packages/webui/src/lib/styles/tokens.css` on `:root`, with dark overrides via both `@media (prefers-color-scheme: dark)` and `html[data-theme="dark"]`.

**Brand** (3 tokens)
```
--color-primary           Base brand color
--color-primary-hover     Hover state
--color-primary-subtle    Low-opacity background tint
```

**Surfaces** (4 tokens)
```
--color-bg                Page background
--color-surface           Card/panel background
--color-surface-raised    Elevated surface (footers, raised cards)
--color-surface-sunken    Recessed surface (code blocks, inset areas)
```

**Sidebar** (8 tokens)
```
--color-sidebar-bg        Sidebar background
--color-sidebar-text      Default sidebar text
--color-sidebar-text-hover
--color-sidebar-text-active
--color-sidebar-border
--color-sidebar-hover-bg
--color-sidebar-active-bg
--color-sidebar-accent    Active link accent (left border) -- NOTE: missing from dark mode media query, needs override
```

**Text** (5 tokens)
```
--text-primary            Body text
--text-secondary          Less prominent text
--text-muted              Labels, placeholders, metadata
--text-faint              Disabled, decorative
--text-inverse            Text on colored backgrounds
```

**Borders** (3 tokens)
```
--color-border            Default border
--color-border-strong     More prominent border (inputs, dividers)
--color-border-focus      Focus ring border
```

**Status** (16 tokens across success/danger/warning/info)
```
--color-{status}          Primary status color
--color-{status}-hover    Hover (danger only)
--color-{status}-subtle   Low-opacity background
--color-{status}-text     High-contrast text
--color-{status}-strong   Stronger variant (success only)
--color-{status}-border   Border variant (warning, info)
--color-{status}-bg       Background variant (warning)
```

**Interactive** (3 tokens)
```
--color-ghost             Ghost button background
--color-ghost-hover       Ghost button hover
--color-ghost-text        Ghost button text
```

**Overlay** (1 token)
```
--color-overlay           Modal backdrop
```

**Tags/Pills** (4 tokens)
```
--color-tag-bg / --color-tag-text           Accent-colored tags
--color-tag-neutral-bg / --color-tag-neutral-text  Neutral tags
```

**Template Highlight** (3 tokens)
```
--color-template-bg / --color-template-border / --color-template-star
```

**Chart Palette** (8 tokens)
```
--color-chart-1 through --color-chart-8     Data visualization colors
```

**Typography** (14 tokens)
```
--font-sans / --font-mono                   Font families (2)
--text-xs through --text-2xl                Font size scale (6 steps)
--font-normal / --font-medium / --font-semibold / --font-bold  Weights (4)
--leading-tight / --leading-normal          Line heights (2)
```

**Spacing** (9 tokens)
```
--space-1 through --space-12                4px to 48px scale (9 tokens: 1,2,3,4,5,6,8,10,12 -- no --space-7 or --space-9)
```

**Radii** (4 tokens)
```
--radius-sm / --radius-md / --radius-lg / --radius-full
```

**Shadows** (3 tokens)
```
--shadow-sm / --shadow-md / --shadow-lg
```

**Z-index** (3 tokens)
```
--z-dropdown (100) / --z-modal (10000) / --z-toast (10100)
```

### 3.2 Token Gaps

The following tokens are needed but not yet defined:

| Gap | Rationale |
|-----|-----------|
| `--color-primary-text` | Text color for use on primary backgrounds (currently `--text-inverse` is used, but semantically wrong for non-white-on-dark cases) |
| `--color-link` / `--color-link-hover` | Link text colors (currently hardcoded or using `--color-primary`) |
| `--sidebar-width` | Sidebar width is hardcoded to `220px` in layout; should be a token for RightSidebar parity |
| `--content-padding` | Content area padding is hardcoded to `2rem`; tokenizing enables consistent PageWrapper math |
| `--transition-fast` / `--transition-normal` | Transition durations are repeated as `0.15s` everywhere |
| `--z-sidebar` | Sidebar stacking context (for mobile overlays) |
| `--z-popover` | Popover/combobox dropdown (between dropdown and modal) |
| JD pipeline status colors | `StatusBadge` hardcodes hex values for `discovered`, `applying`, `applied`, `interviewing`, `offered`, `withdrawn`, `closed` |
| `--space-16` / `--space-20` | Larger spacing for page-level margins and section gaps |

### 3.3 Token Consumption Rule

**Components MUST use tokens. Hardcoded values are forbidden.**

```css
/* CORRECT */
.card { background: var(--color-surface); border-radius: var(--radius-md); }

/* FORBIDDEN */
.card { background: #ffffff; border-radius: 6px; }
```

The only acceptable exception is the global reset (`margin: 0; padding: 0; box-sizing: border-box`) in `tokens.css` itself.

Fallback values in `var()` are acceptable during migration but should be removed once tokens are stable:
```css
/* Acceptable during migration */
.toggle-btn { color: var(--text-muted, #6b7280); }

/* Target state -- no fallback */
.toggle-btn { color: var(--text-muted); }
```

**Known violation:** `ViewToggle` currently uses hardcoded fallback values in several `var()` calls (e.g., `var(--color-border-strong, #d1d5db)`). These should be removed once tokens are confirmed stable.

### 3.4 Token Naming Convention

```
--{category}-{purpose}[-{modifier}]
```

| Category | Examples |
|----------|----------|
| `color` | `--color-primary`, `--color-surface-raised`, `--color-danger-subtle` |
| `text` | `--text-primary`, `--text-muted` (color), `--text-sm`, `--text-2xl` (size) |
| `font` | `--font-sans`, `--font-medium` |
| `space` | `--space-4`, `--space-12` |
| `radius` | `--radius-md`, `--radius-full` |
| `shadow` | `--shadow-sm`, `--shadow-lg` |
| `z` | `--z-modal`, `--z-toast` |
| `leading` | `--leading-tight`, `--leading-normal` |

**Modifiers** describe variants: `-hover`, `-subtle`, `-strong`, `-text`, `-bg`, `-border`, `-active`.

**Note on the `--text-` prefix collision:** `--text-primary` is a color; `--text-sm` is a size. This is an inherited naming pattern. New text-color tokens should use `--text-` and new text-size tokens should also use `--text-`. Context disambiguates (color properties vs. `font-size` properties). If a future refactor is warranted, the migration path would be `--text-color-*` and `--text-size-*`.

### 3.5 Theming and Branding

To create a new theme or brand variant:

1. **Copy the `:root` block** from `tokens.css`.
2. **Replace color values** under a new selector (e.g., `html[data-theme="brand-x"]`).
3. **Typography, spacing, radii, and shadows** are theme-independent by default but can be overridden per-brand.
4. **The sidebar palette is fully tokenized** and can be themed independently of content colors.
5. **Chart palette** should be updated together to ensure accessibility contrast on the new background.

The theme store (`$lib/stores/theme.svelte.ts`) manages persistence via `localStorage('forge-theme')` and applies the `data-theme` attribute on `<html>`.

---

## 4. CSS Architecture

### 4.1 File Organization

```
packages/webui/src/
  lib/
    styles/
      tokens.css          # CSS custom properties (design tokens)
      base.css            # Global utility classes (buttons, forms, modals, badges, cards)
    components/
      *.svelte            # Shared components with scoped <style> blocks
  routes/
    +layout.svelte        # Imports tokens.css + base.css; defines viewport containers
    **/ +page.svelte      # Page orchestrators; minimal scoped styles
```

**Import order** in `+layout.svelte`:
```svelte
<script>
  import '$lib/styles/tokens.css'   // 1. Tokens first (custom properties)
  import '$lib/styles/base.css'     // 2. Base classes second (consume tokens)
</script>
```

### 4.2 Global Styles (base.css)

Global CSS classes live in `base.css` and are available everywhere without import. They cover patterns that appear across many components and pages:

| Class Family | Purpose |
|-------------|---------|
| `.btn`, `.btn-primary`, `.btn-ghost`, `.btn-danger`, etc. | Button variants and sizes |
| `.form-field`, `.field-label`, `.field-input`, `.field-select` | Form input styling |
| `.form-group`, `.form-actions` | Form layout |
| `.modal-overlay`, `.modal-dialog`, `.modal-header`, `.modal-body`, `.modal-footer` | Modal structure |
| `.badge`, `.pill`, `.pill-neutral`, `.count-badge` | Status indicators and tags |
| `.card`, `.card-header`, `.card-title`, `.card-meta`, `.card--template` | Card patterns |
| `.split-panel`, `.list-panel`, `.editor-panel` | Split-panel layout (legacy; prefer `SplitPanel` component) |
| `.banner-warning`, `.banner-info` | Alert banners |
| `.page-title`, `.page-subtitle`, `.section-title` | Typography patterns |

**When to add to base.css:** A pattern belongs in `base.css` when it appears in 3+ components/pages AND has no behavioral logic (no `$state`, no event handlers, no lifecycle). If it needs logic, it should be a Svelte component.

### 4.3 Scoped Styles (Svelte `<style>`)

Every Svelte component uses a scoped `<style>` block for its internal styling. Svelte's compiler automatically scopes these classes to the component, preventing leakage.

**Scoped styles are for:**
- Component-specific layout (flex arrangements, grid, positioning)
- Component-specific states (`.active`, `.selected`, `.expanded`)
- Hover/focus/transition effects unique to the component
- Size and spacing adjustments for the component's own elements

**Scoped styles consume tokens:**
```svelte
<style>
  .list-panel-header {
    padding: var(--space-5) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }
</style>
```

### 4.4 CSS Class Naming

**Global classes (base.css):** Flat kebab-case.
```css
.btn-primary { }
.field-input { }
.modal-overlay { }
.card-header { }
```

**Component-scoped classes:** BEM-like naming within the component's namespace. The block name matches the component. Elements use `__`. Modifiers use `--`.
```css
/* ListPanelHeader.svelte */
.list-panel-header { }
.list-panel-header__title { }
.list-panel-header__actions { }
.list-panel-header__new-btn { }

/* PageWrapper.svelte */
.page-wrapper { }
.page-wrapper--overflow-auto { }
.page-wrapper--overflow-hidden { }
```

**The BEM convention is a naming guide, not strict BEM methodology.** We do not require separate block/element/modifier files or follow the full BEM file structure. The point is predictable, collision-free class names within a component.

### 4.5 The Button Strategy

Buttons use global CSS classes from `base.css` rather than a `<Button>` component. See ADR-003 for rationale.

Usage pattern:
```svelte
<button class="btn btn-primary">Save</button>
<button class="btn btn-ghost btn-sm">Cancel</button>
<button class="btn btn-danger btn-xs">Delete</button>
<button class="btn-icon">&#9881;</button>
```

Composition:
- **Base:** `.btn` (always required, provides flex/padding/cursor/transition)
- **Variant:** `.btn-primary` | `.btn-ghost` | `.btn-danger` | `.btn-danger-ghost` | `.btn-secondary`
- **Size:** `.btn-xs` | `.btn-sm` | (default) | `.btn-lg`
- **Special:** `.btn-icon` (standalone, not combined with `.btn`)
- **Semantic aliases:** `.btn-save`, `.btn-delete`, `.btn-new`, `.btn-edit` (empty rules for readability)

### 4.6 Forbidden Patterns

The following MUST NOT appear in component or page styles:

| Pattern | Why | Use Instead |
|---------|-----|-------------|
| Hardcoded hex/rgb colors | Breaks theming | `var(--color-*)` or `var(--text-*)` |
| Hardcoded font sizes | Breaks scale consistency | `var(--text-*)` |
| Hardcoded spacing values | Breaks spacing rhythm | `var(--space-*)` |
| `!important` | Specificity wars | Fix selector specificity |
| `:global()` in components | Leaks styles; defeats scoping | Add to `base.css` or use component-scoped classes |
| Inline `style=` for colors/spacing | Unthemeable, unsearchable | Scoped class with tokens |
| `@import` in component `<style>` | Svelte does not support it | Import in `+layout.svelte` or use tokens |

**Exceptions:** Inline `style=` is acceptable for truly dynamic values computed at runtime (e.g., `style:width="{listWidth}px"`, `style:background={computedColor}`). The rule targets static values that should be tokens.

---

## 5. Naming Conventions

### 5.1 Component Files

```
PascalCase.svelte
```

Examples: `SplitPanel.svelte`, `KanbanCard.svelte`, `ListPanelHeader.svelte`, `StatusBadge.svelte`

Subdirectories group related components: `kanban/`, `graph/`, `resume/`, `charts/`, `filters/`, `contacts/`, `jd/`.

### 5.2 CSS Classes

| Context | Convention | Example |
|---------|-----------|---------|
| Global (base.css) | kebab-case | `.btn-primary`, `.field-input`, `.modal-overlay` |
| Component-scoped | BEM-like | `.list-panel-header__title`, `.page-wrapper--overflow-auto` |
| Utility/state | kebab-case | `.active`, `.selected`, `.pulsing`, `.open` |

### 5.3 Props

Props use camelCase and are defined via `$props()` with a TypeScript interface:

```typescript
interface Props {
  title: string               // Required string
  subtitle?: string           // Optional string
  listWidth?: number           // Optional with default in destructuring
  isOpen?: boolean             // Boolean state props use 'is' prefix
  mode?: 'list' | 'board'     // Union literals for constrained values
  actions?: Snippet            // Snippet props for composable slots
  children?: Snippet           // Default slot
}

let { title, subtitle, listWidth = 320, ...rest }: Props = $props()
```

**Boolean props:** Use `is` prefix when the prop describes a state (`isOpen`, `isActive`, `isModal`). Omit prefix when the prop is a capability (`destructive`, `disabled`).

**Bindable props:** Use `$bindable()` for two-way binding (e.g., search input value):
```typescript
let { value = $bindable('') }: Props = $props()
```

### 5.4 Events and Callbacks

Callback props use the `on` prefix with no separator, matching Svelte 5's native event naming:

```typescript
interface Props {
  onclick?: () => void                        // Simple action
  onchange?: (value: string) => void          // Value change
  onNew?: () => void                          // Domain action
  onClose?: () => void                        // Dismiss
  onconfirm?: () => void                      // Confirmation
  oncancel?: () => void                       // Cancellation
}
```

**Convention:** Use **camelCase** for custom (non-DOM) callbacks: `onClose`, `onUpdate`, `onNew`, `onDelete`. This is consistent with Svelte convention for non-DOM event handlers. DOM events stay lowercase to match native HTML: `onclick`, `onchange`, `onkeydown`, `oninput`.

**Rule of thumb:** If the browser fires it (click, change, keydown, input, submit), use lowercase. If the component defines it (Close, New, Update, Delete, Drop), use camelCase.

### 5.5 Snippet Props (Slots)

Svelte 5 uses `Snippet` for composable content slots:

```typescript
import type { Snippet } from 'svelte'

interface Props {
  children: Snippet                           // Default slot
  actions?: Snippet                           // Named slot
  tab?: Snippet<[{ tab: TabItem; active: boolean }]>  // Typed slot with render args
  list: Snippet                               // Required named slot
  detail: Snippet                             // Required named slot
}
```

**Naming:** Snippet props describe the content region they fill: `actions`, `header`, `footer`, `list`, `detail`, `tab`. The default slot is always `children`.

**Rendering:**
```svelte
{@render children()}
{#if actions}{@render actions()}{/if}
```

### 5.6 Type Exports

Components that define reusable types export them from the `<script>` block:

```typescript
// TabBar.svelte
export interface TabItem {
  value: string
  label: string
}
```

Consumers import the type alongside the component:
```typescript
import TabBar, { type TabItem } from '$lib/components/TabBar.svelte'
```

---

## 6. Architecture Decision Records

### ADR-001: Component Layers (Container -> View -> Component -> Atom)

**Context:** As Forge grew past 70+ Svelte files, there was no shared vocabulary for discussing component responsibilities. Different developers (human and AI) would create components at arbitrary abstraction levels, leading to inconsistent composition patterns and duplicated layout code.

**Decision:** Adopt a four-layer hierarchy: Container, View, Component, Atom. Each layer has a defined responsibility (see Section 1). Components are classified into exactly one layer.

**Rationale:**
- **Clear ownership:** When building a new feature, the developer knows which layer each piece belongs to.
- **Predictable composition:** Containers compose views, views compose components, components compose atoms. No ad-hoc nesting.
- **Testability:** Atoms and components can be tested in isolation. Views require mock data. Containers are tested via integration.

**Alternatives considered:**
- *Flat component library:* No hierarchy. Rejected because it provides no guidance on composition.
- *Atomic Design (atoms/molecules/organisms/templates/pages):* Five levels is more granularity than Forge needs. The molecule/organism distinction is ambiguous for this codebase.

**Status:** Accepted.

---

### ADR-002: Design Tokens over Hardcoded Values

**Context:** The original codebase had 40+ Svelte files with duplicated hex colors, font sizes, and spacing values. Changing the brand color required a find-and-replace across every file, with high risk of missed instances.

**Decision:** All visual properties -- colors, typography, spacing, radii, shadows, z-indices -- are defined as CSS custom properties (tokens) in `tokens.css`. Components consume tokens exclusively. Hardcoded visual values are forbidden (see Section 4.6).

**Rationale:**
- **Theming:** Light/dark mode is a token swap, not a component rewrite.
- **Consistency:** A spacing scale ensures visual rhythm. A color palette ensures accessible contrast.
- **Discoverability:** `tokens.css` is the single source of truth for available values.
- **Branding:** A new theme is created by overriding token values under a new `data-theme` selector.

**How to add a new token:**
1. Add the custom property to the `:root` block in `tokens.css` with the light-mode value.
2. Add the dark-mode override in the `@media (prefers-color-scheme: dark)` block.
3. Add the light and dark explicit overrides in `html[data-theme="light"]` and `html[data-theme="dark"]`.
4. Use the token in component styles via `var(--token-name)`.

**Token naming scheme:** `--{category}-{purpose}[-{modifier}]`. See Section 3.4.

**Status:** Accepted and enforced.

---

### ADR-003: Global Button CSS over Button Component

**Context:** Buttons are the most common interactive element. Two approaches were considered: a `<Button>` Svelte component with variant/size props, or global CSS classes in `base.css`.

**Decision:** Buttons are styled via global CSS classes (`.btn`, `.btn-primary`, `.btn-ghost`, etc.) in `base.css`. There is no `<Button>` Svelte component.

**Rationale:**
- **Zero abstraction cost:** A `<button class="btn btn-primary">` is immediately readable. No prop lookup required.
- **Native HTML:** The `<button>` element retains all native attributes (`type`, `disabled`, `form`, `aria-*`) without prop forwarding boilerplate.
- **Composability:** Buttons appear inside other components' `<style>` contexts. A CSS class composes naturally; a component wrapper adds nesting complexity.
- **Event handling:** `onclick` works natively. A `<Button>` component would need to forward all possible DOM events.
- **SSR compatibility:** CSS classes work identically in SSR and CSR contexts.

**Trade-offs:**
- No type checking on variant/size combinations (a component could enforce valid combos via union props).
- No centralized rendering logic (each usage is raw HTML).
- These trade-offs are acceptable given the simplicity of button styling.

**Known exception:** `ListPanelHeader` uses a scoped `.list-panel-header__new-btn` class instead of `.btn .btn-primary .btn-sm`. This is a debt item -- the component should be migrated to use global button classes.

**Status:** Accepted.

---

### ADR-004: Progressive Adoption with Enforcement Tests

**Context:** The design system was introduced into an existing codebase with 70+ Svelte files already in production. A big-bang migration is risky and impractical. But without enforcement, new code will continue using hardcoded values.

**Decision:** Adopt progressively with two enforcement mechanisms:

1. **`.claude/rules` directives:** AI agents building Forge UI are instructed to use tokens and base classes. This catches new code at authorship time.
2. **Grep-based CI tests:** A test script scans `*.svelte` files for forbidden patterns (hardcoded hex colors, hardcoded `px` font sizes outside `var()`, `!important` declarations). This catches regressions.

**Enforcement test examples:**
```bash
# No hardcoded hex colors in component styles
grep -rn '#[0-9a-fA-F]\{3,8\}' packages/webui/src/lib/components/**/*.svelte

# No !important
grep -rn '!important' packages/webui/src/lib/components/**/*.svelte
```

**Migration strategy:**
- Phase 1 (done): Create `tokens.css` and `base.css`. Update shared components.
- Phase 2 (done): Sweep page-level styles to consume tokens.
- Phase 3 (current): Extract reusable components from page-level patterns.
- Phase 4 (future): Enable CI enforcement tests as blocking checks.

**Status:** Accepted. CI tests are defined but not yet blocking.

---

### ADR-005: Svelte 5 Runes ($props, $state, Snippet) as Component API Standard

**Context:** Svelte 5 introduced runes (`$props`, `$state`, `$derived`, `$effect`) and `Snippet` as replacements for `export let`, stores, reactive declarations, and slots. The Forge codebase must choose one API style.

**Decision:** All Forge components use Svelte 5 runes exclusively. The Svelte 4 patterns (`export let`, `$$slots`, `<slot>`, `$:`, reactive stores with `$` prefix) are not used.

**Component API pattern:**
```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    title: string
    actions?: Snippet
  }

  let { title, actions }: Props = $props()
  let internalState = $state(false)
  let computed = $derived(title.toUpperCase())
</script>
```

**Key rules:**
- `$props()` replaces `export let`. Always destructured with a TypeScript interface.
- `$state()` replaces `let x = value` for reactive local state.
- `$derived()` replaces `$: x = ...` for computed values.
- `$effect()` replaces `$: { ... }` for side effects. Prefer `onMount` for one-time setup.
- `Snippet` replaces `<slot>`. Render via `{@render snippetName()}`.
- `$bindable()` replaces `bind:value` prop support on the component side.
- Reactive state must use assignment, not mutation. `Set.add()` does not trigger reactivity; use `Record` property writes instead.

**Status:** Accepted and enforced in all existing components.

---

### ADR-006: Detail Components Render in Both Inline and Modal Contexts

**Context:** Many entities in Forge (bullets, organizations, job descriptions) need a detail view. Sometimes this view appears in the right pane of a `SplitPanel`. Sometimes it appears as a `Modal` overlay (e.g., from a kanban card click or a chain-view drill-down).

**Decision:** The same detail component is used for both contexts. The component receives an `isModal` prop (or the parent wraps it in a `Modal`), but the component's internal layout and content are identical.

**Rationale:**
- **Single source of truth:** One component means one set of fields, validations, and event handlers.
- **Consistent UX:** The user sees identical information whether they're in split-panel or modal mode.
- **Less code:** No duplicated detail views.

**Implementation pattern:**
```svelte
<!-- In SplitPanel detail pane -->
<BulletDetail bullet={selected} />

<!-- In modal context -->
{#if modalOpen}
  <div class="modal-overlay">
    <div class="modal-dialog" style="max-width: 640px;">
      <BulletDetail bullet={selected} onClose={() => modalOpen = false} />
    </div>
  </div>
{/if}
```

The detail component adapts to its container's dimensions via flex/grid layout. It does not hardcode width or height.

**Status:** Accepted. Implemented for `BulletDetailModal`, `OrgDetailModal`, `ChainViewModal`.

---

## 7. Documentation Requirements

### 7.1 Documentation Structure

```
docs/
  src/
    design-system/
      index.md                  # This foundation doc (Doc 1)
      layout-containers.md      # Doc 2: Layout & Containers
      navigation-headers.md     # Doc 3: Navigation & Headers
      content-patterns.md       # Doc 4: Content Patterns
      interactive-systems.md    # Doc 5: Interactive Systems
      data-viz.md               # Doc 6: Data Visualization
      adrs/
        001-component-layers.md
        002-design-tokens.md
        003-global-button-css.md
        004-progressive-adoption.md
        005-svelte5-runes.md
        006-dual-context-detail.md
      tokens/
        colors.md
        typography.md
        spacing.md
```

### 7.2 Component Documentation Format

Each component that warrants its own section should include:

**Description:** One paragraph explaining purpose and layer classification.

**API:** Table of props with name, type, default, and description.

**Snippets:** List of Snippet props with their render arguments.

**Usage Example:** Minimal Svelte code showing the component in context.

**Do / Don't:** Two-column guidance showing correct and incorrect usage.

**Accessibility:** ARIA roles, keyboard interaction, focus management notes.

### 7.3 ADR Format

ADRs follow the format used in Section 6:
- **Context:** What problem or decision point triggered this.
- **Decision:** What was decided.
- **Rationale:** Why this choice was made.
- **Alternatives considered:** What else was evaluated and why it was rejected.
- **Status:** Accepted, Deprecated, or Superseded (with link to successor).

---

## 8. Implementation Guidelines

### 8.1 Page Types

Forge pages fall into three categories based on their scroll and layout behavior:

**FlowPage** -- The page content scrolls naturally within the browser viewport. The content area has `overflow: auto` (the default from `+layout.svelte`). Used for form-heavy pages, settings, and simple lists.

Examples: `/config/profile`, `/config/export`, `/data/notes`

```svelte
<!-- FlowPage: no PageWrapper needed, content scrolls naturally -->
<PageHeader title="Profile" />
<div class="form-group">
  <!-- form fields -->
</div>
```

**AppPage** -- The page fills the viewport exactly; internal regions scroll independently. Uses `PageWrapper` with `overflow="hidden"` to prevent the outer content area from scrolling. Internal panes (list panel, detail panel) manage their own scroll.

Examples: `/data/sources` (SplitPanel), `/opportunities/organizations` (KanbanBoard), `/data/bullets`

```svelte
<!-- AppPage: PageWrapper constrains height, panels scroll internally -->
<PageWrapper overflow="hidden">
  <SplitPanel {listWidth}>
    {#snippet list()} ... {/snippet}
    {#snippet detail()} ... {/snippet}
  </SplitPanel>
</PageWrapper>
```

**DualModePage** -- The page switches between two view modes (e.g., list and board). A `ViewToggle` controls the active mode. Both modes live in the same `+page.svelte`. One mode may be a FlowPage layout (list) while the other is an AppPage layout (kanban).

Examples: `/data/bullets` (list/board), `/data/sources` (list/board)

```svelte
<ViewToggle mode={viewMode} onchange={(m) => viewMode = m} />
{#if viewMode === 'list'}
  <!-- FlowPage-style scrolling list -->
{:else}
  <!-- AppPage-style kanban board -->
{/if}
```

### 8.2 When to Create a New Component

Create a new component when:

1. **The same markup+styles appear in 3+ places.** Extract into `$lib/components/`.
2. **The element has interactive behavior** (state, event handlers, lifecycle). Even if used once, encapsulation aids readability.
3. **The element is domain-agnostic** and could be reused in a different page context.
4. **The element needs to render in both inline and modal contexts** (ADR-006).

Do NOT create a new component when:
- The markup is a one-line element with no state (use a CSS class from `base.css`).
- The "component" would just be a thin wrapper around a native HTML element with no added behavior (see ADR-003 for buttons).
- The pattern is page-specific and unlikely to appear elsewhere.

### 8.3 Adding to the Barrel Export

When a new shared component is created in `$lib/components/`, add it to the barrel export in `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as NewComponent } from './NewComponent.svelte'
```

**Order:** Maintain alphabetical order within the file.

**Sub-directory components:** Only export components from `index.ts` if they are consumed outside their subdirectory. Internal composition components (e.g., `KanbanColumn` is only used by `KanbanBoard`) do not need barrel exports.

**Type exports:** If the component exports a type, re-export it:
```typescript
export { default as TabBar } from './TabBar.svelte'
export type { TabItem } from './TabBar.svelte'
```

### 8.4 Testing Expectations

**Unit tests:** Components with logic (computed values, state machines, validation) should have unit tests via Vitest + `@testing-library/svelte`.

**Visual regression tests:** Deferred to a future Playwright/Storybook setup. Not required for initial implementation.

**Adoption enforcement tests:** Grep-based scripts that verify:
- No hardcoded hex colors in `$lib/components/**/*.svelte` `<style>` blocks
- No `!important` declarations in component styles
- No `export let` in component `<script>` blocks (Svelte 5 enforcement)
- All shared components are listed in `index.ts` barrel export
- No `:global()` in component-scoped styles

**Accessibility tests:** Components with ARIA roles (`TabBar`, `ConfirmDialog`, `Modal`) should have keyboard interaction tests verifying focus management, tab order, and screen reader announcements.

### 8.5 Component File Template

New components should follow this structure:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    // Required props first
    title: string
    // Optional props second
    subtitle?: string
    // Snippet props last
    actions?: Snippet
  }

  let { title, subtitle, actions }: Props = $props()
</script>

<!-- Template: semantic HTML, token-based classes -->
<div class="my-component">
  <h2 class="my-component__title">{title}</h2>
  {#if subtitle}
    <p class="my-component__subtitle">{subtitle}</p>
  {/if}
  {#if actions}
    <div class="my-component__actions">
      {@render actions()}
    </div>
  {/if}
</div>

<style>
  .my-component {
    /* Layout */
    display: flex;
    align-items: center;
    gap: var(--space-4);
    /* Appearance -- tokens only */
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  .my-component__title {
    font-size: var(--text-xl);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
  }

  .my-component__subtitle {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .my-component__actions {
    flex-shrink: 0;
    display: flex;
    gap: var(--space-2);
  }
</style>
```

---

## 9. Cross-References

This document is referenced by all other docs in the series:

| Doc | Title | Focus |
|-----|-------|-------|
| **Doc 1** | Foundation (this document) | Vocabulary, tokens, architecture, ADRs |
| **Doc 2** | Layout & Containers | Viewport, sidebars, content area, PageWrapper, SplitPanel |
| **Doc 3** | Navigation & Headers | PageHeader, ListPanelHeader, TabBar, ViewToggle, filter bars |
| **Doc 4** | Content Patterns | Entry, PaddedEntry, SectionedList, forms, detail panels, EmptyState |
| **Doc 5** | Interactive Systems | KanbanBoard, modals, drawers, drag-and-drop, dual-context rendering |
| **Doc 6** | Data Visualization | Charts (ECharts), graphs (Sigma.js), render viewports |

---

## Appendix A: Existing Component Inventory

Components that exist in the codebase as of 2026-04-04, organized by directory.

**`$lib/components/` (root)**
`ConfidenceBar`, `ConfirmDialog`, `DriftBanner`, `EmptyPanel`, `EmptyState`, `ListPanelHeader`, `ListSearchInput`, `LoadingSpinner`, `OrgCombobox`, `PageHeader`, `PageWrapper`, `ProfileMenu`, `SplitPanel`, `StatusBadge`, `TabBar`, `Toast`, `ToastContainer`, `ViewToggle`, `BulletDetailModal`, `ChainViewModal`, `DerivePerspectivesDialog`, `SourcesView`, `SummaryPicker`

**`$lib/components/kanban/`**
`KanbanBoard`, `KanbanColumn`, `KanbanCard`, `GenericKanban`, `GenericKanbanColumn`, `OrgPickerModal`, `OrgDetailModal`, `BulletKanbanCard`, `PerspectiveKanbanCard`, `ResumeKanbanCard`, `SourceKanbanCard`, `JDKanbanCard`

**`$lib/components/graph/`**
`GraphView`, `GraphFilterPanel`, `GraphSearchBar`, `GraphToolbar`, `LocalGraphWidget`

**`$lib/components/charts/`**
`EChart`, `SkillsSunburst`, `SkillsTreemap`, `BulletsTreemap`, `DomainsTreemap`, `JDSkillRadar`, `CompensationBulletGraph`, `ApplicationGantt`, `RoleChoropleth`

**`$lib/components/resume/`**
`HeaderEditor`, `AddSectionDropdown`, `DragNDropView`, `LatexView`, `MarkdownView`, `OverrideBanner`, `PdfView`, `SkillsPicker`, `SourcePicker`, `SourceView`, `ResumeLinkedJDs`, `JDPickerModal`

**`$lib/components/jd/`**
`JDCard`, `JDEditor`, `JDLinkedResumes`, `JDSkillExtraction`, `JDSkillPicker`, `ExtractedSkillCard`, `ResumePickerModal`

**`$lib/components/filters/`**
`BulletFilterBar`, `PerspectiveFilterBar`, `ResumeFilterBar`, `SourceFilterBar`, `JDFilterBar`

**`$lib/components/contacts/`**
`ContactCard`, `ContactEditor`, `ContactLinkSection`, `LinkContactDialog`

---

## Appendix B: Token Quick Reference

Compact reference for the most-used tokens.

```css
/* Colors */
var(--color-primary)          var(--color-surface)         var(--color-bg)
var(--color-border)           var(--color-border-strong)   var(--color-border-focus)
var(--color-ghost)            var(--color-overlay)

/* Text */
var(--text-primary)           var(--text-secondary)        var(--text-muted)
var(--text-faint)             var(--text-inverse)

/* Status (4 base + variants) */
var(--color-success)          var(--color-danger)          var(--color-warning)
var(--color-info)             /* NOTE: --color-info is defined but absent from Appendix quick ref until now */

/* Typography */
var(--text-xs)   var(--text-sm)   var(--text-base)   var(--text-lg)
var(--text-xl)   var(--text-2xl)

var(--font-sans)  var(--font-mono)
var(--font-normal) var(--font-medium) var(--font-semibold) var(--font-bold)
var(--leading-tight) var(--leading-normal)

/* Spacing */
var(--space-1)  var(--space-2)  var(--space-3)  var(--space-4)
var(--space-5)  var(--space-6)  var(--space-8)  var(--space-10)  var(--space-12)

/* Radii */
var(--radius-sm)  var(--radius-md)  var(--radius-lg)  var(--radius-full)

/* Shadows */
var(--shadow-sm)  var(--shadow-md)  var(--shadow-lg)

/* Z-index */
var(--z-dropdown)  var(--z-modal)  var(--z-toast)
```

---

## Acceptance Criteria

1. All components listed in the taxonomy table (Section 2) use Svelte 5 runes (`$props`, `$state`, `$derived`, `$effect`). No `export let` or `<slot>` in any shared component.
2. No hardcoded hex/rgb color values appear in `$lib/components/**/*.svelte` `<style>` blocks (enforcement grep passes).
3. No `!important` declarations appear in component-scoped styles.
4. No `:global()` selectors appear in component-scoped styles (exceptions must be documented).
5. All spacing values in component styles use `var(--space-*)` tokens -- no raw `px` or `rem` for padding/margin/gap.
6. Every component in `$lib/components/` has a corresponding entry in the barrel export (`index.ts`), or is documented as internal-only.
7. Button elements across all pages use `.btn` + variant classes from `base.css`, not component-scoped button styles (ListPanelHeader exception documented in ADR-003).
8. The taxonomy table Status column accurately reflects which components exist vs. are planned.
9. Callback props follow the camelCase convention for custom events (`onClose`, `onNew`) and lowercase for DOM events (`onclick`, `onchange`).
10. Grep enforcement tests (Section 8.4) are defined and runnable, even if not yet blocking in CI.
