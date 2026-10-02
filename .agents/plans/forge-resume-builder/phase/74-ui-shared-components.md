# Phase 74: UI Shared Components — Extraction & Progressive Adoption

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract 7 shared Svelte components + 1 global CSS file from 10+ pages of duplicated code, then progressively migrate each page with adoption enforcement tests.

**Architecture:** Create components first with tests, migrate one reference page each to prove they work, then migrate remaining pages one at a time. CI grep tests catch regressions. .claude/rules file prevents AI agents from reintroducing inline patterns.

**Tech Stack:** Svelte 5 (runes mode), CSS design tokens, bun:test

**Non-Goals:** Changing behavior or functionality of any page. Adding new features to pages during migration. Creating a Svelte `<Button>` component (we use CSS classes). Responsive/mobile layouts.

**Depends on:** Nothing -- this is a standalone refactoring phase.
**Blocks:** Nothing -- all changes are backward-compatible.

**Internal task parallelization:** Phase A tasks 74.1 and 74.2 must land first (rules + test scaffold). Tasks 74.3-74.6 are parallelizable (different components, though all touch notes page so coordinate commits). Task 74.7 depends on 74.3-74.6. Phase B tasks 74.8-74.11 are parallelizable. Task 74.12 depends on 74.8-74.11. Task 74.13 depends on everything.

**Reference specs:** All 8 specs are in `refs/specs/2026-04-04-*.md`

**Key files:**
- Components: `packages/webui/src/lib/components/` (new files go here)
- Component barrel: `packages/webui/src/lib/components/index.ts`
- Global CSS: `packages/webui/src/lib/styles/base.css`
- Design tokens: `packages/webui/src/lib/styles/tokens.css`
- Agent rules: `.claude/rules/ui-shared-components.md` (new)
- Adoption test: `packages/webui/src/__tests__/component-adoption.test.ts` (new)

---

## Current Violation Counts (baseline)

These counts were measured at plan creation time. The adoption enforcement test uses these as `expectedViolations` so it passes today and fails if new violations are added.

| Anti-pattern | Regex | Files | Allowed In |
|---|---|---|---|
| PageWrapper | `height:\s*calc\(100vh\s*-\s*4rem\)` | 9 | `PageWrapper.svelte` |
| SplitPanel | `\.list-panel\s*\{` | 8 | `SplitPanel.svelte`, `base.css` |
| ListPanelHeader | `\.btn-new\s*\{` | 7 | `ListPanelHeader.svelte` |
| Global Button CSS | `\.btn-primary\s*\{` | 22 | `base.css` |
| PageHeader | `\.page-title\s*\{` | 16 | `PageHeader.svelte`, `base.css` |
| TabBar | `\.tab-btn\s*\{` | 2 | `TabBar.svelte` |
| EmptyPanel | `\.editor-empty\s*\{\|\.empty-editor\s*\{` | 7 | `EmptyPanel.svelte` |
| ListSearchInput | `\.search-input\s*\{` | 11 | `ListSearchInput.svelte` |

---

## Phase A: Infrastructure + Structural Components (fixes bugs)

### Task 74.1: Create agent rules file

**Files to create:**
- `.claude/rules/ui-shared-components.md`

**Goal:** Prevent AI agents from reintroducing inline patterns when creating or modifying Svelte files or CSS files in the webui package.

#### Steps

- [ ] Create `.claude/rules/ui-shared-components.md` with the following content:

```markdown
---
globs:
  - "packages/webui/src/**/*.svelte"
  - "packages/webui/src/**/*.css"
---

# UI Shared Component Rules

These rules apply to all Svelte and CSS files in the webui package.

## Forbidden Patterns

1. **No inline viewport-escape CSS.** Full-viewport pages MUST use `<PageWrapper>`, not inline `height: calc(100vh - 4rem); margin: -2rem`.

2. **No inline split-panel CSS.** Two-column list+detail layouts MUST use `<SplitPanel>`, not inline `.list-panel` / `.editor-panel` CSS.

3. **No inline list-header CSS.** Split-panel list headers MUST use `<ListPanelHeader>`, not inline `.list-header` + `.btn-new` markup and CSS.

4. **No page-scoped button CSS.** Button styling MUST use global `.btn` + `.btn-primary`/`.btn-danger`/`.btn-ghost` classes from `base.css`, not page-scoped button CSS. "Create new" action buttons MUST use `--color-primary`, not `--color-info`.

5. **No inline page-title CSS.** Page titles MUST use `<PageHeader>`, not inline `.page-title` CSS.

6. **No inline tab-btn CSS.** Tab navigation MUST use `<TabBar>`, not hand-rolled `.tab-btn` or `.filter-tab` CSS.

7. **No inline editor-empty CSS.** "No selection" panel states MUST use `<EmptyPanel>`, not inline `.editor-empty` CSS or `<EmptyState>`.

8. **No inline search-input CSS.** List search/filter inputs MUST use `<ListSearchInput>`, not inline `.search-input` CSS.

## Component Import Pattern

All shared components are exported from `$lib/components/index.ts`:

```svelte
<script lang="ts">
  import { PageWrapper, SplitPanel, ListPanelHeader, PageHeader, TabBar, EmptyPanel, ListSearchInput } from '$lib/components'
</script>
```

## When Adding New Pages

New pages that use any of the patterns above MUST use the shared component. Never copy CSS from an existing page -- import the component instead.
```

- [ ] Commit: `chore(webui): add agent rules for UI shared components`

---

### Task 74.2: Create adoption enforcement test scaffold

**Files to create:**
- `packages/webui/src/__tests__/component-adoption.test.ts`

**Goal:** Create the test file that tracks anti-pattern violations across the codebase. Each pattern has an `expectedViolations` count set to the current baseline. The test fails if new violations are added (count goes up) but also fails if violations are fixed without updating the count (forcing the count to ratchet down).

#### Steps

- [ ] Create directory: `mkdir -p packages/webui/src/__tests__`

- [ ] Create `packages/webui/src/__tests__/component-adoption.test.ts` with the following content:

