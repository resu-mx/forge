import { defineConfig } from '@playwright/test'

// The browser-first acceptance spec: the Rust API runs inside the page, so the only server is
// the one serving the static UI. Build the wasm bundles first (`just wasm-bundle typst-bundle`).
//
// Locally this uses the dev server (reused if it is already up on :5198). In CI set
// FORGE_E2E_PREVIEW=1 to serve the production build instead, which starts faster and is what
// ships.
const preview = !!process.env.FORGE_E2E_PREVIEW
const PORT = 5198

export default defineConfig({
  testDir: './e2e/wasm',
  timeout: 120_000,
  expect: { timeout: 15_000 },
  retries: 0,
  // One browser at a time: the board drag below is driven by animation frames and is not
  // reliable with several pages competing for the machine.
  workers: 1,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    // Full Chromium (new headless mode), the closest thing to the Chrome people run.
    channel: 'chromium',
    baseURL: `http://localhost:${PORT}`,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: preview
      ? `bun run build && bun run preview --port ${PORT} --strictPort`
      : `bun run dev --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 300_000,
  },
})
