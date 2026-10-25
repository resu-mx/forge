import { expect, test, type Page } from '@playwright/test'
import { inPage, openApp } from './helpers'

/**
 * The schema head a snapshot carries, and what the runtime does with one it cannot open
 * (resu-mx/forge#254, partial fix for #123).
 *
 * - An export is a rollback-journal SQLite file (header bytes 18 and 19 are 1), because the
 *   snapshot service rejects anything else, and it names its schema head, the newest bundle
 *   migration it knows, and the bundle version.
 * - `importDatabase` refuses a file written by a newer bundle before anything is replaced, and
 *   puts the old data back if a file it did accept turns out not to open.
 *
 * The files the tests import are real exports with the names in `_migrations` patched in place,
 * so no SQLite library is needed. A name keeps its length, which keeps every record and index
 * entry the same size; the patched head still sorts last, so the index stays in order.
 */

interface Exported {
  bytes: Buffer
  schema_head: string | null
  known_head: string
  bundle_version: string
}

type ImportResult =
  | { ok: true; info: { imported_bytes: number; schema_head: string | null; known_head: string } }
  | { ok: false; name: string; message: string; code?: string; schemaHead?: string; knownHead?: string }

async function exportDatabase(page: Page): Promise<Exported> {
  const out = await page.evaluate(async () => {
    const runtime = (
      window as unknown as {
        forgeRuntime: {
          exportDatabase(): Promise<{
            bytes: Uint8Array
            schema_head: string | null
            known_head: string
            bundle_version: string
          }>
        }
      }
    ).forgeRuntime
    const { bytes, ...rest } = await runtime.exportDatabase()
    let s = ''
    for (const b of bytes) s += String.fromCharCode(b)
    return { ...rest, b64: btoa(s) }
  })
  return { ...out, bytes: Buffer.from(out.b64, 'base64') }
}

/** Import `bytes`, returning the outcome instead of throwing so a test can look at both. */
async function tryImport(page: Page, bytes: Buffer): Promise<ImportResult> {
  return page.evaluate(async (b64) => {
    const runtime = (window as unknown as { forgeRuntime: { importDatabase(b: Uint8Array): Promise<any> } })
      .forgeRuntime
    try {
      const info = await runtime.importDatabase(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)))
      return { ok: true as const, info }
    } catch (e) {
      const err = e as Error & { code?: string; schemaHead?: string; knownHead?: string }
      return {
        ok: false as const,
        name: err.name,
        message: err.message,
        code: err.code,
        schemaHead: err.schemaHead,
        knownHead: err.knownHead,
      }
    }
  }, bytes.toString('base64'))
}

/** Rename every occurrence of `from` in `bytes` to `to`, which must have the same length. */
function rename(bytes: Buffer, from: string, to: string): Buffer {
  expect(to.length).toBe(from.length)
  const out = Buffer.from(bytes)
  let at = out.indexOf(from)
  let count = 0
  while (at !== -1) {
    out.write(to, at, 'latin1')
    count++
    at = out.indexOf(from, at + to.length)
  }
  expect(count, `"${from}" in the exported file`).toBeGreaterThan(0)
  return out
}

async function createOrg(page: Page, name: string) {
  return inPage<{ id: string }>(page, `return ok(await forge.organizations.create({ name: '${name}' }))`)
}

async function orgNames(page: Page): Promise<string[]> {
  return inPage<string[]>(page, `return ok(await forge.organizations.list()).map((o) => o.name)`)
}

test('an export is a rollback-journal file that names its schema head and bundle', async ({ page }) => {
  await openApp(page, '/')
  await createOrg(page, 'Journal marker one')
  const first = await exportDatabase(page)

  // The SQLite header, then file-format bytes 18 and 19: 1,1 is a rollback journal, 2,2 is WAL.
  expect(first.bytes.subarray(0, 16).toString('latin1')).toBe('SQLite format 3\0')
  expect([first.bytes[18], first.bytes[19]], 'header bytes 18-19 of the first export').toEqual([1, 1])

  expect(first.known_head).toMatch(/^\d{3}_\w+$/)
  expect(first.schema_head).toBe(first.known_head)
  expect(first.bundle_version).toMatch(/^\d+\.\d+\.\d+/)

  // More writes, then another export: still a rollback-journal file with the same head.
  await createOrg(page, 'Journal marker two')
  const second = await exportDatabase(page)
  expect([second.bytes[18], second.bytes[19]], 'header bytes 18-19 after more writes').toEqual([1, 1])
  expect(second.schema_head).toBe(first.schema_head)

  // `start` reported the same heads when the runtime opened the database.
  const info = await page.evaluate(
    () =>
      (
        window as unknown as {
          forgeRuntime: { status(): { state: string; info?: Record<string, unknown> } }
        }
      ).forgeRuntime.status().info,
  )
  expect(info).toMatchObject({
    schema_head: first.schema_head,
    known_head: first.known_head,
    bundle_version: first.bundle_version,
  })
})

