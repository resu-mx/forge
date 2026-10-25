# `packages/webui/`: web UI

SvelteKit 2 + Svelte 5 on Vite, built with `adapter-static`. The dev server listens on `:5173`
(`just app` starts it in the default mode; `just webui` starts it against an API server, which
needs `just api` running alongside).

## Modes

- **wasm** (the default): the API is the Rust runtime inside a browser Worker (`@forge/runtime`).
  There is no server. A plain `bun run dev` or `build` needs the bundles first:
  `just wasm-bundle typst-bundle` (or `just app`, which builds them and starts the UI).
- **api**: `VITE_FORGE_MODE=api`. Vite proxies `/api` to `FORGE_API_URL`, which defaults to
  `http://localhost:3000`. In Docker it is `http://core:3000`. `just dev`, `just debug`,
  `just webui`, the Docker stacks and the image build all set it.
  - In api mode `@forge/runtime/worker-factory` is aliased to `src/lib/runtime-stub.ts`, so the
    build never bundles the wasm. The default lives in `src/lib/sdk.ts` and `vite.config.ts`; keep
    them in step.
  - The runtime fires `forge:changed` after every write. A list page that listens for it
    (`src/lib/forge-changed.ts`) shows agent writes made through `window.forge` without a
    reload. So far only `SourcesView` and `BulletsView` listen.

## Accounts (sign-in)

Signed out, the app is local-only and unchanged. Signed in, the account service (`id.resu.mx`,
BetterAuth over Auth0) tells the app who you are; the contract is the snapshot-API spec.

- `src/lib/config.ts`: `idUrl` / `dataUrl` (build variables `VITE_ID_URL` / `VITE_DATA_URL`, defaulting to the
  production hosts) and `signInAvailable()` / `syncAvailable()`. They are true only on `app.resu.mx`,
  `localhost` and `127.0.0.1`: a `*.pages.dev` preview cannot carry the `.resu.mx` cookie. Where they are
  false the sign-in UI is not rendered and the app makes no request to the account service.
- The defaults live in `src/lib/config-defaults.js` (plain JS) because `svelte.config.js` imports them to
  build the CSP `connect-src`. Add a service host there, not in two places.
- `src/lib/stores/session.svelte.ts` holds the `$state`; the logic is in `src/lib/session-core.ts`, which has
  no Svelte in it so `bun test` can drive it with a stubbed `fetch`. How a refused invite-only sign-in comes
  back on the URL is read in one function, `readAuthReturn`.
- The wasm e2e build points both hosts at `*.e2e.test` (`e2e/wasm/e2e-hosts.ts`); `e2e/wasm/auth.spec.ts`
  stubs them with `page.route`. Stub them in any spec that needs the signed-in UI.

## Cloud sync

A signed-in person's database is backed up to the snapshot service and restored on another
device. It exists only when the build switches it on (`VITE_CLOUD_SYNC=on`, off by default; the
production build in `app-build.yml` sets it explicitly), in wasm mode, on a host that can carry the
session cookie (`syncAvailable()`); elsewhere nothing is created, requested or rendered. The e2e
config turns it on.

- `src/lib/sync/engine.ts` and `decide.ts` are the engine and its decision table (pure, injected
  dependencies, tested with fakes). `controller.ts` connects it to the session, runtime ownership,
  `forge:changed`, page hide, toasts and the reload a pull asks for, and exposes one `view`.
  `stores/sync.svelte.ts` holds that view in `$state.raw` and attaches the DOM listeners. Keep
  logic in `$lib/sync`, not in the store.
- `$state.raw`, not `$state`, for the view: the conflict object must keep its identity so a
  dismissed dialog stays dismissed.
- Wording lives in `src/lib/sync/copy.ts` as pure functions (status line, conflict dialog text,
  error messages). Change words there, with its test.
- `local-data.ts` decides whether this browser "has data". It counts only tables the schema never
  seeds (sources, bullets, resumes, job descriptions); do not add a seeded table.
- UI: `SyncIndicator` (sidebar, under the account), `SyncConflictDialog` (mounted once in the
  layout; opens itself, always offers "Export this browser's data first", can be put away with
  "Decide later"), and the "Cloud sync" card on Settings, Storage.
- "Delete cloud copy" deletes the data, turns sync off in this browser and unbinds it; another
  device that still syncs asks before uploading again. A browser bound to an empty cloud (it never
  synced a copy) is not asked: its first edits upload as usual.
- `e2e/wasm/cloud-sync.spec.ts` stubs both services with an in-memory snapshot server that
  enforces what the real one does, and runs two browser contexts as two devices.

## Acceptance spec

`e2e/wasm/core-loop.spec.ts` drives the browser-first core loop in Chromium with no server
(`bun run test:e2e:wasm`; `FORGE_E2E_PREVIEW=1` serves the production build, as CI does). The
agent's steps go through `window.forge`; approving, creating the resume and the preview are UI
steps. It runs with one worker because the board drag is driven by animation frames. Playwright's
own browser download can hang under Node 26: use `bun --bun x playwright install chromium`.
The older `e2e/*.spec.ts` files use `playwright.config.ts` and need the TypeScript server.
Shared steps (`inPage`, `openApp`, `importDataset`) are in `e2e/wasm/helpers.ts`.
`e2e/wasm/demo-datasets.spec.ts` skips unless `FORGE_DEMO_DATA_DIR` is set; run it with
`just demo-data e2e` (see `packages/demo-data/AGENTS.md`).

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
