/**
 * Persona `cleared-security-engineer`: a senior security engineer near Chantilly, VA with an
 * active TS/SCI and CI polygraph, a military network defense background and two federal
 * contractor roles, applying for principal, architect and lead roles in VA, MD, DC and CO.
 *
 * Coverage it is built for: the Federal Resume template with its clearance section, a
 * clearance credential (with polygraph and SCI access) and a driver's license, four
 * certifications (one in progress) on a generated certifications section, a presentations
 * section, a military unit, a government agency and a volunteer organization, a protected
 * veteran answer bank, every source type, and every board column for sources, bullets,
 * perspectives, resumes, JDs and organizations.
 */

import type { PersonaCorpus } from '../../corpus/types'
import { contacts, jds, notes, resumes, summaries } from './applications'
import { bullets, perspectives, sources } from './career'
import { answers, certifications, credentials, industries, orgs, profile, roleTypes, skills } from './reference'

export const clearedSecurityEngineer: PersonaCorpus = {
  slug: 'cleared-security-engineer',
  summary:
    'Casey Thornbury, a cleared senior security engineer (TS/SCI with CI polygraph) near Chantilly, VA, moving from military network defense through federal contracting toward principal, zero trust architect and lead federal roles.',
  accountAgeDays: 560,
  profile,
  answers,
  industries,
  roleTypes,
  skills,
  orgs,
  credentials,
  certifications,
  sources,
  bullets,
  perspectives,
  summaries,
  jds,
  contacts,
  resumes,
  notes,
  dir: import.meta.dir,
}
