/**
 * Shared test helpers for repository tests.
 * Creates in-memory databases with migrations applied and optional seed data.
 */
import { Database } from 'bun:sqlite'
import { getDatabase } from '../connection'
import { runMigrations } from '../migrate'
import { resolve } from 'path'

const MIGRATIONS_DIR = resolve(import.meta.dir, '../migrations')

/** Create a fresh database (in-memory unless `path` is given) with all migrations applied */
export function createTestDb(path: string = ':memory:'): Database {
  const db = getDatabase(path)
  runMigrations(db, MIGRATIONS_DIR)
  return db
}

/** Generate a valid UUID v4 for testing */
export function testUuid(): string {
  return crypto.randomUUID()
}

/** Seed a test organization and return its ID */
export function seedOrganization(db: Database, opts: {
  name?: string
  orgType?: string
  industry?: string
  size?: string
  worked?: boolean
  employmentType?: string
  website?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO organizations (id, name, org_type, industry, size, worked, employment_type, website)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      opts.name ?? 'Test Corp',
      opts.orgType ?? 'company',
      opts.industry ?? null,
      opts.size ?? null,
      opts.worked === undefined ? 0 : (opts.worked ? 1 : 0),
      opts.employmentType ?? null,
      opts.website ?? null,
    ]
  )
  return id
}

/** Seed a test source and return its ID */
export function seedSource(db: Database, opts: {
  title?: string
  description?: string
  status?: string
  sourceType?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO sources (id, title, description, source_type, status)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      opts.title ?? 'Test Source',
      opts.description ?? 'Led a team of 4 engineers to migrate cloud forensics platform.',
      opts.sourceType ?? 'general',
      opts.status ?? 'approved',
    ]
  )
  return id
}

/** Seed a test bullet with source associations and return its ID */
export function seedBullet(db: Database, sourceIds: Array<{ id: string; isPrimary?: boolean }> = [], opts: {
  content?: string
  status?: string
  domain?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO bullets (id, content, source_content_snapshot, status, domain)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      opts.content ?? 'Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch',
      'snapshot of source content',
      opts.status ?? 'approved',
      opts.domain ?? null,
    ]
  )
  // Insert bullet_sources rows
  for (const src of sourceIds) {
    const isPrimary = src.isPrimary !== undefined
      ? (src.isPrimary ? 1 : 0)
      : (sourceIds.length === 1 ? 1 : 0)
    db.run(
      'INSERT INTO bullet_sources (bullet_id, source_id, is_primary) VALUES (?, ?, ?)',
      [id, src.id, isPrimary]
    )
  }
  return id
}

/** Seed a test perspective and return its ID */
export function seedPerspective(db: Database, bulletId: string, opts: {
  content?: string
  status?: string
  archetype?: string
  domain?: string
  framing?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO perspectives (id, bullet_id, content, bullet_content_snapshot, target_archetype, domain, framing, status)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      bulletId,
      opts.content ?? 'Led cloud platform migration enabling ML-based log analysis pipeline',
      'snapshot of bullet content',
      opts.archetype ?? 'agentic-ai',
      opts.domain ?? 'ai_ml',
      opts.framing ?? 'accomplishment',
      opts.status ?? 'approved',
    ]
  )
  return id
}

/** Seed a test resume and return its ID */
export function seedResume(db: Database, opts: {
  name?: string
  targetRole?: string
  targetEmployer?: string
  archetype?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO resumes (id, name, target_role, target_employer, archetype)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      opts.name ?? 'Test Resume',
      opts.targetRole ?? 'AI Engineer',
      opts.targetEmployer ?? 'Anthropic',
      opts.archetype ?? 'agentic-ai',
    ]
  )
  return id
}

/** Seed a resume section and return its ID */
export function seedResumeSection(db: Database, resumeId: string, title: string, entryType: string, position?: number): string {
  const id = testUuid()
  db.run(
    `INSERT INTO resume_sections (id, resume_id, title, entry_type, position)
     VALUES (?, ?, ?, ?, ?)`,
    [id, resumeId, title, entryType, position ?? 0]
  )
  return id
}

