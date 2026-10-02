# Next Session Prompt

Paste this to start the next session:

---

Forge data quality audit continuation. Check memory for `project_data_quality_2026_04_07.md` for full context on what was done last session.

## Priority order for this session

### 1. MCP Bullet Tools (job-hunting-e35)
Implement `forge_create_bullet`, `forge_add_bullet_skill`, and `forge_remove_bullet_skill` MCP tools. See the beads issue for the full spec. This unblocks collaborative bullet creation workflows that currently require raw SQL. While you're in the MCP package, also fix the void-return serialization bug in `forge_remove_resume_skill` and `forge_remove_resume_entry`.

### 2. Cross-Entity Taxonomy Alignment (job-hunting-dr4)
This is the big sweep. Key inputs from the 94g skills taxonomy audit:
- **112 orphan skills** have no bullet links (57% of all skills). Decide: add bullet links where appropriate, or mark as aspirational.
- **7 resume skills lack bullet backing**: Go, Rust, TypeScript, Bash, PostgreSQL, Neo4J, Agile — these appear on resumes but have no bullet evidence. At minimum, add bullet_skills links where bullets clearly use these technologies.
- **Cloud/AWS naming tangle**: "AWS" vs "AWS EKS" vs "Cloud" vs "Cloud (AWS, Azure, GCP)" vs "GCP" vs "Azure" — propose a cleanup strategy.
- **Duplicate skills to resolve**: Elasticsearch/Elastic, RAG/RAG Pipelines
- **Programming language bullet tagging**: only Python, PowerShell, SQL, Terraform, Terragrunt have bullet links. Core languages (Go, Rust, TypeScript, Bash, C, C++, Java, JavaScript) need tagging where bullets clearly use them.
- **Classical ML skills** (Deep Learning, PyTorch, TensorFlow, Fine-tuning) have 0 bullet links despite being in the DB.
- **Empty-archetype resumes**: "Anthropic Public Sector" and "Anthropic Sandboxing" have blank archetype values.
- Work collaboratively — propose changes in batches, wait for approval before executing.

### 3. Projects Audit (job-hunting-oy2)
Interview me about project descriptions. Many projects have thin descriptions. The projects include Sanskrit, Bespin, LLM Agent Experiment Platform, Forge, Domain Hunter, Goose, and others. Propose what to expand, ask targeted questions.

### Also open but lower priority
- **job-hunting-8kd**: Extend perspectives to cover project sources (feature design)
- Presentation bullet synthetic ID bug (code fix in compiler)
- `edu_description` for all 6 education entries



CLAUDE_CODE_ENABLE_TELEMETRY=1 \
OTEL_LOG_USER_PROMPTS=1 \
OTEL_LOG_TOOL_DETAILS=1 \
OTEL_LOG_TOOL_CONTENT=1 \
OTEL_METRICS_INCLUDE_ACCOUNT_UUID=1 \
OTEL_METRICS_INCLUDE_SESSION_ID=1 \
OTEL_METRICS_INCLUDE_VERSION=1 \
OTEL_RESOURCE_ATTRIBUTES="department=engineering,team.id=platform,cost_center=eng-123" \
OTEL_METRICS_EXPORTER=otlp \
OTEL_METRIC_EXPORT_INTERVAL=1000 \
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
OTEL_EXPORTER_OTLP_PROTOCOL=otlp \
OTEL_EXPORTER_OTLP_HEADERS="foo=bar" \
CLAUDE_CODE_EFFORT_LEVEL=low \



OTEL_LOGS_EXPORTER=otlp \
OTEL_LOGS_EXPORT_INTERVAL=1000 \
OTEL_TRACES_EXPORTER=otlp \
OTEL_TRACES_EXPORT_INTERVAL=1000 \
claude --model 'haiku' --print "what is 2+2"


https://code.claude.com/docs/en/monitoring-usage
https://code.claude.com/docs/en/env-vars
