# Phase 22: Component Debug Helpers

**Goal:** Create Svelte 5 debug utilities for tracing component state and effect execution, enabling rapid diagnosis of rendering issues by logging every state transition to the browser console.

**Non-Goals:** Adding debug helpers to all components by default. Building a devtools panel UI. Persisting debug logs. Server-side logging (Phase 23). Production-visible logging.

**Depends on:** Phase 21 (imports `isDevMode` from `@forge/sdk`)
**Blocks:** Nothing
**Parallelizable with:** Phase 23

**Internal task parallelization:** T22.1 must complete before T22.2. T22.3 depends on T22.2.

**Tech Stack:** TypeScript, Svelte 5 runes (`$effect`), SvelteKit

**Reference:** `refs/specs/2026-03-30-observability-structured-logging.md` section 2

**Architecture:**
- Debug utilities live in `packages/webui/src/lib/debug.svelte.ts` (must be `.svelte.ts` for Svelte 5 runes)
- Imports `isDevMode` from `@forge/sdk` (added in Phase 21)
- Used by any Svelte component via `import { debugState } from '$lib/debug.svelte'`

**Fallback Strategies:**
- If `isDevMode()` is not available (Phase 21 not yet deployed), temporarily inline a local `const DEV = true` and swap once Phase 21 lands
- If `$effect` causes unexpected re-renders in debug helpers, add a `$effect.pre` variant that runs before DOM updates

---

## Context

The resumes page (`packages/webui/src/routes/resumes/+page.svelte`) has a known infinite loading bug. Phase 21 adds SDK-level request logging, but the bug may be in the component's state management -- e.g., a reactive dependency cycle causing infinite re-fetches, or a state flag that never transitions from `loading: true` to `loading: false`.

The component has several `$state` variables that interact:
- `loading`, `detailLoading` -- loading flags
- `selectedResumeId` -- currently selected resume
- `resumeDetail` -- fetched detail for selected resume
- `gapAnalysis` -- gap analysis data

Without visibility into when these states change and in what order, debugging requires adding temporary `console.log` calls and removing them later. The `debugState()` helper makes this a one-liner that auto-disables in production.

---

## Tasks

### Task 22.1: Create debug.svelte.ts

**File to create:** `packages/webui/src/lib/debug.svelte.ts`

**Goal:** Implement `debugState` and `tracedEffect` utilities that log component state transitions using Svelte 5 runes.

#### Steps

- [ ] **Create `packages/webui/src/lib/debug.svelte.ts`:**

```typescript
/**
 * Svelte 5 debug utilities for component state tracing.
 *
 * These helpers log state changes and effect executions to console.debug,
 * which is hidden by default in Chrome DevTools (enable 'Verbose' level).
 * All helpers are no-ops when isDevMode() returns false.
 *
 * Must be a .svelte.ts file because it uses $effect (a Svelte 5 rune that
 * requires the Svelte compiler).
 */
import { isDevMode } from '@forge/sdk'

/**
 * Log a snapshot of component state every time any of the accessed reactive
 * values change.
 *
 * @param label - Prefix for the console.debug output, e.g. 'resumes'
 * @param stateGetter - A function that returns an object of state values to log.
 *                      Svelte's reactivity system tracks which $state vars this
 *                      function reads, so the effect re-runs whenever they change.
 *
 * @example
 * ```svelte
 * <script>
 *   import { debugState } from '$lib/debug.svelte'
 *   let loading = $state(true)
 *   let selectedId = $state<string | null>(null)
 *   debugState('resumes', () => ({ loading, selectedId }))
 * </script>
 * ```
 *
 * Output:
 * ```
 * [forge:resumes] { loading: true, selectedId: null }
 * [forge:resumes] { loading: false, selectedId: null }
 * [forge:resumes] { loading: false, selectedId: "abc-123" }
 * ```
 */
export function debugState(label: string, stateGetter: () => Record<string, unknown>): void {
  if (!isDevMode()) return
  $effect(() => {
    console.debug(`[forge:${label}]`, stateGetter())
  })
}

/**
 * Wrap an effect with debug logging that fires when the effect executes.
 *
 * In dev mode, logs a message before running the effect function.
 * In production, just runs the effect directly with no wrapper overhead.
 *
 * @param label - Descriptive name for this effect, e.g. 'fetch-resume-detail'
 * @param fn - The effect function. May return a cleanup function.
 *
 * @example
 * ```svelte
 * <script>
 *   import { tracedEffect } from '$lib/debug.svelte'
 *   let selectedId = $state<string | null>(null)
 *   tracedEffect('fetch-detail', () => {
 *     if (selectedId) fetchDetail(selectedId)
 *   })
 * </script>
 * ```
 *
 * Output:
 * ```
 * [forge:effect] fetch-detail fired
 * ```
 */
export function tracedEffect(label: string, fn: () => void | (() => void)): void {
  if (isDevMode()) {
    $effect(() => {
      console.debug(`[forge:effect] ${label} fired`)
      return fn()
    })
  } else {
    $effect(fn)
  }
}
```

