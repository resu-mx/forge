import type { ResumeDocument } from '@forge/sdk'

/**
 * Generate a Markdown string from a compiled ResumeDocument IR.
 * Shared between ResumeEditor (source view) and ResumePreview (markdown tab).
 */
export function generateMarkdownFromIR(doc: ResumeDocument): string {
  const lines: string[] = []
  lines.push(`# ${doc.header.name}`)
  if (doc.header.tagline) lines.push(`*${doc.header.tagline}*`)
  if (doc.header.clearance) lines.push(`**${doc.header.clearance}**`)
  const contact: string[] = []
  if (doc.header.location) contact.push(doc.header.location)
  if (doc.header.email) contact.push(doc.header.email)
  if (doc.header.phone) contact.push(doc.header.phone)
  if (doc.header.linkedin) contact.push(`[LinkedIn](${doc.header.linkedin})`)
  if (doc.header.github) contact.push(`[GitHub](${doc.header.github})`)
  if (doc.header.website) contact.push(`[Website](${doc.header.website})`)
  if (contact.length > 0) lines.push(contact.join(' | '))
  lines.push('')

  for (const section of doc.sections) {
    lines.push(`## ${section.title}`)
    lines.push('')
    for (const item of section.items) {
      if (item.kind === 'experience_group') {
        lines.push(`### ${item.organization}`)
        for (const sub of item.subheadings) {
          lines.push(`**${sub.title}** | ${sub.date_range}`)
          lines.push('')
          for (const b of sub.bullets) {
            lines.push(`- ${b.content}`)
          }
          lines.push('')
        }
      } else if (item.kind === 'summary') {
        lines.push(item.content)
        lines.push('')
      } else if (item.kind === 'education') {
        lines.push(`**${item.institution}**${item.location ? ` | ${item.location}` : ''}`)
        lines.push(`${item.degree} | ${item.date}`)
        lines.push('')
      } else if (item.kind === 'skill_group') {
        for (const cat of item.categories) {
          lines.push(`**${cat.label}:** ${cat.skills.join(', ')}`)
        }
        lines.push('')
      } else if (item.kind === 'project') {
        lines.push(`**${item.name}**${item.date ? ` | ${item.date}` : ''}`)
        for (const b of item.bullets) {
          lines.push(`- ${b.content}`)
        }
        lines.push('')
      } else if (item.kind === 'clearance') {
        lines.push(`- ${item.content}`)
      } else if (item.kind === 'certification_group') {
        for (const cat of item.categories) {
          lines.push(`**${cat.label}:** ${cat.certs.map(c => (c.in_progress ? `${c.name} (in progress)` : c.name)).join(', ')}`)
        }
        lines.push('')
      } else if (item.kind === 'presentation') {
        lines.push(`**${item.title}**${item.date ? ` | ${item.date}` : ''}`)
        for (const b of item.bullets) {
          lines.push(`- ${b.content}`)
        }
        lines.push('')
      }
    }
  }

  return lines.join('\n')
}