```typescript
/**
 * Component Adoption Enforcement Tests
 *
 * These tests scan the codebase for anti-patterns that should be replaced
 * by shared components. The expectedViolations count starts at the current
 * baseline and must decrease as pages are migrated.
 *
 * If you migrate a page, update the expectedViolations count downward.
 * If you add a new violation, the test will fail -- use the shared component instead.
 */
import { describe, test, expect } from 'bun:test'
import { readFileSync, readdirSync, statSync } from 'fs'
import { join, relative } from 'path'

const WEBUI_SRC = join(import.meta.dir, '..')
const ROUTES_DIR = join(WEBUI_SRC, 'routes')
const LIB_DIR = join(WEBUI_SRC, 'lib')

interface AntiPattern {
  name: string
  pattern: RegExp
  allowedIn: string[]  // filenames where the pattern is allowed (e.g. the component itself)
  message: string
  expectedViolations: number  // current count -- must decrease over time
}

const ANTI_PATTERNS: AntiPattern[] = [
  {
    name: 'PageWrapper',
    pattern: /height:\s*calc\(100vh\s*-\s*4rem\)/,
    allowedIn: ['PageWrapper.svelte'],
    message: 'Use <PageWrapper> instead of inline viewport-escape CSS (height: calc(100vh - 4rem))',
    expectedViolations: 9,
  },
  {
    name: 'SplitPanel',
    pattern: /\.list-panel\s*\{/,
    allowedIn: ['SplitPanel.svelte', 'base.css'],
    message: 'Use <SplitPanel> instead of inline .list-panel CSS',
    expectedViolations: 8,
  },
  {
    name: 'ListPanelHeader',
    pattern: /\.btn-new\s*\{/,
    allowedIn: ['ListPanelHeader.svelte'],
    message: 'Use <ListPanelHeader> instead of inline .btn-new CSS',
    expectedViolations: 7,
  },
  {
    name: 'GlobalButtonCSS',
    pattern: /\.btn-primary\s*\{/,
    allowedIn: ['base.css'],
    message: 'Use global .btn-primary class from base.css instead of page-scoped button CSS',
    expectedViolations: 22,
  },
  {
    name: 'PageHeader',
    pattern: /\.page-title\s*\{/,
    allowedIn: ['PageHeader.svelte', 'base.css'],
    message: 'Use <PageHeader> instead of page-scoped .page-title styles',
    expectedViolations: 16,
  },
  {
    name: 'TabBar',
    pattern: /\.tab-btn\s*\{/,
    allowedIn: ['TabBar.svelte'],
    message: 'Use <TabBar> instead of page-scoped .tab-btn styles',
    expectedViolations: 2,
  },
  {
    name: 'EmptyPanel',
    pattern: /\.editor-empty\s*\{|\.empty-editor\s*\{/,
    allowedIn: ['EmptyPanel.svelte'],
    message: 'Use <EmptyPanel> instead of page-scoped .editor-empty / .empty-editor styles',
    expectedViolations: 7,
  },
  {
    name: 'ListSearchInput',
    pattern: /\.search-input\s*\{/,
    allowedIn: ['ListSearchInput.svelte'],
    message: 'Use <ListSearchInput> instead of page-scoped .search-input styles',
    expectedViolations: 11,
  },
]

function collectFiles(dir: string, extensions: string[]): string[] {
  const results: string[] = []

  function walk(d: string) {
    let entries: string[]
    try {
      entries = readdirSync(d)
    } catch {
      return
    }

    for (const entry of entries) {
      const fullPath = join(d, entry)
      let stat
      try {
        stat = statSync(fullPath)
      } catch {
        continue
      }

      if (stat.isDirectory()) {
        // Skip node_modules, .svelte-kit, build directories
        if (entry === 'node_modules' || entry === '.svelte-kit' || entry === 'build') continue
        walk(fullPath)
      } else if (extensions.some(ext => entry.endsWith(ext))) {
        results.push(fullPath)
      }
    }
  }

  walk(dir)
  return results
}

function isAllowed(filePath: string, allowedIn: string[]): boolean {
  const fileName = filePath.split('/').pop() ?? ''
  return allowedIn.some(allowed => fileName === allowed)
}

describe('Component Adoption Enforcement', () => {
  const files = collectFiles(WEBUI_SRC, ['.svelte', '.css'])

  for (const ap of ANTI_PATTERNS) {
    test(`${ap.name}: no new violations (expected ${ap.expectedViolations})`, () => {
      const violations: string[] = []

      for (const filePath of files) {
        if (isAllowed(filePath, ap.allowedIn)) continue

        let content: string
        try {
          content = readFileSync(filePath, 'utf-8')
        } catch {
          continue
        }

        if (ap.pattern.test(content)) {
          violations.push(relative(WEBUI_SRC, filePath))
        }
      }

      if (violations.length > ap.expectedViolations) {
        expect.unreachable(
          `${ap.name}: Found ${violations.length} violations (expected at most ${ap.expectedViolations}). ` +
          `New violation(s):\n  ${violations.join('\n  ')}\n\n` +
          `${ap.message}`
        )
      }

      if (violations.length < ap.expectedViolations) {
        expect.unreachable(
          `${ap.name}: Found ${violations.length} violations but expected ${ap.expectedViolations}. ` +
          `Great -- you migrated pages! Update expectedViolations to ${violations.length} in component-adoption.test.ts.\n` +
          `Remaining violations:\n  ${violations.join('\n  ')}`
        )
      }

      // Exact match -- passes
      expect(violations.length).toBe(ap.expectedViolations)
    })
  }
})
```

- [ ] Run the test to verify it passes with current violation counts: `cd packages/webui && bun test src/__tests__/component-adoption.test.ts`

- [ ] Commit: `test(webui): add component adoption enforcement tests`

---

### Task 74.3: Create PageWrapper component + migrate notes page

**Files to create:**
- `packages/webui/src/lib/components/PageWrapper.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)
- `packages/webui/src/routes/data/notes/+page.svelte` (reference migration)
- `packages/webui/src/__tests__/component-adoption.test.ts` (update count)

**Goal:** Create the PageWrapper component that encapsulates the viewport-escape CSS pattern, then migrate the notes page as the reference implementation.

**Spec reference:** `refs/specs/2026-04-04-page-wrapper-component.md`

#### Steps

- [ ] Create `packages/webui/src/lib/components/PageWrapper.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    overflow?: 'auto' | 'hidden' | 'visible'
    children: Snippet
  }

  let { overflow = 'hidden', children }: Props = $props()
</script>

<div class="page-wrapper page-wrapper--overflow-{overflow}">
  {@render children()}
</div>

<style>
  .page-wrapper {
    height: calc(100vh - var(--space-8) * 2);
    margin: calc(-1 * var(--space-8));
    display: flex;
    flex-direction: column;
  }

  .page-wrapper--overflow-auto    { overflow: auto; }
  .page-wrapper--overflow-hidden  { overflow: hidden; }
  .page-wrapper--overflow-visible { overflow: visible; }
