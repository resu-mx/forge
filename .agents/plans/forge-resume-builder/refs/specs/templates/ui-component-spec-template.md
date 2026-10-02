# UI Component Spec Template

> Copy this template for each shared UI component spec. Fill in all sections.
> Delete this instruction block in the final spec.

# [Component Name] — UI Component Spec

**Date:** YYYY-MM-DD
**Status:** Draft
**Package:** `@forge/webui`
**Category:** structural | polish
**Replaces:** [list of per-page implementations this component eliminates]

---

## Overview

**What:** One sentence describing what this component is.

**Why:** What problem does it solve? Reference the audit finding (inconsistency, duplication, bug).

**Pages affected:** List every page that currently implements this pattern inline and will migrate to this component.

---

## Component API

### Props

| Prop | Type | Default | Required | Description |
|------|------|---------|----------|-------------|
| ... | ... | ... | ... | ... |

### Snippets (Slots)

| Snippet | Description | When to use |
|---------|-------------|-------------|
| ... | ... | ... |

### Events / Callbacks

| Callback | Signature | Description |
|----------|-----------|-------------|
| ... | ... | ... |

### TypeScript Types

```typescript
// Export from $lib/components/[ComponentName].svelte or $lib/types.ts
interface ComponentNameProps {
  // ...
}
```

---

## Styling

### Concrete CSS (copy-paste)

```css
/* Complete CSS for this component — uses design tokens exclusively */
```

### Token Consumption

| Token | Usage | Fallback |
|-------|-------|----------|
| `--color-primary` | ... | ... |
| ... | ... | ... |

### Branding Strategy

How does this component support rebranding? Which tokens control its appearance?

> All visual properties (colors, spacing, typography, radii) MUST come from tokens in
> `packages/webui/src/lib/styles/tokens.css`. Rebranding means changing token values
> only — no component CSS changes needed.

### Rendered Markup

```html
<!-- The exact HTML structure this component produces -->
```

---

## Behavior

### State Management

How does this component manage internal state? What state flows in via props vs. managed internally?

### Accessibility

- Keyboard navigation requirements
- ARIA attributes
- Focus management

### Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| ... | ... |

---

## Examples

### Explicit Examples (DO THIS)

```svelte
<!-- Example 1: Basic usage — [description] -->
```

```svelte
<!-- Example 2: With optional features — [description] -->
```

### Implicit Examples (THIS IS THE PATTERN)

> These examples show how the component composes with other shared components
> to form a complete page. They demonstrate the intended architectural pattern.

```svelte
<!-- Typical page composition showing this component in context -->
```

### Anti-Examples (DON'T DO THIS)

```svelte
<!-- WRONG: [description of what's wrong and why] -->
```

```css
/* WRONG: [description of the anti-pattern] */
```

---

## Goals

- [ ] ...
- [ ] ...

## Non-Goals

- ...
- ...

---

## Allowed / Not Allowed

### ALLOWED after adoption

- Using `<ComponentName>` in all pages matching this pattern
- Extending via snippets/slots for page-specific content
- ...

### NOT ALLOWED after adoption

> These rules are enforced by the adoption grep test in CI.

- ❌ [specific CSS pattern] in any page-scoped `<style>` block
- ❌ [specific markup pattern] instead of using this component
- ❌ ...

---

## Adoption Strategy

### Progressive Adoption

1. Component is created with tests
2. Reference page is migrated first (proves the component works)
3. Remaining pages migrate one at a time
4. Each migration is a separate commit (easy to revert)

### Reference Page

**Page:** `[route path]` (`[file path]`)
**Why this page:** [simplest / most representative / fewest edge cases]

### Migration Order

| Order | Page | File | Complexity | Notes |
|-------|------|------|------------|-------|
| 1 | [reference] | ... | Low | Reference implementation |
| 2 | ... | ... | ... | ... |

### Migration Checklist (per page)

- [ ] Import shared component
- [ ] Replace inline markup with component usage
- [ ] Remove page-scoped CSS that the component now provides
- [ ] Verify visual match (before/after screenshot or manual check)
- [ ] Verify no regressions in interactive behavior
- [ ] Commit with message: `refactor(webui): migrate [page] to <ComponentName>`

### Coexistence Rules

During migration, old and new implementations coexist. Rules:

- New pages MUST use the shared component (enforced by CLAUDE.md rule)
- Existing pages keep their inline CSS until migrated (no rush)
- The grep test tracks remaining violations — count decreases toward zero
- Component API is frozen after reference page ships (no breaking changes during migration)

---

## Adoption Enforcement

### CI Grep Test

```typescript
// In packages/webui/src/__tests__/component-adoption.test.ts
{
  name: 'ComponentName',
  pattern: /[regex matching the anti-pattern]/,
  allowedIn: ['ComponentName.svelte'],
  message: 'Use <ComponentName> instead of [anti-pattern description]',
}
```

### CLAUDE.md Rule

```markdown
- [Layout type] MUST use `<ComponentName>`, not [anti-pattern description]
```

---

## Testing

### Unit Tests

| Test | Description |
|------|-------------|
| ... | ... |

### Component Tests

| Test | Description |
|------|-------------|
| ... | ... |

### Adoption Enforcement Tests

| Test | Description |
|------|-------------|
| Anti-pattern grep | Fails if [pattern] found outside [component file] |
| Migration count | Tracks remaining pages to migrate (informational, not blocking) |

---

## Acceptance Criteria

- [ ] Component renders correctly with all prop combinations
- [ ] Reference page migrated with no visual diff
- [ ] All design tokens used (no hardcoded values)
- [ ] Grep test added and passing
- [ ] CLAUDE.md rule added
- [ ] Component exported from `$lib/components/`

## Failure Criteria

- Component introduces a 9th visual variant instead of matching the canonical pattern
- Hardcoded color/spacing values instead of token references
- Migration breaks interactive behavior on any page
- Component API requires page-specific props (should be generic)
