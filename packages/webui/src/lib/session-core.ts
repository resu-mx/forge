/**
 * The account session logic, free of Svelte so it can be unit-tested with a stubbed `fetch`.
 * `stores/session.svelte.ts` wraps it in module `$state`.
 *
 * Contract (the snapshot-API spec): `GET {id}/api/me` answers `200 {user, storageId}` or `401`;
 * sign-in is `POST {id}/api/auth/sign-in/oauth2 {providerId, callbackURL}` answering `{url}`,
 * which the browser then navigates to; sign-out is `POST {id}/api/auth/sign-out`. A sign-in the
 * invite-only allowlist refuses comes back to the callback URL carrying `signup_not_allowed`.
 */

export type SessionState = 'unknown' | 'out' | 'in'

export interface SessionUser {
  id?: string
  email?: string
  name?: string
  [key: string]: unknown
}

export interface SessionSnapshot {
  /** False when this origin cannot carry the session cookie: nothing here talks to the network. */
  available: boolean
  /** `unknown` until the first answer from the account service; stays so if it is unreachable. */
  state: SessionState
  user: SessionUser | null
  /** The id the person's data is stored under (from `/api/me`); the sync engine keys on it. */
  storageId: string | null
  /** Last problem worth showing: `signup_not_allowed`, `sign_in_failed` or `sign_out_failed`. */
  error: string | null
  /** A sign-in or sign-out request is in flight. */
  busy: boolean
}

export const SIGNUP_NOT_ALLOWED = 'signup_not_allowed'

export const SIGNUP_REFUSED_MESSAGE =
  "This alpha is invite-only, and that account isn't on the invite list yet. Forge keeps working in this browser without an account."

export interface AuthReturn {
  code: typeof SIGNUP_NOT_ALLOWED
  /** The same URL with the error parameters removed, for `history.replaceState`. */
  cleanHref: string
}

const ERROR_KEYS = ['error', 'error_description']

function normaliseCode(value: string): string {
  return value.trim().toLowerCase().replace(/[\s-]+/g, '_')
}