</style>
```

- [ ] Add export to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as PageWrapper } from './PageWrapper.svelte'
```

- [ ] Migrate `packages/webui/src/routes/data/notes/+page.svelte`:

**BEFORE** (lines 4, 172, 327-333):
```svelte
<!-- line 4: imports -->
  import { LoadingSpinner, EmptyState, ConfirmDialog } from '$lib/components'

<!-- line 172: markup -->
<div class="notes-page">

<!-- lines 327-333: style -->
  .notes-page {
    display: flex;
    gap: 0;
    height: calc(100vh - 4rem);
    margin: -2rem;
  }
```

**AFTER:**
```svelte
<!-- line 4: imports -->
  import { LoadingSpinner, EmptyState, ConfirmDialog, PageWrapper } from '$lib/components'

<!-- line 172: markup -->
<PageWrapper>
  <div class="notes-page">
    <!-- ... all existing content stays the same ... -->
  </div>
</PageWrapper>

<!-- style: remove viewport-escape, keep flex layout -->
  .notes-page {
    display: flex;
    gap: 0;
    flex: 1;
    min-height: 0;
  }
```

The key change: `.notes-page` loses `height: calc(100vh - 4rem)` and `margin: -2rem` (provided by PageWrapper) but gains `flex: 1; min-height: 0;` to fill the PageWrapper's height as a flex child.

- [ ] Update `packages/webui/src/__tests__/component-adoption.test.ts`: change PageWrapper `expectedViolations` from `9` to `8`.

- [ ] Visually verify: The notes page should look identical. The list panel and editor panel should fill the viewport height. Scrolling within each panel should work independently.

- [ ] Commit: `refactor(webui): create PageWrapper component, migrate notes page`

---

### Task 74.4: Create SplitPanel component + migrate notes page

**Files to create:**
- `packages/webui/src/lib/components/SplitPanel.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)
- `packages/webui/src/routes/data/notes/+page.svelte` (reference migration)
- `packages/webui/src/__tests__/component-adoption.test.ts` (update count)

**Goal:** Create the SplitPanel component that provides the two-column list+detail layout, then migrate the notes page.

**Spec reference:** `refs/specs/2026-04-04-split-panel-component.md`

#### Steps

- [ ] Create `packages/webui/src/lib/components/SplitPanel.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    listWidth?: number
    list: Snippet
    detail: Snippet
  }

  let { listWidth = 320, list, detail }: Props = $props()
</script>

<div class="split-panel">
  <div class="split-list" style:width="{listWidth}px">
    {@render list()}
  </div>
  <div class="split-detail">
    {@render detail()}
  </div>
</div>

<style>
  .split-panel {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .split-list {
    flex-shrink: 0;
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--color-surface);
  }

  .split-detail {
    flex: 1;
    overflow-y: auto;
    min-width: 0;
    background: var(--color-surface);
  }
</style>
```

- [ ] Add export to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as SplitPanel } from './SplitPanel.svelte'
```

- [ ] Migrate `packages/webui/src/routes/data/notes/+page.svelte`:

**BEFORE** (assuming PageWrapper already applied from 74.3):
```svelte
<script lang="ts">
  import { LoadingSpinner, EmptyState, ConfirmDialog, PageWrapper } from '$lib/components'
</script>

<PageWrapper>
  <div class="notes-page">
    <!-- Left panel -->
    <div class="list-panel">
      <!-- list content -->
    </div>

    <!-- Right panel -->
    <div class="editor-panel">
      <!-- editor content -->
    </div>
  </div>
</PageWrapper>

<style>
  .notes-page {
    display: flex;
    gap: 0;
    flex: 1;
    min-height: 0;
  }

  .list-panel {
    width: 320px;
    flex-shrink: 0;
    border-right: 1px solid var(--color-border);
    background: var(--color-surface);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* ... */

  .editor-panel {
    flex: 1;
    overflow-y: auto;
    background: var(--color-surface);
  }
</style>
```

**AFTER:**
```svelte
<script lang="ts">
  import { LoadingSpinner, EmptyState, ConfirmDialog, PageWrapper, SplitPanel } from '$lib/components'
</script>

<PageWrapper>
  <SplitPanel>
    {#snippet list()}
      <div class="list-header">
        <h2>Notes</h2>
        <button class="btn-new" onclick={startNew}>+ New</button>
      </div>
      <!-- ... rest of list content ... -->
    {/snippet}
    {#snippet detail()}
      {#if !selectedNote && !editing}
        <div class="editor-empty">
          <p>Select a note or create a new one.</p>
        </div>
      {:else}
        <div class="editor-content">
          <!-- ... rest of editor content ... -->
        </div>
      {/if}
    {/snippet}
  </SplitPanel>
</PageWrapper>
```

Remove from `<style>`: `.notes-page`, `.list-panel`, `.editor-panel` rules. The `.notes-page` wrapper div is no longer needed.

- [ ] Update `packages/webui/src/__tests__/component-adoption.test.ts`: change SplitPanel `expectedViolations` from `8` to `7`.

- [ ] Visually verify: The notes page should look identical. The left panel should be 320px wide with a right border. The right panel should scroll independently.

- [ ] Commit: `refactor(webui): create SplitPanel component, migrate notes page`

---

### Task 74.5: Create ListPanelHeader component + migrate notes page

**Files to create:**
- `packages/webui/src/lib/components/ListPanelHeader.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)
- `packages/webui/src/routes/data/notes/+page.svelte` (reference migration)
- `packages/webui/src/__tests__/component-adoption.test.ts` (update count)

**Goal:** Create the ListPanelHeader component with consistent `--color-primary` button color, then migrate the notes page.

**Spec reference:** `refs/specs/2026-04-04-list-panel-header-component.md`

#### Steps

- [ ] Create `packages/webui/src/lib/components/ListPanelHeader.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    title: string
    onNew?: () => void
    newLabel?: string
    actions?: Snippet
  }

  let { title, onNew, newLabel = '+ New', actions }: Props = $props()
</script>

