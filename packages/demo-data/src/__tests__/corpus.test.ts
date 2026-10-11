/**
 * Corpus checks, run for every persona without a server: content conventions, references,
 * enum values (checked against the seeded rows of a freshly migrated database), board
 * coverage, the uniqueness the fingerprint's natural keys rely on, and API preconditions
 * (a derivation needs an approved bullet, a resume entry an approved perspective, ...).
 */

import { afterAll, describe, expect, test } from 'bun:test'
import { existsSync, readFileSync } from 'fs'
import { join } from 'path'
import { scanText, scanValue } from '../conventions'
import {
  ARCHETYPES,
  CERTIFICATE_SUBTYPES,
  CONTACT_JD_RELATIONSHIPS,
  CONTACT_ORG_RELATIONSHIPS,
  CONTACT_RESUME_RELATIONSHIPS,
  CREDENTIAL_TYPES,
  DEGREE_LEVELS,
  DOMAINS,
  EDUCATION_TYPES,
  EMPLOYMENT_TYPES,
  FRAMINGS,
  JD_STATUSES,
  LOCATION_MODALITIES,
  NOTE_ENTITY_TYPES,
  ORG_STATUSES,
  ORG_TAGS,
  ORG_TYPES,
  type PersonaCorpus,
  PRESENTATION_TYPES,
  REVIEW_STATUSES,
  SECTION_ENTRY_TYPES,
  SKILL_CATEGORIES,
  SOURCE_TYPES,
  TEMPLATES,
} from '../corpus/types'
import { ANSWER_OPTIONS } from '../corpus/types'
import { jdPath, resumePath } from '../generate/applications'
import { PERSONAS } from '../personas'
import { migratedDb, tempDir } from './helpers'

/** Size bands each persona promises (ranges are inclusive). */
const SIZES: Record<string, Record<string, [number, number]>> = {
  'early-career-developer': {
    orgs: [12, 18],
    sources: [10, 15],
    bullets: [30, 45],
    perspectives: [40, 60],
    skills: [25, 40],
    jds: [9, 11],
    contacts: [6, 8],
    notes: [6, 8],
    summaries: [2, 3],
    resumes: [5, 5],
  },
  'cleared-security-engineer': {
    orgs: [12, 18],
    sources: [10, 15],
    bullets: [30, 45],
    perspectives: [40, 60],
    skills: [25, 40],
    jds: [9, 11],
    contacts: [6, 8],
    notes: [6, 8],
    summaries: [2, 3],
    resumes: [5, 5],
  },
  'ai-ml-engineer': {
    orgs: [12, 18],
    sources: [10, 15],
    bullets: [30, 45],
    perspectives: [40, 60],
    skills: [25, 40],
    jds: [9, 11],
    contacts: [6, 8],
    notes: [6, 8],
    summaries: [2, 3],
    resumes: [5, 5],
  },
}

/** Column names of the template sections, by template (from migration 008). */
function templateSections(db: ReturnType<typeof migratedDb>): Map<string, { title: string; entry_type: string }[]> {
  const rows = db.query('SELECT name, sections FROM resume_templates').all() as { name: string; sections: string }[]
  return new Map(rows.map((r) => [r.name, JSON.parse(r.sections) as { title: string; entry_type: string }[]]))
}

const tmp = tempDir('corpus')
const db = migratedDb(join(tmp.dir, 'schema.sqlite'))
afterAll(() => {
  db.close()
  tmp.cleanup()
})

describe('enumerations match the seeded rows', () => {
  const names = (sql: string) => (db.query(sql).all() as { v: string }[]).map((r) => r.v).sort()
  test('archetypes', () => expect(names('SELECT name AS v FROM archetypes')).toEqual([...ARCHETYPES].sort()))
  test('domains', () => expect(names('SELECT name AS v FROM domains')).toEqual([...DOMAINS].sort()))
  test('skill categories', () => expect(names('SELECT slug AS v FROM skill_categories')).toEqual([...SKILL_CATEGORIES].sort()))
  test('built-in templates', () => expect(names('SELECT name AS v FROM resume_templates WHERE is_builtin = 1')).toEqual([...TEMPLATES].sort()))
  test('no built-in template has a certifications section (the generator adds one)', () => {
    for (const sections of templateSections(db).values()) expect(sections.map((s) => s.entry_type)).not.toContain('certifications')
  })
})

