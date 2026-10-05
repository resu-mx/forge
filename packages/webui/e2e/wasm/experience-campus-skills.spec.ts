import { expect, test, type Page } from '@playwright/test'

/**
 * Experience → Education in the browser, with no server (resu-mx/forge#31): the campus
 * picker, a new campus from the "+" modal, and a source's skills.
 *
 * The agent seeds the organization, its campus, a skill and the source through `window.forge`;
 * the person picks, creates, links and unlinks in the UI.
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

async function selectSource(page: Page, title: string) {
  await page.getByText(title).first().click()
  await expect(page.locator('#edu-campus')).toBeEnabled()
}

const chips = (page: Page) => page.locator('.skill-pills .skill-pill')

// KNOWN PRODUCT BUG, so `test.fail`: SourcesView drops an unsaved campus selection. Creating the
// campus fires `forge:changed`; ~150 ms later `loadSources(true)` replaces `sources`, the derived
// `selectedSource` is a new object, and the `$effect` at SourcesView.svelte:205 calls
// `populateFormFromSource`, resetting `formCampusId` to the saved value (null). The assertion
// right after "Create & Select" is therefore correct and fails. Remove `.fail` when that is fixed
// (the test then reports "expected to fail" until you do). With the selection forced past that
// point, every later step in this test passes.
test.fail('experience: campus picker, new campus and source skills, in the browser', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))
  await openApp(page)

  // ── The agent: an institution with a campus, a skill, and an education source ───────────
  const seed = await inPage<{ org: string; source: string }>(
    page,
    `const org = ok(await forge.organizations.create({ name: 'Analytical University', org_type: 'education', tags: ['university'] }))
     const addr = ok(await forge.addresses.create({ name: 'Main Campus', city: 'London', state: 'ENG' }))
     ok(await forge.organizations.createLocation(org.id, { name: 'Main Campus', modality: 'in_person', address_id: addr.id }))
     ok(await forge.skills.create({ name: 'Rust' }))
     const source = ok(await forge.sources.create({
       title: 'BSc Mathematics', description: 'Pure mathematics, first-class honours.', source_type: 'education',
       education: { education_type: 'degree', education_organization_id: org.id, degree_level: 'bachelors', degree_type: 'BSc' },
     }))
     return { org: org.id, source: source.id }`,
  )

  // ── The picker lists the organization's campuses, labelled from their address ───────────
  await page.goto('/experience/education')
  await selectSource(page, 'BSc Mathematics')
  await expect(page.locator('#edu-campus option', { hasText: 'Main Campus (London, ENG)' })).toHaveCount(1)

  // ── A new campus from the "+" modal: created, selected, saved ───────────────────────────
  await page.getByRole('button', { name: 'New campus' }).click()
  const modal = page.getByRole('dialog', { name: 'New Campus' })
  await modal.getByLabel('Name').fill('Online')
  await modal.getByLabel('Modality').selectOption('remote')
  await modal.getByLabel('City').fill('Arlington')
  await modal.getByLabel('State').fill('VA')
  await modal.getByRole('button', { name: 'Create & Select' }).click()
  await expect(modal).toBeHidden()
  // The runtime's `forge:changed` refresh is debounced by 150 ms; let it land so the selection is
  // checked after the list reload a person would see, not in the instant before it.
  await page.waitForTimeout(600)

  const online = await inPage<{ id: string }>(
    page,
    `return ok(await forge.organizations.listLocations('${seed.org}')).find(l => l.name === 'Online')`,
  )
  await expect(page.locator('#edu-campus')).toHaveValue(online.id)
  await page.getByRole('button', { name: 'Save', exact: true }).click()
  await expect
    .poll(
      async () =>
        (await inPage<{ education?: { campus_id: string | null } }>(page, `return ok(await forge.sources.get('${seed.source}'))`))
          .education?.campus_id,
    )
    .toBe(online.id)

  // ── Still selected after a reload ───────────────────────────────────────────────────────
  await openApp(page, '/experience/education')
  await selectSource(page, 'BSc Mathematics')
  await expect(page.locator('#edu-campus')).toHaveValue(online.id)
  await expect(page.locator('#edu-campus option:checked')).toHaveText('Online (Arlington, VA)')

  // ── Skills: link an existing one, create one, remove one ────────────────────────────────
  const search = page.getByPlaceholder('Search or create skill...')
  await search.fill('Rus')
  await page.locator('.skill-dropdown').getByRole('button', { name: /^Rust/ }).click()
  await expect(chips(page).filter({ hasText: 'Rust' })).toHaveCount(1)
  await search.fill('terraform')
  await page.getByRole('button', { name: '+ Create "terraform"' }).click()
  await expect(chips(page).filter({ hasText: 'Terraform' })).toHaveCount(1)
  await page.getByRole('button', { name: 'Remove Rust' }).click()
  await expect(chips(page)).toHaveCount(1)

  // ── Survives a reload, and the API agrees ───────────────────────────────────────────────
  await openApp(page, '/experience/education')
  await selectSource(page, 'BSc Mathematics')
  await expect(chips(page)).toHaveText([/Terraform/])
  const linked = await inPage<{ id: string; name: string }[]>(
    page,
    `const r = await fetch('/api/sources/${seed.source}/skills')
     if (!r.ok) throw new Error('HTTP ' + r.status)
     return (await r.json()).data`,
  )
  expect(linked.map((s) => s.name)).toEqual(['Terraform'])

  // ── A failed call is an error, not a silent no-op ───────────────────────────────────────
  // Unlink behind the UI's back; the UI's own unlink then gets 404 "Skill link not found".
  await inPage(page, `await fetch('/api/sources/${seed.source}/skills/${linked[0].id}', { method: 'DELETE' })`)
  await page.getByRole('button', { name: 'Remove Terraform' }).click()
  await expect(page.getByRole('alert').filter({ hasText: 'Failed to remove skill' })).toBeVisible()

  expect(pageErrors).toEqual([])
})