<div class="list-panel-header">
  <h2 class="list-panel-header__title">{title}</h2>
  {#if actions || onNew}
    <div class="list-panel-header__actions">
      {#if actions}
        {@render actions()}
      {/if}
      {#if onNew}
        <button
          class="list-panel-header__new-btn"
          type="button"
          onclick={onNew}
        >
          {newLabel}
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .list-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-5) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    gap: var(--space-2);
  }

  .list-panel-header__title {
    font-size: var(--text-xl);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .list-panel-header__actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
  }

  .list-panel-header__new-btn {
    padding: 0.35rem var(--space-3);
    background: var(--color-primary);
    color: var(--text-inverse);
    border: none;
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    font-weight: var(--font-medium);
    cursor: pointer;
    transition: background 0.15s;
    white-space: nowrap;
  }

  .list-panel-header__new-btn:hover {
    background: var(--color-primary-hover);
  }

  .list-panel-header__new-btn:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
  }
</style>
```

- [ ] Add export to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as ListPanelHeader } from './ListPanelHeader.svelte'
```

- [ ] Migrate `packages/webui/src/routes/data/notes/+page.svelte`:

**BEFORE** (inside the `list` snippet):
```svelte
<div class="list-header">
  <h2>Notes</h2>
  <button class="btn-new" onclick={startNew}>+ New</button>
</div>
```

**AFTER:**
```svelte
<ListPanelHeader title="Notes" onNew={startNew} />
```

Remove from `<style>`: `.list-header`, `.list-header h2`, `.btn-new`, `.btn-new:hover` rules.

Update import line to include `ListPanelHeader`.

- [ ] Update `packages/webui/src/__tests__/component-adoption.test.ts`: change ListPanelHeader `expectedViolations` from `7` to `6`.

- [ ] Visually verify: The list panel header should look identical. The "New" button should use `--color-primary` (purple, not blue). Title should be left-aligned, button right-aligned.

- [ ] Commit: `refactor(webui): create ListPanelHeader component, migrate notes page`

---

### Task 74.6: Update global button CSS in base.css + migrate summaries page

**Files to modify:**
- `packages/webui/src/lib/styles/base.css` (update existing button classes)
- `packages/webui/src/routes/resumes/summaries/+page.svelte` (reference migration)
- `packages/webui/src/__tests__/component-adoption.test.ts` (update count)

**Goal:** Ensure global button CSS in base.css has `focus-visible` styles and the `text-decoration: none` rule from the spec, then migrate the summaries page by removing its page-scoped button CSS.

**Spec reference:** `refs/specs/2026-04-04-global-button-css.md`

**Note:** `base.css` already has `.btn`, `.btn-primary`, `.btn-danger`, `.btn-ghost`, `.btn-sm`, `.btn-lg`, and several other button classes. The spec calls for adding `:focus-visible` and `text-decoration: none` which are currently missing.

#### Steps

- [ ] Update `packages/webui/src/lib/styles/base.css` -- add these rules to the existing `.btn` block (after `.btn:disabled`):

```css
.btn:focus-visible {
  outline: 2px solid var(--color-border-focus);
  outline-offset: 2px;
}
```

Also add `text-decoration: none;` to the `.btn` base class (for `<a>` tag usage).

- [ ] Migrate `packages/webui/src/routes/resumes/summaries/+page.svelte`:

Identify all page-scoped button CSS rules (`.btn`, `.btn-primary`, `.btn-ghost`, `.btn-danger-ghost`, `.btn-sm`, `.btn-xs`, and any `:hover`/`:disabled` variants). These should ALL be removed from the page's `<style>` block since `base.css` provides them globally.

Also rename any non-standard button class names in the markup:
- `.btn-save` -> `btn btn-primary`
- `.btn-delete` -> `btn btn-danger-ghost`

The page may still define `.btn-xs` locally if the global doesn't provide it -- check `base.css` (it does provide `.btn-xs`).

- [ ] Update `packages/webui/src/__tests__/component-adoption.test.ts`: change GlobalButtonCSS `expectedViolations` from `22` to `21`.

- [ ] Visually verify: All buttons on the summaries page should look identical. Check: primary (purple), ghost (transparent), danger (red), small size, disabled state, hover effects, focus ring on Tab.

- [ ] Commit: `refactor(webui): add focus-visible to global buttons, migrate summaries page`

---

### Task 74.7: Migrate remaining structural pages

**Files to modify (one commit per page):**
- `packages/webui/src/routes/data/contacts/+page.svelte`
- `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte`
- `packages/webui/src/routes/data/organizations/+page.svelte`
- `packages/webui/src/routes/data/skills/+page.svelte`
- `packages/webui/src/routes/data/bullets/+page.svelte`
- `packages/webui/src/routes/resumes/+page.svelte`
- `packages/webui/src/routes/data/sources/SourcesView.svelte`
- `packages/webui/src/routes/data/sources/SkillsView.svelte`
- `packages/webui/src/lib/components/SourcesView.svelte`
- `packages/webui/src/lib/components/kanban/KanbanBoard.svelte` (PageWrapper only)

**Files to modify (adoption test):**
- `packages/webui/src/__tests__/component-adoption.test.ts` (update counts after each page)

**Goal:** Migrate all remaining pages that use inline viewport-escape CSS, split-panel CSS, and list-header CSS to the shared components.

#### Steps

For EACH page below, apply the following migration pattern. Do one commit per page.

**Migration pattern for each page:**

1. Add imports: `PageWrapper`, `SplitPanel`, `ListPanelHeader` (as applicable)
2. Wrap the outer page div in `<PageWrapper>` -- remove `height: calc(100vh - 4rem)` and `margin: -2rem` from page CSS
3. Replace `.list-panel` + `.editor-panel` divs with `<SplitPanel>` using `{#snippet list()}` and `{#snippet detail()}` -- remove `.list-panel`, `.editor-panel`, and split-panel CSS
4. Replace `.list-header` + `.btn-new` divs with `<ListPanelHeader title="..." onNew={...} newLabel="...">` -- remove `.list-header`, `.btn-new` CSS
5. Remove all page-scoped CSS rules that are now provided by the components
6. Update adoption test `expectedViolations` counts
7. Visually verify
8. Commit with message: `refactor(webui): migrate [page-name] to shared structural components`

**Page-specific notes:**

