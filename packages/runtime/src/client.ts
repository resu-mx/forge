/**
 * Page-side client for the database Worker.
 *
 * - Elects one owner tab with a Web Lock. A second tab waits (and takes over when the
 *   owner closes); it does not fight over the database.
 * - Exposes a `fetch` that answers `/api/...` requests from the Worker, so the web UI
 *   and the TypeScript SDK work unchanged against the in-browser Rust API.
 */

import type { HeaderPairs, ImportInfo, RawResponse, StartInfo, WorkerReply, WorkerRequest } from './protocol'

export type RuntimeStatus =
  | { state: 'starting' }
  /** Another tab owns the database; this tab takes over when that tab closes. */
  | { state: 'waiting' }
  | { state: 'ready'; info: StartInfo; persisted: boolean | null }
  | { state: 'error'; message: string }

/** The slice of `Worker` the client uses (so tests can fake it). */
export interface WorkerLike {
  postMessage(message: unknown, transfer?: Transferable[]): void
  terminate(): void
  onmessage: ((event: MessageEvent) => void) | null
  onerror: ((event: ErrorEvent) => void) | null
}

/** The slice of `navigator.locks` the client uses. */
export interface LockManagerLike {
  request(name: string, options: { ifAvailable: true }, callback: (lock: unknown | null) => Promise<unknown>): Promise<unknown>
  request(name: string, callback: (lock: unknown) => Promise<unknown>): Promise<unknown>
}

export interface RuntimeOptions {
  /** May be async, so a bundler can load the Worker chunk lazily. */
  createWorker: () => WorkerLike | Promise<WorkerLike>
  /** Defaults to `navigator.locks`. */
  locks?: LockManagerLike
  /** Defaults to `navigator.storage.persist()`. */
  persist?: () => Promise<boolean>
  lockName?: string
  /** Defaults to `location.href`. */
  baseHref?: string
}

export interface ForgeRuntime {
  status(): RuntimeStatus
  onStatus(listener: (status: RuntimeStatus) => void): () => void
  /** Resolves once this tab owns the database and it is open; rejects on a start error. */
  ready: Promise<void>
  /** Serves `/api/...` from the Worker. Same signature as the platform `fetch`. */
  fetch: typeof fetch
  /** The whole database as a SQLite file. */
  exportDatabase(): Promise<Uint8Array>
  /** Replace the database with a SQLite file. Keeps the current data if the file is rejected. */
  importDatabase(bytes: Uint8Array): Promise<ImportInfo>
}

const NULL_BODY_STATUSES = new Set([101, 204, 205, 304])