#### Acceptance Criteria
- [ ] File compiles through SvelteKit's Vite pipeline (`.svelte.ts` extension required)
- [ ] `debugState` creates a `$effect` that logs state snapshots when `isDevMode()` is true
- [ ] `debugState` is a no-op (no `$effect` created) when `isDevMode()` is false
- [ ] `tracedEffect` wraps the effect with a debug log prefix when `isDevMode()` is true
- [ ] `tracedEffect` falls back to plain `$effect(fn)` when `isDevMode()` is false
- [ ] Both functions have JSDoc with usage examples

---

### Task 22.2: Add debugState to resumes page for immediate diagnosis

**File to modify:** `packages/webui/src/routes/resumes/+page.svelte`

**Goal:** Add a single `debugState()` call to the resumes page to immediately trace the state flow and diagnose the infinite loading bug.

#### Steps

- [ ] **Add import at top of `<script>` block** (after existing imports):

```typescript
import { debugState } from '$lib/debug.svelte'
```

- [ ] **Add debugState call after the state declarations** (around line 50, after `let detailLoading = $state(false)`). The exact line depends on what other state variables exist, but it should go after all the `$state` declarations and before any `$effect` or `onMount` calls:

```typescript
// Debug: trace all key state transitions to browser console
debugState('resumes', () => ({
  loading,
  detailLoading,
  selectedResumeId,
  hasDetail: !!resumeDetail,
  resumeCount: resumes.length,
}))
```

This will log every state transition as a snapshot object. The output will look like:
```
[forge:resumes] { loading: true, detailLoading: false, selectedResumeId: null, hasDetail: false, resumeCount: 0 }
[forge:resumes] { loading: false, detailLoading: false, selectedResumeId: null, hasDetail: false, resumeCount: 3 }
[forge:resumes] { loading: false, detailLoading: true, selectedResumeId: "abc-123", hasDetail: false, resumeCount: 3 }
[forge:resumes] { loading: false, detailLoading: false, selectedResumeId: "abc-123", hasDetail: true, resumeCount: 3 }
```

If the infinite loading bug is a state management issue, the logs will show which transition never fires (e.g., `detailLoading` stuck at `true`, or `loading` never going to `false`).

#### Acceptance Criteria
- [ ] `debugState` import added to resumes page
- [ ] `debugState('resumes', ...)` call added after state declarations
- [ ] State getter accesses all key state variables: `loading`, `detailLoading`, `selectedResumeId`, `resumeDetail` (as boolean), `resumes` (as count)
- [ ] No other changes to the page logic

---

### Task 22.3: Verify

**Files:** None created or modified. This is a verification task.

**Goal:** Confirm that the debug helpers work in the browser and provide useful diagnostic output.

#### Steps

- [ ] Start the WebUI dev server (`just webui` or equivalent)
- [ ] Navigate to `http://localhost:5173/resumes`
- [ ] Open browser DevTools Console
- [ ] Enable 'Verbose' log level (Chrome/Edge) or check that `console.debug` output is visible (Firefox/Safari)
- [ ] Observe `[forge:resumes]` log lines showing state transitions
- [ ] Click on a resume to trigger `selectedResumeId` change
- [ ] Verify the state snapshot updates in the console
- [ ] Check: does `loading` transition to `false`? Does `detailLoading` transition from `true` to `false`?
- [ ] Cross-reference with `[forge:sdk]` logs from Phase 21 to correlate API requests with state transitions
- [ ] Document findings

#### Acceptance Criteria
- [ ] `[forge:resumes]` logs appear in browser console showing state snapshots
- [ ] State transitions are visible as new log lines whenever any tracked state changes
- [ ] Combined with `[forge:sdk]` logs, the request-to-state flow is fully traceable

---

## Testing Requirements

| Category | Test | Location |
|----------|------|----------|
| Smoke | `[forge:resumes]` logs appear in browser console | Manual |
| Smoke | State transitions logged when clicking resume items | Manual |
| Smoke | `[forge:effect]` logs appear if `tracedEffect` is used | Manual |

**No unit tests.** The `debugState` and `tracedEffect` functions use `$effect`, which is a Svelte compiler directive that can only run inside the Svelte compiler pipeline. Testing requires either a browser or a Svelte component test harness (e.g., `@testing-library/svelte`), which is not currently set up. The functions are simple wrappers -- if `$effect` and `console.debug` work (they do), the helpers work. Verification is done via manual smoke testing.

If a Svelte component test harness is added in a future phase, add tests for:
- `debugState` logs when state changes
- `debugState` is silent when `isDevMode()` returns false
- `tracedEffect` logs the label when the effect fires

---

## Documentation Requirements

- [ ] JSDoc on `debugState()` with usage example and sample output (included in the code above)
- [ ] JSDoc on `tracedEffect()` with usage example and sample output (included in the code above)
- [ ] File-level doc comment explaining why `.svelte.ts` extension is required
- [ ] No new documentation files needed
