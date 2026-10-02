# Phase 52: Node Labels & Display (Spec H3)

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-graph-node-labels.md](../refs/specs/2026-04-03-graph-node-labels.md)
**Depends on:** Phase 48 (Generic GraphView)
**Blocks:** Phase 54 (Graph Search -- search uses slug matching)
**Parallelizable with:** Phase 51, Phase 53, Phase 56 -- creates `graph.labels.ts` (new file), modifies `GraphView.svelte` in distinct sections

## Goal

Establish a label display strategy for dense graphs: labels are hidden by default, revealed on hover and selection, and use auto-generated shortname slugs (`src:raytheon-pcfe`, `blt:built-ai`) for compact identification. Slugs are computed during graph construction and stored as graphology node attributes for downstream consumers (H5 search). Selected nodes and their immediate neighbors show labels; all others remain hidden.

> **Spec correction:** The spec example `'blt:ai-taxonomy'` is incorrect per the slug algorithm (stop words are not filtered before the type prefix, and the first two significant words of "Built an AI taxonomy pipeline" are "built" and "ai"). The plan's `'blt:built-ai'` is correct.

## Non-Goals

- Label editing UI (handled by entity edit forms)
- Rich label rendering (HTML-in-canvas, multi-line, images)
- Label collision avoidance (hide-by-default strategy avoids the problem)
- Per-node label styling (color differentiation is via node color, not label color)

## Context

Phase 48's `GraphView` renders labels on every node via Sigma's built-in label renderer, which becomes unreadable beyond ~20 nodes. The `nodeReducer` already handles selection highlighting and label hiding for dimmed nodes. This phase adds slug generation, stores slugs as graphology attributes, configures a custom `labelRenderer` that only draws labels for hovered/selected nodes, and extends the `nodeReducer` to show labels on neighbors of the selected node.

The slug system provides the `slug` field that Phase 54 (Graph Search) needs for autocomplete matching.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Slug Generation (algorithm, slug as label, user-overridable displayName) | Yes |
| 2. Label Visibility (hidden by default, show on hover, show on selection, neighbor labels) | Yes |
| 3. Component Interface (no GraphNode changes, new exports) | Yes |
| 4. Testing | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/graph.labels.ts` | `generateSlug()`, `slugifyName()`, `TYPE_PREFIXES` |
| `packages/webui/src/lib/components/graph/__tests__/graph-labels.test.ts` | Unit tests for slug generation (15 cases) |

## Files to Modify

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/graph/GraphView.svelte` | Use `generateSlug` in node construction; set `forceLabel` in `nodeReducer` for selected + neighbor nodes; configure custom `labelRenderer` in Sigma settings |

## Fallback Strategies

- **Unknown entity type for slug prefix:** Falls back to first 3 characters of the type string (`TYPE_PREFIXES[type] ?? type.slice(0, 3)`). No crash.
- **All-stop-word name producing empty identifier:** `generateSlug('source', 'the a of')` returns `'src:'`. Consumers must handle bare-colon slugs gracefully. The slug is still unique enough for search matching.
- **Sigma v3 `forceLabel` not honored by custom `labelRenderer`:** The custom renderer explicitly checks `data.forceLabel` -- it does not rely on Sigma's built-in behavior. If `forceLabel` is not in the data, the check simply fails and no label renders (safe default).
- **`drawLabel` function not importable from Sigma:** The custom `labelRenderer` calls Sigma's built-in `drawLabel` function. If the import path changes across Sigma versions, fall back to inline canvas text rendering (`context.fillText`).

---

## Tasks

### T52.1: Write Slug Generation Module

**File:** `packages/webui/src/lib/components/graph/graph.labels.ts`

[IMPORTANT] `slugifyName` strips special characters, filters stop words, and truncates at word boundaries. The `maxWords` parameter defaults to 2 for compact slugs.

[MINOR] The stop word list is intentionally small (12 common English words). Domain-specific words are not filtered.