describe('jdPath and resumePath walk forward to every status', () => {
  test('every JD status is reachable and ends at itself', () => {
    for (const s of JD_STATUSES) {
      const path = jdPath(s)
      if (s === 'discovered') expect(path).toEqual([])
      else expect(path.at(-1)).toBe(s)
      expect(path).not.toContain('discovered')
    }
  })
  test('every resume status is reachable from draft', () => {
    for (const s of REVIEW_STATUSES) expect(resumePath(s).at(-1) ?? 'draft').toBe(s)
  })
})

describe('content scanner', () => {
  test('accepts fictional contact data', () => {
    expect(scanText('Mail a.b@example.com or call (614) 555-0142; see https://code.example.com/x and docs.example.org.')).toEqual([])
    expect(scanText('Staging lives at https://app.staging.test/login')).toEqual([])
  })
  test('rejects real-looking contact data and real organizations', () => {
    expect(scanText('mail me at someone@gmail.com')).not.toEqual([])
    expect(scanText('see https://www.realcompany.com/jobs')).not.toEqual([])
    expect(scanText('visit realsite.io for more')).not.toEqual([])
    expect(scanText('call 614-867-5309')).not.toEqual([])
    expect(scanText('(614) 555-1234')).not.toEqual([])
    expect(scanText('Previously at Google')).not.toEqual([])
    expect(scanText('B.S. from Stanford University')).not.toEqual([])
    expect(scanText('Contract with the NSA')).not.toEqual([])
  })
  test('does not trip on technology names', () => {
    expect(scanText('Node.js, ASP.NET, Vue.js and scikit-learn')).toEqual([])
  })
})

