import { describe, expect, it, beforeEach, afterEach, mock } from 'bun:test'
import { ForgeClient } from '../client'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build a minimal Response-like object that satisfies the fetch contract. */
function jsonResponse(
  body: unknown,
  init: { status?: number } = {},
): Response {
  const status = init.status ?? 200
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

function textResponse(body: string, status: number): Response {
  return new Response(body, {
    status,
    headers: { 'Content-Type': 'text/html' },
  })
}

function noContentResponse(): Response {
  return new Response(null, { status: 204 })
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe('ForgeClient', () => {
  const originalFetch = globalThis.fetch
  let fetchMock: ReturnType<typeof mock>
  let client: ForgeClient

  beforeEach(() => {
    fetchMock = mock(() => Promise.resolve(new Response()))
    globalThis.fetch = fetchMock as typeof fetch
    client = new ForgeClient({ baseUrl: 'http://localhost:3000' })
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  // -----------------------------------------------------------------------
  // Base URL normalization
  // -----------------------------------------------------------------------

  describe('base URL normalization', () => {
    it('strips a trailing slash', async () => {
      const c = new ForgeClient({ baseUrl: 'http://localhost:3000/' })
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: '1' } })),
      )

      await c.request('GET', '/api/sources')

      const calledUrl = (fetchMock.mock.calls[0] as [string])[0]
      expect(calledUrl).toBe('http://localhost:3000/api/sources')
    })

    it('strips multiple trailing slashes', async () => {
      const c = new ForgeClient({ baseUrl: 'http://localhost:3000///' })
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: '1' } })),
      )

      await c.request('GET', '/api/sources')

      const calledUrl = (fetchMock.mock.calls[0] as [string])[0]
      expect(calledUrl).toBe('http://localhost:3000/api/sources')
    })

    it('leaves a URL without trailing slash unchanged', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: { id: '1' } })),
      )

      await client.request('GET', '/api/sources')

      const calledUrl = (fetchMock.mock.calls[0] as [string])[0]
      expect(calledUrl).toBe('http://localhost:3000/api/sources')
    })
  })

  // -----------------------------------------------------------------------
  // request<T> — success
  // -----------------------------------------------------------------------

  describe('request<T> — success', () => {
    it('returns { ok: true, data } for a 200 JSON response', async () => {
      const payload = { id: 'abc', title: 'Test Source' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: payload })),
      )

      const result = await client.request<{ id: string; title: string }>(
        'GET',
        '/api/sources/abc',
      )

      expect(result).toEqual({ ok: true, data: payload })
    })

    it('returns { ok: true, data } for a 201 JSON response', async () => {
      const payload = { id: 'new', title: 'Created' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: payload }, { status: 201 })),
      )

      const result = await client.request<{ id: string; title: string }>(
        'POST',
        '/api/sources',
        { title: 'Created', description: 'A source' },
      )

      expect(result).toEqual({ ok: true, data: payload })
    })
  })

  // -----------------------------------------------------------------------
  // request<T> — 204 No Content
  // -----------------------------------------------------------------------

  describe('request<T> — 204 No Content', () => {
    it('returns { ok: true, data: undefined }', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(noContentResponse()),
      )

      const result = await client.request<void>('DELETE', '/api/sources/abc')

      expect(result.ok).toBe(true)
      if (result.ok) {
        expect(result.data).toBeUndefined()
      }
    })
  })

  // -----------------------------------------------------------------------
  // request<T> — error responses
  // -----------------------------------------------------------------------

  describe('request<T> — error responses', () => {
    it('returns { ok: false, error } for a 404', async () => {
      const error = { code: 'NOT_FOUND', message: 'Source not found: xyz' }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ error }, { status: 404 })),
      )

      const result = await client.request('GET', '/api/sources/xyz')

      expect(result).toEqual({ ok: false, error })
    })

    it('returns { ok: false, error } for a 400 validation error', async () => {
      const error = {
        code: 'VALIDATION_ERROR',
        message: 'title is required',
        details: { field: 'title' },
      }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ error }, { status: 400 })),
      )

      const result = await client.request('POST', '/api/sources', {})

      expect(result).toEqual({ ok: false, error })
    })

    it('returns { ok: false, error } for a 409 conflict', async () => {
      const error = {
        code: 'CONFLICT',
        message: 'Derivation already in progress',
      }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ error }, { status: 409 })),
      )

      const result = await client.request(
        'POST',
        '/api/sources/abc/derive-bullets',
      )

      expect(result).toEqual({ ok: false, error })
    })
  })

  // -----------------------------------------------------------------------
  // request<T> — unknown status / non-JSON
  // -----------------------------------------------------------------------

  describe('request<T> — unknown status with non-JSON body', () => {
    it('returns UNKNOWN_ERROR for a 503 with HTML body', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(textResponse('<html>Service Unavailable</html>', 503)),
      )

      const result = await client.request('GET', '/api/sources')

      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('UNKNOWN_ERROR')
        expect(result.error.message).toContain('503')
      }
    })

    it('returns UNKNOWN_ERROR for error response without error envelope', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ unexpected: true }, { status: 500 })),
      )

      const result = await client.request('GET', '/api/sources')

      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('UNKNOWN_ERROR')
        expect(result.error.message).toContain('500')
      }
    })
  })

  // -----------------------------------------------------------------------
  // request<T> — 2xx with an empty body
  // -----------------------------------------------------------------------

  describe('request<T> — 2xx with an empty body', () => {
    it('returns { ok: true, data: undefined } for a 201 with no body', async () => {
      fetchMock.mockImplementation(() => Promise.resolve(new Response(null, { status: 201 })))

      const result = await client.request<void>('POST', '/api/contacts/c1/organizations', {
        organization_id: 'o1',
        relationship: 'recruiter',
      })

      expect(result.ok).toBe(true)
      if (result.ok) expect(result.data).toBeUndefined()
    })

    it('treats a 200 with an empty string body as success', async () => {
      fetchMock.mockImplementation(() => Promise.resolve(new Response('', { status: 200 })))
      expect((await client.request('POST', '/api/x')).ok).toBe(true)
    })

    it('treats a 201 with a whitespace-only body as success', async () => {
      fetchMock.mockImplementation(() => Promise.resolve(new Response(' \n', { status: 201 })))
      expect((await client.request('POST', '/api/x')).ok).toBe(true)
    })

    it('still fails for a 500 with an empty body', async () => {
      fetchMock.mockImplementation(() => Promise.resolve(new Response(null, { status: 500 })))

      const result = await client.request('POST', '/api/x')

      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('UNKNOWN_ERROR')
        expect(result.error.message).toContain('500')
      }
    })

    it('still fails for a 200 with an HTML body', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(textResponse('<html>proxy</html>', 200)),
      )

      const result = await client.request('GET', '/api/x')

      expect(result.ok).toBe(false)
      if (!result.ok) expect(result.error.code).toBe('UNKNOWN_ERROR')
    })
  })

  // -----------------------------------------------------------------------
  // request<T> — network error
  // -----------------------------------------------------------------------

  describe('request<T> — network error', () => {
    it('returns NETWORK_ERROR when fetch throws', async () => {
      fetchMock.mockImplementation(() =>
        Promise.reject(new TypeError('Failed to fetch')),
      )

      const result = await client.request('GET', '/api/sources')

      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('NETWORK_ERROR')
        expect(result.error.message).toContain('Failed to fetch')
      }
    })
  })

  // -----------------------------------------------------------------------
  // request<T> — headers & body
  // -----------------------------------------------------------------------

  describe('request<T> — headers & body serialization', () => {
    it('sets Content-Type: application/json when body is provided', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: {} }, { status: 201 })),
      )

      await client.request('POST', '/api/sources', { title: 'Test' })

      const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
      const headers = init.headers as Record<string, string>
      expect(headers['Content-Type']).toBe('application/json')
    })

    it('does NOT set Content-Type when there is no body (GET)', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: {} })),
      )

      await client.request('GET', '/api/sources')

      const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
      const headers = init.headers as Record<string, string>
      expect(headers['Content-Type']).toBeUndefined()
    })

    it('serializes the body as JSON', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: {} }, { status: 201 })),
      )

      const body = { title: 'Hello', description: 'World' }
      await client.request('POST', '/api/sources', body)

      const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
      expect(init.body).toBe(JSON.stringify(body))
    })

    it('does not send a body for GET requests', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: {} })),
      )

      await client.request('GET', '/api/sources')

      const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
      expect(init.body).toBeUndefined()
    })

    it('passes the correct HTTP method', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data: {} })),
      )

      await client.request('PATCH', '/api/sources/abc', { title: 'Updated' })

      const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
      expect(init.method).toBe('PATCH')
    })
  })

  // -----------------------------------------------------------------------
  // requestList<T> — success
  // -----------------------------------------------------------------------

  describe('requestList<T> — success', () => {
    it('returns { ok: true, data, pagination } for a paginated response', async () => {
      const data = [
        { id: '1', title: 'Source A' },
        { id: '2', title: 'Source B' },
      ]
      const pagination = { total: 42, offset: 0, limit: 50 }

      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ data, pagination })),
      )

      const result = await client.requestList<{ id: string; title: string }>(
        'GET',
        '/api/sources',
      )

      expect(result).toEqual({ ok: true, data, pagination })
    })
  })

  // -----------------------------------------------------------------------
  // requestList<T> — error
  // -----------------------------------------------------------------------

  describe('requestList<T> — error', () => {
    it('returns { ok: false, error } for a 400', async () => {
      const error = {
        code: 'VALIDATION_ERROR',
        message: 'Invalid filter parameter',
      }
      fetchMock.mockImplementation(() =>
        Promise.resolve(jsonResponse({ error }, { status: 400 })),
      )

      const result = await client.requestList(
        'GET',
        '/api/sources',
        { status: 'invalid' },
      )

      expect(result).toEqual({ ok: false, error })
    })

    it('returns NETWORK_ERROR when fetch throws', async () => {
      fetchMock.mockImplementation(() =>
        Promise.reject(new Error('Network down')),
      )

      const result = await client.requestList('GET', '/api/sources')

      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('NETWORK_ERROR')
      }
    })

    it('returns UNKNOWN_ERROR for non-JSON error response', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(textResponse('Bad Gateway', 502)),
      )

      const result = await client.requestList('GET', '/api/sources')

      expect(result.ok).toBe(false)
      if (!result.ok) {
        expect(result.error.code).toBe('UNKNOWN_ERROR')
        expect(result.error.message).toContain('502')
      }
    })
  })

  // -----------------------------------------------------------------------
  // requestList<T> — query string serialization
  // -----------------------------------------------------------------------

  describe('requestList<T> — query string', () => {
    it('appends params as a query string', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.requestList('GET', '/api/sources', {
        offset: '10',
        limit: '25',
        status: 'approved',
      })

      const calledUrl = (fetchMock.mock.calls[0] as [string])[0]
      expect(calledUrl).toContain('offset=10')
      expect(calledUrl).toContain('limit=25')
      expect(calledUrl).toContain('status=approved')
    })

    it('does not append ? when params are empty', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.requestList('GET', '/api/sources', {})

      const calledUrl = (fetchMock.mock.calls[0] as [string])[0]
      expect(calledUrl).toBe('http://localhost:3000/api/sources')
    })

    it('does not append ? when params are undefined', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.requestList('GET', '/api/sources')

      const calledUrl = (fetchMock.mock.calls[0] as [string])[0]
      expect(calledUrl).toBe('http://localhost:3000/api/sources')
    })

    it('URL-encodes special characters in params', async () => {
      fetchMock.mockImplementation(() =>
        Promise.resolve(
          jsonResponse({
            data: [],
            pagination: { total: 0, offset: 0, limit: 50 },
          }),
        ),
      )

      await client.requestList('GET', '/api/sources', {
        search: 'hello world&more',
      })

      const calledUrl = (fetchMock.mock.calls[0] as [string])[0]
      // URLSearchParams encodes spaces as + and & as %26
      expect(calledUrl).toContain('search=hello+world%26more')
    })
  })

  // -----------------------------------------------------------------------
  // Debug logging
  // -----------------------------------------------------------------------

  describe('debug logging', () => {
    let debugClient: ForgeClient

    beforeEach(() => {
      fetchMock = mock(() =>
        Promise.resolve(
          jsonResponse({ data: { id: '1' } }, { status: 200 }),
        ),
      )
      globalThis.fetch = fetchMock as typeof fetch
      debugClient = new ForgeClient({
        baseUrl: 'http://localhost:3000',
        debug: { logToConsole: false },
      })
    })

    it('captures successful request in debug store', async () => {
      await debugClient.sources.get('abc')
      const entries = debugClient.debug.getAll()
      expect(entries.length).toBeGreaterThanOrEqual(1)
      const last = entries[entries.length - 1]
      expect(last.method).toBe('GET')
      expect(last.path).toBe('/api/sources/abc')
      expect(last.ok).toBe(true)
      expect(last.duration_ms).toBeGreaterThanOrEqual(0)
    })

    it('logs an empty 201 as ok with the real status', async () => {
      globalThis.fetch = mock(() =>
        Promise.resolve(new Response(null, { status: 201 })),
      ) as typeof fetch

      await debugClient.request('POST', '/api/contacts/c1/organizations', {})

      const entries = debugClient.debug.getAll()
      const last = entries[entries.length - 1]
      expect(last.ok).toBe(true)
      expect(last.status).toBe(201)
    })

    it('captures error response in debug store', async () => {
      fetchMock = mock(() =>
        Promise.resolve(
          jsonResponse(
            { error: { code: 'NOT_FOUND', message: 'Source not found' } },
            { status: 404 },
          ),
        ),
      )
      globalThis.fetch = fetchMock as typeof fetch
      await debugClient.sources.get('missing')
      const errors = debugClient.debug.getErrors()
      expect(errors).toHaveLength(1)
      expect(errors[0].error_code).toBe('NOT_FOUND')
      expect(errors[0].status).toBe(404)
    })

    it('captures network error in debug store', async () => {
      fetchMock = mock(() => Promise.reject(new Error('fetch failed')))
      globalThis.fetch = fetchMock as typeof fetch
      await debugClient.sources.get('abc')
      const errors = debugClient.debug.getErrors()
      expect(errors).toHaveLength(1)
      expect(errors[0].direction).toBe('error')
      expect(errors[0].error_code).toBe('NETWORK_ERROR')
    })

    it('requestList() captures pagination fields in debug store', async () => {
      globalThis.fetch = mock(() =>
        Promise.resolve(
          jsonResponse({
            data: [{ id: '1' }],
            pagination: { total: 42, offset: 0, limit: 50 },
          }),
        ),
      ) as typeof fetch

      const c = new ForgeClient({
        baseUrl: 'http://test',
        debug: { logToConsole: false },
      })
      await c.sources.list()

      const entries = c.debug.getAll()
      expect(entries).toHaveLength(1)
      expect(entries[0].pagination_total).toBe(42)
      expect(entries[0].pagination_offset).toBe(0)
      expect(entries[0].pagination_limit).toBe(50)
    })

    it('captures X-Request-Id from response header', async () => {
      globalThis.fetch = mock(() =>
        Promise.resolve(
          new Response(JSON.stringify({ data: { id: '1' } }), {
            status: 200,
            headers: {
              'Content-Type': 'application/json',
              'X-Request-Id': 'test-req-123',
            },
          }),
        ),
      ) as typeof fetch

      const c = new ForgeClient({
        baseUrl: 'http://test',
        debug: { logToConsole: false },
      })
      await c.sources.get('1')

      const entries = c.debug.getAll()
      expect(entries[0].request_id).toBe('test-req-123')
    })

    it('debug store is disabled when debug: false', () => {
      const c = new ForgeClient({
        baseUrl: 'http://test',
        debug: false,
      })
      expect(c.debug.enabled).toBe(false)
    })

    it('request captures request_body_size for POST', async () => {
      fetchMock = mock(() =>
        Promise.resolve(
          jsonResponse({ data: { id: 'new' } }, { status: 201 }),
        ),
      )
      globalThis.fetch = fetchMock as typeof fetch

      const body = { title: 'Test', description: 'A source' }
      await debugClient.request('POST', '/api/sources', body)

      const entries = debugClient.debug.getAll()
      expect(entries).toHaveLength(1)
      expect(entries[0].request_body_size).toBe(
        JSON.stringify(body).length,
      )
    })
  })
})
