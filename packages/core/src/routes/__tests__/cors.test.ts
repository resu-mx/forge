// packages/core/src/routes/__tests__/cors.test.ts

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, type TestContext } from './helpers'
import { parseCorsOrigins } from '../cors-origins'

describe('CORS', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  test('allows localhost:5173 origin (webui)', async () => {
    const res = await ctx.app.request('/api/health', {
      method: 'GET',
      headers: { Origin: 'http://localhost:5173' },
    })
    expect(res.status).toBe(200)
    expect(res.headers.get('access-control-allow-origin')).toBe('http://localhost:5173')
  })

  test('allows chrome-extension://<id> origin', async () => {
    const extOrigin = 'chrome-extension://abcdef1234567890abcdef1234567890'
    const res = await ctx.app.request('/api/health', {
      method: 'GET',
      headers: { Origin: extOrigin },
    })
    expect(res.status).toBe(200)
    expect(res.headers.get('access-control-allow-origin')).toBe(extOrigin)
  })

  test('allows moz-extension://<id> origin', async () => {
    const extOrigin = 'moz-extension://12345678-1234-1234-1234-123456789abc'
    const res = await ctx.app.request('/api/health', {
      method: 'GET',
      headers: { Origin: extOrigin },
    })
    expect(res.status).toBe(200)
    expect(res.headers.get('access-control-allow-origin')).toBe(extOrigin)
  })

  test('allows 127.0.0.1:5173 origin (webui alternate)', async () => {
    const res = await ctx.app.request('/api/health', {
      method: 'GET',
      headers: { Origin: 'http://127.0.0.1:5173' },
    })
    expect(res.status).toBe(200)
    expect(res.headers.get('access-control-allow-origin')).toBe('http://127.0.0.1:5173')
  })

  test('allows chrome-extension://<id> preflight', async () => {
    const extOrigin = 'chrome-extension://abcdef1234567890abcdef1234567890'
    const res = await ctx.app.request('/api/organizations', {
      method: 'OPTIONS',
      headers: {
        Origin: extOrigin,
        'Access-Control-Request-Method': 'POST',
        'Access-Control-Request-Headers': 'content-type',
      },
    })
    expect(res.status).toBe(204)
    expect(res.headers.get('access-control-allow-origin')).toBe(extOrigin)
    expect(res.headers.get('access-control-allow-methods')).toContain('POST')
  })

  test('rejects unknown cross-origin', async () => {
    const res = await ctx.app.request('/api/health', {
      method: 'GET',
      headers: { Origin: 'http://evil.example.com' },
    })
    // Hono cors() without matching origin sends request through but omits
    // the Allow-Origin header, so browser blocks it client-side.
    expect(res.status).toBe(200)
    expect(res.headers.get('access-control-allow-origin')).toBeNull()
  })
})

// ── FORGE_CORS_ORIGINS (resu-mx/forge#126) ──────────────────────────────────
//
// An operator's list of exact origins, or `*`, replaces the built-in policy in dev and in
// production. Unset or blank keeps the defaults. Both runtimes read the variable, so these tests
// run against the Rust server under `just parity` too: the server inherits this process's
// environment when `createTestApp()` starts it, so the variable is set first.

describe('parseCorsOrigins', () => {
  test('unset and blank values are "not configured", so the built-in policy applies', () => {
    expect(parseCorsOrigins(undefined)).toBeNull()
    for (const blank of ['', '   ', ',', ' , ,, ', '/', ' / ']) {
      expect(parseCorsOrigins(blank)).toBeNull()
    }
  })

  test('a * entry means any origin, wherever it appears', () => {
    expect(parseCorsOrigins('*')).toBe('*')
    expect(parseCorsOrigins('http://a.test, *')).toBe('*')
  })

  test('entries are trimmed, empty ones dropped, a trailing slash tolerated, and the rest kept as written', () => {
    expect(parseCorsOrigins('http://localhost:4173')).toEqual(['http://localhost:4173'])
    expect(parseCorsOrigins(' http://localhost:4173/ , ,chrome-extension://abc//')).toEqual([
      'http://localhost:4173',
      'chrome-extension://abc',
    ])
    expect(parseCorsOrigins('HTTP://Example.com')).toEqual(['HTTP://Example.com'])
  })
})