for (const corpus of Object.values(PERSONAS)) {
  describe(`persona ${corpus.slug}`, () => {
    const { dir: _dir, ...content } = corpus
    const skillKeys = new Set(corpus.skills.map((s) => s.key))
    const orgKeys = new Set(corpus.orgs.map((o) => o.key))
    const byKey = <T extends { key: string }>(items: T[]) => new Map(items.map((i) => [i.key, i]))
    const sources = byKey(corpus.sources)
    const bullets = byKey(corpus.bullets)
    const perspectives = byKey(corpus.perspectives)
    const jds = byKey(corpus.jds)
    const resumes = byKey(corpus.resumes)
    const templates = templateSections(db)

    test('content conventions hold for every string in the corpus and every JD file', () => {
      expect(scanValue(content)).toEqual([])
      for (const jd of corpus.jds) {
        const path = join(corpus.dir, 'jds', jd.file)
        expect(existsSync(path)).toBe(true)
        const text = readFileSync(path, 'utf8')
        expect(text.trim().length).toBeGreaterThan(200)
        expect(scanText(text).map((p) => `${jd.file}: ${p}`)).toEqual([])
      }
    })

    test('sizes are in the promised bands and under the 200-row list clamp', () => {
      const actual: Record<string, number> = {
        orgs: corpus.orgs.length,
        sources: corpus.sources.length,
        bullets: corpus.bullets.length,
        perspectives: corpus.perspectives.length,
        skills: corpus.skills.length,
        jds: corpus.jds.length,
        contacts: corpus.contacts.length,
        notes: corpus.notes.length,
        summaries: corpus.summaries.length,
        resumes: corpus.resumes.length,
      }
      for (const [k, [lo, hi]] of Object.entries(SIZES[corpus.slug] ?? {})) {
        expect({ [k]: actual[k] }).toEqual({ [k]: Math.min(Math.max(actual[k] ?? -1, lo), hi) })
      }
      expect(corpus.bullets.length).toBeLessThan(150)
      expect(corpus.perspectives.length).toBeLessThan(150)
    })

    test('keys are unique per kind', () => {
      const kinds: [string, { key: string }[]][] = [
        ['industries', corpus.industries], ['roleTypes', corpus.roleTypes], ['skills', corpus.skills], ['orgs', corpus.orgs],
        ['credentials', corpus.credentials], ['certifications', corpus.certifications], ['sources', corpus.sources],
        ['bullets', corpus.bullets], ['perspectives', corpus.perspectives], ['summaries', corpus.summaries], ['jds', corpus.jds],
        ['contacts', corpus.contacts], ['resumes', corpus.resumes], ['notes', corpus.notes],
      ]
      for (const [kind, items] of kinds) {
        const keys = items.map((i) => i.key)
        expect({ kind, dupes: keys.filter((k, i) => keys.indexOf(k) !== i) }).toEqual({ kind, dupes: [] })
      }
      const entryKeys = corpus.resumes.flatMap((r) => r.entries.flatMap((e) => (e.key ? [e.key] : [])))
      expect(entryKeys.filter((k, i) => entryKeys.indexOf(k) !== i)).toEqual([])
    })

    test('natural keys are unique (the fingerprint maps ids to them)', () => {
      const unique = (label: string, values: string[]) => {
        const dupes = values.filter((v, i) => values.indexOf(v) !== i)
        expect({ label, dupes }).toEqual({ label, dupes: [] })
      }
      const addresses = [corpus.profile.address, ...corpus.orgs.flatMap((o) => (o.locations ?? []).flatMap((l) => (l.address ? [l.address] : [])))]
      unique('address name / street_1', addresses.map((a) => `${a.name} / ${a.street_1 ?? '-'}`))
      unique('org names', corpus.orgs.map((o) => o.name))
      unique('location names per org', corpus.orgs.flatMap((o) => (o.locations ?? []).map((l) => `${o.key}/${l.name}`)))
      unique('skill names', corpus.skills.map((s) => s.name))
      unique('industry names', corpus.industries.map((s) => s.name))
      unique('role type names', corpus.roleTypes.map((s) => s.name))
      unique('source titles', corpus.sources.map((s) => s.title))
      unique('bullet contents', corpus.bullets.map((b) => b.content))
      unique('perspective contents', corpus.perspectives.map((p) => p.content))
      unique('certification short names', corpus.certifications.map((c) => c.short_name))
      unique('credential labels', corpus.credentials.map((c) => c.label))
      unique('summary titles', corpus.summaries.map((s) => s.title))
      unique('JD title @ org', corpus.jds.map((j) => `${j.title} @ ${j.orgKey ?? '-'}`))
      unique('contact names', corpus.contacts.map((c) => c.name))
      unique('resume names', corpus.resumes.map((r) => r.name))
      unique('note titles or contents', ['About this demo dataset', ...corpus.notes.map((n) => n.title ?? n.content)])
    })

    test('skills: case-insensitively unique, stored as written, at least 8 categories', () => {
      const lower = corpus.skills.map((s) => s.name.toLowerCase())
      expect(lower.filter((n, i) => lower.indexOf(n) !== i)).toEqual([])
      for (const s of corpus.skills) {
        // The API upper-cases the first character of a new skill's name.
        expect({ skill: s.name, first: s.name[0] }).toEqual({ skill: s.name, first: s.name[0]?.toUpperCase() })
        expect(SKILL_CATEGORIES).toContain(s.category)
        for (const d of s.domains ?? []) expect(DOMAINS).toContain(d)
      }
      expect(new Set(corpus.skills.map((s) => s.category)).size).toBeGreaterThanOrEqual(8)
    })

    test('every referenced skill is in the catalog (a derivation would invent unknown ones)', () => {
      const refs = [
        ...corpus.sources.flatMap((s) => s.skills),
        ...corpus.bullets.flatMap((b) => b.technologies),
        ...corpus.certifications.flatMap((c) => c.skills),
        ...corpus.summaries.flatMap((s) => s.skills),
        ...corpus.jds.flatMap((j) => j.skills),
        ...corpus.resumes.flatMap((r) => Object.values(r.skills).flat()),
        ...corpus.notes.flatMap((n) => n.refs.filter((r) => r.type === 'skill').map((r) => r.key)),
      ]
      expect(refs.filter((k) => !skillKeys.has(k))).toEqual([])
    })

    test('profile and answer bank', () => {
      const p = corpus.profile
      expect(p.salary_minimum <= p.salary_target && p.salary_target <= p.salary_stretch).toBe(true)
      expect(corpus.answers.map((a) => a.field_kind).sort()).toEqual(Object.keys(ANSWER_OPTIONS).sort())
      for (const a of corpus.answers) expect(ANSWER_OPTIONS[a.field_kind]?.values).toContain(a.value)
    })

    test('organizations use valid enums and cover every kanban status and NULL', () => {
      for (const o of corpus.orgs) {
        expect(ORG_TYPES).toContain(o.org_type)
        for (const t of o.tags) expect(ORG_TAGS).toContain(t)
        if (o.employment_type) expect(EMPLOYMENT_TYPES).toContain(o.employment_type)
        if (o.status !== null) expect(ORG_STATUSES).toContain(o.status)
        for (const l of o.locations ?? []) expect(LOCATION_MODALITIES).toContain(l.modality)
        expect(o.daysAgo).toBeLessThan(corpus.accountAgeDays - 1)
      }
      const statuses = new Set(corpus.orgs.map((o) => o.status))
      for (const s of [...ORG_STATUSES, null]) expect({ status: s, covered: statuses.has(s) }).toEqual({ status: s, covered: true })
    })

    test('sources: valid extensions, references and dates; every type and status covered', () => {
      const locations = new Set(corpus.orgs.flatMap((o) => (o.locations ?? []).map((l) => `${o.key}/${l.key}`)))
      for (const s of corpus.sources) {
        expect(SOURCE_TYPES).toContain(s.source_type)
        expect(REVIEW_STATUSES).toContain(s.status)
        const ext = { role: s.role, project: s.project, education: s.education, presentation: s.presentation }
        for (const [type, value] of Object.entries(ext)) expect({ key: s.key, type, present: value !== undefined }).toEqual({ key: s.key, type, present: s.source_type === type })
        if (s.role) expect(orgKeys.has(s.role.orgKey)).toBe(true)
        if (s.project?.orgKey) expect(orgKeys.has(s.project.orgKey)).toBe(true)
        if (s.education) {
          expect(EDUCATION_TYPES).toContain(s.education.education_type)
          if (s.education.orgKey) expect(orgKeys.has(s.education.orgKey)).toBe(true)
          if (s.education.campusKey) expect(locations.has(s.education.campusKey)).toBe(true)
          if (s.education.degree_level) expect(DEGREE_LEVELS).toContain(s.education.degree_level)
          if (s.education.certificate_subtype) expect(CERTIFICATE_SUBTYPES).toContain(s.education.certificate_subtype)
        }
        if (s.presentation) expect(PRESENTATION_TYPES).toContain(s.presentation.presentation_type)
        if (s.start && s.end) expect(s.start.monthsAgo).toBeGreaterThanOrEqual(s.end.monthsAgo)
        expect(s.daysAgo).toBeLessThan(corpus.accountAgeDays - 1)
      }
      for (const t of SOURCE_TYPES) expect(corpus.sources.map((s) => s.source_type)).toContain(t)
      for (const t of REVIEW_STATUSES) expect(corpus.sources.map((s) => s.status)).toContain(t)
    })

    test('bullets: derivations come from sources, drafts are drafts, rejections have reasons', () => {
      for (const b of corpus.bullets) {
        expect(sources.has(b.sourceKey)).toBe(true)
        expect(REVIEW_STATUSES).toContain(b.status)
        if (b.domain) expect(DOMAINS).toContain(b.domain)
        if (b.via === 'draft') expect(b.status).toBe('draft')
        else expect(b.status).not.toBe('draft') // a commit lands in_review
        expect(Boolean(b.rejection_reason?.trim())).toBe(b.status === 'rejected')
        if (b.reopenedAfter) expect(b.status).toBe('in_review')
      }
      for (const t of REVIEW_STATUSES) expect(corpus.bullets.map((b) => b.status)).toContain(t)
    })

    test('perspectives: derived only from approved bullets; drafts are drafts', () => {
      for (const p of corpus.perspectives) {
        const bullet = bullets.get(p.bulletKey)
        expect(bullet).toBeDefined()
        expect(ARCHETYPES).toContain(p.archetype)
        expect(DOMAINS).toContain(p.domain)
        expect(FRAMINGS).toContain(p.framing)
        // archived bullets are approved while the API runs; the overlay archives them later
        if (p.via === 'derive') expect(['approved', 'archived']).toContain(bullet?.status ?? 'missing')
        if (p.via === 'draft') expect(p.status).toBe('draft')
        else expect(p.status).not.toBe('draft')
        expect(Boolean(p.rejection_reason?.trim())).toBe(p.status === 'rejected')
      }
      for (const t of REVIEW_STATUSES) expect(corpus.perspectives.map((p) => p.status)).toContain(t)
    })

    test('JDs: valid orgs and statuses, all nine statuses covered, salaries ordered', () => {
      for (const jd of corpus.jds) {
        if (jd.orgKey) expect(orgKeys.has(jd.orgKey)).toBe(true)
        if (jd.salary_min !== undefined && jd.salary_max !== undefined) expect(jd.salary_min).toBeLessThanOrEqual(jd.salary_max)
      }
      for (const s of JD_STATUSES) expect(corpus.jds.map((j) => j.status)).toContain(s)
    })

    test('resumes: one per status, template sections exist, entries use approved, unarchived material', () => {
      for (const s of REVIEW_STATUSES) expect(corpus.resumes.filter((r) => r.status === s).length).toBeGreaterThanOrEqual(1)
      for (const r of corpus.resumes) {
        expect(ARCHETYPES).toContain(r.archetype)
        const sections = [...(templates.get(r.template) ?? []), ...r.extraSections]
        expect(sections.length).toBeGreaterThan(0)
        const typeOf = (title: string) => sections.find((s) => s.title === title)?.entry_type
        for (const extra of r.extraSections) expect(SECTION_ENTRY_TYPES).toContain(extra.entry_type)
        for (const e of r.entries) {
          expect([e.perspectiveKey, e.sourceKey, e.content].filter((x) => x !== undefined).length).toBe(1)
          expect(['skills', 'certifications', undefined]).not.toContain(typeOf(e.section))
          if (e.perspectiveKey) {
            const p = perspectives.get(e.perspectiveKey)
            expect({ entry: e.perspectiveKey, status: p?.status }).toEqual({ entry: e.perspectiveKey, status: 'approved' })
            expect(bullets.get(p?.bulletKey ?? '')?.status).not.toBe('archived')
            expect(sources.get(bullets.get(p?.bulletKey ?? '')?.sourceKey ?? '')?.status).not.toBe('archived')
          }
          if (e.sourceKey) expect(['approved', 'in_review']).toContain(sources.get(e.sourceKey)?.status ?? 'missing')
        }
        for (const title of Object.keys(r.skills)) expect(typeOf(title)).toBe('skills')
        for (const title of Object.keys(r.certifications)) expect(typeOf(title)).toBe('certifications')
        for (const key of Object.values(r.certifications).flat()) expect(corpus.certifications.map((c) => c.key)).toContain(key)
        // generated_tagline comes from linked JDs, and every resume must have one
        expect(r.jdKeys.length).toBeGreaterThan(0)
        for (const k of r.jdKeys) expect(jds.has(k)).toBe(true)
        if (r.summaryKey) expect(corpus.summaries.map((s) => s.key)).toContain(r.summaryKey)
      }
    })

    test('contacts cover every relationship of all three link kinds', () => {
      for (const c of corpus.contacts) {
        if (c.orgKey) expect(orgKeys.has(c.orgKey)).toBe(true)
        for (const l of c.orgLinks) expect(orgKeys.has(l.orgKey)).toBe(true)
        for (const l of c.jdLinks) expect(jds.has(l.jdKey)).toBe(true)
        for (const l of c.resumeLinks) expect(resumes.has(l.resumeKey)).toBe(true)
      }
      const orgRels = new Set(corpus.contacts.flatMap((c) => c.orgLinks.map((l) => l.relationship)))
      const jdRels = new Set(corpus.contacts.flatMap((c) => c.jdLinks.map((l) => l.relationship)))
      const resumeRels = new Set(corpus.contacts.flatMap((c) => c.resumeLinks.map((l) => l.relationship)))
      expect([...CONTACT_ORG_RELATIONSHIPS].filter((r) => !orgRels.has(r))).toEqual([])
      expect([...CONTACT_JD_RELATIONSHIPS].filter((r) => !jdRels.has(r))).toEqual([])
      expect([...CONTACT_RESUME_RELATIONSHIPS].filter((r) => !resumeRels.has(r))).toEqual([])
    })

    test('notes reference all eleven entity types, and every reference resolves', () => {
      const keysOf: Record<string, Set<string>> = {
        source: new Set(sources.keys()),
        bullet: new Set(bullets.keys()),
        perspective: new Set(perspectives.keys()),
        resume_entry: new Set(corpus.resumes.flatMap((r) => r.entries.flatMap((e) => (e.key ? [e.key] : [])))),
        resume: new Set(resumes.keys()),
        skill: skillKeys,
        organization: orgKeys,
        job_description: new Set(jds.keys()),
        contact: new Set(corpus.contacts.map((c) => c.key)),
        credential: new Set(corpus.credentials.map((c) => c.key)),
        certification: new Set(corpus.certifications.map((c) => c.key)),
      }
      for (const n of corpus.notes) for (const r of n.refs) expect({ note: n.key, ref: r, ok: keysOf[r.type]?.has(r.key) }).toEqual({ note: n.key, ref: r, ok: true })
      const types = new Set(corpus.notes.flatMap((n) => n.refs.map((r) => r.type)))
      expect([...NOTE_ENTITY_TYPES].filter((t) => !types.has(t))).toEqual([])
    })

    test('qualifications', () => {
      for (const c of corpus.credentials) {
        expect(CREDENTIAL_TYPES).toContain(c.credential_type)
        if (c.orgKey) expect(orgKeys.has(c.orgKey)).toBe(true)
      }
      for (const c of corpus.certifications) {
        if (c.issuerKey) expect(orgKeys.has(c.issuerKey)).toBe(true)
        if (c.in_progress) expect(c.earned).toBeUndefined()
      }
    })

    test('summaries reference known industries and role types', () => {
      for (const s of corpus.summaries) {
        if (s.industryKey) expect(corpus.industries.map((i) => i.key)).toContain(s.industryKey)
        if (s.roleTypeKey) expect(corpus.roleTypes.map((i) => i.key)).toContain(s.roleTypeKey)
      }
      expect(corpus.summaries.some((s) => s.industryKey) && corpus.summaries.some((s) => s.roleTypeKey)).toBe(true)
    })
  })
}

