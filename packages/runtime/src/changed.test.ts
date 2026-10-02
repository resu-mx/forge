import { describe, expect, test } from 'bun:test'
import { createRuntime, FORGE_CHANGED_EVENT, type ForgeChange, type LockManagerLike, type WorkerLike } from './client'

class FakeWorker implements WorkerLike {
  onmessage: ((event: MessageEvent) => void) | null = null
  onerror: ((event: ErrorEvent) => void) | null = null
  constructor(private reply: (msg: any) => unknown) {}
  postMessage(message: unknown): void {
    const id = (message as { id: number }).id
    queueMicrotask(() => {
      try {
        this.onmessage?.({ data: { id, ok: true, result: this.reply(message) } } as MessageEvent)
      } catch (e) {
        this.onmessage?.({ data: { id, ok: false, error: (e as Error).message } } as MessageEvent)
      }
    })
  }
  terminate() {}
}

const oneLock: LockManagerLike = {
  request(_name: string, a: unknown, b?: unknown) {
    const cb = (typeof a === 'function' ? a : b) as (lock: unknown) => Promise<unknown>
    return cb({})
  },
}

const enc = (s: string) => new TextEncoder().encode(s)

function setup(status = 200, extra: Partial<Parameters<typeof createRuntime>[0]> = {}) {
  const changes: ForgeChange[] = []
  const runtime = createRuntime({
    createWorker: () =>
      new FakeWorker((msg) => {
        if (msg.op === 'start') return { version: 't', migrations: 1 }
        if (msg.op === 'import') return { tables: 1, rows: 0 }
        return { status, headers: [], body: enc('{}') }
      }),
    locks: oneLock,
    persist: async () => true,
    baseHref: 'http://forge.test/',
    onChange: (c) => changes.push(c),
    ...extra,
  })
  return { runtime, changes }
}

describe('forge:changed', () => {
  test('a successful write is announced with its method and path', async () => {
    const { runtime, changes } = setup()
    await runtime.fetch('/api/sources', { method: 'POST', body: '{}' })
    await runtime.fetch('/api/bullets/b1/approve', { method: 'PATCH' })
    await runtime.fetch('/api/sources/s1', { method: 'DELETE' })
    expect(changes).toEqual([
      { method: 'POST', path: '/api/sources' },
      { method: 'PATCH', path: '/api/bullets/b1/approve' },
      { method: 'DELETE', path: '/api/sources/s1' },
    ])
  })

  test('reads, PDFs and derivation prepare are not changes', async () => {
    const { runtime, changes } = setup()
    await runtime.fetch('/api/sources')
    await runtime.fetch('/api/resumes/r1/pdf', { method: 'POST' })
    await runtime.fetch('/api/derivations/prepare', { method: 'POST', body: '{}' })
    expect(changes).toEqual([])
  })

  test('a failed write is not a change', async () => {
    const { runtime, changes } = setup(422)
    await runtime.fetch('/api/sources', { method: 'POST', body: '{}' })
    expect(changes).toEqual([])
  })

  test('importing a database is a change', async () => {
    const { runtime, changes } = setup()
    await runtime.importDatabase(new Uint8Array([1]))
    expect(changes).toEqual([{ method: 'IMPORT', path: '/database' }])
  })

  test('by default it is a DOM event on globalThis, and a throwing listener does not fail the request', async () => {
    const seen: unknown[] = []
    const g = globalThis as unknown as { dispatchEvent?: unknown }
    const had = g.dispatchEvent
    g.dispatchEvent = (e: CustomEvent) => {
      seen.push([e.type, e.detail])
      throw new Error('listener blew up')
    }
    try {
      const { runtime } = setup(200, { onChange: undefined })
      const res = await runtime.fetch('/api/sources', { method: 'POST', body: '{}' })
      expect(res.status).toBe(200)
      expect(seen).toEqual([[FORGE_CHANGED_EVENT, { method: 'POST', path: '/api/sources' }]])
    } finally {
      g.dispatchEvent = had
    }
  })
})
