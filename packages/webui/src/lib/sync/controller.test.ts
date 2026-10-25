import { describe, expect, test } from 'bun:test'
import { createSyncController, type SyncViewState } from './controller'
import type { ExportResult, ImportResult, SyncClock, SyncFetch, SyncRecord, SyncRuntime, SyncStorage } from './types'

const A = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'
const HEAD = '055_dataset_meta'
const T0 = 1_700_000_000_000

/** Let promise callbacks and the fake fetch finish: the engine awaits several hops. */
async function settle(rounds = 12) {
  for (let i = 0; i < rounds; i++) await new Promise<void>((resolve) => setTimeout(resolve, 0))
}

class FakeClock implements SyncClock {
  private t = T0
  private id = 1
  private timers: { id: number; at: number; fn: () => void }[] = []
  now() {
    return this.t
  }
  setTimeout(fn: () => void, ms: number) {
    const id = this.id++
    this.timers.push({ id, at: this.t + ms, fn })
    return id
  }
  clearTimeout(handle: unknown) {
    this.timers = this.timers.filter((timer) => timer.id !== handle)
  }
  async advance(ms: number) {
    const end = this.t + ms
    for (;;) {
      const due = this.timers.filter((timer) => timer.at <= end).sort((a, b) => a.at - b.at)[0]
      if (!due) break
      this.timers = this.timers.filter((timer) => timer !== due)
      this.t = Math.max(this.t, due.at)
      due.fn()
      await settle()
    }
    this.t = end
  }
}

function dbBytes(tag: string): Uint8Array {
  const out = new Uint8Array(100 + tag.length)
  out.set(new TextEncoder().encode('SQLite format 3\0'))
  out[18] = 1
  out[19] = 1
  out.set(new TextEncoder().encode(tag), 100)
  return out
}

interface Sent {
  method: string
  headers: Record<string, string>
}

/** The snapshot API in one object: ETag preconditions, the headers the contract names. */
class FakeServer {
  object: { bytes: Uint8Array; etag: string } | null = null
  requests: Sent[] = []
  /** Answers for the next requests, ahead of the real handler. An Error is a network failure. */
  script: Array<Response | Error> = []
  private n = 0

  put(tag: string) {
    this.object = { bytes: dbBytes(tag), etag: `"v${++this.n}"` }
  }

  fetch: SyncFetch = async (_url, init) => {
    const method = init.method ?? 'GET'
    const headers = { ...(init.headers as Record<string, string> | undefined) }
    this.requests.push({ method, headers })
    const scripted = this.script.shift()
    if (scripted instanceof Error) throw scripted
    if (scripted) return scripted
    const cur = this.object
    if (method === 'HEAD' || method === 'GET') {
      if (!cur) return new Response(null, { status: 404 })
      return new Response(method === 'GET' ? (cur.bytes as BodyInit) : null, {
        status: 200,
        headers: { ETag: cur.etag, 'X-Forge-Schema-Head': HEAD, 'X-Forge-Bundle-Version': '0.1.0' },
      })
    }
    if (method === 'DELETE') {
      this.object = null
      return new Response(null, { status: 204 })
    }
    const ifMatch = headers['If-Match']
    const ifNoneMatch = headers['If-None-Match']
    if ((ifMatch === undefined) === (ifNoneMatch === undefined)) return new Response(null, { status: 428 })
    if (ifNoneMatch !== undefined && cur) return new Response(null, { status: 412 })
    if (ifMatch !== undefined && cur?.etag !== ifMatch) return new Response(null, { status: 412 })
    this.object = { bytes: init.body as Uint8Array, etag: `"v${++this.n}"` }
    return new Response(JSON.stringify({ etag: this.object.etag }), {
      status: 200,
      headers: { ETag: this.object.etag },
    })
  }
}

class FakeRuntime implements SyncRuntime {
  imported: Uint8Array[] = []
  async exportDatabase(): Promise<ExportResult> {
    return { bytes: dbBytes('local'), schema_head: HEAD, known_head: HEAD, bundle_version: '0.1.0' }
  }
  async importDatabase(bytes: Uint8Array): Promise<ImportResult> {
    this.imported.push(bytes)
    return { imported_bytes: bytes.length, migrations: 1, schema_head: HEAD, known_head: HEAD, bundle_version: '0.1.0' }
  }
  knownHead() {
    return HEAD
  }
}

