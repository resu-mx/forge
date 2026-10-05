import type { Result, Skill } from '@forge/sdk'

/**
 * Attach keywords that were picked before the summary existed. Sequential, in pick order.
 * Returns the skills that were NOT attached, so the caller can name them.
 */
export async function attachKeywords(
  summaryId: string,
  skills: Skill[],
  add: (summaryId: string, skillId: string) => Promise<Result<void>>,
): Promise<Skill[]> {
  const failed: Skill[] = []
  for (const skill of skills) {
    const res = await add(summaryId, skill.id)
    if (!res.ok) failed.push(skill)
  }
  return failed
}

export function keywordSaveToast(failed: Skill[]): { type: 'success' | 'error'; message: string } {
  if (failed.length === 0) return { type: 'success', message: 'Summary created' }
  const what = failed.length === 1 ? '1 keyword was' : `${failed.length} keywords were`
  return {
    type: 'error',
    message: `Summary created, but ${what} not saved: ${failed.map((s) => s.name).join(', ')}`,
  }
}

export function keywordLoadFailureMessage(failed: number, total: number, reason?: string): string | null {
  if (failed === 0) return null
  const base = `Couldn't load keywords for ${failed} of ${total} ${total === 1 ? 'summary' : 'summaries'}`
  return reason ? `${base}: ${reason}` : base
}