function errorResponse(status: number, code: string, message: string): Response {
  return new Response(JSON.stringify({ error: { code, message } }), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

export function createRuntime(options: RuntimeOptions): ForgeRuntime {
  const locks = options.locks ?? (globalThis.navigator?.locks as LockManagerLike | undefined)
  const lockName = options.lockName ?? 'forge-database-owner'
  const baseHref = options.baseHref ?? globalThis.location?.href ?? 'http://localhost/'
  const persist =
    options.persist ?? (() => (globalThis.navigator?.storage?.persist?.() ?? Promise.resolve(false)))

  let status: RuntimeStatus = { state: 'starting' }
  const listeners = new Set<(s: RuntimeStatus) => void>()
  const setStatus = (next: RuntimeStatus) => {
    status = next
    for (const l of listeners) l(next)
  }

  let worker: WorkerLike | undefined
  let nextId = 1
  const pending = new Map<number, { resolve(v: unknown): void; reject(e: Error): void }>()

  function call<T>(msg: Omit<WorkerRequest, 'id'>, transfer: Transferable[] = []): Promise<T> {
    if (!worker) return Promise.reject(new Error('the Forge database is not open in this tab'))
    const id = nextId++
    return new Promise<T>((resolve, reject) => {
      pending.set(id, { resolve: resolve as (v: unknown) => void, reject })
      worker!.postMessage({ ...msg, id }, transfer)
    })
  }

  let resolveReady!: () => void
  let rejectReady!: (e: Error) => void
  const ready = new Promise<void>((res, rej) => {
    resolveReady = res
    rejectReady = rej
  })
  // A rejected `ready` nobody awaits yet must not surface as an unhandled rejection.
  ready.catch(() => {})

  /** This tab owns the database: spin up the Worker and open it. */
  async function own(): Promise<void> {
    try {
      worker = await options.createWorker()
      worker.onmessage = (event) => {
        const reply = event.data as WorkerReply
        const p = pending.get(reply.id)
        if (!p) return
        pending.delete(reply.id)
        if (reply.ok) p.resolve(reply.result)
        else p.reject(new Error(reply.error))
      }
      worker.onerror = (event) => {
        const message = event.message || 'the Forge database Worker failed'
        for (const p of pending.values()) p.reject(new Error(message))
        pending.clear()
        setStatus({ state: 'error', message })
      }

      const info = await call<StartInfo>({ op: 'start' })
      let persisted: boolean | null = null
      try {
        persisted = await persist()
      } catch {
        persisted = null
      }
      setStatus({ state: 'ready', info, persisted })
      resolveReady()
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e)
      setStatus({ state: 'error', message })
      rejectReady(new Error(message))
    }
  }

  // Hold the lock for the life of the tab: the callback never settles, and the browser
  // releases the lock when the tab closes or crashes, which hands over to a waiter.
  const hold = () => new Promise<never>(() => {})

  if (!locks) {
    // No Web Locks (very old browser): own the database and hope for the best.
    void own()
  } else {
    void locks.request(lockName, { ifAvailable: true }, async (lock) => {
      if (lock) {
        await own()
        return hold()
      }
      setStatus({ state: 'waiting' })
      void locks.request(lockName, async () => {
        setStatus({ state: 'starting' })
        await own()
        return hold()
      })
    })
  }

  async function runtimeFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
    const req =
      input instanceof Request
        ? new Request(input, init)
        : new Request(new URL(String(input), baseHref).toString(), init)
    const url = new URL(req.url, baseHref)

    if (status.state === 'waiting') {
      return errorResponse(503, 'STORAGE_BUSY', 'Forge is open in another tab. Close it, or switch to that tab.')
    }
    try {
      await ready
    } catch {
      // fall through to the status check below
    }
    if (status.state !== 'ready') {
      const message = status.state === 'error' ? status.message : 'the Forge database is not available'
      return errorResponse(503, 'STORAGE_UNAVAILABLE', message)
    }

    const body = new Uint8Array(await req.arrayBuffer())
    const headers: HeaderPairs = [...req.headers.entries()]
    try {
      const res = await call<RawResponse>(
        { op: 'request', method: req.method, path: url.pathname + url.search, headers, body },
        body.byteLength > 0 ? [body.buffer] : [],
      )
      return new Response(NULL_BODY_STATUSES.has(res.status) ? null : (res.body as BodyInit), {
        status: res.status,
        headers: res.headers,
      })
    } catch (e) {
      return errorResponse(500, 'RUNTIME_ERROR', e instanceof Error ? e.message : String(e))
    }
  }

  return {
    status: () => status,
    onStatus(listener) {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
    ready,
    fetch: runtimeFetch as typeof fetch,
    async exportDatabase() {
      await ready
      return call<Uint8Array>({ op: 'export' })
    },
    async importDatabase(bytes) {
      await ready
      return call<ImportInfo>({ op: 'import', bytes }, [bytes.buffer as ArrayBuffer])
    },
  }
}

/**
 * Route same-origin `/api/...` calls made with the global `fetch` to the runtime. This
 * covers the web UI's raw `fetch('/api/...')` calls as well as the SDK. Returns a function
 * that restores the original `fetch`.
 */
export function installApiFetch(runtime: ForgeRuntime, prefix = '/api/'): () => void {
  const original = globalThis.fetch
  const base = () => globalThis.location?.href ?? 'http://localhost/'

  globalThis.fetch = ((input: RequestInfo | URL, init?: RequestInit) => {
    const raw = input instanceof Request ? input.url : String(input)
    const url = new URL(raw, base())
    const sameOrigin = url.origin === new URL(base()).origin
    if (sameOrigin && url.pathname.startsWith(prefix)) return runtime.fetch(input, init)
    return original(input, init)
  }) as typeof fetch

  return () => {
    globalThis.fetch = original
  }
}