interface Setup {
  available?: boolean
  hasData?: boolean
  owner?: boolean
  cloud?: string
  record?: Partial<SyncRecord>
}

function make(setup: Setup = {}) {
  const clock = new FakeClock()
  const server = new FakeServer()
  if (setup.cloud) server.put(setup.cloud)
  const runtime = new FakeRuntime()
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
  const toasts: Array<{ message: string; type?: string }> = []
  const views: SyncViewState[] = []
  const calls = { reload: 0, refreshSession: 0, exportBackup: 0 }
  let exportFails = false
  const controller = createSyncController({
    available: setup.available ?? true,
    baseUrl: 'https://data.example',
    fetch: server.fetch,
    runtime,
    storage,
    clock,
    isOwner: () => state.owner,
    localHasData: async () => state.hasData,
    notify: (t) => toasts.push(t),
    reload: () => {
      calls.reload++
    },
    refreshSession: () => {
      calls.refreshSession++
    },
    exportBackup: async () => {
      calls.exportBackup++
      if (exportFails) throw new Error('disk full')
    },
    onChange: (v) => views.push(v),
  })
  return {
    controller,
    clock,
    server,
    runtime,
    state,
    toasts,
    views,
    calls,
    failExport: () => {
      exportFails = true
    },
    record: () => stored,
    view: () => controller.get(),
    methods: () => server.requests.map((r) => r.method),
  }
}

const boundToA = (dirty: boolean): Partial<SyncRecord> => ({ boundStorageId: A, lastSyncedEtag: '"v1"', dirty })

describe('where cloud sync does not exist', () => {
  test('nothing runs and nothing is requested', async () => {
    const h = make({ available: false, hasData: true, cloud: 'cloud' })
    h.controller.setAccount(A)
    h.controller.notifyChange()
    h.controller.pageHidden()
    h.controller.runtimeChanged()
    await h.controller.syncNow()
    await h.controller.turnOn()
    await h.controller.deleteCloudCopy()
    await h.controller.resolve('use-cloud')
    await h.clock.advance(120_000)

    expect(h.server.requests).toEqual([])
    expect(h.view()).toMatchObject({ available: false, signedIn: false, status: 'idle', conflict: null })
    expect(h.toasts).toEqual([])
  })
})

describe('the account', () => {
  test('signing in with data here and an empty cloud uploads and reports synced', async () => {
    const h = make({ hasData: true })
    h.controller.setAccount(A)
    await settle()

    expect(h.methods()).toEqual(['HEAD', 'PUT'])
    expect(h.view()).toMatchObject({ available: true, signedIn: true, enabled: true, status: 'synced', busy: false })
    expect(h.view().lastSyncAt).toBe(T0)
    expect(h.toasts).toEqual([])
  })

  test('signed out: no request, and the view says signed out', async () => {
    const h = make({ hasData: true, cloud: 'cloud' })
    h.controller.setAccount(null)
    await settle()

    expect(h.server.requests).toEqual([])
    expect(h.view()).toMatchObject({ signedIn: false, status: 'signed-out' })
  })

  test('the same account again does nothing', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.controller.setAccount(A)
    await settle()
    h.controller.setAccount(A)
    h.controller.setAccount(A)
    await settle()
    expect(h.methods()).toEqual(['HEAD'])
  })

  test('a changed account goes back through the table', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.controller.setAccount(A)
    await settle()
    h.controller.setAccount(null)
    await settle()
    h.controller.setAccount(A)
    await settle()
    expect(h.methods()).toEqual(['HEAD', 'HEAD'])
  })
})