```typescript
/**
 * Type prefix mapping for slug generation.
 * Short codes for entity types used in graph node identification.
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
 * Convert a name/title to a compact slug identifier.
 * Takes the first N significant words (default 2), lowercases, hyphenates.
 * Strips common stop words (the, a, an, of, for, and, to, in, with, on, at, by).
 *
 * Examples:
 *   slugifyName('Raytheon PCFE Migration Project')   -> 'raytheon-pcfe'
 *   slugifyName('Built an AI taxonomy pipeline')     -> 'built-ai'
 *   slugifyName('Kubernetes')                        -> 'kubernetes'
 *   slugifyName('C++ & Python (Advanced)')           -> 'c-python'
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

/**
 * Generate a shortname slug for a graph node.
 * Format: {type_prefix}:{identifier}
 *
 * Examples:
 *   generateSlug('source', 'Raytheon PCFE Migration')     -> 'src:raytheon-pcfe'
 *   generateSlug('bullet', 'Built AI taxonomy pipeline')   -> 'blt:built-ai'
 *   generateSlug('perspective', 'DevSecOps Security Lead') -> 'psp:devsecops-security'
 *   generateSlug('organization', 'Booz Allen Hamilton')    -> 'org:booz-allen'
 *   generateSlug('skill', 'Kubernetes')                    -> 'skl:kubernetes'
 */
export function generateSlug(type: string, name: string): string {
  const prefix = TYPE_PREFIXES[type] ?? type.slice(0, 3)
  const identifier = slugifyName(name)
  return `${prefix}:${identifier}`
}
```

**Acceptance criteria:**
- `generateSlug('source', 'Raytheon PCFE Migration')` returns `'src:raytheon-pcfe'`.
- `slugifyName` strips stop words, special characters, and truncates at 20 chars.
- Unknown types fall back to first 3 characters.
- Empty input returns empty identifier.
- All-stop-word input returns empty identifier.

**Failure criteria:**
- Stop words appear in slugs.
- Slug identifier exceeds 20 characters.
- Special characters appear in slugs.

---

### T52.2: Integrate Slug Generation into Graph Construction

**File:** `packages/webui/src/lib/components/graph/GraphView.svelte`

[CRITICAL] Slugs are computed during graph construction and stored as graphology node attributes. Phase 54 (search) reads slugs from `graph.getNodeAttributes(nodeId).slug`, NOT from the raw `GraphNode[]` props. If slugs are not stored on graphology nodes, search will not work.

[IMPORTANT] The `displayName` property is read from the `GraphNode` index signature. If present, it overrides the auto-generated slug as the rendered label. The original `label` is preserved as `fullLabel` for tooltip display.

Modify the node-adding section of the graph-building `$effect`:

```typescript
// --- In GraphView.svelte, add import ---
import { generateSlug } from './graph.labels'

// --- In the buildAndAssign() function, replace the node loop ---
for (const node of _nodes) {
  const slug = generateSlug(node.type, node.label)
  g.addNode(node.id, {
    ...node,
    label: (node as any).displayName ?? slug,  // rendered label
    fullLabel: node.label,                      // original for tooltips
    slug,                                       // for search matching (H5)
    x: node.x ?? Math.random() * 100,
    y: node.y ?? Math.random() * 100,
    size: node.size ?? _config.nodeDefaults.size,
    color: node.color
      ?? _config.colorMap?.[node.type]
      ?? _config.nodeDefaults.color,
    nodeType: node.type,
    type: 'circle',
  })
}
```

**Acceptance criteria:**
- Each graphology node has `slug`, `fullLabel`, and `label` attributes.
- `label` is `displayName` when present, otherwise the auto-generated slug.
- `fullLabel` preserves the original `GraphNode.label` value.
- `slug` is always the auto-generated value (not overridden by `displayName`).

**Failure criteria:**
- Slugs not stored on graphology nodes (breaks Phase 54 search).
- `displayName` override not applied to rendered label.

---

### T52.3: Configure Custom Label Renderer and Extend `nodeReducer`

**File:** `packages/webui/src/lib/components/graph/GraphView.svelte`

[IMPORTANT] Sigma v3's `forceLabel: true` returned from `nodeReducer` is passed through to the custom `labelRenderer` as `data.forceLabel`. The custom renderer must check this flag explicitly.

[IMPORTANT] The `drawLabel` function is imported from `sigma/rendering`. If the import path differs in the installed version, fall back to `context.fillText()`.

[MINOR] Neighbor label display uses `graph.hasEdge()` in both directions to handle both directed and undirected edges.

Extend the Sigma initialization block:

