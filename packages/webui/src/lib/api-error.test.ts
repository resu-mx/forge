import { describe, expect, test } from 'bun:test'
import { responseError } from './api-error'

describe('responseError', () => {
  test('reads the error envelope', async () => {
    const message = 'GET /api/sources/s1/skills is not available in the browser runtime yet'
    const res = new Response(JSON.stringify({ error: { code: 'NOT_IMPLEMENTED', message } }), {
      status: 501,
      headers: { 'Content-Type': 'application/json' },
    })
    expect(await responseError(res)).toEqual({ code: 'NOT_IMPLEMENTED', message })
  })

  test('falls back to the status, without the word non-JSON', async () => {
    const e = await responseError(new Response('<html>Bad Gateway</html>', { status: 502 }))
    expect(e).toEqual({ code: 'UNKNOWN_ERROR', message: 'HTTP 502' })
    expect(e.message).not.toContain('non-JSON')
  })

  test('falls back when JSON has no usable envelope', async () => {
    const e = await responseError(new Response('{"error":"nope"}', { status: 500 }))
    expect(e).toEqual({ code: 'UNKNOWN_ERROR', message: 'HTTP 500' })
  })
})