describe('conflicts', () => {
  test('a cloud copy plus data here is a conflict the view carries', async () => {
    const h = make({ cloud: 'cloud', hasData: true })
    h.controller.setAccount(A)
    await settle()

    expect(h.view().status).toBe('conflict')
    expect(h.view().conflict).toMatchObject({ reason: 'unbound', storageId: A })
    expect(h.methods()).toEqual(['HEAD'])
  })

  test('use the cloud copy: pulls it in and asks the page to reload', async () => {
    const h = make({ cloud: 'cloud', hasData: true })
    h.controller.setAccount(A)
    await settle()

    await h.controller.resolve('use-cloud')

    expect(h.runtime.imported).toEqual([dbBytes('cloud')])
    expect(h.calls.reload).toBe(1)
    expect(h.view().conflict).toBeNull()
    expect(h.view().busy).toBe(false)
  })

  test('turn sync off: no request, off, and it stays off in the stored record', async () => {
    const h = make({ cloud: 'cloud', hasData: true })
    h.controller.setAccount(A)
    await settle()
    h.server.requests.length = 0

    await h.controller.resolve('turn-off')

    expect(h.server.requests).toEqual([])
    expect(h.view()).toMatchObject({ enabled: false, status: 'off', conflict: null })
    expect(h.record()).toMatchObject({ syncEnabled: false, boundStorageId: null })
  })

  test('a choice that cannot apply is a toast, not a crash', async () => {
    const h = make({ record: boundToA(false), hasData: true }) // cloud copy gone while bound
    h.controller.setAccount(A)
    await settle()
    expect(h.view().conflict).toMatchObject({ reason: 'cloud-deleted' })

    await h.controller.resolve('use-cloud')

    expect(h.toasts).toEqual([expect.objectContaining({ type: 'error', message: expect.stringContaining('no cloud copy') })])
    expect(h.view().conflict).toMatchObject({ reason: 'cloud-deleted' })
  })
})

describe('what triggers a push', () => {
  test('a data change pushes once the quiet period has passed', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.controller.setAccount(A)
    await settle()
    h.server.requests.length = 0

    h.controller.notifyChange()
    await h.clock.advance(29_000)
    expect(h.server.requests).toEqual([])
    await h.clock.advance(1_000)

    expect(h.methods()).toEqual(['PUT'])
    expect(h.server.requests[0].headers['If-Match']).toBe('"v1"')
  })

  test('hiding the page pushes at once, without waiting for the quiet period', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.controller.setAccount(A)
    await settle()
    h.server.requests.length = 0

    h.controller.notifyChange()
    h.controller.pageHidden()
    await settle()

    expect(h.methods()).toEqual(['PUT'])
  })

  test('hiding the page with nothing changed sends nothing', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.controller.setAccount(A)
    await settle()
    h.server.requests.length = 0

    h.controller.pageHidden()
    await settle()

    expect(h.server.requests).toEqual([])
  })
})

describe('only the database owner syncs', () => {
  test('a tab that does not own the database waits, then syncs when it becomes the owner', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false), owner: false })
    h.controller.setAccount(A)
    await settle()
    expect(h.view().status).toBe('not-owner')
    expect(h.server.requests).toEqual([])

    h.state.owner = true
    h.controller.runtimeChanged()
    await settle()

    expect(h.methods()).toEqual(['HEAD'])
    expect(h.view().status).toBe('synced')
  })

  test('a runtime change while already synced does nothing', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.controller.setAccount(A)
    await settle()
    h.server.requests.length = 0

    h.controller.runtimeChanged()
    await settle()

    expect(h.server.requests).toEqual([])
  })

  test('and does nothing while signed out', async () => {
    const h = make({ cloud: 'cloud', owner: false })
    h.controller.setAccount(null)
    h.state.owner = true
    h.controller.runtimeChanged()
    await settle()
    expect(h.server.requests).toEqual([])
  })
})

describe('a pull', () => {
  test('replacing the database asks the page to reload exactly once', async () => {
    const h = make({ record: boundToA(false) })
    h.server.put('first') // "v1": what this browser last saw
    h.server.put('second') // "v2": another device pushed
    h.controller.setAccount(A)
    await settle()

    expect(h.runtime.imported).toEqual([dbBytes('second')])
    expect(h.calls.reload).toBe(1)
  })
})

