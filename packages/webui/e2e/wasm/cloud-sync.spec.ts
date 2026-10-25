import { expect, test, type Browser, type BrowserContext, type Page, type TestInfo } from '@playwright/test'
import { E2E_DATA_URL, E2E_ID_URL } from './e2e-hosts'
import { inPage, openApp } from './helpers'

/**
 * Cloud sync in the browser-first app (resu-mx/forge#256).
 *
 * Both services are stubbed with `page.route`, on hosts that exist nowhere else (see
 * `e2e-hosts.ts`). The snapshot stub keeps one object in memory and enforces what the real service
 * enforces: the SQLite header and rollback-journal bytes, exactly one precondition, the headers a
 * snapshot must declare, and the CORS allow-list. So a request the real service would refuse fails
 * here too. Each browser context is one device: its own OPFS database and its own localStorage,
 * talking to the same cloud.
 *
 * Contract: the snapshot-API spec. HEAD/GET/PUT/DELETE on /v1/snapshot, ETag preconditions,
 * 404 when nothing is stored, 204 on delete.
 */

const ME = {
  user: { id: 'user-1', email: 'ada@example.test', name: 'Ada Lovelace' },
  storageId: '5f0e7c2a-0000-4000-8000-0000000000aa',
}

interface Stored {
  bytes: Buffer
  etag: string
  schemaHead: string
  bundleVersion: string
}

interface PutSeen {
  ifMatch?: string
  ifNoneMatch?: string
  schemaHead?: string
  bundleVersion?: string
  contentType?: string
  bytes: Buffer
}

/** The snapshot service, in memory, shared by every device in a test. */
class CloudServer {
  object: Stored | null = null
  /** `METHOD` of every request that reached the stub, in order, preflights excluded. */
  log: string[] = []
  puts: PutSeen[] = []
  private n = 0

  /** Count of requests so far: take it before an action and compare after. */
  get length() {
    return this.log.length
  }

  respond(method: string, headers: Record<string, string>, body: Buffer | null) {
    this.log.push(method)
    const cur = this.object
    const json = (status: number, payload: unknown) => ({
      status,
      headers: { 'content-type': 'application/json' },
      body: Buffer.from(JSON.stringify(payload)),
    })

    if (method === 'HEAD' || method === 'GET') {
      if (!cur) return method === 'GET' ? json(404, { error: 'not_found' }) : { status: 404, headers: {}, body: undefined }
      return {
        status: 200,
        headers: {
          'content-type': 'application/vnd.sqlite3',
          etag: cur.etag,
          'x-forge-schema-head': cur.schemaHead,
          'x-forge-bundle-version': cur.bundleVersion,
          'last-modified': 'Sat, 25 Oct 2026 12:00:00 GMT',
        },
        body: method === 'GET' ? cur.bytes : undefined,
      }
    }

    if (method === 'DELETE') {
      this.object = null
      return { status: 204, headers: {}, body: undefined }
    }

    if (method === 'PUT') {
      const bytes = body ?? Buffer.alloc(0)
      this.puts.push({
        ifMatch: headers['if-match'],
        ifNoneMatch: headers['if-none-match'],
        schemaHead: headers['x-forge-schema-head'],
        bundleVersion: headers['x-forge-bundle-version'],
        contentType: headers['content-type'],
        bytes,
      })
      const magic = Buffer.from('SQLite format 3\0')
      if (bytes.length < 20 || !bytes.subarray(0, 16).equals(magic) || bytes[18] !== 1 || bytes[19] !== 1) {
        return json(422, { error: 'invalid_snapshot' })
      }
      if (!headers['x-forge-schema-head'] || !headers['x-forge-bundle-version']) return json(422, { error: 'invalid_snapshot' })
      const ifMatch = headers['if-match']
      const ifNoneMatch = headers['if-none-match']
      if ((ifMatch === undefined) === (ifNoneMatch === undefined)) return json(428, { error: 'precondition_required' })
      if (ifNoneMatch !== undefined && cur) return json(412, { error: 'precondition_failed' })
      if (ifMatch !== undefined && cur?.etag !== ifMatch) return json(412, { error: 'precondition_failed' })
      const etag = `"v${++this.n}"`
      this.object = {
        bytes,
        etag,
        schemaHead: headers['x-forge-schema-head'],
        bundleVersion: headers['x-forge-bundle-version'],
      }
      return { status: 200, headers: { 'content-type': 'application/json', etag }, body: Buffer.from(JSON.stringify({ etag })) }
    }

    return json(405, { error: 'method_not_allowed' })
  }
}

