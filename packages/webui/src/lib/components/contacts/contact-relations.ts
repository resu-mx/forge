/**
 * A contact's relationships, from the contact's side (resu-mx/forge#36): which SDK call lists,
 * links and unlinks each kind, and what the picker offers. ContactLinkSection is the other
 * direction (the contacts of one organization, JD or resume).
 *
 * No `$lib` imports: the client is a parameter, so `bun test` can run this with a fake.
 */
import type {
  ContactJDRelationship,
  ContactOrgRelationship,
  ContactResumeRelationship,
  ForgeClient,
  Result,
} from '@forge/sdk'

export type RelationKind = 'organization' | 'job_description' | 'resume'
export interface RelationRow {
  target_id: string
  label: string
  sublabel: string | null
  relationship: string
}
export interface TargetOption {
  id: string
  label: string
}
export type RelationClient = Pick<ForgeClient, 'contacts' | 'organizations' | 'jobDescriptions' | 'resumes'>

export const RELATION_LABELS: Record<RelationKind, { noun: string; plural: string }> = {
  organization: { noun: 'Organization', plural: 'organizations' },
  job_description: { noun: 'Job Description', plural: 'job descriptions' },
  resume: { noun: 'Resume', plural: 'resumes' },
}

const TARGET_LIMIT = 200 // both servers clamp list limits to 200
const byLabel = (a: TargetOption, b: TargetOption) => a.label.localeCompare(b.label)

export async function listRelations(
  forge: RelationClient,
  kind: RelationKind,
  contactId: string,
): Promise<Result<RelationRow[]>> {
  switch (kind) {
    case 'organization': {
      const r = await forge.contacts.listOrganizations(contactId)
      return r.ok
        ? {
            ok: true,
            data: r.data.map((o) => ({
              target_id: o.id,
              label: o.name,
              sublabel: null,
              relationship: o.relationship,
            })),
          }
        : r
    }
    case 'job_description': {
      const r = await forge.contacts.listJobDescriptions(contactId)
      return r.ok
        ? {
            ok: true,
            data: r.data.map((j) => ({
              target_id: j.id,
              label: j.title,
              sublabel: j.organization_name,
              relationship: j.relationship,
            })),
          }
        : r
    }
    case 'resume': {
      const r = await forge.contacts.listResumes(contactId)
      return r.ok
        ? {
            ok: true,
            data: r.data.map((x) => ({
              target_id: x.id,
              label: x.name,
              sublabel: null,
              relationship: x.relationship,
            })),
          }
        : r
    }
  }
}

export function linkRelation(
  forge: RelationClient,
  kind: RelationKind,
  contactId: string,
  targetId: string,
  relationship: string,
): Promise<Result<void>> {
  switch (kind) {
    case 'organization':
      return forge.contacts.linkOrganization(contactId, targetId, relationship as ContactOrgRelationship)
    case 'job_description':
      return forge.contacts.linkJobDescription(contactId, targetId, relationship as ContactJDRelationship)
    case 'resume':
      return forge.contacts.linkResume(contactId, targetId, relationship as ContactResumeRelationship)
  }
}

export function unlinkRelation(
  forge: RelationClient,
  kind: RelationKind,
  contactId: string,
  targetId: string,
  relationship: string,
): Promise<Result<void>> {
  switch (kind) {
    case 'organization':
      return forge.contacts.unlinkOrganization(contactId, targetId, relationship)
    case 'job_description':
      return forge.contacts.unlinkJobDescription(contactId, targetId, relationship)
    case 'resume':
      return forge.contacts.unlinkResume(contactId, targetId, relationship)
  }
}

export async function listTargets(forge: RelationClient, kind: RelationKind): Promise<Result<TargetOption[]>> {
  switch (kind) {
    case 'organization': {
      const r = await forge.organizations.list({ limit: TARGET_LIMIT })
      return r.ok ? { ok: true, data: r.data.map((o) => ({ id: o.id, label: o.name })).sort(byLabel) } : r
    }
    case 'job_description': {
      const r = await forge.jobDescriptions.list({ limit: TARGET_LIMIT })
      return r.ok
        ? {
            ok: true,
            data: r.data
              .map((j) => ({
                id: j.id,
                label: j.organization_name ? `${j.title} — ${j.organization_name}` : j.title,
              }))
              .sort(byLabel),
          }
        : r
    }
    case 'resume': {
      const r = await forge.resumes.list({ limit: TARGET_LIMIT })
      return r.ok ? { ok: true, data: r.data.map((x) => ({ id: x.id, label: x.name })).sort(byLabel) } : r
    }
  }
}
