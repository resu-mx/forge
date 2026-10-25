import { describe, expect, test } from 'bun:test'
import { createSyncEngine, parseRetryAfter } from './engine'
import {
  SYNC_STORAGE_KEY,
  type ExportResult,
  type ImportResult,
  type SyncClock,
  type SyncEvent,
  type SyncFetch,
  type SyncOptions,
  type SyncRecord,
  type SyncRuntime,
  type SyncStorage,
} from './types'

const A = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'
const B = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'
const HEAD = '055_dataset_meta'
const T0 = 1_700_000_000_000

// ── Fakes ───────────────────────────────────────────────────────────

/** Let everything that is already runnable (promise callbacks, fake fetches) finish. */
const settle = () => new Promise<void>((resolve) => setTimeout(resolve, 0))

class FakeClock implements SyncClock {
  private t = T0
  private nextId = 1
  private timers: { id: number; at: number; fn: () => void }[] = []

  now() {
    return this.t
  }
  setTimeout(fn: () => void, ms: number) {
    const id = this.nextId++
    this.timers.push({ id, at: this.t + ms, fn })
    return id
  }
  clearTimeout(handle: unknown) {
    this.timers = this.timers.filter((timer) => timer.id !== handle)
  }
  /** Timers that have not fired. */
  get pending() {
    return this.timers.length
  }
  /** Move time on, firing each timer that falls due and letting its work finish. */
  async advance(ms: number) {
    const end = this.t + ms
    for (;;) {
      const due = this.timers.filter((timer) => timer.at <= end).sort((a, b) => a.at - b.at || a.id - b.id)[0]
      if (!due) break
      this.timers = this.timers.filter((timer) => timer !== due)
      this.t = Math.max(this.t, due.at)
      due.fn()
      await settle()
    }
    this.t = end
  }
}

interface Sent {
  /** When it was sent, on the fake clock. */
  at: number
  method: string
  headers: Record<string, string>
  body?: Uint8Array
  credentials?: RequestCredentials
}

interface Stored {
  bytes: Uint8Array
  etag: string
  schemaHead: string
  bundleVersion: string
}

/** The snapshot API: one object, ETag preconditions, the headers the contract names. */
class FakeServer {
  object: Stored | null = null
  requests: Sent[] = []
  /** Answers for the next requests, in order, ahead of the real handler. An Error is a network failure. */
  script: Array<Response | Error> = []
  /** Runs when a request of that method arrives, before it is answered. */
  hooks: { PUT?: () => void; GET?: () => void } = {}
  now: () => number = () => T0
  private counter = 0

  /** Store `tag` as the cloud copy (another device's push). */
  put(tag: string, schemaHead = HEAD): string {
    const etag = `"v${++this.counter}"`
    this.object = { bytes: dbBytes(tag), etag, schemaHead, bundleVersion: '0.1.0' }
    return etag
  }

  get etag() {
    return this.object?.etag
  }

  count(method: string) {
    return this.requests.filter((r) => r.method === method).length
  }

  fetch: SyncFetch = async (_url, init) => {
    const method = init.method ?? 'GET'
    const sent: Sent = {
      at: this.now(),
      method,
      headers: { ...(init.headers as Record<string, string> | undefined) },
      body: init.body as Uint8Array | undefined,
      credentials: init.credentials,
    }
    this.requests.push(sent)
    if (method === 'PUT') this.hooks.PUT?.()
    if (method === 'GET') this.hooks.GET?.()

    const scripted = this.script.shift()
    if (scripted instanceof Error) throw scripted
    if (scripted) return scripted
    return this.handle(sent)
  }

  private handle({ method, headers, body }: Sent): Response {
    const current = this.object
    if (method === 'HEAD' || method === 'GET') {
      if (!current) return new Response(method === 'GET' ? '{"error":"not_found"}' : null, { status: 404 })
      return new Response(method === 'GET' ? (current.bytes as BodyInit) : null, {
        status: 200,
        headers: {
          ETag: current.etag,
          'X-Forge-Schema-Head': current.schemaHead,
          'X-Forge-Bundle-Version': current.bundleVersion,
          'Last-Modified': 'Sat, 24 Oct 2026 12:00:00 GMT',
        },
      })
    }
    const ifMatch = headers['If-Match']
    const ifNoneMatch = headers['If-None-Match']
    if ((ifMatch === undefined) === (ifNoneMatch === undefined)) {
      return new Response('{"error":"precondition_required"}', { status: 428 })
    }
    if (ifNoneMatch !== undefined && current) return new Response('{"error":"precondition_failed"}', { status: 412 })
    if (ifMatch !== undefined && current?.etag !== ifMatch) {
      return new Response('{"error":"precondition_failed"}', { status: 412 })
    }
    const etag = `"v${++this.counter}"`
    this.object = {
      bytes: body as Uint8Array,
      etag,
      schemaHead: headers['X-Forge-Schema-Head'],
      bundleVersion: headers['X-Forge-Bundle-Version'],
    }
    return new Response(JSON.stringify({ etag }), { status: 200, headers: { ETag: etag } })
  }
}

/** A SQLite-looking file: the header, the journal bytes, then `tag` so files differ. */
function dbBytes(tag: string, journal: [number, number] = [1, 1]): Uint8Array {
  const out = new Uint8Array(100 + tag.length)
  out.set(new TextEncoder().encode('SQLite format 3\0'))
  out[18] = journal[0]
  out[19] = journal[1]
  out.set(new TextEncoder().encode(tag), 100)
  return out
}

class FakeRuntime implements SyncRuntime {
  known: string | undefined = HEAD
  exportResult: ExportResult = { bytes: dbBytes('local'), schema_head: HEAD, known_head: HEAD, bundle_version: '0.1.0' }
  exports = 0
  imported: Uint8Array[] = []
  importError: Error | undefined
  /** Runs inside `importDatabase`, where the real runtime announces `forge:changed`. */
  onImport: (() => void) | undefined
  /** Runs inside `exportDatabase`, where a user can keep editing. */
  onExport: (() => void) | undefined

