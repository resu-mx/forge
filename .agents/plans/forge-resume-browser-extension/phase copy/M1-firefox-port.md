# M1 — Firefox/Zen Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the Forge extension (v0.0.9) to work on Firefox/Zen alongside Chrome, producing dual-browser builds from a single codebase.

**Architecture:** Drop the Chrome-only `@crxjs/vite-plugin` in favor of a manual Vite build with a custom plugin that copies manifests and popup HTML. Both Chrome and Firefox MV3 support `chrome.*` Promise-based APIs, so no runtime polyfill is needed — only the manifest format differs (`service_worker` vs `background.scripts`). Build produces `dist/chrome/` and `dist/firefox/` from a `BROWSER` env var.

**Tech Stack:** Vite 8, Svelte 5, Bun

**Bead:** 3bp.27 — Extension: Firefox support (WebExtensions polyfill + manifest adaptations)

**Worktree:** `.claude/worktrees/forge-ext-m1-firefox/`
**Branch:** `feat/forge-ext/m1-firefox`

---

## File Map

| Action | File | Purpose |
|--------|------|---------|
| Modify | `packages/extension/vite.config.ts` | Drop CRXJS, add manual manifest/popup copy plugin, browser-aware output |
| Modify | `packages/extension/manifest.json` | Update paths for non-CRXJS build (stays as `manifest.json` — Chrome canonical) |
| Create | `packages/extension/manifest.firefox.json` | Firefox MV3 manifest with `background.scripts` + gecko settings |
| Modify | `packages/extension/package.json` | Drop `@crxjs/vite-plugin`, add `web-ext`, add browser-specific scripts |
| Create | `packages/extension/build.ts` | Tiny build script that runs both browser builds sequentially |
| Modify | `packages/extension/tests/background/client.test.ts` | No changes expected (mocks `globalThis.chrome`, still valid) |
| Modify | `packages/extension/tests/background/smoke.test.ts` | No changes expected |

---

### Task 1: Create worktree and branch

**Files:**
- Create: `.claude/worktrees/forge-ext-m1-firefox/` (git worktree)

- [ ] **Step 1: Create worktree from main**

```bash
cd /Users/adam/notes/job-hunting
git worktree add .claude/worktrees/forge-ext-m1-firefox -b feat/forge-ext/m1-firefox main
```

- [ ] **Step 2: Verify worktree**

```bash
cd .claude/worktrees/forge-ext-m1-firefox
git branch --show-current
```

Expected: `feat/forge-ext/m1-firefox`

- [ ] **Step 3: Install deps in worktree**

```bash
cd .claude/worktrees/forge-ext-m1-firefox
bun install
```

- [ ] **Step 4: Verify tests pass in worktree**

```bash
cd .claude/worktrees/forge-ext-m1-firefox/packages/extension
bun test
```

Expected: 58 pass, 0 fail

---

### Task 2: Create Firefox manifest

**Files:**
- Modify: `packages/extension/manifest.json` → becomes Chrome-canonical manifest
- Create: `packages/extension/manifest.firefox.json`

- [ ] **Step 1: Write the Firefox manifest test**

Create `packages/extension/tests/build/manifests.test.ts`:

```ts
// packages/extension/tests/build/manifests.test.ts

import { describe, test, expect } from 'bun:test'
import chromeManifest from '../../manifest.json'
import firefoxManifest from '../../manifest.firefox.json'

describe('Chrome manifest', () => {
  test('is MV3', () => {
    expect(chromeManifest.manifest_version).toBe(3)
  })

  test('has service_worker background', () => {
    expect(chromeManifest.background.service_worker).toBeDefined()
    expect((chromeManifest.background as any).scripts).toBeUndefined()
  })

  test('popup path matches build output', () => {
    expect(chromeManifest.action.default_popup).toBe('popup/index.html')
  })
})

describe('Firefox manifest', () => {
  test('is MV3', () => {
    expect(firefoxManifest.manifest_version).toBe(3)
  })

  test('has background.scripts (not service_worker)', () => {
    expect((firefoxManifest.background as any).service_worker).toBeUndefined()
    expect(firefoxManifest.background.scripts).toEqual(['background.js'])
  })

  test('has gecko settings with extension ID', () => {
    expect(firefoxManifest.browser_specific_settings?.gecko?.id).toBeDefined()
  })

  test('popup path matches Chrome manifest', () => {
    expect(firefoxManifest.action.default_popup).toBe(chromeManifest.action.default_popup)
  })

  test('same permissions as Chrome', () => {
    expect(firefoxManifest.permissions).toEqual(chromeManifest.permissions)
  })

  test('same host_permissions as Chrome', () => {
    expect(firefoxManifest.host_permissions).toEqual(chromeManifest.host_permissions)
  })

  test('same version as Chrome', () => {
    expect(firefoxManifest.version).toBe(chromeManifest.version)
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/extension && bun test tests/build/manifests.test.ts
```

