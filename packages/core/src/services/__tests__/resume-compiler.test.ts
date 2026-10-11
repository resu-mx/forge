import { describe, test, expect, beforeEach, afterEach, spyOn } from 'bun:test'
import type { Database } from 'bun:sqlite'
import { compileResumeIR, syntheticUUID, formatDateRange, buildOrgDisplayString, formatClearanceLevel, formatPolygraph } from '../resume-compiler'
import type { EducationItem, ExperienceGroup, ProjectItem, PresentationItem } from '../../types'
import { createTestDb, seedSource, seedBullet, seedPerspective, seedResume, seedResumeEntry, seedResumeSection, seedOrganization, seedResumeSkill, seedProfile, seedSummary } from '../../db/__tests__/helpers'

describe('compileResumeIR', () => {
  let db: Database

  beforeEach(() => {
    db = createTestDb()
  })

  afterEach(() => db.close())

  test('returns null for non-existent resume', () => {
    const result = compileResumeIR(db, 'non-existent-id')
    expect(result).toBeNull()
  })

  test('empty resume produces no sections', () => {
    const resumeId = seedResume(db)
    const result = compileResumeIR(db, resumeId)
    expect(result).not.toBeNull()
    expect(result!.sections).toHaveLength(0)
  })

  test('header uses profile name and resume target_role for tagline', () => {
    // Profile is seeded by migration with name = 'User'
    const resumeId = seedResume(db, { name: 'Adam Smith', targetRole: 'Security Engineer' })
    const result = compileResumeIR(db, resumeId)!
    // name comes from profile (migration-seeded 'User'), not resume
    expect(result.header.name).toBe('User')
    expect(result.header.tagline).toBe('Security Engineer')
    expect(result.header.email).toBeNull()
  })

  test('header tagline parsed from resume header JSON, contact from profile', () => {
    const resumeId = seedResume(db, { name: 'Test Resume' })
    seedProfile(db, {
      name: 'Adam Smith',
      email: 'adam@example.com',
      addressName: 'Reston, VA',
    })
    const headerJson = JSON.stringify({
      tagline: 'Security Engineer',
    })
    db.run('UPDATE resumes SET header = ? WHERE id = ?', [headerJson, resumeId])

    const result = compileResumeIR(db, resumeId)!
    expect(result.header.name).toBe('Adam Smith')       // from profile
    expect(result.header.tagline).toBe('Security Engineer')  // from resume header JSON
    expect(result.header.location).toBe('Reston, VA')    // from profile address.name
    expect(result.header.email).toBe('adam@example.com') // from profile
    // No clearance credentials seeded → null
    expect(result.header.clearance).toBeNull()
  })

  // ── Header clearance one-liner ─────────────────────────────────

  test('header clearance shows one-liner when active clearance credential exists', () => {
    const resumeId = seedResume(db)
    db.run(
      `INSERT INTO credentials (id, credential_type, label, status, details)
       VALUES (?, 'clearance', 'TS/SCI', 'active', ?)`,
      [crypto.randomUUID(), JSON.stringify({
        level: 'top_secret',
        polygraph: 'ci',
        clearance_type: 'personnel',
        access_programs: ['sci'],
      })],
    )

    const result = compileResumeIR(db, resumeId)!
    expect(result.header.clearance).toBe('Active TS/SCI Clearance with CI Poly')
  })

  test('header clearance is null when no active clearance credentials exist', () => {
    const resumeId = seedResume(db)
    // Only an inactive credential — should not appear in header
    db.run(
      `INSERT INTO credentials (id, credential_type, label, status, details)
       VALUES (?, 'clearance', 'Old Secret', 'inactive', ?)`,
      [crypto.randomUUID(), JSON.stringify({
        level: 'secret',
        polygraph: null,
        clearance_type: 'personnel',
        access_programs: [],
      })],
    )

    const result = compileResumeIR(db, resumeId)!
    expect(result.header.clearance).toBeNull()
  })

  test('header clearance picks the highest-level among multiple active clearances', () => {
    const resumeId = seedResume(db)
    // Secret clearance
    db.run(
      `INSERT INTO credentials (id, credential_type, label, status, details)
       VALUES (?, 'clearance', 'Secret', 'active', ?)`,
      [crypto.randomUUID(), JSON.stringify({
        level: 'secret',
        polygraph: null,
        clearance_type: 'personnel',
        access_programs: [],
      })],
    )
    // Top Secret — should win
    db.run(
      `INSERT INTO credentials (id, credential_type, label, status, details)
       VALUES (?, 'clearance', 'TS/SCI', 'active', ?)`,
      [crypto.randomUUID(), JSON.stringify({
        level: 'top_secret',
        polygraph: 'full_scope',
        clearance_type: 'personnel',
        access_programs: ['sci'],
      })],
    )

    const result = compileResumeIR(db, resumeId)!
    expect(result.header.clearance).toBe('Active TS/SCI Clearance with Full-Scope Poly')
  })

  test('header clearance omits polygraph when none/null', () => {
    const resumeId = seedResume(db)
    db.run(
      `INSERT INTO credentials (id, credential_type, label, status, details)
       VALUES (?, 'clearance', 'Secret', 'active', ?)`,
      [crypto.randomUUID(), JSON.stringify({
        level: 'secret',
        polygraph: null,
        clearance_type: 'personnel',
        access_programs: [],
      })],
    )

    const result = compileResumeIR(db, resumeId)!
    expect(result.header.clearance).toBe('Active Secret Clearance')
    expect(result.header.clearance).not.toContain('with')
  })

  test('header clearance ignores non-clearance credentials', () => {
    const resumeId = seedResume(db)
    db.run(
      `INSERT INTO credentials (id, credential_type, label, status, details)
       VALUES (?, 'drivers_license', 'VA CDL', 'active', ?)`,
      [crypto.randomUUID(), JSON.stringify({ class: 'A', state: 'VA', endorsements: [] })],
    )

    const result = compileResumeIR(db, resumeId)!
    expect(result.header.clearance).toBeNull()
  })

  test('freeform section renders entries', () => {
    const resumeId = seedResume(db)
    const secId = seedResumeSection(db, resumeId, 'Summary', 'freeform', 0)
    seedResumeEntry(db, secId, { content: 'Security engineer with 14+ years experience', position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const summarySection = result.sections.find(s => s.type === 'freeform')
    expect(summarySection).toBeDefined()
    expect(summarySection!.title).toBe('Summary')
    expect(summarySection!.items).toHaveLength(1)
    const item = summarySection!.items[0]
    expect(item.kind).toBe('summary')
    if (item.kind === 'summary') {
      expect(item.content).toBe('Security engineer with 14+ years experience')
    }
  })

  test('experience section groups entries by organization', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Raytheon' })

    // Create a role source at Raytheon
    const sourceId = seedSource(db, { title: 'Principal Cloud Forensics Engineer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, end_date, is_current)
       VALUES (?, ?, ?, ?, ?)`,
      [sourceId, orgId, '2024-03-01', '2025-07-01', 0]
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'Built cloud platform' })
    const perspId = seedPerspective(db, bulletId, { content: 'Architected cloud forensics platform' })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')
    expect(expSection).toBeDefined()
    expect(expSection!.items).toHaveLength(1)
    const group = expSection!.items[0]
    expect(group.kind).toBe('experience_group')
    if (group.kind === 'experience_group') {
      expect(group.organization).toBe('Raytheon')
      expect(group.subheadings).toHaveLength(1)
      expect(group.subheadings[0].title).toBe('Principal Cloud Forensics Engineer')
      expect(group.subheadings[0].bullets).toHaveLength(1)
    }
  })

  test('experience groups unlinked roles under "⚠ Unlinked Sources" label', () => {
    const resumeId = seedResume(db)
    // Create two role sources WITHOUT organization_id
    const src1 = seedSource(db, { title: 'Cloud Engineer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, start_date, is_current, work_arrangement)
       VALUES (?, ?, ?, ?)`,
      [src1, '2024-01-01', 1, 'remote']
    )
    const src2 = seedSource(db, { title: 'DevOps Lead', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, start_date, is_current, work_arrangement)
       VALUES (?, ?, ?, ?)`,
      [src2, '2023-01-01', 0, 'onsite']
    )

    const b1 = seedBullet(db, [{ id: src1, isPrimary: true }], { content: 'Cloud work' })
    const p1 = seedPerspective(db, b1, { content: 'Architected cloud platform' })
    const b2 = seedBullet(db, [{ id: src2, isPrimary: true }], { content: 'DevOps work' })
    const p2 = seedPerspective(db, b2, { content: 'Led DevOps transformation' })

    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: p1, position: 0 })
    seedResumeEntry(db, secId, { perspectiveId: p2, position: 1 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!

    // Both unlinked roles collapse into one group (not per-source)
    expect(expSection.items).toHaveLength(1)
    const group = expSection.items[0] as ExperienceGroup
    expect(group.kind).toBe('experience_group')

    // Group label should be the actionable "⚠ Unlinked Sources", not "Other (Remote)"
    expect(group.organization).toBe('⚠ Unlinked Sources')
    expect(group.organization).not.toContain('Other')
    expect(group.organization).not.toContain('Remote')

    // Both roles still appear as subheadings within the group
    expect(group.subheadings).toHaveLength(2)
    const titles = group.subheadings.map(s => s.title).sort()
    expect(titles).toEqual(['Cloud Engineer', 'DevOps Lead'])
  })

  test('experience groups unlinked roles separately from linked orgs', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Raytheon' })

    // Linked role (has organization_id)
    const linkedSrc = seedSource(db, { title: 'Security Analyst', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current)
       VALUES (?, ?, ?, ?)`,
      [linkedSrc, orgId, '2023-01-01', 0]
    )
    // Unlinked role (no organization_id)
    const unlinkedSrc = seedSource(db, { title: 'Freelance Consultant', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, start_date, is_current, work_arrangement)
       VALUES (?, ?, ?, ?)`,
      [unlinkedSrc, '2024-01-01', 1, 'remote']
    )

    const b1 = seedBullet(db, [{ id: linkedSrc, isPrimary: true }])
    const p1 = seedPerspective(db, b1)
    const b2 = seedBullet(db, [{ id: unlinkedSrc, isPrimary: true }])
    const p2 = seedPerspective(db, b2)

    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: p1, position: 0 })
    seedResumeEntry(db, secId, { perspectiveId: p2, position: 1 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!

    // Should produce 2 groups: Raytheon + Unlinked
    expect(expSection.items).toHaveLength(2)
    const orgNames = expSection.items.map(i => (i as ExperienceGroup).organization).sort()
    expect(orgNames).toEqual(['Raytheon', '⚠ Unlinked Sources'])
  })

  test('experience uses entry_content when cloned', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'ACME' })
    const sourceId = seedSource(db, { title: 'Engineer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current)
       VALUES (?, ?, ?, ?)`,
      [sourceId, orgId, '2024-01-01', 1]
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }])
    const perspId = seedPerspective(db, bulletId, { content: 'Original perspective' })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0, content: 'Cloned content' })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    const group = expSection.items[0]
    if (group.kind === 'experience_group') {
      expect(group.subheadings[0].bullets[0].content).toBe('Cloned content')
      expect(group.subheadings[0].bullets[0].is_cloned).toBe(true)
    }
  })

  test('skills section reads from resume_skills', () => {
    const resumeId = seedResume(db)
    const secId = seedResumeSection(db, resumeId, 'Technical Skills', 'skills', 0)

    // Create skills and link them via resume_skills
    const skillId1 = crypto.randomUUID()
    const skillId2 = crypto.randomUUID()
    db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [skillId1, 'Python', 'language'])
    db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [skillId2, 'Kubernetes', 'methodology'])
    seedResumeSkill(db, secId, skillId1, 0)
    seedResumeSkill(db, secId, skillId2, 1)

    const result = compileResumeIR(db, resumeId)!
    const skillsSection = result.sections.find(s => s.type === 'skills')
    expect(skillsSection).toBeDefined()
    expect(skillsSection!.items).toHaveLength(1)
    const group = skillsSection!.items[0]
    if (group.kind === 'skill_group') {
      expect(group.categories.length).toBeGreaterThanOrEqual(1)
      const langs = group.categories.find(c => c.label === 'Languages')
      expect(langs).toBeDefined()
      expect(langs!.skills).toContain('Python')
      const devops = group.categories.find(c => c.label === 'Methodology')
      expect(devops).toBeDefined()
      expect(devops!.skills).toContain('Kubernetes')
    }
  })

  test('education section populates from source_education', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Western Governors University' })
    const sourceId = seedSource(db, { title: 'WGU Degree', sourceType: 'education' })
    db.run(
      `INSERT INTO source_education (source_id, education_type, organization_id, field, end_date)
       VALUES (?, ?, ?, ?, ?)`,
      [sourceId, 'degree', orgId, 'Cybersecurity', '2023-06-01']
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'B.S. Cybersecurity' })
    const perspId = seedPerspective(db, bulletId, { content: 'B.S. Cybersecurity & Information Assurance' })
    const secId = seedResumeSection(db, resumeId, 'Education', 'education', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const eduSection = result.sections.find(s => s.type === 'education')
    expect(eduSection).toBeDefined()
    const item = eduSection!.items[0]
    if (item.kind === 'education') {
      expect(item.institution).toBe('Western Governors University')
      expect(item.degree).toBe('B.S. Cybersecurity & Information Assurance')
      expect(item.date).toBe('2023')
    }
  })

  test('education section falls back to source.title when content is null (direct-source reference)', () => {
    // Post-T95.5 fix: SourcePicker adds direct-source entries with no
    // content (pure reference mode). The compiler's degree fallback chain
    // must include source.title so the entry renders with the canonical
    // source name ("Cloud Security") instead of an empty string.
    const resumeId = seedResume(db)
    const sourceId = seedSource(db, {
      title: 'Cloud Security',
      sourceType: 'education',
      description: 'GIAC certifications included', // must NOT be used
    })
    db.run(
      `INSERT INTO source_education (source_id, education_type, end_date, degree_level)
       VALUES (?, ?, ?, ?)`,
      [sourceId, 'degree', '2025-10-01', 'graduate_certificate']
    )

    const secId = seedResumeSection(db, resumeId, 'Education', 'education', 0)
    // No content, no perspective — pure direct-source reference
    seedResumeEntry(db, secId, { sourceId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const eduSection = result.sections.find(s => s.type === 'education')
    expect(eduSection).toBeDefined()
    expect(eduSection!.items).toHaveLength(1)
    const item = eduSection!.items[0] as EducationItem
    expect(item.kind).toBe('education')
    // Degree text must come from source.title, NOT source.description
    expect(item.degree).toBe('Cloud Security')
    expect(item.degree).not.toContain('GIAC certifications included')
    expect(item.degree_level).toBe('graduate_certificate')
    expect(item.date).toBe('2025')
    expect(item.source_id).toBe(sourceId)
  })

  test('certification section reads from resume_certifications junction', () => {
    // After cert-rework: certifications are per-resume via the
    // resume_certifications junction table. The compiler must only render
    // certs that are explicitly pinned to this resume section.
    const resumeId = seedResume(db)
    const certId = crypto.randomUUID()
    const isc2OrgId = crypto.randomUUID()
    db.run(`INSERT INTO organizations (id, name, org_type) VALUES (?, 'ISC2', 'company')`, [isc2OrgId])
    db.run(
      `INSERT INTO certifications (id, short_name, long_name, issuer_id, date_earned, in_progress)
       VALUES (?, 'CISSP', 'Certified Information Systems Security Professional', ?, '2024-06-01', 0)`,
      [certId, isc2OrgId],
    )

    const secId = seedResumeSection(db, resumeId, 'Certifications', 'certifications', 0)
    const rcId = crypto.randomUUID()
    db.run(
      `INSERT INTO resume_certifications (id, resume_id, certification_id, section_id, position)
       VALUES (?, ?, ?, ?, ?)`,
      [rcId, resumeId, certId, secId, 0],
    )

    const result = compileResumeIR(db, resumeId)!
    const certSection = result.sections.find(s => s.type === 'certifications')
    expect(certSection).toBeDefined()
    expect(certSection!.items).toHaveLength(1)

    const group = certSection!.items[0]
    expect(group.kind).toBe('certification_group')
    if (group.kind === 'certification_group') {
      // Grouped by issuer
      expect(group.categories[0].label).toBe('ISC2')
      // Name is just short_name (no year/credential decoration at compile time)
      expect(group.categories[0].certs[0].name).toBe('CISSP')
      expect(group.categories[0].certs[0].entry_id).toBe(rcId)
      expect(group.categories[0].certs[0].source_id).toBeNull()
      expect(group.categories[0].certs[0].in_progress).toBe(false)
    }
  })

  test('an in-progress certification is flagged and keeps its plain name', () => {
    const resumeId = seedResume(db)
    const orgId = crypto.randomUUID()
    db.run(`INSERT INTO organizations (id, name, org_type) VALUES (?, 'ISC2', 'company')`, [orgId])
    const secId = seedResumeSection(db, resumeId, 'Certifications', 'certifications', 0)
    const pin = (short: string, inProgress: number, position: number) => {
      const certId = crypto.randomUUID()
      db.run(
        `INSERT INTO certifications (id, short_name, long_name, issuer_id, in_progress)
         VALUES (?, ?, ?, ?, ?)`,
        [certId, short, short, orgId, inProgress],
      )
      db.run(
        `INSERT INTO resume_certifications (id, resume_id, certification_id, section_id, position)
         VALUES (?, ?, ?, ?, ?)`,
        [crypto.randomUUID(), resumeId, certId, secId, position],
      )
    }
    pin('CISSP', 0, 0)
    pin('CCSP', 1, 1)

    const result = compileResumeIR(db, resumeId)!
    const group = result.sections.find(s => s.type === 'certifications')!.items[0]
    expect(group.kind).toBe('certification_group')
    if (group.kind === 'certification_group') {
      const certs = group.categories[0].certs
      expect(certs.map(c => [c.name, c.in_progress])).toEqual([
        ['CISSP', false],
        ['CCSP', true],
      ])
    }
  })

  test('education section sorts entries by end_date descending (most recent first)', () => {
    const resumeId = seedResume(db)
    const oldSourceId = seedSource(db, {
      title: 'Old Degree',
      sourceType: 'education',
    })
    const midSourceId = seedSource(db, {
      title: 'Mid Degree',
      sourceType: 'education',
    })
    const newSourceId = seedSource(db, {
      title: 'New Degree',
      sourceType: 'education',
    })
    const inProgressSourceId = seedSource(db, {
      title: 'In-Progress Degree',
      sourceType: 'education',
    })
    db.run(
      `INSERT INTO source_education (source_id, education_type, end_date)
       VALUES (?, 'degree', '2018-06-01'),
              (?, 'degree', '2022-06-01'),
              (?, 'degree', '2025-06-01'),
              (?, 'degree', NULL)`,
      [oldSourceId, midSourceId, newSourceId, inProgressSourceId]
    )

    const secId = seedResumeSection(db, resumeId, 'Education', 'education', 0)
    // Insert in a shuffled order by position to prove end_date sort wins
    seedResumeEntry(db, secId, { sourceId: midSourceId, position: 0 })
    seedResumeEntry(db, secId, { sourceId: newSourceId, position: 1 })
    seedResumeEntry(db, secId, { sourceId: inProgressSourceId, position: 2 })
    seedResumeEntry(db, secId, { sourceId: oldSourceId, position: 3 })

    const result = compileResumeIR(db, resumeId)!
    const eduSection = result.sections.find(s => s.type === 'education')
    expect(eduSection).toBeDefined()
    expect(eduSection!.items).toHaveLength(4)

    // Expected order: in-progress (null end_date) first, then 2025 → 2022 → 2018
    const titles = eduSection!.items.map(i => (i as EducationItem).degree)
    expect(titles).toEqual(['In-Progress Degree', 'New Degree', 'Mid Degree', 'Old Degree'])
  })

  test('education section renders direct-source entries (no perspective chain)', () => {
    // Regression test for T95.5-class bug: education/certification/clearance
    // entries added via SourcePicker's content-only path have
    // perspective_id=NULL and source_id set directly. The compiler must
    // LEFT JOIN perspectives + COALESCE the source chain so these still
    // render with full structured data from source_education.
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Western Governors University' })
    const sourceId = seedSource(db, {
      title: 'WGU Degree',
      sourceType: 'education',
      description: 'B.S. Cybersecurity & Information Assurance',
    })
    db.run(
      `INSERT INTO source_education (source_id, education_type, organization_id, field, end_date, degree_level, degree_type, gpa)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
      [sourceId, 'degree', orgId, 'Cybersecurity', '2023-06-01', 'bachelors', 'bs', '3.8']
    )

    // NO bullet, NO perspective — the entry points directly at the source.
    const secId = seedResumeSection(db, resumeId, 'Education', 'education', 0)
    seedResumeEntry(db, secId, {
      sourceId,
      content: 'B.S. Cybersecurity & Information Assurance',
      position: 0,
    })

    const result = compileResumeIR(db, resumeId)!
    const eduSection = result.sections.find(s => s.type === 'education')
    expect(eduSection).toBeDefined()
    expect(eduSection!.items).toHaveLength(1)
    const item = eduSection!.items[0] as EducationItem
    expect(item.kind).toBe('education')
    expect(item.institution).toBe('Western Governors University')
    expect(item.degree).toBe('B.S. Cybersecurity & Information Assurance')
    expect(item.date).toBe('2023')
    expect(item.degree_level).toBe('bachelors')
    expect(item.degree_type).toBe('bs')
    expect(item.field).toBe('Cybersecurity')
    expect(item.gpa).toBe('3.8')
    expect(item.source_id).toBe(sourceId)
  })

  test('certification section groups by issuer and handles missing issuer', () => {
    const resumeId = seedResume(db)
    // Cert with issuer org
    const awsOrgId = crypto.randomUUID()
    db.run(`INSERT INTO organizations (id, name, org_type) VALUES (?, 'Amazon Web Services', 'company')`, [awsOrgId])
    const awsCertId = crypto.randomUUID()
    db.run(
      `INSERT INTO certifications (id, short_name, long_name, issuer_id, date_earned, in_progress)
       VALUES (?, 'AWS SA Pro', 'AWS Solutions Architect Professional', ?, '2024-08-15', 0)`,
      [awsCertId, awsOrgId],
    )
    // Cert without issuer
    const noIssuerCertId = crypto.randomUUID()
    db.run(
      `INSERT INTO certifications (id, short_name, long_name, in_progress)
       VALUES (?, 'Self-Study Badge', 'Self-Study Badge', 0)`,
      [noIssuerCertId],
    )

    const secId = seedResumeSection(db, resumeId, 'Certifications', 'certifications', 0)
    // Pin both certs to this resume section
    db.run(
      `INSERT INTO resume_certifications (id, resume_id, certification_id, section_id, position) VALUES (?, ?, ?, ?, ?)`,
      [crypto.randomUUID(), resumeId, awsCertId, secId, 0],
    )
    db.run(
      `INSERT INTO resume_certifications (id, resume_id, certification_id, section_id, position) VALUES (?, ?, ?, ?, ?)`,
      [crypto.randomUUID(), resumeId, noIssuerCertId, secId, 1],
    )

    const result = compileResumeIR(db, resumeId)!
    const certSection = result.sections.find(s => s.type === 'certifications')
    expect(certSection).toBeDefined()
    const group = certSection!.items[0]
    expect(group.kind).toBe('certification_group')
    if (group.kind === 'certification_group') {
      const labels = group.categories.map(c => c.label).sort()
      expect(labels).toContain('Amazon Web Services')
      expect(labels).toContain('Other') // null issuer falls back to 'Other'
    }
  })

  test('certification section returns empty items when no junction rows exist', () => {
    // Section exists but no resume_certifications rows — should produce 0 items.
    const resumeId = seedResume(db)
    // Cert exists globally but NOT pinned to this resume
    db.run(
      `INSERT INTO certifications (id, short_name, long_name, in_progress) VALUES (?, 'Security+', 'CompTIA Security+', 0)`,
      [crypto.randomUUID()],
    )
    seedResumeSection(db, resumeId, 'Certifications', 'certifications', 0)

    const result = compileResumeIR(db, resumeId)!
    const certSection = result.sections.find(s => s.type === 'certifications')
    expect(certSection).toBeDefined()
    expect(certSection!.items).toHaveLength(0)
  })

  test('certification junction — only pinned certs appear (isolation)', () => {
    // Two resumes; cert is pinned only to resume1. resume2 must see 0 items.
    const resume1Id = seedResume(db)
    const resume2Id = seedResume(db)
    const certId = crypto.randomUUID()
    const pmiOrgId = crypto.randomUUID()
    db.run(`INSERT INTO organizations (id, name, org_type) VALUES (?, 'PMI', 'company')`, [pmiOrgId])
    db.run(
      `INSERT INTO certifications (id, short_name, long_name, issuer_id, in_progress)
       VALUES (?, 'PMP', 'Project Management Professional', ?, 0)`,
      [certId, pmiOrgId],
    )

    const sec1Id = seedResumeSection(db, resume1Id, 'Certifications', 'certifications', 0)
    seedResumeSection(db, resume2Id, 'Certifications', 'certifications', 0)

    // Pin only to resume1
    db.run(
      `INSERT INTO resume_certifications (id, resume_id, certification_id, section_id, position) VALUES (?, ?, ?, ?, ?)`,
      [crypto.randomUUID(), resume1Id, certId, sec1Id, 0],
    )

    const r1 = compileResumeIR(db, resume1Id)!
    const r2 = compileResumeIR(db, resume2Id)!

    const cert1 = r1.sections.find(s => s.type === 'certifications')!
    const cert2 = r2.sections.find(s => s.type === 'certifications')!

    expect(cert1.items).toHaveLength(1)
    if (cert1.items[0].kind === 'certification_group') {
      expect(cert1.items[0].categories[0].certs[0].name).toBe('PMP')
    }
    expect(cert2.items).toHaveLength(0)
  })

  test('clearance section renders credentials (Phase 84)', () => {
    // Migration 037 moved clearances out of sources and into the
    // credentials entity. A clearance section now renders every
    // credential with credential_type='clearance' — no resume_entries
    // row required. The test seeds a credential directly.
    const resumeId = seedResume(db)
    const credId = crypto.randomUUID()
    db.run(
      `INSERT INTO credentials (id, credential_type, label, status, details)
       VALUES (?, 'clearance', ?, 'active', ?)`,
      [
        credId,
        'Top Secret / SCI',
        JSON.stringify({
          level: 'top_secret',
          polygraph: 'ci',
          clearance_type: 'personnel',
          access_programs: ['sci'],
        }),
      ],
    )

    // The section still needs to exist on the resume for the IR to
    // include it. Entries are optional for clearance.
    seedResumeSection(db, resumeId, 'Security Clearance', 'clearance', 0)

    const result = compileResumeIR(db, resumeId)!
    const clearanceSection = result.sections.find(s => s.type === 'clearance')
    expect(clearanceSection).toBeDefined()
    expect(clearanceSection!.items.length).toBeGreaterThanOrEqual(1)
    const item = clearanceSection!.items.find(i => i.kind === 'clearance' && (i as any).entry_id === credId)
    expect(item).toBeDefined()
    if (item && item.kind === 'clearance') {
      // The credential has a user-authored label, so that wins over the
      // synthesized formatter output.
      expect(item.content).toBe('Top Secret / SCI')
      // No source chain — credentials aren't sources.
      expect(item.source_id).toBeNull()
    }
  })

  test('education section includes campus fields when campus_id is set', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Western Governors University' })

    // Create a location with address
    const addrId = crypto.randomUUID()
    db.run(
      `INSERT INTO addresses (id, name, city, state) VALUES (?, ?, ?, ?)`,
      [addrId, 'Salt Lake City Campus', 'Salt Lake City', 'UT']
    )
    const campusId = crypto.randomUUID()
    db.run(
      `INSERT INTO org_locations (id, organization_id, name, modality, address_id)
       VALUES (?, ?, ?, ?, ?)`,
      [campusId, orgId, 'Salt Lake City Campus', 'in_person', addrId]
    )

    const sourceId = seedSource(db, { title: 'WGU Degree', sourceType: 'education' })
    db.run(
      `INSERT INTO source_education (source_id, education_type, organization_id, campus_id, field, end_date)
       VALUES (?, ?, ?, ?, ?, ?)`,
      [sourceId, 'degree', orgId, campusId, 'Cybersecurity', '2023-06-01']
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'B.S. Cybersecurity' })
    const perspId = seedPerspective(db, bulletId, { content: 'B.S. Cybersecurity & Information Assurance' })
    const secId = seedResumeSection(db, resumeId, 'Education', 'education', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const eduSection = result.sections.find(s => s.type === 'education')
    expect(eduSection).toBeDefined()
    const item = eduSection!.items[0] as EducationItem
    expect(item.kind).toBe('education')
    expect(item.institution).toBe('Western Governors University')
    expect(item.campus_name).toBe('Salt Lake City Campus')
    expect(item.campus_city).toBe('Salt Lake City')
    expect(item.campus_state).toBe('UT')
  })

  test('education section has null campus fields when no campus_id', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Western Governors University' })
    const sourceId = seedSource(db, { title: 'WGU Degree', sourceType: 'education' })
    db.run(
      `INSERT INTO source_education (source_id, education_type, organization_id, field, end_date)
       VALUES (?, ?, ?, ?, ?)`,
      [sourceId, 'degree', orgId, 'Cybersecurity', '2023-06-01']
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'B.S. Cybersecurity' })
    const perspId = seedPerspective(db, bulletId, { content: 'B.S. Cybersecurity & Information Assurance' })
    const secId = seedResumeSection(db, resumeId, 'Education', 'education', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const eduSection = result.sections.find(s => s.type === 'education')
    expect(eduSection).toBeDefined()
    const item = eduSection!.items[0] as EducationItem
    expect(item.kind).toBe('education')
    expect(item.campus_name).toBeNull()
    expect(item.campus_city).toBeNull()
    expect(item.campus_state).toBeNull()
  })

  test('clearance section falls back to structured details when label is empty', () => {
    // When a credential has no label, the compiler synthesizes one from
    // the structured details JSON using the human-readable formatters
    // (formatClearanceLevel, formatPolygraph). An inactive status gets
    // an "(Inactive)" suffix so the exceptional case is visible.
    const resumeId = seedResume(db)
    const credId = crypto.randomUUID()
    db.run(
      `INSERT INTO credentials (id, credential_type, label, status, details)
       VALUES (?, 'clearance', '', 'inactive', ?)`,
      [
        credId,
        JSON.stringify({
          level: 'top_secret',
          polygraph: 'ci',
          clearance_type: 'personnel',
          access_programs: [],
        }),
      ],
    )

    seedResumeSection(db, resumeId, 'Security Clearance', 'clearance', 0)

    const result = compileResumeIR(db, resumeId)!
    const clrSection = result.sections.find(s => s.type === 'clearance')
    expect(clrSection).toBeDefined()
    const item = clrSection!.items.find(i => i.kind === 'clearance' && (i as any).entry_id === credId)
    expect(item).toBeDefined()
    if (item && item.kind === 'clearance') {
      expect(item.content).toBe('TS/SCI with CI Poly (Inactive)')
    }
  })

  test('projects section groups by source title', () => {
    const resumeId = seedResume(db)
    const sourceId = seedSource(db, { title: 'AI Memory Architecture', sourceType: 'project' })
    db.run(
      `INSERT INTO source_projects (source_id, is_personal, end_date) VALUES (?, ?, ?)`,
      [sourceId, 1, '2024-06-01']
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'Built memory system' })
    const perspId = seedPerspective(db, bulletId, { content: 'Designed graph-based memory system' })
    const secId = seedResumeSection(db, resumeId, 'Projects', 'projects', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const projSection = result.sections.find(s => s.type === 'projects')
    expect(projSection).toBeDefined()
    const item = projSection!.items[0]
    if (item.kind === 'project') {
      expect(item.name).toBe('AI Memory Architecture')
      expect(item.bullets).toHaveLength(1)
    }
  })

  test('source_chain includes source_title, bullet_preview, perspective_preview', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Raytheon' })
    const sourceId = seedSource(db, { title: 'Principal Cloud Forensics Engineer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current)
       VALUES (?, ?, ?, ?)`,
      [sourceId, orgId, '2024-01-01', 1]
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'Built cloud platform' })
    const perspId = seedPerspective(db, bulletId, { content: 'Architected cloud forensics platform' })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    const group = expSection.items[0] as ExperienceGroup
    const bullet = group.subheadings[0].bullets[0]

    expect(bullet.source_chain).toBeDefined()
    expect(bullet.source_chain!.source_title).toBeDefined()
    expect(bullet.source_chain!.source_title.length).toBeGreaterThan(0)
    expect(bullet.source_chain!.bullet_preview).toBeDefined()
    expect(bullet.source_chain!.bullet_preview.length).toBeGreaterThan(0)
    expect(bullet.source_chain!.perspective_preview).toBeDefined()
    expect(bullet.source_chain!.perspective_preview.length).toBeGreaterThan(0)
  })

  test('bullet_preview uses original bullet content, perspective_preview uses reframing', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'TestCorp' })
    const sourceId = seedSource(db, { title: 'Engineer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current)
       VALUES (?, ?, ?, ?)`,
      [sourceId, orgId, '2024-01-01', 1]
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], {
      content: 'Original bullet about cloud migration infrastructure and tooling'
    })
    const perspId = seedPerspective(db, bulletId, {
      content: 'Led cloud platform migration initiative across multiple regions'
    })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    const group = expSection.items[0] as ExperienceGroup
    const bullet = group.subheadings[0].bullets[0]

    expect(bullet.source_chain!.bullet_preview).not.toEqual(
      bullet.source_chain!.perspective_preview
    )
    expect(bullet.source_chain!.bullet_preview).toContain('Original bullet')
    expect(bullet.source_chain!.perspective_preview).toContain('Led cloud')
  })

  test('source_title truncated to 60 characters for long titles', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'BigCorp' })
    const longTitle = 'Principal Cloud Forensics Engineer and Platform Security Lead for Enterprise Division'
    const sourceId = seedSource(db, { title: longTitle, sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current)
       VALUES (?, ?, ?, ?)`,
      [sourceId, orgId, '2024-01-01', 1]
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'Did stuff' })
    const perspId = seedPerspective(db, bulletId, { content: 'Accomplished things' })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    const group = expSection.items[0] as ExperienceGroup
    const bullet = group.subheadings[0].bullets[0]

    expect(bullet.source_chain!.source_title.length).toBeLessThanOrEqual(63) // 60 + '...'
    expect(bullet.source_chain!.source_title).toEndWith('...')
  })

  test('buildProjectsSection enriches source_chain with previews', () => {
    const resumeId = seedResume(db)
    const sourceId = seedSource(db, { title: 'AI Memory Architecture', sourceType: 'project' })
    db.run(
      `INSERT INTO source_projects (source_id, is_personal, end_date) VALUES (?, ?, ?)`,
      [sourceId, 1, '2024-06-01']
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], {
      content: 'Built graph-based memory system for AI agents'
    })
    const perspId = seedPerspective(db, bulletId, {
      content: 'Designed hierarchical memory architecture enabling long-term agent recall'
    })
    const secId = seedResumeSection(db, resumeId, 'Projects', 'projects', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const projSection = result.sections.find(s => s.type === 'projects')!
    const item = projSection.items[0] as ProjectItem
    const bullet = item.bullets[0]

    expect(bullet.source_chain).toBeDefined()
    expect(bullet.source_chain!.source_title).toBe('AI Memory Architecture')
    expect(bullet.source_chain!.bullet_preview).toContain('Built graph-based')
    expect(bullet.source_chain!.perspective_preview).toContain('Designed hierarchical')
  })

  test('buildPresentationsSection enriches source_chain with previews', () => {
    const resumeId = seedResume(db)
    const sourceId = seedSource(db, { title: 'Cloud Forensics at Scale', sourceType: 'general' })

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], {
      content: 'Presented cloud forensics findings at RSA Conference'
    })
    const perspId = seedPerspective(db, bulletId, {
      content: 'Delivered keynote on cloud-native forensics methodologies'
    })
    const secId = seedResumeSection(db, resumeId, 'Presentations', 'presentations', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const presSection = result.sections.find(s => s.type === 'presentations')!
    const item = presSection.items[0] as PresentationItem
    const bullet = item.bullets[0]

    expect(bullet.source_chain).toBeDefined()
    expect(bullet.source_chain!.source_title).toBe('Cloud Forensics at Scale')
    expect(bullet.source_chain!.bullet_preview).toContain('Presented cloud')
    expect(bullet.source_chain!.perspective_preview).toContain('Delivered keynote')
  })

  test('section IDs come from resume_sections table', () => {
    const resumeId = seedResume(db)
    const secId = seedResumeSection(db, resumeId, 'Summary', 'freeform', 0)
    seedResumeEntry(db, secId, { content: 'Summary text', position: 0 })

    const result = compileResumeIR(db, resumeId)!
    expect(result.sections[0].id).toBe(secId)
  })

  test('section titles come from resume_sections table', () => {
    const resumeId = seedResume(db)
    const secId = seedResumeSection(db, resumeId, 'My Custom Title', 'freeform', 0)
    seedResumeEntry(db, secId, { content: 'Text', position: 0 })

    const result = compileResumeIR(db, resumeId)!
    expect(result.sections[0].title).toBe('My Custom Title')
  })

  test('empty sections appear in IR with no items', () => {
    const resumeId = seedResume(db)
    seedResumeSection(db, resumeId, 'Empty Section', 'experience', 0)

    const ir = compileResumeIR(db, resumeId)!
    expect(ir.sections).toHaveLength(1)
    expect(ir.sections[0].title).toBe('Empty Section')
    expect(ir.sections[0].items).toHaveLength(0)
  })

  test('two sections of same entry_type have entries in correct section', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'CivilianCorp' })
    const orgId2 = seedOrganization(db, { name: 'MilOrg' })
    const s1 = seedResumeSection(db, resumeId, 'Civilian Work', 'experience', 0)
    const s2 = seedResumeSection(db, resumeId, 'Military Service', 'experience', 1)

    // Seed entries into each section
    const sourceId1 = seedSource(db, { title: 'Civilian Role', sourceType: 'role' })
    db.run('INSERT INTO source_roles (source_id, organization_id, start_date, is_current) VALUES (?, ?, ?, ?)',
      [sourceId1, orgId, '2024-01-01', 1])
    const bulletId1 = seedBullet(db, [{ id: sourceId1, isPrimary: true }], { content: 'Civilian work bullet' })
    const p1 = seedPerspective(db, bulletId1, { content: 'Civilian perspective' })

    const sourceId2 = seedSource(db, { title: 'Military Role', sourceType: 'role' })
    db.run('INSERT INTO source_roles (source_id, organization_id, start_date, is_current) VALUES (?, ?, ?, ?)',
      [sourceId2, orgId2, '2020-01-01', 0])
    const bulletId2 = seedBullet(db, [{ id: sourceId2, isPrimary: true }], { content: 'Military work bullet' })
    const p2 = seedPerspective(db, bulletId2, { content: 'Military perspective' })

    seedResumeEntry(db, s1, { perspectiveId: p1, position: 0 })
    seedResumeEntry(db, s2, { perspectiveId: p2, position: 0 })

    const ir = compileResumeIR(db, resumeId)!
    expect(ir.sections).toHaveLength(2)
    expect(ir.sections[0].title).toBe('Civilian Work')
    expect(ir.sections[1].title).toBe('Military Service')
    expect(ir.sections[0].items).toHaveLength(1)
    expect(ir.sections[1].items).toHaveLength(1)
  })

  // ── parseHeader with profile ────────────────────────────────────────

  test('uses profile contact fields instead of resume header', () => {
    const resumeId = seedResume(db, { name: 'Resume Name' })
    seedProfile(db, {
      name: 'Adam',
      email: 'adam@test.com',
      phone: '+1-555-0123',
      addressName: 'DC',
    })
    // Set resume header JSON with a tagline
    db.run("UPDATE resumes SET header = ? WHERE id = ?", [
      JSON.stringify({ tagline: 'Security Engineer' }),
      resumeId,
    ])

    const ir = compileResumeIR(db, resumeId)
    expect(ir).not.toBeNull()
    expect(ir!.header.name).toBe('Adam')  // from profile
    expect(ir!.header.email).toBe('adam@test.com')  // from profile
    expect(ir!.header.phone).toBe('+1-555-0123')  // from profile
    expect(ir!.header.location).toBe('DC')  // from profile address.name
    expect(ir!.header.tagline).toBe('Security Engineer')  // from resume header
  })

  test('falls back to resume name when profile is missing', () => {
    const resumeId = seedResume(db, { name: 'Fallback Resume' })
    db.run('DELETE FROM user_profile')

    const ir = compileResumeIR(db, resumeId)
    expect(ir).not.toBeNull()
    expect(ir!.header.name).toBe('Fallback Resume')
  })

  test('tagline defaults to target_role when header JSON has no tagline', () => {
    const resumeId = seedResume(db, { targetRole: 'AI Engineer' })
    seedProfile(db, { name: 'Adam' })

    const ir = compileResumeIR(db, resumeId)
    expect(ir!.header.tagline).toBe('AI Engineer')
  })

  test('contact info changes when profile is updated', () => {
    const resumeId = seedResume(db)
    seedProfile(db, { name: 'Before', email: 'old@test.com' })

    const before = compileResumeIR(db, resumeId)
    expect(before!.header.email).toBe('old@test.com')

    // Update profile
    db.run("UPDATE user_profile SET email = 'new@test.com'")

    const after = compileResumeIR(db, resumeId)
    expect(after!.header.email).toBe('new@test.com')
  })

  // ── Phase 92: resume-level tagline (tagline_override + generated_tagline) ─

  test('compileResumeIR uses tagline_override when set', () => {
    const resumeId = seedResume(db, { targetRole: 'Fallback Role' })
    db.run('UPDATE resumes SET tagline_override = ? WHERE id = ?', ['Override Tagline', resumeId])

    const ir = compileResumeIR(db, resumeId)
    expect(ir!.header.tagline).toBe('Override Tagline')
  })

  test('compileResumeIR uses generated_tagline when no override set', () => {
    const resumeId = seedResume(db, { targetRole: 'Fallback Role' })
    db.run('UPDATE resumes SET generated_tagline = ? WHERE id = ?', ['Generated Tagline', resumeId])

    const ir = compileResumeIR(db, resumeId)
    expect(ir!.header.tagline).toBe('Generated Tagline')
  })

  test('compileResumeIR prefers tagline_override over generated_tagline', () => {
    const resumeId = seedResume(db)
    db.run(
      'UPDATE resumes SET tagline_override = ?, generated_tagline = ? WHERE id = ?',
      ['Override Wins', 'Generated', resumeId],
    )

    const ir = compileResumeIR(db, resumeId)
    expect(ir!.header.tagline).toBe('Override Wins')
  })

  test('compileResumeIR falls back to target_role when no tagline set', () => {
    const resumeId = seedResume(db, { targetRole: 'Fallback Role' })

    const ir = compileResumeIR(db, resumeId)
    expect(ir!.header.tagline).toBe('Fallback Role')
  })

  test('buildSummary returns null when resume has no summary_id and no override', () => {
    const resumeId = seedResume(db)
    const result = compileResumeIR(db, resumeId)!
    expect(result.summary).toBeNull()
  })

  test('buildSummary returns template mode when only summary_id is set', () => {
    const resumeId = seedResume(db)
    const summaryId = crypto.randomUUID()
    db.run(
      `INSERT INTO summaries (id, title, description, is_template)
       VALUES (?, ?, ?, ?)`,
      [summaryId, 'Senior Engineer Template', 'Template description text.', 1]
    )
    db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [summaryId, resumeId])

    const result = compileResumeIR(db, resumeId)!
    expect(result.summary).not.toBeNull()
    expect(result.summary!.summary_id).toBe(summaryId)
    expect(result.summary!.title).toBe('Senior Engineer Template')
    expect(result.summary!.content).toBe('Template description text.')
    expect(result.summary!.is_override).toBe(false)
  })

  test('buildSummary returns override mode when both summary_id and summary_override are set', () => {
    const resumeId = seedResume(db)
    const summaryId = crypto.randomUUID()
    db.run(
      `INSERT INTO summaries (id, title, description, is_template)
       VALUES (?, ?, ?, ?)`,
      [summaryId, 'Template Title', 'Template text.', 1]
    )
    db.run(
      'UPDATE resumes SET summary_id = ?, summary_override = ? WHERE id = ?',
      [summaryId, 'Locally edited text.', resumeId]
    )

    const result = compileResumeIR(db, resumeId)!
    expect(result.summary!.summary_id).toBe(summaryId)
    expect(result.summary!.title).toBe('Template Title')
    expect(result.summary!.content).toBe('Locally edited text.')
    expect(result.summary!.is_override).toBe(true)
  })

  test('buildSummary returns freeform mode when only summary_override is set', () => {
    const resumeId = seedResume(db)
    db.run(
      'UPDATE resumes SET summary_override = ? WHERE id = ?',
      ['Freeform only text, no template.', resumeId]
    )

    const result = compileResumeIR(db, resumeId)!
    expect(result.summary).not.toBeNull()
    expect(result.summary!.summary_id).toBeNull()
    expect(result.summary!.title).toBeNull()
    expect(result.summary!.content).toBe('Freeform only text, no template.')
    expect(result.summary!.is_override).toBe(true)
  })

  test('buildSummary returns null when summary_id points at a deleted summary', () => {
    const resumeId = seedResume(db)
    const summaryId = crypto.randomUUID()
    db.run(
      `INSERT INTO summaries (id, title, description, is_template)
       VALUES (?, ?, ?, ?)`,
      [summaryId, 'Will Be Deleted', 'Some text.', 1]
    )
    db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [summaryId, resumeId])
    // ON DELETE SET NULL fires: summary_id becomes NULL on the resume row
    db.run('DELETE FROM summaries WHERE id = ?', [summaryId])

    const result = compileResumeIR(db, resumeId)!
    expect(result.summary).toBeNull()
  })

  test('compileResumeIR handles deleted summary gracefully', () => {
    const resumeId = seedResume(db)
    const summaryId = seedSummary(db)

    db.run('UPDATE resumes SET summary_id = ? WHERE id = ?', [summaryId, resumeId])
    db.run('DELETE FROM summaries WHERE id = ?', [summaryId])

    // summary_id is now NULL due to ON DELETE SET NULL
    const ir = compileResumeIR(db, resumeId)
    expect(ir).not.toBeNull()
    // Should not crash -- falls back to default behavior
  })
})

