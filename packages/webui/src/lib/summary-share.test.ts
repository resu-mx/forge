import { describe, expect, test } from 'bun:test'
import type { Resume } from '@forge/sdk'
import { sharedSummaryWarningName, UNNAMED_SHARER } from './summary-share'

const resume = (name: string) => ({ id: 'r1', name }) as Resume
const page = (n: number) => ({ total: n, offset: 0, limit: 1 })

describe('sharedSummaryWarningName', () => {
  test('names the first linked resume', () => {
    expect(sharedSummaryWarningName({ ok: true, data: [resume('Resume A')], pagination: page(1) })).toBe('Resume A')
  })

  test('falls back when the lookup fails (501 in the browser runtime today)', () => {
    expect(
      sharedSummaryWarningName({
        ok: false,
        error: { code: 'NOT_IMPLEMENTED', message: 'GET /api/summaries/s1/linked-resumes is not available in the browser runtime yet' },
      }),
    ).toBe(UNNAMED_SHARER)
  })

  test('falls back when the lookup returns no rows', () => {
    expect(sharedSummaryWarningName({ ok: true, data: [], pagination: page(0) })).toBe(UNNAMED_SHARER)
  })
})