Expected: FAIL — `manifest.firefox.json` does not exist yet, and `manifest.json` still has CRXJS-style paths.

- [ ] **Step 3: Update Chrome manifest paths for non-CRXJS build**

The current `manifest.json` has `"default_popup": "src/popup/index.html"` and `"service_worker": "src/background/index.ts"` — these are source paths that CRXJS rewrites. Without CRXJS, the manifest must reference built output paths.

Update `packages/extension/manifest.json`:

```json
{
  "manifest_version": 3,
  "name": "Forge",
  "version": "0.0.9",
  "description": "Capture job descriptions and autofill applications into Forge.",
  "action": {
    "default_popup": "popup/index.html",
    "default_title": "Forge"
  },
  "permissions": ["activeTab", "scripting", "clipboardWrite", "storage"],
  "host_permissions": ["*://*.linkedin.com/*", "http://localhost:3000/*", "*://*.myworkdayjobs.com/*", "*://*.myworkday.com/*"],
  "background": {
    "service_worker": "background.js",
    "type": "module"
  }
}
```

Key changes: `default_popup` → `popup/index.html`, `service_worker` → `background.js` (build output paths, not source paths).

- [ ] **Step 4: Create Firefox manifest**

Create `packages/extension/manifest.firefox.json`:

```json
{
  "manifest_version": 3,
  "name": "Forge",
  "version": "0.0.9",
  "description": "Capture job descriptions and autofill applications into Forge.",
  "action": {
    "default_popup": "popup/index.html",
    "default_title": "Forge"
  },
  "permissions": ["activeTab", "scripting", "clipboardWrite", "storage"],
  "host_permissions": ["*://*.linkedin.com/*", "http://localhost:3000/*", "*://*.myworkdayjobs.com/*", "*://*.myworkday.com/*"],
  "background": {
    "scripts": ["background.js"],
    "type": "module"
  },
  "browser_specific_settings": {
    "gecko": {
      "id": "forge-extension@forge.local",
      "strict_min_version": "128.0"
    }
  }
}
```

Differences from Chrome:
- `background.scripts` array instead of `background.service_worker`
- `browser_specific_settings.gecko` with extension ID and min Firefox version (128 = stable MV3 support)

- [ ] **Step 5: Run manifest tests**

```bash
cd packages/extension && bun test tests/build/manifests.test.ts
```

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add packages/extension/manifest.json packages/extension/manifest.firefox.json packages/extension/tests/build/manifests.test.ts
git commit -m "feat(ext/m1): add Firefox manifest, update Chrome manifest paths for non-CRXJS build"
```

---

### Task 3: Replace CRXJS with manual Vite build plugin

**Files:**
- Modify: `packages/extension/vite.config.ts`
- Modify: `packages/extension/package.json`

- [ ] **Step 1: Write the build output test**

Create `packages/extension/tests/build/output.test.ts`:

```ts
// packages/extension/tests/build/output.test.ts

import { describe, test, expect, beforeAll } from 'bun:test'
import { existsSync, readFileSync } from 'fs'
import { resolve } from 'path'

const distRoot = resolve(import.meta.dir, '../../dist')