```typescript
// --- In initSigma(), add to the Sigma constructor options ---
import { drawLabel } from 'sigma/rendering'

// Add labelRenderer to the Sigma settings object:
labelRenderer: (
  context: CanvasRenderingContext2D,
  data: Record<string, any>,
  settings: Record<string, any>
) => {
  // Only render label for hovered or selected nodes
  if (data.highlighted || data.forceLabel) {
    drawLabel(context, data, settings)
  }
},

// --- Replace the existing nodeReducer with ---
nodeReducer: (node: string, data: Record<string, any>) => {
  // Selection highlighting takes priority
  if (selectedNodeId) {
    if (node === selectedNodeId) {
      return { ...data, forceLabel: true, zIndex: Z_FOREGROUND }
    }
    if (
      graph!.hasEdge(selectedNodeId, node)
      || graph!.hasEdge(node, selectedNodeId)
    ) {
      // Neighbor of selected node: show label
      return { ...data, forceLabel: true, zIndex: Z_FOREGROUND }
    }
    return { ...data, color: DIM_NODE_COLOR, label: '', zIndex: Z_BACKGROUND }
  }

  // Edge hover: highlight endpoint nodes (from H2/Phase 51)
  if (hoveredEdge && graph) {
    try {
      const [src, tgt] = graph.extremities(hoveredEdge)
      if (node === src || node === tgt) {
        return { ...data, zIndex: Z_FOREGROUND }
      }
      return { ...data, color: DIM_NODE_COLOR, label: '', zIndex: Z_BACKGROUND }
    } catch {
      return data
    }
  }

  return data
},
```

**Acceptance criteria:**
- Labels are hidden by default (custom `labelRenderer` only draws for `highlighted` or `forceLabel`).
- Hovering a node shows its label (Sigma sets `data.highlighted`).
- Selected node gets `forceLabel: true` (label persists until deselected).
- Neighbors of selected node get `forceLabel: true` (contextual labels).
- Non-neighbor nodes during selection get `label: ''` (hidden).
- Clicking stage clears selection and hides all labels.

**Failure criteria:**
- Labels render on all nodes despite custom `labelRenderer`.
- Neighbor labels do not appear on selection.
- `drawLabel` import fails (Sigma version mismatch).

---

### T52.4: Write Slug Generation Unit Tests

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

  it('handles multiple spaces and tabs', () => {
    expect(slugifyName('  word1   word2  ')).toBe('word1-word2')
  })

  it('handles hyphenated input and truncates at word boundary', () => {
    // 'cloud-native-infrastructure' is 27 chars, truncated at 20 = 'cloud-native-infrastr',
    // lastIndexOf('-') > 5 gives 'cloud-native'
    expect(slugifyName('cloud-native infrastructure')).toBe('cloud-native')
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
    expect(generateSlug('source', 'the a of')).toBe('src:')
  })

  it('handles empty name', () => {
    expect(generateSlug('bullet', '')).toBe('blt:')
  })
})

describe('TYPE_PREFIXES', () => {
  it('contains all expected entity types', () => {
    expect(TYPE_PREFIXES).toHaveProperty('source', 'src')
    expect(TYPE_PREFIXES).toHaveProperty('bullet', 'blt')
    expect(TYPE_PREFIXES).toHaveProperty('perspective', 'psp')
    expect(TYPE_PREFIXES).toHaveProperty('resume_entry', 'ent')
    expect(TYPE_PREFIXES).toHaveProperty('organization', 'org')
    expect(TYPE_PREFIXES).toHaveProperty('skill', 'skl')
    expect(TYPE_PREFIXES).toHaveProperty('job_description', 'jd')
    expect(TYPE_PREFIXES).toHaveProperty('resume', 'rsm')
  })
})
```

**Acceptance criteria:**
- All 16 test cases pass.
- Stop word filtering, special character stripping, truncation, and prefix mapping are verified.
- Edge cases (empty, all-stop-words, unknown types) are covered.

**Failure criteria:**
- Any test fails, indicating a slug generation bug.

---

## Testing Support

| Test file | Test count | Type |
|-----------|-----------|------|
| `__tests__/graph-labels.test.ts` | 16 | Unit |
| **Total** | **16** | |

**Run command:** `cd packages/webui && npx vitest run src/lib/components/graph/__tests__/graph-labels.test.ts`

## Documentation Requirements

- Export `generateSlug`, `slugifyName`, and `TYPE_PREFIXES` from `graph.labels.ts`.
- No new user-facing docs (internal component module).

## Parallelization Notes

- T52.1 (new file `graph.labels.ts`) is independent -- can start immediately.
- T52.2 and T52.3 both modify `GraphView.svelte` -- must run sequentially after T52.1.
- T52.4 (tests) depends on T52.1 only.
- This phase is parallelizable with Phase 51 and 53. Phase 54 depends on this phase (needs slugs).
- If Phase 51 (edge rendering) is implemented concurrently, the `nodeReducer` changes in T52.3 must be merged carefully with T51.4's edge-hover endpoint highlighting (both modify the same reducer). The T52.3 code above already includes the edge-hover logic from T51.4.