  async exportDatabase() {
    this.exports++
    this.onExport?.()
    return this.exportResult
  }
  async importDatabase(bytes: Uint8Array): Promise<ImportResult> {
    this.onImport?.()
    if (this.importError) throw this.importError
    this.imported.push(bytes)
    return { imported_bytes: bytes.length, migrations: 1, schema_head: HEAD, known_head: HEAD, bundle_version: '0.1.0' }
  }
  knownHead() {
    return this.known
  }
}

interface Setup {
  record?: Partial<SyncRecord>
  /** Seed the cloud copy: the tag of its bytes. Its ETag is `"v1"`. */
  cloud?: string
  hasData?: boolean
  owner?: boolean
  options?: Partial<SyncOptions>
}

function make(setup: Setup = {}) {
  const clock = new FakeClock()
  const server = new FakeServer()
  server.now = () => clock.now()
  if (setup.cloud) server.put(setup.cloud)
  const runtime = new FakeRuntime()
  const events: SyncEvent[] = []
  const state = { owner: setup.owner ?? true, hasData: setup.hasData ?? false }
  let stored: SyncRecord | null = setup.record
    ? { boundStorageId: null, lastSyncedEtag: null, dirty: false, lastSyncAt: null, ...setup.record }
    : null
  const storage: SyncStorage = {
    get: () => (stored ? { ...stored } : null),
    set: (record) => {
      stored = { ...record }
    },
  }
  const engine = createSyncEngine({
    baseUrl: 'https://data.example/',
    fetch: server.fetch,
    runtime,
    storage,
    clock,
    isOwner: () => state.owner,
    localHasData: async () => state.hasData,
    onEvent: (event) => events.push(event),
    options: setup.options,
  })
  return {
    engine,
    clock,
    server,
    runtime,
    events,
    state,
    record: () => stored,
    status: () => engine.state().status,
    errors: () => events.filter((e) => e.type === 'error').map((e) => (e as { error: unknown }).error),
    puts: () => server.requests.filter((r) => r.method === 'PUT'),
  }
}

/** A browser bound to account A whose last sync saw ETag "v1". */
const boundToA = (dirty: boolean): SyncRecord => ({
  boundStorageId: A,
  lastSyncedEtag: '"v1"',
  dirty,
  lastSyncAt: null,
})

// ── The decision table, end to end ──────────────────────────────────

describe('on start: the table rows', () => {
  test('none + never bound + local has data: first upload with If-None-Match, then bound', async () => {
    const h = make({ hasData: true })
    await h.engine.setAccount(A)

    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD', 'PUT'])
    const put = h.puts()[0]
    expect(put.headers['If-None-Match']).toBe('*')
    expect(put.headers['If-Match']).toBeUndefined()
    expect(put.headers['X-Forge-Schema-Head']).toBe(HEAD)
    expect(put.headers['X-Forge-Bundle-Version']).toBe('0.1.0')
    expect(put.headers['Content-Type']).toBe('application/vnd.sqlite3')
    expect(put.body).toEqual(h.runtime.exportResult.bytes)

    expect(h.record()).toEqual({ boundStorageId: A, lastSyncedEtag: h.server.etag!, dirty: false, lastSyncAt: T0 })
    expect(h.status()).toBe('synced')
    expect(h.events).toContainEqual({ type: 'synced', action: 'first-upload', etag: h.server.etag!, at: T0 })
  })

  test('none + never bound + local empty: bind, nothing uploaded', async () => {
    const h = make({ hasData: false })
    await h.engine.setAccount(A)

    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD'])
    expect(h.record()).toEqual({ boundStorageId: A, lastSyncedEtag: null, dirty: false, lastSyncAt: T0 })
    expect(h.status()).toBe('synced')
  })

  test('exists + never bound: asks the user, touches nothing', async () => {
    const h = make({ cloud: 'cloud', hasData: true })
    await h.engine.setAccount(A)

    expect(h.status()).toBe('conflict')
    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD'])
    expect(h.runtime.imported).toEqual([])
    expect(h.runtime.exports).toBe(0)
    expect(h.record()).toBeNull()
    const conflict = h.engine.state().conflict!
    expect(conflict).toMatchObject({ reason: 'unbound', storageId: A, boundStorageId: null })
    expect(conflict.remote).toMatchObject({ kind: 'exists', etag: '"v1"', schemaHead: HEAD, bundleVersion: '0.1.0' })
    expect(h.events).toContainEqual({ type: 'conflict', conflict })
  })

  test('ETag equals last synced + dirty: push with If-Match', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    await h.engine.setAccount(A)

    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD', 'PUT'])
    expect(h.puts()[0].headers['If-Match']).toBe('"v1"')
    expect(h.puts()[0].headers['If-None-Match']).toBeUndefined()
    expect(h.record()).toEqual({ boundStorageId: A, lastSyncedEtag: '"v2"', dirty: false, lastSyncAt: T0 })
    expect(h.events).toContainEqual({ type: 'synced', action: 'push', etag: '"v2"', at: T0 })
  })

  test('ETag equals last synced + clean: nothing to do', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)

    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD'])
    expect(h.record()).toEqual({ ...boundToA(false), lastSyncAt: T0 })
    expect(h.events).toContainEqual({ type: 'synced', action: 'noop', etag: '"v1"', at: T0 })
    expect(h.runtime.exports + h.runtime.imported.length).toBe(0)
  })

  test('ETag differs + clean: pull, import, record the new ETag, then ask for a reload', async () => {
    const h = make({ record: boundToA(false) })
    h.server.put('first') // "v1"
    h.server.put('second') // "v2": another device pushed
    await h.engine.setAccount(A)

    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD', 'GET'])
    expect(h.runtime.imported).toEqual([dbBytes('second')])
    expect(h.record()).toEqual({ boundStorageId: A, lastSyncedEtag: '"v2"', dirty: false, lastSyncAt: T0 })
    const types = h.events.map((e) => e.type)
    expect(types.indexOf('synced')).toBeGreaterThan(-1)
    // The record is written before the page is told to reload, so the next page starts bound.
    expect(types.indexOf('reload')).toBeGreaterThan(types.indexOf('synced'))
    expect(h.status()).toBe('synced')
  })

  test('ETag differs + dirty: asks the user, keeps both versions', async () => {
    const h = make({ record: boundToA(true) })
    h.server.put('first')
    h.server.put('second')
    await h.engine.setAccount(A)

    expect(h.status()).toBe('conflict')
    expect(h.engine.state().conflict).toMatchObject({ reason: 'diverged', boundStorageId: A })
    expect(h.runtime.imported).toEqual([])
    expect(h.puts()).toHaveLength(0)
    expect(h.record()).toEqual(boundToA(true))
  })

  test('a deleted cloud copy while bound: the local data goes up again', async () => {
    const h = make({ record: boundToA(false), hasData: true })
    await h.engine.setAccount(A)

    expect(h.puts()[0].headers['If-None-Match']).toBe('*')
    expect(h.record()).toMatchObject({ boundStorageId: A, lastSyncedEtag: h.server.etag, dirty: false })
  })
})

