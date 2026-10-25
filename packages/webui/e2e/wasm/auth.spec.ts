import { expect, test, type Page } from '@playwright/test'
import { E2E_ID_URL } from './e2e-hosts'
import { inPage, openApp } from './helpers'

/**
 * Account sign-in in the browser-first app (resu-mx/forge#255).
 *
 * The account service is stubbed with `page.route`: the build under test points VITE_ID_URL at
 * `E2E_ID_URL` (see playwright.wasm.config.ts), a host that exists nowhere else. localhost counts
 * as a host that can carry the session cookie, so the sign-in UI is on here. Where it is off (a
 * preview host) is covered by the unit tests of `$lib/config`.
 *
 * Contract: GET /api/me is 200 {user, storageId} or 401; sign-in is POST /api/auth/sign-in/oauth2
 * answering {url}; sign-out is POST /api/auth/sign-out; a sign-up the invite-only allowlist
 * refuses comes back to the callback URL carrying `signup_not_allowed`.
 */

const ME = {
  user: { id: 'user-1', email: 'ada@example.test', name: 'Ada Lovelace' },
  storageId: '5f0e7c2a-0000-4000-8000-000000000001',
}

interface IdStub {
  /** `METHOD /path` of every request that reached the stub, preflights excluded. */
  requests: string[]
  signInBodies: Array<Record<string, unknown>>
}

/**
 * Answer the account service for this page. `afterSignIn` picks what a sign-in attempt ends in:
 * the stub identity provider sends the browser back to the callback URL, signed in or refused.
 */
async function stubIdHost(
  page: Page,
  opts: { signedIn: boolean; meFails?: boolean; afterSignIn?: 'refused' | 'ok' },
): Promise<IdStub> {
  const stub: IdStub = { requests: [], signInBodies: [] }
  let signedIn = opts.signedIn

  await page.route(`${E2E_ID_URL}/**`, async (route) => {
    const req = route.request()
    const url = new URL(req.url())
    // The page's own origin, echoed back: a credentialed fetch rejects `*`.
    const origin = req.headers()['origin']
    const cors = origin
      ? {
          'Access-Control-Allow-Origin': origin,
          'Access-Control-Allow-Credentials': 'true',
          'Access-Control-Allow-Headers': 'Content-Type',
          'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
          Vary: 'Origin',
        }
      : {}
    const json = (status: number, body: unknown) =>
      route.fulfill({ status, contentType: 'application/json', headers: cors, body: JSON.stringify(body) })

    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers: cors })
    stub.requests.push(`${req.method()} ${url.pathname}`)

    if (req.method() === 'GET' && url.pathname === '/api/me') {
      if (opts.meFails) return route.abort('connectionrefused')
      return signedIn ? json(200, ME) : json(401, { error: 'unauthorized' })
    }
    if (req.method() === 'POST' && url.pathname === '/api/auth/sign-in/oauth2') {
      stub.signInBodies.push(req.postDataJSON())
      const callbackURL = String((req.postDataJSON() as { callbackURL: string }).callbackURL)
      return json(200, { url: `${E2E_ID_URL}/idp?cb=${encodeURIComponent(callbackURL)}`, redirect: true })
    }
    if (req.method() === 'GET' && url.pathname === '/idp') {
      // The identity provider and the auth callback, in one hop: back to the app.
      const back = new URL(url.searchParams.get('cb')!)
      if (opts.afterSignIn === 'refused') back.searchParams.set('error', 'signup_not_allowed')
      else signedIn = true
      return route.fulfill({
        status: 200,
        contentType: 'text/html',
        body: `<!doctype html><title>idp</title><script>location.replace(${JSON.stringify(back.toString())})</script>`,
      })
    }
    if (req.method() === 'POST' && url.pathname === '/api/auth/sign-out') {
      signedIn = false
      return json(200, { success: true })
    }
    return json(404, { error: 'not_found' })
  })
  return stub
}

const sidebarAccount = (page: Page) => page.locator('.profile-button-area')

test('signed out: a Sign in button and nothing else changes', async ({ page }) => {
  const id = await stubIdHost(page, { signedIn: false })
  await openApp(page)

  await expect(sidebarAccount(page).getByRole('button', { name: 'Sign in' })).toBeVisible()
  await expect(page.getByTestId('account-email')).toHaveCount(0)
  await expect(sidebarAccount(page).getByRole('button', { name: 'Sign out' })).toHaveCount(0)
  await expect(page.locator('.toast')).toHaveCount(0)
  expect(id.requests).toContain('GET /api/me')

  // The app itself does not wait on the account: the local database is open and usable.
  await inPage(page, 'ok(await forge.health())')

  await page.locator('.profile-button').click()
  const menu = page.locator('.profile-menu')
  await expect(menu).toContainText('Not signed in. Your data stays in this browser.')
  await expect(menu.getByRole('button', { name: 'Sign in' })).toBeVisible()
  await expect(menu.getByRole('button', { name: 'Storage', exact: true })).toBeVisible()
})

