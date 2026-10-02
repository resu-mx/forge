import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  // The browser-first spec has its own config (playwright.wasm.config.ts).
  testIgnore: '**/wasm/**',
  timeout: 30000,
  retries: 0,
  use: {
    baseURL: 'http://localhost:5173',
    screenshot: 'only-on-failure',
  },
  webServer: {
    command: 'cd ../.. && just dev',
    url: 'http://localhost:5173',
    reuseExistingServer: true,
    timeout: 30000,
  },
})