describe('a 412 re-enters the table', () => {
  test('on a push: the cloud copy moved, and this browser is dirty, so the user is asked', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    h.server.hooks.PUT = () => {
      h.server.hooks.PUT = undefined
      h.server.put('another device') // lands between our HEAD and our PUT
    }
    await h.engine.setAccount(A)

    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD', 'PUT', 'HEAD'])
    expect(h.status()).toBe('conflict')
    expect(h.engine.state().conflict).toMatchObject({ reason: 'diverged' })
    expect(h.engine.state().conflict!.remote).toMatchObject({ kind: 'exists', etag: '"v2"' })
    expect(h.errors()).toEqual([])
  })

  test('on a debounced push: straight to a push with the last ETag, and the table on 412', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0

    h.engine.notifyChange()
    h.server.put('another device')
    await h.clock.advance(30_000)

    // No HEAD first: the push goes out with If-Match, is refused, and only then is the cloud checked.
    expect(h.server.requests.map((r) => r.method)).toEqual(['PUT', 'HEAD'])
    expect(h.puts()[0].headers['If-Match']).toBe('"v1"')
    expect(h.status()).toBe('conflict')
  })

  test('on a first upload: someone created the cloud copy first, so the user is asked', async () => {
    const h = make({ hasData: true })
    h.server.hooks.PUT = () => {
      h.server.hooks.PUT = undefined
      h.server.put('created elsewhere')
    }
    await h.engine.setAccount(A)

    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD', 'PUT', 'HEAD'])
    expect(h.engine.state().conflict).toMatchObject({ reason: 'unbound' })
    expect(h.record()).toBeNull()
  })

  test('a table that never settles stops after a few passes', async () => {
    const h = make({ hasData: true, options: { maxReentries: 2 } })
    // Every HEAD finds no cloud copy and every create-only PUT is refused: three passes, then give up.
    for (let pass = 0; pass < 3; pass++) {
      h.server.script.push(new Response(null, { status: 404 }), new Response(null, { status: 412 }))
    }
    await h.engine.setAccount(A)

    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD', 'PUT', 'HEAD', 'PUT', 'HEAD', 'PUT'])
    expect(h.errors()).toEqual([expect.objectContaining({ code: 'conflict_loop' })])
    expect(h.status()).toBe('error')
    expect(h.clock.pending).toBe(0)
  })
})

describe('resolving a conflict', () => {
  test('use-cloud: pulls the cloud copy over this browser, bound and clean, then reloads', async () => {
    const h = make({ cloud: 'cloud', hasData: true })
    await h.engine.setAccount(A)
    expect(h.status()).toBe('conflict')

    await h.engine.resolveConflict('use-cloud')

    expect(h.runtime.imported).toEqual([dbBytes('cloud')])
    expect(h.record()).toEqual({ boundStorageId: A, lastSyncedEtag: '"v1"', dirty: false, lastSyncAt: T0 })
    expect(h.engine.state()).toEqual({ status: 'synced', conflict: null, error: null })
    expect(h.events.at(-1)).toEqual({ type: 'reload' })
  })

  test('use-cloud discards unsaved local changes: that is what the user chose', async () => {
    const h = make({ record: boundToA(true) })
    h.server.put('first')
    h.server.put('second')
    await h.engine.setAccount(A)

    await h.engine.resolveConflict('use-cloud')

    expect(h.runtime.imported).toEqual([dbBytes('second')])
    expect(h.record()).toMatchObject({ lastSyncedEtag: '"v2"', dirty: false })
  })

  test('replace-cloud: uploads this browser over the cloud copy it was shown', async () => {
    const h = make({ cloud: 'cloud', hasData: true })
    await h.engine.setAccount(A)

    await h.engine.resolveConflict('replace-cloud')

    expect(h.puts()).toHaveLength(1)
    expect(h.puts()[0].headers['If-Match']).toBe('"v1"')
    expect(h.runtime.imported).toEqual([])
    expect(h.record()).toEqual({ boundStorageId: A, lastSyncedEtag: '"v2"', dirty: false, lastSyncAt: T0 })
    expect(h.status()).toBe('synced')
  })

  test('replace-cloud does not overwrite a cloud copy the user has not seen', async () => {
    const h = make({ cloud: 'cloud', hasData: true })
    await h.engine.setAccount(A)
    h.server.put('changed again while the dialog was open')

    await h.engine.resolveConflict('replace-cloud')

    expect(h.puts()).toHaveLength(0)
    expect(h.status()).toBe('conflict')
    expect(h.engine.state().conflict!.remote).toMatchObject({ kind: 'exists', etag: '"v2"' })
  })

  test('with no conflict pending it does nothing', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0

    await h.engine.resolveConflict('use-cloud')
    await h.engine.resolveConflict('replace-cloud')

    expect(h.server.requests).toEqual([])
  })

  test('a failed resolution leaves the conflict to try again', async () => {
    const h = make({ cloud: 'cloud', hasData: true })
    await h.engine.setAccount(A)
    h.server.script.push(new Response(null, { status: 200, headers: { ETag: '"v1"' } }), new Response(null, { status: 413 }))

    await h.engine.resolveConflict('replace-cloud')

    expect(h.status()).toBe('error')
    expect(h.engine.state().conflict).not.toBeNull()
    // Not retried on its own: the user chose, the user chooses again.
    expect(h.clock.pending).toBe(0)

    await h.engine.resolveConflict('replace-cloud')
    expect(h.status()).toBe('synced')
    expect(h.engine.state().conflict).toBeNull()
  })
})

