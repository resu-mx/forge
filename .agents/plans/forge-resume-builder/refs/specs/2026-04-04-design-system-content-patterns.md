# Forge Design System: Content Patterns

**Date:** 2026-04-04
**Doc:** 4 of 6 (Design System Series)
**Status:** Reference specification
**Depends on:** Doc 1 (Design System Foundation)

This document specifies the content-level components and atoms that populate views and panels. These are the building blocks that render data rows, form inputs, tag collections, and entity detail editors. Every component here lives in the Component or Atom layer (see Doc 1, Section 1).

---

## 1. Entry

**Layer:** Component
**Purpose:** Minimal list row for dense data lists. No background, no border, no padding opinion. Inherits visual context from its parent container. Used in domain tables, education items, role items, and any flat list where density matters.

### 1.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `onclick` | `() => void` | -- | Click handler; makes the entry interactive |
| `selected` | `boolean` | `false` | Whether this entry is visually selected |
| `disabled` | `boolean` | `false` | Dims the entry and disables interaction |
| `children` | `Snippet` | -- | Content to render inside the entry |

### 1.2 CSS Specification

```css
/* Entry.svelte - scoped styles */
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
```

### 1.3 Usage

```svelte
<!-- DO THIS: Entry in a dense list -->
<ul class="domain-list">
  {#each domains as domain (domain.id)}
    <li>
      <Entry onclick={() => select(domain.id)} selected={selectedId === domain.id}>
        <span class="domain-name">{domain.name}</span>
        <span class="domain-count">{domain.perspective_count}</span>
      </Entry>
    </li>
  {/each}
</ul>
```

```svelte
<!-- DON'T DO THIS: Adding background/border/padding to Entry -->
<!-- Entry inherits from its parent. If you need a card, use PaddedEntry. -->
<Entry onclick={handler}>
  <div style="background: white; padding: 1rem; border: 1px solid #ccc;">
    {item.name}
  </div>
</Entry>
```

### 1.4 Key Distinction from PaddedEntry

Entry has **no visual container of its own**. It is a transparent row that lets the parent's background show through. The only visual states are hover highlight and selected accent border. If the item needs to look like a standalone card, use PaddedEntry instead.

---

## 2. PaddedEntry

**Layer:** Component
**Purpose:** Card-style list item with its own background, border, border-radius, and padding. Used in summaries, templates, kanban cards, notes list, resume cards, and any context where items should appear as distinct visual cards.

### 2.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `onclick` | `() => void` | -- | Click handler |
| `selected` | `boolean` | `false` | Whether this entry is visually selected |
| `disabled` | `boolean` | `false` | Dims the entry and disables interaction |
| `variant` | `'default' \| 'template'` | `'default'` | Visual variant for special card types |
| `children` | `Snippet` | -- | Content to render inside the card |

### 2.2 CSS Specification

```css
/* PaddedEntry.svelte - scoped styles */
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
  padding-left: calc(var(--space-4) - 3px); /* compensate for 3px border vs 1px default = 2px difference, but we subtract 3px to account for border-left replacing 1px of the original border */
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
```

### 2.3 Usage

```svelte
<!-- DO THIS: PaddedEntry as a summary card -->
<div class="summary-list">
  {#each summaries as summary (summary.id)}
    <PaddedEntry
      onclick={() => select(summary.id)}
      selected={selectedId === summary.id}
      variant={summary.is_template ? 'template' : 'default'}
    >
      <div class="summary-display">
        <h3 class="summary-title">{summary.title}</h3>
        {#if summary.role}
          <p class="summary-role">{summary.role}</p>
        {/if}
      </div>
    </PaddedEntry>
  {/each}
</div>
```

```svelte
<!-- DON'T DO THIS: Nesting PaddedEntry inside another card wrapper -->
<!-- PaddedEntry IS the card. Don't double-wrap. -->
<div class="card">
  <PaddedEntry onclick={handler}>
    {item.name}
  </PaddedEntry>
</div>
```

### 2.4 Current Pattern (summaries page)

The summaries page currently implements PaddedEntry inline as `.summary-card`:

```css
/* Current pattern in summaries/+page.svelte */
.summary-card {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 1rem;
  background: var(--color-surface);
}
.summary-card.template {
  border-color: var(--color-template-border);
  background: var(--color-template-bg);
}
```

This is the exact pattern PaddedEntry extracts. The component replaces these repeated inline styles.

---

## 3. SectionedList

