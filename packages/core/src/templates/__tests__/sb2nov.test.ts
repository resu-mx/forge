import { describe, test, expect } from 'bun:test'
import { sb2nov } from '../sb2nov'
import type { IRSection, ResumeHeader } from '../../types'

describe('sb2nov template', () => {
  describe('preamble', () => {
    test('contains documentclass', () => {
      expect(sb2nov.preamble).toContain('\\documentclass[letterpaper,10pt]{article}')
    })

    test('contains required packages', () => {
      expect(sb2nov.preamble).toContain('\\usepackage[empty]{fullpage}')
      expect(sb2nov.preamble).toContain('\\usepackage{titlesec}')
      expect(sb2nov.preamble).toContain('\\usepackage{hyperref}')
      expect(sb2nov.preamble).toContain('\\usepackage{fancyhdr}')
    })

    test('contains resume command definitions', () => {
      expect(sb2nov.preamble).toContain('\\newcommand{\\resumeItem}')
      expect(sb2nov.preamble).toContain('\\newcommand{\\resumeSubheading}')
      expect(sb2nov.preamble).toContain('\\newcommand{\\resumeSubSubheading}')
      expect(sb2nov.preamble).toContain('\\newcommand{\\resumeProjectHeading}')
      expect(sb2nov.preamble).toContain('\\newcommand{\\resumeSubHeadingListStart}')
      expect(sb2nov.preamble).toContain('\\newcommand{\\resumeSubHeadingListEnd}')
      expect(sb2nov.preamble).toContain('\\newcommand{\\resumeItemListStart}')
      expect(sb2nov.preamble).toContain('\\newcommand{\\resumeItemListEnd}')
    })
  })

  describe('renderHeader', () => {
    const header: ResumeHeader = {
      name: 'Adam Smith',
      tagline: 'Security Engineer | Cloud + DevSecOps',
      location: 'Reston, VA',
      email: 'adam@example.com',
      phone: '816-447-6683',
      linkedin: 'https://linkedin.com/in/adam',
      github: 'https://github.com/adam',
      website: null,
      clearance: null,
    }

    test('wraps in begin/end center', () => {
      const result = sb2nov.renderHeader(header)
      expect(result).toContain('\\begin{center}')
      expect(result).toContain('\\end{center}')
    })

    test('renders name with Huge scshape', () => {
      const result = sb2nov.renderHeader(header)
      expect(result).toContain('\\textbf{\\Huge \\scshape Adam Smith}')
    })

    test('renders tagline', () => {
      const result = sb2nov.renderHeader(header)
      expect(result).toContain('Security Engineer | Cloud + DevSecOps')
    })

    test('renders email with href', () => {
      const result = sb2nov.renderHeader(header)
      expect(result).toContain('\\href{mailto:adam@example.com}{\\underline{adam@example.com}}')
    })

    test('renders phone with tel href', () => {
      const result = sb2nov.renderHeader(header)
      expect(result).toContain('\\href{tel:+8164476683}')
    })

    test('renders LinkedIn and GitHub links', () => {
      const result = sb2nov.renderHeader(header)
      expect(result).toContain('\\href{https://linkedin.com/in/adam}{\\underline{LinkedIn}}')
      expect(result).toContain('\\href{https://github.com/adam}{\\underline{GitHub}}')
    })

    test('contact parts delimited by $|$', () => {
      const result = sb2nov.renderHeader(header)
      expect(result).toContain('$|$')
    })
  })

  describe('renderSection - experience', () => {
    test('first role uses resumeSubheading, second uses resumeSubSubheading', () => {
      const section: IRSection = {
        id: 's1', type: 'experience', title: 'Experience', display_order: 0,
        items: [{
          kind: 'experience_group', id: 'g1', organization: 'Raytheon',
          subheadings: [
            {
              id: 'sh1', title: 'Principal Engineer', date_range: 'Mar 2024 - Jul 2025',
              source_id: null,
              bullets: [{ content: 'Built cloud platform', entry_id: 'e1', is_cloned: false }],
            },
            {
              id: 'sh2', title: 'Cloud Forensics Engineer', date_range: 'Aug 2023 - Mar 2024',
              source_id: null,
              bullets: [{ content: 'Built MLOps pipeline', entry_id: 'e2', is_cloned: false }],
            },
          ],
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeSubheading')
      expect(result).toContain('{Raytheon}{}')
      expect(result).toContain('{Principal Engineer}{Mar 2024 - Jul 2025}')
      expect(result).toContain('\\resumeSubSubheading')
      expect(result).toContain('{Cloud Forensics Engineer}{Aug 2023 - Mar 2024}')
    })

    test('bullets wrapped in resumeItemListStart/End', () => {
      const section: IRSection = {
        id: 's1', type: 'experience', title: 'Experience', display_order: 0,
        items: [{
          kind: 'experience_group', id: 'g1', organization: 'ACME',
          subheadings: [{
            id: 'sh1', title: 'Engineer', date_range: '2024',
            source_id: null,
            bullets: [{ content: 'Did things', entry_id: 'e1', is_cloned: false }],
          }],
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeItemListStart')
      expect(result).toContain('\\resumeItem{Did things}')
      expect(result).toContain('\\resumeItemListEnd')
    })
  })

  describe('renderSection - skills', () => {
    test('renders with textbf label and colon', () => {
      const section: IRSection = {
        id: 's1', type: 'skills', title: 'Technical Skills', display_order: 0,
        items: [{
          kind: 'skill_group',
          categories: [
            { label: 'Languages', skills: ['Python', 'Rust'] },
          ],
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\section{Technical Skills}')
      expect(result).toContain('\\textbf{Languages}{: Python, Rust}')
      expect(result).toContain('\\begin{itemize}[leftmargin=0.15in, label={}]')
    })
  })

  describe('renderSection - education', () => {
    test('uses resumeSubheading with institution and degree', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education', institution: 'WGU', degree: 'B.S. Cybersecurity',
          date: '2023', entry_id: 'e1', source_id: 's1',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeSubheading')
      expect(result).toContain('{WGU}{}')
      expect(result).toContain('{B.S. Cybersecurity}{2023}')
    })
  })

  describe('renderSection - projects', () => {
    test('uses resumeProjectHeading with textbf name', () => {
      const section: IRSection = {
        id: 's1', type: 'projects', title: 'Projects', display_order: 0,
        items: [{
          kind: 'project', name: 'AI Memory', date: '2024',
          entry_id: 'e1', source_id: 's1',
          bullets: [{ content: 'Designed system', entry_id: 'e1', is_cloned: false }],
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeProjectHeading')
      expect(result).toContain('{\\textbf{AI Memory}}{2024}')
      expect(result).toContain('\\resumeItem{Designed system}')
    })
  })

  describe('renderSection - certifications', () => {
    test('renders like skills pattern', () => {
      const section: IRSection = {
        id: 's1', type: 'certifications', title: 'Certifications', display_order: 0,
        items: [{
          kind: 'certification_group',
          categories: [{
            label: 'GIAC',
            certs: [
              { name: 'GCFR', entry_id: null, source_id: null, in_progress: false },
              { name: 'GPCS', entry_id: null, source_id: null, in_progress: false },
            ],
          }],
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\textbf{GIAC}{: GCFR, GPCS}')
    })

    test('marks a certification that is still in progress', () => {
      const section: IRSection = {
        id: 's1', type: 'certifications', title: 'Certifications', display_order: 0,
        items: [{
          kind: 'certification_group',
          categories: [{
            label: 'GIAC',
            certs: [
              { name: 'GCFR', entry_id: null, source_id: null, in_progress: false },
              { name: 'GPCS', entry_id: null, source_id: null, in_progress: true },
            ],
          }],
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\textbf{GIAC}{: GCFR, GPCS (in progress)}')
    })
  })

  describe('renderSection - clearance', () => {
    test('renders plain text under section', () => {
      const section: IRSection = {
        id: 's1', type: 'clearance', title: 'Security Clearance', display_order: 0,
        items: [{ kind: 'clearance', content: 'TS/SCI - Active', entry_id: 'e1', source_id: 's1' }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\section{Security Clearance}')
      expect(result).toContain('TS/SCI - Active')
    })
  })

  describe('renderSection - presentations', () => {
    test('renders with LaTeX-quoted title and emph venue', () => {
      const section: IRSection = {
        id: 's1', type: 'presentations', title: 'Presentations', display_order: 0,
        items: [{
          kind: 'presentation', title: 'Backdoors in LLMs', venue: 'PHA', date: '2024',
          entry_id: 'e1', source_id: 's1',
          bullets: [{ content: 'Presented findings', entry_id: 'e1', is_cloned: false }],
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain("\\textbf{``Backdoors in LLMs''}")
      expect(result).toContain('\\emph{PHA, 2024}')
      expect(result).toContain('\\resumeItem{Presented findings}')
    })
  })

  describe('renderSection - education sub-types', () => {
    test('degree renders with degree_type, field, GPA, location', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'MIT',
          degree: 'Master of Science in CS',
          date: '2022',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'degree',
          degree_level: 'masters',
          degree_type: 'M.S.',
          field: 'Computer Science',
          gpa: '3.9/4.0',
          location: 'Cambridge, MA',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeSubheading')
      expect(result).toContain('{MIT}{Cambridge, MA}')
      expect(result).toContain('M.S. in Computer Science')
      expect(result).toContain('GPA: 3.9/4.0')
      expect(result).toContain('{2022}')
    })

    test('degree without degree_type falls back to perspective content', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'WGU',
          degree: 'B.S. Cybersecurity',
          date: '2023',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'degree',
          degree_level: 'bachelors',
          degree_type: null,
          gpa: null,
          location: null,
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{B.S. Cybersecurity}{2023}')
    })

    test('certificate renders with issuing body and credential ID', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'AWS',
          degree: 'AWS Solutions Architect Professional',
          date: '2027',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'certificate',
          certificate_subtype: 'vendor',
          issuing_body: 'Amazon Web Services',
          credential_id: 'ABC123',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{AWS Solutions Architect Professional}{Exp. 2027}')
      expect(result).toContain('Amazon Web Services -- Credential ID: ABC123')
    })

    test('course renders with institution and location', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'SANS Institute',
          degree: 'SANS SEC504: Hacker Tools',
          date: '2024',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'course',
          location: 'Las Vegas',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{SANS SEC504: Hacker Tools}{2024}')
      expect(result).toContain('{SANS Institute, Las Vegas}{}')
    })

    test('self_taught renders as resumeItem', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'Unknown',
          degree: 'Self-taught Rust',
          date: '',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'self_taught',
          edu_description: 'Learned Rust through open-source contributions and The Rust Book.',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeItem{Learned Rust through open-source contributions and The Rust Book.}')
      expect(result).not.toContain('\\resumeSubheading')
    })

    test('self_taught without edu_description falls back to degree', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'Unknown',
          degree: 'Self-taught Python',
          date: '',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'self_taught',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeItem{Self-taught Python}')
    })

    test('degree with campus_city and campus_state uses campus location', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'WGU',
          degree: 'B.S. Cybersecurity',
          date: '2023',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'degree',
          degree_type: 'B.S.',
          field: 'Cybersecurity',
          campus_city: 'Salt Lake City',
          campus_state: 'UT',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{WGU}{Salt Lake City, UT}')
    })

    test('degree with only campus_city uses city alone', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'MIT',
          degree: 'M.S. CS',
          date: '2022',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'degree',
          campus_city: 'Arlington',
          campus_state: null,
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{MIT}{Arlington}')
    })

    test('degree with no campus falls back to deprecated location', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'Online Univ',
          degree: 'B.S. IT',
          date: '2020',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'degree',
          location: 'Online',
          campus_city: null,
          campus_state: null,
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{Online Univ}{Online}')
    })

    test('degree with no campus and no location renders empty', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'WGU',
          degree: 'B.S. Cyber',
          date: '2023',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'degree',
          campus_city: null,
          campus_state: null,
          location: null,
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{WGU}{}')
    })

    test('course with campus location prefers campus over deprecated location', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'SANS Institute',
          degree: 'SANS SEC504',
          date: '2024',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'course',
          location: 'Las Vegas',
          campus_city: 'Orlando',
          campus_state: 'FL',
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{SANS Institute, Orlando, FL}{}')
    })

    test('course without campus falls back to deprecated location', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'SANS Institute',
          degree: 'SANS SEC504',
          date: '2024',
          entry_id: 'e1',
          source_id: 's1',
          education_type: 'course',
          location: 'Las Vegas',
          campus_city: null,
          campus_state: null,
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('{SANS Institute, Las Vegas}{}')
    })

    test('education without education_type defaults to degree rendering', () => {
      const section: IRSection = {
        id: 's1', type: 'education', title: 'Education', display_order: 0,
        items: [{
          kind: 'education',
          institution: 'State University',
          degree: 'B.A. English',
          date: '2018',
          entry_id: 'e1',
          source_id: 's1',
          // No education_type -- pre-migration data
        }],
      }

      const result = sb2nov.renderSection(section)
      expect(result).toContain('\\resumeSubheading')
      expect(result).toContain('{State University}{}')
      expect(result).toContain('{B.A. English}{2018}')
    })
  })

  describe('footer', () => {
    test('contains end document', () => {
      expect(sb2nov.footer).toContain('\\end{document}')
    })
  })
})
