# Bullet Inventory Dedup Analysis

**Date**: 2026-04-06
**Total bullets**: 143
**Proposed removals**: 53
**Bullets after dedup**: ~90

---

## Cluster 1: Cloud emulation / air-gapped environments (Senior DevOps role)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `e262878d` | Architected cloud emulation layer providing secure development access to air-gapped environment simulations, reducing access bottlenecks for distributed teams |
| REMOVE | `41ab0188` | *(exact same text)* |
| REMOVE | `c87ddea1` | Reduced access bottlenecks for distributed teams via cloud-based air-gapped environment simulations *(weaker subset)* |

---

## Cluster 2: IaC / Terraform (Senior DevOps)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `30264018` | Implemented infrastructure-as-code using Terragrunt and Terraform to provision ephemeral, isolated AWS environments with automated compliance validation |
| REMOVE | `77b8e797` | Automated compliance validation for ephemeral AWS environments provisioned via infrastructure-as-code *(subset)* |

---

## Cluster 3: K8s + GitLab CI/CD (Senior DevOps)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `222c882d` | Designed and deployed Kubernetes-based container orchestration using Helm for reproducible, isolated development environments with GitLab CI/CD integration |
| REMOVE | `bd0bcdb0` | Integrated GitLab CI/CD with Kubernetes-based container orchestration for development environments *(subset)* |

---

## Cluster 4: RAG memory / FalkorDB / Graphiti (6 bullets, keep 2)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `14d62ff4` | Architected graph-based memory system using FalkorDB and Graphiti...solving domain knowledge retention problem for teams losing SMEs...team of 10 *(best: has business impact)* |
| **KEEP** | `45c0d3ab` | Designed graph-based RAG memory system...evaluated graph-based, GraphRAG, and vector retrieval approaches *(best technical detail)* |
| REMOVE | `9602611d` | Built graph-based RAG memory system... *(near-identical to 45c0d3ab)* |
| REMOVE | `2235e933` | Designed graph-based RAG memory system... *(near-identical to 45c0d3ab)* |
| REMOVE | `df3903d7` | Researched and iteratively implemented agentic platform infrastructure... *(vaguer version)* |
| REMOVE | `b70e9dd2` | Responsible for researching and iteratively implementing agentic platform... *(Indeed version, weakest)* |

---

## Cluster 5: Data ingestion platform / Fluvio / Quine (4 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `4156a500` | Partnered with cross-functional units to design and implement a scalable data ingestion and analysis platform leveraging AWS S3, Fluvio, Quine, OpenMetaData, CockroachDB, and Janus Graph...*(most complete)* |
| REMOVE | `9f97404c` | Created scalable data ingestion platform using Fluvio, Quine, and Janus Graph... |
| REMOVE | `19910552` | Built data ingestion and analysis platform using AWS S3, Fluvio, Quine... |
| REMOVE | `2eef9392` | Designed and implemented scalable data ingestion and analysis platform... *(reframed variant)* |

---

## Cluster 6: MLOps pipeline (2 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `e7122592` | Built end-to-end MLOps pipeline integrating Jupyter, Sematic, OpenMetadata, MLFlow, LakeFS, Feast, HuggingFace, and DeepChecks...*(more complete tool list)* |
| REMOVE | `763af83e` | Built end-to-end MLOps pipeline integrating Jupyter, Sematic, MLFlow, LakeFS, Feast, and DeepChecks... *(subset of tools)* |

---

## Cluster 7: AI skill ecosystem analysis (exact duplicate)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `76d4235d` | Conducted systematic analysis of AI skill ecosystem, cataloging 200+ skills across 27 external registries and 30+ functional categories... |
| REMOVE | `b5aa6e86` | *(exact same text)* |

---

## Cluster 8: 13-phase MCP implementation (exact duplicate)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `2c1e88a6` | Executed 13-phase feature implementation for MCP server HTTP transport... |
| REMOVE | `3f0cc7e7` | *(exact same text)* |

---

