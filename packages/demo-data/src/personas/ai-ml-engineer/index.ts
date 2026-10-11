/**
 * Persona `ai-ml-engineer`: a staff engineer in Seattle, WA whose career runs from robotics
 * (a Ph.D. and a robotics software role) through applied ML to an agent platform, now
 * weighing senior and staff offers in agent platforms, solutions architecture and trading.
 *
 * Coverage it is built for: open-source and personal projects, one presentation of every
 * type, an Academic CV (presentations, publications and awards sections) next to Standard
 * Tech Resumes, the agentic-ai, solutions-architect and hft archetypes, JDs in WA, CA, NY and
 * remote at senior and staff salaries, a driver's license, earned and in-progress
 * certifications, and every board column for sources, bullets, perspectives, resumes, JDs
 * and organizations.
 */

import type { PersonaCorpus } from '../../corpus/types'
import { contacts, jds, notes, resumes, summaries } from './applications'
import { bullets, perspectives, sources } from './career'
import { answers, certifications, credentials, industries, orgs, profile, roleTypes, skills } from './reference'

export const aiMlEngineer: PersonaCorpus = {
  slug: 'ai-ml-engineer',
  summary:
    'Talia Brennan-Osei, a staff engineer in Seattle, WA who moved from robotics research through applied ML to building an agent platform, and is now weighing agent platform, solutions architect and trading-firm roles.',
  accountAgeDays: 540,
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