describe('switching accounts', () => {
  test('always asks, even when the ETags line up and nothing is dirty', async () => {
    const h = make({ cloud: 'cloud', record: { boundStorageId: B, lastSyncedEtag: '"v1"', dirty: false } })
    await h.engine.setAccount(A)

    expect(h.status()).toBe('conflict')
    expect(h.engine.state().conflict).toMatchObject({ reason: 'account-switch', storageId: A, boundStorageId: B })
    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD'])
    expect(h.runtime.imported).toEqual([])
    expect(h.record()).toMatchObject({ boundStorageId: B })
  })

  test('asks when the new account has nothing stored, and only replace-cloud applies', async () => {
    const h = make({ record: { boundStorageId: B, lastSyncedEtag: '"v1"', dirty: false }, hasData: true })
    await h.engine.setAccount(A)

    expect(h.engine.state().conflict).toMatchObject({ reason: 'account-switch', remote: { kind: 'none' } })
    expect(h.puts()).toHaveLength(0)

    await expect(h.engine.resolveConflict('use-cloud')).rejects.toThrow('no cloud copy')
    expect(h.status()).toBe('conflict')

    await h.engine.resolveConflict('replace-cloud')
    expect(h.puts()[0].headers['If-None-Match']).toBe('*')
    expect(h.record()).toMatchObject({ boundStorageId: A, dirty: false })
  })

  test('use-cloud rebinds to the new account', async () => {
    const h = make({ cloud: 'theirs', record: { boundStorageId: B, lastSyncedEtag: '"v9"', dirty: true } })
    await h.engine.setAccount(A)
    await h.engine.resolveConflict('use-cloud')

    expect(h.runtime.imported).toEqual([dbBytes('theirs')])
    expect(h.record()).toEqual({ boundStorageId: A, lastSyncedEtag: '"v1"', dirty: false, lastSyncAt: T0 })
  })

  test('no pushes go out while the question is open', async () => {
    const h = make({ cloud: 'cloud', record: { boundStorageId: B, lastSyncedEtag: '"v1"', dirty: false } })
    await h.engine.setAccount(A)
    h.server.requests.length = 0

    h.engine.notifyChange()
    await h.clock.advance(120_000)
    await h.engine.flush()

    expect(h.server.requests).toEqual([])
    expect(h.record()).toMatchObject({ dirty: true })
  })
})

// ── Pushing: debounce, flush ────────────────────────────────────────

describe('debounced push', () => {
  async function started() {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0
    return h
  }

  test('a change marks the record dirty at once and pushes 30 s after the last change', async () => {
    const h = await started()

    h.engine.notifyChange()
    expect(h.record()!.dirty).toBe(true)
    expect(h.server.requests).toEqual([])

    await h.clock.advance(29_999)
    expect(h.server.requests).toEqual([])
    await h.clock.advance(1)

    expect(h.puts()).toHaveLength(1)
    expect(h.puts()[0].headers['If-Match']).toBe('"v1"')
    expect(h.record()).toEqual({ boundStorageId: A, lastSyncedEtag: '"v2"', dirty: false, lastSyncAt: T0 + 30_000 })
  })

  test('each change restarts the quiet period', async () => {
    const h = await started()

    h.engine.notifyChange()
    await h.clock.advance(20_000)
    h.engine.notifyChange()
    await h.clock.advance(29_999)
    expect(h.puts()).toHaveLength(0)
    h.engine.notifyChange()
    await h.clock.advance(29_999)
    expect(h.puts()).toHaveLength(0)

    await h.clock.advance(1)
    expect(h.puts()).toHaveLength(1)
  })

  test('a burst of changes is one push', async () => {
    const h = await started()
    for (let i = 0; i < 20; i++) h.engine.notifyChange()
    await h.clock.advance(60_000)
    expect(h.puts()).toHaveLength(1)
  })

  test('the delay is an option', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false), options: { debounceMs: 5_000 } })
    await h.engine.setAccount(A)
    h.engine.notifyChange()
    await h.clock.advance(4_999)
    expect(h.puts()).toHaveLength(0)
    await h.clock.advance(1)
    expect(h.puts()).toHaveLength(1)
  })

  test('an edit made while the push runs keeps the record dirty and pushes again', async () => {
    const h = await started()
    h.runtime.onExport = () => {
      h.runtime.onExport = undefined
      h.engine.notifyChange()
    }

    h.engine.notifyChange()
    await h.clock.advance(30_000)

    expect(h.puts()).toHaveLength(1)
    expect(h.record()).toMatchObject({ lastSyncedEtag: '"v2"', dirty: true })
    expect(h.clock.pending).toBe(1)

    await h.clock.advance(30_000)
    expect(h.puts()).toHaveLength(2)
    expect(h.puts()[1].headers['If-Match']).toBe('"v2"')
    expect(h.record()).toMatchObject({ lastSyncedEtag: '"v3"', dirty: false })
  })

  test('the change an import announces is not an edit', async () => {
    const h = make({ record: boundToA(false) })
    h.server.put('first')
    h.server.put('second')
    let during: SyncRecord | null = null
    h.runtime.onImport = () => {
      // The runtime announces forge:changed from inside importDatabase.
      h.engine.notifyChange()
      during = h.record()
    }

    await h.engine.setAccount(A)

    expect(during).toMatchObject({ dirty: false })
    expect(h.record()).toMatchObject({ lastSyncedEtag: '"v2"', dirty: false })
    expect(h.clock.pending).toBe(0)
  })

  test('the ETag recorded after a pull is the one that came with the bytes, not the one the HEAD saw', async () => {
    const h = make({ record: boundToA(false) })
    h.server.put('first')
    h.server.put('second') // "v2": what the HEAD sees
    h.server.hooks.GET = () => {
      h.server.hooks.GET = undefined
      h.server.put('third') // "v3": pushed from another device before the GET is answered
    }

    await h.engine.setAccount(A)

    expect(h.runtime.imported).toEqual([dbBytes('third')])
    expect(h.record()).toMatchObject({ lastSyncedEtag: '"v3"', dirty: false })
  })

  test('changes made before sign-in are remembered and pushed once signed in', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.engine.notifyChange()
    expect(h.record()!.dirty).toBe(true)
    expect(h.clock.pending).toBe(0)
    expect(h.server.requests).toEqual([])

    await h.engine.setAccount(A)
    expect(h.puts()).toHaveLength(1)
  })
})

