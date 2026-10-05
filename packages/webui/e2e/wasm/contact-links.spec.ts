import { expect, test, type Page } from '@playwright/test'

/**
 * A contact's relationships, in the browser with no server (resu-mx/forge#36).
 *
 * The agent seeds a contact, an organization, a JD and a resume through `window.forge`;
 * the person links and unlinks them from the contact editor.
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

async function openContact(page: Page, name: string) {
  await page.getByText(name, { exact: true }).first().click()
  await expect(page.getByRole('region', { name: 'Linked Organizations' })).toBeVisible()
}

/** Link one target from a section, through the UI, and check the success toast and the row. */
async function link(
  page: Page,
  section: string,
  noun: string,
  option: string,
  relationship: string,
  rowText: string,
) {
  const region = page.getByRole('region', { name: section })
  await region.getByRole('button', { name: `+ Link ${noun}` }).click()
  const dialog = page.getByRole('dialog', { name: `Link ${noun}` })
  await dialog.locator('#link-target').selectOption({ label: option })
  await dialog.getByLabel('Relationship').selectOption(relationship)
  await dialog.getByRole('button', { name: 'Link', exact: true }).click()
  await expect(page.getByRole('alert').filter({ hasText: `${noun} linked` })).toBeVisible()
  await expect(dialog).toBeHidden()
  await expect(region.getByText(rowText)).toBeVisible()
}

test('contact links: organization, JD and resume from the contact editor', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))
  await openApp(page)

  // ── The agent: a contact and the things it can be linked to ─────────────────────────────
  const ids = await inPage<{ contact: string; org: string; jd: string; resume: string }>(
    page,
    `const org = ok(await forge.organizations.create({ name: 'Acme Robotics' }))
     ok(await forge.contacts.create({ name: 'Zed Decoy' }))
     const contact = ok(await forge.contacts.create({ name: 'Rita Recruiter', title: 'Talent Partner' }))
     const jd = ok(await forge.jobDescriptions.create({ title: 'Staff Platform Engineer', raw_text: 'Build the platform.', organization_id: org.id }))
     const resume = ok(await forge.resumes.create({ name: 'Platform resume', target_role: 'Platform Engineer',
       target_employer: 'Acme Robotics', archetype: 'infrastructure' }))
     return { contact: contact.id, org: org.id, jd: jd.id, resume: resume.id }`,
  )

  await page.goto('/data/contacts')
  await openContact(page, 'Rita Recruiter')

  // ── The organization picker offers organizations, not contacts ──────────────────────────
  await page
    .getByRole('region', { name: 'Linked Organizations' })
    .getByRole('button', { name: '+ Link Organization' })
    .click()
  const orgDialog = page.getByRole('dialog', { name: 'Link Organization' })
  await expect(orgDialog.locator('#link-target option', { hasText: 'Acme Robotics' })).toHaveCount(1)
  await expect(orgDialog.locator('#link-target option', { hasText: 'Zed Decoy' })).toHaveCount(0)
  await orgDialog.getByRole('button', { name: 'Cancel' }).click()
  await expect(orgDialog).toBeHidden()

  // ── The person: link one of each ────────────────────────────────────────────────────────
  await link(page, 'Linked Organizations', 'Organization', 'Acme Robotics', 'recruiter', 'Acme Robotics')
  await link(
    page,
    'Linked Job Descriptions',
    'Job Description',
    'Staff Platform Engineer — Acme Robotics',
    'hiring_manager',
    'Staff Platform Engineer',
  )
  await link(page, 'Linked Resumes', 'Resume', 'Platform resume', 'reference', 'Platform resume')

  // ── All three survive a reload, and the store agrees ────────────────────────────────────
  await openApp(page, '/data/contacts')
  await openContact(page, 'Rita Recruiter')
  await expect(page.getByRole('region', { name: 'Linked Organizations' }).getByText('Acme Robotics')).toBeVisible()
  await expect(
    page.getByRole('region', { name: 'Linked Job Descriptions' }).getByText('Staff Platform Engineer'),
  ).toBeVisible()
  await expect(page.getByRole('region', { name: 'Linked Resumes' }).getByText('Platform resume')).toBeVisible()
  const saved = await inPage<{ orgs: any[]; jds: any[]; resumes: any[] }>(
    page,
    `return {
       orgs: ok(await forge.contacts.listOrganizations('${ids.contact}')),
       jds: ok(await forge.contacts.listJobDescriptions('${ids.contact}')),
       resumes: ok(await forge.contacts.listResumes('${ids.contact}')),
     }`,
  )
  expect(saved.orgs).toEqual([expect.objectContaining({ id: ids.org, relationship: 'recruiter' })])
  expect(saved.jds).toEqual([expect.objectContaining({ id: ids.jd, relationship: 'hiring_manager' })])
  expect(saved.resumes).toEqual([expect.objectContaining({ id: ids.resume, relationship: 'reference' })])

  // ── Unlink the organization: gone, and still gone after a reload ────────────────────────
  const orgs = page.getByRole('region', { name: 'Linked Organizations' })
  await orgs.getByRole('button', { name: 'Unlink Acme Robotics' }).click()
  await expect(page.getByRole('alert').filter({ hasText: 'Organization unlinked' })).toBeVisible()
  await expect(orgs.getByText('No organizations linked.')).toBeVisible()
  await openApp(page, '/data/contacts')
  await openContact(page, 'Rita Recruiter')
  await expect(
    page.getByRole('region', { name: 'Linked Organizations' }).getByText('No organizations linked.'),
  ).toBeVisible()
  expect(await inPage<unknown[]>(page, `return ok(await forge.contacts.listOrganizations('${ids.contact}'))`)).toEqual([])

  expect(pageErrors).toEqual([])
})
