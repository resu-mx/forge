# Phase 77: Content Patterns Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development

**Goal:** Implement Entry, PaddedEntry, SectionedList, TagsList atoms + standardize form patterns with TitledDataInput. Fix existing component gaps (EmptyPanel/EmptyState, checkbox patterns).

**Depends on:** Phase 75 (layout tokens), Phase 76 (headers)

**Tech Stack:** Svelte 5 (runes mode), CSS design tokens, bun:test

**Key files:**
- Components: `packages/webui/src/lib/components/` (new files go here)
- Component barrel: `packages/webui/src/lib/components/index.ts`
- Global CSS: `packages/webui/src/lib/styles/base.css`
- Design tokens: `packages/webui/src/lib/styles/tokens.css`
- Tests: `packages/webui/src/__tests__/content-patterns.test.ts` (new)

**Non-goals:**
- Don't migrate ALL pages -- just reference pages (summaries, domains)
- Don't create OrgCombobox (noted as gap in spec, deferred)
- Don't create checkbox patterns (noted as gap in spec section 6.3, deferred)

**Internal task parallelization:**
- Tasks 77.1-77.4 are independent (different components)
- Task 77.5 is independent (CSS only)
- Task 77.6 is independent (existing component fix)
- Task 77.7 depends on 77.1-77.6 (tests validate all components)
- Task 77.8 depends on 77.1-77.4 (migrations use the new components)

---

## Task 77.1: Create Entry.svelte

**Files to create:**
- `packages/webui/src/lib/components/Entry.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)

**Satisfies acceptance criteria:** #1 (Entry selected state shows primary border)

### Implementation

Create `packages/webui/src/lib/components/Entry.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    onclick?: () => void
    selected?: boolean
    disabled?: boolean
    children: Snippet
  }

  let {
    onclick,
    selected = false,
    disabled = false,
    children,
  }: Props = $props()
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="entry"
  class:selected
  class:disabled
  role={onclick ? 'button' : undefined}
  tabindex={onclick ? 0 : undefined}
  onclick={onclick}
  onkeydown={(e) => {
    if (onclick && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault()
      onclick()
    }
  }}
>
  {@render children()}
</div>

<style>
  .entry {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    transition: background 0.12s;
    color: var(--text-primary);
    font-size: var(--text-sm);
    border-left: 3px solid transparent;
  }

  .entry:hover {
    background: var(--color-surface-raised);
  }

  .entry.selected {
    background: var(--color-primary-subtle);
    border-left-color: var(--color-primary);
  }

  .entry.disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }
</style>
```

Add to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as Entry } from './Entry.svelte'
```

### Verification

The test in 77.7 will check:
- `.entry.selected` contains `--color-primary`
- `.entry` has `border-left: 3px solid transparent`
- `.entry.disabled` has `opacity: 0.5` and `pointer-events: none`

---

## Task 77.2: Create PaddedEntry.svelte

**Files to create:**
- `packages/webui/src/lib/components/PaddedEntry.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)

**Satisfies acceptance criteria:** #2 (PaddedEntry no layout shift), #3 (PaddedEntry disabled state)

### Implementation

Create `packages/webui/src/lib/components/PaddedEntry.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    onclick?: () => void
    selected?: boolean
    disabled?: boolean
    variant?: 'default' | 'template'
    children: Snippet
  }

  let {
    onclick,
    selected = false,
    disabled = false,
    variant = 'default',
    children,
  }: Props = $props()
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="padded-entry"
  class:selected
  class:disabled
  class:padded-entry--template={variant === 'template'}
  role={onclick ? 'button' : undefined}
  tabindex={onclick ? 0 : undefined}
  onclick={onclick}
  onkeydown={(e) => {
    if (onclick && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault()
      onclick()
    }
  }}
>
  {@render children()}
</div>

<style>
  .padded-entry {
    padding: var(--space-4);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: box-shadow 0.12s, border-color 0.12s;
  }

  .padded-entry:hover {
    box-shadow: var(--shadow-sm);
    border-color: var(--color-border-strong);
  }

  .padded-entry.selected {
    border-left: 3px solid var(--color-primary);
    padding-left: calc(var(--space-4) - 2px);
    background: var(--color-primary-subtle);
  }

  .padded-entry.disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  .padded-entry--template {
    border-color: var(--color-template-border);
    background: var(--color-template-bg);
  }
