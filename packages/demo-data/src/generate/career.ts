/**
 * Phases 4–6: sources (with their type extension and skills), bullets, perspectives.
 *
 * Bullets come from `derivations.prepare` → `commitBullets`, which writes what the app
 * writes for a real derivation: the primary `bullet_sources` link, the source snapshot and
 * a `prompt_logs` row. Perspectives come from approved bullets the same way. Drafts use the
 * plain create routes. Statuses no route can reach are queued for the post-pass overlay.
 */

import { unwrap } from '../api'
import type { BulletSpec, PerspectiveSpec, SourceSpec } from '../corpus/types'
import { type GenContext, phase, relDate, skillName, stamp } from './context'

const CLIENT_ID = 'demo-data-generator'

interface BulletRow {
  id: string
  content: string
  status: string
  prompt_log_id: string | null
}

interface PerspectiveRow {
  id: string
  content: string
  status: string
  prompt_log_id: string | null
}

function sourceBody(ctx: GenContext, s: SourceSpec): Record<string, unknown> {
  const { ledger } = ctx
  const body: Record<string, unknown> = {
    title: s.title,
    description: s.description,
    source_type: s.source_type,
    start_date: relDate(ctx, s.start),
    end_date: relDate(ctx, s.end),
  }
  if (s.role) {
    body.role = {
      organization_id: ledger.id('org', s.role.orgKey),
      is_current: s.role.is_current ?? false,
      work_arrangement: s.role.work_arrangement,
      total_comp_notes: s.role.total_comp_notes,
    }
  }
  if (s.project) {
    body.project = {
      organization_id: s.project.orgKey ? ledger.id('org', s.project.orgKey) : undefined,
      is_personal: s.project.is_personal ?? false,
      open_source: s.project.open_source ?? false,
      url: s.project.url,
    }
  }
  if (s.education) {
    const e = s.education
    body.education = {
      education_type: e.education_type,
      education_organization_id: e.orgKey ? ledger.id('org', e.orgKey) : undefined,
      campus_id: e.campusKey ? ledger.id('location', e.campusKey) : undefined,
      field: e.field,
      is_in_progress: e.is_in_progress ?? false,
      credential_id: e.credential_id,
      url: e.url,
      degree_level: e.degree_level,
      degree_type: e.degree_type,
      certificate_subtype: e.certificate_subtype,
      gpa: e.gpa,
      location: e.location,
      edu_description: e.edu_description,
    }
  }
  if (s.presentation) {
    body.presentation = { ...s.presentation }
  }
  return body
}

/** Phase 4. */
export async function phaseSources(ctx: GenContext): Promise<void> {
  phase(ctx, 'sources')
  const { corpus, raw, ledger } = ctx
  for (const s of corpus.sources) {
    const t = stamp(ctx, s.daysAgo, `source:${s.key}`)
    const row = await raw<{ id: string; status: string; description: string }>('POST', '/sources', sourceBody(ctx, s))
    if (row.status !== 'draft') throw new Error(`source ${s.key}: created as ${row.status}, expected draft`)
    if (row.description !== s.description) throw new Error(`source ${s.key}: description was altered on create`)
    ledger.add('source', s.key, 'sources', row.id, { created_at: t, updated_at: t })
    // Source skills have no SDK method.
    for (const skillKey of s.skills) {
      await raw('POST', `/sources/${row.id}/skills`, { skill_id: ledger.id('skill', skillKey) })
    }
  }
}

async function walkBullet(ctx: GenContext, b: BulletSpec, id: string): Promise<void> {
  const { forge, ledger } = ctx
  const reviewed = () => stamp(ctx, b.reviewedDaysAgo ?? b.daysAgo, `bullet:${b.key}:reviewed`, ledger.time('bullet', b.key))
  switch (b.status) {
    case 'approved':
    case 'archived': {
      await unwrap(forge.bullets.approve(id), `approve bullet ${b.key}`)
      ledger.setTime('bullet', b.key, 'approved_at', reviewed())
      if (b.status === 'archived') ctx.overlay.push({ table: 'bullets', ref: `bullet:${b.key}`, id, from: 'approved', to: 'archived' })
      break
    }
    case 'rejected':
      await unwrap(forge.bullets.reject(id, { rejection_reason: b.rejection_reason ?? '' }), `reject bullet ${b.key}`)
      break
    case 'in_review':
      if (b.reopenedAfter) {
        await unwrap(forge.bullets.reject(id, { rejection_reason: b.reopenedAfter }), `reject bullet ${b.key}`)
        await unwrap(forge.bullets.reopen(id), `reopen bullet ${b.key}`)
      }
      break
    case 'draft':
      break
  }
}

