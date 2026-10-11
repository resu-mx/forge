/**
 * Phases 1–2: reference data (industries, role types, the whole skill catalog) and the
 * profile with its answer bank.
 */

import { unwrap } from '../api'
import { ANSWER_OPTIONS } from '../corpus/types'
import { type GenContext, phase, stamp } from './context'

interface Named {
  id: string
  name: string
}

/** Load the seeded archetypes, domains and templates by name. */
export async function loadSeeded(ctx: GenContext): Promise<void> {
  const archetypes = await ctx.raw<Named[]>('GET', '/archetypes?limit=200')
  const domains = await ctx.raw<Named[]>('GET', '/domains?limit=200')
  const templates = await ctx.raw<Named[]>('GET', '/templates')
  for (const a of archetypes) ctx.seeded.archetypes.set(a.name, a.id)
  for (const d of domains) ctx.seeded.domains.set(d.name, d.id)
  for (const t of templates) ctx.seeded.templates.set(t.name, t.id)
}

/**
 * Phase 1. The full skill catalog is created before any derivation: committing bullets
 * links technologies by case-insensitive name and creates a lower-case `tool` skill for
 * any name it does not find.
 */
export async function phaseReference(ctx: GenContext): Promise<void> {
  phase(ctx, 'reference data: industries, role types, skill catalog')
  const { corpus, forge, ledger } = ctx
  const setupDay = corpus.accountAgeDays - 1

  for (const ind of corpus.industries) {
    const row = await unwrap(forge.industries.create({ name: ind.name, description: ind.description }), `industry ${ind.key}`)
    ledger.add('industry', ind.key, 'industries', row.id, { created_at: stamp(ctx, setupDay, `industry:${ind.key}`) })
  }
  for (const rt of corpus.roleTypes) {
    const row = await unwrap(forge.roleTypes.create({ name: rt.name, description: rt.description }), `role type ${rt.key}`)
    ledger.add('role_type', rt.key, 'role_types', row.id, { created_at: stamp(ctx, setupDay, `role_type:${rt.key}`) })
  }
  for (const skill of corpus.skills) {
    const row = await unwrap(forge.skills.create({ name: skill.name, category: skill.category }), `skill ${skill.key}`)
    if (row.name !== skill.name || row.category !== skill.category) {
      throw new Error(`skill ${skill.key}: stored as "${row.name}" (${row.category}), expected "${skill.name}" (${skill.category})`)
    }
    ledger.add('skill', skill.key, 'skills', row.id, { created_at: stamp(ctx, setupDay, `skill:${skill.key}`) })
    for (const domain of skill.domains ?? []) {
      const domainId = ctx.seeded.domains.get(domain)
      if (!domainId) throw new Error(`skill ${skill.key}: unknown domain ${domain}`)
      await unwrap(forge.skills.addDomain(row.id, domainId), `skill ${skill.key} domain ${domain}`)
    }
  }
}

/** Phase 2. Update the placeholder profile row (migration 005), then the answer bank. */
export async function phaseProfile(ctx: GenContext): Promise<void> {
  phase(ctx, 'profile and answer bank')
  const { corpus, forge, ledger } = ctx
  const p = corpus.profile
  await unwrap(
    forge.profile.update({
      name: p.name,
      email: p.email,
      phone: p.phone,
      address: { ...p.address, country_code: 'US' },
      urls: p.urls,
      salary_minimum: p.salary_minimum,
      salary_target: p.salary_target,
      salary_stretch: p.salary_stretch,
    }),
    'profile update',
  )
  const profile = await unwrap(forge.profile.get(), 'profile get')
  if (!profile.address_id) throw new Error('profile update did not create an address')
  const created = ctx.clock.epoch(corpus.accountAgeDays)
  const updated = stamp(ctx, corpus.accountAgeDays - 1, 'profile:updated', created)
  ledger.add('profile', 'profile', 'user_profile', profile.id, { created_at: created, updated_at: updated })
  ledger.add('address', 'profile', 'addresses', profile.address_id, { created_at: updated, updated_at: updated })

  for (const answer of corpus.answers) {
    const option = ANSWER_OPTIONS[answer.field_kind]
    if (!option) throw new Error(`unknown answer field_kind ${answer.field_kind}`)
    const row = await unwrap(
      forge.answerBank.upsert({ field_kind: answer.field_kind, label: option.label, value: answer.value }),
      `answer ${answer.field_kind}`,
    )
    const t = stamp(ctx, corpus.accountAgeDays - 2, `answer:${answer.field_kind}`)
    ledger.add('answer', answer.field_kind, 'answer_bank', row.id, { created_at: t, updated_at: t })
  }
}
