# Domains

Domain framing categorizes what field/discipline the work belongs to. Orthogonal to narrative framing (accomplishment/responsibility/context) and archetype targeting.

## Values

| Domain | Description |
|---|---|
| `systems_engineering` | Infrastructure, distributed systems, platform work |
| `software_engineering` | Application development, APIs, tooling |
| `security` | Offensive/defensive security, compliance, forensics |
| `devops` | CI/CD, deployment automation, observability |
| `ai_ml` | Machine learning, LLMs, data science, MLOps |
| `leadership` | Team management, mentoring, cross-functional coordination |

## Usage

- `Perspective.domain` — nullable, one of the above values
- Used in gap analysis to determine domain coverage breadth
- Extensible: TEXT field, not enum constraint

## Source

Existing system: `bullets.framing` column in `data/schema.sql` (lines 128-129). Note the field name collision — the existing system calls this "framing" but Forge separates it into `domain` (this taxonomy) and `framing` (narrative structure).