**Layer:** Component
**Purpose:** Groups items under labeled section headers with visual dividers. Used when a single list contains categorized subgroups (summaries vs templates, skills by category, bullets by status).

### 3.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `sections` | `SectionDef[]` | -- | Array of section definitions |
| `renderItem` | `Snippet<[item: unknown]>` | -- | Snippet to render each item |
| `emptyMessage` | `string` | `'No items'` | Message when a section is empty |
| `hideWhenEmpty` | `boolean` | `false` | When true, sections with zero items are not rendered at all |

```typescript
export interface SectionDef {
  title: string
  count?: number
  items: unknown[]
  icon?: string      // Optional icon/emoji before title
}
```

### 3.2 CSS Specification

```css
/* SectionedList.svelte - scoped styles */
.sectioned-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
}

.sectioned-list__section {
  /* no extra styling -- sections are visual groups */
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

.sectioned-list__count {
  /* Uses global .count-badge class from base.css */
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
```

### 3.3 Usage

```svelte
<!-- DO THIS: SectionedList with templates and regular summaries -->
<SectionedList
  sections={[
    { title: 'Templates', count: templates.length, icon: '\u2733', items: templates },
    { title: 'Summaries', count: instances.length, items: instances },
  ]}
>
  {#snippet renderItem(summary)}
    <PaddedEntry onclick={() => select(summary.id)}>
      <h3>{summary.title}</h3>
    </PaddedEntry>
  {/snippet}
</SectionedList>
```

```svelte
<!-- DON'T DO THIS: Manually repeating section headers -->
<!-- Use SectionedList instead of duplicating the section header pattern. -->
{#if templates.length > 0}
  <section>
    <h2 class="section-title">Templates</h2>
    <div class="list">{#each templates as t}...{/each}</div>
  </section>
{/if}
<section>
  <h2 class="section-title">Summaries</h2>
  <div class="list">{#each instances as s}...{/each}</div>
</section>
```

### 3.4 Current Pattern (summaries page)

The summaries page manually builds sectioned groups:

```svelte
<!-- Current inline pattern -->
<section class="section">
  <h2 class="section-title"><span class="star">&#9733;</span> Templates</h2>
  <div class="summary-list">
    {#each templates as summary}...{/each}
  </div>
</section>
<section class="section">
  <h2 class="section-title">Summaries</h2>
  ...
</section>
```

SectionedList extracts this repeating section + header + items pattern into a single component.

---

## 4. TagsList

**Layer:** Atom
**Purpose:** Inline horizontal list of tag pills. Each tag is a small pill with a background color and optional remove button. Used for org tags on kanban cards, source skills, bullet technologies, note entity references, and any list of labels.

### 4.1 API

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `tags` | `string[]` | -- | Array of tag labels to display |
| `onRemove` | `(tag: string) => void` | -- | If provided, each tag shows a remove button |
| `color` | `'accent' \| 'neutral' \| 'info' \| 'success' \| 'warning'` | `'accent'` | Color variant for the pills |
| `size` | `'sm' \| 'md'` | `'sm'` | Pill size |

### 4.2 CSS Specification

```css
/* TagsList.svelte - scoped styles */
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

/* Color variants -- consume global pill tokens and status tokens */
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
```

### 4.3 Usage

```svelte
<!-- DO THIS: Tags on a kanban card -->
{#if org.tags && org.tags.length > 0}
  <TagsList tags={org.tags} color="accent" />
{/if}

<!-- DO THIS: Removable technology tags in an editor -->
<TagsList
  tags={editTechnologies}
  onRemove={(tech) => removeTechnology(tech)}
  color="info"
  size="md"
/>
```

```svelte
<!-- DON'T DO THIS: Manually building tag pills inline -->
<!-- Use TagsList instead of repeating the pill pattern. -->
<div class="tag-pills">
  {#each tags as tag}
    <span class="pill">{tag}</span>
  {/each}
</div>
```

### 4.4 Relationship to Global `.pill` Class

The global `.pill` class in `base.css` provides the base pill styling. TagsList uses this as its foundation but adds the flex container, remove button, color variants, and size variants. Standalone `.pill` elements (e.g., a single status label) can still use the global class directly. TagsList is for **lists** of tags.

---

## 4.5 OrgCombobox

**File:** `$lib/components/OrgCombobox.svelte`
**Layer:** Component
**Status:** Exists but not yet documented

