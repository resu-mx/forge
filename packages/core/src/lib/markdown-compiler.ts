/**
 * IR -> Markdown compiler.
 *
 * Pure function: takes a ResumeDocument, returns a GHFM string.
 * All user-authored content is escaped via escapeMarkdown().
 */

import type { ResumeDocument, IRSection, ResumeHeader } from '../types'
import { escapeMarkdown } from './escape'

export function compileToMarkdown(doc: ResumeDocument): string {
  const lines: string[] = []

  // Header
  lines.push(...renderHeader(doc.header))
  lines.push('')

  // Sections
  for (const section of doc.sections) {
    lines.push(...renderSection(section))
    lines.push('')
  }

  return lines.join('\n').trimEnd() + '\n'
}

function renderHeader(header: ResumeHeader): string[] {
  const lines: string[] = []
  lines.push(`# ${escapeMarkdown(header.name)}`)

  if (header.tagline) {
    lines.push(escapeMarkdown(header.tagline))
  }

  // Clearance one-liner (between tagline and contact info)
  if (header.clearance) {
    lines.push(`**${escapeMarkdown(header.clearance)}**`)
  }

  // Contact line
  const contact: string[] = []
  if (header.location) contact.push(escapeMarkdown(header.location))
  if (header.email) contact.push(escapeMarkdown(header.email))
  if (header.phone) contact.push(escapeMarkdown(header.phone))
  if (header.linkedin) contact.push(`[LinkedIn](${header.linkedin})`)
  if (header.github) contact.push(`[GitHub](${header.github})`)
  if (header.website) contact.push(`[Website](${header.website})`)

  if (contact.length > 0) {
    lines.push(contact.join(' | '))
  }

  return lines
}

function renderSection(section: IRSection): string[] {
  const lines: string[] = []
  lines.push(`## ${escapeMarkdown(section.title)}`)
  lines.push('')

  switch (section.type) {
    case 'summary':
      for (const item of section.items) {
        if (item.kind === 'summary') {
          lines.push(escapeMarkdown(item.content))
        }
      }
      break

    case 'experience':
      for (const item of section.items) {
        if (item.kind === 'experience_group') {
          lines.push(`### ${escapeMarkdown(item.organization)}`)
          for (const sub of item.subheadings) {
            const titleWithLoc = sub.location ? `${sub.title} (${sub.location})` : sub.title
            lines.push(`**${escapeMarkdown(titleWithLoc)}** | ${escapeMarkdown(sub.date_range)}`)
            for (const bullet of sub.bullets) {
              lines.push(`- ${escapeMarkdown(bullet.content)}`)
            }
            lines.push('')
          }
        }
      }
      break

    case 'skills':
      for (const item of section.items) {
        if (item.kind === 'skill_group') {
          for (const cat of item.categories) {
            lines.push(`**${escapeMarkdown(cat.label)}**: ${cat.skills.map(s => escapeMarkdown(s)).join(', ')}`)
          }
        }
      }
      break

    case 'education':
      for (const item of section.items) {
        if (item.kind === 'education') {
          lines.push(`**${escapeMarkdown(item.institution)}**`)
          lines.push(`${escapeMarkdown(item.degree)} | ${escapeMarkdown(item.date)}`)
          lines.push('')
        }
      }
      break

    case 'projects':
      for (const item of section.items) {
        if (item.kind === 'project') {
          const dateSuffix = item.date ? ` | ${escapeMarkdown(item.date)}` : ''
          lines.push(`### ${escapeMarkdown(item.name)}${dateSuffix}`)
          for (const bullet of item.bullets) {
            lines.push(`- ${escapeMarkdown(bullet.content)}`)
          }
          lines.push('')
        }
      }
      break

    case 'certifications':
      for (const item of section.items) {
        if (item.kind === 'certification_group') {
          for (const cat of item.categories) {
            const names = cat.certs.map(c => escapeMarkdown(c.name) + (c.in_progress ? ' (in progress)' : ''))
            lines.push(`**${escapeMarkdown(cat.label)}**: ${names.join(', ')}`)
          }
        }
      }
      break

    case 'clearance':
      for (const item of section.items) {
        if (item.kind === 'clearance') {
          lines.push(escapeMarkdown(item.content))
        }
      }
      break

    case 'presentations':
      for (const item of section.items) {
        if (item.kind === 'presentation') {
          const venueDate = [item.venue, item.date].filter(Boolean).join(', ')
          lines.push(`### ${escapeMarkdown(item.title)}${venueDate ? ` | ${escapeMarkdown(venueDate)}` : ''}`)
          for (const bullet of item.bullets) {
            lines.push(`- ${escapeMarkdown(bullet.content)}`)
          }
          lines.push('')
        }
      }
      break

    default:
      // custom / awards / fallback
      for (const item of section.items) {
        if ('content' in item && typeof item.content === 'string') {
          lines.push(`- ${escapeMarkdown(item.content)}`)
        }
      }
      break
  }

  return lines
}
