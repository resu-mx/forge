// @ts-nocheck -- bun:test types are not resolvable by svelte-check; keeps `bun run check` at baseline
import { describe, expect, test } from 'bun:test'
import {
  SIGNUP_NOT_ALLOWED,
  SIGNUP_REFUSED_MESSAGE,
  createSession,
  initialSnapshot,
  readAuthReturn,
} from './session-core'

const ID = 'https://id.resu.mx'
const ME = { user: { id: 'u1', email: 'ada@example.test' }, storageId: '5f0e7c2a-0000-4000-8000-000000000001' }

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } })
}

type Call = { url: string; init?: RequestInit }

/** A session over a scripted `fetch`: `routes` maps "METHOD path" to a response or a function. */
function harness(
  routes: Record<string, Response | (() => Response | Promise<Response>)>,
  opts: { available?: boolean; href?: string } = {},
) {
  const calls: Call[] = []
  const toasts: { message: string; type?: string }[] = []
  const navigations: string[] = []
  const replaced: string[] = []
  const changes: unknown[] = []
  let href = opts.href ?? 'https://app.resu.mx/resumes?tab=a'

  const session = createSession({
    idUrl: ID,
    available: opts.available ?? true,
    fetch: async (input, init) => {
      const url = String(input)
      calls.push({ url, init })
      const key = `${init?.method ?? 'GET'} ${url.replace(ID, '')}`
      const route = routes[key]
      if (!route) throw new Error(`unexpected request ${key}`)
      return typeof route === 'function' ? route() : route.clone()
    },
    getHref: () => href,
    navigate: (url) => navigations.push(url),
    replaceUrl: (url) => {
      replaced.push(url)
      href = url
    },
    notify: (t) => toasts.push(t),
    onChange: (s) => changes.push(s),
  })
  return { session, calls, toasts, navigations, replaced, changes }
}

describe('refresh: GET /api/me', () => {
  test('a 200 puts the session in, with the user and storageId, sending the cookie', async () => {
    const h = harness({ 'GET /api/me': json(ME) })
    expect(h.session.get().state).toBe('unknown')
    await h.session.refresh()
    expect(h.session.get()).toMatchObject({ state: 'in', user: ME.user, storageId: ME.storageId, error: null })
    expect(h.calls).toHaveLength(1)
    expect(h.calls[0].url).toBe(`${ID}/api/me`)
    expect(h.calls[0].init?.credentials).toBe('include')
  })

  test('a 401 puts the session out', async () => {
    const h = harness({ 'GET /api/me': json({ error: 'unauthorized' }, 401) })
    await h.session.refresh()
    expect(h.session.get()).toMatchObject({ state: 'out', user: null, storageId: null })
    expect(h.toasts).toEqual([])
  })

  test('a 401 after being signed in drops the user', async () => {
    let status = 200
    const h = harness({ 'GET /api/me': () => (status === 200 ? json(ME) : json({ error: 'unauthorized' }, 401)) })
    await h.session.refresh()
    status = 401
    await h.session.refresh()
    expect(h.session.get()).toMatchObject({ state: 'out', user: null, storageId: null })
  })

  test('a network error keeps the state unknown and shows nothing', async () => {
    const h = harness({
      'GET /api/me': () => {
        throw new TypeError('Failed to fetch')
      },
    })
    await h.session.refresh()
    expect(h.session.get().state).toBe('unknown')
    expect(h.toasts).toEqual([])
    expect(h.session.get().error).toBeNull()
  })

  test('a network blip after being signed in does not sign the person out', async () => {
    let up = true
    const h = harness({
      'GET /api/me': () => {
        if (!up) throw new TypeError('Failed to fetch')
        return json(ME)
      },
    })
    await h.session.refresh()
    up = false
    await h.session.refresh()
    expect(h.session.get().state).toBe('in')
  })

  test('a 5xx or a malformed body leaves the state alone', async () => {
    const down = harness({ 'GET /api/me': json({ error: 'bad_gateway' }, 502) })
    await down.session.refresh()
    expect(down.session.get().state).toBe('unknown')

    const odd = harness({ 'GET /api/me': json({ user: null, storageId: 7 }) })
    await odd.session.refresh()
    expect(odd.session.get().state).toBe('unknown')
    expect(odd.toasts).toEqual([])
  })

  test('only the newest answer lands: a slow stale one cannot undo a newer one', async () => {
    const slow: { resolve?: (r: Response) => void } = {}
    let n = 0
    const h = harness({
      'GET /api/me': () => {
        n += 1
        return n === 1 ? new Promise<Response>((resolve) => (slow.resolve = resolve)) : json({ error: 'x' }, 401)
      },
    })
    const first = h.session.refresh()
    await h.session.refresh() // newer: 401, out
    slow.resolve(json(ME)) // older: would say "in"
    await first
    expect(h.session.get().state).toBe('out')
  })

  test('reports every change to the store', async () => {
    const h = harness({ 'GET /api/me': json(ME) })
    await h.session.refresh()
    expect(h.changes.at(-1)).toBe(h.session.get())
  })
})

