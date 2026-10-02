import { afterEach, describe, expect, test } from 'bun:test'
import { createRuntime, installApiFetch, type LockManagerLike, type WorkerLike } from './client'
import type { WorkerRequest } from './protocol'

// ── Fakes ───────────────────────────────────────────────────────────

/** A Worker that answers like forge-wasm would, recording what it was asked. */
class FakeWorker implements WorkerLike {
  onmessage: ((event: MessageEvent) => void) | null = null
  onerror: ((event: ErrorEvent) => void) | null = null
  received: WorkerRequest[] = []
  terminated = false
  constructor(private behaviour: (msg: WorkerRequest) => unknown | Promise<unknown>) {}

  postMessage(message: unknown): void {
    const msg = message as WorkerRequest
    this.received.push(msg)
    queueMicrotask(async () => {
      try {
        const result = await this.behaviour(msg)
        this.onmessage?.({ data: { id: msg.id, ok: true, result } } as MessageEvent)
      } catch (e) {
        this.onmessage?.({ data: { id: msg.id, ok: false, error: (e as Error).message } } as MessageEvent)
      }
    })
  }
  terminate() {
    this.terminated = true
  }
}

/** A single-lock Web Locks stand-in. `release()` simulates the owning tab closing. */
class FakeLocks implements LockManagerLike {
  private held = false
  private waiters: Array<() => void> = []
  request(_name: string, a: unknown, b?: unknown): Promise<unknown> {
    const options = typeof a === 'function' ? {} : (a as { ifAvailable?: boolean })
    const callback = (typeof a === 'function' ? a : b) as (lock: unknown) => Promise<unknown>
    if (!this.held) {
      this.held = true
      return callback({})
    }
    if (options.ifAvailable) return callback(null)
    return new Promise((resolve) => {
      this.waiters.push(() => resolve(callback({})))
    })
  }
  release() {
    this.held = false
    const next = this.waiters.shift()
    if (next) {
      this.held = true
      next()
    }
  }
}

const tick = () => new Promise((r) => setTimeout(r, 0))

const okBehaviour = (msg: WorkerRequest): unknown => {
  switch (msg.op) {
    case 'start':
      return { version: '0.1.0', migrations: 52 }
    case 'request':
      if (msg.path === '/api/nothing') return { status: 204, headers: [], body: new Uint8Array() }
      return {
        status: 200,
        headers: [['content-type', 'application/json'], ['x-echo-path', msg.path]],
        body: new TextEncoder().encode(JSON.stringify({ method: msg.method, bytes: msg.body.byteLength })),
      }
    case 'export':
      return new Uint8Array([1, 2, 3])
    case 'import':
      return { imported_bytes: msg.bytes.byteLength, migrations: 52 }
  }
}

function runtimeWith(locks: LockManagerLike, behaviour = okBehaviour, persist = async () => true) {
  const workers: FakeWorker[] = []
  const runtime = createRuntime({
    createWorker: () => {
      const w = new FakeWorker(behaviour)
      workers.push(w)
      return w
    },
    locks,
    persist,
    baseHref: 'http://forge.test/app',
  })
  return { runtime, workers }
}

// ── Ownership and status ────────────────────────────────────────────

describe('createRuntime: owning the database', () => {
  test('the first tab owns the database, starts the worker and reports ready', async () => {
    const { runtime, workers } = runtimeWith(new FakeLocks())
    await runtime.ready
    expect(workers).toHaveLength(1)
    expect(workers[0].received[0].op).toBe('start')
    expect(runtime.status()).toEqual({ state: 'ready', info: { version: '0.1.0', migrations: 52 }, persisted: true })
  })

  test('reports whether persistent storage was granted, and survives persist() throwing', async () => {
    const denied = runtimeWith(new FakeLocks(), okBehaviour, async () => false)
    await denied.runtime.ready
    expect((denied.runtime.status() as { persisted: boolean | null }).persisted).toBe(false)

    const broken = runtimeWith(new FakeLocks(), okBehaviour, async () => {
      throw new Error('no storage API')
    })
    await broken.runtime.ready
    expect((broken.runtime.status() as { persisted: boolean | null }).persisted).toBeNull()
  })

  test('a second tab waits and does not touch the database', async () => {
    const locks = new FakeLocks()
    const first = runtimeWith(locks)
    await first.runtime.ready
    const second = runtimeWith(locks)
    await tick()

    expect(second.runtime.status()).toEqual({ state: 'waiting' })
    expect(second.workers).toHaveLength(0)

    const res = await second.runtime.fetch('/api/sources')
    expect(res.status).toBe(503)
    expect((await res.json()).error.code).toBe('STORAGE_BUSY')
  })

  test('the waiting tab takes over when the owner goes away', async () => {
    const locks = new FakeLocks()
    const first = runtimeWith(locks)
    await first.runtime.ready
    const second = runtimeWith(locks)
    await tick()
    const seen: string[] = []
    second.runtime.onStatus((s) => seen.push(s.state))

    locks.release() // the first tab closes
    await second.runtime.ready

    expect(seen).toEqual(['starting', 'ready'])
    expect(second.workers).toHaveLength(1)
    const res = await second.runtime.fetch('/api/health')
    expect(res.status).toBe(200)
  })

  test('a start failure is reported, and requests fail with 503 rather than hanging', async () => {
    const { runtime } = runtimeWith(new FakeLocks(), (msg) => {
      if (msg.op === 'start') throw new Error('OPFS is not available')
      return null
    })
    await expect(runtime.ready).rejects.toThrow('OPFS is not available')
    expect(runtime.status()).toEqual({ state: 'error', message: 'OPFS is not available' })

    const res = await runtime.fetch('/api/sources')
    expect(res.status).toBe(503)
    const body = await res.json()
    expect(body.error.code).toBe('STORAGE_UNAVAILABLE')
    expect(body.error.message).toContain('OPFS is not available')
  })

  test('works without Web Locks by owning the database directly', async () => {
    const workers: FakeWorker[] = []
    const runtime = createRuntime({
      createWorker: () => {
        const w = new FakeWorker(okBehaviour)
        workers.push(w)
        return w
      },
      locks: undefined,
      persist: async () => true,
      baseHref: 'http://forge.test/',
    })
    // `undefined` falls back to navigator.locks, which Bun lacks, so this takes the direct path.
    await runtime.ready
    expect(workers).toHaveLength(1)
  })
})

