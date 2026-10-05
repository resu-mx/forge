import type {
  Domain,
  DomainWithUsage,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
} from '../types'

export interface CreateDomain {
  name: string
  description?: string
}

export interface UpdateDomain {
  name?: string
  description?: string | null
}

export class DomainsResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateDomain): Promise<Result<Domain>> {
    return this.request<Domain>('POST', '/api/domains', input)
  }

  list(params?: PaginationParams): Promise<PaginatedResult<DomainWithUsage>> {
    const p: Record<string, string> = {}
    if (params?.offset !== undefined) p.offset = String(params.offset)
    if (params?.limit !== undefined) p.limit = String(params.limit)
    return this.requestList<DomainWithUsage>('GET', '/api/domains', Object.keys(p).length > 0 ? p : undefined)
  }

  get(id: string): Promise<Result<Domain>> {
    return this.request<Domain>('GET', `/api/domains/${id}`)
  }

  update(id: string, input: UpdateDomain): Promise<Result<Domain>> {
    return this.request<Domain>('PATCH', `/api/domains/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/domains/${id}`)
  }
}