/** The hash as `[prefix, params]`: `#/x?error=a` is `['/x?', ...]`, `#error=a` is `['', ...]`. */
function splitHash(hash: string): { prefix: string; params: URLSearchParams } | null {
  const body = hash.replace(/^#/, '')
  const q = body.indexOf('?')
  if (q >= 0) return { prefix: body.slice(0, q + 1), params: new URLSearchParams(body.slice(q + 1)) }
  return body.includes('=') ? { prefix: '', params: new URLSearchParams(body) } : null
}

/**
 * Read an auth error that came back on the page URL. This is the one place that knows how the
 * auth Worker carries a refusal back; it looks in the query string and in the hash, in `error`
 * and `error_description`, and ignores case and `-` versus `_`. Adjust it here if the carrier
 * changes. Returns null when the URL carries no refusal (other `error` values are left alone,
 * they may belong to the app).
 */
export function readAuthReturn(href: string): AuthReturn | null {
  let url: URL
  try {
    url = new URL(href)
  } catch {
    return null
  }
  const hash = splitHash(url.hash)
  const values = ERROR_KEYS.flatMap((key) => [url.searchParams.get(key), hash?.params.get(key) ?? null])
  const refused = values.some((value) => value !== null && normaliseCode(value).includes(SIGNUP_NOT_ALLOWED))
  if (!refused) return null

  for (const key of ERROR_KEYS) url.searchParams.delete(key)
  if (hash) {
    for (const key of ERROR_KEYS) hash.params.delete(key)
    const rest = hash.params.toString()
    const kept = hash.prefix + rest
    url.hash = kept.replace(/\?$/, '')
  }
  return { code: SIGNUP_NOT_ALLOWED, cleanHref: url.toString() }
}

/** The URL to send the browser to, or null if it is not a plain web URL (never `javascript:`). */
function navigationTarget(value: unknown): string | null {
  if (typeof value !== 'string') return null
  try {
    const url = new URL(value)
    const local = url.hostname === 'localhost' || url.hostname === '127.0.0.1'
    return url.protocol === 'https:' || (url.protocol === 'http:' && local) ? url.toString() : null
  } catch {
    return null
  }
}

export interface SessionDeps {
  /** Origin of the account service. */
  idUrl: string
  /** Whether this origin can hold a session at all (`signInAvailable()`). */
  available: boolean
  fetch: typeof fetch
  getHref(): string
  /** Leave the app for `url` (the identity provider). */
  navigate(url: string): void
  /** Replace the address bar's URL without a navigation. */
  replaceUrl(url: string): void
  notify(opts: { message: string; type?: 'error' | 'info' | 'success'; duration?: number }): void
  onChange(snapshot: SessionSnapshot): void
}

export function initialSnapshot(available: boolean): SessionSnapshot {
  return { available, state: available ? 'unknown' : 'out', user: null, storageId: null, error: null, busy: false }
}

export interface SessionController {
  get(): SessionSnapshot
  refresh(): Promise<void>
  signIn(): Promise<void>
  signOut(): Promise<void>
}

export function createSession(deps: SessionDeps): SessionController {
  let snapshot = initialSnapshot(deps.available)
  // Only the newest `/api/me` answer may land: a slow, stale one must not undo a sign-out.
  let latest = 0

  function set(patch: Partial<SessionSnapshot>) {
    snapshot = { ...snapshot, ...patch }
    deps.onChange(snapshot)
  }

  function consumeAuthReturn() {
    const returned = readAuthReturn(deps.getHref())
    if (!returned) return
    deps.replaceUrl(returned.cleanHref)
    set({ error: returned.code })
    deps.notify({ message: SIGNUP_REFUSED_MESSAGE, type: 'info', duration: 12000 })
  }

  async function refresh(): Promise<void> {
    if (!deps.available) return
    consumeAuthReturn()
    const mine = ++latest
    try {
      const res = await deps.fetch(`${deps.idUrl}/api/me`, { credentials: 'include' })
      if (mine !== latest) return
      if (res.status === 401) {
        set({ state: 'out', user: null, storageId: null })
        return
      }
      if (!res.ok) return // The service is unwell: say nothing, keep what we know.
      const body = (await res.json()) as { user?: SessionUser; storageId?: unknown }
      if (mine !== latest) return
      if (body && typeof body.user === 'object' && body.user !== null && typeof body.storageId === 'string') {
        set({ state: 'in', user: body.user, storageId: body.storageId })
      }
    } catch {
      // Offline, blocked or an unreadable body: stay as we were, and do not nag about it.
    }
  }

  function failedSignIn() {
    set({ error: 'sign_in_failed', busy: false })
    deps.notify({ message: 'Could not start sign-in. Check your connection and try again.', type: 'error' })
  }

  async function signIn(): Promise<void> {
    if (!deps.available || snapshot.busy) return
    set({ busy: true, error: null })
    // Come back to this page without a stale error parameter on it.
    const callbackURL = readAuthReturn(deps.getHref())?.cleanHref ?? deps.getHref()
    try {
      const res = await deps.fetch(`${deps.idUrl}/api/auth/sign-in/oauth2`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        // `errorCallbackURL` makes the auth service send a refused sign-in back here too, instead
        // of to its own error page.
        body: JSON.stringify({ providerId: 'auth0', callbackURL, errorCallbackURL: callbackURL }),
      })
      const body = res.ok ? ((await res.json().catch(() => null)) as { url?: unknown } | null) : null
      const target = navigationTarget(body?.url)
      if (!target) return failedSignIn()
      deps.navigate(target)
      // Stay busy: the page is about to be replaced by the identity provider.
    } catch {
      failedSignIn()
    }
  }

  async function signOut(): Promise<void> {
    if (!deps.available || snapshot.busy) return
    set({ busy: true, error: null })
    try {
      const res = await deps.fetch(`${deps.idUrl}/api/auth/sign-out`, { method: 'POST', credentials: 'include' })
      if (!res.ok) throw new Error(`sign-out answered ${res.status}`)
    } catch {
      set({ error: 'sign_out_failed' })
      deps.notify({ message: 'Could not sign out. Try again in a moment.', type: 'error' })
    }
    set({ busy: false })
    await refresh()
  }

  return { get: () => snapshot, refresh, signIn, signOut }
}
