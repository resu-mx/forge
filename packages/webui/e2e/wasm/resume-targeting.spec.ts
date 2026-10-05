import { expect, test, type Page } from '@playwright/test'

/**
 * resu-mx/forge#34 in the browser-first runtime: a resume's Targeted Job Descriptions, its
 * tagline (override, reset, regenerate) and the shared-summary warning.
 */

type Forge = Record<string, any>

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

/** The browser dispatcher's message for an unported route (crates/forge-wasm/src/dispatch.rs). */
const NOT_PORTED = 'not available in the browser runtime'

const JD_TEXT =
  'Platform SRE. Run Kubernetes and Terraform for a payments platform; own SLOs, on-call and incident review.'

function trackErrors(page: Page): string[] {
  const errors: string[] = []
  page.on('pageerror', (e) => errors.push(e.message))
  return errors
}

async function seedResume(page: Page, name: string, summaryId?: string): Promise<{ id: string }> {
  const input = {
    name,
    target_role: 'Platform Engineer',
    target_employer: 'Acme',
    archetype: 'infrastructure',
    ...(summaryId ? { summary_id: summaryId } : {}),
  }
  return inPage(page, `return ok(await forge.resumes.create(${JSON.stringify(input)}))`)
}

async function seedJd(page: Page, title: string): Promise<{ id: string }> {
  return inPage(
    page,
    `return ok(await forge.jobDescriptions.create(${JSON.stringify({ title, raw_text: JD_TEXT })}))`,
  )
}

async function openResume(page: Page, name: string) {
  await openApp(page, '/resumes')
  await page.getByText(name).first().click()
}

/** The person creates a resume in the UI; the summary picker opens afterwards. */
async function createResumeInUi(page: Page, name: string) {
  await page.getByRole('button', { name: 'New Resume' }).first().click()
  await page.getByLabel('Name').fill(name)
  await page.getByLabel('Target Role').fill('Platform Engineer')
  await page.getByLabel('Target Employer').fill('Acme')
  await page.locator('#create-archetype').selectOption({ label: 'infrastructure' })
  await page.getByRole('button', { name: 'Create Resume' }).click()
  await expect(page.getByRole('heading', { name: 'Pick a Summary' })).toBeVisible()
}

const targeted = (page: Page) => page.locator('.resume-linked-jds')
const toasts = (page: Page) => page.locator('.toast-container')

test('targeted JDs: lists linked JDs, links and unlinks from the resume, survives a reload', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  const resume = await seedResume(page, 'Resume A')
  const a = await seedJd(page, 'Platform SRE')
  const b = await seedJd(page, 'Data Platform Lead')
  await seedJd(page, 'Payments SRE')
  await inPage(
    page,
    `ok(await forge.jobDescriptions.linkResume('${a.id}', '${resume.id}'))
     ok(await forge.jobDescriptions.linkResume('${b.id}', '${resume.id}'))`,
  )

  await openResume(page, 'Resume A')
  await expect(targeted(page).locator('.linked-card')).toHaveCount(2)

  await targeted(page).getByRole('button', { name: '+ Link JD' }).click()
  await page.locator('.picker-item', { hasText: 'Payments SRE' }).click()
  await expect(targeted(page).locator('.linked-card')).toHaveCount(3)
  await targeted(page).getByRole('button', { name: 'Unlink Platform SRE' }).click()
  await expect(targeted(page).locator('.linked-card')).toHaveCount(2)
  await expect(toasts(page)).not.toContainText(NOT_PORTED)

  await openResume(page, 'Resume A')
  await expect(targeted(page).locator('.linked-card')).toHaveCount(2)
  const titles = await inPage<string[]>(
    page,
    `return ok(await forge.resumes.listJobDescriptions('${resume.id}')).map(j => j.title).sort()`,
  )
  expect(titles).toEqual(['Data Platform Lead', 'Payments SRE'])
  expect(pageErrors).toEqual([])
})

test('tagline: regenerate, override survives a reload, reset returns the generated tagline', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  const resume = await seedResume(page, 'Tagline resume')
  const jd = await seedJd(page, 'Platform SRE')
  await inPage(page, `ok(await forge.jobDescriptions.linkResume('${jd.id}', '${resume.id}'))`)

  await openResume(page, 'Tagline resume')
  const header = page.locator('.header-editor')
  await expect(header).toBeVisible()
  await expect(header.getByRole('alert')).toHaveCount(0) // tagline state loaded

  await header.getByRole('button', { name: 'Regenerate' }).click()
  await expect(header.locator('.tagline-badge.generated')).toHaveText('AUTO')
  const generated = (await header.locator('.header-tagline').innerText()).trim()
  expect(generated.length).toBeGreaterThan(0)

  await header.getByRole('button', { name: 'Set Override' }).click()
  await header.locator('.tagline-input').fill('Platform · SRE · Kubernetes')
  await header.getByRole('button', { name: 'Save Override' }).click()
  await expect(header.locator('.tagline-badge.override')).toHaveText('OVERRIDE')
  await expect(header.locator('.header-tagline')).toHaveText('Platform · SRE · Kubernetes')

  await openResume(page, 'Tagline resume')
  await expect(header.locator('.tagline-badge.override')).toBeVisible()
  await expect(header.locator('.header-tagline')).toHaveText('Platform · SRE · Kubernetes')

  await header.getByRole('button', { name: 'Reset to Generated' }).click()
  await expect(header.locator('.tagline-badge.generated')).toBeVisible()
  await expect(header.locator('.header-tagline')).toHaveText(generated)
  await expect(toasts(page)).not.toContainText(NOT_PORTED)
  expect(pageErrors).toEqual([])
})

async function seedSharedSummary(page: Page): Promise<{ id: string }> {
  const s = await inPage<{ id: string }>(
    page,
    `return ok(await forge.summaries.create({ title: 'Platform SRE summary' }))`,
  )
  await seedResume(page, 'Resume A', s.id)
  return s
}

async function linkSharedSummary(page: Page) {
  await page
    .locator('.picker-item', { hasText: 'Platform SRE summary' })
    .getByRole('button', { name: 'Link', exact: true })
    .click()
}

test('shared summary: linking a summary another resume uses names that resume', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  const s = await seedSharedSummary(page)

  await openApp(page, '/resumes')
  await createResumeInUi(page, 'Resume B')
  await linkSharedSummary(page)
  const warning = page.locator('.warning-box')
  await expect(warning).toContainText('This summary is also used by Resume A')
  await expect(warning.getByRole('button', { name: 'Clone' })).toBeVisible()
  await warning.getByRole('button', { name: 'Link Anyway' }).click()

  await expect
    .poll(
      async () =>
        (
          await inPage<{ name: string; summary_id: string | null }[]>(page, `return ok(await forge.resumes.list())`)
        ).find((r) => r.name === 'Resume B')?.summary_id,
    )
    .toBe(s.id)
  await expect(toasts(page)).not.toContainText(NOT_PORTED)
  expect(pageErrors).toEqual([])
})

test('shared summary: the warning still appears when the name lookup fails', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  await seedSharedSummary(page)

  await openApp(page, '/resumes')
  await inPage(
    page,
    `forge.summaries.linkedResumes = async () =>
      ({ ok: false, error: { code: 'INTERNAL_ERROR', message: 'injected' } })`,
  )
  await createResumeInUi(page, 'Resume B')
  await linkSharedSummary(page)
  await expect(page.locator('.warning-box')).toContainText('This summary is also used by another resume')
  expect(pageErrors).toEqual([])
})
