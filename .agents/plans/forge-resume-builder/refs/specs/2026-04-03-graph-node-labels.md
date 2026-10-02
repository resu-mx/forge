# Node Labels & Display

**Date:** 2026-04-03
**Spec:** H3 (Node Labels & Display)
**Phase:** TBD (next available)
**Depends on:** H1 (Generic GraphView Component)

## Overview

Dense graphs with overlapping text labels are unreadable. The current chain view renders labels on every node regardless of density, which breaks down once the graph exceeds ~20 nodes. This spec establishes a label display strategy: labels are hidden by default, revealed on hover (tooltip-style) and on node selection, and use auto-generated shortname slugs for compact identification.

The slug system gives every entity a human-scannable identifier (e.g., `src:raytheon-pcfe`, `blt:ai-taxonomy`, `psp:devsecops-sec`) that works both as a graph label and as a search target (consumed by H5 Graph Search). Users can optionally override the display name via a stored field on the entity.

## Non-Goals

- **Label editing UI:** This spec covers label display only. Editing display names is handled by the entity's edit form (existing CRUD UI).
- **Rich label rendering:** No HTML-in-canvas, no multi-line labels, no images in labels. Plain text only.
- **Label collision avoidance:** No automatic label repositioning to prevent overlap. The hide-by-default strategy avoids the problem entirely.
- **Per-node label styling:** All labels use the same font/size. Color differentiation is handled by node color, not label color.

---

## 1. Slug Generation

### 1.1 Algorithm

Slugs follow the pattern `{type_prefix}:{identifier}` where:

- **type_prefix** is a short code for the entity type (3 characters)
- **identifier** is derived from the entity's name or content (lowercase, hyphenated, max 20 chars)

```typescript
/**
 * Type prefix mapping for slug generation.
 */
export const TYPE_PREFIXES: Record<string, string> = {
  source: 'src',
  bullet: 'blt',
  perspective: 'psp',
  resume_entry: 'ent',
  organization: 'org',
  skill: 'skl',
  job_description: 'jd',
  resume: 'rsm',
}

/**
 * Generate a shortname slug for a graph node.
 *
 * Examples:
 *   generateSlug('source', 'Raytheon PCFE Migration')     -> 'src:raytheon-pcfe'
 *   generateSlug('bullet', 'Built AI taxonomy pipeline')   -> 'blt:ai-taxonomy'
 *   generateSlug('perspective', 'DevSecOps Security Lead') -> 'psp:devsecops-sec'
 *   generateSlug('organization', 'Booz Allen Hamilton')    -> 'org:booz-allen'
 *   generateSlug('skill', 'Kubernetes')                    -> 'skl:kubernetes'
 */
export function generateSlug(type: string, name: string): string {
  const prefix = TYPE_PREFIXES[type] ?? type.slice(0, 3)
  const identifier = slugifyName(name)
  return `${prefix}:${identifier}`
}

/**
 * Convert a name/title to a compact slug identifier.
 * Takes the first 2-3 significant words, lowercases, hyphenates.
 * Strips common stop words (the, a, an, of, for, and, to, in, with).
 */
export function slugifyName(name: string, maxWords = 2): string {
  const STOP_WORDS = new Set([
    'the', 'a', 'an', 'of', 'for', 'and', 'to', 'in', 'with', 'on', 'at', 'by',
  ])

  const words = name
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .split(/\s+/)
    .filter(w => w.length > 0 && !STOP_WORDS.has(w))

  const selected = words.slice(0, maxWords)
  const slug = selected.join('-')

  // Truncate to 20 chars, trimming at word boundary if possible
  if (slug.length <= 20) return slug
  const truncated = slug.slice(0, 20)
  const lastHyphen = truncated.lastIndexOf('-')
  return lastHyphen > 5 ? truncated.slice(0, lastHyphen) : truncated
}
```

### 1.2 Slug as Label