`OrgCombobox` is a searchable select/create combo box for choosing organizations. It provides type-ahead search with the ability to create new organizations inline. This component is a gap in the current spec -- a full API and CSS specification should be added when the component is stabilized.

**Known usage:** Source editor (organization selection), JD editor (employer selection), resume target employer field.

---

## 5. DataInput

**Layer:** Atom
**Purpose:** Styled form input with consistent border, border-radius, padding, focus ring, and disabled state. Covers text, number, date, email, tel, and url input types. This is the atomic building block for all text-entry fields in the system.

### 5.1 CSS Specification (Global Class)

DataInput is implemented as the global `.field-input` CSS class in `base.css`, not as a Svelte component. This follows the same rationale as buttons (ADR-003): inputs are native HTML elements that benefit from zero-abstraction CSS classes.

```css
/* base.css - already defined */
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

/* Textarea variant */
textarea.field-input {
  resize: vertical;
  line-height: var(--leading-normal);
  min-height: 60px;
}

/* Select variant */
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
```

### 5.2 Usage

```svelte
<!-- DO THIS: Text input with field-input class -->
<input type="text" class="field-input" bind:value={name} placeholder="Enter name" />

<!-- DO THIS: Textarea with field-input class -->
<textarea class="field-input" bind:value={description} rows="4" placeholder="Description..." />

<!-- DO THIS: Select with field-select class -->
<select class="field-select" bind:value={status}>
  <option value="draft">Draft</option>
  <option value="active">Active</option>
</select>

<!-- DO THIS: Number input -->
<input type="number" class="field-input" bind:value={salary} step="1000" placeholder="120000" />
```

```svelte
<!-- DON'T DO THIS: Inline styling on inputs -->
<input
  type="text"
  style="padding: 0.5rem; border: 1px solid #ccc; border-radius: 6px;"
  bind:value={name}
/>

<!-- DON'T DO THIS: Redefining input styles in component scoped CSS -->
<style>
  input {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--color-border-strong);
    border-radius: 6px;    /* should use var(--radius-md) */
    font-size: 0.875rem;   /* should use var(--text-sm) */
  }
</style>
```

### 5.3 Current Inconsistencies

Several pages define their own input styles instead of using `.field-input`:

| Page | Current Class | Issue |
|------|--------------|-------|
| `config/profile` | `.form-field input` | Padding `0.5rem 0.75rem` vs standard `0.375rem 0.625rem`; radius `6px` instead of `var(--radius-md)` |
| `data/domains/DomainsView` | `.form-input` | Padding `0.5rem 0.75rem`; radius `6px` instead of `var(--radius-md)` |
| `BulletDetailModal` | `.field-textarea` | Radius `6px` instead of `var(--radius-md)` |

All of these should migrate to `.field-input` and `.field-select` from `base.css`.

---

## 6. TitledDataInput

**Layer:** Atom
**Purpose:** Composed atom pairing a label with a DataInput. Provides consistent label styling, spacing between label and input, and optional help text. This is the standard way to create labeled form fields throughout the application.

### 6.1 CSS Specification (Global Class)

TitledDataInput is implemented as the global `.form-field` + `.field-label` CSS classes in `base.css`:

```css
/* base.css - already defined */
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

/* Help text (to be added to base.css) */
.field-hint {
  font-size: var(--text-xs);
  color: var(--text-faint);
  margin-top: calc(-1 * var(--space-1)); /* tighten gap between input and hint */
}

/* Required indicator */
.field-required {
  color: var(--color-danger);
}
```

### 6.2 Usage

```svelte
<!-- DO THIS: Standard labeled text input -->
<div class="form-field">
  <label class="field-label" for="pf-name">
    Name <span class="field-required">*</span>
  </label>
  <input id="pf-name" type="text" class="field-input" bind:value={name} required />
</div>

<!-- DO THIS: Labeled textarea -->
<div class="form-field">
  <label class="field-label" for="desc">Description</label>
  <textarea id="desc" class="field-input" bind:value={description} rows="4" />
</div>

<!-- DO THIS: Labeled select with help text -->
<div class="form-field">
  <label class="field-label" for="domain">Domain</label>
  <select id="domain" class="field-select" bind:value={selectedDomain}>
    <option value={null}>-- No domain --</option>
    {#each domains as d}
      <option value={d.name}>{d.name}</option>
    {/each}
  </select>
  <span class="field-hint">The experience domain this bullet belongs to</span>
</div>
```