describe('flush', () => {
  test('pushes at once, and the quiet period is not waited out or repeated', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0

    h.engine.notifyChange()
    await h.engine.flush()
    expect(h.puts()).toHaveLength(1)
    expect(h.clock.pending).toBe(0)

    await h.clock.advance(120_000)
    expect(h.puts()).toHaveLength(1)
  })

  test('with nothing to push it does nothing', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0

    await h.engine.flush()
    expect(h.server.requests).toEqual([])
  })

  test('twice in a row is one push', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.engine.notifyChange()

    await Promise.all([h.engine.flush(), h.engine.flush()])
    expect(h.puts()).toHaveLength(1)
  })

  test('never overlaps an operation that is already running', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    let inFlight = 0
    let overlapped = false
    const inner = h.server.fetch
    const wrapped = (async (url: string, init: RequestInit) => {
      inFlight++
      if (inFlight > 1) overlapped = true
      await settle()
      try {
        return await inner(url, init)
      } finally {
        inFlight--
      }
    }) as SyncFetch
    const engine = createSyncEngine({
      baseUrl: 'https://data.example',
      fetch: wrapped,
      runtime: h.runtime,
      storage: { get: () => h.record(), set: () => {} },
      clock: h.clock,
      isOwner: () => true,
      localHasData: async () => false,
      onEvent: () => {},
    })
    await Promise.all([engine.setAccount(A), engine.flush(), engine.sync()])
    expect(overlapped).toBe(false)
  })
})

// ── What the engine refuses to do ───────────────────────────────────

describe('refusing to push', () => {
  test('bytes in WAL mode (header bytes 18-19 are 2,2): nothing is uploaded and the record stays dirty', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    h.runtime.exportResult = { ...h.runtime.exportResult, bytes: dbBytes('wal', [2, 2]) }
    await h.engine.setAccount(A)

    expect(h.puts()).toHaveLength(0)
    expect(h.errors()).toEqual([expect.objectContaining({ code: 'journal_mode' })])
    expect(h.status()).toBe('error')
    expect(h.record()).toEqual(boundToA(true))
    // Not a failure retrying can fix, so nothing is scheduled and later edits do not try again.
    expect(h.clock.pending).toBe(0)
    h.engine.notifyChange()
    expect(h.clock.pending).toBe(0)
  })

  test('each of the two journal bytes is checked', async () => {
    for (const journal of [[1, 2], [2, 1], [0, 0]] as [number, number][]) {
      const h = make({ cloud: 'cloud', record: boundToA(true) })
      h.runtime.exportResult = { ...h.runtime.exportResult, bytes: dbBytes('x', journal) }
      await h.engine.setAccount(A)
      expect(h.puts()).toHaveLength(0)
      expect(h.errors()).toEqual([expect.objectContaining({ code: 'journal_mode' })])
    }
  })

  test('the same check guards a first upload', async () => {
    const h = make({ hasData: true })
    h.runtime.exportResult = { ...h.runtime.exportResult, bytes: dbBytes('wal', [2, 2]) }
    await h.engine.setAccount(A)

    expect(h.puts()).toHaveLength(0)
    expect(h.record()).toBeNull()
    expect(h.errors()).toEqual([expect.objectContaining({ code: 'journal_mode' })])
  })

  test('bytes that are not a SQLite file, or an export that names no schema head', async () => {
    const notSqlite = make({ cloud: 'cloud', record: boundToA(true) })
    notSqlite.runtime.exportResult = { ...notSqlite.runtime.exportResult, bytes: new Uint8Array(200) }
    await notSqlite.engine.setAccount(A)
    expect(notSqlite.puts()).toHaveLength(0)
    expect(notSqlite.errors()).toEqual([expect.objectContaining({ code: 'invalid_export' })])

    const noHead = make({ cloud: 'cloud', record: boundToA(true) })
    noHead.runtime.exportResult = { ...noHead.runtime.exportResult, schema_head: null }
    await noHead.engine.setAccount(A)
    expect(noHead.puts()).toHaveLength(0)
    expect(noHead.errors()).toEqual([expect.objectContaining({ code: 'invalid_export' })])
  })

  test('a manual sync tries again after such a failure', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    h.runtime.exportResult = { ...h.runtime.exportResult, bytes: dbBytes('wal', [2, 2]) }
    await h.engine.setAccount(A)
    h.runtime.exportResult = { ...h.runtime.exportResult, bytes: dbBytes('fixed') }

    await h.engine.sync()
    expect(h.puts()).toHaveLength(1)
    expect(h.status()).toBe('synced')
  })
})

