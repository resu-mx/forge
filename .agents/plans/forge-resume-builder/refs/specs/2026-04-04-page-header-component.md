# PageHeader -- UI Component Spec

**Date:** 2026-04-04
**Status:** Draft
**Package:** `@forge/webui`
**Category:** structural
**Replaces:** 8+ per-page `.page-title` / `.subtitle` style blocks (4 debug subpages, dashboard, summaries, templates, chain, profile, domains, resumes)

---

## Overview

**What:** A shared page header component that renders a page title, optional subtitle, and optional action buttons in a consistent flex layout.

**Why:** The `.page-title { font-size: var(--text-2xl); ... }` pattern is copy-pasted across 8+ pages with minor variations in spacing, font-weight, and subtitle styling. This duplication creates visual inconsistency (some use `--font-bold`, some `--font-semibold`; subtitle spacing varies from `--space-1` to `--space-2`) and makes rebranding require touching every page.

**Pages affected:**
- `/config/debug/prompts/+page.svelte`
- `/config/debug/api/+page.svelte`
- `/config/debug/events/+page.svelte`
- `/config/debug/ui/+page.svelte`
- `/config/debug/+page.svelte`
- `/resumes/summaries/+page.svelte`
- `/resumes/templates/+page.svelte`
- `/chain/+page.svelte`
- `/+page.svelte` (dashboard)
- `/resumes/+page.svelte`
- `/data/domains/DomainsView.svelte`
- `/data/domains/ArchetypesView.svelte`
- `/data/sources/BulletsView.svelte`

---

## Component API

### Props

| Prop | Type | Default | Required | Description |
|------|------|---------|----------|-------------|
| `title` | `string` | -- | Yes | The page title rendered as an `<h1>` |
| `subtitle` | `string` | `undefined` | No | Optional subtitle rendered as a `<p>` below the title |

### Snippets (Slots)

| Snippet | Description | When to use |
|---------|-------------|-------------|
| `actions` | Right-aligned action area (buttons, toggles, etc.) | When the page header needs a primary CTA or toolbar buttons |

### Events / Callbacks

None. This is a purely presentational component.

### TypeScript Types

```typescript
// Export from $lib/components/PageHeader.svelte
interface PageHeaderProps {
  title: string;
  subtitle?: string;
  actions?: import('svelte').Snippet;
}
```

---

## Styling

### Concrete CSS (copy-paste)

```css
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

.page-header .page-header-subtitle {
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
```

### Token Consumption

| Token | Usage | Fallback |
|-------|-------|----------|
| `--text-2xl` | Page title font size (24px) | `1.5rem` |
| `--text-sm` | Subtitle font size (12.8px) | `0.8rem` |
| `--font-bold` | Title font weight | `700` |
| `--text-primary` | Title text color | `#1a1a2e` |
| `--text-muted` | Subtitle text color | `#6b7280` |
| `--space-1` | Gap between title and subtitle | `0.25rem` |
| `--space-4` | Gap between text block and actions | `1rem` |
| `--space-6` | Bottom margin of header | `1.5rem` |
| `--leading-tight` | Title line-height | `1.25` |
| `--leading-normal` | Subtitle line-height | `1.5` |

### Branding Strategy

All visual properties come from design tokens. Rebranding the page header means changing `--text-2xl`, `--text-primary`, `--text-muted`, and spacing tokens in `tokens.css`. No component CSS changes needed. The flex layout is structural and brand-independent.

### Rendered Markup

```html
<header class="page-header">
  <div class="page-header-text">
    <h1>Page Title Here</h1>
    <!-- conditional: only rendered when subtitle prop is provided -->
    <p class="page-header-subtitle">Subtitle text here</p>
  </div>
  <!-- conditional: only rendered when actions snippet is provided -->
  <div class="page-header-actions">
    <!-- actions snippet content -->
  </div>
</header>
```

---

## Behavior

### State Management

PageHeader is stateless. All content flows in via props and snippets. No internal state.

### Accessibility

- Uses semantic `<header>` element for landmark navigation
- Uses `<h1>` for the title, ensuring proper heading hierarchy
- No interactive elements within the component itself (actions are passed in)
- Screen readers can navigate to the page header via landmark shortcuts

### Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| No subtitle provided | Subtitle `<p>` is not rendered; title takes full height |
| No actions snippet provided | Actions `<div>` is not rendered; title block takes full width |
| Very long title | Title wraps naturally; `min-width: 0` on text block prevents overflow |
| Both subtitle and actions provided | Title/subtitle on left, actions right-aligned at flex-start |

---

## Examples

### Explicit Examples (DO THIS)

```svelte
<!-- Example 1: Basic usage -- minimal page header with just a title -->
<PageHeader title="Notes" />
```

```svelte
<!-- Example 2: With subtitle -- debug subpage pattern -->
<PageHeader title="Prompt Logs" subtitle="AI derivation audit trail" />
```

```svelte
<!-- Example 3: With action button -- list page pattern -->
<PageHeader title="Sources" subtitle="Manage your experience sources">
  {#snippet actions()}
    <button class="btn btn-primary" onclick={startNew}>+ New Source</button>
  {/snippet}
</PageHeader>
```

### Implicit Examples (THIS IS THE PATTERN)

```svelte
<!-- Typical page composition showing PageHeader in context -->
<script lang="ts">
  import { PageHeader } from '$lib/components'
</script>

<div class="debug-subpage">
  <PageHeader title="Prompt Logs" subtitle="AI derivation audit trail" />

  <EmptyState
    title="Coming soon"
    description="Prompt logs will appear here after AI derivation runs."
  />
</div>
```

### Anti-Examples (DON'T DO THIS)