```svelte
<!-- DON'T DO THIS: Inconsistent label styling -->
<label style="font-size: 0.8rem; font-weight: 600; color: var(--text-secondary);">
  Name
</label>
<input type="text" bind:value={name} />

<!-- DON'T DO THIS: Using different class names for the same pattern -->
<div class="form-row">
  <span class="form-label">Name</span>
  <input class="form-input" bind:value={name} />
</div>
```

### 6.3 Checkbox / Toggle Input Pattern

For boolean fields, use a horizontal layout with the label beside the checkbox:

```svelte
<!-- DO THIS: Checkbox in a form-field -->
<div class="form-field form-field--inline">
  <label class="field-label" for="is-template">
    <input id="is-template" type="checkbox" bind:checked={isTemplate} />
    Mark as template
  </label>
</div>
```

```css
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
```

### 6.4 Current Naming Variants

The same TitledDataInput pattern exists under multiple class names across the codebase:

| Page | Label Class | Input Class | Container Class |
|------|------------|-------------|-----------------|
| `summaries` | `.field-label` | `.field-input` | `.form-field` |
| `notes` | `.form-group label` | `.form-group input` | `.form-group` |
| `profile` | `.form-field label` | `.form-field input` | `.form-field` |
| `domains` | `.form-label` | `.form-input` | `.form-row` |
| `BulletDetailModal` | `.field-label` | `.field-textarea` | `.field-group` |

The canonical names are `.form-field` (container), `.field-label` (label), `.field-input` (input), and `.field-hint` (help text). All pages should converge on these.

---

## 7. Forms Composition

**Layer:** Pattern (not a component)
**Purpose:** Documents how TitledDataInput elements compose into complete edit forms. This is a CSS + markup pattern, not a Svelte component.

### 7.1 Form Structure

```svelte
<form class="entity-form" onsubmit={(e) => { e.preventDefault(); handleSave() }}>
  <!-- Vertical stack of form fields -->
  <div class="form-group">
    <div class="form-field">
      <label class="field-label" for="title">Title <span class="field-required">*</span></label>
      <input id="title" type="text" class="field-input" bind:value={title} required />
    </div>

    <div class="form-field">
      <label class="field-label" for="desc">Description</label>
      <textarea id="desc" class="field-input" bind:value={description} rows="4" />
    </div>

    <div class="form-field">
      <label class="field-label" for="status">Status</label>
      <select id="status" class="field-select" bind:value={status}>
        <option value="draft">Draft</option>
        <option value="active">Active</option>
      </select>
    </div>
  </div>

  <!-- Form actions always at bottom, separated by border -->
  <div class="form-actions">
    <button class="btn btn-primary" type="submit" disabled={saving}>
      {saving ? 'Saving...' : 'Save'}
    </button>
    <button class="btn btn-ghost" type="button" onclick={cancel}>Cancel</button>
  </div>
</form>
```

### 7.2 CSS Specification

```css
/* Form container -- scoped per page or in base.css */
.entity-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  max-width: 640px;
}

/* .form-group and .form-actions are already in base.css */
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
```

### 7.3 Grid Layout Variant

For forms with many short fields (profile page), use a CSS grid:

```css
.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-4);
}

.form-grid .full-width {
  grid-column: 1 / -1;
}
```

```svelte
<!-- Grid form for compact field sets -->
<div class="form-grid">
  <div class="form-field full-width">
    <label class="field-label" for="name">Name</label>
    <input id="name" type="text" class="field-input" bind:value={name} />
  </div>
  <div class="form-field">
    <label class="field-label" for="email">Email</label>
    <input id="email" type="email" class="field-input" bind:value={email} />
  </div>
  <div class="form-field">
    <label class="field-label" for="phone">Phone</label>
    <input id="phone" type="tel" class="field-input" bind:value={phone} />
  </div>
</div>
```

### 7.4 Form Section Headers