| Page | PageWrapper | SplitPanel | ListPanelHeader | Notes |
|------|-------------|------------|-----------------|-------|
| contacts | Yes | Yes | Yes (fixes `--color-info` -> `--color-primary` on btn-new) | Has `.loading-container` that wraps split panel -- put inside PageWrapper |
| job-descriptions | Yes | Yes (list mode only) | Yes | Has list mode / detail mode toggle -- SplitPanel is conditional |
| organizations | Yes | Yes | Yes | Standard split-panel |
| skills | Yes | No | No | Wrapper page -- no split panel |
| bullets | Yes | No | No | Wrapper page -- no split panel |
| resumes | Yes | Yes (list mode only) | No | Has list mode and detail mode |
| SourcesView (routes) | No | Yes | Yes | Sub-view inside sources tab page |
| SkillsView (routes) | No | Yes | Yes | Sub-view inside sources tab page |
| SourcesView (lib) | No | Yes | Yes | Component version |
| KanbanBoard | Yes | No | No | PageWrapper only (has `calc(100vh - 4rem)`) |

**Expected final violation counts after all Task 74.7 pages are migrated:**

| Pattern | Expected |
|---------|----------|
| PageWrapper | 1 (the `_old_page.svelte.bak` file) |
| SplitPanel | 1 (`base.css` -- allowed) |
| ListPanelHeader | 0 |

Update `expectedViolations` in `component-adoption.test.ts` after each page migration. The test should pass at every step.

- [ ] Commit per page (see commit message pattern above)

---

## Phase B: Polish Components + Remaining Migrations

### Task 74.8: Create PageHeader component + migrate debug subpages

**Files to create:**
- `packages/webui/src/lib/components/PageHeader.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)
- `packages/webui/src/routes/config/debug/prompts/+page.svelte` (reference)
- `packages/webui/src/routes/config/debug/api/+page.svelte`
- `packages/webui/src/routes/config/debug/events/+page.svelte`
- `packages/webui/src/routes/config/debug/ui/+page.svelte`
- `packages/webui/src/routes/config/debug/+page.svelte`
- `packages/webui/src/__tests__/component-adoption.test.ts` (update count)

**Goal:** Create the PageHeader component and migrate the 5 debug subpages (simplest usage -- title + subtitle, no actions).

**Spec reference:** `refs/specs/2026-04-04-page-header-component.md`

#### Steps

- [ ] Create `packages/webui/src/lib/components/PageHeader.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    title: string
    subtitle?: string
    actions?: Snippet
  }

  let { title, subtitle, actions }: Props = $props()
</script>

