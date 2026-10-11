import { expect, test } from '@playwright/test'
import { openApp } from './helpers'

/** The Bullets and Perspectives tabs keep independent list/board modes (resu-mx/forge#207). */
test('Bullets and Perspectives tabs keep their own view mode', async ({ page }) => {
  await openApp(page, '/data/bullets')

  const board = page.getByRole('radio', { name: 'Board' })
  const list = page.getByRole('radio', { name: 'List' })
  const bulletsTab = page.getByRole('button', { name: 'Bullets', exact: true })
  const perspectivesTab = page.getByRole('button', { name: 'Perspectives', exact: true })
  const stored = (key: string) => page.evaluate((k) => localStorage.getItem(k), `forge:viewMode:${key}`)

  // Perspectives -> board
  await perspectivesTab.click()
  await board.click()
  await expect(board).toHaveAttribute('aria-checked', 'true')
  expect(await stored('perspectives')).toBe('board')
  expect(await stored('bullets')).toBeNull()

  // Bullets keeps its own (list) mode
  await bulletsTab.click()
  await expect(list).toHaveAttribute('aria-checked', 'true')
  await expect(board).toHaveAttribute('aria-checked', 'false')

  // Perspectives is still board
  await perspectivesTab.click()
  await expect(board).toHaveAttribute('aria-checked', 'true')

  // Survives a reload: Bullets list, Perspectives board
  await page.reload()
  await page.waitForFunction(() => !!(window as unknown as { forge?: unknown }).forge)
  await expect(list).toHaveAttribute('aria-checked', 'true')
  await perspectivesTab.click()
  await expect(board).toHaveAttribute('aria-checked', 'true')
})