When a form has multiple logical sections (e.g., profile's contact info vs salary expectations), use a section heading with a top border:

```css
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

```svelte
<!-- DO THIS -->
<h3 class="form-section-heading">Salary Expectations</h3>
<div class="form-grid">
  <div class="form-field">
    <label class="field-label" for="sal-min">Minimum ($)</label>
    <input id="sal-min" type="number" class="field-input" bind:value={salaryMin} />
  </div>
  ...
</div>
```

### 7.5 Common Anti-Patterns

```svelte
<!-- DON'T DO THIS: Button classes redefined in component styles -->
<!-- Use the global .btn classes from base.css. -->
<style>
  .btn-save { background: var(--color-primary); color: var(--text-inverse); }
  .btn-save:hover:not(:disabled) { background: var(--color-primary-hover); }
</style>

<!-- DON'T DO THIS: Inconsistent form-actions placement -->
<!-- Actions go at the bottom of the form, not between fields. -->
<div class="form-field">
  <label>Title</label>
  <input bind:value={title} />
</div>
<div class="form-actions">
  <button class="btn btn-primary">Save</button>
</div>
<div class="form-field">
  <label>Description</label>
  <textarea bind:value={desc} />
</div>

<!-- DON'T DO THIS: Hardcoded border-radius on form containers -->
<div style="border-radius: 8px; padding: 1.5rem;">
  <!-- Use var(--radius-lg) and var(--space-6) -->
</div>
```

---

## 8. Detail Component Pattern (ADR-006)

The dual-context detail component pattern (inline in SplitPanel vs. modal overlay) is fully specified in **Doc 5 (Interactive Systems) Section 7**. See that section for:

- Architecture diagram and composition rule
- Implementation patterns for inline and modal contexts
- Component requirements for dual-context rendering (no hardcoded width, optional `onClose`, flex layout)
- The `isModal` prop variant (ChainViewModal pattern)
- Anti-patterns and migration guidance

This section exists here as a cross-reference to avoid duplicating the full specification. The detail panel CSS below is the canonical style guide for content patterns within detail components:

### 8.1 Detail Panel CSS

```css
/* Detail panel - scoped in the detail component */
.detail-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  /* No width, no max-width. The container decides. */
}

.detail-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--color-border);
  gap: var(--space-3);
  flex-shrink: 0;
}

.detail-panel__title {
  font-size: var(--text-lg);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
  flex: 1;
}

.detail-panel__close {
  flex-shrink: 0;
}

.detail-panel__body {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-5);
}

.detail-panel__body .form-field {
  margin-bottom: var(--space-4);
}

.detail-panel__footer {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-5);
  border-top: 1px solid var(--color-border);
  flex-shrink: 0;
}
```

---

## 9. Component Summary Matrix

Quick reference for when to use each content pattern.

| Pattern | Has Own Background | Has Border | Has Padding | Typical Context |
|---------|-------------------|-----------|-------------|-----------------|
| **Entry** | No | No (transparent left border) | Minimal (space-2 space-3) | Dense lists, table rows |
| **PaddedEntry** | Yes (surface) | Yes (border) | Yes (space-4) | Card lists, summaries, notes |
| **SectionedList** | No (wraps entries) | Section header border-bottom | No | Categorized groups |
| **TagsList** | Per-pill only | No | No | Inline label sets |
| **DataInput** | Yes (surface) | Yes (border-strong) | Yes (internal) | Any text entry |
| **TitledDataInput** | Inherits from DataInput | Inherits | Inherits | Labeled form fields |
| **Detail Panel** | No (fills container) | Header/footer borders | Yes (space-5 body) | SplitPanel detail, Modal body |

---

## 10. Accessibility Notes

### 10.1 Entry and PaddedEntry

- Interactive entries (`onclick` provided) MUST have `role="button"` and `tabindex="0"`.
- Keyboard: Enter and Space activate the entry.
- Selected entries SHOULD have `aria-selected="true"` when inside a listbox context.

### 10.2 DataInput

- Every `<input>` and `<textarea>` MUST have an associated `<label>` element via `for`/`id` pairing, or be wrapped in a `<label>`.
- Required fields MUST have the `required` attribute on the input and a visible indicator (`.field-required` asterisk).
- Disabled inputs use `:disabled` pseudo-class (native semantics), not CSS-only opacity tricks.

### 10.3 TagsList

- When `onRemove` is present, each remove button MUST have `aria-label="Remove {tagName}"`.
- The tag list container SHOULD have `role="list"` and each tag `role="listitem"` for screen reader enumeration.

### 10.4 Detail Panel (Modal Context)

- The modal wrapper MUST have `role="dialog"` and `aria-modal="true"`.
- Focus MUST be trapped inside the modal when open.
- Escape key MUST dismiss the modal (handled by the parent wrapper, not the detail component).
- The close button MUST have `aria-label="Close"`.

---

## 11. EmptyState and EmptyPanel

### 11.1 EmptyState

**File:** `$lib/components/EmptyState.svelte`
**Layer:** Component
**Purpose:** Centered message with an optional call-to-action button. Used in main content areas when no data exists for a given view or filter.

**When to use:**
- A list/board/chart has no items to display
- A search/filter returns zero results
- A new user has not yet created any entities of a given type

**API:**

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `title` | `string` | -- | Short heading (e.g., "No bullets yet") |
| `description` | `string` | `undefined` | Longer explanatory text |
| `children` | `Snippet` | `undefined` | Optional CTA button or link |

**CSS:**

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
```

