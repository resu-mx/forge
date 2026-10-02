# Phase 57: UI Consistency & Layout Standardization (Spec B)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-ui-consistency.md](../refs/specs/2026-04-03-ui-consistency.md)
**Depends on:** Phase 42 (Design System — tokens must exist before this phase runs)
**Blocks:** None
**Parallelizable with:** Phase 58 (Profile Menu), Phase 59 (ECharts Infrastructure) — no overlapping files

## Goal

Sweep all route-level `+page.svelte` files to fix spacing inconsistencies, remove redundant UI elements, and convert remaining hardcoded CSS values to Phase 42 design tokens. Three specific improvements:

1. Fix spacing on data pages (Notes, Skills, Organizations) to match `/data/bullets` card spacing.
2. Remove the redundant Type badge from the five experience pages where the route already filters to a single `source_type`.
3. Convert all hardcoded hex colors, font sizes, font weights, border-radius, and box-shadow values in route-level `.svelte` files to Spec A tokens.

## Non-Goals

- Component-level CSS changes (completed in Phase 42)
- New features beyond grouping toggles for Skills and Organizations (UI consistency improvements matching the education page's existing grouping pattern)
- Layout restructuring (pages keep existing split-panel / single-column layouts)
- Responsive / mobile design
- Dark mode visual polish (Phase 42 handles token dark values; this phase only ensures tokens are referenced)

## Context

Phase 42 established `tokens.css` and `base.css` and migrated all `$lib/components/*` to use them. Route-level `+page.svelte` files still contain hardcoded hex colors, pixel/rem font sizes, and inconsistent spacing. This phase is the "last mile" sweep to bring the entire UI onto the design token system.

The existing nav.test.ts shows the navigation structure has 5 top-level entries: Dashboard, Data (4 children — no "Sources"), Opportunities (2 children), Resumes (plain link), Config (4 children). The current `SourcesView.svelte` shows the type badge at line 717 and the `sourceTypeFilter` guard at line 748.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Notes Page Padding | Yes |
| 2. Skills Page Padding & Grouping | Yes |
| 3. Organizations Page Padding & Grouping | Yes |
| 4. Remove Type Badge from Experience Pages | Yes |
| 5. Page-Level CSS Token Sweep | Yes |
| Non-Goals | Yes |
| Testing | Yes |

## Files to Create

None. This phase modifies existing files only.

## Files to Modify

| File | Changes |
|------|---------|
| `packages/webui/src/routes/data/notes/+page.svelte` | Add list padding, card spacing, convert all hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/data/sources/SkillsView.svelte` | Add list padding, card spacing, add group-by-category toggle, convert all hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/data/organizations/+page.svelte` | Add list padding, card spacing, add group-by-org_type and group-by-tag toggles, convert all hardcoded colors/fonts to tokens |
| `packages/webui/src/lib/components/SourcesView.svelte` | Wrap `.type-badge` in `{#if !sourceTypeFilter}` block |
| `packages/webui/src/routes/+page.svelte` (Dashboard) | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/data/domains/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/data/domains/DomainsView.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/data/domains/ArchetypesView.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/opportunities/organizations/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/opportunities/job-descriptions/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/resumes/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/resumes/summaries/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/resumes/templates/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/config/profile/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/config/export/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/config/debug/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/chain/+page.svelte` | Convert hardcoded colors/fonts to tokens |
| `packages/webui/src/routes/data/sources/BulletsView.svelte` | Convert hardcoded colors/fonts to tokens |

## Fallback Strategies

- **Phase 42 tokens not yet merged:** If `tokens.css` is not yet available at implementation time, the conversion rules cannot be applied. This phase MUST wait for Phase 42 to land. No interim measures — hardcoded values remain until tokens exist.
- **Grouping toggle interaction with existing filters:** The Skills and Organizations pages already have filter dropdowns. The grouping toggle is a separate control (not mutually exclusive with filters). If a filter is active and grouping is enabled, groups show only filtered items. Empty groups (all items filtered out) are hidden. If this interaction causes visual issues, fall back to disabling grouping when a filter is active (show a tooltip: "Disable filter to use grouping").
- **SourcesView type badge used in unexpected contexts:** The `{#if !sourceTypeFilter}` guard is safe because `sourceTypeFilter` defaults to `undefined` in the component's props. Any consumer that does not pass `sourceTypeFilter` will continue to see the type badge unchanged. If a new consumer is discovered that passes `sourceTypeFilter` but still needs the badge, the guard can be extended to `{#if !sourceTypeFilter || showTypeBadge}`.
- **Semantic one-off colors:** Some colors (e.g., reference tag colors `.ref-source`, `.ref-bullet`) may not map to existing tokens. These are left as hardcoded values with an inline comment: `/* semantic: no token — reference type color */`. A future design system update can add semantic tokens for these.
- **Font size rounding:** Some existing `font-size` values (e.g., `0.78rem`, `0.85rem`) do not exactly match a token. These are rounded to the nearest token (`--text-sm` for `0.75rem-0.875rem` range, `--text-base` for `0.875rem-1rem` range). Visual differences of ≤1px are acceptable.

---

## Tasks

### T57.1: Add Padding and Token Conversion to Notes Page

**File:** `packages/webui/src/routes/data/notes/+page.svelte`

[CRITICAL] The notes page currently has zero spacing between note cards. Cards sit flush against each other with only a thin `border-bottom: 1px solid #f3f4f6` separating them. This is the highest-visibility spacing issue across data pages.

[IMPORTANT] All hardcoded hex colors must be converted to tokens. The notes page has the most CSS of any data page (~300 lines of `<style>`), making it the largest conversion target.

Apply the following changes to the `<style>` block:

1. **Add padding inside the note list container:**
```css
.note-list {
  padding: var(--space-2) 0;
}
```

2. **Add gap between note cards:**
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

3. **Convert all hardcoded colors to tokens:**

| Hardcoded value | Token |
|----------------|-------|
| `#e5e7eb` | `var(--color-border)` |
| `#6c63ff` | `var(--color-primary)` |
| `#5a52e0` | `var(--color-primary-hover)` |
| `#1a1a1a`, `#1a1a2e` | `var(--text-primary)` |
| `#374151` | `var(--text-secondary)` |
| `#6b7280` | `var(--text-muted)` |
| `#9ca3af` | `var(--text-faint)` |
| `#f9fafb` | `var(--color-surface-raised)` |
| `#f3f4f6` | `var(--color-surface-sunken)` |
| `#eef2ff` | `var(--color-primary-subtle)` |
| `#fff`, `#ffffff` | `var(--color-surface)` |
| `#ef4444` | `var(--color-danger)` |
| `#dc2626` | `var(--color-danger-hover)` |
| `#fee2e2` | `var(--color-danger-subtle)` |
| `rgba(108, 99, 255, 0.15)` | `var(--color-primary-subtle)` |
| `#d1d5db` | `var(--color-border-strong)` |

4. **Convert hardcoded font sizes to tokens:**

| Hardcoded value | Token |
|----------------|-------|
| `1.1rem` | `var(--text-xl)` |
| `0.9rem` | `var(--text-base)` |
| `0.875rem` | `var(--text-base)` |
| `0.85rem` | `var(--text-sm)` |
| `0.8rem` | `var(--text-sm)` |
| `0.75rem` | `var(--text-sm)` |
| `0.7rem` | `var(--text-xs)` |
| `0.6rem` | `var(--text-xs)` |
| `0.78rem` | `var(--text-sm)` |

5. **Convert reference tag colors to tokens where possible.** For `.ref-source`, `.ref-bullet`, etc., use token-based equivalents. If no matching token exists, leave as hardcoded with an inline comment:
```css
.ref-source {
  background: var(--color-primary-subtle);
  color: var(--color-primary);
  /* semantic: reference type color — uses primary palette */
}
```

**Acceptance criteria:**
- Visible padding between note card entries (at least `var(--space-1)` gap).
- Padding around the list container edges (at least `var(--space-2)` top/bottom).
- No hardcoded hex colors remain in the `<style>` block (except documented semantic one-offs).
- No hardcoded `font-size` values remain.
- Visual appearance in light mode is identical to before (tokens map to same hex values).

**Failure criteria:**
- Cards still touch each other with no visible gap.
- Colors change visually in light mode (token mapping is wrong).
- Reference tag colors break (wrong token applied to semantic colors).

---

### T57.2: Add Padding, Grouping, and Token Conversion to Skills Page

**Files:**
- `packages/webui/src/routes/data/skills/+page.svelte`
- `packages/webui/src/routes/data/sources/SkillsView.svelte`

[IMPORTANT] The Skills page wraps `SkillsView` in a full-height container. All CSS work happens in `SkillsView.svelte`, not the wrapper page. The wrapper page has minimal CSS (just `.skills-page-wrapper` height/margin).

[IMPORTANT] Skills already have a `category` field (`CATEGORIES` array: `'ai_ml'`, `'cloud'`, `'database'`, `'devops'`, `'frameworks'`, `'general'`, `'language'`, `'languages'`, `'os'`, `'security'`, `'tools'`). The grouping toggle groups by this existing field — no new data is needed.

1. **Add padding inside the skill list container** (in `SkillsView.svelte`):
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

2. **Add group-by-category support** (in `SkillsView.svelte`):

Add state and derived values:
```typescript
let groupBy = $state<'flat' | 'by_category'>('flat')

let groupedSkills = $derived.by(() => {
  if (groupBy !== 'by_category') return null
  const groups: Record<string, Skill[]> = {}
  for (const skill of filteredSkills) {
    const cat = skill.category ?? 'general'
    if (!groups[cat]) groups[cat] = []
    groups[cat].push(skill)
  }
  // Sort groups alphabetically by category name
  return Object.entries(groups).sort(([a], [b]) => a.localeCompare(b))
})

// Track collapsed groups
let collapsedGroups = $state<Record<string, boolean>>({})

function toggleGroup(group: string) {
  collapsedGroups[group] = !collapsedGroups[group]
}
```

Add group bar UI below the filter bar:
```svelte
<div class="group-bar">
  <label for="skill-group-by">Group by:</label>
  <select id="skill-group-by" bind:value={groupBy}>
    <option value="flat">None</option>
    <option value="by_category">Category</option>
  </select>
</div>
```

Add grouped rendering with collapsible sections:
```svelte
{#if groupBy === 'by_category' && groupedSkills}
  {#each groupedSkills as [category, categorySkills]}
    <div class="group-section">
      <button class="group-header" onclick={() => toggleGroup(category)}>
        <span class="group-chevron" class:collapsed={collapsedGroups[category]}>&#9656;</span>
        <span class="group-label">{category.replace(/_/g, ' ')}</span>
        <span class="group-count">{categorySkills.length}</span>
      </button>
      {#if !collapsedGroups[category]}
        <ul class="skill-list">
          {#each categorySkills as skill}
            <!-- existing skill card rendering -->
          {/each}
        </ul>
      {/if}
    </div>
  {/each}
{:else}
  <ul class="skill-list">
    {#each filteredSkills as skill}
      <!-- existing skill card rendering -->
    {/each}
  </ul>
{/if}
```

Add group section styles (matching `SourcesView` pattern):
```css
.group-bar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--color-border);
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.group-bar select {
  padding: var(--space-1) var(--space-2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  background: var(--color-surface);
  color: var(--text-primary);
}

.group-section {
  border-bottom: 1px solid var(--color-border);
}

.group-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-2) var(--space-3);
  background: var(--color-surface-sunken);
  border: none;
  cursor: pointer;
  font-size: var(--text-sm);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
}

.group-header:hover {
  background: var(--color-surface-raised);
}

.group-chevron {
  display: inline-block;
  transition: transform 0.15s ease;
  font-size: var(--text-xs);
  color: var(--text-muted);
}

.group-chevron.collapsed {
  transform: rotate(0deg);
}

.group-chevron:not(.collapsed) {
  transform: rotate(90deg);
}

.group-label {
  text-transform: capitalize;
}

.group-count {
  margin-left: auto;
  font-size: var(--text-xs);
  color: var(--text-faint);
  font-weight: var(--font-normal);
}
```

3. **Convert all hardcoded hex colors and font sizes to tokens** (same mapping as T57.1).

**Acceptance criteria:**
- Visible padding between skill card entries.
- Group-by-category toggle appears below the filter bar.
- When grouping is active, skills are bucketed under collapsible section headers (one per category).
- Section headers show category name (capitalized, underscores replaced with spaces) and count.
- Clicking a section header collapses/expands the group.
- When flat, skills render as before.
- No hardcoded hex colors or font sizes remain in the `<style>` block.

**Failure criteria:**
- Grouping breaks the filter (filtered skills should still be grouped correctly).
- Empty groups are shown (groups with zero skills after filtering should be hidden).
- Section headers have no click handler (not collapsible).

---

### T57.3: Add Padding, Grouping, and Token Conversion to Organizations Page

**File:** `packages/webui/src/routes/data/organizations/+page.svelte`

[IMPORTANT] Organizations have both `org_type` and `tags` fields. The `org_type` is a single value from `ORG_TYPES` array. Tags is an array — grouping by tag uses the first tag, with "No tags" for untagged items.

1. **Add padding inside the organization list container:**
```css
.org-list {
  padding: var(--space-2) 0;
}

.org-list li {
  padding: 0 var(--space-3);
  margin-bottom: var(--space-1);
}

.org-card {
  border-radius: var(--radius-md);
  border-bottom: none;
}
```

2. **Add group-by support:**

Add state and derived values:
```typescript
let groupBy = $state<'flat' | 'by_org_type' | 'by_tag'>('flat')

let groupedOrgs = $derived.by(() => {
  if (groupBy === 'flat') return null
  const groups: Record<string, Organization[]> = {}

  for (const org of filteredOrgs) {
    let key: string
    if (groupBy === 'by_org_type') {
      key = org.org_type ?? 'other'
    } else {
      // by_tag: use first tag, or "No tags" if untagged
      key = (org.tags && org.tags.length > 0) ? org.tags[0] : 'No tags'
    }
    if (!groups[key]) groups[key] = []
    groups[key].push(org)
  }

  return Object.entries(groups).sort(([a], [b]) => a.localeCompare(b))
})

let collapsedGroups = $state<Record<string, boolean>>({})

function toggleGroup(group: string) {
  collapsedGroups[group] = !collapsedGroups[group]
}
```

Add group bar UI:
```svelte
<div class="group-bar">
  <label for="org-group-by">Group by:</label>
  <select id="org-group-by" bind:value={groupBy}>
    <option value="flat">None</option>
    <option value="by_org_type">Type</option>
    <option value="by_tag">Tag</option>
  </select>
</div>
```

Add grouped rendering (same collapsible section pattern as T57.2):
```svelte
{#if groupBy !== 'flat' && groupedOrgs}
  {#each groupedOrgs as [groupName, groupOrgs]}
    <div class="group-section">
      <button class="group-header" onclick={() => toggleGroup(groupName)}>
        <span class="group-chevron" class:collapsed={collapsedGroups[groupName]}>&#9656;</span>
        <span class="group-label">{groupName.replace(/_/g, ' ')}</span>
        <span class="group-count">{groupOrgs.length}</span>
      </button>
      {#if !collapsedGroups[groupName]}
        <ul class="org-list">
          {#each groupOrgs as org}
            <!-- existing org card rendering -->
          {/each}
        </ul>
      {/if}
    </div>
  {/each}
{:else}
  <ul class="org-list">
    {#each filteredOrgs as org}
      <!-- existing org card rendering -->
    {/each}
  </ul>
{/if}
```

3. **Convert all hardcoded hex colors and font sizes to tokens** (same mapping as T57.1).

**Acceptance criteria:**
- Visible padding between org card entries.
- Group-by dropdown with three options: None, Type, Tag.
- When grouping by org_type, organizations bucketed under collapsible headers (`company`, `nonprofit`, etc.).
- When grouping by tag, organizations bucketed by first tag; untagged items under "No tags" group.
- Section headers show group name (capitalized, underscores replaced with spaces) and count.
- No hardcoded hex colors or font sizes remain.

**Failure criteria:**
- Group-by-tag shows an organization under multiple groups (only first tag is used).
- "No tags" group is missing for organizations without tags.
- Grouping breaks existing tag filter interaction.

---

### T57.4: Remove Type Badge from Experience Pages

**File:** `packages/webui/src/lib/components/SourcesView.svelte`

[CRITICAL] The type badge at line 717 renders for every source card. When `sourceTypeFilter` is set (all 5 experience pages), every card shows the same redundant badge (e.g., "role" on every card in `/experience/roles`). The filter tabs are already hidden when `sourceTypeFilter` is set (line 748), but the type badge was not given the same treatment.

Wrap the type badge in an `{#if !sourceTypeFilter}` block:

```svelte
<!-- Before (line 717): -->
<span class="type-badge type-{source.source_type}">{source.source_type}</span>

<!-- After: -->
{#if !sourceTypeFilter}
  <span class="type-badge type-{source.source_type}">{source.source_type}</span>
{/if}
```

[MINOR] The `.type-badge` CSS styles remain in the `<style>` block. They are still needed when `sourceTypeFilter` is not set (e.g., if SourcesView is used on `/data/sources` without a filter).

**Acceptance criteria:**
- On `/experience/roles`, `/experience/projects`, `/experience/education`, `/experience/clearances`, `/experience/general`: no Type badge visible in source cards.
- On any view where `sourceTypeFilter` is `undefined` (e.g., the data sources view): Type badge still visible.
- No CSS changes needed — only the template `{#if}` guard.

**Failure criteria:**
- Type badge hidden on views that do NOT use `sourceTypeFilter`.
- Template syntax error from misplaced `{#if}` block.

---

### T57.5: Page-Level CSS Token Sweep

**Files:** All route-level `.svelte` files listed in the scope table.

[IMPORTANT] This is a mechanical find-and-replace task across ~15 files. Each file has its own `<style>` block with hardcoded values. The conversion follows a consistent mapping (same as T57.1). No logic changes — only CSS values change.

For each file in the sweep list:

1. **Replace hardcoded hex colors** with `var(--token)` using the mapping from T57.1.
2. **Replace hardcoded `font-size` values** with `var(--text-*)` tokens using the mapping from T57.1.
3. **Replace hardcoded `font-weight` values:**

| Hardcoded value | Token |
|----------------|-------|
| `400` | `var(--font-normal)` |
| `500` | `var(--font-medium)` |
| `600` | `var(--font-semibold)` |
| `700` | `var(--font-bold)` |

4. **Replace hardcoded `border-radius` values:**

| Hardcoded value | Token |
|----------------|-------|
| `4px`, `0.25rem` | `var(--radius-sm)` |
| `6px`, `0.375rem` | `var(--radius-md)` |
| `8px`, `0.5rem` | `var(--radius-lg)` |
| `12px`, `0.75rem` | `var(--radius-xl)` |
| `50%`, `9999px` | `var(--radius-full)` |

5. **Replace hardcoded `box-shadow` values** with `var(--shadow-*)` tokens where applicable.

6. **Keep layout-specific values** (widths, heights, flex ratios, grid definitions) as-is.

7. **Remove local `.btn` / `.btn-primary` redeclarations** where they duplicate `base.css` classes.

Sweep order (by CSS volume, descending):
1. `routes/+page.svelte` (Dashboard) — likely largest route-level CSS
2. `routes/data/domains/DomainsView.svelte`
3. `routes/data/domains/ArchetypesView.svelte`
4. `routes/data/domains/+page.svelte`
5. `routes/opportunities/organizations/+page.svelte`
6. `routes/opportunities/job-descriptions/+page.svelte`
7. `routes/resumes/+page.svelte`
8. `routes/resumes/summaries/+page.svelte`
9. `routes/resumes/templates/+page.svelte`
10. `routes/config/profile/+page.svelte`
11. `routes/config/export/+page.svelte`
12. `routes/config/debug/+page.svelte`
13. `routes/chain/+page.svelte`
14. `routes/data/sources/BulletsView.svelte`
15. `routes/data/sources/SourcesView.svelte` (route-level component, not `$lib`)

[ANTI-PATTERN] Some files may have locally-scoped `.btn` styles that shadow `base.css` classes. These should be removed. Verify each removal by checking that the element uses the base class and does not rely on the local override for layout-specific styling.

[GAP] The `routes/+layout.svelte` is listed as "Already done in Spec A" but should be spot-checked to confirm no hardcoded values remain.

**Acceptance criteria:**
- Every route-level `.svelte` file under `packages/webui/src/routes/` uses `var(--token)` references instead of hardcoded hex colors and rem/px font sizes.
- No file contains raw `#` color values in its `<style>` block (exception: documented semantic one-offs with inline comment).
- No file contains hardcoded `font-size`, `font-weight`, `border-radius` values that have a token equivalent.
- Light-mode appearance is visually identical (tokens map to same values).

**Failure criteria:**
- A token is used that does not exist in `tokens.css` (runtime CSS error, property falls through to browser default).
- A local `.btn` class removal breaks a button's appearance (the base class does not cover all needed styles).
- Hardcoded values remain in a file that was supposed to be swept.

---

## Testing Support

### Visual Tests (Primary)

All testing for this phase is visual. No automated CSS test framework is used.

| Test | What to verify |
|------|---------------|
| Side-by-side comparison | For each modified page, compare before/after screenshots. Colors, spacing, typography should look identical in light mode. |
| Dark mode spot-check | Toggle `data-theme="dark"` on `<html>` and verify all pages render with dark colors from tokens. No stray white backgrounds or black-on-black text. |
| Notes page spacing | Visible padding between note cards and around the list container. |
| Skills page spacing | Visible padding between skill cards. |
| Skills page grouping | Toggle group-by-category. Skills grouped under collapsible headers with correct counts. Toggle collapse/expand. |
| Organizations page spacing | Visible padding between org cards. |
| Organizations page grouping | Toggle group-by-org_type. Toggle group-by-tag. Verify grouping works. |
| Experience pages badge | Navigate to each of 5 experience routes. Type badge NOT shown. Navigate to a view WITHOUT `sourceTypeFilter`. Type badge IS shown. |
| Full walkthrough | Click through every route in sidebar. No visual regressions, no missing styles, no unstyled elements, no white-on-white text. |

### Smoke Tests (Automated)

No new automated tests are required for CSS-only changes. Existing component tests continue to pass because template structure is unchanged (only CSS values change and one `{#if}` guard is added).

For the grouping toggles (Skills and Organizations), manual verification is sufficient because:
- The grouping logic is `$derived` from existing state — no new API calls.
- The collapsible section UI is a pattern already established in `SourcesView` education grouping.
- Automated testing of Svelte reactive templates requires a component test harness (Playwright) which is not yet established.

### Regression Gate

Before merging, verify:
1. `bun run check` passes (no TypeScript errors from template changes).
2. `bun run build` succeeds (no broken imports or missing tokens).
3. All existing Vitest tests pass (no test touches route-level CSS).

---

## Documentation Requirements

- No new documentation files.
- The spec file serves as the design document.
- This plan file serves as the implementation reference.
- Inline code comments for:
  - Semantic one-off colors that are not converted to tokens (with rationale).
  - `{#if !sourceTypeFilter}` guard on type badge (link to this spec section 4).
  - Grouping toggle state management in Skills and Organizations (note that it follows the education grouping pattern from `SourcesView`).

---

## Parallelization Notes

**Within this phase:**
- T57.1 (Notes), T57.2 (Skills), T57.3 (Organizations), T57.4 (Type Badge), and T57.5 (Sweep) are all independent — they modify different files with no imports between them.
- T57.4 modifies `$lib/components/SourcesView.svelte` while T57.5 modifies `routes/data/sources/SourcesView.svelte` (a different file). No conflict.
- All five tasks can be developed in parallel.

**Recommended execution order:**
1. T57.4 (Type Badge — smallest change, highest confidence, validates the `sourceTypeFilter` pattern)
2. T57.1 (Notes — establishes the padding/card spacing pattern that T57.2 and T57.3 follow)
3. T57.2 + T57.3 (Skills + Organizations — parallel, both follow the pattern from T57.1)
4. T57.5 (CSS Sweep — mechanical, does last, benefits from having the color mapping validated by T57.1-T57.3)

**Cross-phase:**
- This phase depends on Phase 42 (tokens must exist).
- This phase is independent of Phase 58 (Profile Menu) and Phase 59 (ECharts Infrastructure).
- All three phases (57, 58, 59) can run in parallel as they modify non-overlapping files.
- Exception: Phase 58 also modifies `+layout.svelte` (adds profile button). If Phase 57's sweep (T57.5) touches `+layout.svelte` CSS, coordinate with Phase 58. The spec says `+layout.svelte` is "Already done in Spec A" so T57.5 only spot-checks it — no conflict expected.
