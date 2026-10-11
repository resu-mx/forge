/**
 * Phases 7–10: summaries, job descriptions, contacts, resumes.
 */

import { readFileSync } from 'fs'
import { join } from 'path'
import { unwrap } from '../api'
import type { JdStatus, ResumeSpec, ReviewStatus } from '../corpus/types'
import { type GenContext, phase, stamp } from './context'

/** Phase 7. */
export async function phaseSummaries(ctx: GenContext): Promise<void> {
  phase(ctx, 'summaries')
  const { corpus, forge, ledger } = ctx
  for (const s of corpus.summaries) {
    const row = await unwrap(
      forge.summaries.create({
        title: s.title,
        role: s.role,
        description: s.description,
        is_template: s.is_template ?? false,
        industry_id: s.industryKey ? ledger.id('industry', s.industryKey) : null,
        role_type_id: s.roleTypeKey ? ledger.id('role_type', s.roleTypeKey) : null,
        notes: s.notes,
      }),
      `summary ${s.key}`,
    )
    const t = stamp(ctx, s.daysAgo, `summary:${s.key}`)
    ledger.add('summary', s.key, 'summaries', row.id, { created_at: t, updated_at: t })
    for (const skillKey of s.skills) {
      await unwrap(forge.summaries.addSkill(row.id, ledger.id('skill', skillKey)), `summary ${s.key} skill ${skillKey}`)
    }
  }
}

const FORWARD: readonly JdStatus[] = ['discovered', 'analyzing', 'applying', 'applied', 'interviewing', 'offered']

/**
 * The statuses a JD passes through after `discovered` to reach `target`: forward along the
 * pipeline, then into a terminal state from a plausible point.
 */
export function jdPath(target: JdStatus): JdStatus[] {
  const i = FORWARD.indexOf(target)
  if (i >= 0) return FORWARD.slice(1, i + 1)
  switch (target) {
    case 'rejected':
      return ['analyzing', 'applying', 'applied', 'interviewing', 'rejected']
    case 'withdrawn':
      return ['analyzing', 'applying', 'applied', 'withdrawn']
    case 'closed':
      return ['analyzing', 'closed']
    default:
      throw new Error(`no path to JD status ${target}`)
  }
}

/** Phase 8. */
export async function phaseJobDescriptions(ctx: GenContext): Promise<void> {
  phase(ctx, 'job descriptions')
  const { corpus, forge, ledger } = ctx
  for (const jd of corpus.jds) {
    const rawText = readFileSync(join(corpus.dir, 'jds', jd.file), 'utf8').trim()
    const row = await unwrap(
      forge.jobDescriptions.create({
        title: jd.title,
        organization_id: jd.orgKey ? ledger.id('org', jd.orgKey) : undefined,
        url: jd.url,
        raw_text: rawText,
        salary_min: jd.salary_min,
        salary_max: jd.salary_max,
        salary_range: jd.salary_range,
        location: jd.location,
      }),
      `job description ${jd.key}`,
    )
    if (row.status !== 'discovered') throw new Error(`JD ${jd.key}: created as ${row.status}`)
    for (const status of jdPath(jd.status)) {
      await unwrap(forge.jobDescriptions.update(row.id, { status }), `JD ${jd.key} → ${status}`)
    }
    const created = stamp(ctx, jd.daysAgo, `jd:${jd.key}`)
    const updated = jd.status === 'discovered' ? created : stamp(ctx, jd.updatedDaysAgo ?? jd.daysAgo, `jd:${jd.key}:updated`, created)
    ledger.add('jd', jd.key, 'job_descriptions', row.id, { created_at: created, updated_at: updated })
    for (const skillKey of jd.skills) {
      await unwrap(forge.jobDescriptions.addSkill(row.id, { skill_id: ledger.id('skill', skillKey) }), `JD ${jd.key} skill ${skillKey}`)
    }
  }
}

/** Phase 9: contacts and their organization and JD links (resume links come with resumes). */
export async function phaseContacts(ctx: GenContext): Promise<void> {
  phase(ctx, 'contacts')
  const { corpus, forge, ledger } = ctx
  for (const c of corpus.contacts) {
    const row = await unwrap(
      forge.contacts.create({
        name: c.name,
        title: c.title,
        email: c.email,
        phone: c.phone,
        linkedin: c.linkedin,
        team: c.team,
        dept: c.dept,
        notes: c.notes,
        organization_id: c.orgKey ? ledger.id('org', c.orgKey) : undefined,
      }),
      `contact ${c.key}`,
    )
    const t = stamp(ctx, c.daysAgo, `contact:${c.key}`)
    ledger.add('contact', c.key, 'contacts', row.id, { created_at: t, updated_at: t })
    for (const link of c.orgLinks) {
      await unwrap(forge.contacts.linkOrganization(row.id, ledger.id('org', link.orgKey), link.relationship), `contact ${c.key} org link`)
    }
    for (const link of c.jdLinks) {
      await unwrap(forge.contacts.linkJobDescription(row.id, ledger.id('jd', link.jdKey), link.relationship), `contact ${c.key} JD link`)
    }
  }
}