test('a database from a newer bundle is refused and the live data is untouched', async ({ page }) => {
  await openApp(page, '/')
  await createOrg(page, 'Newer-refused marker')
  const exported = await exportDatabase(page)

  const newerHead = '999' + exported.known_head.slice(3)
  const fromTheFuture = rename(exported.bytes, exported.known_head, newerHead)
  const result = await tryImport(page, fromTheFuture)

  expect(result.ok).toBe(false)
  if (result.ok) return
  expect(result.name).toBe('ImportRefusedError')
  expect(result.code).toBe('schema_too_new')
  expect(result.schemaHead).toBe(newerHead)
  expect(result.knownHead).toBe(exported.known_head)
  expect(result.message).toContain('current data kept')
  expect(result.message).toContain('newer version of Forge')

  // The database is still open, still holds what it held, and still takes writes.
  expect(await orgNames(page)).toContain('Newer-refused marker')
  await createOrg(page, 'After the refusal')
  expect(await orgNames(page)).toEqual(expect.arrayContaining(['Newer-refused marker', 'After the refusal']))

  // And it is still what is stored: a reload opens the same data.
  await page.reload()
  await openApp(page, '/')
  expect(await orgNames(page)).toEqual(expect.arrayContaining(['Newer-refused marker', 'After the refusal']))
})

test('a file that is not a database is refused with its own code', async ({ page }) => {
  await openApp(page, '/')
  await createOrg(page, 'Not-sqlite marker')

  const result = await tryImport(page, Buffer.from('this is not a SQLite database file at all'))
  expect(result.ok).toBe(false)
  if (result.ok) return
  expect(result.name).toBe('ImportRefusedError')
  expect(result.code).toBe('not_sqlite')
  expect(await orgNames(page)).toContain('Not-sqlite marker')
})

test('a file that is accepted but will not open is rolled back to the old data', async ({ page }) => {
  await openApp(page, '/')
  await createOrg(page, 'Rollback marker')
  const exported = await exportDatabase(page)

  // Forget that the first migration ran: opening the file replays it over the existing tables,
  // which fails. The head is unchanged, so the file passes the schema check and is really
  // swapped in before the failure shows.
  const unopenable = rename(exported.bytes, '001_initial', '001_initiaX')
  const result = await tryImport(page, unopenable)

  expect(result.ok).toBe(false)
  if (result.ok) return
  expect(result.name).not.toBe('ImportRefusedError')
  expect(result.message).toContain('current data kept')
  expect(result.message).toContain('could not open the imported database')

  // The previous database is back, open, and writable.
  expect(await orgNames(page)).toContain('Rollback marker')
  await createOrg(page, 'After the rollback')
  await page.reload()
  await openApp(page, '/')
  expect(await orgNames(page)).toEqual(expect.arrayContaining(['Rollback marker', 'After the rollback']))
})

test('importing a current export reports the schema head it ends up with', async ({ page }) => {
  await openApp(page, '/')
  await createOrg(page, 'Round-trip marker')
  const exported = await exportDatabase(page)

  const result = await tryImport(page, exported.bytes)
  expect(result.ok).toBe(true)
  if (!result.ok) return
  expect(result.info.imported_bytes).toBe(exported.bytes.length)
  expect(result.info.schema_head).toBe(exported.known_head)
  expect(result.info.known_head).toBe(exported.known_head)
  expect(await orgNames(page)).toContain('Round-trip marker')

  // The re-imported database exports as a rollback-journal file too.
  const again = await exportDatabase(page)
  expect([again.bytes[18], again.bytes[19]]).toEqual([1, 1])
})
