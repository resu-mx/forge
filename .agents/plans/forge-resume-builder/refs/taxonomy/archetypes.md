# Archetypes

Resume archetypes define the persona/lens through which experience is presented. Seeded from the existing job-hunting system's 6 battle-tested archetypes.

## Values

| Archetype | Description | Target Roles |
|---|---|---|
| `agentic-ai` | AI/ML engineering, LLM agents, MLOps | AI Engineer, ML Engineer, Applied AI |
| `infrastructure` | Platform engineering, DevOps, cloud infra | Platform Engineer, DevOps Engineer, SRE |
| `security-engineer` | Defensive security, DFIR, cloud security | Security Engineer, Detection Engineer |
| `solutions-architect` | System design, cross-team integration | Solutions Architect, Technical Architect |
| `public-sector` | Federal/DoD experience, clearance-relevant | Federal roles, defense contractors |
| `hft` | Low-latency systems, performance engineering | Systems Engineer, Quantitative Infra |

## Usage

- `Perspective.target_archetype` — nullable, one of the above values
- `Resume.archetype` — required, determines gap analysis baseline
- Extensible: new archetypes can be added without schema migration (TEXT field, not enum constraint)

## Source

Existing system: `applications/0-archetypes/` directory with one resume template per archetype.