/** Phase 5. */
export async function phaseBullets(ctx: GenContext): Promise<void> {
  phase(ctx, 'bullets (derive + drafts)')
  const { corpus, forge, raw, ledger } = ctx

  // Derived bullets: one prepare/commit per source, in corpus order.
  const bySource = new Map<string, BulletSpec[]>()
  for (const b of corpus.bullets.filter((x) => x.via === 'derive')) {
    bySource.set(b.sourceKey, [...(bySource.get(b.sourceKey) ?? []), b])
  }
  for (const [sourceKey, bullets] of bySource) {
    const sourceId = ledger.id('source', sourceKey)
    const prep = await unwrap(
      forge.derivations.prepare({ entity_type: 'source', entity_id: sourceId, client_id: CLIENT_ID }),
      `prepare bullets for ${sourceKey}`,
    )
    const created = (await unwrap(
      forge.derivations.commitBullets(prep.derivation_id, {
        bullets: bullets.map((b) => ({ content: b.content, technologies: b.technologies.map((k) => skillName(ctx, k)), metrics: b.metrics })),
      }),
      `commit bullets for ${sourceKey}`,
    )) as BulletRow[]
    if (created.length !== bullets.length) throw new Error(`commit for ${sourceKey} created ${created.length} bullets, expected ${bullets.length}`)

    let lastDerived = ledger.time('source', sourceKey)
    bullets.forEach((b, i) => {
      const row = created[i] as BulletRow
      if (row.content !== b.content) throw new Error(`bullet ${b.key}: commit order mismatch`)
      const t = stamp(ctx, b.daysAgo, `bullet:${b.key}`, ledger.time('source', sourceKey))
      ledger.add('bullet', b.key, 'bullets', row.id, { created_at: t })
      if (!row.prompt_log_id) throw new Error(`bullet ${b.key}: commit wrote no prompt log`)
      ledger.add('prompt_log', `bullet:${b.key}`, 'prompt_logs', row.prompt_log_id)
      if (t > lastDerived) lastDerived = t
    })
    ledger.setTime('source', sourceKey, 'last_derived_at', lastDerived)
    ledger.setTime('source', sourceKey, 'updated_at', lastDerived)
  }

  // Draft bullets: POST /bullets with the primary source and its current description.
  for (const b of corpus.bullets.filter((x) => x.via === 'draft')) {
    const source = corpus.sources.find((s) => s.key === b.sourceKey)
    if (!source) throw new Error(`bullet ${b.key}: unknown source ${b.sourceKey}`)
    const row = await unwrap(
      forge.bullets.create({
        content: b.content,
        source_content_snapshot: source.description,
        metrics: b.metrics,
        domain: b.domain,
        technologies: b.technologies.map((k) => skillName(ctx, k)),
        source_ids: [{ id: ledger.id('source', b.sourceKey), is_primary: true }],
      }),
      `create draft bullet ${b.key}`,
    )
    if (row.status !== 'draft') throw new Error(`bullet ${b.key}: created as ${row.status}, expected draft`)
    ledger.add('bullet', b.key, 'bullets', row.id, { created_at: stamp(ctx, b.daysAgo, `bullet:${b.key}`, ledger.time('source', b.sourceKey)) })
  }

  // Domains (a derivation leaves them null), then the status walk.
  for (const b of corpus.bullets) {
    const id = ledger.id('bullet', b.key)
    if (b.domain && b.via === 'derive') await raw('PATCH', `/bullets/${id}`, { domain: b.domain })
    await walkBullet(ctx, b, id)
  }
}

async function walkPerspective(ctx: GenContext, p: PerspectiveSpec, id: string): Promise<void> {
  const { forge, ledger } = ctx
  switch (p.status) {
    case 'approved':
    case 'archived': {
      await unwrap(forge.perspectives.approve(id), `approve perspective ${p.key}`)
      ledger.setTime(
        'perspective',
        p.key,
        'approved_at',
        stamp(ctx, p.reviewedDaysAgo ?? p.daysAgo, `perspective:${p.key}:reviewed`, ledger.time('perspective', p.key)),
      )
      if (p.status === 'archived') ctx.overlay.push({ table: 'perspectives', ref: `perspective:${p.key}`, id, from: 'approved', to: 'archived' })
      break
    }
    case 'rejected':
      await unwrap(forge.perspectives.reject(id, { rejection_reason: p.rejection_reason ?? '' }), `reject perspective ${p.key}`)
      break
    case 'in_review':
    case 'draft':
      break
  }
}

/** Phase 6. */
export async function phasePerspectives(ctx: GenContext): Promise<void> {
  phase(ctx, 'perspectives (derive + drafts)')
  const { corpus, forge, ledger } = ctx

  for (const p of corpus.perspectives) {
    const bulletId = ledger.id('bullet', p.bulletKey)
    const bulletCreated = ledger.time('bullet', p.bulletKey)
    const after = ledger.get('bullet', p.bulletKey).times.approved_at ?? bulletCreated
    let row: PerspectiveRow
    if (p.via === 'derive') {
      const prep = await unwrap(
        forge.derivations.prepare({
          entity_type: 'bullet',
          entity_id: bulletId,
          client_id: CLIENT_ID,
          params: { archetype: p.archetype, domain: p.domain, framing: p.framing },
        }),
        `prepare perspective ${p.key}`,
      )
      row = (await unwrap(
        forge.derivations.commitPerspective(prep.derivation_id, { content: p.content, reasoning: p.reasoning ?? `Framed for ${p.archetype}.` }),
        `commit perspective ${p.key}`,
      )) as PerspectiveRow
      if (row.status !== 'in_review') throw new Error(`perspective ${p.key}: committed as ${row.status}`)
      if (!row.prompt_log_id) throw new Error(`perspective ${p.key}: commit wrote no prompt log`)
      ledger.add('perspective', p.key, 'perspectives', row.id, { created_at: stamp(ctx, p.daysAgo, `perspective:${p.key}`, after) })
      ledger.add('prompt_log', `perspective:${p.key}`, 'prompt_logs', row.prompt_log_id)
    } else {
      row = (await unwrap(
        forge.perspectives.create({
          bullet_id: bulletId,
          content: p.content,
          target_archetype: p.archetype,
          domain: p.domain,
          framing: p.framing,
          auto_approve: false,
        }),
        `create draft perspective ${p.key}`,
      )) as PerspectiveRow
      if (row.status !== 'draft') throw new Error(`perspective ${p.key}: created as ${row.status}, expected draft`)
      ledger.add('perspective', p.key, 'perspectives', row.id, { created_at: stamp(ctx, p.daysAgo, `perspective:${p.key}`, bulletCreated) })
    }
    if (row.content !== p.content) throw new Error(`perspective ${p.key}: content was altered (${JSON.stringify(row.content)})`)
    await walkPerspective(ctx, p, row.id)
  }
}