/** Whether this device is signed in: the session cookie, as far as the stubs are concerned. */
interface Account {
  signedIn: boolean
}

/** The account service: `GET /api/me`, and the CORS a credentialed fetch needs. */
async function stubAccountHost(context: BrowserContext, account: Account) {
  await context.route(`${E2E_ID_URL}/**`, async (route) => {
    const req = route.request()
    const origin = req.headers()['origin']
    const cors = origin
      ? { 'Access-Control-Allow-Origin': origin, 'Access-Control-Allow-Credentials': 'true', Vary: 'Origin' }
      : {}
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers: cors })
    if (req.method() === 'GET' && new URL(req.url()).pathname === '/api/me') {
      return route.fulfill({
        status: account.signedIn ? 200 : 401,
        contentType: 'application/json',
        headers: cors,
        body: JSON.stringify(account.signedIn ? ME : { error: 'unauthorized' }),
      })
    }
    return route.fulfill({ status: 404, headers: cors, body: '{}' })
  })
}

/** The snapshot service, with the real one's CORS allow-list and its 401 for a signed-out device. */
async function stubCloudHost(context: BrowserContext, cloud: CloudServer, account: Account) {
  await context.route(`${E2E_DATA_URL}/**`, async (route) => {
    const req = route.request()
    const origin = req.headers()['origin']
    const cors: Record<string, string> = origin
      ? {
          'Access-Control-Allow-Origin': origin,
          'Access-Control-Allow-Credentials': 'true',
          'Access-Control-Expose-Headers': 'ETag, Last-Modified, X-Forge-Schema-Head, X-Forge-Bundle-Version, Retry-After',
          'Cache-Control': 'private, no-store',
          Vary: 'Origin',
        }
      : {}
    if (req.method() === 'OPTIONS') {
      return route.fulfill({
        status: 204,
        headers: {
          ...cors,
          'Access-Control-Allow-Methods': 'GET, HEAD, PUT, DELETE, OPTIONS',
          'Access-Control-Allow-Headers': 'If-Match, If-None-Match, Content-Type, X-Forge-Schema-Head, X-Forge-Bundle-Version',
        },
      })
    }
    if (new URL(req.url()).pathname !== '/v1/snapshot') return route.fulfill({ status: 404, headers: cors, body: '{}' })
    if (!account.signedIn) {
      return route.fulfill({ status: 401, contentType: 'application/json', headers: cors, body: '{"error":"unauthorized"}' })
    }
    const res = cloud.respond(req.method(), req.headers(), req.postDataBuffer())
    return route.fulfill({ status: res.status, headers: { ...cors, ...res.headers }, body: res.body })
  })
}

interface Device {
  context: BrowserContext
  page: Page
  account: Account
}

/** A fresh browser profile (its own database and localStorage) talking to `cloud`. */
async function newDevice(browser: Browser, testInfo: TestInfo, cloud: CloudServer, signedIn: boolean): Promise<Device> {
  const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL, acceptDownloads: true })
  const account: Account = { signedIn }
  await stubAccountHost(context, account)
  await stubCloudHost(context, cloud, account)
  return { context, page: await context.newPage(), account }
}

