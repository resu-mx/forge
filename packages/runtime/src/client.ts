/**
 * Page-side client for the database Worker.
 *
 * - Elects one owner tab with a Web Lock. A second tab waits (and takes over when the
 *   owner closes); it does not fight over the database.
 * - Exposes a `fetch` that answers `/api/...` requests from the Worker, so the web UI
 *   and the TypeScript SDK work unchanged against the in-browser Rust API.
 */

import type {
  HeaderPairs,
  ImportInfo,
  PdfReply,
  PdfRequest,
  RawResponse,
  StartInfo,
  WorkerReply,
  WorkerRequest,
} from './protocol'

export type RuntimeStatus =
  | { state: 'starting' }
  /** Another tab owns the database; this tab takes over when that tab closes. */
  | { state: 'waiting' }
  | { state: 'ready'; info: StartInfo; persisted: boolean | null }
  | { state: 'error'; message: string }

/** The slice of `Worker` the client uses (so tests can fake it). */
/** `Omit<T, 'id'>` applied to each member of a union (a plain Omit would keep only the shared keys). */
type WithoutId<T> = T extends unknown ? Omit<T, 'id'> : never

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
  /**
   * The Typst compiler Worker, created on the first PDF request (it is a 25 MB module). Without
   * it, PDF requests answer 501 and everything else works.
   */
  createPdfWorker?: () => WorkerLike | Promise<WorkerLike>
  /** Defaults to `navigator.locks`. */
  locks?: LockManagerLike
  /** Defaults to `navigator.storage.persist()`. */
  persist?: () => Promise<boolean>
  lockName?: string
  /** Defaults to `location.href`. */
  baseHref?: string
  /**
   * Called after a request that changed stored data succeeded. Defaults to dispatching a
   * `forge:changed` event on `globalThis`, which is how the UI (and an agent driving the page)
   * learns that data moved underneath it.
   */
  onChange?: (change: ForgeChange) => void
}

/** What `forge:changed` carries in `event.detail`. */
export interface ForgeChange {
  method: string
  path: string
}

export const FORGE_CHANGED_EVENT = 'forge:changed'