## Cluster 9: MCP Streamable HTTP transport (2 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `5bcb97d8` | Implemented Streamable HTTP transport for Zettelkasten MCP server...delivered 103 unit tests, Docker deployment, and CI/CD pipelines *(more concrete deliverables)* |
| REMOVE | `a436df2d` | Contributed Streamable HTTP transport implementation... *(weaker verb, overlapping)* |

---

## Cluster 10: AI context component library (3 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `6d096a7c` | Architected AI context component library with 200+ reusable skill definitions across 27 skill registries...*(most specific with numbers)* |
| REMOVE | `e8276759` | Developed reusable AI context component library with deterministic generation framework... |
| REMOVE | `7d502c94` | Established taxonomy for AI context types (memories, rules, patterns, project/team/personal context)... |

**QUESTION**: `7d502c94` (taxonomy angle) is a different framing from `6d096a7c` (200+ skills). Worth keeping both?

---

## Cluster 11: LLM agent / TTP detection (5 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `4fd9ac46` | Built and deployed LLM agent platform with MCP-style tool integrations for automated TTP-to-detection-rule generation; evaluated model outputs across accuracy, token efficiency, and uncertainty metrics *(most complete)* |
| REMOVE | `8613d6a2` | Built and deployed LLM agent platform for automated TTP-to-detection-rule generation... |
| REMOVE | `82bee4ac` | Built and deployed LLM agent platform for automated security workflow generation... |
| REMOVE | `8a22835b` | Built LLM agent platform for automated TTP-to-detection-rule generation... |
| REMOVE | `f3295e54` | Built LLM agent platform for automated security workflow generation... |

---

## Cluster 12: K8s detection rules / T1611 (exact duplicate)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `e5c9f164` | Researched Kubernetes attack surfaces and container escape techniques; developed 16 new detection rules for Splunk targeting T1611 |
| REMOVE | `74482eae` | *(exact same text)* |

---

## Cluster 13: Cloud-native DFIR platform (4 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `a6c1197c` | Collaboratively engineered and deployed cloud-native platforms utilizing Kubernetes, Terraform, and AWS...400% reduction in MTTD and saving >400 man hours *(strongest metrics)* |
| REMOVE | `15b850e8` | Engineered cloud-native platform using K8s, Terraform, AWS; reduced MTTD by 80% |
| REMOVE | `ef68789e` | Engineered cloud-native DFIR platform... *(near-identical to 15b850e8)* |
| REMOVE | `f2352155` | Lead design and development of declarative distributed systems... *(Indeed version)* |

---

## Cluster 14: Cybersecurity training range (5 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `33cd32a1` | Architected cybersecurity training range for judicial IT department; sat with end users to build hands-on labs, iterating exercises based on what actually stuck *(most human/concrete)* |
| REMOVE | `6c6653ba` | Architected cybersecurity training range...wrote training materials... |
| REMOVE | `cd1a4765` | Architected cybersecurity training range...translated complex security concepts... |
| REMOVE | `659ce2c7` | Built cybersecurity training range...wrote training materials... |
| REMOVE | `292a5beb` | Architected and led the implementation of a nationally impactful cybersecurity training range... |

---

## Cluster 15: Technical briefings (4 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `a4ffe6ba` | Delivered technical briefings to mission leadership on active threats, incident findings, and capability recommendations *(most complete)* |
| REMOVE | `99cc1d9a` | Delivered technical briefings to mission leadership on threat findings... |
| REMOVE | `22211c78` | Delivered technical briefings to mission leadership on threat landscape... |
| REMOVE | `01a16ebe` | Delivered technical briefings to DoD mission leadership... |

---

## Cluster 16: Red team / offensive ops (6 bullets, keep 2)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `85cbf517` | Planned, designed, and implemented red-team infrastructure...secure access plane, ephemeral proxy plane, and ChatOps-centric C2 *(best technical detail)* |
| **KEEP** | `620303ae` | Conducted authorized red and purple team operations...delivered vulnerability assessments and remediation recommendations *(best outcome-focused)* |
| REMOVE | `c4489e86` | Conducted authorized red team operations against strategic partner infrastructure... |
| REMOVE | `02c558d6` | Planned, designed, implemented and codified complex red-team infrastructure... *(original source version)* |
| REMOVE | `faeec5fc` | Planned, organized, prepared for, and carried out authorized offensive cyber effects... |

