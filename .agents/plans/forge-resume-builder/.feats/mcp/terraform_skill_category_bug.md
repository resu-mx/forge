# Bug: Terraform skill categorized as "language" instead of "infrastructure"

## Problem

The Terraform skill (`cf5ea003-687e-4833-ac4f-def3342200c1`) has `category = 'language'`. It renders under "Languages" in the Technical Competencies section alongside Python. Terraform is an IaC tool, not a programming language — it should be `category = 'infrastructure'` or `category = 'tool'`.

## Discovered

2026-04-08 in the Federal Sales Engineer resume markdown export:
```
**Languages**: Python, Terraform   <-- wrong grouping
```

## Fix

```sql
UPDATE skills SET category = 'infrastructure' WHERE id = 'cf5ea003-687e-4833-ac4f-def3342200c1';
```