describe('early-career-developer specifics', () => {
  const c: PersonaCorpus | undefined = PERSONAS['early-career-developer']
  test('covers the flows it exists for', () => {
    expect(c).toBeDefined()
    if (!c) return
    const edu = c.sources.flatMap((s) => (s.education ? [s.education] : []))
    for (const t of EDUCATION_TYPES) expect(edu.map((e) => e.education_type)).toContain(t)
    expect(edu.some((e) => e.degree_level === 'bachelors' && e.gpa && e.campusKey)).toBe(true)
    expect(edu.some((e) => e.is_in_progress && (e.degree_level === 'masters' || e.degree_level === 'graduate_certificate'))).toBe(true)
    expect(edu.some((e) => e.certificate_subtype === 'completion')).toBe(true)
    const uni = c.orgs.find((o) => o.tags.includes('university'))
    expect(uni?.locations?.some((l) => l.address)).toBe(true)
    expect(c.orgs.some((o) => o.employment_type === 'intern')).toBe(true)
    const presentations = c.sources.flatMap((s) => (s.presentation ? [s.presentation.presentation_type] : []))
    expect(presentations).toContain('lightning_talk')
    expect(presentations).toContain('poster')
    expect(c.certifications.some((x) => x.in_progress)).toBe(true)
    expect(c.credentials.some((x) => x.credential_type === 'drivers_license')).toBe(true)
    expect(c.resumes.every((r) => r.template === 'Standard Tech Resume')).toBe(true)
  })
})

