# Prompt Template: Bullet → Perspective Derivation

## Template

```
You are a resume content assistant. Given a factual bullet point, reframe it
for a target role archetype. The reframing must:
- Only use facts present in the original bullet
- Emphasize aspects relevant to the target archetype
- NOT add claims, technologies, outcomes, or context not in the bullet
- Use active voice, concise phrasing

Original bullet:
---
{bullet.content}
Technologies: {bullet.technologies}
Metrics: {bullet.metrics}
---

Target archetype: {archetype}
Target domain: {domain}
Framing style: {framing} (accomplishment | responsibility | context)

Respond with a JSON object:
{
  "content": "reframed bullet text",
  "reasoning": "brief explanation of what was emphasized and why"
}
```

## Example Input

```
Original bullet:
---
Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch over 6 months
Technologies: ELK, AWS OpenSearch
Metrics: 4 engineers, 6 months
---

Target archetype: agentic-ai
Target domain: ai_ml
Framing style: accomplishment
```

## Example Output

```json
{
  "content": "Led cloud platform migration enabling ML-based log analysis pipeline on AWS OpenSearch",
  "reasoning": "Emphasized the ML/analytics enablement aspect of the migration (OpenSearch supports ML features) and the cloud platform angle relevant to AI infrastructure. Kept the 'led' verb and platform migration fact. Did not add specific ML tools not mentioned in the original."
}
```

## Multiple Perspectives from One Bullet

The same bullet can produce different perspectives by varying archetype/domain/framing:

| Archetype | Domain | Framing | Result |
|---|---|---|---|
| `agentic-ai` | `ai_ml` | `accomplishment` | "Led cloud platform migration enabling ML-based log analysis pipeline on AWS OpenSearch" |
| `infrastructure` | `devops` | `accomplishment` | "Led 4-engineer team delivering 6-month cloud forensics platform migration from ELK to AWS OpenSearch" |
| `security-engineer` | `security` | `accomplishment` | "Migrated cloud forensics platform to AWS OpenSearch, establishing scalable security log analysis" |
| `infrastructure` | `systems_engineering` | `responsibility` | "Owned cloud forensics platform architecture across ELK-to-OpenSearch migration for 4-person team" |

## Notes

- `reasoning` is logged but not shown to the user by default — exists for debugging prompt quality
- Template version tracked as "bullet-to-perspective-v1" in prompt_logs
