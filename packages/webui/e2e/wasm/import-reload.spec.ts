import { readFileSync } from 'node:fs'
import { expect, test } from '@playwright/test'
import { inPage, openApp } from './helpers'

/**
 * The runtime starts reliably when the page reloads right after `importDatabase`
 * (resu-mx/forge#205).
 *
 * The reloaded page takes over the database lock while the previous page's Worker may still hold
 * its OPFS sync access handles, so the first start can fail with "An error occurred while creating
 * sync access handle". The client retries that start with a fresh Worker. About 1 in 5 immediate
 * reloads used to fail, so a single pass proves little: repeat it.
 */

const RELOADS = 20

test('the runtime becomes ready after an import followed by an immediate reload, 20 times in a row', async ({
  page,
}) => {
  test.setTimeout(5 * 60_000)

  await openApp(page, '/')
  const marker = await inPage<{ id: string }>(
    page,
    `return ok(await forge.organizations.create({ name: 'Import reload marker' }))`,
  )

  // Each import is a real SQLite file: IMPORT_RELOAD_FILE if set (a larger one, such as a generated
  // demo dataset, holds the old Worker's handles longer), else the page's own database.
  const file = process.env.IMPORT_RELOAD_FILE
  const b64 = file
    ? readFileSync(file).toString('base64')
    : await page.evaluate(async () => {
        const runtime = (window as unknown as { forgeRuntime: { exportDatabase(): Promise<Uint8Array> } })
          .forgeRuntime
        const bytes = await runtime.exportDatabase()
        let s = ''
        for (const b of bytes) s += String.fromCharCode(b)
        return btoa(s)
      })

  for (let i = 1; i <= RELOADS; i++) {
    await page.evaluate(async (data) => {
      const bytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0))
      const runtime = (window as unknown as { forgeRuntime: { importDatabase(b: Uint8Array): Promise<unknown> } })
        .forgeRuntime
      await runtime.importDatabase(bytes)
    }, b64)

    // No pause: this is the reload that used to race the old Worker's handles.
    await page.reload()
    await page.waitForFunction(() => !!(window as unknown as { forge?: unknown }).forge)

    const state = await page.evaluate(async () => {
      const runtime = (
        window as unknown as { forgeRuntime: { ready: Promise<void>; status(): { state: string; message?: string } } }
      ).forgeRuntime
      try {
        await runtime.ready
        return runtime.status().state
      } catch (e) {
        return `error: ${(e as Error).message}`
      }
    })
    expect(state, `reload ${i} of ${RELOADS}`).toBe('ready')
  }

  // The imported data is what the last page opened (the marker only exists in the self-export).
  if (!file) {
    const org = await inPage<{ name: string }>(page, `return ok(await forge.organizations.get('${marker.id}'))`)
    expect(org.name).toBe('Import reload marker')
  }
})
