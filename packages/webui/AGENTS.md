# `packages/webui/`: web UI

SvelteKit 2 + Svelte 5 on Vite, built with `adapter-static`. The dev server listens on `:5173`
(`just webui` starts it, but it needs `just api` running alongside).

## Modes

- **api** (the default): Vite proxies `/api` to `FORGE_API_URL`, which defaults to
  `http://localhost:3000`. In Docker it is `http://core:3000`.
- **wasm**: `VITE_FORGE_MODE=wasm` serves the API from the Rust runtime inside a browser Worker
  (`@forge/runtime`).
  - Build the runtime first with `just wasm-bundle`, plus `just typst-bundle` if you need PDFs.
  - Outside wasm mode, `@forge/runtime/worker-factory` is aliased to `src/lib/runtime-stub.ts`,
    so an ordinary build never bundles the wasm.
  - The runtime fires `forge:changed` after every write. A list page that listens for it
    (`src/lib/forge-changed.ts`) shows agent writes made through `window.forge` without a
    reload. So far only `SourcesView` and `BulletsView` listen.

## Checks

| Command | Runs |
|---|---|
| `bun run check` | `svelte-kit sync` + `svelte-check` |
| `bun run test:e2e` | Playwright; specs in `e2e/` |
| `bun test src` | unit tests in `src/__tests__/` (`bun:test`). Limit it to `src/`: a bare `bun test` also picks up the Playwright specs |

## Shared components (required)

These rules apply to every Svelte and CSS file in this package.

### Forbidden patterns

1. **No inline viewport-escape CSS.** Full-viewport pages MUST use `<PageWrapper>`, not inline `height: calc(100vh - 4rem); margin: -2rem`.

2. **No inline split-panel CSS.** Two-column list+detail layouts MUST use `<SplitPanel>`, not inline `.list-panel` / `.editor-panel` CSS.

3. **No inline list-header CSS.** Split-panel list headers MUST use `<ListPanelHeader>`, not inline `.list-header` + `.btn-new` markup and CSS.

4. **No page-scoped button CSS.** Button styling MUST use global `.btn` + `.btn-primary`/`.btn-danger`/`.btn-ghost` classes from `base.css`, not page-scoped button CSS. "Create new" action buttons MUST use `--color-primary`, not `--color-info`.

5. **No inline page-title CSS.** Page titles MUST use `<PageHeader>`, not inline `.page-title` CSS.

6. **No inline tab-btn CSS.** Tab navigation MUST use `<TabBar>`, not hand-rolled `.tab-btn` or `.filter-tab` CSS.

7. **No inline editor-empty CSS.** "No selection" panel states MUST use `<EmptyPanel>`, not inline `.editor-empty` CSS or `<EmptyState>`.

8. **No inline search-input CSS.** List search/filter inputs MUST use `<ListSearchInput>`, not inline `.search-input` CSS.

### Component import pattern

All shared components are exported from `$lib/components/index.ts`:

```svelte
<script lang="ts">
  import { PageWrapper, SplitPanel, ListPanelHeader, PageHeader, TabBar, EmptyPanel, ListSearchInput } from '$lib/components'
</script>
```

### When adding new pages

New pages that use any of the patterns above MUST use the shared component. Never copy CSS from an existing page -- import the component instead.