During graph construction (H1's node-building loop), the `label` attribute on each graphology node is set to:

1. The user's custom `displayName` if set on the entity (stored in the database, optional field).
2. Otherwise, the auto-generated slug from `generateSlug(node.type, node.label)`.

The original full `label` from the `GraphNode` prop is preserved as a separate attribute (`fullLabel`) for tooltip display.

```typescript
// In GraphView.svelte graph construction $effect:
for (const node of nodes) {
  const slug = generateSlug(node.type, node.label)
  g.addNode(node.id, {
    label: node.displayName ?? slug,
    fullLabel: node.label,  // preserved for tooltips
    slug,                   // preserved for search matching (H5)
    // ... rest of node attributes
  })
}
```

> **Slug storage for downstream consumers:** Slugs are computed during graph construction in `GraphView.svelte` and stored as graphology node attributes. For H5 (search) to access slugs, `buildSearchIndex` must read from the graphology `Graph` object (via `graph.getNodeAttributes(nodeId).slug`), NOT from the raw `GraphNode[]` props -- slugs do not exist on the original props.

### 1.3 User-Overridable Display Name

The `GraphNode` interface (H1) already has an index signature `[key: string]: unknown`, so consumers can pass `displayName` as custom metadata without changing the type definition. No type changes needed.

---

## 2. Label Visibility

### 2.1 Default: Hidden

Labels are hidden by default. H1's `labelThreshold` config (default: `6`) controls the zoom level at which labels appear. For this spec, the default is raised to a very high threshold so labels are effectively always hidden unless the user zooms in significantly:

```typescript
// Recommended consumer config for dense graphs:
const config: Partial<GraphConfig> = {
  labelThreshold: 1e6, // effectively never show labels by zoom alone
  // NOTE: Avoid `Infinity` — some Sigma internals may not handle it
  // gracefully. Use `Number.MAX_SAFE_INTEGER` or `1e6` instead.
}
```

For sparse graphs (<20 nodes), consumers can use a normal threshold (e.g., `6`) to show labels when zoomed in.

### 2.2 Show on Hover

When a node is hovered, its label appears as a tooltip-style overlay above the node. This uses Sigma.js's `labelRenderer` override:

```typescript
// Custom label renderer: only render label for hovered or selected nodes
labelRenderer: (
  context: CanvasRenderingContext2D,
  data: PartialButFor<NodeDisplayData, 'x' | 'y' | 'size' | 'label' | 'color'>,
  settings: Settings
) => {
  // Only render if this node is hovered or selected
  if (data.highlighted || data.forceLabel) {
    drawLabel(context, data, settings)
  }
}
```

The `data.highlighted` flag is set by Sigma when the node is hovered. The `data.forceLabel` flag is set by the `nodeReducer` for the selected node.

> **Sigma v3 `forceLabel` behavior:** In Sigma v3, `forceLabel: true` can be returned from `nodeReducer` and Sigma honors it for the built-in label renderer. When using a custom `labelRenderer`, the custom function receives `data.forceLabel` from the reducer output -- the custom renderer must check this flag explicitly (as shown above).

### 2.3 Show on Selection

When a node is clicked (selected), its label persists until deselected. The `nodeReducer` sets `forceLabel: true` on the selected node:

```typescript
// In nodeReducer (extending H1):
if (node === selectedNodeId) {
  return { ...data, forceLabel: true, zIndex: Z_FOREGROUND }
}
```

### 2.4 Neighbor Labels on Selection

When a node is selected, its immediate neighbors also show their labels (dimmed neighbors do not). This helps the user understand the local context:

```typescript
// In nodeReducer:
if (selectedNodeId) {
  if (node === selectedNodeId) {
    return { ...data, forceLabel: true, zIndex: Z_FOREGROUND }
  }
  if (graph.hasEdge(selectedNodeId, node) || graph.hasEdge(node, selectedNodeId)) {
    return { ...data, forceLabel: true, zIndex: Z_FOREGROUND }
  }
  return { ...data, color: DIM_NODE_COLOR, label: '', zIndex: Z_BACKGROUND }
}
```

---

## 3. Component Interface

### 3.1 GraphNode Extension

No changes to the `GraphNode` interface. Consumers pass `displayName` via the existing index signature:

```typescript
const node: GraphNode = {
  id: 'src-1',
  label: 'Raytheon PCFE Migration Project',  // full name
  type: 'source',
  displayName: 'PCFE Migration',             // optional override
}
```

### 3.2 New Exports

```typescript
// graph.labels.ts
export const TYPE_PREFIXES: Record<string, string>
export function generateSlug(type: string, name: string): string
export function slugifyName(name: string, maxWords?: number): string
```

---

## 4. Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.labels.ts` | Slug generation (`generateSlug`, `slugifyName`, `TYPE_PREFIXES`) |

## 5. Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Use `generateSlug` in graph construction; set `forceLabel` in `nodeReducer` for selected/neighbor nodes; configure custom `labelRenderer` in Sigma settings |

---

## 6. Testing Approach

### 6.1 Unit Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-labels.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { generateSlug, slugifyName, TYPE_PREFIXES } from '../graph.labels'

describe('slugifyName', () => {
  it('takes first two significant words', () => {
    expect(slugifyName('Raytheon PCFE Migration Project')).toBe('raytheon-pcfe')
  })

  it('strips stop words', () => {
    expect(slugifyName('Built an AI taxonomy pipeline')).toBe('built-ai')
  })

  it('handles single word', () => {
    expect(slugifyName('Kubernetes')).toBe('kubernetes')
  })

  it('truncates long slugs at word boundary', () => {
    expect(slugifyName('Superlongcompanyname Anotherlongword', 2).length).toBeLessThanOrEqual(20)
  })

  it('strips special characters', () => {
    expect(slugifyName('C++ & Python (Advanced)')).toBe('c-python')
  })

  it('handles empty string', () => {
    expect(slugifyName('')).toBe('')
  })

  it('returns empty string when all words are stop words', () => {
    expect(slugifyName('the a of')).toBe('')
  })

  it('respects maxWords parameter', () => {
    expect(slugifyName('one two three four', 3)).toBe('one-two-three')
  })
})

describe('generateSlug', () => {
  it('uses known type prefix', () => {
    expect(generateSlug('source', 'Raytheon PCFE')).toBe('src:raytheon-pcfe')
  })

  it('uses first 3 chars for unknown type', () => {
    expect(generateSlug('widget', 'My Widget')).toBe('wid:my-widget')
  })

  it('maps all known types', () => {
    for (const [type, prefix] of Object.entries(TYPE_PREFIXES)) {
      const slug = generateSlug(type, 'Test Name')
      expect(slug.startsWith(`${prefix}:`)).toBe(true)
    }
  })

  it('handles all-stop-word names producing bare-colon slug', () => {
    // `slugifyName('the a of')` returns '' so `generateSlug` returns 'src:'
    // Consumers should handle bare-colon slugs gracefully.
    expect(generateSlug('source', 'the a of')).toBe('src:')
  })
})
```

### 6.2 Reducer Logic Tests

**File:** `packages/webui/src/lib/components/graph/__tests__/graph-label-display.test.ts`

Test the nodeReducer label logic in isolation:

- Selected node gets `forceLabel: true`.
- Neighbor of selected node gets `forceLabel: true`.
- Non-neighbor gets `label: ''` (hidden).
- With no selection, no `forceLabel` is set.

---

## 7. Acceptance Criteria

### Slug generation
- [ ] `generateSlug('source', 'Raytheon PCFE Migration')` returns `'src:raytheon-pcfe'`
- [ ] `generateSlug('bullet', 'Built AI taxonomy pipeline')` returns `'blt:built-ai'`
- [ ] `generateSlug('perspective', 'DevSecOps Security Lead')` returns `'psp:devsecops-security'`
- [ ] Stop words (the, a, of, for, etc.) are excluded from slug
- [ ] Slugs are capped at 20 characters for the identifier portion
- [ ] Unknown types use first 3 characters as prefix

### Label visibility
- [ ] Labels are hidden by default on all nodes (no visible text at normal zoom)
- [ ] Hovering a node shows its label (slug or display name)
- [ ] Clicking a node shows its label persistently until deselected
- [ ] Selecting a node also shows labels on its immediate neighbors
- [ ] Clicking the background (stage) clears selection and hides all labels

### Display name override
- [ ] Nodes with a `displayName` property use it as the rendered label
- [ ] Nodes without `displayName` use the auto-generated slug
- [ ] The full original `label` is preserved as `fullLabel` for tooltip access

### Tests
- [ ] `slugifyName` unit tests pass (7 cases)
- [ ] `generateSlug` unit tests pass (3 cases)
- [ ] Reducer label logic tests pass (4 cases)

---

## 8. Dependencies

- **Runtime:** None beyond H1's dependencies
- **Spec dependencies:** H1 (Generic GraphView Component)
- **Blocked by:** Nothing beyond H1
- **Blocks:** H5 (Graph Search) — search uses slug matching for autocomplete