describe('signed out by configuration', () => {
  test('exposes out and available:false, and never touches the network', async () => {
    const h = harness({}, { available: false })
    expect(h.session.get()).toEqual(initialSnapshot(false))
    expect(h.session.get()).toMatchObject({ available: false, state: 'out' })
    await h.session.refresh()
    await h.session.signIn()
    await h.session.signOut()
    expect(h.calls).toEqual([])
    expect(h.navigations).toEqual([])
    expect(h.toasts).toEqual([])
    expect(h.changes).toEqual([])
  })

  test('a refusal on the URL is ignored there too: no toast, no URL rewrite', async () => {
    const h = harness({}, { available: false, href: `https://x.pages.dev/?error=${SIGNUP_NOT_ALLOWED}` })
    await h.session.refresh()
    expect(h.toasts).toEqual([])
    expect(h.replaced).toEqual([])
  })
})

describe('signIn: POST /api/auth/sign-in/oauth2', () => {
  const auth0 = 'https://example.us.auth0.com/authorize?client_id=c&state=s'

  test('posts the provider and callback URL, then navigates to the returned url', async () => {
    const h = harness({ 'POST /api/auth/sign-in/oauth2': json({ url: auth0, redirect: true }) })
    await h.session.signIn()
    const call = h.calls[0]
    expect(call.url).toBe(`${ID}/api/auth/sign-in/oauth2`)
    expect(call.init?.method).toBe('POST')
    expect(call.init?.credentials).toBe('include')
    expect(call.init?.headers).toEqual({ 'Content-Type': 'application/json' })
    const body = JSON.parse(String(call.init?.body))
    expect(body).toMatchObject({ providerId: 'auth0', callbackURL: 'https://app.resu.mx/resumes?tab=a' })
    expect(h.navigations).toEqual([auth0])
  })

  test('the callback URL is the current page without a stale error parameter', async () => {
    const h = harness(
      { 'POST /api/auth/sign-in/oauth2': json({ url: auth0 }) },
      { href: `https://app.resu.mx/resumes?tab=a&error=${SIGNUP_NOT_ALLOWED}` },
    )
    await h.session.signIn()
    expect(JSON.parse(String(h.calls[0].init?.body)).callbackURL).toBe('https://app.resu.mx/resumes?tab=a')
  })

  test('an error response toasts and does not navigate', async () => {
    const h = harness({ 'POST /api/auth/sign-in/oauth2': json({ error: 'nope' }, 403) })
    await h.session.signIn()
    expect(h.navigations).toEqual([])
    expect(h.toasts).toHaveLength(1)
    expect(h.toasts[0].type).toBe('error')
    expect(h.session.get()).toMatchObject({ error: 'sign_in_failed', busy: false })
  })

  test('a network error toasts and does not navigate', async () => {
    const h = harness({
      'POST /api/auth/sign-in/oauth2': () => {
        throw new TypeError('Failed to fetch')
      },
    })
    await h.session.signIn()
    expect(h.navigations).toEqual([])
    expect(h.toasts).toHaveLength(1)
    expect(h.session.get().busy).toBe(false)
  })

  test('only navigates to a web URL: never javascript: or data:', async () => {
    for (const url of ['javascript:alert(1)', 'data:text/html,x', 'not a url', '', 42, null]) {
      const h = harness({ 'POST /api/auth/sign-in/oauth2': json({ url }) })
      await h.session.signIn()
      expect(h.navigations).toEqual([])
      expect(h.session.get().error).toBe('sign_in_failed')
    }
    const local = harness({ 'POST /api/auth/sign-in/oauth2': json({ url: 'http://localhost:8787/authorize' }) })
    await local.session.signIn()
    expect(local.navigations).toEqual(['http://localhost:8787/authorize'])
    const plain = harness({ 'POST /api/auth/sign-in/oauth2': json({ url: 'http://idp.example.test/authorize' }) })
    await plain.session.signIn()
    expect(plain.navigations).toEqual([])
  })

  test('a second click while one is in flight does not post again', async () => {
    const gate: { resolve?: (r: Response) => void } = {}
    const h = harness({
      'POST /api/auth/sign-in/oauth2': () => new Promise<Response>((resolve) => (gate.resolve = resolve)),
    })
    const first = h.session.signIn()
    expect(h.session.get().busy).toBe(true)
    await h.session.signIn()
    expect(h.calls).toHaveLength(1)
    gate.resolve(json({ url: auth0 }))
    await first
    expect(h.navigations).toEqual([auth0])
  })
})