// These tests run AFTER `bun run build:chrome` and `bun run build:firefox`
// Skip if dist/ doesn't exist (CI may not build first)
const chromeDistExists = existsSync(resolve(distRoot, 'chrome/manifest.json'))
const firefoxDistExists = existsSync(resolve(distRoot, 'firefox/manifest.json'))

describe('Chrome build output', () => {
  const skip = !chromeDistExists

  test.skipIf(skip)('has manifest.json', () => {
    const manifest = JSON.parse(readFileSync(resolve(distRoot, 'chrome/manifest.json'), 'utf-8'))
    expect(manifest.manifest_version).toBe(3)
    expect(manifest.background.service_worker).toBe('background.js')
  })

  test.skipIf(skip)('has background.js', () => {
    expect(existsSync(resolve(distRoot, 'chrome/background.js'))).toBe(true)
  })

  test.skipIf(skip)('has popup/index.html', () => {
    const html = readFileSync(resolve(distRoot, 'chrome/popup/index.html'), 'utf-8')
    expect(html).toContain('<div id="app">')
  })

  test.skipIf(skip)('has content scripts', () => {
    expect(existsSync(resolve(distRoot, 'chrome/content/linkedin.js'))).toBe(true)
    expect(existsSync(resolve(distRoot, 'chrome/content/workday.js'))).toBe(true)
  })

  test.skipIf(skip)('content scripts are IIFE (no ES imports)', () => {
    const linkedin = readFileSync(resolve(distRoot, 'chrome/content/linkedin.js'), 'utf-8')
    // Content scripts injected via executeScript cannot use ES module imports.
    // They must be IIFE or plain scripts — no `import` statements.
    expect(linkedin).not.toMatch(/^import\s/m)
  })
})

describe('Firefox build output', () => {
  const skip = !firefoxDistExists

  test.skipIf(skip)('has manifest.json with background.scripts', () => {
    const manifest = JSON.parse(readFileSync(resolve(distRoot, 'firefox/manifest.json'), 'utf-8'))
    expect(manifest.manifest_version).toBe(3)
    expect(manifest.background.scripts).toEqual(['background.js'])
    expect(manifest.browser_specific_settings?.gecko?.id).toBeDefined()
  })

  test.skipIf(skip)('has background.js', () => {
    expect(existsSync(resolve(distRoot, 'firefox/background.js'))).toBe(true)
  })

  test.skipIf(skip)('has popup/index.html', () => {
    expect(existsSync(resolve(distRoot, 'firefox/popup/index.html'))).toBe(true)
  })

  test.skipIf(skip)('has content scripts', () => {
    expect(existsSync(resolve(distRoot, 'firefox/content/linkedin.js'))).toBe(true)
    expect(existsSync(resolve(distRoot, 'firefox/content/workday.js'))).toBe(true)
  })
})
```

- [ ] **Step 2: Run to verify tests skip (no builds yet)**

```bash
cd packages/extension && bun test tests/build/output.test.ts
```

Expected: All tests SKIP (dist doesn't exist yet with new structure).

- [ ] **Step 3: Remove CRXJS from package.json**

In `packages/extension/package.json`, remove `"@crxjs/vite-plugin"` from `devDependencies`.

- [ ] **Step 4: Rewrite vite.config.ts**

Replace `packages/extension/vite.config.ts` entirely:

```ts
// packages/extension/vite.config.ts

import { defineConfig, type Plugin } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { copyFileSync, cpSync, mkdirSync } from 'fs'
import { resolve } from 'path'

const browser = (process.env.BROWSER ?? 'chrome') as 'chrome' | 'firefox'
const outDir = `dist/${browser}`

/**
 * Custom plugin to copy the browser-appropriate manifest and popup HTML
 * into the build output. Replaces @crxjs/vite-plugin.
 */
function extensionBuildPlugin(): Plugin {
  return {
    name: 'forge-extension-build',
    writeBundle() {
      // Copy the right manifest
      const manifestSrc = browser === 'firefox'
        ? 'manifest.firefox.json'
        : 'manifest.json'
      copyFileSync(manifestSrc, resolve(outDir, 'manifest.json'))
    },
  }
}