test('signed in: the account email and Sign out, in the sidebar and the profile menu', async ({ page }) => {
  await stubIdHost(page, { signedIn: true })
  await openApp(page)

  await expect(page.getByTestId('account-email')).toHaveText('ada@example.test')
  await expect(sidebarAccount(page).getByRole('button', { name: 'Sign out' })).toBeVisible()
  await expect(sidebarAccount(page).getByRole('button', { name: 'Sign in' })).toHaveCount(0)

  await page.locator('.profile-button').click()
  const menu = page.locator('.profile-menu')
  await expect(page.getByTestId('menu-account-email')).toHaveText('Signed in as ada@example.test')
  await expect(menu.getByRole('button', { name: 'Sign out' })).toBeVisible()
})

test('sign out from the sidebar returns to the signed-out UI', async ({ page }) => {
  const id = await stubIdHost(page, { signedIn: true })
  await openApp(page)
  await expect(page.getByTestId('account-email')).toBeVisible()

  await sidebarAccount(page).getByRole('button', { name: 'Sign out' }).click()

  await expect(sidebarAccount(page).getByRole('button', { name: 'Sign in' })).toBeVisible()
  await expect(page.getByTestId('account-email')).toHaveCount(0)
  expect(id.requests).toEqual(['GET /api/me', 'POST /api/auth/sign-out', 'GET /api/me'])
})

test('sign out from the profile menu closes the menu and signs out', async ({ page }) => {
  await stubIdHost(page, { signedIn: true })
  await openApp(page)
  await page.locator('.profile-button').click()
  await page.locator('.profile-menu').getByRole('button', { name: 'Sign out' }).click()

  await expect(page.locator('.profile-menu')).toHaveCount(0)
  await expect(sidebarAccount(page).getByRole('button', { name: 'Sign in' })).toBeVisible()
})

test('Sign in posts the Auth0 provider and the current page, then follows the returned url', async ({ page }) => {
  const id = await stubIdHost(page, { signedIn: false, afterSignIn: 'ok' })
  await openApp(page, '/resumes')

  await sidebarAccount(page).getByRole('button', { name: 'Sign in' }).click()

  // The stub identity provider returns the browser to the callback URL with a session.
  await expect(page.getByTestId('account-email')).toHaveText('ada@example.test')
  expect(id.signInBodies).toHaveLength(1)
  expect(id.signInBodies[0]).toMatchObject({ providerId: 'auth0' })
  expect(String(id.signInBodies[0].callbackURL)).toMatch(/\/resumes$/)
  expect(new URL(page.url()).pathname).toBe('/resumes')
  await expect(page.locator('.toast')).toHaveCount(0)
})

test('a sign-in the invite-only allowlist refuses shows a toast and cleans the URL', async ({ page }) => {
  await stubIdHost(page, { signedIn: false, afterSignIn: 'refused' })
  await openApp(page, '/resumes')

  await sidebarAccount(page).getByRole('button', { name: 'Sign in' }).click()

  await expect(page.locator('.toast').filter({ hasText: 'This alpha is invite-only' })).toBeVisible()
  await expect.poll(() => page.url()).not.toContain('error=')
  expect(new URL(page.url()).pathname).toBe('/resumes')
  // Still signed out, still usable, and one toast, not one per check.
  await expect(sidebarAccount(page).getByRole('button', { name: 'Sign in' })).toBeVisible()
  await expect(page.locator('.toast')).toHaveCount(1)
})

test('an unreachable account service leaves the app alone: no account UI, no toast', async ({ page }) => {
  await stubIdHost(page, { signedIn: false, meFails: true })
  await openApp(page)

  await inPage(page, 'ok(await forge.health())')
  // Give the failed check time to have landed before asserting the absence of a reaction.
  await page.waitForTimeout(500)
  await expect(sidebarAccount(page).getByRole('button', { name: 'Sign in' })).toHaveCount(0)
  await expect(sidebarAccount(page).getByRole('button', { name: 'Sign out' })).toHaveCount(0)
  await expect(page.locator('.toast')).toHaveCount(0)
  await expect(page.locator('.profile-button')).toBeVisible()
})