// ── fetch ───────────────────────────────────────────────────────────

describe('createRuntime: fetch', () => {
  test('forwards method, path with query and a JSON body, and maps the response', async () => {
    const { runtime, workers } = runtimeWith(new FakeLocks())
    const res = await runtime.fetch('/api/sources?limit=5', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title: 'T' }),
    })
    expect(res.status).toBe(200)
    expect(res.headers.get('content-type')).toBe('application/json')
    expect(res.headers.get('x-echo-path')).toBe('/api/sources?limit=5')
    expect(await res.json()).toEqual({ method: 'POST', bytes: JSON.stringify({ title: 'T' }).length })

    const sent = workers[0].received.find((m) => m.op === 'request') as Extract<WorkerRequest, { op: 'request' }>
    expect(sent.method).toBe('POST')
    expect(sent.headers.find(([k]) => k === 'content-type')?.[1]).toBe('application/json')
  })

  test('accepts a URL, an absolute same-origin URL, and a Request', async () => {
    const { runtime } = runtimeWith(new FakeLocks())
    expect((await runtime.fetch(new URL('http://forge.test/api/health'))).status).toBe(200)
    expect((await runtime.fetch('http://forge.test/api/health')).status).toBe(200)
    expect((await runtime.fetch(new Request('http://forge.test/api/health', { method: 'GET' }))).status).toBe(200)
  })

  test('a 204 has no body (a Response cannot carry one)', async () => {
    const { runtime } = runtimeWith(new FakeLocks())
    const res = await runtime.fetch('/api/nothing', { method: 'DELETE' })
    expect(res.status).toBe(204)
    expect(await res.text()).toBe('')
  })

  test('a worker error becomes a 500 in the standard envelope', async () => {
    const { runtime } = runtimeWith(new FakeLocks(), (msg) => {
      if (msg.op === 'request') throw new Error('boom')
      return okBehaviour(msg)
    })
    const res = await runtime.fetch('/api/health')
    expect(res.status).toBe(500)
    expect((await res.json()).error).toEqual({ code: 'RUNTIME_ERROR', message: 'boom' })
  })
})

// ── Export / import ─────────────────────────────────────────────────

describe('createRuntime: database export and import', () => {
  test('exports bytes from the worker', async () => {
    const { runtime } = runtimeWith(new FakeLocks())
    expect(Array.from(await runtime.exportDatabase())).toEqual([1, 2, 3])
  })

  test('imports bytes and reports what the worker did', async () => {
    const { runtime } = runtimeWith(new FakeLocks())
    expect(await runtime.importDatabase(new Uint8Array(10))).toEqual({ imported_bytes: 10, migrations: 52 })
  })

  test('a rejected import surfaces the worker message', async () => {
    const { runtime } = runtimeWith(new FakeLocks(), (msg) => {
      if (msg.op === 'import') throw new Error('import rejected, current data kept: not a database')
      return okBehaviour(msg)
    })
    await expect(runtime.importDatabase(new Uint8Array(3))).rejects.toThrow('current data kept')
  })
})

// ── installApiFetch ─────────────────────────────────────────────────

describe('installApiFetch', () => {
  const realFetch = globalThis.fetch
  afterEach(() => {
    globalThis.fetch = realFetch
  })

  test('routes same-origin /api/ calls to the runtime and everything else to the original fetch', async () => {
    const calls: string[] = []
    globalThis.fetch = (async (input: RequestInfo | URL) => {
      calls.push('original:' + String(input))
      return new Response('static')
    }) as typeof fetch
    // Under Bun there is no `location`; the wrapper then resolves against http://localhost/.
    const { runtime } = runtimeWith(new FakeLocks())
    const restore = installApiFetch(runtime)

    const api = await fetch('/api/health')
    expect(api.headers.get('x-echo-path')).toBe('/api/health')

    expect(await (await fetch('/geo/us-states.json')).text()).toBe('static')
    expect(await (await fetch('https://elsewhere.example/api/health')).text()).toBe('static')
    expect(calls).toEqual(['original:/geo/us-states.json', 'original:https://elsewhere.example/api/health'])

    restore()
    await fetch('/api/health')
    expect(calls.at(-1)).toBe('original:/api/health')
  })
})