describe('signOut: POST /api/auth/sign-out', () => {
  test('posts, then re-checks the session and ends out', async () => {
    let signedIn = true
    const h = harness({
      'GET /api/me': () => (signedIn ? json(ME) : json({ error: 'unauthorized' }, 401)),
      'POST /api/auth/sign-out': () => {
        signedIn = false
        return json({ success: true })
      },
    })
    await h.session.refresh()
    expect(h.session.get().state).toBe('in')
    await h.session.signOut()
    expect(h.calls.map((c) => `${c.init?.method ?? 'GET'} ${c.url.replace(ID, '')}`)).toEqual([
      'GET /api/me',
      'POST /api/auth/sign-out',
      'GET /api/me',
    ])
    expect(h.calls[1].init?.credentials).toBe('include')
    expect(h.session.get()).toMatchObject({ state: 'out', user: null, storageId: null, busy: false })
  })

  test('a failed sign-out toasts and the re-check keeps the truth', async () => {
    const h = harness({
      'GET /api/me': json(ME),
      'POST /api/auth/sign-out': json({ error: 'forbidden' }, 403),
    })
    await h.session.refresh()
    await h.session.signOut()
    expect(h.toasts).toHaveLength(1)
    expect(h.toasts[0].type).toBe('error')
    expect(h.session.get()).toMatchObject({ state: 'in', busy: false, error: 'sign_out_failed' })
  })
})

describe('a refused sign-up coming back', () => {
  test('toasts once, strips the error from the URL, and records it', async () => {
    const h = harness(
      { 'GET /api/me': json({ error: 'unauthorized' }, 401) },
      { href: `https://app.resu.mx/resumes?tab=a&error=${SIGNUP_NOT_ALLOWED}#top` },
    )
    await h.session.refresh()
    expect(h.toasts).toHaveLength(1)
    expect(h.toasts[0].message).toBe(SIGNUP_REFUSED_MESSAGE)
    expect(h.toasts[0].message).toContain('invite-only')
    expect(h.replaced).toEqual(['https://app.resu.mx/resumes?tab=a#top'])
    expect(h.session.get()).toMatchObject({ state: 'out', error: SIGNUP_NOT_ALLOWED })

    await h.session.refresh() // the URL is clean now: no second toast
    expect(h.toasts).toHaveLength(1)
    expect(h.replaced).toHaveLength(1)
  })

  test('a normal load does not toast or rewrite the URL', async () => {
    const h = harness({ 'GET /api/me': json(ME) })
    await h.session.refresh()
    expect(h.toasts).toEqual([])
    expect(h.replaced).toEqual([])
  })
})

describe('readAuthReturn (the one place that knows the carrier)', () => {
  const base = 'https://app.resu.mx/resumes'

  test('reads the code from the query string', () => {
    expect(readAuthReturn(`${base}?error=signup_not_allowed`)).toEqual({
      code: SIGNUP_NOT_ALLOWED,
      cleanHref: base,
    })
  })

  test('reads it from the hash, plain or after a path', () => {
    expect(readAuthReturn(`${base}#error=signup_not_allowed`)?.cleanHref).toBe(base)
    expect(readAuthReturn(`${base}#/x?error=signup_not_allowed&keep=1`)?.cleanHref).toBe(`${base}#/x?keep=1`)
    expect(readAuthReturn(`${base}#/x?error=signup_not_allowed`)?.cleanHref).toBe(`${base}#/x`)
  })

  test('reads error_description, ignoring case and - versus _', () => {
    expect(readAuthReturn(`${base}?error=access_denied&error_description=signup_not_allowed`)?.cleanHref).toBe(base)
    expect(readAuthReturn(`${base}?error=SIGNUP_NOT_ALLOWED`)?.code).toBe(SIGNUP_NOT_ALLOWED)
    expect(readAuthReturn(`${base}?error=signup-not-allowed`)?.code).toBe(SIGNUP_NOT_ALLOWED)
    expect(readAuthReturn(`${base}?error=Signup%20not%20allowed`)?.code).toBe(SIGNUP_NOT_ALLOWED)
  })

  test('keeps every other query parameter and the path', () => {
    expect(readAuthReturn(`${base}?a=1&error=signup_not_allowed&b=two`)?.cleanHref).toBe(`${base}?a=1&b=two`)
  })

  test('removes it from both places when both carry it', () => {
    expect(readAuthReturn(`${base}?error=signup_not_allowed#error=signup_not_allowed`)?.cleanHref).toBe(base)
  })

  test('ignores other errors, other hashes, and unparseable input', () => {
    expect(readAuthReturn(`${base}?error=access_denied`)).toBeNull()
    expect(readAuthReturn(`${base}?error=`)).toBeNull()
    expect(readAuthReturn(`${base}#section`)).toBeNull()
    expect(readAuthReturn(`${base}#/x?other=signup_not_allowed`)).toBeNull()
    expect(readAuthReturn(base)).toBeNull()
    expect(readAuthReturn('not a url')).toBeNull()
  })
})