export default defineConfig({
  plugins: [
    svelte(),
    extensionBuildPlugin(),
  ],
  build: {
    outDir,
    emptyOutDir: true,
    // Content scripts must be IIFE — they're injected via executeScript
    // which doesn't support ES module imports in the injected context.
    rollupOptions: {
      input: {
        'background': 'src/background/index.ts',
        'content-linkedin': 'src/content/linkedin.ts',
        'content-workday': 'src/content/workday.ts',
        'popup': 'src/popup/index.html',
      },
      output: {
        entryFileNames(chunkInfo) {
          if (chunkInfo.name === 'background') return 'background.js'
          if (chunkInfo.name === 'content-linkedin') return 'content/linkedin.js'
          if (chunkInfo.name === 'content-workday') return 'content/workday.js'
          // popup entry: vite processes the HTML, JS lands in assets/
          return 'assets/[name]-[hash].js'
        },
        // Content scripts need inlineDynamicImports-like behavior.
        // Force them into single chunks (no shared chunks that break injection).
        manualChunks(id, { getModuleInfo }) {
          // Plugin modules imported by content scripts must stay in the
          // content script bundle — not get extracted to a shared chunk.
          // The background worker is a separate entry and can have shared chunks.
          // This is the "shared chunk constraint" from the prototype.
          return undefined  // let rollup decide, but see chunkFileNames below
        },
        chunkFileNames: 'assets/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash][extname]',
      },
    },
  },
})
```

**Note:** The shared chunk constraint (background worker must NOT import plugin modules) is enforced by architecture, not build config. Content scripts import plugins directly; background worker never imports from `plugin/plugins/*`. If this is violated, content scripts will break because `executeScript` can't load ES module chunks.

- [ ] **Step 5: Update popup HTML path**

The popup HTML at `src/popup/index.html` is used as a Vite HTML entry. Vite will output it relative to the entry path. We need the output at `popup/index.html` in the dist.

Vite places HTML entries based on their path relative to project root. `src/popup/index.html` → output at `dist/{browser}/src/popup/index.html`. To get `popup/index.html`, move the HTML to the root-level `popup/` path or use Vite's `root` option.

The simplest fix: add a Vite plugin to move the HTML output:

Update the `extensionBuildPlugin` in `vite.config.ts` — add HTML relocation to `writeBundle`:

```ts
function extensionBuildPlugin(): Plugin {
  return {
    name: 'forge-extension-build',
    writeBundle() {
      // Copy the right manifest
      const manifestSrc = browser === 'firefox'
        ? 'manifest.firefox.json'
        : 'manifest.json'
      copyFileSync(manifestSrc, resolve(outDir, 'manifest.json'))

      // Vite outputs popup HTML at src/popup/index.html (preserving input path).
      // Move to popup/index.html to match manifest's default_popup.
      const builtPopupDir = resolve(outDir, 'src/popup')
      const targetPopupDir = resolve(outDir, 'popup')
      try {
        mkdirSync(targetPopupDir, { recursive: true })
        cpSync(builtPopupDir, targetPopupDir, { recursive: true })
        // Clean up the intermediate src/ directory
        const { rmSync } = require('fs')
        rmSync(resolve(outDir, 'src'), { recursive: true, force: true })
      } catch {
        // If src/popup doesn't exist, Vite may have used a different output path
        // Check if popup/index.html already exists at the right location
      }
    },
  }
}
```

Actually, a cleaner approach — use Vite's `build.rollupOptions.input` with an alias:

Change the input to use a resolved path with a custom output name. Replace the input entry:

```ts
input: {
  'background': resolve(__dirname, 'src/background/index.ts'),
  'content-linkedin': resolve(__dirname, 'src/content/linkedin.ts'),
  'content-workday': resolve(__dirname, 'src/content/workday.ts'),
  // Use resolve to get absolute path — Vite preserves directory structure
  // relative to the determined 'root'. We handle popup output in the plugin.
  'popup/index': resolve(__dirname, 'src/popup/index.html'),
},
```

By naming the key `popup/index`, Vite should output the HTML at `popup/index.html`.

Test this during Step 6 and adjust as needed. The plugin's `writeBundle` hook can fix up paths if Vite doesn't cooperate.

- [ ] **Step 6: Run bun install to remove CRXJS**

```bash
cd packages/extension && bun install
```

- [ ] **Step 7: Build for Chrome**

```bash
cd packages/extension && BROWSER=chrome bun run build
```

Expected: Build succeeds. Check output structure:

```bash
ls -la dist/chrome/
# Should contain: manifest.json, background.js, popup/index.html, content/linkedin.js, content/workday.js
```

- [ ] **Step 8: Build for Firefox**

```bash
cd packages/extension && BROWSER=firefox bun run build
```

Expected: Build succeeds with Firefox manifest.

- [ ] **Step 9: Run build output tests**

```bash
cd packages/extension && bun test tests/build/output.test.ts
```

Expected: All tests PASS (both builds exist now).

- [ ] **Step 10: Run all existing tests**

```bash
cd packages/extension && bun test
```

Expected: All 58 existing tests still pass (source code unchanged). Build output tests also pass. New manifest tests pass.

- [ ] **Step 11: Commit**

```bash
git add packages/extension/vite.config.ts packages/extension/package.json packages/extension/tests/build/
git commit -m "feat(ext/m1): replace CRXJS with manual Vite build, dual-browser output"
```

---

### Task 4: Add browser-specific dev and build scripts

**Files:**
- Modify: `packages/extension/package.json`
- Create: `packages/extension/build.ts`

- [ ] **Step 1: Create dual-build script**

Create `packages/extension/build.ts`:

```ts
// packages/extension/build.ts
// Builds the extension for both Chrome and Firefox sequentially.

import { $ } from 'bun'

console.log('Building Chrome...')
await $`BROWSER=chrome bun run vite build`

console.log('Building Firefox...')
await $`BROWSER=firefox bun run vite build`

console.log('Done. Outputs:')
console.log('  dist/chrome/  — load in chrome://extensions')
console.log('  dist/firefox/ — load in about:debugging')
```

- [ ] **Step 2: Update package.json scripts**

Update the `scripts` section in `packages/extension/package.json`:

```json
{
  "scripts": {
    "dev": "BROWSER=firefox vite build --watch",
    "dev:chrome": "BROWSER=chrome vite build --watch",
    "dev:firefox": "BROWSER=firefox vite build --watch",
    "build": "bun run build.ts",
    "build:chrome": "BROWSER=chrome vite build",
    "build:firefox": "BROWSER=firefox vite build",
    "test": "bun test"
  }
}
```

Note: `dev` defaults to Firefox since that's the user's daily browser (Zen). Chrome dev is explicitly `dev:chrome`.

- [ ] **Step 3: Verify both build scripts work**

```bash
cd packages/extension && bun run build
```

Expected: Both `dist/chrome/` and `dist/firefox/` produced.

- [ ] **Step 4: Verify dev:firefox starts watch mode**

```bash
cd packages/extension && timeout 5 bun run dev:firefox || true
```

Expected: Vite starts in watch mode, builds Firefox output, then we kill it.

- [ ] **Step 5: Commit**

```bash
git add packages/extension/build.ts packages/extension/package.json
git commit -m "feat(ext/m1): add dual-browser build scripts, default dev to Firefox"
```

---

### Task 5: Verify Chrome build loads as extension

This is a manual verification task. The build must produce a loadable Chrome extension.

**Files:** None (verification only)

- [ ] **Step 1: Build for Chrome**

```bash
cd packages/extension && bun run build:chrome
```

- [ ] **Step 2: Verify dist structure**

```bash
ls -R dist/chrome/
```

Expected structure:
```
dist/chrome/
├── manifest.json
├── background.js
├── popup/
│   └── index.html
├── content/
│   ├── linkedin.js
│   └── workday.js
└── assets/
    └── *.js (Svelte runtime, shared code)
```

- [ ] **Step 3: Verify manifest references resolve**

```bash
cd dist/chrome
# Check that files referenced in manifest exist:
cat manifest.json | grep -E '"(background\.js|popup/index\.html)"'
test -f background.js && echo "background.js OK"
test -f popup/index.html && echo "popup/index.html OK"
```

- [ ] **Step 4: Check content scripts are self-contained (no ES imports)**

```bash
head -5 dist/chrome/content/linkedin.js
# Should NOT start with 'import' — must be IIFE or plain script
```

If content scripts have `import` statements, the shared chunk constraint is violated. Fix in vite.config.ts by forcing IIFE format for content script entries.

- [ ] **Step 5: Manual test — load in Chrome**

1. Open `chrome://extensions`
2. Enable Developer mode
3. Click "Load unpacked" → select `packages/extension/dist/chrome/`
4. Extension should load with no errors
5. Click extension icon → popup opens with health dot
6. Navigate to a LinkedIn job page → click "Capture Job" → should work

- [ ] **Step 6: Record results and fix any issues**

If the build output structure is wrong or the extension fails to load, fix vite.config.ts and rebuild. Common issues:
- Popup HTML at wrong path → adjust `extensionBuildPlugin` to move it
- Content scripts have ES imports → add `output.format: 'iife'` for content entries (requires splitting into separate rollup builds)
- Background worker can't load → check `background.js` exists and is valid module

---

### Task 6: Verify Firefox/Zen build loads as extension

Manual verification in the user's actual browser.

**Files:** None (verification only)

- [ ] **Step 1: Build for Firefox**

```bash
cd packages/extension && bun run build:firefox
```

- [ ] **Step 2: Verify Firefox manifest in output**

```bash
cat dist/firefox/manifest.json | bun -e "const m = JSON.parse(await Bun.stdin.text()); console.log('background:', JSON.stringify(m.background)); console.log('gecko:', JSON.stringify(m.browser_specific_settings))"
```

Expected:
```
background: {"scripts":["background.js"],"type":"module"}
gecko: {"gecko":{"id":"forge-extension@forge.local","strict_min_version":"128.0"}}
```

- [ ] **Step 3: Manual test — load in Zen/Firefox**

1. Open `about:debugging#/runtime/this-firefox`
2. Click "Load Temporary Add-on..."
3. Select `packages/extension/dist/firefox/manifest.json`
4. Extension should load with no errors in the console
5. Click extension icon → popup opens with health dot
6. Navigate to a LinkedIn job page → click "Capture Job" → should work
7. Navigate to a Workday application page → click "Autofill" → should fill fields

- [ ] **Step 4: Check for Firefox-specific API issues**

Known potential issues in Firefox:
- `chrome.scripting.executeScript` — verify it works with `files: ['content/linkedin.js']`
- `chrome.tabs.sendMessage` return value — verify response comes back correctly
- `chrome.storage.local` — verify config loads on popup open
- `chrome.runtime.getManifest()` — verify version displays in popup

If any API calls fail, see Task 7 for the fix pattern.

- [ ] **Step 5: Record results**

Note which features work and which fail. Proceed to Task 7 for any fixes needed.

---

### Task 7: Fix Firefox-specific API compatibility issues

This task handles any issues discovered during Task 6. It may be empty if Firefox works out of the box.

**Files:**
- Possibly create: `packages/extension/src/lib/browser.ts` (if polyfill is needed)
- Possibly modify: files that use broken `chrome.*` APIs

- [ ] **Step 1: If `chrome.*` APIs work on Firefox — skip this task**

If Task 6 passed with no issues, mark this task complete and proceed to Task 8.

- [ ] **Step 2: If specific APIs fail — create browser abstraction**

If `chrome.scripting.executeScript` or another API fails on Firefox, create a thin abstraction:

Create `packages/extension/src/lib/browser.ts`:

```ts
// packages/extension/src/lib/browser.ts
//
// Cross-browser API abstraction. Firefox supports `browser.*` natively
// with Promise returns. Chrome supports `chrome.*` with Promises in MV3.
// Prefer `browser` (Firefox-native) and fall back to `chrome` (Chrome).

type BrowserAPI = typeof chrome

const browser: BrowserAPI =
  (globalThis as any).browser ?? (globalThis as any).chrome

export default browser
```

Then replace failing `chrome.*` calls in the affected files with:

```ts
import browser from '../lib/browser'

// Replace: chrome.tabs.query(...)
// With:    browser.tabs.query(...)
```

Only replace calls that actually fail on Firefox. Don't preemptively replace working calls.

- [ ] **Step 3: If polyfill is needed for edge cases — install webextension-polyfill**

Only if the thin abstraction in Step 2 isn't sufficient:

```bash
cd packages/extension && bun add webextension-polyfill && bun add -d @types/webextension-polyfill
```

Replace `src/lib/browser.ts`:

```ts
import browser from 'webextension-polyfill'
export default browser
```

Update test mocks to also set `globalThis.browser`:

```ts
;(globalThis as any).browser = (globalThis as any).chrome
```

- [ ] **Step 4: Re-verify both browsers**

After any fixes, rebuild and verify both Chrome and Firefox still work:

```bash
cd packages/extension && bun run build
bun test
```

- [ ] **Step 5: Commit fixes (if any)**

```bash
git add -A
git commit -m "fix(ext/m1): resolve Firefox API compatibility issues"
```

---

### Task 8: Bump version to 0.1.1, final verification, commit

**Files:**
- Modify: `packages/extension/manifest.json` (version)
- Modify: `packages/extension/manifest.firefox.json` (version)

- [ ] **Step 1: Bump version in both manifests**

In `packages/extension/manifest.json`:
```json
"version": "0.1.1",
```

In `packages/extension/manifest.firefox.json`:
```json
"version": "0.1.1",
```

- [ ] **Step 2: Run full test suite**

```bash
cd packages/extension && bun test
```

Expected: All tests pass (58 existing + new manifest + build output tests).

- [ ] **Step 3: Full rebuild**

```bash
cd packages/extension && bun run build
```

Expected: Both `dist/chrome/` and `dist/firefox/` built successfully.

- [ ] **Step 4: Final manual verification checklist**

- [ ] `bun run build` succeeds for both browsers
- [ ] Chrome: extension loads from `dist/chrome/`, popup shows v0.1.1, health check works
- [ ] Firefox/Zen: extension loads from `dist/firefox/`, popup shows v0.1.1, health check works
- [ ] Chrome: capture JD on LinkedIn works
- [ ] Firefox/Zen: capture JD on LinkedIn works
- [ ] Chrome: autofill on Workday works
- [ ] Firefox/Zen: autofill on Workday works
- [ ] All tests pass

- [ ] **Step 5: Commit version bump**

```bash
git add packages/extension/manifest.json packages/extension/manifest.firefox.json
git commit -m "chore(ext): bump version to 0.1.1 (M1)"
```

---

### Task 9: Merge to main and rebuild dist

**Files:** None (git operations only)

- [ ] **Step 1: Merge worktree branch to main**

```bash
cd /Users/adam/notes/job-hunting
git checkout main
git merge feat/forge-ext/m1-firefox --no-ff -m "Merge M1 (Firefox/Zen port)"
```

- [ ] **Step 2: Rebuild dist on main**

```bash
cd packages/extension && bun run build
```

Per project rules: rebuild dist/ on main after every extension merge.

- [ ] **Step 3: Close the bead**

```bash
bd close job-hunting-3bp.27 --sha $(git rev-parse HEAD)
```

- [ ] **Step 4: Clean up worktree**

```bash
cd /Users/adam/notes/job-hunting
git worktree remove .claude/worktrees/forge-ext-m1-firefox
```

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Content scripts produce ES imports after dropping CRXJS | Check build output for `import` statements. If present, split Vite build into two passes: one for content scripts (IIFE format) and one for background + popup (ESM). |
| Firefox `chrome.scripting.executeScript` behaves differently | Task 7 has a layered fix strategy: try as-is → thin abstraction → full polyfill. |
| Vite popup HTML output lands at wrong path | `extensionBuildPlugin` has a `writeBundle` hook that can relocate files. Adjust paths as needed during Task 5. |
| Zen (Firefox fork) has non-standard quirks | Test in both stock Firefox and Zen. Zen tracks Firefox stable closely, so quirks are unlikely but possible. |