### 11.2 EmptyPanel

**File:** `$lib/components/EmptyPanel.svelte`
**Layer:** Component
**Purpose:** Specialized empty state for the detail pane of a SplitPanel. Lighter styling, no CTA. Displays a message like "Select an item to view details."

**When to use:**
- SplitPanel detail pane when no item is selected
- Any secondary panel that requires an item selection to show content

**API:**

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `message` | `string` | `'Select an item to view details'` | The placeholder message |

**CSS:**

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

### 11.3 When to Use Which

| Scenario | Use |
|----------|-----|
| No data exists for the view | `EmptyState` with title + description + optional CTA |
| Filter/search returns zero results | `EmptyState` with "No results" message (no CTA) |
| SplitPanel detail pane, nothing selected | `EmptyPanel` |
| KanbanColumn with zero cards | Inline message (handled by GenericKanbanColumn) |

---

## 12. Cross-References

| Doc | Relevance |
|-----|-----------|
| Doc 1 (Foundation) | Token definitions, CSS architecture, ADRs |
| Doc 2 (Layout & Containers) | PageWrapper, ContentArea that host these patterns |
| Doc 3 (Navigation & Headers) | PageHeader, ListPanelHeader that sit above Entry lists |
| Doc 5 (Interactive Systems) | KanbanBoard and modals that compose Entry/PaddedEntry; ADR-006 detail panel pattern |
| Doc 6 (Data Visualization) | MetricContainer cards that follow PaddedEntry visual language |

---

## Appendix A: Token Quick Reference for Content Patterns

The tokens most frequently used by the components in this document:

```css
/* Entry/PaddedEntry */
var(--color-surface)           /* PaddedEntry background */
var(--color-surface-raised)    /* Entry hover background */
var(--color-primary-subtle)    /* Selected background */
var(--color-primary)           /* Selected left-border accent */
var(--color-border)            /* PaddedEntry border */
var(--color-border-strong)     /* Input borders */
var(--radius-lg)               /* PaddedEntry border-radius */
var(--shadow-sm)               /* PaddedEntry hover shadow */

/* Tags */
var(--color-tag-bg)            /* Accent pill background */
var(--color-tag-text)          /* Accent pill text */
var(--color-tag-neutral-bg)    /* Neutral pill background */
var(--color-tag-neutral-text)  /* Neutral pill text */

/* Form fields */
var(--color-border-focus)      /* Input focus border */
var(--color-primary-subtle)    /* Input focus ring */
var(--text-sm)                 /* Input and label font size */
var(--font-medium)             /* Label font weight */
var(--text-secondary)          /* Label color */
var(--text-faint)              /* Placeholder and hint color */
var(--color-danger)            /* Required asterisk */
var(--radius-md)               /* Input border-radius */

/* Section headers */
var(--text-sm)                 /* Section title size */
var(--font-semibold)           /* Section title weight */
var(--text-secondary)          /* Section title color */

/* Detail panel */
var(--space-4)                 /* Header/footer padding */
var(--space-5)                 /* Body padding */
var(--text-lg)                 /* Detail panel title size */
```

---

## Acceptance Criteria

1. Entry selected state shows primary-colored left border (`--color-primary`) and subtle background (`--color-primary-subtle`).
2. PaddedEntry has no layout shift on selection -- the `padding-left` adjustment compensates exactly for the wider border.
3. PaddedEntry disabled state dims to 50% opacity and disables pointer events.
4. All forms use TitledDataInput pattern (`.form-field` + `.field-label` + `.field-input`) -- no custom label/input styling.
5. SectionedList renders section headers with uppercase styling and count badges.
6. TagsList remove buttons have `aria-label="Remove {tagName}"` for accessibility.
7. Detail components accept `onClose` as optional (present in modal context, absent in inline context).
8. EmptyPanel renders centered italic text for "nothing selected" states in SplitPanel detail panes.
9. EmptyState renders centered title + description + optional CTA for empty data views.
10. No `onsubmit|preventDefault` Svelte 4 syntax -- all forms use `onsubmit={(e) => { e.preventDefault(); ... }}`.