</style>
```

Add to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as PaddedEntry } from './PaddedEntry.svelte'
```

### Verification

The test in 77.7 will check:
- `.padded-entry` always has `border: 1px solid` (1px base) and `.padded-entry.selected` has `border-left: 3px solid` with `padding-left: calc` to compensate -- no layout shift because the 2px difference is absorbed by padding reduction
- `.padded-entry.disabled` has `opacity: 0.5` and `pointer-events: none`

---

## Task 77.3: Create SectionedList.svelte

**Files to create:**
- `packages/webui/src/lib/components/SectionedList.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)

**Satisfies acceptance criteria:** #5 (SectionedList section headers with uppercase)

### Implementation

Create `packages/webui/src/lib/components/SectionedList.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  export interface SectionDef {
    title: string
    count?: number
    items: unknown[]
    icon?: string
  }

  interface Props {
    sections: SectionDef[]
    renderItem: Snippet<[item: unknown]>
    emptyMessage?: string
    hideWhenEmpty?: boolean
  }

  let {
    sections,
    renderItem,
    emptyMessage = 'No items',
    hideWhenEmpty = false,
  }: Props = $props()
</script>

<div class="sectioned-list">
  {#each sections as section (section.title)}
    {#if !hideWhenEmpty || section.items.length > 0}
      <div class="sectioned-list__section">
        <div class="sectioned-list__header">
          {#if section.icon}
            <span class="sectioned-list__icon">{section.icon}</span>
          {/if}
          <span class="sectioned-list__title">{section.title}</span>
          {#if section.count !== undefined}
            <span class="count-badge">{section.count}</span>
          {/if}
        </div>
        {#if section.items.length > 0}
          <div class="sectioned-list__items">
            {#each section.items as item}
              {@render renderItem(item)}
            {/each}
          </div>
        {:else}
          <p class="sectioned-list__empty">{emptyMessage}</p>
        {/if}
      </div>
    {/if}
  {/each}
</div>

<style>
  .sectioned-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  .sectioned-list__header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
    padding-bottom: var(--space-2);
    border-bottom: 1px solid var(--color-border);
  }

  .sectioned-list__title {
    font-size: var(--text-sm);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .sectioned-list__items {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .sectioned-list__empty {
    font-size: var(--text-sm);
    color: var(--text-faint);
    font-style: italic;
    padding: var(--space-2) 0;
  }
</style>
```

Add to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as SectionedList } from './SectionedList.svelte'
```

### Verification

The test in 77.7 will check:
- `sectioned-list__title` has `text-transform: uppercase`
- `sectioned-list__header` class exists
- File contains `count-badge` class reference

---

## Task 77.4: Create TagsList.svelte

**Files to create:**
- `packages/webui/src/lib/components/TagsList.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)

**Satisfies acceptance criteria:** #6 (TagsList remove aria-label)

### Implementation

Create `packages/webui/src/lib/components/TagsList.svelte`:

```svelte
<script lang="ts">
  interface Props {
    tags: string[]
    onRemove?: (tag: string) => void
    color?: 'accent' | 'neutral' | 'info' | 'success' | 'warning'
    size?: 'sm' | 'md'
  }

  let {
    tags,
    onRemove,
    color = 'accent',
    size = 'sm',
  }: Props = $props()
</script>

<div class="tags-list" role="list">
  {#each tags as tag (tag)}
    <span
      class="tags-list__pill tags-list__pill--{size} tags-list__pill--{color}"
      role="listitem"
    >
      {tag}
      {#if onRemove}
        <button
          class="tags-list__remove"
          type="button"
          aria-label="Remove {tag}"
          onclick={() => onRemove?.(tag)}
        >
          &times;
        </button>
      {/if}
    </span>
  {/each}
</div>

<style>
  .tags-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    align-items: center;
  }

  .tags-list__pill {
    display: inline-flex;
    align-items: center;
    gap: 0.15em;
    border-radius: var(--radius-sm);
    font-weight: var(--font-medium);
    white-space: nowrap;
    line-height: 1.4;
  }

  /* Size variants */
  .tags-list__pill--sm {
    padding: 0.05em 0.3em;
    font-size: var(--text-xs);
  }

  .tags-list__pill--md {
    padding: 0.15em 0.45em;
    font-size: var(--text-sm);
  }

  /* Color variants */
  .tags-list__pill--accent {
    background: var(--color-tag-bg);
    color: var(--color-tag-text);
  }

  .tags-list__pill--neutral {
    background: var(--color-tag-neutral-bg);
    color: var(--color-tag-neutral-text);
  }

  .tags-list__pill--info {
    background: var(--color-info-subtle);
    color: var(--color-info-text);
  }

  .tags-list__pill--success {
    background: var(--color-success-subtle);
    color: var(--color-success-text);
  }

  .tags-list__pill--warning {
    background: var(--color-warning-subtle);
    color: var(--color-warning-text);
  }

  .tags-list__remove {
    background: none;
    border: none;
    font-size: 0.85em;
    line-height: 1;
    cursor: pointer;
    color: inherit;
    opacity: 0.6;
    padding: 0;
  }

  .tags-list__remove:hover {
    opacity: 1;
  }
</style>
```

Add to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as TagsList } from './TagsList.svelte'
```

### Verification

The test in 77.7 will check:
- File contains `aria-label="Remove {tag}"` pattern
- File contains `role="list"` and `role="listitem"`

---

## Task 77.5: Standardize form field CSS in base.css

**Files to modify:**
- `packages/webui/src/lib/styles/base.css`

**Satisfies acceptance criteria:** #4 (Forms use TitledDataInput pattern)

### Current state

`base.css` already has `.form-field`, `.field-label`, `.field-input`, `.field-select`, `.form-group`, and `.form-actions`. What's missing from the spec:

1. `.field-hint` class (help text below inputs)
2. `.field-required` class (red asterisk)
3. `.form-field--inline` class (checkbox layout)
4. `.field-input:disabled` state
5. `.field-input::placeholder` styling
6. `textarea.field-input` needs `min-height: 60px`
7. `.field-input` and `.field-select` should use `var(--text-sm)` not `var(--text-base)` per spec
8. `.field-input` needs `width: 100%` and `transition` per spec
9. `.form-group` gap should be `var(--space-4)` per spec (currently `var(--space-2)`)
10. `.form-actions` needs `padding-top` and `border-top` per spec
11. `.form-section-heading` class (form section dividers)
12. `.form-grid` class (two-column form layout)

### Implementation

In `packages/webui/src/lib/styles/base.css`, replace the existing form input section:

Find:
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

Replace with:
```css
/* ---- Form inputs ---- */
.form-field {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.form-field--inline {
  flex-direction: row;
  align-items: center;
  gap: var(--space-2);
}

.form-field--inline input[type="checkbox"] {
  width: 1rem;
  height: 1rem;
  accent-color: var(--color-primary);
}

.field-label {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--text-secondary);
}

.field-required {
  color: var(--color-danger);
}

.field-hint {
  font-size: var(--text-xs);
  color: var(--text-faint);
  margin-top: calc(-1 * var(--space-1));
}

.field-input {
  padding: 0.375rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-family: inherit;
  color: var(--text-primary);
  background: var(--color-surface);
  transition: border-color 0.15s, box-shadow 0.15s;
  width: 100%;
}

.field-input:focus {
  outline: none;
  border-color: var(--color-border-focus);
  box-shadow: 0 0 0 2px var(--color-primary-subtle);
}

.field-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: var(--color-surface-sunken);
}

.field-input::placeholder {
  color: var(--text-faint);
}

/* Select */
.field-select {
  padding: 0.375rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-family: inherit;
  color: var(--text-primary);
  background: var(--color-surface);
  cursor: pointer;
  width: 100%;
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
  min-height: 60px;
}

/* Form layout */
.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.form-actions {
  display: flex;
  gap: var(--space-2);
  padding-top: var(--space-4);
  border-top: 1px solid var(--color-border);
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-4);
}

.form-grid .full-width {
  grid-column: 1 / -1;
}

.form-section-heading {
  font-size: var(--text-base);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
  margin-top: var(--space-6);
  margin-bottom: var(--space-2);
  padding-top: var(--space-4);
  border-top: 1px solid var(--color-border);
}
```

### Verification

The test in 77.7 will check:
- `base.css` contains `.form-field` with `flex-direction: column`
- `base.css` contains `.field-label` with `--font-medium`
- `base.css` contains `.field-hint`
- `base.css` contains `.field-required`
- `base.css` contains `.form-section-heading`

---

## Task 77.6: Fix EmptyPanel and EmptyState token usage

**Files to modify:**
- `packages/webui/src/lib/components/EmptyPanel.svelte`
- `packages/webui/src/lib/components/EmptyState.svelte`

**Satisfies acceptance criteria:** #8 (EmptyPanel centered italic), #9 (EmptyState title + description + CTA)

### Current issues

**EmptyPanel:** Currently has an `actionLabel`/`onAction` prop and action button styling. Per spec, EmptyPanel is a lightweight "nothing selected" placeholder -- no CTA. It should also use `font-style: italic` and `--text-faint` per spec. The current component uses `--text-muted` (too prominent) and is missing `font-style: italic`.

**EmptyState:** Has hardcoded padding (`3rem 2rem`) instead of tokens (`--space-12`). Font size `0.9rem` instead of `--text-base`. Missing `.empty-state__cta` wrapper class. Uses inline props (`action`/`onaction`) instead of `children` snippet for CTA. Class names use `.title` and `.description` (too generic) instead of BEM-prefixed `.empty-state__title` and `.empty-state__description`.

### Implementation

Replace `packages/webui/src/lib/components/EmptyPanel.svelte` with:

```svelte
<script lang="ts">
  interface Props {
    message?: string
  }

  let {
    message = 'Select an item to view details.',
  }: Props = $props()
</script>

<div class="empty-panel">
  <p class="empty-panel__message">{message}</p>
</div>

<style>
  .empty-panel {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-faint);
    font-size: var(--text-sm);
    font-style: italic;
  }

  .empty-panel__message {
    text-align: center;
    line-height: var(--leading-normal);
  }
</style>
```

Replace `packages/webui/src/lib/components/EmptyState.svelte` with:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    title: string
    description?: string
    children?: Snippet
  }

  let {
    title,
    description,
    children,
  }: Props = $props()
</script>

<div class="empty-state">
  <h3 class="empty-state__title">{title}</h3>
  {#if description}
    <p class="empty-state__description">{description}</p>
  {/if}
  {#if children}
    <div class="empty-state__cta">
      {@render children()}
    </div>
  {/if}
</div>

<style>
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

  .empty-state__title {
    font-size: var(--text-lg);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
  }

  .empty-state__description {
    font-size: var(--text-base);
    line-height: var(--leading-normal);
    max-width: 400px;
  }

  .empty-state__cta {
    margin-top: var(--space-2);
  }
</style>
```

### Breaking change: EmptyState API

The old EmptyState used `action` (string) and `onaction` (callback) props. The new one uses a `children` snippet for the CTA. All call sites must be updated.

Search for existing usage:

```
grep -r "EmptyState" packages/webui/src/routes/ --include="*.svelte"
```

Each call site like:
```svelte
<EmptyState
  title="No summaries yet"
  description="Create one to get started."
  action="Create Summary"
  onaction={startCreate}
/>
```

Must become:
```svelte
<EmptyState
  title="No summaries yet"
  description="Create one to get started."
>
  <button class="btn btn-primary" onclick={startCreate}>Create Summary</button>
</EmptyState>
```

The task implementer MUST grep for all `<EmptyState` usages and update them. Known call sites at plan creation time:

1. `packages/webui/src/routes/resumes/summaries/+page.svelte` (line 180-185)
2. `packages/webui/src/routes/data/domains/DomainsView.svelte` (line 167) -- note this one uses a `message` prop that doesn't exist in either old or new API; this is a bug to fix

For DomainsView line 167, the current code is:
```svelte
<EmptyState message="No domains found. Create one to get started." />
```

This should become:
```svelte
<EmptyState title="No domains found" description="Create one to get started." />
```

The implementer MUST search for all other EmptyState usages and update each one.

### Verification

The test in 77.7 will check:
- EmptyPanel has `font-style: italic` and `--text-faint`
- EmptyPanel has `display: flex`, `align-items: center`, `justify-content: center`
- EmptyState has `.empty-state__title`, `.empty-state__description`, `.empty-state__cta`
- EmptyState has no hardcoded rem/px padding values

---

## Task 77.7: Write ALL acceptance tests

**Files to create:**
- `packages/webui/src/__tests__/content-patterns.test.ts`

**Satisfies acceptance criteria:** All 10 criteria verified

### Implementation

Create `packages/webui/src/__tests__/content-patterns.test.ts`:

```typescript
import { describe, test, expect } from 'bun:test'
import { readFileSync, readdirSync } from 'fs'
import { join } from 'path'

const COMPONENTS = join(import.meta.dir, '..', 'lib', 'components')
const STYLES = join(import.meta.dir, '..', 'lib', 'styles')
const ROUTES = join(import.meta.dir, '..', 'routes')

function read(path: string): string {
  return readFileSync(path, 'utf-8')
}

function readComponent(name: string): string {
  return read(join(COMPONENTS, name))
}

function readCSS(name: string): string {
  return read(join(STYLES, name))
}

/** Recursively find all .svelte files under a directory */
function findSvelteFiles(dir: string): string[] {
  const results: string[] = []
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = join(dir, entry.name)
    if (entry.isDirectory()) {
      results.push(...findSvelteFiles(fullPath))
    } else if (entry.name.endsWith('.svelte')) {
      results.push(fullPath)
    }
  }
  return results
}

describe('Content Patterns', () => {
  // AC #1: Entry selected state shows primary border
  describe('Entry.svelte', () => {
    const content = readComponent('Entry.svelte')

    test('selected state uses --color-primary for border', () => {
      expect(content).toContain('.entry.selected')
      expect(content).toContain('--color-primary')
    })

    test('has always-present transparent left border', () => {
      expect(content).toMatch(/border-left:\s*3px\s+solid\s+transparent/)
    })

    test('selected state has --color-primary-subtle background', () => {
      expect(content).toContain('--color-primary-subtle')
    })

    test('has role="button" for interactive entries', () => {
      expect(content).toContain('role=')
      expect(content).toContain('tabindex=')
    })

    test('handles keyboard activation (Enter and Space)', () => {
      expect(content).toContain("e.key === 'Enter'")
      expect(content).toContain("e.key === ' '")
    })
  })

  // AC #2: PaddedEntry no layout shift on selection
  describe('PaddedEntry.svelte', () => {
    const content = readComponent('PaddedEntry.svelte')

    test('base state has 1px border', () => {
      expect(content).toMatch(/\.padded-entry\s*\{[^}]*border:\s*1px\s+solid/)
    })

    test('selected state compensates border width with padding-left calc', () => {
      expect(content).toContain('.padded-entry.selected')
      expect(content).toMatch(/padding-left:\s*calc/)
    })

    test('selected state uses 3px primary border', () => {
      expect(content).toMatch(/border-left:\s*3px\s+solid\s+var\(--color-primary\)/)
    })

    // AC #3: PaddedEntry disabled state
    test('disabled state has opacity 0.5 and pointer-events none', () => {
      expect(content).toContain('.padded-entry.disabled')
      expect(content).toContain('opacity: 0.5')
      expect(content).toContain('pointer-events: none')
    })

    test('supports template variant', () => {
      expect(content).toContain('padded-entry--template')
      expect(content).toContain('--color-template-border')
      expect(content).toContain('--color-template-bg')
    })
  })

  // AC #4: Forms use TitledDataInput pattern
  describe('TitledDataInput (base.css)', () => {
    const content = readCSS('base.css')

    test('has .form-field with column layout', () => {
      expect(content).toContain('.form-field')
      expect(content).toMatch(/\.form-field\s*\{[^}]*flex-direction:\s*column/)
    })

    test('has .field-label with correct styling', () => {
      expect(content).toContain('.field-label')
      expect(content).toMatch(/\.field-label\s*\{[^}]*--font-medium/)
    })

    test('has .field-hint class', () => {
      expect(content).toContain('.field-hint')
      expect(content).toContain('--text-faint')
    })

    test('has .field-required class', () => {
      expect(content).toContain('.field-required')
      expect(content).toContain('--color-danger')
    })

    test('has .form-section-heading class', () => {
      expect(content).toContain('.form-section-heading')
    })

    test('has .form-grid class for two-column layouts', () => {
      expect(content).toContain('.form-grid')
      expect(content).toContain('grid-template-columns: 1fr 1fr')
    })

    test('.field-input has disabled state', () => {
      expect(content).toContain('.field-input:disabled')
      expect(content).toContain('--color-surface-sunken')
    })

    test('.field-input has placeholder styling', () => {
      expect(content).toContain('.field-input::placeholder')
    })

    test('.field-input uses var(--text-sm) not var(--text-base)', () => {
      // Match .field-input { ... font-size: var(--text-sm) ... }
      const fieldInputBlock = content.match(/\.field-input\s*\{[^}]*\}/)?.[0] ?? ''
      expect(fieldInputBlock).toContain('--text-sm')
      expect(fieldInputBlock).not.toContain('--text-base')
    })
  })

  // AC #5: SectionedList section headers with uppercase
  describe('SectionedList.svelte', () => {
    const content = readComponent('SectionedList.svelte')

    test('renders section header elements', () => {
      expect(content).toContain('sectioned-list__header')
    })

    test('section title has uppercase styling', () => {
      expect(content).toContain('text-transform: uppercase')
    })

    test('uses count-badge class for counts', () => {
      expect(content).toContain('count-badge')
    })

    test('has empty message with italic styling', () => {
      expect(content).toContain('sectioned-list__empty')
      expect(content).toContain('font-style: italic')
    })
  })

  // AC #6: TagsList remove aria-label
  describe('TagsList.svelte', () => {
    const content = readComponent('TagsList.svelte')

    test('remove button has aria-label with tag name interpolation', () => {
      expect(content).toMatch(/aria-label="Remove \{tag\}"/)
    })

    test('has role="list" on container', () => {
      expect(content).toContain('role="list"')
    })

    test('has role="listitem" on each tag', () => {
      expect(content).toContain('role="listitem"')
    })

    test('supports color variants', () => {
      expect(content).toContain('tags-list__pill--accent')
      expect(content).toContain('tags-list__pill--neutral')
      expect(content).toContain('tags-list__pill--info')
      expect(content).toContain('tags-list__pill--success')
      expect(content).toContain('tags-list__pill--warning')
    })

    test('supports size variants', () => {
      expect(content).toContain('tags-list__pill--sm')
      expect(content).toContain('tags-list__pill--md')
    })
  })

  // AC #7: Detail components accept onClose as optional
  describe('Detail component onClose', () => {
    test('ChainViewModal has optional onClose prop', () => {
      const content = readComponent('ChainViewModal.svelte')
      expect(content).toMatch(/onClose\??:\s*\(\)\s*=>\s*void/)
    })
  })

  // AC #8: EmptyPanel renders centered italic
  describe('EmptyPanel.svelte', () => {
    const content = readComponent('EmptyPanel.svelte')

    test('has centered flex layout', () => {
      expect(content).toContain('display: flex')
      expect(content).toContain('align-items: center')
      expect(content).toContain('justify-content: center')
    })

    test('uses italic font style', () => {
      expect(content).toContain('font-style: italic')
    })

    test('uses --text-faint color (not --text-muted)', () => {
      expect(content).toContain('--text-faint')
    })

    test('has no action button (lightweight placeholder only)', () => {
      expect(content).not.toContain('actionLabel')
      expect(content).not.toContain('onAction')
      expect(content).not.toContain('<button')
    })
  })

  // AC #9: EmptyState renders title + description + CTA
  describe('EmptyState.svelte', () => {
    const content = readComponent('EmptyState.svelte')

    test('has title element with BEM class', () => {
      expect(content).toContain('empty-state__title')
    })

    test('has description element with BEM class', () => {
      expect(content).toContain('empty-state__description')
    })

    test('has CTA wrapper with BEM class', () => {
      expect(content).toContain('empty-state__cta')
    })

    test('uses design tokens for padding (no hardcoded rem/px)', () => {
      const styleBlock = content.match(/<style[^>]*>([\s\S]*?)<\/style>/)?.[1] ?? ''
      // Padding should be var(--space-12), not 3rem or 2rem
      expect(styleBlock).not.toMatch(/padding:\s*\d+(\.\d+)?rem/)
      expect(styleBlock).toContain('--space-12')
    })

    test('uses children snippet for CTA (not action/onaction props)', () => {
      expect(content).toContain('children')
      expect(content).toContain('@render children()')
      expect(content).not.toMatch(/\bonaction\b/)
    })
  })

  // AC #10: No Svelte 4 event modifier syntax
  describe('No Svelte 4 syntax', () => {
    const allSvelteFiles = [
      ...findSvelteFiles(join(import.meta.dir, '..', 'lib', 'components')),
      ...findSvelteFiles(ROUTES),
    ]

    test('no |preventDefault or |stopPropagation modifiers in any .svelte file', () => {
      const violations: string[] = []
      for (const file of allSvelteFiles) {
        const content = read(file)
        if (content.includes('|preventDefault') || content.includes('|stopPropagation')) {
          violations.push(file)
        }
      }
      expect(violations).toEqual([])
    })
  })
})
```

### Run command

```bash
cd packages/webui && bun test src/__tests__/content-patterns.test.ts
```

---

## Task 77.8: Migrate reference pages

**Files to modify:**
- `packages/webui/src/routes/resumes/summaries/+page.svelte` (migrate to PaddedEntry + SectionedList)
- `packages/webui/src/routes/data/domains/DomainsView.svelte` (migrate form inputs to `.field-input`, fix EmptyState usage)

**Depends on:** Tasks 77.1-77.4, 77.5, 77.6

### 77.8a: Migrate summaries page to PaddedEntry + SectionedList

The summaries page currently:
- Manually renders `.summary-card` divs (should use `PaddedEntry`)
- Manually renders section headers for Templates vs Summaries (should use `SectionedList`)
- Uses old EmptyState API with `action`/`onaction` props (should use children snippet)
- Defines `.form-field`, `.field-label`, `.field-input` in scoped CSS (should use global classes from base.css)

**Changes:**

1. Add imports for `PaddedEntry`, `SectionedList` from `$lib/components`
2. Replace `<EmptyState ... action="Create Summary" onaction={startCreate} />` with:
   ```svelte
   <EmptyState title="No summaries yet" description="Create one to get started. Summaries can be linked to resumes or promoted to templates for reuse.">
     <button class="btn btn-primary" onclick={startCreate}>Create Summary</button>
   </EmptyState>
   ```
3. Replace the manual Templates/Summaries sections with `<SectionedList>`:
   ```svelte
   <SectionedList
     sections={[
       { title: 'Templates', count: templates.length, icon: '\u2733', items: templates },
       { title: 'Summaries', count: instances.length, items: instances },
     ]}
     hideWhenEmpty={true}
   >
     {#snippet renderItem(item)}
       {@const summary = item as Summary}
       <PaddedEntry
         selected={editing === summary.id}
         variant={summary.is_template ? 'template' : 'default'}
       >
         {#if editing === summary.id}
           {@render editForm(summary)}
         {:else}
           {@render summaryDisplay(summary)}
         {/if}
       </PaddedEntry>
     {/snippet}
   </SectionedList>
   ```
4. Remove scoped CSS for `.summary-card`, `.summary-card.template`, `.section`, `.section-title`, `.star`, `.form-field`, `.field-label`, `.field-input`, `.field-input:focus` (all now handled by PaddedEntry/SectionedList/base.css globals)
5. Keep page-specific CSS for `.summaries-page`, `.page-header`, `.page-title`, `.subtitle`, `.summary-display`, `.summary-info`, `.summary-header`, `.summary-title`, `.resume-count`, `.summary-role`, `.summary-tagline`, `.summary-actions`, `.edit-form`, `.edit-actions`, `.banner`, `.banner-info`, `.banner-warn`
6. Refactor the `summaryRow` snippet into two snippets: `editForm(summary)` and `summaryDisplay(summary)` to work inside PaddedEntry children

### 77.8b: Migrate DomainsView form inputs

The DomainsView currently:
- Defines its own `.form-input`, `.form-label`, `.form-hint`, `.form-row` scoped CSS
- Uses `border-radius: 6px` instead of `var(--radius-md)`
- Uses padding `0.5rem 0.75rem` instead of spec's `0.375rem 0.625rem`
- Uses `<EmptyState message="...">` which is a non-existent prop

**Changes:**

1. Replace `class="form-input"` with `class="field-input"` on all inputs
2. Replace `class="form-input compact"` with `class="field-input"` (compact is a local concern; if needed, add a scoped override)
3. Replace `class="form-label"` with `class="field-label"`
4. Replace `class="form-hint"` with `class="field-hint"`
5. Replace `class="form-row"` with `class="form-group"` (or keep as scoped layout class for the horizontal create form)
6. Fix EmptyState usage from `<EmptyState message="..." />` to `<EmptyState title="No domains found" description="Create one to get started." />`
7. Remove scoped CSS for `.form-input`, `.form-label`, `.form-hint` (use globals from base.css)
8. Remove scoped `.btn` / `.btn-primary` / `.btn-ghost` / `.btn-danger` / `.btn-sm` redefinitions (use globals from base.css)
9. Replace `border-radius: 8px` with `var(--radius-lg)` in `.create-form`
10. Replace `border-radius: 6px` with `var(--radius-md)` in remaining scoped styles

### Verification

After migration:
- Summaries page renders identically (visual inspection)
- Summaries page no longer has scoped `.form-field`, `.field-label`, `.field-input` CSS
- DomainsView no longer has `.form-input` or `.form-label` classes
- DomainsView no longer redefines `.btn` scoped CSS
- Both pages use global classes from base.css

---

## Acceptance Criteria Traceability

| # | Criterion | Task | Test |
|---|-----------|------|------|
| 1 | Entry selected state shows primary border | 77.1 | `Entry.svelte > selected state uses --color-primary for border` |
| 2 | PaddedEntry no layout shift | 77.2 | `PaddedEntry.svelte > selected state compensates border width with padding-left calc` |
| 3 | PaddedEntry disabled state | 77.2 | `PaddedEntry.svelte > disabled state has opacity 0.5 and pointer-events none` |
| 4 | Forms use TitledDataInput | 77.5 | `TitledDataInput (base.css) > has .form-field with column layout` + related |
| 5 | SectionedList section headers | 77.3 | `SectionedList.svelte > section title has uppercase styling` |
| 6 | TagsList remove aria-label | 77.4 | `TagsList.svelte > remove button has aria-label with tag name interpolation` |
| 7 | Detail components accept onClose optional | -- (verify existing) | `Detail component onClose > ChainViewModal has optional onClose prop` |
| 8 | EmptyPanel centered italic | 77.6 | `EmptyPanel.svelte > uses italic font style` + `has centered flex layout` |
| 9 | EmptyState title + description + CTA | 77.6 | `EmptyState.svelte > has title/description/CTA elements` |
| 10 | No Svelte 4 syntax | -- (verify existing) | `No Svelte 4 syntax > no \|preventDefault or \|stopPropagation` |

---

## Final index.ts state

After all tasks, `packages/webui/src/lib/components/index.ts` should contain:

```typescript
export { default as Toast } from './Toast.svelte'
export { default as ToastContainer } from './ToastContainer.svelte'
export { default as StatusBadge } from './StatusBadge.svelte'
export { default as LoadingSpinner } from './LoadingSpinner.svelte'
export { default as EmptyState } from './EmptyState.svelte'
export { default as ConfirmDialog } from './ConfirmDialog.svelte'
export { default as DriftBanner } from './DriftBanner.svelte'
export { default as PageWrapper } from './PageWrapper.svelte'
export { default as SplitPanel } from './SplitPanel.svelte'
export { default as ListPanelHeader } from './ListPanelHeader.svelte'
export { default as PageHeader } from './PageHeader.svelte'
export { default as ProfileMenu } from './ProfileMenu.svelte'
export { default as TabBar } from './TabBar.svelte'
export { default as EmptyPanel } from './EmptyPanel.svelte'
export { default as ListSearchInput } from './ListSearchInput.svelte'
export { default as Entry } from './Entry.svelte'
export { default as PaddedEntry } from './PaddedEntry.svelte'
export { default as SectionedList } from './SectionedList.svelte'
export { default as TagsList } from './TagsList.svelte'
```