const addSource = (page: Page, title: string) =>
  inPage(page, `return ok(await forge.sources.create({ title: '${title}', description: 'Notes.', source_type: 'general' }))`)

const sourceTitles = (page: Page) =>
  inPage<string[]>(page, 'return ok(await forge.sources.list()).map((s) => s.title).sort()')

const indicator = (page: Page) => page.getByTestId('sync-indicator')

/** Use the app signed out, then sign in and load again: what a person with local data does. */
async function signInWithData(device: Device, title: string) {
  await openApp(device.page, '/')
  await addSource(device.page, title)
  device.account.signedIn = true
  await device.page.reload()
}

/** A device that has uploaded `title` as the cloud copy. Leaves it open on `/`. */
async function deviceWithCloudCopy(browser: Browser, testInfo: TestInfo, cloud: CloudServer, title = 'Babbage Notes') {
  const device = await newDevice(browser, testInfo, cloud, false)
  await signInWithData(device, title)
  await expect(indicator(device.page)).toContainText('Synced')
  return device
}

test.describe('cloud sync', () => {
  test('signing in with data here and nothing in the cloud uploads it, create-only', async ({ browser }, testInfo) => {
    const cloud = new CloudServer()
    const device = await newDevice(browser, testInfo, cloud, false)

    await signInWithData(device, 'Babbage Notes')
    await expect(indicator(device.page)).toContainText('Synced')

    expect(cloud.log).toEqual(['HEAD', 'PUT'])
    const put = cloud.puts[0]
    expect(put.ifNoneMatch).toBe('*')
    expect(put.ifMatch).toBeUndefined()
    expect(put.contentType).toBe('application/vnd.sqlite3')
    expect(put.schemaHead).toBeTruthy()
    expect(put.bundleVersion).toBeTruthy()
    // What left the browser is a whole SQLite file in rollback-journal mode.
    expect(put.bytes.subarray(0, 15).toString()).toBe('SQLite format 3')
    expect([put.bytes[18], put.bytes[19]]).toEqual([1, 1])
    expect(cloud.object).not.toBeNull()
  })

  test('a change made after syncing is pushed when the page is hidden, with the ETag it last saw', async ({ browser }, testInfo) => {
    const cloud = new CloudServer()
    const device = await deviceWithCloudCopy(browser, testInfo, cloud)
    const before = cloud.object!
    expect(cloud.puts).toHaveLength(1)

    await addSource(device.page, 'Second Notes') // announces forge:changed; the push waits for a quiet period
    expect(cloud.puts).toHaveLength(1)

    // Hide the page: what a tab switch or a close does. The push goes out at once.
    await device.page.evaluate(() => {
      Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true })
      document.dispatchEvent(new Event('visibilitychange'))
    })
    await expect.poll(() => cloud.puts.length).toBe(2)

    expect(cloud.puts[1].ifMatch).toBe(before.etag)
    expect(cloud.puts[1].ifNoneMatch).toBeUndefined()
    expect(cloud.puts[1].bytes.equals(before.bytes)).toBe(false)
    expect(cloud.object!.etag).not.toBe(before.etag)
    await expect(indicator(device.page)).toContainText('Synced')
  })

  test('signed out, nothing is requested', async ({ browser }, testInfo) => {
    const cloud = new CloudServer()
    cloud.object = { bytes: Buffer.alloc(0), etag: '"x"', schemaHead: 'x', bundleVersion: 'x' }
    const device = await newDevice(browser, testInfo, cloud, false)

    await openApp(device.page, '/settings/storage')
    await addSource(device.page, 'Local only')
    await expect(device.page.getByTestId('sync-sign-in')).toBeVisible()
    await expect(indicator(device.page)).toHaveCount(0)

    expect(cloud.log).toEqual([])
  })

  test('a second device is asked, takes the cloud copy, and ends up with the data', async ({ browser }, testInfo) => {
    const cloud = new CloudServer()
    await deviceWithCloudCopy(browser, testInfo, cloud)
    const second = await newDevice(browser, testInfo, cloud, true)

    await openApp(second.page, '/')
    const dialog = second.page.getByRole('dialog', { name: 'Your account already has cloud data' })
    await expect(dialog).toBeVisible()
    // Nothing has been replaced yet.
    expect(await sourceTitles(second.page)).toEqual([])
    expect(cloud.puts).toHaveLength(1)

    const reloaded = second.page.waitForEvent('load')
    await dialog.getByTestId('sync-choice-use-cloud').click()
    await reloaded
    await openApp(second.page, '/')

    expect(await sourceTitles(second.page)).toEqual(['Babbage Notes'])
    await expect(indicator(second.page)).toContainText('Synced')
    await expect(second.page.getByTestId('sync-conflict')).toHaveCount(0)
  })

  test('the dialog offers an export first, and keeping this browser replaces the cloud copy', async ({ browser }, testInfo) => {
    const cloud = new CloudServer()
    await deviceWithCloudCopy(browser, testInfo, cloud)
    const before = cloud.object!.etag
    const second = await newDevice(browser, testInfo, cloud, false)
    await signInWithData(second, 'Lovelace Notes')

    const dialog = second.page.getByRole('dialog', { name: 'Your account already has cloud data' })
    await expect(dialog).toBeVisible()
    const download = second.page.waitForEvent('download')
    await dialog.getByTestId('sync-export-first').click()
    expect((await download).suggestedFilename()).toMatch(/^forge-\d{4}-\d{2}-\d{2}\.db$/)
    // Exporting does not answer the question.
    await expect(dialog).toBeVisible()
    expect(cloud.puts).toHaveLength(1)

    await dialog.getByTestId('sync-choice-replace-cloud').click()
    await expect(indicator(second.page)).toContainText('Synced')

    expect(cloud.puts).toHaveLength(2)
    expect(cloud.puts[1].ifMatch).toBe(before)
    expect(cloud.object!.etag).not.toBe(before)
    expect(await sourceTitles(second.page)).toEqual(['Lovelace Notes'])
  })

  test('"Decide later" puts the dialog away and the sidebar brings it back', async ({ browser }, testInfo) => {
    const cloud = new CloudServer()
    await deviceWithCloudCopy(browser, testInfo, cloud)
    const second = await newDevice(browser, testInfo, cloud, false)
    await signInWithData(second, 'Lovelace Notes')

    const dialog = second.page.getByRole('dialog', { name: 'Your account already has cloud data' })
    await expect(dialog).toBeVisible()
    await dialog.getByTestId('sync-decide-later').click()
    await expect(dialog).toHaveCount(0)
    await expect(indicator(second.page)).toContainText('Choose which copy to keep')
    expect(cloud.puts).toHaveLength(1) // nothing was uploaded, nothing replaced

    await indicator(second.page).click()
    await expect(dialog).toBeVisible()
  })

  test('Delete cloud copy deletes it, turns sync off, and nothing is requested afterwards', async ({ browser }, testInfo) => {
    const cloud = new CloudServer()
    const device = await deviceWithCloudCopy(browser, testInfo, cloud)

    await openApp(device.page, '/settings/storage')
    await expect(device.page.getByTestId('sync-status')).toContainText('Synced')
    await device.page.getByTestId('sync-delete').click()
    const confirm = device.page.getByRole('alertdialog', { name: 'Delete your cloud copy?' })
    await expect(confirm).toContainText('permanently deletes')
    await confirm.getByRole('button', { name: 'Delete cloud copy' }).click()

    await expect(device.page.getByTestId('sync-status')).toContainText('Cloud sync is off')
    await expect(device.page.getByText('Your cloud copy was deleted')).toBeVisible()
    expect(cloud.object).toBeNull()
    expect(cloud.log.at(-1)).toBe('DELETE')
    expect(await sourceTitles(device.page)).toEqual(['Babbage Notes']) // this browser's data is untouched

    // A reload does not bring sync back: no request at all.
    const seen = cloud.length
    await device.page.reload()
    await expect(device.page.getByTestId('sync-status')).toContainText('Cloud sync is off')
    expect(cloud.length).toBe(seen)

    // Turning it on again uploads, since the cloud has nothing.
    await device.page.getByTestId('sync-toggle').click()
    await expect(device.page.getByTestId('sync-status')).toContainText('Synced')
    expect(cloud.log.slice(seen)).toEqual(['HEAD', 'PUT'])
    expect(cloud.object).not.toBeNull()
  })

  test('another device that still syncs asks, rather than putting the deleted copy back', async ({ browser }, testInfo) => {
    const cloud = new CloudServer()
    const first = await deviceWithCloudCopy(browser, testInfo, cloud)

    // The second device takes the cloud copy, so it is bound, clean, and holds the data.
    const second = await newDevice(browser, testInfo, cloud, true)
    await openApp(second.page, '/')
    const reloaded = second.page.waitForEvent('load')
    await second.page.getByTestId('sync-choice-use-cloud').click()
    await reloaded
    await openApp(second.page, '/')
    await expect(indicator(second.page)).toContainText('Synced')

    // The first device deletes the cloud copy.
    await openApp(first.page, '/settings/storage')
    await first.page.getByTestId('sync-delete').click()
    await first.page
      .getByRole('alertdialog', { name: 'Delete your cloud copy?' })
      .getByRole('button', { name: 'Delete cloud copy' })
      .click()
    await expect(first.page.getByTestId('sync-status')).toContainText('Cloud sync is off')
    const putsAfterDelete = cloud.puts.length

    // The second opens Forge: it finds the copy gone and asks.
    await second.page.reload()
    const dialog = second.page.getByRole('dialog', { name: 'Your cloud copy was deleted' })
    await expect(dialog).toBeVisible()
    expect(cloud.puts).toHaveLength(putsAfterDelete)
    expect(cloud.object).toBeNull()
    await expect(dialog.getByTestId('sync-choice-use-cloud')).toHaveCount(0) // nothing to use

    await dialog.getByTestId('sync-choice-turn-off').click()
    await expect(indicator(second.page)).toContainText('Cloud sync is off')
    expect(cloud.puts).toHaveLength(putsAfterDelete)
    expect(cloud.object).toBeNull()
    expect(await sourceTitles(second.page)).toEqual(['Babbage Notes'])
  })

  test('a second device can upload again after the copy was deleted, if the person says so', async ({ browser }, testInfo) => {
    const cloud = new CloudServer()
    const first = await deviceWithCloudCopy(browser, testInfo, cloud)
    const second = await newDevice(browser, testInfo, cloud, true)
    await openApp(second.page, '/')
    const reloaded = second.page.waitForEvent('load')
    await second.page.getByTestId('sync-choice-use-cloud').click()
    await reloaded
    await openApp(second.page, '/')
    await expect(indicator(second.page)).toContainText('Synced')

    await openApp(first.page, '/settings/storage')
    await first.page.getByTestId('sync-delete').click()
    await first.page
      .getByRole('alertdialog', { name: 'Delete your cloud copy?' })
      .getByRole('button', { name: 'Delete cloud copy' })
      .click()
    await expect(first.page.getByTestId('sync-status')).toContainText('Cloud sync is off')

    await second.page.reload()
    const dialog = second.page.getByRole('dialog', { name: 'Your cloud copy was deleted' })
    await dialog.getByTestId('sync-choice-replace-cloud').click()
    await expect(indicator(second.page)).toContainText('Synced')

    expect(cloud.object).not.toBeNull()
    expect(cloud.puts.at(-1)!.ifNoneMatch).toBe('*') // created anew, not a replace
  })
})
