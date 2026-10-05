import type {
  Address,
  CreateAddress,
  PaginatedResult,
  PaginationParams,
  RequestFn,
  RequestListFn,
  Result,
  UpdateAddress,
} from '../types'

/** Shared addresses (`addresses` table), referenced by the profile and by org locations. */
export class AddressesResource {
  constructor(
    private request: RequestFn,
    private requestList: RequestListFn,
  ) {}

  create(input: CreateAddress): Promise<Result<Address>> {
    return this.request<Address>('POST', '/api/addresses', input)
  }

  list(params?: PaginationParams): Promise<PaginatedResult<Address>> {
    const p: Record<string, string> = {}
    if (params?.offset !== undefined) p.offset = String(params.offset)
    if (params?.limit !== undefined) p.limit = String(params.limit)
    return this.requestList<Address>('GET', '/api/addresses', Object.keys(p).length > 0 ? p : undefined)
  }

  get(id: string): Promise<Result<Address>> {
    return this.request<Address>('GET', `/api/addresses/${id}`)
  }

  update(id: string, input: UpdateAddress): Promise<Result<Address>> {
    return this.request<Address>('PATCH', `/api/addresses/${id}`, input)
  }

  delete(id: string): Promise<Result<void>> {
    return this.request<void>('DELETE', `/api/addresses/${id}`)
  }
}