describe('refusing to import', () => {
  test('a cloud copy from a newer bundle (X-Forge-Schema-Head past the runtime\'s known head)', async () => {
    const h = make({ record: boundToA(false) })
    h.server.put('older')
    h.server.put('newer schema', '999_from_the_future')
    await h.engine.setAccount(A)

    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD'])
    expect(h.runtime.imported).toEqual([])
    expect(h.errors()).toEqual([expect.objectContaining({ code: 'schema_too_new' })])
    expect(h.record()).toEqual(boundToA(false))
    expect(h.events.some((e) => e.type === 'reload')).toBe(false)
  })

  test('the same head as the runtime knows, or an older one, is fine', async () => {
    for (const head of [HEAD, '001_initial']) {
      const h = make({ record: boundToA(false) })
      h.server.put('older')
      h.server.put('current', head)
      await h.engine.setAccount(A)
      expect(h.runtime.imported).toEqual([dbBytes('current')])
    }
  })

  test('the user choosing the cloud copy is refused the same way, and keeps their conflict', async () => {
    const h = make({ hasData: true })
    h.server.put('newer schema', '999_from_the_future')
    await h.engine.setAccount(A)
    expect(h.status()).toBe('conflict')

    await h.engine.resolveConflict('use-cloud')

    expect(h.runtime.imported).toEqual([])
    expect(h.errors()).toEqual([expect.objectContaining({ code: 'schema_too_new' })])
    expect(h.engine.state().conflict).not.toBeNull()
    expect(h.record()).toBeNull()
  })

  test('the runtime refusing a file (its own schema check) is reported the same way and changes nothing', async () => {
    const h = make({ record: boundToA(false) })
    h.server.put('first')
    h.server.put('second')
    h.runtime.importError = Object.assign(new Error('import rejected, current data kept'), { code: 'schema_too_new' })
    await h.engine.setAccount(A)

    expect(h.errors()).toEqual([expect.objectContaining({ code: 'schema_too_new' })])
    expect(h.record()).toEqual(boundToA(false))
    expect(h.events.some((e) => e.type === 'reload')).toBe(false)
    expect(h.clock.pending).toBe(0)
  })

  test('any other import failure is reported and the record is untouched', async () => {
    const h = make({ record: boundToA(false) })
    h.server.put('first')
    h.server.put('second')
    h.runtime.importError = new Error('disk full')
    await h.engine.setAccount(A)

    expect(h.errors()).toEqual([expect.objectContaining({ code: 'import_failed', message: expect.stringContaining('disk full') })])
    expect(h.record()).toEqual(boundToA(false))
  })

  test('without a known head from the runtime nothing is imported', async () => {
    const h = make({ record: boundToA(false) })
    h.server.put('first')
    h.server.put('second')
    h.runtime.known = undefined
    await h.engine.setAccount(A)

    expect(h.runtime.imported).toEqual([])
    expect(h.errors()).toEqual([expect.objectContaining({ code: 'runtime_unavailable' })])
  })

  test('an edit made while the cloud copy downloads is not overwritten: the table is asked again', async () => {
    const h = make({ record: boundToA(false) })
    h.server.put('first')
    h.server.put('second')
    h.server.hooks.GET = () => h.engine.notifyChange()

    await h.engine.setAccount(A)

    expect(h.runtime.imported).toEqual([])
    expect(h.status()).toBe('conflict')
    expect(h.engine.state().conflict).toMatchObject({ reason: 'diverged' })
    expect(h.record()).toMatchObject({ dirty: true })
  })
})

// ── Errors ──────────────────────────────────────────────────────────

describe('401', () => {
  test('on the first check: signed out, nothing retried, nothing scheduled', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    h.server.script.push(new Response(null, { status: 401 }))
    await h.engine.setAccount(A)

    expect(h.status()).toBe('signed-out')
    expect(h.events).toContainEqual({ type: 'unauthorized' })
    expect(h.errors()).toEqual([])
    expect(h.clock.pending).toBe(0)

    await h.clock.advance(3_600_000)
    expect(h.server.requests).toHaveLength(1)
  })

  test('in the middle of a push: stops, and the record stays dirty for the next sign-in', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0
    h.server.script.push(new Response(null, { status: 401 }))

    h.engine.notifyChange()
    await h.clock.advance(30_000)

    expect(h.status()).toBe('signed-out')
    expect(h.record()).toMatchObject({ dirty: true })
    expect(h.clock.pending).toBe(0)

    // Edits keep being remembered, but nothing is sent until an account is set again.
    h.engine.notifyChange()
    await h.clock.advance(120_000)
    expect(h.server.requests).toHaveLength(1)

    await h.engine.setAccount(A)
    expect(h.puts()).toHaveLength(2)
    expect(h.status()).toBe('synced')
  })

  test('signing out cancels what is pending and sends nothing', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0
    h.engine.notifyChange()
    expect(h.clock.pending).toBe(1)

    await h.engine.setAccount(null)

    expect(h.status()).toBe('signed-out')
    expect(h.clock.pending).toBe(0)
    await h.clock.advance(120_000)
    expect(h.server.requests).toEqual([])
  })
})

