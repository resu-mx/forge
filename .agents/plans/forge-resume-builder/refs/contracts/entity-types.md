# Entity Type Definitions

Shared between `@forge/core` (source of truth) and `@forge/sdk` (consumed types).

## Core Entities

```typescript
interface Employer {
  id: string           // UUID
  name: string
  created_at: string   // ISO 8601
}

interface Project {
  id: string
  name: string
  employer_id: string | null
  description: string | null
  created_at: string
}

interface Source {
  id: string
  title: string
  description: string
  employer_id: string | null
  project_id: string | null
  start_date: string | null    // ISO 8601 date
  end_date: string | null
  status: 'draft' | 'approved' | 'deriving'
  updated_by: 'human' | 'ai'
  last_derived_at: string | null
  created_at: string
  updated_at: string
}

interface Bullet {
  id: string
  source_id: string
  content: string
  source_content_snapshot: string
  technologies: string[]         // denormalized from junction table for API responses
  metrics: string | null
  status: 'draft' | 'pending_review' | 'approved' | 'rejected'
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null     // 'human' for MVP
  created_at: string
}

interface Perspective {
  id: string
  bullet_id: string
  content: string
  bullet_content_snapshot: string
  target_archetype: string | null
  domain: string | null
  framing: 'accomplishment' | 'responsibility' | 'context'
  status: 'draft' | 'pending_review' | 'approved' | 'rejected'
  rejection_reason: string | null
  prompt_log_id: string | null
  approved_at: string | null
  approved_by: string | null
  created_at: string
}

interface Resume {
  id: string
  name: string
  target_role: string
  target_employer: string
  archetype: string
  status: 'draft' | 'final'
  created_at: string
  updated_at: string
}

interface ResumePerspective {
  resume_id: string
  perspective_id: string
  section: 'summary' | 'work_history' | 'projects' | 'education' | 'skills' | 'awards'
  position: number
}

interface Skill {
  id: string
  name: string
  category: string | null
}

interface PromptLog {
  id: string
  entity_type: 'bullet' | 'perspective'
  entity_id: string
  prompt_template: string
  prompt_input: string
  raw_response: string
  created_at: string
}
```

## Pagination & Result Types

```typescript
type Result<T> =
  | { ok: true; data: T }
  | { ok: false; error: ForgeError }

interface ForgeError {
  code: string
  message: string
  details?: unknown
}

interface Pagination {
  total: number
  offset: number
  limit: number
}

type PaginatedResult<T> =
  | { ok: true; data: T[]; pagination: Pagination }
  | { ok: false; error: ForgeError }

interface PaginationParams {
  offset?: number   // default 0
  limit?: number    // default 50, max 200
}

interface ReviewQueue {
  bullets: { count: number; items: BulletReviewItem[] }
  perspectives: { count: number; items: PerspectiveReviewItem[] }
}
```

## Input Types (for create/update operations)

```typescript
interface CreateSource {
  title: string
  description: string
  employer_id?: string
  project_id?: string
  start_date?: string
  end_date?: string
}

interface UpdateSource {
  title?: string
  description?: string
  employer_id?: string | null
  project_id?: string | null
  start_date?: string | null
  end_date?: string | null
}

interface DerivePerspectiveInput {
  archetype: string
  domain: string
  framing: 'accomplishment' | 'responsibility' | 'context'
}

interface CreateResume {
  name: string
  target_role: string
  target_employer: string
  archetype: string
}

interface AddResumePerspective {
  perspective_id: string
  section: string
  position: number
}

interface ReorderPerspectives {
  perspectives: Array<{
    perspective_id: string
    section: string
    position: number
  }>
}

interface RejectInput {
  rejection_reason: string
}
```

## Rich Response Types (with nested data)

```typescript
interface SourceWithBullets extends Source {
  bullet_count: number
}

interface BulletWithRelations extends Bullet {
  source: Source
  perspective_count: number
}

interface PerspectiveWithChain extends Perspective {
  bullet: Bullet
  source: Source
}

interface ResumeWithPerspectives extends Resume {
  sections: Record<string, Array<Perspective & { position: number }>>
}
```