**QUESTION**: `8f3211ce` — "Chained misconfigurations across Active Directory, network segmentation, and cloud identity boundaries to demonstrate lateral movement paths in classified environments" — this is a distinct technique demonstration. Keep or remove?

---

## Cluster 17: Field signal collections / SIGINT (4 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `055d29ab` | Performed field-based cellular and wifi signal collections supporting intelligence operations in contested environments *(most specific)* |
| REMOVE | `67ee3062` | Performed field-based signal collections and analysis... |
| REMOVE | `d7b51a5b` | Performed field-based cellular and wifi signal collections... |
| REMOVE | `2a5f9416` | Performed operations in field environments... *(Indeed original, vaguest)* |

---

## Cluster 18: Service mesh stability / 37% error decrease (4 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `ff2df84e` | Deployed and stabilized customer applications in service mesh environments through embedded on-site support and continuous fault testing; 37% decrease in error rates *(adds "embedded on-site")* |
| REMOVE | `d0bdb74e` | Stabilized customer applications in service mesh environments... |
| REMOVE | `1350df7b` | Stabilized customer-facing applications in service mesh environments... |
| REMOVE | `f7df9a0a` | Contributed to the improvement of the stability of customer applications... *(original source, wordier)* |

---

## Cluster 19: Linux log correlation tools (3 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `977b11ff` | Developed internal Linux log correlation tools (Python) that automated identification of related events, reducing analysis time by approximately 25% *(has metric + Python)* |
| REMOVE | `24394eed` | Developed internal Linux log correlation tools (Python) automating identification... *(no metric)* |
| REMOVE | `827293f0` | Significantly enhanced the forensic investigation process by developing internal Linux log correlation tools...~25% *(wordier original)* |

---

## Cluster 20: AMI hardening pipeline (2 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `27628d1e` | Developed an AMI hardening CI/CD pipeline using Packer, Ansible, and GitLab CI/CD...99% build success rate *(has metric)* |
| REMOVE | `6306d509` | Automated the implementation of AMI hardening strategies... *(no metric)* |

---

## Cluster 21: Splunk monitoring (3 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `eb86a5e4` | Operated Splunk SIEM with visibility into 2+ trillion daily events; directly monitored 10,000+ endpoints *(combines both metrics)* |
| REMOVE | `4b971c86` | Utilized Splunk to perform active monitoring of 10,000+ endpoints *(subset)* |
| REMOVE | `81a7b563` | Utilized Splunk to monitor over 2 trillion daily events *(subset)* |

---

## Cluster 22: Reverse engineering / CTI (2 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `84a4b3c3` | Reverse engineered malicious code samples; produced 200+ cyber threat intelligence reports disseminated DoD-wide *(has metric)* |
| REMOVE | `0c31613f` | Reverse engineered malicious code samples to aid in attribution |

---

## Cluster 23: Traffic analysis / threat hunting (2 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `948ebcfe` | Conducted live full-spectrum traffic analysis and threat hunting across 4 network environments in DoD enterprise *(has quantifier)* |
| REMOVE | `fbee5ad6` | Conducted live full spectrum traffic analysis and threat hunting *(no detail)* |

---

## Cluster 24: Cloud threat hunting methodology (2 bullets, keep 1)

| Action | ID | Content |
|--------|----|---------|
| **KEEP** | `d568c52a` | Developed systematic cloud threat hunting methodology combining identity graph analysis with platform log correlation; documented 20+ novel attack patterns *(more specific)* |
| REMOVE | `22c4d670` | Proactively hunted for threats across customer cloud infrastructure... *(wordier, less specific)* |

---

## Open Questions

1. **Cluster 4 (RAG memory)** — Keeping 2 versions (business-impact + technical). Want just 1?
2. **Cluster 10 (AI context library)** — `7d502c94` (taxonomy) is a different angle from `6d096a7c` (200+ skills). Worth keeping both?
3. **Cluster 16 (Red team)** — `8f3211ce` (AD lateral movement chains) is a distinct technique. Keep or remove?