/** Seed a test resume entry and return its ID */
export function seedResumeEntry(db: Database, sectionId: string, opts: {
  perspectiveId?: string
  sourceId?: string
  content?: string | null
  position?: number
} = {}): string {
  const id = testUuid()
  // Look up resume_id from the section
  const section = db.query('SELECT resume_id FROM resume_sections WHERE id = ?').get(sectionId) as { resume_id: string }
  db.run(
    `INSERT INTO resume_entries (id, resume_id, section_id, perspective_id, source_id, content, position)
     VALUES (?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      section.resume_id,
      sectionId,
      opts.perspectiveId ?? null,
      opts.sourceId ?? null,
      opts.content ?? null,
      opts.position ?? 0,
    ]
  )
  return id
}

/** Seed a resume skill and return its ID */
export function seedResumeSkill(db: Database, sectionId: string, skillId: string, position?: number): string {
  const id = testUuid()
  db.run(
    `INSERT INTO resume_skills (id, section_id, skill_id, position)
     VALUES (?, ?, ?, ?)`,
    [id, sectionId, skillId, position ?? 0]
  )
  return id
}

/**
 * Seed a test skill and return its ID. Category defaults to 'language'.
 * Idempotent: if a skill with the same name already exists (e.g., seeded
 * by migration 041), returns the existing row's ID instead of crashing.
 */
export function seedSkill(db: Database, opts: {
  name?: string
  category?: string | null
} = {}): string {
  const name = opts.name ?? 'Python'
  const category = opts.category ?? 'language'

  // Check if skill already exists (migration 041 seeds common names)
  const existing = db
    .query('SELECT id FROM skills WHERE name = ?')
    .get(name) as { id: string } | null
  if (existing) return existing.id

  const id = testUuid()
  db.run(
    `INSERT INTO skills (id, name, category) VALUES (?, ?, ?)`,
    [id, name, category],
  )
  return id
}

/** Seed a test summary and return its ID */
export function seedSummary(db: Database, opts: {
  title?: string
  role?: string | null
  description?: string | null
  isTemplate?: number
  notes?: string | null
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO summaries (id, title, role, description, is_template, notes)
     VALUES (?, ?, ?, ?, ?, ?)`,
    [
      id,
      opts.title ?? 'Test Summary',
      opts.role ?? 'Security Engineer',
      opts.description ?? null,
      opts.isTemplate ?? 0,
      opts.notes ?? null,
    ]
  )
  return id
}

/** Seed a test user note and return its ID */
export function seedUserNote(db: Database, opts: {
  title?: string
  content?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO user_notes (id, title, content)
     VALUES (?, ?, ?)`,
    [
      id,
      opts.title ?? null,
      opts.content ?? 'A test note',
    ]
  )
  return id
}

/** Seed a test domain and return its ID */
export function seedDomain(db: Database, opts: {
  name?: string
  description?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO domains (id, name, description)
     VALUES (?, ?, ?)`,
    [
      id,
      opts.name ?? 'test_domain',
      opts.description ?? null,
    ]
  )
  return id
}

/** Seed a test archetype and return its ID */
export function seedArchetype(db: Database, opts: {
  name?: string
  description?: string
} = {}): string {
  const id = testUuid()
  db.run(
    `INSERT INTO archetypes (id, name, description)
     VALUES (?, ?, ?)`,
    [
      id,
      opts.name ?? 'test-archetype',
      opts.description ?? null,
    ]
  )
  return id
}

/** Link an archetype to a domain in archetype_domains */
export function seedArchetypeDomain(db: Database, archetypeId: string, domainId: string): void {
  db.run(
    `INSERT INTO archetype_domains (archetype_id, domain_id)
     VALUES (?, ?)`,
    [archetypeId, domainId]
  )
}

/** Seed a test job description and return its ID */
export function seedJobDescription(
  db: Database,
  opts: {
    organizationId?: string | null
    title?: string
    url?: string | null
    rawText?: string
    status?: string
    salaryRange?: string | null
    location?: string | null
  } = {},
): string {
  const id = testUuid()
  db.run(
    `INSERT INTO job_descriptions (id, organization_id, title, url, raw_text, status, salary_range, location)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    [
      id,
      opts.organizationId ?? null,
      opts.title ?? 'Senior Security Engineer',
      opts.url ?? null,
      opts.rawText ?? 'We are looking for a senior security engineer to join our team...',
      opts.status ?? 'discovered',
      opts.salaryRange ?? null,
      opts.location ?? null,
    ],
  )
  return id
}

/** Seed a test user profile and return its ID.
 *
 * After migration 046, user_profile no longer has location/linkedin/github/website.
 * Location is via address_id → addresses table; URLs are in profile_urls table.
 */
export function seedProfile(db: Database, opts: {
  name?: string
  email?: string | null
  phone?: string | null
  addressName?: string | null
} = {}): string {
  const id = testUuid()
  // Delete any existing profile (single-row table)
  db.run('DELETE FROM user_profile')

  let addressId: string | null = null
  if (opts.addressName) {
    addressId = testUuid()
    db.run(
      `INSERT INTO addresses (id, name, country_code) VALUES (?, ?, 'US')`,
      [addressId, opts.addressName],
    )
  }

  db.run(
    `INSERT INTO user_profile (id, name, email, phone, address_id)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      opts.name ?? 'Test User',
      opts.email ?? null,
      opts.phone ?? null,
      addressId,
    ]
  )
  return id
}

/** Seed a test resume template and return its ID */
export function seedResumeTemplate(db: Database, opts: {
  name?: string
  description?: string | null
  sections?: Array<{ title: string; entry_type: string; position: number }>
  isBuiltin?: boolean
} = {}): string {
  const id = testUuid()
  const sections = opts.sections ?? [
    { title: 'Summary', entry_type: 'freeform', position: 0 },
    { title: 'Experience', entry_type: 'experience', position: 1 },
    { title: 'Technical Skills', entry_type: 'skills', position: 2 },
  ]
  db.run(
    `INSERT INTO resume_templates (id, name, description, sections, is_builtin)
     VALUES (?, ?, ?, ?, ?)`,
    [
      id,
      opts.name ?? 'Test Template',
      opts.description ?? null,
      JSON.stringify(sections),
      opts.isBuiltin ? 1 : 0,
    ]
  )
  return id
}
