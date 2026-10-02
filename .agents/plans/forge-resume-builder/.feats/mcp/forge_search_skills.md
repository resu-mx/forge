# Feature: forge_search_skills

## Problem

No MCP tool exists to search or list skills. When building a resume's Technical Competencies section, you need to find skill UUIDs to pass to `forge_add_resume_skill`. Currently requires direct DB queries (`SELECT id, name, category FROM skills WHERE name IN (...)`).

## Discovered

2026-04-08 while building the Federal Sales Engineer resume. Needed to find IDs for Docker, Terraform, Helm, Splunk, CI/CD, DevSecOps, Service Mesh. Had to fall back to direct SQLite queries.

## Proposed Tool

```ts
forge_search_skills({
  search?: string,     // Full-text search on skill name
  category?: string,   // Filter by category (language, platform, infrastructure, security, etc.)
  limit?: number,      // Pagination
  offset?: number,
})
```

Returns: `{ data: [{ id, name, category, notes }], pagination: { total, offset, limit } }`