describe('cleared-security-engineer specifics', () => {
  const c: PersonaCorpus | undefined = PERSONAS['cleared-security-engineer']
  test('covers the flows it exists for', () => {
    expect(c).toBeDefined()
    if (!c) return
    const clearance = c.credentials.find((x) => x.credential_type === 'clearance')
    expect(clearance?.status).toBe('active')
    expect(clearance?.details).toEqual({ level: 'top_secret', polygraph: 'ci', clearance_type: 'personnel', access_programs: ['sci'] })
    expect(c.orgs.find((o) => o.key === clearance?.orgKey)?.org_type).toBe('government')
    expect(c.credentials.some((x) => x.credential_type === 'drivers_license')).toBe(true)
    expect(c.certifications.length).toBeGreaterThanOrEqual(3)
    expect(c.certifications.length).toBeLessThanOrEqual(4)
    for (const cert of c.certifications) {
      expect(cert.skills.length).toBeGreaterThan(0)
      expect(cert.issuerKey).toBeDefined()
    }
    expect(c.certifications.some((x) => x.in_progress)).toBe(true)
    const orgTypes = new Set(c.orgs.map((o) => o.org_type))
    for (const t of ['military', 'government', 'volunteer'] as const) expect(orgTypes).toContain(t)
    const employment = new Set(c.orgs.map((o) => o.employment_type))
    for (const t of ['military_active', 'contractor', 'volunteer'] as const) expect(employment).toContain(t)
    const federal = c.resumes.filter((r) => r.template === 'Federal Resume')
    expect(federal.length).toBeGreaterThanOrEqual(2)
    expect(c.resumes.some((r) => r.template === 'Standard Tech Resume')).toBe(true)
    expect(federal.some((r) => r.extraSections.some((s) => s.entry_type === 'certifications') && Object.values(r.certifications).flat().length > 0)).toBe(true)
    expect(c.resumes.some((r) => r.extraSections.some((s) => s.entry_type === 'presentations'))).toBe(true)
    for (const a of ['security-engineer', 'public-sector', 'infrastructure'] as const) {
      expect(c.resumes.map((r) => r.archetype)).toContain(a)
      expect(c.perspectives.filter((p) => p.status === 'approved').map((p) => p.archetype)).toContain(a)
    }
    const locations = c.jds.map((j) => j.location ?? '')
    for (const where of [', VA', ', MD', ', DC', ', CO', 'Remote']) expect({ where, found: locations.some((l) => l.includes(where)) }).toEqual({ where, found: true })
    const answer = (k: string) => c.answers.find((a) => a.field_kind === k)?.value
    expect(answer('eeo.veteran')).toBe('Protected Veteran')
    for (const k of ['eeo.gender', 'eeo.race', 'eeo.disability']) expect(['Decline to self-identify', 'Prefer not to say']).toContain(answer(k) ?? '')
    expect(c.profile.salary_minimum).toBeGreaterThanOrEqual(150000)
  })
})

