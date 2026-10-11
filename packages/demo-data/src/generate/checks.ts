/**
 * Phase 12: checks through the API before the server shuts down.
 */

import { fetchBytes, unwrap } from '../api'
import { type GenContext, phase } from './context'

export interface ApiCheckReport {
  drift: number
  reviewPending: { bullets: number; perspectives: number }
  pdfBytes: Record<string, number>
}

export async function phaseApiChecks(ctx: GenContext): Promise<ApiCheckReport> {
  phase(ctx, 'API checks: drift, review queue, PDFs, gaps')
  const { corpus, forge, ledger } = ctx

  const drift = await unwrap(forge.integrity.drift(), 'integrity drift')
  const driftCount = Array.isArray(drift) ? drift.length : Object.keys(drift ?? {}).length
  if (driftCount !== 0) throw new Error(`integrity drift is not empty: ${JSON.stringify(drift).slice(0, 500)}`)

  // Before the overlay, archived bullets/perspectives are still approved, so the review
  // queue holds exactly the corpus's in_review items.
  const queue = await unwrap(forge.review.pending(), 'review queue')
  const wantBullets = corpus.bullets.filter((b) => b.status === 'in_review').length
  const wantPerspectives = corpus.perspectives.filter((p) => p.status === 'in_review').length
  if (queue.bullets.count !== wantBullets || queue.perspectives.count !== wantPerspectives) {
    throw new Error(
      `review queue has ${queue.bullets.count} bullets / ${queue.perspectives.count} perspectives, expected ${wantBullets} / ${wantPerspectives}`,
    )
  }

  const pdfBytes: Record<string, number> = {}
  for (const r of corpus.resumes) {
    const id = ledger.id('resume', r.key)
    const pdf = await fetchBytes(ctx.baseUrl, 'POST', `/resumes/${id}/pdf`)
    if (new TextDecoder().decode(pdf.slice(0, 4)) !== '%PDF') throw new Error(`resume ${r.key}: PDF does not start with %PDF`)
    pdfBytes[r.key] = pdf.byteLength
    await unwrap(forge.resumes.gaps(id), `resume ${r.key} gaps`)
  }

  return { drift: driftCount, reviewPending: { bullets: queue.bullets.count, perspectives: queue.perspectives.count }, pdfBytes }
}
