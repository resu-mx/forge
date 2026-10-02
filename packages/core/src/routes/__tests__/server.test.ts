import { describe, test, expect, beforeEach, afterEach, spyOn } from 'bun:test'
import { Database } from 'bun:sqlite'
import { createTestApp, apiRequest, type TestContext } from './helpers'

describe('Server', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  test('GET /health returns 200 with server ok and version', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/health')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(body.data.server).toBe('ok')
    expect(typeof body.data.version).toBe('string')
  })

  test('unknown route returns 404 with error envelope', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/nonexistent')
    expect(res.status).toBe(404)
    const body = await res.json()
    expect(body.error).toBeDefined()
    expect(body.error.code).toBe('NOT_FOUND')
    expect(body.error.message).toContain('Route not found')
  })

  test('X-Request-Id header is present on responses', async () => {
    const res = await apiRequest(ctx.app, 'GET', '/health')
    const requestId = res.headers.get('X-Request-Id')
    expect(requestId).toBeTruthy()
    expect(requestId).toHaveLength(36) // UUID v4
  })

  test('CORS headers are present when Origin matches', async () => {
    // Send a request with an Origin header that matches the dev CORS config
    const url = 'http://localhost/api/health'
    const res = await ctx.app.request(url, {
      method: 'GET',
      headers: { Origin: 'http://localhost:5173' },
    })
    const allowOrigin = res.headers.get('Access-Control-Allow-Origin')
    expect(allowOrigin).toBe('http://localhost:5173')
  })

  // Mocks performance.now and spies on this process's console: it tests the TS logger,
  // so it cannot apply when the server under test is a separate (Rust) process.
  test.skipIf(!!process.env.FORGE_TEST_SERVER_BIN)('slow request (>500ms) logged at warn level', async () => {
    // Mock performance.now to simulate a slow response
    const originalNow = performance.now
    let callCount = 0
    performance.now = () => {
      callCount++
      // First call (start): return 0
      // Second call (end): return 600 (>500ms)
      return callCount === 1 ? 0 : 600
    }

    const warnSpy = spyOn(console, 'warn').mockImplementation(() => {})

    const res = await apiRequest(ctx.app, 'GET', '/health')
    expect(res.status).toBe(200)

    // Should have logged at warn level due to >500ms duration
    expect(warnSpy).toHaveBeenCalled()

    warnSpy.mockRestore()
    performance.now = originalNow
  })
})