describe('errors that retrying cannot fix', () => {
  const cases: [number, string][] = [
    [413, 'too_large'],
    [422, 'invalid_snapshot'],
    [428, 'precondition_required'],
    [403, 'unexpected_status'],
    [400, 'unexpected_status'],
  ]
  for (const [status, code] of cases) {
    test(`${status} on a push is ${code}: reported once, not retried, record stays dirty`, async () => {
      const h = make({ cloud: 'cloud', record: boundToA(false) })
      await h.engine.setAccount(A)
      h.server.requests.length = 0
      h.server.script.push(new Response(`{"error":"x"}`, { status }))

      h.engine.notifyChange()
      await h.clock.advance(30_000)

      expect(h.puts()).toHaveLength(1)
      expect(h.errors()).toEqual([expect.objectContaining({ code, status })])
      expect(h.status()).toBe('error')
      expect(h.record()).toMatchObject({ dirty: true, lastSyncedEtag: '"v1"' })
      expect(h.clock.pending).toBe(0)
      await h.clock.advance(3_600_000)
      expect(h.puts()).toHaveLength(1)

      // Further edits do not hammer the service either; a manual sync does try again.
      h.engine.notifyChange()
      expect(h.clock.pending).toBe(0)
      await h.engine.sync()
      expect(h.status()).toBe('synced')
    })
  }

  test('413 on a first upload leaves the browser unbound', async () => {
    const h = make({ hasData: true })
    h.server.script.push(new Response(null, { status: 404 }), new Response(null, { status: 413 }))
    await h.engine.setAccount(A)

    expect(h.errors()).toEqual([expect.objectContaining({ code: 'too_large' })])
    expect(h.record()).toBeNull()
  })
})

describe('429', () => {
  async function rateLimited(retryAfter?: string, options?: Partial<SyncOptions>) {
    const h = make({ cloud: 'cloud', record: boundToA(false), options })
    await h.engine.setAccount(A)
    h.server.requests.length = 0
    h.server.script.push(
      new Response(null, { status: 429, headers: retryAfter === undefined ? {} : { 'Retry-After': retryAfter } }),
    )
    h.engine.notifyChange()
    await h.clock.advance(30_000)
    return h
  }

  test('waits for Retry-After, then tries the push again', async () => {
    const h = await rateLimited('7')
    const error = h.errors()[0] as { code: string; retryAt: number }
    expect(error).toMatchObject({ code: 'rate_limited', retryAt: T0 + 30_000 + 7_000 })
    expect(h.status()).toBe('error')
    expect(h.puts()).toHaveLength(1)

    await h.clock.advance(6_999)
    expect(h.puts()).toHaveLength(1)
    await h.clock.advance(1)

    expect(h.puts()).toHaveLength(2)
    expect(h.status()).toBe('synced')
    expect(h.engine.state().error).toBeNull()
    expect(h.record()).toMatchObject({ dirty: false, lastSyncedEtag: '"v2"' })
  })

  test('Retry-After as an HTTP date', async () => {
    const at = new Date(T0 + 30_000 + 12_000).toUTCString()
    const h = await rateLimited(at)
    const error = h.errors()[0] as { retryAt: number }
    // HTTP dates have one-second resolution.
    expect(Math.abs(error.retryAt - (T0 + 30_000 + 12_000))).toBeLessThan(1000)
  })

  test('a very long Retry-After is capped', async () => {
    const h = await rateLimited('86400', { backoffCapMs: 60_000 })
    expect((h.errors()[0] as { retryAt: number }).retryAt).toBe(T0 + 30_000 + 60_000)
  })

  test('without Retry-After it backs off like any other transient error', async () => {
    const h = await rateLimited(undefined)
    expect((h.errors()[0] as { retryAt: number }).retryAt).toBe(T0 + 30_000 + 2_000)
  })

  test('parseRetryAfter', () => {
    expect(parseRetryAfter('5', 0)).toBe(5000)
    expect(parseRetryAfter('0', 0)).toBe(1000)
    expect(parseRetryAfter(' 12 ', 0)).toBe(12_000)
    expect(parseRetryAfter(new Date(10_000).toUTCString(), 0)).toBe(10_000)
    expect(parseRetryAfter(new Date(0).toUTCString(), 60_000)).toBe(1000)
    expect(parseRetryAfter(null, 0)).toBeNull()
    expect(parseRetryAfter('soon', 0)).toBeNull()
  })
})

describe('network and server errors back off', () => {
  test('a network error on the first check retries with doubling delays, capped', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false), options: { backoffCapMs: 5_000, maxRetries: 10 } })
    for (let i = 0; i < 5; i++) h.server.script.push(new TypeError('Failed to fetch'))
    await h.engine.setAccount(A)
    await h.clock.advance(60_000)

    // Five failures, then the sixth attempt gets through: 2 s, 4 s, then capped at 5 s.
    expect(h.server.requests.map((r) => r.at - T0)).toEqual([0, 2_000, 6_000, 11_000, 16_000, 21_000])
    expect(h.errors().map((e) => (e as { code: string }).code)).toEqual(Array(5).fill('network'))
    expect(h.status()).toBe('synced')
    expect(h.engine.state().error).toBeNull()
  })

  test('a 502 on a push is retried the same way, and succeeds', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0
    h.server.script.push(new Response(null, { status: 502 }))

    h.engine.notifyChange()
    await h.clock.advance(30_000)
    expect(h.errors()).toEqual([expect.objectContaining({ code: 'server', status: 502, retryAt: T0 + 32_000 })])
    expect(h.record()).toMatchObject({ dirty: true })

    await h.clock.advance(2_000)
    expect(h.puts()).toHaveLength(2)
    expect(h.status()).toBe('synced')
    expect(h.record()).toMatchObject({ dirty: false })
  })

  test('success resets the backoff', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0
    h.server.script.push(new TypeError('offline'), new TypeError('offline'))
    h.engine.notifyChange()
    await h.clock.advance(30_000) // fails, retry in 2 s
    await h.clock.advance(2_000) // fails again, retry in 4 s
    await h.clock.advance(4_000) // succeeds
    expect(h.status()).toBe('synced')

    h.server.script.push(new TypeError('offline'))
    h.engine.notifyChange()
    await h.clock.advance(30_000)
    const error = h.engine.state().error as { retryAt: number }
    expect(error.retryAt - h.clock.now()).toBe(2_000)
  })

  test('after the retries run out it waits for the next change instead of looping', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false), options: { maxRetries: 2 } })
    await h.engine.setAccount(A)
    h.server.requests.length = 0
    for (let i = 0; i < 3; i++) h.server.script.push(new TypeError('offline'))

    h.engine.notifyChange()
    await h.clock.advance(30_000)
    await h.clock.advance(2_000)
    await h.clock.advance(4_000)

    expect(h.puts()).toHaveLength(3)
    expect(h.status()).toBe('error')
    expect((h.engine.state().error as { retryAt?: number }).retryAt).toBeUndefined()
    expect(h.clock.pending).toBe(0)
    await h.clock.advance(3_600_000)
    expect(h.puts()).toHaveLength(3)

    // The next edit starts a fresh attempt, and the service is back.
    h.engine.notifyChange()
    await h.clock.advance(30_000)
    expect(h.status()).toBe('synced')
  })

  test('a failure on the first check is retried as a sync', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    h.server.script.push(new Response(null, { status: 503 }))
    await h.engine.setAccount(A)
    expect(h.status()).toBe('error')

    await h.clock.advance(2_000)
    expect(h.server.requests.map((r) => r.method)).toEqual(['HEAD', 'HEAD', 'PUT'])
    expect(h.status()).toBe('synced')
  })

  test('a manual flush does not wait for a scheduled retry, and cancels it', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0
    h.server.script.push(new TypeError('offline'))
    h.engine.notifyChange()
    await h.clock.advance(30_000)
    expect(h.clock.pending).toBe(1)

    await h.engine.flush()

    expect(h.puts()).toHaveLength(2)
    expect(h.status()).toBe('synced')
    expect(h.clock.pending).toBe(0)
  })
})