<header class="page-header">
  <div class="page-header-text">
    <h1>{title}</h1>
    {#if subtitle}
      <p class="page-header-subtitle">{subtitle}</p>
    {/if}
  </div>
  {#if actions}
    <div class="page-header-actions">
      {@render actions()}
    </div>
  {/if}
</header>

<style>
  .page-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }

  .page-header-text {
    flex: 1;
    min-width: 0;
  }

  .page-header h1 {
    font-size: var(--text-2xl);
    font-weight: var(--font-bold);
    color: var(--text-primary);
    line-height: var(--leading-tight);
    margin: 0;
  }

  .page-header-subtitle {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin-top: var(--space-1);
    line-height: var(--leading-normal);
  }

  .page-header-actions {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
</style>
```

- [ ] Add export to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as PageHeader } from './PageHeader.svelte'
```

- [ ] Migrate each debug subpage. For each page:

**BEFORE (typical debug subpage):**
```svelte
<h1 class="page-title">Prompt Logs</h1>
<p class="subtitle">AI derivation audit trail</p>

<style>
  .page-title {
    font-size: var(--text-2xl);
    font-weight: var(--font-bold);
    color: var(--text-primary);
    margin-bottom: var(--space-1);
  }

  .subtitle {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin-bottom: var(--space-6);
  }
</style>
```

**AFTER:**
```svelte
<script lang="ts">
  import { PageHeader } from '$lib/components'
</script>

<PageHeader title="Prompt Logs" subtitle="AI derivation audit trail" />
```

Remove `.page-title` and `.subtitle` from the page's `<style>` block.

- [ ] Update `packages/webui/src/__tests__/component-adoption.test.ts`: change PageHeader `expectedViolations` from `16` to `11` (removing 5 debug pages).

- [ ] Visually verify each debug subpage: title should be 24px bold, subtitle should be small muted text below.

- [ ] Commit: `refactor(webui): create PageHeader component, migrate debug subpages`

---

### Task 74.9: Create TabBar component + migrate domains page

**Files to create:**
- `packages/webui/src/lib/components/TabBar.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)
- `packages/webui/src/routes/data/domains/+page.svelte` (reference migration)
- `packages/webui/src/__tests__/component-adoption.test.ts` (update count)

**Goal:** Create the TabBar component with proper ARIA roles and keyboard navigation, then migrate the domains page.

**Spec reference:** `refs/specs/2026-04-04-tab-bar-component.md`

#### Steps

- [ ] Create `packages/webui/src/lib/components/TabBar.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte'

  export interface TabItem {
    value: string
    label: string
  }

  interface Props {
    tabs: TabItem[]
    active: string
    onchange: (value: string) => void
    tab?: Snippet<[{ tab: TabItem; active: boolean }]>
  }

  let { tabs, active, onchange, tab: tabSnippet }: Props = $props()

  let tabRefs: HTMLButtonElement[] = $state([])

  function handleClick(value: string) {
    if (value !== active) {
      onchange(value)
    }
  }

  function handleKeydown(e: KeyboardEvent, index: number) {
    let targetIndex = -1

    switch (e.key) {
      case 'ArrowRight':
        targetIndex = (index + 1) % tabs.length
        break
      case 'ArrowLeft':
        targetIndex = (index - 1 + tabs.length) % tabs.length
        break
      case 'Home':
        targetIndex = 0
        break
      case 'End':
        targetIndex = tabs.length - 1
        break
      default:
        return
    }

    e.preventDefault()
    tabRefs[targetIndex]?.focus()
  }
</script>

<nav class="tab-bar" role="tablist" aria-label="Tabs">
  {#each tabs as t, i (t.value)}
    <button
      bind:this={tabRefs[i]}
      class="tab-bar-btn"
      class:active={active === t.value}
      role="tab"
      aria-selected={active === t.value}
      tabindex={active === t.value ? 0 : -1}
      onclick={() => handleClick(t.value)}
      onkeydown={(e) => handleKeydown(e, i)}
    >
      {#if tabSnippet}
        {@render tabSnippet({ tab: t, active: active === t.value })}
      {:else}
        {t.label}
      {/if}
    </button>
  {/each}
</nav>

<style>
  .tab-bar {
    display: flex;
    border-bottom: 1px solid var(--color-border);
    margin-bottom: var(--space-6);
  }

  .tab-bar-btn {
    padding: var(--space-3) var(--space-4);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    font-size: var(--text-sm);
    font-weight: var(--font-medium);
    color: var(--text-muted);
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
    white-space: nowrap;
  }

  .tab-bar-btn:hover {
    color: var(--text-secondary);
  }

  .tab-bar-btn:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: -2px;
    border-radius: var(--radius-sm);
  }

  .tab-bar-btn.active {
    color: var(--color-primary);
    border-bottom-color: var(--color-primary);
  }
</style>
```

- [ ] Add export to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as TabBar } from './TabBar.svelte'
```

- [ ] Migrate `packages/webui/src/routes/data/domains/+page.svelte`:

**BEFORE (typical pattern):**
```svelte
<div class="tab-bar">
  {#each TABS as tab}
    <button
      class="tab-btn"
      class:active={activeTab === tab.value}
      onclick={() => switchTab(tab.value)}
    >
      {tab.label}
    </button>
  {/each}
</div>

<style>
  .tab-bar {
    display: flex;
    border-bottom: 1px solid var(--color-border);
    margin-bottom: var(--space-6);
  }
  .tab-btn {
    padding: 0.75rem 1.25rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    font-size: var(--text-sm);
    color: var(--text-muted);
    cursor: pointer;
  }
  .tab-btn.active {
    color: var(--color-primary);
    border-bottom-color: var(--color-primary);
  }
</style>
```

**AFTER:**
```svelte
<script lang="ts">
  import { TabBar } from '$lib/components'
  // ... existing imports ...

  const TABS = [
    { value: 'domains', label: 'Domains' },
    { value: 'archetypes', label: 'Archetypes' },
  ]
</script>

<TabBar tabs={TABS} active={activeTab} onchange={switchTab} />
```

Remove `.tab-bar`, `.tab-btn`, `.tab-btn:hover`, `.tab-btn.active` from `<style>`.

- [ ] Update `packages/webui/src/__tests__/component-adoption.test.ts`: change TabBar `expectedViolations` from `2` to `1`.

- [ ] Visually verify: Tab bar should look identical. Click tabs to switch views. Test keyboard: Tab to focus, arrow keys to move between tabs, Enter/Space to activate.

- [ ] Commit: `refactor(webui): create TabBar component, migrate domains page`

---

### Task 74.10: Create EmptyPanel component + migrate notes detail panel

**Files to create:**
- `packages/webui/src/lib/components/EmptyPanel.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)
- `packages/webui/src/routes/data/notes/+page.svelte` (reference migration)
- `packages/webui/src/__tests__/component-adoption.test.ts` (update count)

**Goal:** Create the lightweight EmptyPanel component for "no selection" states, then migrate the notes page.

**Spec reference:** `refs/specs/2026-04-04-empty-panel-component.md`

#### Steps

- [ ] Create `packages/webui/src/lib/components/EmptyPanel.svelte`:

```svelte
<script lang="ts">
  interface Props {
    message?: string
    actionLabel?: string
    onAction?: () => void
  }

  let {
    message = 'Select an item to view details.',
    actionLabel,
    onAction,
  }: Props = $props()
</script>

<div class="empty-panel">
  <p class="empty-panel-message">{message}</p>
  {#if actionLabel}
    <button class="empty-panel-action" type="button" onclick={onAction}>
      {actionLabel}
    </button>
  {/if}
</div>

<style>
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
</style>
```

- [ ] Add export to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as EmptyPanel } from './EmptyPanel.svelte'
```

- [ ] Migrate `packages/webui/src/routes/data/notes/+page.svelte`:

**BEFORE** (inside the `detail` snippet):
```svelte
{#if !selectedNote && !editing}
  <div class="editor-empty">
    <p>Select a note or create a new one.</p>
  </div>
```

**AFTER:**
```svelte
{#if !selectedNote && !editing}
  <EmptyPanel message="Select a note or create a new one." />
```

Remove from `<style>`: `.editor-empty` rule.

Update import line to include `EmptyPanel`.

- [ ] Update `packages/webui/src/__tests__/component-adoption.test.ts`: change EmptyPanel `expectedViolations` from `7` to `6`.

- [ ] Visually verify: When no note is selected, the right panel should show centered muted text "Select a note or create a new one." -- identical to before.

- [ ] Commit: `refactor(webui): create EmptyPanel component, migrate notes page`

---

### Task 74.11: Create ListSearchInput component + migrate notes search

**Files to create:**
- `packages/webui/src/lib/components/ListSearchInput.svelte`

**Files to modify:**
- `packages/webui/src/lib/components/index.ts` (add export)
- `packages/webui/src/routes/data/notes/+page.svelte` (reference migration)
- `packages/webui/src/__tests__/component-adoption.test.ts` (update count)

**Goal:** Create the ListSearchInput component with consistent focus ring styling, then migrate the notes page.

**Spec reference:** `refs/specs/2026-04-04-list-search-input-component.md`

#### Steps

- [ ] Create `packages/webui/src/lib/components/ListSearchInput.svelte`:

```svelte
<script lang="ts">
  interface Props {
    value?: string
    placeholder?: string
    [key: string]: unknown
  }

  let { value = $bindable(''), placeholder = 'Search...', ...rest }: Props = $props()
</script>

<input
  type="text"
  class="list-search-input"
  {placeholder}
  bind:value
  {...rest}
/>

<style>
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
</style>
```

- [ ] Add export to `packages/webui/src/lib/components/index.ts`:

```typescript
export { default as ListSearchInput } from './ListSearchInput.svelte'
```

- [ ] Migrate `packages/webui/src/routes/data/notes/+page.svelte`:

**BEFORE:**
```svelte
<div class="filter-bar">
  <input
    type="text"
    class="search-input"
    placeholder="Search notes..."
    bind:value={searchQuery}
  />
</div>
```

**AFTER:**
```svelte
<div class="filter-bar">
  <ListSearchInput bind:value={searchQuery} placeholder="Search notes..." />
</div>
```

Remove from `<style>`: `.search-input` and `.search-input:focus` rules.

Update import line to include `ListSearchInput`.

- [ ] Update `packages/webui/src/__tests__/component-adoption.test.ts`: change ListSearchInput `expectedViolations` from `11` to `10`.

- [ ] Visually verify: The search input should look identical. Type text to filter notes. The focus ring (purple glow) should appear when the input is focused.

- [ ] Commit: `refactor(webui): create ListSearchInput component, migrate notes search`

---

### Task 74.12: Migrate remaining polish pages

**Files to modify:** Multiple pages across routes and lib/components (one commit per page).

**Files to update:** `packages/webui/src/__tests__/component-adoption.test.ts` (update counts after each migration)

**Goal:** Migrate all remaining pages to use PageHeader, TabBar, EmptyPanel, ListSearchInput, and global button CSS.

#### Subtask 74.12a: Migrate remaining PageHeader pages

Migrate these pages to `<PageHeader>` (one commit each):

| Page | File | Notes |
|------|------|-------|
| Summaries | `routes/resumes/summaries/+page.svelte` | Has action button in header |
| Templates | `routes/resumes/templates/+page.svelte` | Has action button in header |
| Chain | `routes/chain/+page.svelte` | Title + subtitle |
| Dashboard | `routes/+page.svelte` | Title only |
| Resumes | `routes/resumes/+page.svelte` | May have action button |
| DomainsView | `routes/data/domains/DomainsView.svelte` | Sub-view title |
| ArchetypesView | `routes/data/domains/ArchetypesView.svelte` | Sub-view title |
| BulletsView | `routes/data/sources/BulletsView.svelte` | Sub-view title |

For pages with action buttons, use the `actions` snippet:

```svelte
<PageHeader title="Summaries" subtitle="Reusable professional summaries and templates">
  {#snippet actions()}
    <button class="btn btn-primary" onclick={startCreate}>+ New Summary</button>
  {/snippet}
</PageHeader>
```

Remove `.page-title`, `.subtitle`, `.page-header` CSS from each page.

**Expected count after all PageHeader migrations:** Update `expectedViolations` to `2` (only `base.css` and one `_old_page.svelte.bak` should remain).

- [ ] Commit per page: `refactor(webui): migrate [page] to <PageHeader>`

#### Subtask 74.12b: Migrate remaining TabBar pages

| Page | File | Notes |
|------|------|-------|
| SourcesView | `routes/data/sources/SourcesView.svelte` | Uses `.filter-tab` with counts -- use `tab` snippet |

```svelte
<TabBar tabs={SOURCE_TABS} active={activeTab} onchange={switchTab}>
  {#snippet tab({ tab: t, active: isActive })}
    {t.label}
    <span class="tab-count">{getCount(t.value)}</span>
  {/snippet}
</TabBar>
```

Remove `.filter-tab` CSS. The `.tab-count` badge styling can remain page-scoped (it is not part of the TabBar spec).

Also check `lib/components/SourcesView.svelte` (if it also has `.filter-tab`).

**Expected count after all TabBar migrations:** Update `expectedViolations` to `0` (only `_old_page.svelte.bak` files may remain -- exclude `.bak` files from the test or accept them).

- [ ] Commit per page: `refactor(webui): migrate [page] to <TabBar>`

#### Subtask 74.12c: Migrate remaining EmptyPanel pages

| Page | File | Notes |
|------|------|-------|
| Organizations | `routes/data/organizations/+page.svelte` | Raw `<p>` pattern |
| Contacts | `routes/data/contacts/+page.svelte` | May use `<EmptyState>` |
| Job Descriptions | `routes/opportunities/job-descriptions/+page.svelte` | May use `<EmptyState>` |
| SourcesView | `routes/data/sources/SourcesView.svelte` | Check for empty editor pattern |
| SkillsView | `routes/data/sources/SkillsView.svelte` | Check for empty editor pattern |
| SourcesView (lib) | `lib/components/SourcesView.svelte` | Check for empty editor pattern |

For pages using `<EmptyState>` in the detail panel's "no selection" state, replace with `<EmptyPanel>`. Keep `<EmptyState>` for actual empty-list scenarios (e.g. "no items found").

**Expected count after all EmptyPanel migrations:** Update `expectedViolations` to `0`.

- [ ] Commit per page: `refactor(webui): migrate [page] to <EmptyPanel>`

#### Subtask 74.12d: Migrate remaining ListSearchInput pages

| Page | File | Notes |
|------|------|-------|
| Organizations | `routes/data/organizations/+page.svelte` | Filter bar |
| Contacts | `routes/data/contacts/+page.svelte` | Filter bar |
| Job Descriptions | `routes/opportunities/job-descriptions/+page.svelte` | Filter bar |
| SkillsView | `routes/data/sources/SkillsView.svelte` | Search within skills tab |
| BulletsView | `routes/data/sources/BulletsView.svelte` | Search within bullets tab |
| ChainViewModal | `lib/components/ChainViewModal.svelte` | Modal search |
| JDPickerModal | `lib/components/resume/JDPickerModal.svelte` | Modal search |
| ResumePickerModal | `lib/components/jd/ResumePickerModal.svelte` | Modal search |
| OrgPickerModal | `lib/components/kanban/OrgPickerModal.svelte` | Modal search |
| GraphSearchBar | `lib/components/graph/GraphSearchBar.svelte` | Graph search |

For each page:
```svelte
<!-- Replace -->
<input type="text" class="search-input" placeholder="Search..." bind:value={query} />

<!-- With -->
<ListSearchInput bind:value={query} placeholder="Search..." />
```

Remove `.search-input` and `.search-input:focus` from page `<style>`.

**Expected count after all ListSearchInput migrations:** Update `expectedViolations` to `0`.

- [ ] Commit per page: `refactor(webui): migrate [page] to <ListSearchInput>`

#### Subtask 74.12e: Migrate remaining global button CSS pages

For each page with page-scoped `.btn-primary {` CSS, remove the page-scoped button rules and ensure the markup uses the global `.btn .btn-primary` (etc.) classes from `base.css`.

| Page | File |
|------|------|
| Notes | `routes/data/notes/+page.svelte` |
| Profile | `routes/config/profile/+page.svelte` |
| Resumes | `routes/resumes/+page.svelte` |
| Templates | `routes/resumes/templates/+page.svelte` |
| Export | `routes/config/export/+page.svelte` |
| BulletsView | `routes/data/sources/BulletsView.svelte` |
| ArchetypesView | `routes/data/domains/ArchetypesView.svelte` |
| DomainsView | `routes/data/domains/DomainsView.svelte` |
| JDEditor | `lib/components/jd/JDEditor.svelte` |
| ContactEditor | `lib/components/contacts/ContactEditor.svelte` |
| DerivePerspectivesDialog | `lib/components/DerivePerspectivesDialog.svelte` |
| OrgPickerModal | `lib/components/kanban/OrgPickerModal.svelte` |
| PdfView | `lib/components/resume/PdfView.svelte` |
| MarkdownView | `lib/components/resume/MarkdownView.svelte` |
| LatexView | `lib/components/resume/LatexView.svelte` |
| DragNDropView | `lib/components/resume/DragNDropView.svelte` |
| SummaryPicker | `lib/components/SummaryPicker.svelte` |
| HeaderEditor | `lib/components/resume/HeaderEditor.svelte` |

Key transformations for non-standard button class names:
- `.btn-save` -> `btn btn-primary`
- `.btn-delete` -> `btn btn-danger` or `btn btn-danger-ghost`
- `.btn-add` -> `btn btn-primary btn-sm` or `btn btn-ghost btn-sm`
- Any other `.btn-*` custom names -> map to the canonical variant

**Expected count after all button CSS migrations:** Update `expectedViolations` to `1` (only `base.css` remains -- which is the allowed source).

- [ ] Commit per page: `refactor(webui): migrate [page] to global button CSS`

---

### Task 74.13: Final adoption enforcement -- zero violations

**Files to modify:**
- `packages/webui/src/__tests__/component-adoption.test.ts`

**Goal:** Update all `expectedViolations` counts to their final values (0 for most, 1 for patterns where `base.css` is the only remaining "violation"), verify the test passes, and confirm all pages have been migrated.

#### Steps

- [ ] Update `expectedViolations` to final values:

```typescript
const ANTI_PATTERNS: AntiPattern[] = [
  {
    name: 'PageWrapper',
    pattern: /height:\s*calc\(100vh\s*-\s*4rem\)/,
    allowedIn: ['PageWrapper.svelte'],
    message: 'Use <PageWrapper> instead of inline viewport-escape CSS',
    expectedViolations: 0,  // was 9 -- all migrated (bak files excluded or accepted)
  },
  {
    name: 'SplitPanel',
    pattern: /\.list-panel\s*\{/,
    allowedIn: ['SplitPanel.svelte', 'base.css'],
    message: 'Use <SplitPanel> instead of inline .list-panel CSS',
    expectedViolations: 0,  // was 8 -- all migrated
  },
  {
    name: 'ListPanelHeader',
    pattern: /\.btn-new\s*\{/,
    allowedIn: ['ListPanelHeader.svelte'],
    message: 'Use <ListPanelHeader> instead of inline .btn-new CSS',
    expectedViolations: 0,  // was 7 -- all migrated
  },
  {
    name: 'GlobalButtonCSS',
    pattern: /\.btn-primary\s*\{/,
    allowedIn: ['base.css'],
    message: 'Use global .btn-primary from base.css',
    expectedViolations: 0,  // was 22 -- all migrated (base.css is allowed)
  },
  {
    name: 'PageHeader',
    pattern: /\.page-title\s*\{/,
    allowedIn: ['PageHeader.svelte', 'base.css'],
    message: 'Use <PageHeader> instead of page-scoped .page-title styles',
    expectedViolations: 0,  // was 16 -- all migrated (base.css is allowed)
  },
  {
    name: 'TabBar',
    pattern: /\.tab-btn\s*\{/,
    allowedIn: ['TabBar.svelte'],
    message: 'Use <TabBar> instead of page-scoped .tab-btn styles',
    expectedViolations: 0,  // was 2 -- all migrated
  },
  {
    name: 'EmptyPanel',
    pattern: /\.editor-empty\s*\{|\.empty-editor\s*\{/,
    allowedIn: ['EmptyPanel.svelte'],
    message: 'Use <EmptyPanel> instead of page-scoped .editor-empty styles',
    expectedViolations: 0,  // was 7 -- all migrated
  },
  {
    name: 'ListSearchInput',
    pattern: /\.search-input\s*\{/,
    allowedIn: ['ListSearchInput.svelte'],
    message: 'Use <ListSearchInput> instead of page-scoped .search-input styles',
    expectedViolations: 0,  // was 11 -- all migrated
  },
]
```

Note: The actual final counts depend on whether `.bak` files are excluded from scanning. If `.bak` files still trigger violations, either:
1. Add `.bak` to the file extension exclusion list in the test, OR
2. Set the expected count to match the remaining `.bak` file count

- [ ] Run the full test suite: `cd packages/webui && bun test src/__tests__/component-adoption.test.ts`

- [ ] Verify no regressions: `cd packages/webui && bun test` (run all webui tests)

- [ ] Verify the final component barrel export in `packages/webui/src/lib/components/index.ts` includes all 7 new exports:

```typescript
export { default as PageWrapper } from './PageWrapper.svelte'
export { default as SplitPanel } from './SplitPanel.svelte'
export { default as ListPanelHeader } from './ListPanelHeader.svelte'
export { default as PageHeader } from './PageHeader.svelte'
export { default as TabBar } from './TabBar.svelte'
export { default as EmptyPanel } from './EmptyPanel.svelte'
export { default as ListSearchInput } from './ListSearchInput.svelte'
```

- [ ] Commit: `refactor(webui): finalize component adoption -- all violations resolved`

---

## Summary

| Task | Component | Type | Reference Page | Files Changed |
|------|-----------|------|----------------|---------------|
| 74.1 | Rules file | Infrastructure | -- | 1 new |
| 74.2 | Adoption test | Infrastructure | -- | 1 new |
| 74.3 | PageWrapper | Structural | notes | 3 modified, 1 new |
| 74.4 | SplitPanel | Structural | notes | 3 modified, 1 new |
| 74.5 | ListPanelHeader | Structural | notes | 3 modified, 1 new |
| 74.6 | Global button CSS | Structural | summaries | 3 modified |
| 74.7 | Structural migrations | Migration | -- | ~10 pages |
| 74.8 | PageHeader | Polish | debug/prompts | 6 modified, 1 new |
| 74.9 | TabBar | Polish | domains | 3 modified, 1 new |
| 74.10 | EmptyPanel | Polish | notes | 3 modified, 1 new |
| 74.11 | ListSearchInput | Polish | notes | 3 modified, 1 new |
| 74.12 | Polish migrations | Migration | -- | ~30 pages |
| 74.13 | Final enforcement | Verification | -- | 1 modified |

**Total new files:** 8 (7 components + 1 rules file + 1 test file = 9)
**Total pages migrated:** ~40 file modifications across routes and lib/components
**CSS lines eliminated:** ~200+ lines of duplicated button CSS + ~100 lines of structural CSS per page
