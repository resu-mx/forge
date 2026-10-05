import { expect, test, type Page } from '@playwright/test'

/**
 * Settings → Extension Config / Extension Logs in the browser-first app (resu-mx/forge#39).
 *
 * The extension can't reach the in-browser database yet (ADR 0002), so both pages are hidden from
 * the profile menu and, when opened directly, explain why without calling /api/extension/*.
 *
 * When the bridge (#107–#109) and the endpoints (#110, #111, #113, #114) land, the change that
 * removes the gate (src/lib/extension-gate.ts) rewrites the GATED block below to assert that
 * config loads/saves and logs list/clear.
 */

type Forge = Record<string, any>

/** Run `body` in the page with the app's own client (`window.forge`) and return its result. */
async function inPage<T>(page: Page, body: string): Promise<T> {
  return page.evaluate(
    async (src) => {
      const forge = (window as unknown as { forge: Forge }).forge
      const ok = <R,>(r: { ok: boolean; data?: R; error?: unknown }): R => {
        if (!r.ok) throw new Error(JSON.stringify(r.error))
        return r.data as R
      }
      return await new Function('forge', 'ok', `return (async () => { ${src} })()`)(forge, ok)
    },
    body,
  ) as Promise<T>
}

async function openApp(page: Page, path = '/') {
  await page.goto(path)
  await page.waitForFunction(() => !!(window as unknown as { forge?: unknown }).forge)
  await inPage(page, 'await window.forgeRuntime.ready')
}

// ── GATED: wasm mode until the extension bridge lands ──────────────────────────────────────

test('profile menu hides the extension pages in the browser app', async ({ page }) => {
  await openApp(page)
  await page.locator('.profile-button').click()
  const menu = page.locator('.profile-menu')
  await expect(menu.getByRole('button', { name: 'Storage', exact: true })).toBeVisible()
  await expect(menu.getByRole('button', { name: 'Extension Config' })).toHaveCount(0)
  await expect(menu.getByRole('button', { name: 'Extension Logs' })).toHaveCount(0)
})

for (const [path, heading] of [
  ['/settings/extension', 'Extension Config'],
  ['/settings/extension-logs', 'Extension Logs'],
] as const) {
  test(`${path} explains itself and calls no extension endpoint`, async ({ page }) => {
    const pageErrors: string[] = []
    page.on('pageerror', (e) => pageErrors.push(e.message))

    await openApp(page, path)
    await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible()
    await expect(page.getByTestId('extension-unavailable')).toContainText(
      "can't connect to the in-browser database yet",
    )

    // Sync with the runtime: it answers in order, so any mount-time call has finished and been logged.
    await inPage(page, 'ok(await forge.health())')
    // The UI's client logs every response (debug: true); 501s from unported routes included.
    expect(await inPage<number>(page, `return forge.debug.getByPath('/api/extension').length`)).toBe(0)

    await expect(page.locator('.toast')).toHaveCount(0)
    await expect(page.getByRole('button', { name: 'Save Config' })).toHaveCount(0)
    await expect(page.getByRole('button', { name: 'Clear All' })).toHaveCount(0)
    expect(pageErrors).toEqual([])
  })
}