// ── Who syncs, and what is stored ───────────────────────────────────

describe('only the database owner syncs', () => {
  test('a tab that is not the owner makes no requests', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true), owner: false })
    await h.engine.setAccount(A)

    expect(h.status()).toBe('not-owner')
    expect(h.server.requests).toEqual([])

    h.engine.notifyChange()
    await h.engine.flush()
    await h.clock.advance(120_000)
    expect(h.server.requests).toEqual([])
    expect(h.runtime.exports + h.runtime.imported.length).toBe(0)
  })

  test('once it becomes the owner, a sync goes ahead', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true), owner: false })
    await h.engine.setAccount(A)
    h.state.owner = true

    await h.engine.sync()

    expect(h.status()).toBe('synced')
    expect(h.puts()).toHaveLength(1)
  })

  test('losing ownership part-way stops before anything is written', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    h.runtime.onExport = () => {
      h.state.owner = false
    }
    await h.engine.setAccount(A)

    expect(h.puts()).toHaveLength(0)
    expect(h.status()).toBe('not-owner')
    expect(h.errors()).toEqual([])
    expect(h.record()).toEqual(boundToA(true))
  })
})

describe('no account', () => {
  test('nothing happens before an account is set', async () => {
    const h = make({ cloud: 'cloud' })
    expect(h.status()).toBe('idle')
    h.engine.notifyChange()
    await h.engine.flush()
    await h.engine.sync()
    await h.clock.advance(120_000)
    expect(h.server.requests).toEqual([])
    expect(h.status()).toBe('idle')
  })
})

describe('what is stored', () => {
  test('the record has exactly the four documented fields, under forge.sync.v1', async () => {
    expect(SYNC_STORAGE_KEY).toBe('forge.sync.v1')
    const h = make({ hasData: true })
    await h.engine.setAccount(A)
    expect(Object.keys(h.record()!).sort()).toEqual(['boundStorageId', 'dirty', 'lastSyncAt', 'lastSyncedEtag'])
  })

  test('the uploaded bytes are the runtime\'s export and nothing else: sync state never rides along', async () => {
    const h = make({ hasData: true })
    await h.engine.setAccount(A)
    expect(h.puts()[0].body).toBe(h.runtime.exportResult.bytes)
  })

  test('every request carries the session cookie', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    await h.engine.setAccount(A)
    expect(h.server.requests.length).toBeGreaterThan(1)
    expect(h.server.requests.every((r) => r.credentials === 'include')).toBe(true)
  })

  test('the ETag is sent back exactly as the service gave it', async () => {
    const h = make({ record: { boundStorageId: A, lastSyncedEtag: 'W/"abc-def"', dirty: true } })
    h.server.put('x')
    h.server.object = { ...h.server.object!, etag: 'W/"abc-def"' }
    await h.engine.setAccount(A)
    expect(h.puts()[0].headers['If-Match']).toBe('W/"abc-def"')
  })
})

describe('status events', () => {
  test('a clean start reports syncing, then synced, once each', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    expect(h.events.filter((e) => e.type === 'status')).toEqual([
      { type: 'status', status: 'syncing' },
      { type: 'status', status: 'synced' },
    ])
  })

  test('a listener that throws does not break the sync', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(true) })
    const engine = createSyncEngine({
      baseUrl: 'https://data.example',
      fetch: h.server.fetch,
      runtime: h.runtime,
      storage: { get: () => h.record(), set: () => {} },
      clock: h.clock,
      isOwner: () => true,
      localHasData: async () => false,
      onEvent: () => {
        throw new Error('listener blew up')
      },
    })
    await engine.setAccount(A)
    expect(h.puts()).toHaveLength(1)
  })

  test('dispose stops the timers and later calls', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    await h.engine.setAccount(A)
    h.server.requests.length = 0
    h.engine.notifyChange()
    expect(h.clock.pending).toBe(1)

    h.engine.dispose()
    expect(h.clock.pending).toBe(0)
    await h.engine.sync()
    await h.engine.flush()
    expect(h.server.requests).toEqual([])
  })
})