describe('syntheticUUID', () => {
  test('produces deterministic output', () => {
    const a = syntheticUUID('test', 'key')
    const b = syntheticUUID('test', 'key')
    expect(a).toBe(b)
  })

  test('different inputs produce different output', () => {
    const a = syntheticUUID('test', 'key1')
    const b = syntheticUUID('test', 'key2')
    expect(a).not.toBe(b)
  })
})

describe('formatDateRange', () => {
  test('formats start and end dates', () => {
    const result = formatDateRange('2024-03-01', '2025-07-01', false)
    expect(result).toContain('2024')
    expect(result).toContain('2025')
    expect(result).toContain(' - ')
  })

  test('formats current role', () => {
    const result = formatDateRange('2018-09-01', null, true)
    expect(result).toContain('Present')
  })

  test('handles no dates', () => {
    expect(formatDateRange(null, null, false)).toBe('')
  })

  test('handles end date only', () => {
    const result = formatDateRange(null, '2023-06-01', false)
    expect(result).toContain('2023')
  })
})

// ── Phase 44: IR Data Quality Tests ─────────────────────────────────

describe('buildOrgDisplayString', () => {
  test('location in parens takes precedence over arrangement', () => {
    expect(buildOrgDisplayString('Raytheon', 'Arlington', 'VA', 'remote'))
      .toBe('Raytheon (Arlington, VA)')
  })

  test('returns org name only when no location or arrangement', () => {
    expect(buildOrgDisplayString('Acme Corp', null, null, null))
      .toBe('Acme Corp')
  })

  test('returns "Other" when org name is null', () => {
    expect(buildOrgDisplayString(null, null, null, null))
      .toBe('Other')
  })

  test('city only shows in parens', () => {
    expect(buildOrgDisplayString('USAFR', 'National Capitol Region', null, null))
      .toBe('USAFR (National Capitol Region)')
  })

  test('state only shows in parens', () => {
    expect(buildOrgDisplayString('TestOrg', null, 'VA', null))
      .toBe('TestOrg (VA)')
  })

  test('arrangement shows in parens when no location', () => {
    expect(buildOrgDisplayString('Cisco', null, null, 'remote'))
      .toBe('Cisco (Remote)')
  })

  test('capitalizes work arrangement', () => {
    expect(buildOrgDisplayString('Corp', null, null, 'contract'))
      .toBe('Corp (Contract)')
  })

  test('location wins over arrangement when both present', () => {
    expect(buildOrgDisplayString('Cisco', 'San Jose', 'CA', 'contract'))
      .toBe('Cisco (San Jose, CA)')
  })
})

