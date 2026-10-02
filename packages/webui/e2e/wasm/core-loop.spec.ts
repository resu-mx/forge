import { expect, test, type Page } from '@playwright/test'

/**
 * The browser-first core loop, with no server: the Rust API runs in a Worker inside the page.
 *
 *   profile → organization → source → bullet → approve → perspective → approve
 *   → resume from a template → PDF
 *
 * Two kinds of step, on purpose:
 *
 *  - **The agent's steps** go through `window.forge`, the same client Claude in Chrome uses:
 *    the profile, organization and source, and deriving the bullet and the perspective
 *    (derivation prepare + commit, with the text supplied here because no model runs in CI).
 *  - **The person's steps** are done in the UI: approving the bullet and the perspective,
 *    creating the resume from a template, and opening its PDF preview.
 *
 * The seeded archetypes, domains and templates are used as they ship.
 */

type Forge = Record<string, any>

/** Run `body` in the page with the app's own client (`window.forge`) and return its result. */
async function inPage<T>(page: Page, body: string): Promise<T> {
  return page.evaluate(
    async (src) => {
      const forge = (window as unknown as { forge: Forge }).forge
      const ok = <R,>(r: { ok: boolean; data?: R; error?: unknown }): R => {
        if (!r.ok) throw new Error(JSON.stringify(r.error))
        return r.data as R
      }
      return await new Function('forge', 'ok', `return (async () => { ${src} })()`)(forge, ok)
    },
    body,
  ) as Promise<T>
}

async function openApp(page: Page, path = '/') {
  await page.goto(path)
  await page.waitForFunction(() => !!(window as unknown as { forge?: unknown }).forge)
  await inPage(page, 'await window.forgeRuntime.ready')
}