describe('errors', () => {
  const server500 = () => new Response(null, { status: 500 })

  test('the same error is toasted once within a minute, however often it repeats', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.server.script.push(server500(), server500(), server500())
    h.controller.setAccount(A)
    await settle()
    expect(h.toasts).toHaveLength(1)

    await h.clock.advance(2_000) // first retry fails the same way
    await h.clock.advance(4_000) // second
    expect(h.toasts).toHaveLength(1)
    expect(h.view().status).toBe('error')
    expect(h.view().error).toMatchObject({ code: 'server' })
  })

  test('a network failure is toasted in plain words that say the data is safe', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.server.script.push(new TypeError('Failed to fetch'))
    h.controller.setAccount(A)
    await settle()

    expect(h.toasts).toEqual([
      expect.objectContaining({ type: 'error', message: expect.stringContaining('safe in this browser') }),
    ])
    expect(h.view().error).toMatchObject({ code: 'network' })
  })

  test('a signed-out answer re-checks the session, and the next sign-in picks sync up again', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.server.script.push(new Response(null, { status: 401 }))
    h.controller.setAccount(A)
    await settle()

    expect(h.calls.refreshSession).toBe(1)
    expect(h.view()).toMatchObject({ signedIn: false, status: 'signed-out' })

    // The session store answers "signed in" again: same account, but the engine had stopped.
    h.controller.setAccount(A)
    await settle()
    expect(h.methods()).toEqual(['HEAD', 'HEAD'])
    expect(h.view().status).toBe('synced')
  })
})

describe('the buttons', () => {
  test('sync now checks the cloud copy and is busy while it runs', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.controller.setAccount(A)
    await settle()
    h.server.requests.length = 0
    h.views.length = 0

    const done = h.controller.syncNow()
    expect(h.view().busy).toBe(true)
    await done

    expect(h.methods()).toEqual(['HEAD'])
    expect(h.view().busy).toBe(false)
    expect(h.views.some((v) => v.busy)).toBe(true)
  })

  test('delete the cloud copy: deletes, turns sync off, and says so', async () => {
    const h = make({ cloud: 'cloud', record: boundToA(false) })
    h.controller.setAccount(A)
    await settle()
    h.server.requests.length = 0

    await h.controller.deleteCloudCopy()

    expect(h.methods()).toEqual(['DELETE'])
    expect(h.server.object).toBeNull()
    expect(h.view()).toMatchObject({ enabled: false, status: 'off', busy: false })
    expect(h.toasts).toEqual([expect.objectContaining({ type: 'success', message: expect.stringContaining('cloud copy was deleted') })])
  })

  test('turn off, then on, while the cloud copy still exists: asks, as on a first sign-in', async () => {
    const h = make({ hasData: true })
    h.controller.setAccount(A)
    await settle() // first upload
    await h.controller.turnOff()
    expect(h.view()).toMatchObject({ enabled: false, status: 'off' })
    h.server.requests.length = 0

    await h.controller.turnOn()

    expect(h.view()).toMatchObject({ enabled: true, status: 'conflict' })
    expect(h.view().conflict).toMatchObject({ reason: 'unbound' })
    expect(h.methods()).toEqual(['HEAD'])
  })

  test('turn off, then on, with no cloud copy: uploads what is here', async () => {
    const h = make({ hasData: true })
    h.controller.setAccount(A)
    await settle()
    await h.controller.turnOff()
    h.server.object = null // deleted elsewhere in the meantime
    h.server.requests.length = 0

    await h.controller.turnOn()

    expect(h.view()).toMatchObject({ enabled: true, status: 'synced' })
    expect(h.methods()).toEqual(['HEAD', 'PUT'])
    expect(h.server.requests[1].headers['If-None-Match']).toBe('*')
  })

  test('export first runs the backup, and a failure is a toast', async () => {
    const h = make({ cloud: 'cloud', hasData: true })
    await h.controller.exportBackup()
    expect(h.calls.exportBackup).toBe(1)
    expect(h.toasts).toEqual([])

    h.failExport()
    await h.controller.exportBackup()
    expect(h.toasts).toEqual([expect.objectContaining({ type: 'error', message: 'Export failed: disk full' })])
  })
})
