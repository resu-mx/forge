# Snorkel AI Resume Alignment Analysis

**Resume**: `32acab19` — Applied Research Engineer – Training Infra
**JD**: GPU clusters, distributed training, K8s/Slurm orchestration, ML experiment tracking, dataset versioning, Python

---

## Problem 1: Entries referencing rejected bullets

These resume entries point to bullets we just rejected in dedup. They need to be removed or swapped to the kept version.

| Entry ID | Current (rejected bullet) | Action | Replacement |
|----------|---------------------------|--------|-------------|
| `a7b702c9` | K8s T1611 rules (74482eae) | SWAP to kept bullet e5c9f164 OR remove | See below |
| `e85b105e` | MLOps pipeline (763af83e) | SWAP to e7122592 (more complete) | Yes — directly relevant |
| `1af90c73` | Data ingestion (9f97404c) | SWAP to 4156a500 (most complete) | Yes — relevant |
| `2523609d` | LLM agent (8613d6a2) | SWAP to 4fd9ac46 (most complete) | Yes — relevant |
| `c1579503` | RAG memory (9602611d) | REMOVE — duplicate of 4266798a | |
| `5ccb5319` | RAG memory (2235e933) | REMOVE — duplicate of 4266798a | |
| `9b8b20e4` | DFIR platform (15b850e8) | SWAP to a6c1197c (stronger metrics) | Yes — shows K8s+AWS at scale |
| `61f60127` | Training range (6c6653ba) | REMOVE — weak fit for Snorkel | |
| `0df2692f` | Red team ops (c4489e86) | REMOVE — weak fit for Snorkel | |
| `c9243f3f` | Service mesh (d0bdb74e) | REMOVE — duplicate of 7f672c51 (ff2df84e) | |
| `48d7cc4a` | Data ingestion (19910552) | REMOVE — duplicate (4156a500 replaces) | |

---

## Problem 2: Weak-fit entries for Training Infra role

These are currently on the resume but don't connect to ML training infrastructure:

| Entry | Content | Issue |
|-------|---------|-------|
| Raytheon: K8s T1611 | Detection rules for Splunk | Pure security, no ML angle |
| Raytheon: Cloud identity tools | 12 log analysis tools | Security tool evaluation, not ML |
| USAF: Training range | Judicial IT cybersecurity training | Irrelevant to ML infra |
| USAF: Red team ops | Container/network bypass | Pure offensive security |
| Leidos: Splunk SIEM | 2T daily events, 10k endpoints | SOC work, not ML |
| Greymatter: Service mesh (dup) | 37% error rate decrease (duplicate) | Already have one, remove dup |
| Greymatter: Data ingestion (dup) | Fluvio/Quine (duplicate) | Already have one, remove dup |

---

## Problem 3: What SHOULD be on this resume

### Strong-fit bullets (keep or add):
1. **MLOps pipeline** (e7122592) — Jupyter, Sematic, MLFlow, LakeFS, Feast, HuggingFace, DeepChecks → directly maps to "experiment tracking, dataset versioning, model artifact management"
2. **Data ingestion platform** (4156a500) — streaming pipelines, lineage tracking → data infrastructure
3. **RAG memory system** (45c0d3ab) — ML system design, evaluation methodology
4. **K8s container orchestration** (222c882d) — Helm, GitLab CI/CD → "job orchestration using Kubernetes"
5. **IaC Terraform/AWS** (30264018) — ephemeral environments → "GPU cluster infrastructure on cloud"
6. **Cloud-native DFIR platform** (a6c1197c) — K8s + Terraform + AWS, 400% MTTD improvement → shows cloud-native platform building at scale
7. **Agent experimentation platform** (66b8b112) — A/B testing, accuracy/efficiency metrics → "experiment tracking" parallel
8. **Service mesh stability** (ff2df84e) — distributed systems reliability, fault testing → "monitor cluster health, fault tolerance"
9. **LLM agent platform** (4fd9ac46) — model evaluation, prompt optimization → shows ML evaluation skills
10. **Cloud emulation layer** (e262878d) — cloud infra for distributed teams → infrastructure at scale
11. **AMI hardening pipeline** (27628d1e) — Packer, Ansible, CI/CD → automation/reproducibility
12. **140 K8s tests** (d74c7c4f) — config drift prevention → infrastructure reliability

### Missing from inventory (gaps vs JD):
- GPU cluster management (no bullets)
- Distributed training concepts (parallelism strategies, memory optimization) (no bullets)
- AWS HyperPod (no bullets)
- Slurm job scheduling (no bullets)
- Post-training workflows (SFT, RLHF) (no bullets — but presentation on LLM backdoors touches this tangentially)

---

## Problem 4: Summary needs reframing

**Current**: "AI solutions architect with hands-on experience designing and deploying LLM agent systems, GenAI applications, and agentic orchestration frameworks in classified environments..."

**Issues**: Heavy clearance/DoD framing, "agentic orchestration" doesn't map to training infra, no mention of infrastructure/platform work

**Should emphasize**: Infrastructure engineer who builds ML training platforms, experiment tracking, K8s orchestration, data pipelines, Python

---

## Problem 5: Skills section misaligned

**Current categories**: ai_ml, data_systems, infrastructure, language, methodology

**Issues**:
- "Dataset Discovery/Refinement/Collection/Cleaning" is granular noise — condense
- Missing: MLFlow, Sematic, Docker, Terraform, AWS (EKS/SageMaker), Ansible, Packer
- "DevSecOps" and "SAFe" are irrelevant for this role
- "Engram" is too niche
- Should add: distributed systems, CI/CD, IaC, observability

---

## Proposed Action Plan

1. Remove 7 weak-fit/duplicate entries
2. Swap 4 entries to their deduped-best versions
3. Add ~3-4 strong-fit entries not currently on resume
4. Rewrite summary for training infra focus
5. Realign skills section
6. Reorder entries by relevance to JD