describe('ai-ml-engineer specifics', () => {
  const c: PersonaCorpus | undefined = PERSONAS['ai-ml-engineer']
  const templates = templateSections(db)

  test('projects: open source, personal and employer-backed', () => {
    expect(c).toBeDefined()
    if (!c) return
    const projects = c.sources.flatMap((s) => (s.project ? [s.project] : []))
    expect(projects.some((p) => p.open_source && p.is_personal)).toBe(true)
    expect(projects.some((p) => p.open_source && !p.is_personal && p.orgKey)).toBe(true)
    for (const p of projects) expect(p.url).toBeDefined()
  })

  test('presentations cover every presentation type', () => {
    if (!c) return
    const types = c.sources.flatMap((s) => (s.presentation ? [s.presentation.presentation_type] : []))
    expect([...PRESENTATION_TYPES].filter((t) => !types.includes(t))).toEqual([])
  })

  test('resumes: an Academic CV with filled presentations and awards sections, plus Standard Tech Resumes', () => {
    if (!c) return
    const cv = c.resumes.find((r) => r.template === 'Academic CV')
    expect(cv).toBeDefined()
    if (!cv) return
    const sections = [...(templates.get(cv.template) ?? []), ...cv.extraSections]
    const filled = (entryType: string) =>
      cv.entries.filter((e) => sections.find((s) => s.title === e.section)?.entry_type === entryType).length
    expect(filled('presentations')).toBeGreaterThanOrEqual(3)
    expect(filled('awards')).toBeGreaterThanOrEqual(2)
    expect(Object.values(cv.certifications).flat().length).toBeGreaterThan(0)
    expect(c.resumes.filter((r) => r.template === 'Standard Tech Resume').length).toBeGreaterThanOrEqual(3)
    // A resume that lists certifications needs a section to put them in.
    for (const r of c.resumes) {
      if (Object.values(r.certifications).flat().length > 0) {
        expect(r.extraSections.map((s) => s.entry_type)).toContain('certifications')
      }
    }
  })

  test('archetypes agentic-ai, solutions-architect and hft drive perspectives and resumes', () => {
    if (!c) return
    for (const a of ['agentic-ai', 'solutions-architect', 'hft'] as const) {
      expect(c.perspectives.filter((p) => p.archetype === a && p.status === 'approved').length).toBeGreaterThanOrEqual(3)
      expect(c.resumes.map((r) => r.archetype)).toContain(a)
    }
    for (const d of DOMAINS) expect(c.perspectives.map((p) => p.domain)).toContain(d)
  })

  test('JDs: WA, CA, NY and remote; senior/staff salaries', () => {
    if (!c) return
    const locations = c.jds.map((j) => j.location ?? '')
    for (const where of [/, WA\b/, /, CA\b/, /, NY\b/, /^Remote/]) expect(locations.some((l) => where.test(l))).toBe(true)
    for (const jd of c.jds) expect(jd.salary_min ?? 0).toBeGreaterThanOrEqual(180000)
    expect(c.profile.salary_minimum).toBeGreaterThanOrEqual(180000)
  })

  test("EEO answers decline; a driver's license and one to three certifications", () => {
    if (!c) return
    for (const a of c.answers.filter((x) => x.field_kind.startsWith('eeo.'))) {
      expect(['Decline to self-identify', 'Prefer not to say']).toContain(a.value)
    }
    expect(c.credentials.some((x) => x.credential_type === 'drivers_license')).toBe(true)
    expect(c.certifications.length).toBeGreaterThanOrEqual(1)
    expect(c.certifications.length).toBeLessThanOrEqual(3)
    expect(c.certifications.some((x) => x.in_progress)).toBe(true)
  })
})