/** Requests that use a write method but leave stored data as it was. */
function changesData(method: string, pathname: string): boolean {
  if (method === 'GET' || method === 'HEAD' || method === 'OPTIONS') return false
  if (pathname.endsWith('/pdf')) return false
  // Preparing a derivation records the request, and nothing a list would show.
  if (pathname === '/api/derivations/prepare') return false
  return true
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

  function call<T>(msg: WithoutId<WorkerRequest>, transfer: Transferable[] = []): Promise<T> {
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

  // ── PDF: the database Worker supplies Typst source, the Typst Worker compiles it ──

  let pdfWorker: Promise<WorkerLike> | undefined
  let nextPdfId = 1
  const pdfPending = new Map<number, { resolve(v: Uint8Array): void; reject(e: Error): void }>()

  /** Start the Typst Worker once, on first use. */
  function ensurePdfWorker(): Promise<WorkerLike> {
    pdfWorker ??= (async () => {
      if (!options.createPdfWorker) throw new Error('PDF output is not configured in this build')
      const w = await options.createPdfWorker()
      w.onmessage = (event) => {
        const reply = event.data as PdfReply
        const p = pdfPending.get(reply.id)
        if (!p) return
        pdfPending.delete(reply.id)
        if (reply.ok) p.resolve(reply.result)
        else p.reject(new Error(reply.error))
      }
      w.onerror = (event) => {
        const message = event.message || 'the Typst compiler failed to load'
        for (const p of pdfPending.values()) p.reject(new Error(message))
        pdfPending.clear()
        pdfWorker = undefined // allow a retry
      }
      return w
    })()
    // A failed start must not be cached forever.
    pdfWorker.catch(() => {
      pdfWorker = undefined
    })
    return pdfWorker
  }

  async function compilePdf(source: string): Promise<Uint8Array> {
    const w = await ensurePdfWorker()
    const id = nextPdfId++
    return new Promise<Uint8Array>((resolve, reject) => {
      pdfPending.set(id, { resolve, reject })
      w.postMessage({ id, op: 'compile', source } satisfies PdfRequest)
    })
  }

  /** The `{ message, details }` forge-typst throws for a bad document; null for any other failure. */
  function parseCompileError(raw: string): { message: string; details: string[] } | null {
    try {
      const v = JSON.parse(raw)
      if (v && typeof v.message === 'string') return { message: v.message, details: Array.isArray(v.details) ? v.details : [] }
    } catch {
      // not JSON
    }
    return null
  }

  /**
   * A PDF in the browser: fetch the Typst source from the database Worker (the same
   * `?format=typst` the server exposes), compile it in the Typst Worker, answer as the
   * server's PDF route would. Returns null when `url` is not a PDF request.
   */
  async function maybeServePdf(method: string, url: URL, body: Uint8Array): Promise<Response | null> {
    const post = method === 'POST' && /^\/api\/resumes\/([^/]+)\/pdf$/.exec(url.pathname)
    const exportGet =
      method === 'GET' && url.searchParams.get('format') === 'pdf' && /^\/api\/export\/resume\/([^/]+)$/.exec(url.pathname)
    const match = post || exportGet
    if (!match) return null
    const resumeId = match[1]

    let source: string | undefined
    let notice: string | null = null
    let disposition = 'inline; filename="resume.pdf"'

    // POST may carry hand-written source: { "typst": "..." } (the old { latex } is ignored).
    if (post && body.byteLength > 0) {
      try {
        const supplied = JSON.parse(new TextDecoder().decode(body))?.typst
        if (typeof supplied === 'string') source = supplied
      } catch {
        // no usable body: compile the generated source
      }
    }

    if (source === undefined) {
      const res = await call<RawResponse>({
        op: 'request',
        method: 'GET',
        path: `/api/export/resume/${resumeId}?format=typst`,
        headers: [],
        body: new Uint8Array(),
      })
      if (res.status !== 200) {
        // 404 and the like: pass the server's answer through unchanged.
        return new Response(NULL_BODY_STATUSES.has(res.status) ? null : (res.body as BodyInit), {
          status: res.status,
          headers: res.headers,
        })
      }
      source = new TextDecoder().decode(res.body)
      const header = (name: string) => res.headers.find(([k]) => k.toLowerCase() === name)?.[1] ?? null
      notice = header('x-forge-pdf-notice')
      if (exportGet) disposition = (header('content-disposition') ?? disposition).replace(/\.typ"/, '.pdf"')
    }

    try {
      const pdf = await compilePdf(source)
      const headers: Record<string, string> = {
        'content-type': 'application/pdf',
        'content-disposition': disposition,
        'x-forge-pdf-cache': 'miss',
      }
      if (notice) headers['x-forge-pdf-notice'] = notice
      return new Response(pdf as BodyInit, { status: 200, headers })
    } catch (e) {
      const raw = e instanceof Error ? e.message : String(e)
      if (!options.createPdfWorker) {
        return errorResponse(501, 'NOT_IMPLEMENTED', 'PDF output is not available in this build')
      }
      const compileError = parseCompileError(raw)
      if (compileError) {
        // The document did not compile: the same shape and status as the server's.
        return new Response(JSON.stringify({ error: { code: 'TYPST_COMPILE_ERROR', ...compileError } }), {
          status: 422,
          headers: { 'content-type': 'application/json' },
        })
      }
      // The compiler itself could not run (for example its module failed to load).
      return errorResponse(500, 'PDF_UNAVAILABLE', raw)
    }
  }

  function announceChange(change: ForgeChange) {
    try {
      if (options.onChange) options.onChange(change)
      else globalThis.dispatchEvent?.(new CustomEvent(FORGE_CHANGED_EVENT, { detail: change }))
    } catch {
      // a listener's failure must not fail the request that already succeeded
    }
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
      const pdf = await maybeServePdf(req.method, url, body)
      if (pdf) return pdf
      const res = await call<RawResponse>(
        { op: 'request', method: req.method, path: url.pathname + url.search, headers, body },
        body.byteLength > 0 ? [body.buffer] : [],
      )
      if (res.status < 400 && changesData(req.method, url.pathname)) announceChange({ method: req.method, path: url.pathname })
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
      const info = await call<ImportInfo>({ op: 'import', bytes }, [bytes.buffer as ArrayBuffer])
      announceChange({ method: 'IMPORT', path: '/database' })
      return info
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
