import type {
  CreateOrganization,
  LocationModality,
  Organization,
  OrganizationFilter,
  OrgAlias,
  OrgLocation,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  UpdateOrganization,
} from '../types'

export interface CreateOrgLocation {
  name: string
  modality?: LocationModality
  address_id?: string | null
  is_headquarters?: boolean
}

export interface UpdateOrgLocation {
  name?: string
  modality?: LocationModality
  address_id?: string | null
  is_headquarters?: boolean
}

function toParams(
  filter?: object,
): Record<string, string> | undefined {
  if (!filter) return undefined
  const out: Record<string, string> = {}
  for (const [k, v] of Object.entries(filter)) {
    if (v !== undefined && v !== null) out[k] = String(v)
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export class OrganizationsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateOrganization): Promise<Result<Organization>> {
    return this.request<Organization>('POST', '/api/organizations', input)
  }

  list(
    filter?: OrganizationFilter & PaginationParams,
  ): Promise<PaginatedResult<Organization>> {
    return this.requestList<Organization>(
      'GET',
      '/api/organizations',
      toParams(filter),
    )
  }

  get(id: string): Promise<Result<Organization>> {
    return this.request<Organization>('GET', `/api/organizations/${id}`)
  }

  update(
    id: string,
    input: UpdateOrganization,
  ): Promise<Result<Organization>> {
    return this.request<Organization>(
      'PATCH',
      `/api/organizations/${id}`,
      input,
    )
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/organizations/${id}`)
  }

  // ── Org Locations (formerly campuses) ─────────────────────────────

  listLocations(orgId: string): Promise<Result<OrgLocation[]>> {
    return this.request<OrgLocation[]>('GET', `/api/organizations/${orgId}/locations`)
  }

  createLocation(orgId: string, input: CreateOrgLocation): Promise<Result<OrgLocation>> {
    return this.request<OrgLocation>('POST', `/api/organizations/${orgId}/locations`, input)
  }

  updateLocation(locationId: string, input: UpdateOrgLocation): Promise<Result<OrgLocation>> {
    return this.request<OrgLocation>('PATCH', `/api/locations/${locationId}`, input)
  }

  deleteLocation(locationId: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/locations/${locationId}`)
  }

  // ── Org name variants ─────────────────────────────────────────────

  listAliases(orgId: string): Promise<Result<OrgAlias[]>> {
    return this.request<OrgAlias[]>('GET', `/api/organizations/${orgId}/aliases`)
  }

  createAlias(orgId: string, alias: string): Promise<Result<OrgAlias>> {
    return this.request<OrgAlias>('POST', `/api/organizations/${orgId}/aliases`, { alias })
  }

  deleteAlias(aliasId: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/aliases/${aliasId}`)
  }
}