describe('formatClearanceLevel', () => {
  test('top_secret renders as TS/SCI (IC default SCI access assumption)', () => {
    expect(formatClearanceLevel('top_secret')).toBe('TS/SCI')
  })

  test('secret renders capitalized', () => {
    expect(formatClearanceLevel('secret')).toBe('Secret')
  })

  test('confidential renders capitalized', () => {
    expect(formatClearanceLevel('confidential')).toBe('Confidential')
  })

  test('public renders as Public Trust', () => {
    expect(formatClearanceLevel('public')).toBe('Public Trust')
  })

  test('DOE Q and L clearances render with agency prefix', () => {
    expect(formatClearanceLevel('q')).toBe('DOE Q')
    expect(formatClearanceLevel('l')).toBe('DOE L')
  })

  test('null level returns empty string', () => {
    expect(formatClearanceLevel(null)).toBe('')
  })

  test('unknown level returns raw value (forward compatibility)', () => {
    expect(formatClearanceLevel('future_level')).toBe('future_level')
  })
})

describe('formatPolygraph', () => {
  test('ci renders as CI Poly', () => {
    expect(formatPolygraph('ci')).toBe('CI Poly')
  })

  test('full_scope renders as Full-Scope Poly', () => {
    expect(formatPolygraph('full_scope')).toBe('Full-Scope Poly')
  })

  test('none and null both return null so the "with X" clause is omitted', () => {
    expect(formatPolygraph('none')).toBeNull()
    expect(formatPolygraph(null)).toBeNull()
  })

  test('unknown polygraph returns raw value (forward compatibility)', () => {
    expect(formatPolygraph('future_poly')).toBe('future_poly')
  })
})

