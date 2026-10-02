# AI Output Validation Strategy

## Overview

All AI-generated output must be validated before persisting to the database. Invalid output is never partially saved.

## Validation Pipeline

```
Claude Code CLI response
  │
  ├─ Step 1: Parse JSON
  │   └─ Failure → AI_ERROR: "Malformed JSON response"
  │
  ├─ Step 2: Schema validation
  │   └─ Failure → AI_ERROR: "Response missing required fields"
  │
  ├─ Step 3: Content provenance check
  │   └─ Warning → logged, but not blocking (heuristic)
  │
  └─ Step 4: Persist
      └─ All bullets/perspectives created in pending_review status
```

## Step 1: JSON Parsing

Claude Code CLI with `--output-format json` should return valid JSON. If not:
- Log the raw response to prompt_logs for debugging
- Return `{ code: "AI_ERROR", message: "Failed to parse AI response as JSON" }`
- No records created

## Step 2: Schema Validation

**For source→bullet derivation, expected schema:**
```typescript
interface BulletDerivationResponse {
  bullets: Array<{
    content: string          // required, non-empty
    technologies: string[]   // required, can be empty array
    metrics: string | null   // optional
  }>
}
```

**For bullet→perspective derivation, expected schema:**
```typescript
interface PerspectiveDerivationResponse {
  content: string     // required, non-empty
  reasoning: string   // required (logged, not shown to user)
}
```

Validation checks:
- All required fields present
- `content` is non-empty string
- `technologies` is array of strings (if present)
- No unexpected fields (warn but don't block)

## Step 3: Content Provenance Check (Heuristic)

Compare AI output against input to flag potential hallucination:
- Extract proper nouns from output not present in input → warning
- Extract technology names from output not present in input → warning
- Check for quantitative claims in output not present in input → warning

This is a heuristic, not a hard gate. Warnings are logged in the prompt_log alongside the response. The human review step is the real hallucination gate.

## Step 4: Persist

On validation success:
1. Create PromptLog entry with full prompt, raw response, and any warnings
2. Create bullet/perspective entities in `pending_review` status
3. Set content snapshots from the source/bullet input
4. Associate technologies via junction table (for bullets)

## Failure Recovery

If any step fails after the `deriving` lock was set:
1. Reset source/bullet status from `deriving` to its previous status
2. No partial records in the database
3. Return typed error to the caller

This is wrapped in a SQLite transaction — either everything commits or nothing does.
