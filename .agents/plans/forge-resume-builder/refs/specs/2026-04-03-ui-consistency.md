# UI Consistency & Layout Standardization

**Date:** 2026-04-03
**Spec:** B (UI Consistency)
**Depends on:** Spec A (Design System & CSS Variables) -- tokens must exist before this spec runs
**Phase:** TBD (after Spec A)

## Overview

After Spec A establishes the design token infrastructure (`tokens.css`, `base.css`) and migrates all `$lib/components/*` to use them, this spec sweeps the remaining route-level `+page.svelte` files. The goal is three-fold:

1. Fix spacing inconsistencies on data pages (Notes, Skills, Organizations) so they match the card spacing established on `/data/bullets`.
2. Remove the redundant Type badge from the five experience pages where the route already filters to a single `source_type`.
3. Convert all remaining hardcoded hex colors and font sizes in route-level `.svelte` files to Spec A tokens.

No new features beyond the grouping toggles specified in Sections 2 and 3 are introduced, which are classified as UI consistency improvements (matching the education page's existing grouping pattern). No component-level CSS changes are made (those are completed in Spec A).

---

## 1. Notes Page Padding (`/data/notes`)

**File:** `packages/webui/src/routes/data/notes/+page.svelte`

### Current state

The notes page uses a split-panel layout (`.list-panel` + `.editor-panel`) with `gap: 0` on the parent flex container. Individual `.note-card` entries sit flush against each other with only a thin `border-bottom: 1px solid #f3f4f6` separating them. There is no vertical spacing between cards, and the list container itself has no internal padding around the card stack.

### Changes

1. **Add padding inside the note list container.** The `.note-list` element should gain top/bottom padding so entries are not flush against the filter bar above and the container edge below.
   ```css
   .note-list {
     padding: var(--space-2) 0;
   }
   ```

2. **Add gap between note cards.** Each `.note-card` entry should have a small bottom margin or the parent list should use `gap` to introduce spacing between entries, matching the `/data/bullets` pattern.
   ```css
   .note-list li {
     padding: 0 var(--space-3);
     margin-bottom: var(--space-1);
   }

   .note-card {
     background: var(--color-surface);
     border: 1px solid var(--color-border);
     border-radius: var(--radius-md);
     /* Remove the border-bottom separator in favor of card spacing */
     border-bottom: none;
   }
   ```

3. **Convert all hardcoded colors in `<style>` to tokens.** Examples:
   - `#e5e7eb` -> `var(--color-border)`
   - `#6c63ff` -> `var(--color-primary)`
   - `#5a52e0` -> `var(--color-primary-hover)`
   - `#1a1a1a` -> `var(--text-primary)`
   - `#374151` -> `var(--text-secondary)`
   - `#6b7280` -> `var(--text-muted)`
   - `#9ca3af` -> `var(--text-faint)`
   - `#f9fafb` -> `var(--color-surface-raised)`
   - `#f3f4f6` -> `var(--color-surface-sunken)`
   - `#eef2ff` -> `var(--color-primary-subtle)` (selected card bg)
   - `#fff` -> `var(--color-surface)`
   - `#ef4444` -> `var(--color-danger)`
   - `#dc2626` -> `var(--color-danger-hover)`
   - `#fee2e2` -> `var(--color-danger-subtle)`
   - `rgba(108, 99, 255, 0.15)` -> `var(--color-primary-subtle)`
   - `#d1d5db` -> `var(--color-border-strong)`

4. **Convert hardcoded font sizes to tokens.** Examples:
   - `1.1rem` -> `var(--text-xl)`
   - `0.9rem` -> `var(--text-base)` (approximate)
   - `0.875rem` -> `var(--text-base)`
   - `0.85rem` -> `var(--text-sm)`
   - `0.8rem` -> `var(--text-sm)`
   - `0.75rem` -> `var(--text-sm)`
   - `0.7rem` -> `var(--text-xs)`
   - `0.6rem` -> `var(--text-xs)`
   - `0.78rem` -> `var(--text-sm)`

5. **Convert reference tag colors to tokens.** The `.ref-source`, `.ref-bullet`, etc. color classes should use token-based equivalents where possible, or be left as semantic one-offs if no matching token exists (document this decision in a code comment).

---

## 2. Skills Page Padding & Grouping (`/data/skills`)

**Files:**
- `packages/webui/src/routes/data/skills/+page.svelte`
- `packages/webui/src/routes/data/sources/SkillsView.svelte`

### Current state

The Skills page wraps `SkillsView` in a full-height container. `SkillsView` uses a split-panel layout similar to Notes. The `.skill-list` has no internal padding, and skill cards sit flush. Skills already have a `category` field but are displayed in a flat list -- there is no grouping option.

### Changes

1. **Add padding inside the skill list container.** Same pattern as Notes:
   ```css
   .skill-list {
     padding: var(--space-2) 0;
   }

   .skill-list li {
     padding: 0 var(--space-3);
     margin-bottom: var(--space-1);
   }

   .skill-card {
     border-radius: var(--radius-md);
     border-bottom: none;
   }
   ```

2. **Add group-by-category support.** Introduce a grouping toggle (similar to the education grouping in `SourcesView`) that groups skills by their `category` field. When grouping is active, skills are rendered under collapsible section headers (one per category). When flat, they render as they do today.

   Implementation approach:
   - Add a `groupBy` state variable: `'flat' | 'by_category'`
   - Add a group bar below the filter bar with a `<select>` to toggle grouping
   - Add a `$derived` that computes grouped skills when `groupBy === 'by_category'`
   - Reuse the same `.group-section`, `.group-header`, `.group-chevron`, `.group-label`, `.group-count` classes from `SourcesView` (or import from `base.css` if Spec A extracted them)

3. **Convert all hardcoded hex colors and font sizes to Spec A tokens** (same mapping as Notes above -- identical color values are used).

---

## 3. Organizations Page Padding & Grouping (`/data/organizations`)

**File:** `packages/webui/src/routes/data/organizations/+page.svelte`

### Current state

The Organizations page uses a split-panel layout with a long list of organizations. It already has a tag filter dropdown but no grouping. The list has no internal padding.

### Changes

1. **Add padding inside the organization list container.** Same pattern as Notes and Skills.

2. **Add group-by support.** Introduce a grouping toggle that groups organizations by `org_type` or by `tags`. When grouping by `org_type`, organizations are bucketed under collapsible headers for each type (`company`, `nonprofit`, `government`, `military`, `education`, `volunteer`, `freelance`, `other`). When grouping by tags, they are bucketed by first tag (an organization can have multiple tags, so the first is used for grouping; a "No tags" group catches untagged items).

   Implementation approach:
   - Add a `groupBy` state variable: `'flat' | 'by_org_type' | 'by_tag'`
   - Add a group bar below the filter bar
   - Add a `$derived` that computes grouped organizations
   - Reuse collapsible group section UI pattern from SourcesView

3. **Convert all hardcoded hex colors and font sizes to Spec A tokens.**

---

## 4. Remove Type Badge from Experience Pages

**Files:**
- `packages/webui/src/lib/components/SourcesView.svelte` (the shared component used by all 5 experience routes)

**Routes affected:**
- `/experience/roles` -- passes `sourceTypeFilter="role"`
- `/experience/projects` -- passes `sourceTypeFilter="project"`
- `/experience/education` -- passes `sourceTypeFilter="education"`
- `/experience/clearances` -- passes `sourceTypeFilter="clearance"`
- `/experience/general` -- passes `sourceTypeFilter="general"`

### Current state

The `SourcesView` component renders a `.type-badge` in every source card's `.card-meta` row:
```svelte
<span class="type-badge type-{source.source_type}">{source.source_type}</span>
```
This badge shows "role", "project", "education", etc. When the component is used on an experience page with `sourceTypeFilter` set, every card in the list has the same badge (e.g., all say "role" on `/experience/roles`). This is redundant because the route already filters to that type.

Note: the filter tabs are already hidden when `sourceTypeFilter` is set (line 748: `{#if !sourceTypeFilter}`). The type badge in the card itself was not given the same treatment.

### Changes

1. **Conditionally hide the `.type-badge`** when `sourceTypeFilter` is set. Wrap the type badge span in an `{#if !sourceTypeFilter}` block:
   ```svelte
   {#if !sourceTypeFilter}
     <span class="type-badge type-{source.source_type}">{source.source_type}</span>
   {/if}
   ```

2. This change is in the shared `$lib/components/SourcesView.svelte`, not in individual route files. The route-level `+page.svelte` files for experience pages are one-liners (`<SourcesView sourceTypeFilter="role" />`) and do not need modification.

3. Verify that `SourcesView` when used without `sourceTypeFilter` (i.e., on `/data/sources` if it exists, or when `sourceTypeFilter` is `undefined`) still shows the type badge.

---

## 5. Page-Level CSS Token Sweep

**Files to sweep (all route-level `.svelte` files):**

| File | Key hardcoded values to convert |
|------|-------------------------------|
| `routes/+page.svelte` (Dashboard) | Colors, font sizes |
| `routes/+layout.svelte` | Already done in Spec A (sidebar tokens) |
| `routes/data/bullets/+page.svelte` | Wrapper only -- minimal CSS |
| `routes/data/skills/+page.svelte` | Wrapper only -- minimal CSS |
| `routes/data/notes/+page.svelte` | Covered in section 1 above |
| `routes/data/organizations/+page.svelte` | Covered in section 3 above |
| `routes/data/domains/+page.svelte` | Colors, font sizes |
| `routes/data/domains/DomainsView.svelte` | Colors, font sizes |
| `routes/data/domains/ArchetypesView.svelte` | Colors, font sizes |
| `routes/experience/roles/+page.svelte` | No CSS (one-liner) |
| `routes/experience/projects/+page.svelte` | No CSS (one-liner) |
| `routes/experience/education/+page.svelte` | No CSS (one-liner) |
| `routes/experience/clearances/+page.svelte` | No CSS (one-liner) |
| `routes/experience/general/+page.svelte` | No CSS (one-liner) |
| `routes/opportunities/organizations/+page.svelte` | Colors, font sizes |
| `routes/opportunities/job-descriptions/+page.svelte` | Colors, font sizes |
| `routes/resumes/+page.svelte` | Colors, font sizes |
| `routes/resumes/summaries/+page.svelte` | Colors, font sizes |
| `routes/resumes/templates/+page.svelte` | Colors, font sizes |
| `routes/config/profile/+page.svelte` | Colors, font sizes |
| `routes/config/export/+page.svelte` | Colors, font sizes |
| `routes/config/debug/+page.svelte` | Colors, font sizes |
| `routes/chain/+page.svelte` | Colors, font sizes |
| `routes/data/sources/SourcesView.svelte` | Colors, font sizes (route-level component) |
| `routes/data/sources/SkillsView.svelte` | Colors, font sizes (route-level component) |
| `routes/data/sources/BulletsView.svelte` | Colors, font sizes (route-level component) |

### Conversion rules

For each file:
1. Replace hardcoded hex colors with the corresponding `var(--token)` from the mapping in Spec A section 4.
2. Replace hardcoded `font-size` values with `var(--text-*)` tokens.
3. Replace hardcoded `font-weight` values with `var(--font-*)` tokens.
4. Replace hardcoded `border-radius` values with `var(--radius-*)` tokens.
5. Replace hardcoded `box-shadow` values with `var(--shadow-*)` tokens where applicable.
6. Replace hardcoded spacing values with `var(--space-*)` tokens only where the value exactly matches a scale step; do not force non-standard values into the scale.
7. Keep layout-specific values (widths, heights, flex ratios, grid definitions) as-is -- those are not part of the design system.
8. Replace `.btn` / `.btn-primary` / etc. local redeclarations with the base.css classes (remove the local `<style>` rules).

---

## Non-Goals

- **Component-level CSS changes** -- already handled by Spec A
- **New features** -- no new features beyond the grouping toggles specified in Sections 2 and 3, which are classified as UI consistency improvements (matching the education page's existing grouping pattern)
- **Layout restructuring** -- pages keep their existing split-panel / single-column layouts
- **Responsive design** -- not in scope for this pass
- **Dark mode testing** -- dark theme correctness is verified in Spec A; this spec only ensures tokens are referenced, not that dark values look good

---

## Testing

All testing is visual (no automated CSS tests).

1. **Side-by-side comparison:** For each modified page, compare before/after screenshots to verify no visual regressions. Colors, spacing, and typography should look identical in light mode (tokens map to the same hex values currently hardcoded).
2. **Dark mode spot-check:** Toggle `data-theme="dark"` on `<html>` and verify that all pages render with dark colors from `tokens.css` (no stray white backgrounds or black-on-black text).
3. **Notes page:** Verify visible padding between note cards and around the list container.
4. **Skills page:** Verify visible padding between skill cards. Toggle group-by-category and verify skills are grouped under collapsible section headers with correct counts.
5. **Organizations page:** Verify visible padding between org cards. Toggle group-by-org_type and group-by-tag and verify grouping works.
6. **Experience pages:** Navigate to each of the 5 experience routes and verify the Type badge is no longer shown in source cards. Navigate to a view that does NOT use `sourceTypeFilter` and verify the Type badge still appears.
7. **Full app walkthrough:** Click through every route in the sidebar and verify no visual regressions (no missing styles, no unstyled elements, no white-on-white text).

---

## Acceptance Criteria

1. **Notes page** (`/data/notes`): Visible padding between note card entries and around the list container edge. No hardcoded hex colors or font sizes remain in the `<style>` block.
2. **Skills page** (`/data/skills`): Visible padding between skill card entries. Group-by-category toggle works and groups skills under collapsible headers. No hardcoded hex colors or font sizes remain.
3. **Organizations page** (`/data/organizations`): Visible padding between org card entries. Group-by-org_type and group-by-tag toggles work. No hardcoded hex colors or font sizes remain.
4. **Experience pages** (all 5): Type badge (`<span class="type-badge ...">`) is not rendered when `sourceTypeFilter` is set. Type badge is still rendered when `sourceTypeFilter` is `undefined`.
5. **CSS sweep**: Every route-level `.svelte` file under `packages/webui/src/routes/` uses `var(--token)` references instead of hardcoded hex colors and `px`/`rem` font sizes. No file contains raw `#` color values in its `<style>` block (exception: one-off semantic colors documented with an inline comment explaining why no token exists).
6. **No visual regressions**: Light-mode appearance is visually consistent in light mode (typography values are rounded to the nearest token in the design scale; differences of ≤1px are acceptable).
7. **Dark mode functional**: Switching to `data-theme="dark"` produces a coherent dark UI across all pages (no stray light backgrounds or unreadable text).
