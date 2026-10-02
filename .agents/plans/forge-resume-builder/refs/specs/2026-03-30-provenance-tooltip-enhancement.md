# Provenance Tooltip Enhancement: Meaningful Names + Chain Links

**Date:** 2026-03-30
**Status:** Design
**Builds on:** Resume Renderer (Phases 19-20), Chain View (Phase 15)

## Purpose

Replace UUID-only provenance tooltips in the DragNDrop resume view with human-readable names and clickable links to the Chain View graph. Currently hovering over a resume bullet shows raw UUIDs for source, bullet, and perspective — this is meaningless to users.

## Goals

1. Show meaningful content previews in provenance tooltips (source title, bullet preview, perspective preview)
2. Make tooltip items clickable — navigate to Chain View with the relevant node highlighted
3. Single-source change in the IR compiler — all views that consume `source_chain` benefit. Currently only DragNDropView renders provenance tooltips.

## Non-Goals

- Full inline chain visualization (that's what the Chain View is for)
- Editing source/bullet/perspective from the tooltip
- Tooltip for non-experience items (skills, education sections don't have source_chain)

---

## 1. Enrich `source_chain` in the IR

### 1.1 Updated Type

The `source_chain` on `ExperienceBullet` currently stores only IDs:

```typescript
source_chain?: {
  perspective_id: string
  bullet_id: string
  source_id: string
}
```

Add human-readable fields:

```typescript
source_chain?: {
  source_id: string
  source_title: string              // e.g., "Principal Cloud Forensics Engineer"
  bullet_id: string
  bullet_preview: string            // first 60 chars of bullet content
  perspective_id: string
  perspective_preview: string       // first 60 chars of perspective content
}
```

### 1.2 IR Compiler Change

The experience, project, and presentation builder queries currently do NOT join the `bullets` table. They join `perspectives` (which has `bullet_id`) and `bullet_sources` (junction), but not `bullets` itself. The original bullet content (`b.content`) is needed for `bullet_preview`. All three builders must add:

```sql
JOIN bullets b ON b.id = p.bullet_id
```

And SELECT `b.content AS bullet_content`. The `ExperienceEntryRow` interface in `resume-compiler.ts` must be extended with `bullet_content: string`. The anonymous row types in `buildProjectsSection` and `buildPresentationsSection` must also include this field.

**Updated source_chain population:**

```typescript
// Current:
source_chain: {
  perspective_id: e.perspective_id,
  bullet_id: e.bullet_id,
  source_id: e.source_id,
}

// CORRECT — bullet uses original content, perspective uses reframing:
source_chain: {
  source_id: e.source_id,
  source_title: truncate(e.source_title, 60),            // truncated — no guaranteed length limit
  bullet_id: e.bullet_id,
  bullet_preview: truncate(e.bullet_content, 60),        // original bullet text from bullets table
  perspective_id: e.perspective_id,
  perspective_preview: truncate(e.perspective_content, 60),  // perspective reframing
}
```

Note: `bullet_preview` and `perspective_preview` will often differ — `bullet_content` is the original bullet text, while `perspective_content` is the perspective's reframing of that bullet for a specific audience/lens.

### 1.3 Other IR Items with source_chain

The same `source_chain` pattern exists on:
- `ExperienceBullet` (experience section)
- `ProjectItem.bullets` (projects section)
- `PresentationItem.bullets` (presentations section)

All three should be enriched the same way. The compiler already populates `source_chain` for these — just add the preview fields.

---

## 2. Tooltip UI Enhancement

### 2.1 DragNDrop View Tooltip

Replace the current tooltip in `DragNDropView.svelte`:

**Current:**
```
Source: 550e8400-e29b-41d4-a716-446655440000
Bullet: 661f9511-f30c-42e5-b827-557766554411
Perspective: 772g0622-g41d-53f6-c938-668877665522
Content has been manually edited
```

**New:**
```
┌─ Provenance Chain ─────────────────────────────┐
│ Source: Principal Cloud Forensics Engineer    → │
│ Bullet: Led 4-engineer team migrating clo... → │
│ Perspective: Led cloud platform migration... → │
│ ⚠ Content has been manually edited             │
└────────────────────────────────────────────────┘
```

Where `→` is a clickable link that navigates to `/chain?highlight={nodeType}-{id}`.

### 2.2 Link Format

Each tooltip row links to the Chain View with a query param that highlights the specific node:

- Source link: `/chain?highlight=source-{source_id}`
- Bullet link: `/chain?highlight=bullet-{bullet_id}`
- Perspective link: `/chain?highlight=perspective-{perspective_id}`

The Chain View should read the `highlight` query param and:
1. Center the graph on that node
2. Highlight it visually (pulsing border, larger size, or color change)
3. Open its detail panel

### 2.3 Tooltip Styling

- Background: `#1a1a2e` (dark, matches existing)
- Text: `#e0e0e0`
- Links: `#6c63ff` (accent color), underline on hover
- Edited badge: amber `#f59e0b`
- Max width: 350px
- All previews are truncated to 60 characters (with `...` appended if truncated, making max display length 63) at the IR level. The tooltip uses CSS `text-overflow: ellipsis` with `max-width: 350px` for additional visual truncation if needed.

### 2.4 Tooltip Interaction Model

The current tooltip has `pointer-events: none` (CSS). This must change to `pointer-events: auto` to allow link clicks. However, this creates a problem: moving the cursor from the bullet-item into the tooltip triggers `mouseleave` on the bullet-item, which hides the tooltip before the user can click.

**Solution: delayed dismissal with tooltip hover awareness.**

```typescript
let tooltipHovered = $state(false)
let dismissTimer: ReturnType<typeof setTimeout> | null = null

function showTooltip(bullet: ExperienceBullet, e: MouseEvent) {
  if (!bullet.source_chain) return
  if (dismissTimer) { clearTimeout(dismissTimer); dismissTimer = null }
  tooltipEntry = bullet
  tooltipPosition = { x: e.clientX + 10, y: e.clientY + 10 }
}

function scheduleHideTooltip() {
  dismissTimer = setTimeout(() => {
    if (!tooltipHovered) {
      tooltipEntry = null
    }
  }, 150)  // 150ms grace period to reach the tooltip
}
```

Template:
```svelte
<!-- On the bullet-item: -->
onmouseleave={scheduleHideTooltip}

<!-- On the tooltip itself: -->
<div class="provenance-tooltip"
  onmouseenter={() => { tooltipHovered = true; if (dismissTimer) clearTimeout(dismissTimer) }}
  onmouseleave={() => { tooltipHovered = false; tooltipEntry = null }}
>
```

CSS change: `.provenance-tooltip { pointer-events: auto; }` (was `none`)

### 2.5 Tooltip Handlers for Project and Presentation Bullets

The DragNDropView currently only has tooltip handlers (`onmouseenter`/`onmouseleave`) on experience bullets (lines 155-156). Project bullets (lines 254-259) and presentation bullets (lines 278-283) have NO tooltip handlers. These must be added for the enriched `source_chain` to be visible on those bullets. The same `showTooltip`/`scheduleHideTooltip` logic applies.

---

## 3. Chain View Highlight Support

### 3.1 Query Param

The Chain View at `/chain` should accept a `?highlight=source-{id}` query param.

The `highlight` query param value (e.g., `source-550e8400-e29b-41d4-a716-446655440000`) is already the graph node key as-is. No splitting is needed for graph lookup. Do NOT use `split('-', 2)` — UUIDs contain hyphens and will be truncated.

On mount (or when the param changes):
1. Use the highlight value directly as the node key: `graph.hasNode(highlight)`
2. For the detail panel, extract the node type: `const nodeType = highlight.slice(0, highlight.indexOf('-'))` — yields `'source'`, `'bullet'`, or `'perspective'`
3. If found:
   - Center the camera on that node (Sigma's `camera.animate`)
   - Increase the node's size temporarily (2x)
   - Set a highlight color (pulsing or bright border)
   - Open the detail panel for that node
4. If not found: ignore silently (node may not be in the current graph filter)

### 3.2 Implementation

In `packages/webui/src/routes/chain/+page.svelte`:

```typescript
// After graph is rendered:
const highlight = new URLSearchParams(window.location.search).get('highlight')
if (highlight && renderer) {
  if (graph.hasNode(highlight)) {
    // Center camera on node using Sigma's node display data
    // Sigma's getNodeDisplayData returns coordinates in the camera's coordinate system,
    // which is what camera.animate expects. Do NOT use raw graph attributes (attrs.x, attrs.y)
    // — those are in graph-space and will produce wrong camera positions.
    const nodeDisplayData = renderer.getNodeDisplayData(highlight)
    if (nodeDisplayData) {
      renderer.getCamera().animate(
        { x: nodeDisplayData.x, y: nodeDisplayData.y, ratio: 0.3 },
        { duration: 500 }
      )
    }
    // Highlight
    const attrs = graph.getNodeAttributes(highlight)
    graph.setNodeAttribute(highlight, 'size', attrs.size * 2)
    graph.setNodeAttribute(highlight, 'highlighted', true)
    // Open detail panel
    selectedNode = highlight
  }
}
```

---

## 4. Acceptance Criteria

### IR
- [ ] `source_chain` includes `source_title`, `bullet_preview`, `perspective_preview`
- [ ] Previews truncated to 60 characters
- [ ] `GET /api/resumes/:id/ir` returns enriched source_chain
- [ ] All three source_chain locations enriched (experience, projects, presentations)

### Tooltip
- [ ] Tooltip shows source title instead of UUID
- [ ] Tooltip shows bullet content preview instead of UUID
- [ ] Tooltip shows perspective content preview instead of UUID
- [ ] Each row is a clickable link to `/chain?highlight=...`
- [ ] Links use accent color, underline on hover
- [ ] Edited indicator still shown when applicable
- [ ] Tooltip max width 350px, previews truncated to 60 chars at IR level
- [ ] Project and presentation bullets show tooltip with enriched source_chain
- [ ] `bullet_preview` shows original bullet content, `perspective_preview` shows reframed content (they differ)
- [ ] `source_title` truncated to 60 chars in IR
- [ ] Tooltip remains visible long enough to click links (hover-delay logic)
- [ ] `.provenance-tooltip` has `pointer-events: auto`

### Chain View
- [ ] Accepts `?highlight=source-{id}` query param
- [ ] Camera centers on highlighted node
- [ ] Highlighted node has increased size
- [ ] Detail panel opens for highlighted node
- [ ] Graceful handling when highlight param references non-existent node

### Tests
- [ ] IR compiler test: enriched source_chain has title and previews
- [ ] IR compiler test: compile resume where bullet content differs from perspective content, assert `bullet_preview != perspective_preview`
- [ ] IR compiler test: assert `source_title` is truncated to 60 chars for long titles
- [ ] E2E: hover tooltip shows source title, not UUID
- [ ] E2E: hover over project bullet, assert tooltip appears with source_chain data
- [ ] Chain View: highlight query param centers and selects node
- [ ] Chain View: navigate to `/chain?highlight=bullet-{uuid-with-hyphens}`, assert correct node is highlighted (regression for UUID hyphen parsing)

---

## 5. Dependencies & Parallelization

### Sequential
1. IR type update (SDK + core) — must be first
2. IR compiler change — depends on types
3. Tooltip UI update — depends on enriched IR data

### Parallel (after IR changes)
- Tooltip UI update (DragNDropView)
- Chain View highlight support

### Files to Modify
- `packages/sdk/src/types.ts` — update `source_chain` fields
- `packages/core/src/types/index.ts` — same
- `packages/core/src/services/resume-compiler.ts` — enrich source_chain in experience/project/presentation builders; add `JOIN bullets b ON b.id = p.bullet_id` to all three builder queries; extend `ExperienceEntryRow` and anonymous row types with `bullet_content: string`
- `packages/webui/src/lib/components/resume/DragNDropView.svelte` — tooltip template + links; add `onmouseenter`/`onmouseleave` tooltip handlers to project bullets (lines 254-259) and presentation bullets (lines 278-283); change `.provenance-tooltip` to `pointer-events: auto`; add hover-delay dismissal logic
- `packages/webui/src/routes/chain/+page.svelte` — highlight query param support; use `renderer.getNodeDisplayData()` for camera positioning

### No New Dependencies
Uses existing Sigma.js camera API and SvelteKit navigation.

---

## 6. Known Limitations

- **Mobile/touch:** Hover tooltips don't work on touch devices. The resume builder is primarily a desktop tool. Touch support is a future enhancement.
- **Tooltip boundary clamping:** Tooltips near viewport edges may overflow. A future enhancement should clamp position to viewport bounds.
