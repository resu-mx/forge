# Entity Relationship Diagram

```
┌───────────┐     ┌───────────┐
│ employers │◄────│ projects  │
│           │ 1:N │           │
└─────┬─────┘     └─────┬─────┘
      │ 0..1             │ 0..1
      ▼                  ▼
┌─────────────────────────────┐
│          sources            │
│  status: draft|approved|    │
│          deriving           │
└──────────────┬──────────────┘
               │ 1:N (ON DELETE RESTRICT)
               ▼
┌─────────────────────────────┐     ┌──────────────────┐
│          bullets             │────►│ bullet_technologies│
│  status: draft|pending|     │ 1:N │  (technology TEXT) │
│          approved|rejected  │     └──────────────────┘
│  source_content_snapshot    │
└──────────────┬──────────────┘──────┐
               │ 1:N (RESTRICT)      │ N:M
               ▼                     ▼
┌─────────────────────────────┐  ┌────────┐
│       perspectives          │  │ skills │
│  status: draft|pending|     │  └────────┘
│          approved|rejected  │      ▲ N:M
│  bullet_content_snapshot    │──────┘
│  target_archetype           │
│  domain                     │
│  framing                    │
└──────────────┬──────────────┘
               │ N:M (via resume_perspectives)
               ▼
┌─────────────────────────────┐
│          resumes            │
│  archetype, target_role     │
│  status: draft|final        │
└─────────────────────────────┘

┌─────────────────────────────┐
│       prompt_logs           │
│  entity_type + entity_id    │◄── bullets.prompt_log_id
│  prompt_input, raw_response │◄── perspectives.prompt_log_id
└─────────────────────────────┘
```

## Cascade Rules

| Parent | Child | On Delete |
|---|---|---|
| employers | sources | SET NULL |
| employers | projects | SET NULL |
| projects | sources | SET NULL |
| sources | bullets | RESTRICT (cannot delete source with bullets) |
| bullets | perspectives | RESTRICT (cannot delete bullet with perspectives) |
| bullets | bullet_technologies | CASCADE |
| bullets | bullet_skills | CASCADE |
| perspectives | perspective_skills | CASCADE |
| perspectives | resume_perspectives | RESTRICT (cannot delete perspective in a resume) |
| resumes | resume_perspectives | CASCADE |
| prompt_logs | bullets | SET NULL |
| prompt_logs | perspectives | SET NULL |