describe('FORGE_CORS_ORIGINS', () => {
  const VARS = ['FORGE_CORS_ORIGINS', 'NODE_ENV', 'FORGE_ENV'] as const
  let saved: Record<string, string | undefined>
  let ctx: TestContext | undefined

  beforeEach(() => {
    saved = Object.fromEntries(VARS.map((k) => [k, process.env[k]]))
    for (const k of VARS) delete process.env[k]
    ctx = undefined
  })

  afterEach(() => {
    ctx?.db.close()
    for (const k of VARS) {
      if (saved[k] === undefined) delete process.env[k]
      else process.env[k] = saved[k]
    }
  })

  /** Start an app with these variables set (the Rust server inherits them at start). */
  function appWith(env: Partial<Record<(typeof VARS)[number], string>>) {
    Object.assign(process.env, env)
    ctx = createTestApp()
    return ctx.app
  }

  /** The `access-control-allow-origin` a GET from `origin` gets back. The request is served either way. */
  async function allowOrigin(app: TestContext['app'], origin: string): Promise<string | null> {
    const res = await app.request('/api/health', { method: 'GET', headers: { Origin: origin } })
    expect(res.status).toBe(200)
    return res.headers.get('access-control-allow-origin')
  }

  test('a configured list replaces the built-in one', async () => {
    const app = appWith({ FORGE_CORS_ORIGINS: 'http://localhost:4173,chrome-extension://abc' })

    expect(await allowOrigin(app, 'http://localhost:4173')).toBe('http://localhost:4173')
    expect(await allowOrigin(app, 'chrome-extension://abc')).toBe('chrome-extension://abc')
    // The built-in dev origins are gone: the operator's list replaced them.
    expect(await allowOrigin(app, 'http://localhost:5173')).toBeNull()
    expect(await allowOrigin(app, 'chrome-extension://other')).toBeNull()
    expect(await allowOrigin(app, 'https://evil.example')).toBeNull()
  })

  test('a preflight from a listed origin is a 204 with the allow header; from another, none', async () => {
    const app = appWith({ FORGE_CORS_ORIGINS: 'http://localhost:4173' })
    const preflight = (origin: string) =>
      app.request('/api/sources', {
        method: 'OPTIONS',
        headers: {
          Origin: origin,
          'Access-Control-Request-Method': 'POST',
          'Access-Control-Request-Headers': 'content-type',
        },
      })

    const allowed = await preflight('http://localhost:4173')
    expect(allowed.status).toBe(204)
    expect(allowed.headers.get('access-control-allow-origin')).toBe('http://localhost:4173')

    const refused = await preflight('https://evil.example')
    expect(refused.headers.get('access-control-allow-origin')).toBeNull()
  })

  test('* allows every origin', async () => {
    const app = appWith({ FORGE_CORS_ORIGINS: '*' })
    expect(await allowOrigin(app, 'https://anything.example')).toBe('*')
  })

  test('a blank value keeps the defaults', async () => {
    const app = appWith({ FORGE_CORS_ORIGINS: ' , ' })
    expect(await allowOrigin(app, 'http://localhost:5173')).toBe('http://localhost:5173')
    expect(await allowOrigin(app, 'https://evil.example')).toBeNull()
  })

  test('whitespace, empty entries and a trailing slash in the list are tolerated', async () => {
    const app = appWith({ FORGE_CORS_ORIGINS: ' http://localhost:4173/ , ,chrome-extension://abc//' })
    expect(await allowOrigin(app, 'http://localhost:4173')).toBe('http://localhost:4173')
    expect(await allowOrigin(app, 'chrome-extension://abc')).toBe('chrome-extension://abc')
    expect(await allowOrigin(app, 'https://evil.example')).toBeNull()
  })

  test('production without the variable still allows any origin', async () => {
    const app = appWith({ NODE_ENV: 'production' })
    expect(await allowOrigin(app, 'https://anything.example')).toBe('*')
  })

  test('production with a list narrows to exactly that list', async () => {
    const app = appWith({ NODE_ENV: 'production', FORGE_CORS_ORIGINS: 'https://app.example' })
    expect(await allowOrigin(app, 'https://app.example')).toBe('https://app.example')
    expect(await allowOrigin(app, 'https://other.example')).toBeNull()
  })
})