/** The resume statuses walked (via PATCH) to reach `target` from `draft`. */
export function resumePath(target: ReviewStatus): ReviewStatus[] {
  switch (target) {
    case 'draft':
      return []
    case 'in_review':
      return ['in_review']
    case 'approved':
      return ['in_review', 'approved']
    case 'rejected':
      return ['in_review', 'rejected']
    case 'archived':
      return ['in_review', 'approved', 'archived']
  }
}

interface SectionRow {
  id: string
  title: string
  entry_type: string
}

async function buildResume(ctx: GenContext, r: ResumeSpec): Promise<void> {
  const { forge, ledger } = ctx
  const templateId = ctx.seeded.templates.get(r.template)
  if (!templateId) throw new Error(`resume ${r.key}: template "${r.template}" is not seeded`)
  const resume = await unwrap(
    forge.resumes.create({
      name: r.name,
      target_role: r.target_role,
      target_employer: r.target_employer,
      archetype: r.archetype,
      summary_id: r.summaryKey ? ledger.id('summary', r.summaryKey) : undefined,
      template_id: templateId,
    }),
    `resume ${r.key}`,
  )
  const created = stamp(ctx, r.daysAgo, `resume:${r.key}`)
  const updated = r.status === 'draft' ? created : stamp(ctx, r.updatedDaysAgo ?? r.daysAgo, `resume:${r.key}:updated`, created)
  ledger.add('resume', r.key, 'resumes', resume.id, { created_at: created, updated_at: updated })

  for (const [i, extra] of r.extraSections.entries()) {
    await unwrap(forge.resumes.createSection(resume.id, { title: extra.title, entry_type: extra.entry_type, position: 100 + i }), `resume ${r.key} section ${extra.title}`)
  }
  const sections = await unwrap(forge.resumes.listSections(resume.id), `resume ${r.key} sections`) as SectionRow[]
  const sectionId = (title: string): string => {
    const s = sections.find((x) => x.title === title)
    if (!s) throw new Error(`resume ${r.key}: no section "${title}" (has ${sections.map((x) => x.title).join(', ')})`)
    return s.id
  }
  for (const s of sections) ledger.add('resume_section', `${r.key}/${s.title}`, 'resume_sections', s.id)

  for (const [i, e] of r.entries.entries()) {
    const entry = await unwrap(
      forge.resumes.addEntry(resume.id, {
        section_id: sectionId(e.section),
        perspective_id: e.perspectiveKey ? ledger.id('perspective', e.perspectiveKey) : undefined,
        source_id: e.sourceKey ? ledger.id('source', e.sourceKey) : undefined,
        content: e.content,
      }),
      `resume ${r.key} entry ${i}`,
    )
    ledger.add('resume_entry', e.key ?? `${r.key}#${i}`, 'resume_entries', entry.id)
  }
  for (const [title, skillKeys] of Object.entries(r.skills)) {
    for (const skillKey of skillKeys) {
      const rs = await unwrap(forge.resumes.addSkill(resume.id, sectionId(title), ledger.id('skill', skillKey)), `resume ${r.key} skill ${skillKey}`)
      ledger.add('resume_skill', `${r.key}/${title}/${skillKey}`, 'resume_skills', rs.id)
    }
  }
  for (const [title, certKeys] of Object.entries(r.certifications)) {
    for (const certKey of certKeys) {
      const rc = await unwrap(
        forge.resumes.addCertification(resume.id, { certification_id: ledger.id('certification', certKey), section_id: sectionId(title) }),
        `resume ${r.key} certification ${certKey}`,
      )
      ledger.add('resume_certification', `${r.key}/${certKey}`, 'resume_certifications', rc.id)
    }
  }
  for (const jdKey of r.jdKeys) {
    await unwrap(forge.jobDescriptions.linkResume(ledger.id('jd', jdKey), resume.id), `link resume ${r.key} to JD ${jdKey}`)
  }
}

/** Phase 10: resumes from templates, then contact → resume links, taglines, status walks. */
export async function phaseResumes(ctx: GenContext): Promise<void> {
  phase(ctx, 'resumes')
  const { corpus, forge, ledger } = ctx
  for (const r of corpus.resumes) await buildResume(ctx, r)

  for (const c of corpus.contacts) {
    for (const link of c.resumeLinks) {
      await unwrap(forge.contacts.linkResume(ledger.id('contact', c.key), ledger.id('resume', link.resumeKey), link.relationship), `contact ${c.key} resume link`)
    }
  }

  // Linking a JD does not regenerate the tagline on the Rust API (resu-mx/forge#71), so
  // regenerate explicitly, after every link exists.
  for (const r of corpus.resumes) {
    const result = await unwrap(forge.resumes.regenerateTagline(ledger.id('resume', r.key)), `regenerate tagline ${r.key}`)
    if (!result.generated_tagline) throw new Error(`resume ${r.key}: regenerated tagline is empty`)
  }

  for (const r of corpus.resumes) {
    for (const status of resumePath(r.status)) {
      await unwrap(forge.resumes.update(ledger.id('resume', r.key), { status }), `resume ${r.key} → ${status}`)
    }
  }
}
