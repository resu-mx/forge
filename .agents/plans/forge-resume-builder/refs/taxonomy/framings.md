# Narrative Framings

Narrative framing describes the rhetorical structure of a perspective bullet. This is orthogonal to domain and archetype.

## Values

| Framing | Description | Example Pattern |
|---|---|---|
| `accomplishment` | Outcome-oriented, emphasizes results and impact | "Reduced X by Y% through Z" |
| `responsibility` | Scope-oriented, emphasizes ownership and breadth | "Owned X across Y systems serving Z users" |
| `context` | Environment-oriented, emphasizes conditions and constraints | "Operated in X environment with Y constraints" |

## Usage

- `Perspective.framing` — required, one of the above values
- Guides the AI during bullet→perspective derivation
- A single bullet can produce multiple perspectives with different framings
- Resume assembly typically mixes framings for variety (avoids uniform sentence structure — AI signal)