```svelte
<!-- WRONG: Inline page title markup instead of using PageHeader -->
<h1 class="page-title">Prompt Logs</h1>
<p class="subtitle">AI derivation audit trail</p>
```

```css
/* WRONG: Page-scoped .page-title styles that duplicate the shared component */
.page-title {
  font-size: var(--text-2xl);
  font-weight: var(--font-bold);
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}
```

---

## Goals

- [x] Eliminate all per-page `.page-title` style blocks
- [x] Guarantee consistent title sizing (--text-2xl), weight (--font-bold), and subtitle styling across all pages
- [x] Provide a standardized actions area to replace ad-hoc button positioning next to titles
- [x] Zero hardcoded values -- 100% token-driven

## Non-Goals

- Breadcrumb navigation (separate component concern)
- Page-level back buttons (handled by layout/router)
- Responsive collapse of actions (pages handle their own responsive layout)

---

## Allowed / Not Allowed

### ALLOWED after adoption

- Using `<PageHeader>` in all pages that display a page-level title
- Extending via `actions` snippet for page-specific buttons, toggles, or controls
- Omitting subtitle when not needed

### NOT ALLOWED after adoption

> These rules are enforced by the adoption grep test in CI.

- `.page-title {` in any page-scoped `<style>` block (use `<PageHeader>` instead)
- Inline `<h1>` with manual font-size/color for page titles outside of `<PageHeader>`
- Hardcoded `font-size: 1.5rem` or `font-size: 24px` on page title elements

---

## Adoption Strategy

### Progressive Adoption

1. Component is created with tests
2. Reference page is migrated first (proves the component works)
3. Remaining pages migrate one at a time
4. Each migration is a separate commit (easy to revert)

### Reference Page

**Page:** `/config/debug/prompts` (`packages/webui/src/routes/config/debug/prompts/+page.svelte`)
**Why this page:** Simplest page header usage -- just a title and subtitle with no actions. Minimal risk, fast to verify.

### Migration Order

| Order | Page | File | Complexity | Notes |
|-------|------|------|------------|-------|
| 1 | /config/debug/prompts | `routes/config/debug/prompts/+page.svelte` | Low | Reference implementation; title + subtitle only |
| 2 | /config/debug/api | `routes/config/debug/api/+page.svelte` | Low | Same pattern as prompts |
| 3 | /config/debug/events | `routes/config/debug/events/+page.svelte` | Low | Same pattern as prompts |
| 4 | /config/debug/ui | `routes/config/debug/ui/+page.svelte` | Low | Same pattern as prompts |
| 5 | /config/debug | `routes/config/debug/+page.svelte` | Low | Debug index; title + subtitle |
| 6 | /resumes/summaries | `routes/resumes/summaries/+page.svelte` | Medium | May have action buttons |
| 7 | /resumes/templates | `routes/resumes/templates/+page.svelte` | Medium | May have action buttons |
| 8 | /chain | `routes/chain/+page.svelte` | Medium | Check for action buttons |
| 9 | /config (profile) | `routes/config/+page.svelte` | Medium | Profile page header |
| 10 | / (dashboard) | `routes/+page.svelte` | Medium | Dashboard title |
| 11 | /resumes | `routes/resumes/+page.svelte` | Medium | Resumes list page |

### Migration Checklist (per page)

- [ ] Import `PageHeader` from `$lib/components`
- [ ] Replace inline `<h1 class="page-title">` and `<p class="subtitle">` with `<PageHeader>` usage
- [ ] Remove page-scoped `.page-title` and `.subtitle` CSS rules
- [ ] Verify visual match (before/after screenshot or manual check)
- [ ] Verify no regressions in interactive behavior
- [ ] Commit with message: `refactor(webui): migrate [page] to <PageHeader>`

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
  name: 'PageHeader',
  pattern: /\.page-title\s*\{/,
  allowedIn: ['PageHeader.svelte'],
  message: 'Use <PageHeader> instead of page-scoped .page-title styles',
}
```

### CLAUDE.md Rule

```markdown
- Page titles MUST use `<PageHeader>`, not inline `.page-title` CSS
```

---

## Testing

### Unit Tests

| Test | Description |
|------|-------------|
| Renders title | `<PageHeader title="Test" />` renders `<h1>Test</h1>` |
| Renders subtitle | `<PageHeader title="T" subtitle="S" />` renders subtitle `<p>` |
| Omits subtitle when not provided | `<PageHeader title="T" />` does not render `.page-header-subtitle` |
| Renders actions snippet | Actions snippet content appears in `.page-header-actions` |
| Omits actions wrapper when no snippet | No `.page-header-actions` div when actions not provided |

### Component Tests

| Test | Description |
|------|-------------|
| Visual regression | Screenshot comparison of PageHeader with title, subtitle, and actions |
| Token-only styling | Computed styles use only token values, no hardcoded colors/sizes |

### Adoption Enforcement Tests

| Test | Description |
|------|-------------|
| Anti-pattern grep | Fails if `.page-title {` found outside `PageHeader.svelte` |
| Migration count | Tracks remaining pages to migrate (informational, not blocking) |

---

## Acceptance Criteria

- [ ] Component renders correctly with all prop combinations
- [ ] Reference page (`/config/debug/prompts`) migrated with no visual diff
- [ ] All design tokens used (no hardcoded values)
- [ ] Grep test added and passing
- [ ] CLAUDE.md rule added
- [ ] Component exported from `$lib/components/`

## Failure Criteria

- Component introduces a new visual variant instead of matching the canonical pattern from debug subpages
- Hardcoded color/spacing values instead of token references
- Migration breaks interactive behavior on any page
- Component API requires page-specific props (should be generic)
