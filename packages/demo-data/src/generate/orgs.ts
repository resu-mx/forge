/**
 * Phase 3: organizations (tags, kanban status, aliases, locations with addresses), then
 * credentials and certifications.
 */

import { unwrap } from '../api'
import { type GenContext, phase, relDate, stamp } from './context'

interface Row {
  id: string
}

export async function phaseOrganizations(ctx: GenContext): Promise<void> {
  phase(ctx, 'organizations, aliases, locations')
  const { corpus, forge, raw, ledger } = ctx

  for (const org of corpus.orgs) {
    const created = stamp(ctx, org.daysAgo, `org:${org.key}`)
    // The SDK's CreateOrganization has no `tags`; the API accepts them.
    const row = await raw<Row>('POST', '/organizations', {
      name: org.name,
      org_type: org.org_type,
      tags: org.tags,
      industry: org.industry,
      size: org.size,
      worked: org.worked,
      employment_type: org.employment_type,
      website: org.website,
      linkedin_url: org.linkedin_url,
      glassdoor_url: org.glassdoor_url,
      glassdoor_rating: org.glassdoor_rating,
      status: org.status ?? undefined,
    })
    ledger.add('org', org.key, 'organizations', row.id, { created_at: created, updated_at: created })

    // Org aliases have no SDK method.
    for (const alias of org.aliases ?? []) {
      const a = await raw<Row>('POST', `/organizations/${row.id}/aliases`, { alias })
      ledger.add('org_alias', `${org.key}/${alias}`, 'org_aliases', a.id)
    }

    for (const loc of org.locations ?? []) {
      let addressId: string | undefined
      if (loc.address) {
        const addr = await unwrap(forge.addresses.create({ ...loc.address, country_code: 'US' }), `address ${loc.address.name}`)
        addressId = addr.id
        ledger.add('address', `${org.key}/${loc.key}`, 'addresses', addr.id, { created_at: created, updated_at: created })
      }
      const l = await unwrap(
        forge.organizations.createLocation(row.id, {
          name: loc.name,
          modality: loc.modality,
          address_id: addressId,
          is_headquarters: loc.is_headquarters ?? false,
        }),
        `location ${org.key}/${loc.key}`,
      )
      ledger.add('location', `${org.key}/${loc.key}`, 'org_locations', l.id)
    }
  }
}

export async function phaseQualifications(ctx: GenContext): Promise<void> {
  phase(ctx, 'credentials and certifications')
  const { corpus, forge, raw, ledger } = ctx

  for (const cred of corpus.credentials) {
    const t = stamp(ctx, cred.daysAgo, `credential:${cred.key}`)
    // The Rust API takes `details` as a JSON *string* (CreateCredential.details: Option<String>);
    // the SDK's object form is rejected there. See the generator's report for the bug.
    const row = await raw<Row>('POST', '/credentials', {
      credential_type: cred.credential_type,
      label: cred.label,
      status: cred.status,
      organization_id: cred.orgKey ? ledger.id('org', cred.orgKey) : undefined,
      details: JSON.stringify(cred.details),
      issued_date: relDate(ctx, cred.issued),
      expiry_date: relDate(ctx, cred.expires),
    })
    ledger.add('credential', cred.key, 'credentials', row.id, { created_at: t, updated_at: t })
  }

  for (const cert of corpus.certifications) {
    const t = stamp(ctx, cert.daysAgo, `certification:${cert.key}`)
    const row = await unwrap(
      forge.certifications.create({
        short_name: cert.short_name,
        long_name: cert.long_name,
        cert_id: cert.cert_id,
        issuer_id: cert.issuerKey ? ledger.id('org', cert.issuerKey) : undefined,
        date_earned: relDate(ctx, cert.earned),
        expiry_date: relDate(ctx, cert.expires),
        credential_id: cert.credential_id,
        credential_url: cert.credential_url,
        credly_url: cert.credly_url,
        in_progress: cert.in_progress,
      }),
      `certification ${cert.key}`,
    )
    ledger.add('certification', cert.key, 'certifications', row.id, { created_at: t, updated_at: t })
    for (const skillKey of cert.skills) {
      await unwrap(forge.certifications.addSkill(row.id, ledger.id('skill', skillKey)), `certification ${cert.key} skill ${skillKey}`)
    }
  }
}