test('core loop: profile to PDF, in the browser', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))

  await openApp(page)

  // ── The agent: profile, organization, source, and a derived bullet ──────────────────────
  await inPage(page, `ok(await forge.profile.update({ name: 'Ada Lovelace', email: 'ada@example.com' }))`)
  const org = await inPage<{ id: string }>(page, `return ok(await forge.organizations.create({ name: 'Analytical Engines Ltd', worked: true }))`)
  const source = await inPage<{ id: string }>(
    page,
    `return ok(await forge.sources.create({
      title: 'Senior Engineer', source_type: 'role',
      description: 'Led the move of the build system to Rust. Cut build time from 40 to 12 minutes across 30 services.',
      role: { organization_id: '${org.id}', start_date: '2020-01', is_current: true },
    }))`,
  )
  const bullet = await inPage<{ id: string; status: string }>(
    page,
    `const p = ok(await forge.derivations.prepare({ entity_type: 'source', entity_id: '${source.id}', client_id: 'e2e' }))
     const made = ok(await forge.derivations.commitBullets(p.derivation_id, { bullets: [{
       content: 'Moved the build system of 30 services to Rust, cutting build time from 40 to 12 minutes',
       technologies: ['Rust'], metrics: '40 to 12 minutes' }] }))
     return made[0]`,
  )
  expect(bullet.status).toBe('in_review')

  // ── The person: approve the bullet in the UI ────────────────────────────────────────────
  await page.goto('/data/bullets')
  await page.getByText('Moved the build system of 30 services').first().click()
  const bulletDialog = page.getByRole('dialog', { name: 'Bullet Details' })
  await expect(bulletDialog.getByText('In Review')).toBeVisible()
  await bulletDialog.getByRole('button', { name: 'Approve' }).click()
  await expect(bulletDialog.getByText('Approved')).toBeVisible()
  expect((await inPage<{ status: string }>(page, `return ok(await forge.bullets.get('${bullet.id}'))`)).status).toBe('approved')

  // ── The agent: a perspective from the approved bullet ───────────────────────────────────
  await openApp(page, '/data/bullets')
  const perspective = await inPage<{ id: string; status: string }>(
    page,
    `const p = ok(await forge.derivations.prepare({ entity_type: 'bullet', entity_id: '${bullet.id}', client_id: 'e2e',
       params: { archetype: 'infrastructure', domain: 'devops', framing: 'accomplishment' } }))
     return ok(await forge.derivations.commitPerspective(p.derivation_id, {
       content: 'Rebuilt CI on Rust for 30 services and cut build time by 70%',
       reasoning: 'Shows platform impact at scale' }))`,
  )
  expect(perspective.status).toBe('in_review')

  // ── The person: approve the perspective on the board (drag In Review → Approved) ────────
  await page.goto('/data/bullets')
  await page.getByRole('button', { name: 'Perspectives', exact: true }).click()
  await page.locator('[class*="view-toggle"] button', { hasText: 'Board' }).click()
  const card = page.getByText('Rebuilt CI on Rust for 30 services').first()
  await expect(card).toBeVisible()
  const approvedBody = page
    .locator('.column', { has: page.getByRole('heading', { name: 'Approved' }) })
    .locator('.column-body')
  // The board uses svelte-dnd-action, which follows pointer movement rather than HTML5 drag
  // events, so move the mouse in steps instead of using dragTo().
  const from = (await card.boundingBox())!
  const to = (await approvedBody.boundingBox())!
  // The library works on animation frames, so pause after picking the card up and again over the
  // target; without the pauses the drop is occasionally missed.
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2)
  await page.mouse.down()
  await page.waitForTimeout(150)
  await page.mouse.move(from.x + from.width / 2 + 10, from.y + from.height / 2 + 10, { steps: 5 })
  await page.waitForTimeout(150)
  await page.mouse.move(to.x + to.width / 2, to.y + 40, { steps: 25 })
  await page.waitForTimeout(300)
  await page.mouse.up()
  await expect
    .poll(async () => (await inPage<{ status: string }>(page, `return ok(await forge.perspectives.get('${perspective.id}'))`)).status)
    .toBe('approved')

  // ── The person: a resume from the built-in template ─────────────────────────────────────
  await page.goto('/resumes')
  await page.getByRole('button', { name: 'New Resume' }).first().click()
  await page.getByLabel('Name').fill('Platform resume')
  await page.getByLabel('Target Role').fill('Platform Engineer')
  await page.getByLabel('Target Employer').fill('Acme')
  await page.locator('#create-archetype').selectOption({ label: 'infrastructure' })
  await page.getByText('Standard Tech Resume').click()
  await page.getByRole('button', { name: 'Create Resume' }).click()
  await page.getByRole('button', { name: 'Skip' }).click() // no summary
  const listResumes = () => inPage<{ id: string; name: string }[]>(page, `return ok(await forge.resumes.list())`)
  await expect.poll(async () => (await listResumes()).length).toBe(1)
  const resume = (await listResumes())[0]
  expect(resume.name).toBe('Platform resume')

  // The perspective goes into the template's experience section.
  await inPage(
    page,
    `const sections = ok(await forge.resumes.listSections('${resume.id}'))
     const exp = sections.find(s => s.entry_type === 'experience')
     ok(await forge.resumes.addEntry('${resume.id}', { section_id: exp.id, perspective_id: '${perspective.id}' }))`,
  )

  // ── Everything survives a reload ────────────────────────────────────────────────────────
  await openApp(page, '/resumes')
  expect(
    await inPage<number>(
      page,
      `const r = ok(await forge.resumes.get('${resume.id}'))
       return r.sections.reduce((n, s) => n + s.entries.length, 0)`,
    ),
  ).toBe(1)

  // ── The PDF: through the UI's preview, and by the API ───────────────────────────────────
  await page.getByText('Platform resume').first().click()
  await page.getByRole('tab', { name: 'Preview', exact: true }).click()
  await expect(page.locator('iframe[title="Resume PDF Preview"]')).toBeVisible({ timeout: 60_000 })

  const pdf = await inPage<{ head: string; bytes: number }>(
    page,
    `const r = await forge.resumes.pdf('${resume.id}')
     if (!r.ok) throw new Error(JSON.stringify(r.error))
     const buf = new Uint8Array(await r.data.arrayBuffer())
     return { head: new TextDecoder().decode(buf.slice(0, 5)), bytes: buf.length }`,
  )
  expect(pdf.head).toBe('%PDF-')
  expect(pdf.bytes).toBeGreaterThan(2000)

  expect(pageErrors).toEqual([])
})
