import { describe, expect, test } from 'bun:test'
import { createRuntime, type LockManagerLike, type WorkerLike } from './client'
import type { PdfReply, PdfRequest, WorkerRequest } from './protocol'

// ── Fakes ───────────────────────────────────────────────────────────

class FakeWorker implements WorkerLike {
  onmessage: ((event: MessageEvent) => void) | null = null
  onerror: ((event: ErrorEvent) => void) | null = null
  received: unknown[] = []
  constructor(private behaviour: (msg: any) => unknown | Promise<unknown>) {}
  postMessage(message: unknown): void {
    this.received.push(message)
    const id = (message as { id: number }).id
    queueMicrotask(async () => {
      try {
        this.onmessage?.({ data: { id, ok: true, result: await this.behaviour(message) } } as MessageEvent)
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

const TYPST = '#set page(paper: "us-letter")\n= Ada\n'
const enc = (s: string) => new TextEncoder().encode(s)
const PDF = enc('%PDF-1.7 fake')

interface Setup {
  /** The database Worker's answer to GET /api/export/resume/:id?format=typst. */
  typst?: { status: number; headers: [string, string][]; body: Uint8Array }
  /** The Typst Worker's behaviour. */
  compile?: (msg: PdfRequest) => unknown
  noPdfWorker?: boolean
}

function setup(opts: Setup = {}) {
  const dbWorker = new FakeWorker((msg: WorkerRequest) => {
    if (msg.op === 'start') return { version: 't', migrations: 1 }
    if (msg.op === 'request') {
      if (msg.path.includes('format=typst')) {
        return (
          opts.typst ?? {
            status: 200,
            headers: [['content-disposition', 'attachment; filename="ada-2026-01-02.typ"']],
            body: enc(TYPST),
          }
        )
      }
      return { status: 200, headers: [], body: enc('{"data":"other"}') }
    }
  })
  let typstWorker: FakeWorker | undefined
  let created = 0
  const runtime = createRuntime({
    createWorker: () => dbWorker,
    createPdfWorker: opts.noPdfWorker
      ? undefined
      : () => {
          created++
          typstWorker = new FakeWorker((opts.compile ?? (() => PDF)) as (m: any) => unknown)
          return typstWorker
        },
    locks: oneLock,
    persist: async () => true,
    baseHref: 'http://forge.test/',
  })
  return { runtime, dbWorker, typst: () => typstWorker, created: () => created }
}

const compileRequests = (w: FakeWorker | undefined) => (w?.received ?? []) as PdfRequest[]

// ── PDF flow ────────────────────────────────────────────────────────

describe('PDF output in the browser runtime', () => {
  test('POST /api/resumes/:id/pdf compiles the generated Typst and answers as the server does', async () => {
    const { runtime, typst } = setup()
    const res = await runtime.fetch('/api/resumes/abc/pdf', { method: 'POST' })

    expect(res.status).toBe(200)
    expect(res.headers.get('content-type')).toBe('application/pdf')
    expect(res.headers.get('content-disposition')).toBe('inline; filename="resume.pdf"')
    expect(res.headers.get('x-forge-pdf-cache')).toBe('miss')
    expect(new TextDecoder().decode(await res.arrayBuffer())).toBe('%PDF-1.7 fake')
    expect(compileRequests(typst()).map((r) => r.source)).toEqual([TYPST])
  })

  test('the database Worker is asked for the Typst export of that resume', async () => {
    const { runtime, dbWorker } = setup()
    await runtime.fetch('/api/resumes/abc/pdf', { method: 'POST' })
    const asked = (dbWorker.received as WorkerRequest[]).find((m) => m.op === 'request') as Extract<WorkerRequest, { op: 'request' }>
    expect(asked.method).toBe('GET')
    expect(asked.path).toBe('/api/export/resume/abc?format=typst')
  })

  test('GET /api/export/resume/:id?format=pdf is an attachment named like the server names it', async () => {
    const { runtime } = setup()
    const res = await runtime.fetch('/api/export/resume/abc?format=pdf')
    expect(res.status).toBe(200)
    expect(res.headers.get('content-type')).toBe('application/pdf')
    expect(res.headers.get('content-disposition')).toBe('attachment; filename="ada-2026-01-02.pdf"')
  })

  test('the Typst compiler is created lazily, once, and not for ordinary requests', async () => {
    const { runtime, created } = setup()
    await runtime.fetch('/api/sources')
    await runtime.fetch('/api/export/resume/abc?format=markdown')
    expect(created()).toBe(0)

    await runtime.fetch('/api/resumes/abc/pdf', { method: 'POST' })
    await runtime.fetch('/api/resumes/abc/pdf', { method: 'POST' })
    expect(created()).toBe(1)
  })

  test('a latex_override notice from the server is passed through on the PDF', async () => {
    const { runtime } = setup({
      typst: {
        status: 200,
        headers: [['x-forge-pdf-notice', 'latex_override is not compiled: this PDF is generated from the resume content']],
        body: enc(TYPST),
      },
    })
    const res = await runtime.fetch('/api/resumes/abc/pdf', { method: 'POST' })
    expect(res.headers.get('x-forge-pdf-notice')).toContain('latex_override')
  })

  test('hand-written { typst } is compiled instead of the generated source (and the old { latex } is ignored)', async () => {
    const { runtime, typst, dbWorker } = setup()
    const res = await runtime.fetch('/api/resumes/abc/pdf', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ typst: '= Mine', latex: '\\documentclass{article}' }),
    })
    expect(res.status).toBe(200)
    expect(compileRequests(typst()).map((r) => r.source)).toEqual(['= Mine'])
    expect((dbWorker.received as WorkerRequest[]).some((m) => m.op === 'request' && m.path.includes('format=typst'))).toBe(false)
    expect(res.headers.get('x-forge-pdf-notice')).toBeNull()
  })

  test('an unknown resume is the server\'s 404, passed through without compiling', async () => {
    const { runtime, created } = setup({
      typst: { status: 404, headers: [['content-type', 'application/json']], body: enc('{"error":{"code":"NOT_FOUND","message":"nope"}}') },
    })
    const res = await runtime.fetch('/api/resumes/missing/pdf', { method: 'POST' })
    expect(res.status).toBe(404)
    expect((await res.json()).error.code).toBe('NOT_FOUND')
    expect(created()).toBe(0)
  })

  test('a document that does not compile is a 422 TYPST_COMPILE_ERROR with the diagnostics', async () => {
    const { runtime } = setup({
      compile: () => {
        throw new Error(JSON.stringify({ message: 'unexpected argument', details: ['unexpected argument'] }))
      },
    })
    const res = await runtime.fetch('/api/resumes/abc/pdf', { method: 'POST' })
    expect(res.status).toBe(422)
    expect((await res.json()).error).toEqual({
      code: 'TYPST_COMPILE_ERROR',
      message: 'unexpected argument',
      details: ['unexpected argument'],
    })
  })

  test('a compiler that cannot run (not a compile error) is a 500, not a 422', async () => {
    const { runtime } = setup({
      compile: () => {
        throw new Error('the Typst module failed to load')
      },
    })
    const res = await runtime.fetch('/api/resumes/abc/pdf', { method: 'POST' })
    expect(res.status).toBe(500)
    expect((await res.json()).error.code).toBe('PDF_UNAVAILABLE')
  })

  test('without a configured compiler PDFs are 501 and everything else still works', async () => {
    const { runtime } = setup({ noPdfWorker: true })
    const res = await runtime.fetch('/api/resumes/abc/pdf', { method: 'POST' })
    expect(res.status).toBe(501)
    expect((await res.json()).error.code).toBe('NOT_IMPLEMENTED')
    expect((await runtime.fetch('/api/sources')).status).toBe(200)
  })

  test('only the PDF routes are intercepted', async () => {
    const { runtime, created } = setup()
    // The Typst export and other formats go to the database Worker untouched.
    const typstRes = await runtime.fetch('/api/export/resume/abc?format=typst')
    expect(typstRes.status).toBe(200)
    expect((await runtime.fetch('/api/resumes/abc')).status).toBe(200)
    expect((await runtime.fetch('/api/resumes/abc/pdf')).status).toBe(200) // GET /pdf is not the POST route
    expect(created()).toBe(0)
  })

  test('concurrent PDFs are matched to their own requests', async () => {
    const { runtime } = setup({ compile: (m) => enc(`%PDF for ${m.source.length}`) })
    const [a, b] = await Promise.all([
      runtime.fetch('/api/resumes/a/pdf', { method: 'POST', body: JSON.stringify({ typst: 'x' }) }),
      runtime.fetch('/api/resumes/b/pdf', { method: 'POST', body: JSON.stringify({ typst: 'xxxxxxxx' }) }),
    ])
    expect(await a.text()).toBe('%PDF for 1')
    expect(await b.text()).toBe('%PDF for 8')
  })
})
