import { readFileSync, statSync } from 'node:fs'
import { basename } from 'node:path'
import { expect, type Page } from '@playwright/test'

/** Shared steps for the browser-first (wasm) specs. */

type Forge = Record<string, any>

/** Run `body` in the page with the app's own client (`window.forge`) and return its result. */
export async function inPage<T>(page: Page, body: string): Promise<T> {
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

/** Open `path` and wait until the in-page runtime owns the database and it is open. */
export async function openApp(page: Page, path = '/') {
  await page.goto(path)
  await page.waitForFunction(() => !!(window as unknown as { forge?: unknown }).forge)
  await inPage(page, 'await window.forgeRuntime.ready')
}

/**
 * Replace the page's database with the SQLite file at `file`, then reload into it.
 *
 * - `ui`: what a person does. Settings → Storage, choose the file, confirm "Replace data"; the
 *   page reloads itself once the import is done.
 * - `runtime`: what an agent does. Hand the bytes to `window.forgeRuntime.importDatabase`, then
 *   reload (as the UI does) so nothing on screen still holds the old database.
 *
 * Either way the page is left on `/` with the runtime ready.
 */
export async function importDataset(page: Page, file: string, via: 'ui' | 'runtime'): Promise<void> {
  if (via === 'ui') {
    await openApp(page, '/settings/storage')
    const input = page.getByTestId('import-file')
    await expect(input).toBeEnabled()
    await input.setInputFiles(file)

    const confirm = page.getByRole('alertdialog', { name: 'Replace all data?' })
    await expect(confirm).toContainText(basename(file))
    const reloaded = page.waitForEvent('load', { timeout: 60_000 })
    await confirm.getByRole('button', { name: 'Replace data' }).click()
    // The success toast shows before the page reloads itself; a rejected file shows
    // "Import failed: …" instead and the page stays put.
    await expect(page.getByText(/^Imported \d+ KB\. Reloading/)).toBeVisible({ timeout: 60_000 })
    await reloaded
    await page.waitForFunction(() => !!(window as unknown as { forge?: unknown }).forge)
    await inPage(page, 'await window.forgeRuntime.ready')
    await openApp(page, '/')
    return
  }

  await openApp(page, '/')
  const info = await page.evaluate(async (b64) => {
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
    const runtime = (window as unknown as { forgeRuntime: { importDatabase(b: Uint8Array): Promise<unknown> } })
      .forgeRuntime
    return (await runtime.importDatabase(bytes)) as { imported_bytes: number; migrations: number | null }
  }, readFileSync(file).toString('base64'))
  expect(info.imported_bytes).toBe(statSync(file).size)
  // Reload after the same pause the Storage page leaves (its setTimeout before
  // location.reload()). Reloading at once intermittently fails the next page's start with
  // "install OPFS storage: An error occurred while creating sync access handle": the tab's
  // Web Lock is handed over before the old Worker lets go of its OPFS handles, and nothing
  // retries. Seen 3 times in 15 immediate reloads; an app bug, not a dataset problem.
  await page.waitForTimeout(800)
  await openApp(page, '/')
}