describe('compileResumeIR — Phase 44 data quality', () => {
  let db: Database

  beforeEach(() => {
    db = createTestDb()
  })

  afterEach(() => db.close())

  // ── T44.1: Header warnings ──────────────────────────────────────

  test('warns when profile name is "User" (seed default)', () => {
    const warnSpy = spyOn(console, 'warn')
    const resumeId = seedResume(db, { name: 'My Resume' })
    // Default profile from migration has name "User"
    compileResumeIR(db, resumeId)
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('user_profile.name is "User"')
    )
    warnSpy.mockRestore()
  })

  test('warns when no profile row exists', () => {
    const warnSpy = spyOn(console, 'warn')
    const resumeId = seedResume(db, { name: 'Fallback Resume' })
    db.run('DELETE FROM user_profile')
    compileResumeIR(db, resumeId)
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('No user_profile row found'),
      expect.anything()
    )
    warnSpy.mockRestore()
  })

  test('warns when profile has no contact fields', () => {
    const warnSpy = spyOn(console, 'warn')
    const resumeId = seedResume(db)
    seedProfile(db, { name: 'Adam' })
    // No email, phone, linkedin, github
    compileResumeIR(db, resumeId)
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('no contact fields populated')
    )
    warnSpy.mockRestore()
  })

  test('no warnings when profile is fully populated', () => {
    const warnSpy = spyOn(console, 'warn')
    const resumeId = seedResume(db)
    seedProfile(db, { name: 'Adam', email: 'adam@example.com', phone: '+1-555-0123' })
    compileResumeIR(db, resumeId)
    expect(warnSpy).not.toHaveBeenCalled()
    warnSpy.mockRestore()
  })

  // ── T44.2: Experience org display ───────────────────────────────

  test('experience org name from organizations table (not "Other")', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Cisco' })
    const sourceId = seedSource(db, { title: 'Network Engineer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current)
       VALUES (?, ?, ?, ?)`,
      [sourceId, orgId, '2024-01-01', 1]
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'Built networks' })
    const perspId = seedPerspective(db, bulletId, { content: 'Architected network infrastructure' })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    const group = expSection.items[0] as ExperienceGroup
    expect(group.organization).toContain('Cisco')
    expect(group.organization).not.toBe('Other')
  })

  test('experience org with HQ campus location', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Raytheon Intelligence & Space' })

    // Create HQ location with address
    const hqAddrId = crypto.randomUUID()
    db.run(
      `INSERT INTO addresses (id, name, city, state) VALUES (?, ?, ?, ?)`,
      [hqAddrId, 'HQ', 'Arlington', 'VA']
    )
    const campusId = crypto.randomUUID()
    db.run(
      `INSERT INTO org_locations (id, organization_id, name, modality, address_id, is_headquarters)
       VALUES (?, ?, ?, ?, ?, ?)`,
      [campusId, orgId, 'HQ', 'in_person', hqAddrId, 1]
    )

    const sourceId = seedSource(db, { title: 'Engineer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current, work_arrangement)
       VALUES (?, ?, ?, ?, ?)`,
      [sourceId, orgId, '2024-01-01', 1, 'remote']
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'Did work' })
    const perspId = seedPerspective(db, bulletId, { content: 'Engineered things' })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    const group = expSection.items[0] as ExperienceGroup
    // New format: location in parens takes precedence over arrangement
    expect(group.organization).toBe('Raytheon Intelligence & Space (Arlington, VA)')
  })

  test('experience org with work arrangement but no location', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Cisco' })
    const sourceId = seedSource(db, { title: 'Engineer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current, work_arrangement)
       VALUES (?, ?, ?, ?, ?)`,
      [sourceId, orgId, '2024-01-01', 1, 'remote']
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'Did work' })
    const perspId = seedPerspective(db, bulletId, { content: 'Built platform' })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    const group = expSection.items[0] as ExperienceGroup
    expect(group.organization).toBe('Cisco (Remote)')
  })

  test('experience org name-only when no campus and no arrangement', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'Acme Corp' })
    const sourceId = seedSource(db, { title: 'Developer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current)
       VALUES (?, ?, ?, ?)`,
      [sourceId, orgId, '2024-01-01', 1]
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'Coded stuff' })
    const perspId = seedPerspective(db, bulletId, { content: 'Developed features' })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    const group = expSection.items[0] as ExperienceGroup
    expect(group.organization).toBe('Acme Corp')
  })

  test('experience groups by org ID, not name (two orgs with same name)', () => {
    const resumeId = seedResume(db)
    const orgId1 = seedOrganization(db, { name: 'Acme' })
    const orgId2 = seedOrganization(db, { name: 'Acme' })

    const sourceId1 = seedSource(db, { title: 'Engineer', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current)
       VALUES (?, ?, ?, ?)`,
      [sourceId1, orgId1, '2024-01-01', 1]
    )
    const bulletId1 = seedBullet(db, [{ id: sourceId1, isPrimary: true }], { content: 'Work 1' })
    const perspId1 = seedPerspective(db, bulletId1, { content: 'Perspective 1' })

    const sourceId2 = seedSource(db, { title: 'Manager', sourceType: 'role' })
    db.run(
      `INSERT INTO source_roles (source_id, organization_id, start_date, is_current)
       VALUES (?, ?, ?, ?)`,
      [sourceId2, orgId2, '2023-01-01', 0]
    )
    const bulletId2 = seedBullet(db, [{ id: sourceId2, isPrimary: true }], { content: 'Work 2' })
    const perspId2 = seedPerspective(db, bulletId2, { content: 'Perspective 2' })

    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId1, position: 0 })
    seedResumeEntry(db, secId, { perspectiveId: perspId2, position: 1 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    // Two different orgs with same name should produce 2 separate groups
    expect(expSection.items).toHaveLength(2)
  })

  test('experience falls back to "⚠ Unlinked Sources" when no org (Layer 3 defense)', () => {
    const resumeId = seedResume(db)
    const sourceId = seedSource(db, { title: 'Freelance', sourceType: 'role' })
    // No source_roles entry (no org)

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'Did freelance work' })
    const perspId = seedPerspective(db, bulletId, { content: 'Consulted on security' })
    const secId = seedResumeSection(db, resumeId, 'Experience', 'experience', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const expSection = result.sections.find(s => s.type === 'experience')!
    const group = expSection.items[0] as ExperienceGroup
    // Layer 3: "Other" replaced with actionable label
    expect(group.organization).toBe('⚠ Unlinked Sources')
  })

  // ── T44.4: Education location from campus ──────────────────────

  test('education location falls back to campus when se.location is NULL', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'University of Maryland' })

    const cpAddrId = crypto.randomUUID()
    db.run(
      `INSERT INTO addresses (id, name, city, state) VALUES (?, ?, ?, ?)`,
      [cpAddrId, 'College Park', 'College Park', 'MD']
    )
    const campusId = crypto.randomUUID()
    db.run(
      `INSERT INTO org_locations (id, organization_id, name, modality, address_id)
       VALUES (?, ?, ?, ?, ?)`,
      [campusId, orgId, 'College Park', 'in_person', cpAddrId]
    )

    const sourceId = seedSource(db, { title: 'UMD Degree', sourceType: 'education' })
    db.run(
      `INSERT INTO source_education (source_id, education_type, organization_id, campus_id, field, end_date)
       VALUES (?, ?, ?, ?, ?, ?)`,
      [sourceId, 'degree', orgId, campusId, 'Computer Science', '2023-06-01']
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'B.S. CS' })
    const perspId = seedPerspective(db, bulletId, { content: 'B.S. Computer Science' })
    const secId = seedResumeSection(db, resumeId, 'Education', 'education', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const eduSection = result.sections.find(s => s.type === 'education')!
    const item = eduSection.items[0] as EducationItem
    expect(item.institution).toBe('University of Maryland')
    expect(item.location).toBe('College Park, MD')
  })

  test('education location from se.location takes priority over campus', () => {
    const resumeId = seedResume(db)
    const orgId = seedOrganization(db, { name: 'WGU' })

    const slcAddrId = crypto.randomUUID()
    db.run(
      `INSERT INTO addresses (id, name, city, state) VALUES (?, ?, ?, ?)`,
      [slcAddrId, 'Main Campus', 'Salt Lake City', 'UT']
    )
    const campusId = crypto.randomUUID()
    db.run(
      `INSERT INTO org_locations (id, organization_id, name, modality, address_id)
       VALUES (?, ?, ?, ?, ?)`,
      [campusId, orgId, 'Main Campus', 'remote', slcAddrId]
    )

    const sourceId = seedSource(db, { title: 'WGU Degree', sourceType: 'education' })
    db.run(
      `INSERT INTO source_education (source_id, education_type, organization_id, campus_id, field, end_date, location)
       VALUES (?, ?, ?, ?, ?, ?, ?)`,
      [sourceId, 'degree', orgId, campusId, 'Cybersecurity', '2023-06-01', 'Online']
    )

    const bulletId = seedBullet(db, [{ id: sourceId, isPrimary: true }], { content: 'B.S. Cybersecurity' })
    const perspId = seedPerspective(db, bulletId, { content: 'B.S. Cybersecurity' })
    const secId = seedResumeSection(db, resumeId, 'Education', 'education', 0)
    seedResumeEntry(db, secId, { perspectiveId: perspId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const eduSection = result.sections.find(s => s.type === 'education')!
    const item = eduSection.items[0] as EducationItem
    expect(item.location).toBe('Online')  // se.location takes priority
  })

  // ── T44.5: Skills orphan handling ──────────────────────────────

  test('skills with orphaned entries produce warning', () => {
    const warnSpy = spyOn(console, 'warn')
    const resumeId = seedResume(db)
    const secId = seedResumeSection(db, resumeId, 'Skills', 'skills', 0)

    // Create a valid skill
    const skillId = crypto.randomUUID()
    db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [skillId, 'Python', 'language'])
    seedResumeSkill(db, secId, skillId, 0)

    // Create an orphaned resume_skills entry (skill_id points to nonexistent skill)
    // Must disable FK checks temporarily to insert the orphan
    const orphanSkillId = crypto.randomUUID()
    db.run('PRAGMA foreign_keys = OFF')
    seedResumeSkill(db, secId, orphanSkillId, 1)
    db.run('PRAGMA foreign_keys = ON')

    seedProfile(db, { name: 'Adam', email: 'adam@test.com' })
    const result = compileResumeIR(db, resumeId)!
    const skillsSection = result.sections.find(s => s.type === 'skills')!

    // Valid skill still appears
    const group = skillsSection.items[0]
    if (group.kind === 'skill_group') {
      const langs = group.categories.find(c => c.label === 'Languages')
      expect(langs).toBeDefined()
      expect(langs!.skills).toContain('Python')
    }

    // Warning about orphan
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('orphaned resume_skills')
    )
    warnSpy.mockRestore()
  })

  test('presentation section renders direct-source entry with venue + type', () => {
    const resumeId = seedResume(db)
    const sourceId = seedSource(db, {
      title: 'Cloud Forensics at Scale',
      sourceType: 'presentation',
      description: 'Lessons from production IR',
    })
    db.run(
      `INSERT INTO source_presentations (source_id, venue, presentation_type, url, coauthors)
       VALUES (?, ?, ?, ?, ?)`,
      [sourceId, 'BSides DC 2024', 'conference_talk', 'https://slides.example.com', 'Jane Smith']
    )
    const secId = seedResumeSection(db, resumeId, 'Presentations', 'presentations', 0)
    seedResumeEntry(db, secId, { sourceId, position: 0 })

    const result = compileResumeIR(db, resumeId)!
    const presSection = result.sections.find(s => s.type === 'presentations')!
    expect(presSection.items).toHaveLength(1)
    const item = presSection.items[0] as PresentationItem
    expect(item.kind).toBe('presentation')
    expect(item.title).toBe('Cloud Forensics at Scale')
    expect(item.venue).toBe('BSides DC 2024')
    expect(item.presentation_type).toBe('conference_talk')
    expect(item.url).toBe('https://slides.example.com')
    expect(item.coauthors).toBe('Jane Smith')
    expect(item.description).toBe('Lessons from production IR')
  })

  test('skills categories are ordered alphabetically', () => {
    const resumeId = seedResume(db)
    seedProfile(db, { name: 'Adam', email: 'adam@test.com' })
    const secId = seedResumeSection(db, resumeId, 'Skills', 'skills', 0)

    // Phase 89: category is a CHECK enum; use valid values that still
    // exercise alphabetical ordering across multiple categories.
    const skillIds = [
      { name: 'Terraform', category: 'tool' },
      { name: 'Python', category: 'language' },
      { name: 'AWS', category: 'platform' },
    ]
    skillIds.forEach((s, i) => {
      const id = crypto.randomUUID()
      db.run('INSERT INTO skills (id, name, category) VALUES (?, ?, ?)', [id, s.name, s.category])
      seedResumeSkill(db, secId, id, i)
    })

    const result = compileResumeIR(db, resumeId)!
    const skillsSection = result.sections.find(s => s.type === 'skills')!
    const group = skillsSection.items[0]
    if (group.kind === 'skill_group') {
      const labels = group.categories.map(c => c.label)
      // Ordered by skill_categories.position: Languages(5) < Platforms(9) < Tools(13)
      expect(labels).toEqual(['Languages', 'Platforms', 'Tools'])
    }
  })
})
