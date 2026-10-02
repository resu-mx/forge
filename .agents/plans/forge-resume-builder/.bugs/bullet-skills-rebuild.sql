-- Rebuild bullet_skills after migration 044 cascade wipe
-- 95 active bullets → skill links derived from bullet content
-- Skill IDs referenced inline via comments

INSERT INTO bullet_skills (bullet_id, skill_id) VALUES

-- ═══════════════════════════════════════════
-- CISCO — Senior DevOps Engineer (3 bullets)
-- ═══════════════════════════════════════════

-- e262878d: Architected cloud emulation layer...air-gapped environment simulations
('e262878d-2da5-461a-acb5-2518809717d6', '772cd89a-39be-4db2-b40c-0303d10b32cf'), -- Air-gapped Deployments
('e262878d-2da5-461a-acb5-2518809717d6', '34636cb3-8db0-4585-ae02-eb0ed05dbb9c'), -- Cloud
('e262878d-2da5-461a-acb5-2518809717d6', '6ca34f60-771a-4700-a6b3-ba07af4d4a37'), -- Cleared Environments

-- 222c882d: Kubernetes-based container orchestration using Helm...GitLab CI/CD
('222c882d-2e7b-4ad6-a454-019d16b5e02b', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes
('222c882d-2e7b-4ad6-a454-019d16b5e02b', '63648cb3-a839-44b1-8f1c-999e9f8281bb'), -- Helm
('222c882d-2e7b-4ad6-a454-019d16b5e02b', '419ba7b0-827a-4c62-b33a-d1ae0537d7e4'), -- GitLab CI/CD
('222c882d-2e7b-4ad6-a454-019d16b5e02b', 'c0708d08-4d7d-461e-b2d3-c4de14701644'), -- CI/CD

-- 30264018: IaC using Terragrunt and Terraform...ephemeral AWS environments
('30264018-b286-4760-abf2-d43b6c5f672d', '21bd8947-eebe-4534-a81f-6221fb4fb9bd'), -- Terragrunt
('30264018-b286-4760-abf2-d43b6c5f672d', 'cf5ea003-687e-4833-ac4f-def3342200c1'), -- Terraform
('30264018-b286-4760-abf2-d43b6c5f672d', '791a6433-c3ab-40f7-9606-82c2fbcccc02'), -- AWS
('30264018-b286-4760-abf2-d43b6c5f672d', 'e484bca5-31a0-4718-943c-dcc0fa2413e7'), -- IaC

-- ═══════════════════════════════════════════
-- GREYMATTER — DevOps Engineer III (14 bullets)
-- ═══════════════════════════════════════════

-- 09dff390: Analyzed customer requirements...balanced engineering solutions
('09dff390-8f4b-4e85-ab9f-be6c00a0d2a0', '4e9a0784-4709-4cdb-a559-b7c09901c8c9'), -- Solutions Architecture
('09dff390-8f4b-4e85-ab9f-be6c00a0d2a0', 'a50134a4-a7a2-4dee-abde-8b8872e29ae0'), -- Stakeholder Communication

-- 48bd3381: Architect Kubernetes-based platform for multi-cloud operations
('48bd3381-4cbe-4de6-ac86-48c1106d3316', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes
('48bd3381-4cbe-4de6-ac86-48c1106d3316', '34636cb3-8db0-4585-ae02-eb0ed05dbb9c'), -- Cloud

-- e07bae1a: Build IaC resources...web frontend
('e07bae1a-0056-44d7-ac6c-23b056cebe2f', 'e484bca5-31a0-4718-943c-dcc0fa2413e7'), -- IaC

-- 4ec469da: Code reviews of customer apps on Kubernetes...unit and integration tests
('4ec469da-40c4-4106-98a5-0bf9e1e5830b', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes
('4ec469da-40c4-4106-98a5-0bf9e1e5830b', '58ec4840-7988-4ede-9e05-94d8ff9b4b31'), -- Code Review

-- ff2df84e: Deployed and stabilized customer apps in service mesh environments
('ff2df84e-de05-49ac-86f2-b2f6f12ad81f', '0e66328b-a782-467f-bf81-cdbae070a87d'), -- Service Mesh
('ff2df84e-de05-49ac-86f2-b2f6f12ad81f', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes

-- 334a3c5c: Distributed elastic applications...incident response platform
('334a3c5c-cd62-4319-8892-5a5424fe732e', '39fe8f41-5c0d-47ff-9004-42d4ed21fb02'), -- Distributed Systems

-- d74c7c4f: 140 unit and integration tests for Kubernetes deployments
('d74c7c4f-2c0b-47b5-9443-30ecec02205e', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes

-- bb8794dd: GitHub Actions CI/CD...Docker build and push to ghcr.io
('bb8794dd-fcae-4375-9eae-cd7ab1a28b80', '3be0dde2-804b-4fb2-8aaa-3f89ed19c51d'), -- GitHub Actions
('bb8794dd-fcae-4375-9eae-cd7ab1a28b80', 'dec1325e-9b18-4aee-ab0b-7e1be50a35d8'), -- Docker
('bb8794dd-fcae-4375-9eae-cd7ab1a28b80', 'c0708d08-4d7d-461e-b2d3-c4de14701644'), -- CI/CD

-- 59d1f7cd: Linkerd service mesh...metrics...observability
('59d1f7cd-d5b7-45cf-bab9-58848bd09033', 'e6c88e4c-41bd-4a46-ba58-024626c787ee'), -- Linkerd
('59d1f7cd-d5b7-45cf-bab9-58848bd09033', '0e66328b-a782-467f-bf81-cdbae070a87d'), -- Service Mesh
('59d1f7cd-d5b7-45cf-bab9-58848bd09033', '41c85575-4781-421f-b97f-253e8003fdac'), -- Observability

-- 8852f1ad: Monitor client services...Amazon EKS
('8852f1ad-727d-4b1c-8380-3460af29958d', 'a26473b3-41f6-45aa-a905-1f253bb01a38'), -- AWS EKS
('8852f1ad-727d-4b1c-8380-3460af29958d', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes

-- bbfd655a: Stabilized customer apps in service mesh...37% decrease
('bbfd655a-c28e-4b11-a442-009de46932bd', '0e66328b-a782-467f-bf81-cdbae070a87d'), -- Service Mesh

-- 9def74f8: Tracked error-rate SLIs...containerized workloads in service mesh
('9def74f8-8181-4a2d-a980-361016379875', 'b13ab73f-c139-4602-92b0-7566cb12ace1'), -- SLO/SLI/Error Budget Engineering
('9def74f8-8181-4a2d-a980-361016379875', '0e66328b-a782-467f-bf81-cdbae070a87d'), -- Service Mesh

-- b7c5e5e7: Worked with customers to diagnose...service mesh...containerized
('b7c5e5e7-2803-4f10-add4-01e652de43fe', '0e66328b-a782-467f-bf81-cdbae070a87d'), -- Service Mesh
('b7c5e5e7-2803-4f10-add4-01e652de43fe', 'b90ab3da-8361-4baf-a107-15b5adb10ee8'), -- Customer-facing Delivery

-- ═══════════════════════════════════════════
-- LEIDOS — Network Security Analyst (11 bullets)
-- ═══════════════════════════════════════════

-- 948ebcfe: full-spectrum traffic analysis and threat hunting across 4 networks
('948ebcfe-937b-42d8-aa60-fc20bc644b98', 'c05341ba-ef1f-4cd8-9354-d067bdcc574b'), -- Traffic Analysis
('948ebcfe-937b-42d8-aa60-fc20bc644b98', 'a022331a-95db-4ff1-828b-9f687106a9ce'), -- Threat Hunting

-- b1b25804: Delivered technical briefings to mission leadership on active threats
('b1b25804-ac84-470f-9f09-bb63634247b1', '8443157d-26e7-448e-8ac6-d380d25ee096'), -- Technical Briefings

-- a4ffe6ba: Delivered technical briefings...active threats, incident findings
('a4ffe6ba-4433-4b47-bb83-b92aefa2f840', '8443157d-26e7-448e-8ac6-d380d25ee096'), -- Technical Briefings
('a4ffe6ba-4433-4b47-bb83-b92aefa2f840', '4aaf9d8b-1c00-4c9d-9a2d-ce2ca26b42c6'), -- Incident Response

-- 14c7bb2b: Delivered technical briefings...threat findings, IR outcomes
('14c7bb2b-a9de-406b-82f4-0b3a1861c79c', '8443157d-26e7-448e-8ac6-d380d25ee096'), -- Technical Briefings

-- dc1e0674: Generated cyber threat intelligence...DoD-wide dissemination
('dc1e0674-8724-461f-88c7-bd054bf83703', '12e2b306-5fde-422b-8ecc-0cbf750b1c25'), -- Cyber Threat Intelligence

-- bf02ba55: Investigated network events...true/false positive/negative
('bf02ba55-7270-42b8-b006-c090448d0c65', 'd0a9bf44-fa16-4d95-8f56-9861b0610ed6'), -- Detection Engineering

-- 9d009054: Monitor network traffic across 4 network environments
('9d009054-22a2-4ada-87d6-5688da1bd89c', 'c05341ba-ef1f-4cd8-9354-d067bdcc574b'), -- Traffic Analysis

-- eb86a5e4: Operated Splunk SIEM...2+ trillion daily events...10,000+ endpoints
('eb86a5e4-6d12-45e2-97d1-4c300efa58f5', '66a975c2-182e-4286-ad87-f9788a6940d5'), -- Splunk
('eb86a5e4-6d12-45e2-97d1-4c300efa58f5', '62b5c66a-4d97-4d38-a84f-842f63df5fe1'), -- SIEM

-- 84a4b3c3: Reverse engineered malicious code...200+ CTI reports
('84a4b3c3-8d44-4b53-83c7-fb300d234b01', '11873c62-b373-4a8f-8b01-c63b06cf9511'), -- Reverse Engineering
('84a4b3c3-8d44-4b53-83c7-fb300d234b01', '12e2b306-5fde-422b-8ecc-0cbf750b1c25'), -- Cyber Threat Intelligence
('84a4b3c3-8d44-4b53-83c7-fb300d234b01', '058ca1a5-e6ad-46d9-8875-cdc76b0d9dc9'), -- Malware Analysis

-- 4095d5bd: Triage incidents and participate in incident response
('4095d5bd-0db9-4249-b638-07744665b701', '4aaf9d8b-1c00-4c9d-9a2d-ce2ca26b42c6'), -- Incident Response

-- ═══════════════════════════════════════════
-- RAYTHEON — Cloud Forensics Engineer (3 bullets)
-- ═══════════════════════════════════════════

-- ee7415c2: technical documentation...architecture diagrams, API specifications
('ee7415c2-74c9-494a-ba04-6f9620bb5d9b', 'c55841ce-2dc4-4b91-aab3-0eaab9b12b12'), -- Technical Writing
('ee7415c2-74c9-494a-ba04-6f9620bb5d9b', 'ba1a5e1c-df98-4332-a44f-003ee994b1c6'), -- API Design

-- e436d45c: Led evaluation of 12 cloud identity and platform log analysis tools
('e436d45c-ad54-41c7-8d36-8c2c79f4ec58', 'a8fa4200-1d72-41ed-bef5-1c8fbe7e78ab'), -- Cloud Forensics
('e436d45c-ad54-41c7-8d36-8c2c79f4ec58', '8e6e9343-6fa3-4726-af5f-65ef712b8e46'), -- Technical Evaluation

-- 4156a500: scalable data ingestion and analysis platform...AWS S3, Fluvio, graph databases
('4156a500-6be0-411a-9692-5ff8c4c11959', 'b35abbbd-7fb6-4e36-b784-4bd9ee800ff5'), -- AWS S3
('4156a500-6be0-411a-9692-5ff8c4c11959', '9ec67a44-3d0b-409d-bbc4-761a431aeb68'), -- Fluvio
('4156a500-6be0-411a-9692-5ff8c4c11959', '7c79ec5e-4348-4dde-a8bf-d9d5c0c33b29'), -- Graph DBs
('4156a500-6be0-411a-9692-5ff8c4c11959', 'c0130354-fa59-4676-9565-96c5936ffd91'), -- Data Pipeline Design

-- ═══════════════════════════════════════════
-- RAYTHEON — Host Forensic Analyst (5 bullets)
-- ═══════════════════════════════════════════

-- 27628d1e: AMI hardening CI/CD pipeline using Packer, Ansible, GitLab CI/CD
('27628d1e-0c61-4472-827d-7bb42f68a383', '545db0e8-aa6e-4c94-93b2-6c5f0ad48798'), -- Packer
('27628d1e-0c61-4472-827d-7bb42f68a383', '7d6c6feb-bd13-4d9f-8831-2cc1109f48cf'), -- Ansible
('27628d1e-0c61-4472-827d-7bb42f68a383', '419ba7b0-827a-4c62-b33a-d1ae0537d7e4'), -- GitLab CI/CD
('27628d1e-0c61-4472-827d-7bb42f68a383', 'c0708d08-4d7d-461e-b2d3-c4de14701644'), -- CI/CD

-- 977b11ff: Linux log correlation tools (Python)
('977b11ff-7ca1-4d51-b892-b69d72012fbc', '820c4f0c-af30-454d-b83c-5fa587d443a1'), -- Python
('977b11ff-7ca1-4d51-b892-b69d72012fbc', '64c40426-921c-48a9-a404-38b314502781'), -- Linux

-- 38aa8e7a: Self-directed research on Kubernetes vulnerabilities, cloud
('38aa8e7a-0e1e-4a2a-852f-953d55d7451b', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes
('38aa8e7a-0e1e-4a2a-852f-953d55d7451b', '07386f87-9bba-401f-a801-1f30b137714e'), -- Research
('38aa8e7a-0e1e-4a2a-852f-953d55d7451b', 'd3520fc3-9f8c-447b-a4fa-aeed170e55c5'), -- Cloud Security

-- fd867adf: Forensic knowledge of Linux and Windows...deep-dive analysis
('fd867adf-1d6a-4c9f-a6e0-b8dfef91a3e2', '64c40426-921c-48a9-a404-38b314502781'), -- Linux
('fd867adf-1d6a-4c9f-a6e0-b8dfef91a3e2', '50af5b91-0538-4cfe-9917-7dcdf3e8976e'), -- Digital Forensics
('fd867adf-1d6a-4c9f-a6e0-b8dfef91a3e2', 'a022331a-95db-4ff1-828b-9f687106a9ce'), -- Threat Hunting

-- 0a30945c: Host-based forensic analysis...Linux and Windows...persistence mechanisms
('0a30945c-9de7-4fad-aec5-47d0ca4b208e', '50af5b91-0538-4cfe-9917-7dcdf3e8976e'), -- Digital Forensics
('0a30945c-9de7-4fad-aec5-47d0ca4b208e', '64c40426-921c-48a9-a404-38b314502781'), -- Linux

-- ═══════════════════════════════════════════
-- RAYTHEON — Principal Cloud Forensics Engineer (30 bullets)
-- ═══════════════════════════════════════════

-- 6d096a7c: AI context component library...200+ skill definitions...27 registries
('6d096a7c-62bf-4e6b-a190-6b89a2ca4058', 'ed755154-9c4c-48f6-a53d-6836f602638d'), -- Context Engineering
('6d096a7c-62bf-4e6b-a190-6b89a2ca4058', '8ca2467e-0fff-43cb-a2bb-5df8054b6f07'), -- AI Agents

-- 650c4ae6: LLM agent platform...structured prompt optimization and few-shot tuning
('650c4ae6-92a3-4658-b6c3-9419dfe86f91', 'e8ebab6c-c9be-433d-b339-bd3caceef94a'), -- LLM Agent Platforms
('650c4ae6-92a3-4658-b6c3-9419dfe86f91', '3081b4f4-bec2-49f7-aa6a-70c851af1edc'), -- Large Language Models
('650c4ae6-92a3-4658-b6c3-9419dfe86f91', '6c3b3a5f-08ad-4a2a-8073-dc69ffd72b40'), -- Prompt Engineering

-- 0cb2f57e: LLM agent platform for security workflow automation
('0cb2f57e-aea0-45b9-b2d9-7c65461ab19c', 'e8ebab6c-c9be-433d-b339-bd3caceef94a'), -- LLM Agent Platforms
('0cb2f57e-aea0-45b9-b2d9-7c65461ab19c', '6c3b3a5f-08ad-4a2a-8073-dc69ffd72b40'), -- Prompt Engineering

-- 015bffb2: LLM agent platform with MCP-compliant interfaces...TTP-to-detection-rule
('015bffb2-dda6-44dc-9ec5-27258ff8b090', 'e8ebab6c-c9be-433d-b339-bd3caceef94a'), -- LLM Agent Platforms
('015bffb2-dda6-44dc-9ec5-27258ff8b090', '0cdf99dd-9010-4e2c-9654-76abeee4bae0'), -- MCP Servers
('015bffb2-dda6-44dc-9ec5-27258ff8b090', 'd0a9bf44-fa16-4d95-8f56-9861b0610ed6'), -- Detection Engineering

-- 4fd9ac46: LLM agent platform with MCP-style tool integrations
('4fd9ac46-83c0-4529-9518-90b0b95fd134', 'e8ebab6c-c9be-433d-b339-bd3caceef94a'), -- LLM Agent Platforms
('4fd9ac46-83c0-4529-9518-90b0b95fd134', '8dbdc49b-b285-4cb3-8583-2d19e9e02e9e'), -- MCP
('4fd9ac46-83c0-4529-9518-90b0b95fd134', '0cdf99dd-9010-4e2c-9654-76abeee4bae0'), -- MCP Servers

-- 17ba507d: Automated workflow platform using Python
('17ba507d-861e-4bbd-ad5f-d84f47491562', '820c4f0c-af30-454d-b83c-5fa587d443a1'), -- Python

-- bd450ca5: Test infrastructure for MCP server...103 unit tests with pytest
('bd450ca5-a81d-4c8a-a6b8-61b0fbdeaddf', '0cdf99dd-9010-4e2c-9654-76abeee4bae0'), -- MCP Servers
('bd450ca5-a81d-4c8a-a6b8-61b0fbdeaddf', '820c4f0c-af30-454d-b83c-5fa587d443a1'), -- Python

-- 774a8bd7: End-to-end pipeline...Jupyter, Sematic, MLFlow, LakeFS, Feast, DeepChecks
('774a8bd7-53ef-4998-8ce6-c89afc58c67d', 'e6124e76-49aa-407b-be2a-8255bad3bd7a'), -- Jupyter
('774a8bd7-53ef-4998-8ce6-c89afc58c67d', '04e73e1c-5707-4cfc-9aa9-2638045c430e'), -- Sematic
('774a8bd7-53ef-4998-8ce6-c89afc58c67d', '395e128b-ece8-4820-b1cf-249a6802e185'), -- MLFlow
('774a8bd7-53ef-4998-8ce6-c89afc58c67d', 'a7569724-2aa3-4f3a-bfcc-290619792e0f'), -- LakeFS
('774a8bd7-53ef-4998-8ce6-c89afc58c67d', '04315ce3-9276-468e-a92c-f6ffdefe7a30'), -- Feast
('774a8bd7-53ef-4998-8ce6-c89afc58c67d', '75e3f3cc-4c09-479b-92ad-afa9d4135acb'), -- DeepChecks
('774a8bd7-53ef-4998-8ce6-c89afc58c67d', '332fa61c-d897-4638-9923-59757df0521b'), -- MLOps
('774a8bd7-53ef-4998-8ce6-c89afc58c67d', 'c0130354-fa59-4676-9565-96c5936ffd91'), -- Data Pipeline Design

-- 8471c6a6: Graph-based RAG memory using FalkorDB and Graphiti
('8471c6a6-5fd3-4e5a-9cf3-c22292e08f91', '074baceb-b5a8-4cf9-b542-e812a7224836'), -- FalkorDB
('8471c6a6-5fd3-4e5a-9cf3-c22292e08f91', 'e3a5912a-32be-43a0-8495-95a184b4f2df'), -- Graphiti
('8471c6a6-5fd3-4e5a-9cf3-c22292e08f91', '600c6a07-5b86-4012-bb98-c9bb9fa5efc6'), -- RAG
('8471c6a6-5fd3-4e5a-9cf3-c22292e08f91', '9133d61a-35f8-4224-add4-388f9223b796'), -- Graph Memory Systems

-- 45c0d3ab: Graph-based memory using FalkorDB and Graphiti...cross-conversation
('45c0d3ab-e83a-4946-87e3-1d94756b40dd', '074baceb-b5a8-4cf9-b542-e812a7224836'), -- FalkorDB
('45c0d3ab-e83a-4946-87e3-1d94756b40dd', 'e3a5912a-32be-43a0-8495-95a184b4f2df'), -- Graphiti
('45c0d3ab-e83a-4946-87e3-1d94756b40dd', '9133d61a-35f8-4224-add4-388f9223b796'), -- Graph Memory Systems

-- a6c1197c: Cloud-native platforms...Kubernetes, Terraform, AWS
('a6c1197c-0172-4357-8646-97387b24cd82', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes
('a6c1197c-0172-4357-8646-97387b24cd82', 'cf5ea003-687e-4833-ac4f-def3342200c1'), -- Terraform
('a6c1197c-0172-4357-8646-97387b24cd82', '791a6433-c3ab-40f7-9606-82c2fbcccc02'), -- AWS
('a6c1197c-0172-4357-8646-97387b24cd82', '34636cb3-8db0-4585-ae02-eb0ed05dbb9c'), -- Cloud

-- b1c00456: Convert legacy to cloud-native...containerization
('b1c00456-1340-4691-a3fb-9c9d7735b311', 'dec1325e-9b18-4aee-ab0b-7e1be50a35d8'), -- Docker
('b1c00456-1340-4691-a3fb-9c9d7735b311', '34636cb3-8db0-4585-ae02-eb0ed05dbb9c'), -- Cloud

-- ee5b983c: ADRs documenting HTTP transport...lazy loading patterns
('ee5b983c-19aa-4988-9bf6-d1115f9fa657', 'c55841ce-2dc4-4b91-aab3-0eaab9b12b12'), -- Technical Writing
('ee5b983c-19aa-4988-9bf6-d1115f9fa657', 'ba1a5e1c-df98-4332-a44f-003ee994b1c6'), -- API Design

-- 2471b3b9: Data ingestion platform using Fluvio, Quine, Janus Graph...ML-ready
('2471b3b9-6e24-4476-a993-386093f72b51', '9ec67a44-3d0b-409d-bbc4-761a431aeb68'), -- Fluvio
('2471b3b9-6e24-4476-a993-386093f72b51', 'b80876a6-e1e1-4616-933f-c82a42cc7583'), -- Quine
('2471b3b9-6e24-4476-a993-386093f72b51', '34401445-17d0-44c9-a6ba-c5d2f3706ff2'), -- Janus Graph
('2471b3b9-6e24-4476-a993-386093f72b51', 'c0130354-fa59-4676-9565-96c5936ffd91'), -- Data Pipeline Design

-- c6b083f1: Memory systems design...layered architecture
('c6b083f1-a67d-4bb3-aec9-1d45de16d162', '9133d61a-35f8-4224-add4-388f9223b796'), -- Graph Memory Systems
('c6b083f1-a67d-4bb3-aec9-1d45de16d162', 'ed755154-9c4c-48f6-a53d-6836f602638d'), -- Context Engineering

-- 03a58a25: Security telemetry pipeline using Fluvio, Quine, Janus Graph
('03a58a25-0cf1-485a-9d8a-dc0c74fa8272', '9ec67a44-3d0b-409d-bbc4-761a431aeb68'), -- Fluvio
('03a58a25-0cf1-485a-9d8a-dc0c74fa8272', 'b80876a6-e1e1-4616-933f-c82a42cc7583'), -- Quine
('03a58a25-0cf1-485a-9d8a-dc0c74fa8272', '34401445-17d0-44c9-a6ba-c5d2f3706ff2'), -- Janus Graph
('03a58a25-0cf1-485a-9d8a-dc0c74fa8272', 'c0130354-fa59-4676-9565-96c5936ffd91'), -- Data Pipeline Design

-- e4663bc2: Mentoring colleagues at all levels
('e4663bc2-c0d6-4c5a-8ec6-f29d7fd3c85c', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring

-- 8da73ad7: Develop, deploy, manage distributed automation tooling
('8da73ad7-0dbd-47c0-b42f-492ea20fd4c4', '39fe8f41-5c0d-47ff-9004-42d4ed21fb02'), -- Distributed Systems

-- b27bce33: Cloud threat hunting...identity graph analysis...20+ novel attack paths
('b27bce33-2b2d-4d45-84a7-f0173b421b64', 'a8fa4200-1d72-41ed-bef5-1c8fbe7e78ab'), -- Cloud Forensics
('b27bce33-2b2d-4d45-84a7-f0173b421b64', 'a022331a-95db-4ff1-828b-9f687106a9ce'), -- Threat Hunting
('b27bce33-2b2d-4d45-84a7-f0173b421b64', '7c79ec5e-4348-4dde-a8bf-d9d5c0c33b29'), -- Graph DBs

-- a020a5f9: MCP server development guide...protocol specification, transport
('a020a5f9-ad39-48e3-bf3e-84c29cd2a1cd', '0cdf99dd-9010-4e2c-9654-76abeee4bae0'), -- MCP Servers
('a020a5f9-ad39-48e3-bf3e-84c29cd2a1cd', '8dbdc49b-b285-4cb3-8583-2d19e9e02e9e'), -- MCP
('a020a5f9-ad39-48e3-bf3e-84c29cd2a1cd', 'c55841ce-2dc4-4b91-aab3-0eaab9b12b12'), -- Technical Writing

-- c52d75c3: Log correlation methodology...identity graph analysis
('c52d75c3-b380-4067-ad50-40f1c88f92a3', 'a8fa4200-1d72-41ed-bef5-1c8fbe7e78ab'), -- Cloud Forensics
('c52d75c3-b380-4067-ad50-40f1c88f92a3', '7c79ec5e-4348-4dde-a8bf-d9d5c0c33b29'), -- Graph DBs

-- d568c52a: Systematic cloud threat hunting...identity graph...platform log correlation
('d568c52a-f0b1-44f9-bd08-9c7c4aad5c99', 'a8fa4200-1d72-41ed-bef5-1c8fbe7e78ab'), -- Cloud Forensics
('d568c52a-f0b1-44f9-bd08-9c7c4aad5c99', 'a022331a-95db-4ff1-828b-9f687106a9ce'), -- Threat Hunting

-- 1fdfd72e: Technical evaluations and code reviews
('1fdfd72e-0441-4879-b907-4c803154d09b', '58ec4840-7988-4ede-9e05-94d8ff9b4b31'), -- Code Review
('1fdfd72e-0441-4879-b907-4c803154d09b', '8e6e9343-6fa3-4726-af5f-65ef712b8e46'), -- Technical Evaluation

-- 31328ab1: Cloud-native DFIR platform...Kubernetes, Terraform, AWS...80% reduction
('31328ab1-15a6-4673-b14e-3f486979b904', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes
('31328ab1-15a6-4673-b14e-3f486979b904', 'cf5ea003-687e-4833-ac4f-def3342200c1'), -- Terraform
('31328ab1-15a6-4673-b14e-3f486979b904', '791a6433-c3ab-40f7-9606-82c2fbcccc02'), -- AWS
('31328ab1-15a6-4673-b14e-3f486979b904', 'a8fa4200-1d72-41ed-bef5-1c8fbe7e78ab'), -- Cloud Forensics

-- 7d502c94: Taxonomy for AI context types...deterministic generation
('7d502c94-ba67-4e60-a957-d15cdcac1cab', 'ed755154-9c4c-48f6-a53d-6836f602638d'), -- Context Engineering

-- 5bcb97d8: Streamable HTTP transport for Zettelkasten MCP server...SDK 1.22.0
('5bcb97d8-2d01-4902-9df7-547add03eec8', '0cdf99dd-9010-4e2c-9654-76abeee4bae0'), -- MCP Servers
('5bcb97d8-2d01-4902-9df7-547add03eec8', '8dbdc49b-b285-4cb3-8583-2d19e9e02e9e'), -- MCP

-- c8ab9388: Lead project initiatives across 5 business units
('c8ab9388-c290-4459-afe1-f6c2572f341a', 'be3ad2d4-9611-40ad-9952-e233f845a9b4'), -- Project Leadership
('c8ab9388-c290-4459-afe1-f6c2572f341a', 'a50134a4-a7a2-4dee-abde-8b8872e29ae0'), -- Stakeholder Communication

-- 33237d91: Led evaluation of 12 cloud identity and platform log analysis tools
('33237d91-cf75-4788-9410-2904c2e7defb', 'a8fa4200-1d72-41ed-bef5-1c8fbe7e78ab'), -- Cloud Forensics
('33237d91-cf75-4788-9410-2904c2e7defb', '8e6e9343-6fa3-4726-af5f-65ef712b8e46'), -- Technical Evaluation

-- d2ab2d84: Led evaluation of 12 platform analysis tools...5 business units
('d2ab2d84-bcc0-48c9-a327-3f0cb3c4c345', '8e6e9343-6fa3-4726-af5f-65ef712b8e46'), -- Technical Evaluation
('d2ab2d84-bcc0-48c9-a327-3f0cb3c4c345', 'a50134a4-a7a2-4dee-abde-8b8872e29ae0'), -- Stakeholder Communication

-- e5c9f164: Kubernetes attack surfaces...16 detection rules for Splunk..."Escape to Host"
('e5c9f164-e749-49a1-8066-e3622029b157', '54d31ad4-8aff-4417-810c-219cb938644c'), -- Kubernetes
('e5c9f164-e749-49a1-8066-e3622029b157', '66a975c2-182e-4286-ad87-f9788a6940d5'), -- Splunk
('e5c9f164-e749-49a1-8066-e3622029b157', 'd0a9bf44-fa16-4d95-8f56-9861b0610ed6'), -- Detection Engineering
('e5c9f164-e749-49a1-8066-e3622029b157', '5a889cb1-7904-4c4c-9638-5daae11d8c59'), -- Container Security

-- ═══════════════════════════════════════════
-- USAF — Cyber Warfare Operator (9 bullets)
-- ═══════════════════════════════════════════

-- 1c7223d2: System hardening standards...security control baselines
('1c7223d2-3814-4a1d-9072-a6f1170b06a6', '4b5de168-b514-4db9-b494-55a868114ee1'), -- STIG Compliance

-- 75770920: Cybersecurity training range for judicial IT department
('75770920-c1fa-4429-9e16-39f87b7b1795', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring

-- 33cd32a1: Cybersecurity training range...hands-on labs
('33cd32a1-baec-440a-afa7-9d231ffaac54', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring

-- 42bec241: Cybersecurity training range with hands-on labs
('42bec241-de61-4a81-94f1-b1886cf5911a', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring

-- 8f3211ce: Chained misconfigurations across AD, network segmentation, cloud identity
('8f3211ce-f55b-45b1-835d-434b80c895fd', '1fc2bd76-6ac9-41ff-a49e-a877549329f7'), -- Red Team Operations
('8f3211ce-f55b-45b1-835d-434b80c895fd', 'c59aee16-9089-4140-b760-da8a730ead4d'), -- Penetration Testing

-- 620303ae: Red and purple team operations...vulnerability assessments
('620303ae-b2aa-4ed9-831c-db9513c57354', '1fc2bd76-6ac9-41ff-a49e-a877549329f7'), -- Red Team Operations
('620303ae-b2aa-4ed9-831c-db9513c57354', 'c59aee16-9089-4140-b760-da8a730ead4d'), -- Penetration Testing
('620303ae-b2aa-4ed9-831c-db9513c57354', 'c1176335-9460-4e10-bfc6-b7104d3c46e7'), -- Vulnerability Assessment

-- 34d8b008: Critical infrastructure (Auth, VPN, CI/CD, IaC)
('34d8b008-9ded-429e-8524-b106c396b3da', 'e484bca5-31a0-4718-943c-dcc0fa2413e7'), -- IaC
('34d8b008-9ded-429e-8524-b106c396b3da', 'c0708d08-4d7d-461e-b2d3-c4de14701644'), -- CI/CD

-- a4e9e509: Mentored junior operators on offensive tooling
('a4e9e509-5d37-4518-9a5c-19be361ac84f', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring
('a4e9e509-5d37-4518-9a5c-19be361ac84f', '1fc2bd76-6ac9-41ff-a49e-a877549329f7'), -- Red Team Operations

-- 85cbf517: Red-team infrastructure for full-scale penetration testing
('85cbf517-d33b-46eb-a5d8-fec4467e556d', '1fc2bd76-6ac9-41ff-a49e-a877549329f7'), -- Red Team Operations
('85cbf517-d33b-46eb-a5d8-fec4467e556d', 'c59aee16-9089-4140-b760-da8a730ead4d'), -- Penetration Testing

-- ═══════════════════════════════════════════
-- USMC — SIGINT Analyst (6 bullets)
-- ═══════════════════════════════════════════

-- cfc0b9f9: Gear accountability
('cfc0b9f9-0d3c-4312-a70e-6068ae395cfb', 'd2d6c4fe-a477-4a98-b03a-6baf2c47c7d2'), -- Military Service

-- 2500319d: Information Assurance
-- (no strong skill match — IA is too general)

-- a799eead: 1st tier maintenance on gear
('a799eead-5a5f-4e62-a222-a203f49e8909', 'd2d6c4fe-a477-4a98-b03a-6baf2c47c7d2'), -- Military Service

-- 055d29ab: Cellular and wifi signal collections...intelligence operations
('055d29ab-3913-4fc4-93bd-f17eee74555c', 'c05341ba-ef1f-4cd8-9354-d067bdcc574b'), -- Traffic Analysis

-- 0c15ea41: Prepared gear for deployment to field environment
('0c15ea41-14ce-4f16-8f93-11bd4a1712aa', 'd2d6c4fe-a477-4a98-b03a-6baf2c47c7d2'), -- Military Service

-- a657204c: Guidance and training to peers
('a657204c-9286-450f-b844-aaaa9f0ef735', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring

-- ═══════════════════════════════════════════
-- ACI FEDERAL — Systems Administrator (3 bullets)
-- ═══════════════════════════════════════════

-- c182a5c3: Remote troubleshooting of customer computers and networks
('c182a5c3-1f3e-4fae-8f21-ac3df4bf5131', 'b90ab3da-8361-4baf-a107-15b5adb10ee8'), -- Customer-facing Delivery

-- 33472b70: Tier 1-3 cases
('33472b70-8412-4433-8589-bc87504b907e', '4aaf9d8b-1c00-4c9d-9a2d-ce2ca26b42c6'), -- Incident Response

-- f8fec576: PowerShell scripts...Active Directory accounts
('f8fec576-746f-469e-b648-ae9d72539f61', '15d6b8e5-2826-44ad-9867-4ad0d2f148ed'), -- PowerShell
('f8fec576-746f-469e-b648-ae9d72539f61', 'c9e47317-a3e9-4be7-a497-286ae9a2ab23'), -- LDAP

-- ═══════════════════════════════════════════
-- AD ASTRA — Sr Cloud Specialist (3 bullets)
-- ═══════════════════════════════════════════

-- fd195d5c: VPN tunnels, SQL databases, remote database connections, IIS
('fd195d5c-fb5f-4d0f-9238-3e1e0cba1732', '959acf67-065c-4713-94fe-fe76e8e1ce0b'), -- SQL
('fd195d5c-fb5f-4d0f-9238-3e1e0cba1732', '34636cb3-8db0-4585-ae02-eb0ed05dbb9c'), -- Cloud

-- f9478694: Installed client software in cloud-based deployments
('f9478694-e83a-479d-bb81-fab2b5226497', '34636cb3-8db0-4585-ae02-eb0ed05dbb9c'), -- Cloud

-- a2c05368: Migrated from on-premises to cloud-hosted
('a2c05368-13f3-44a8-bb2a-ffdba751c7a8', '34636cb3-8db0-4585-ae02-eb0ed05dbb9c'), -- Cloud

-- ═══════════════════════════════════════════
-- 2U / TRILOGY — Teaching Assistant (3 bullets, shared)
-- ═══════════════════════════════════════════

-- ef3b2e37: Enable students learning process
('ef3b2e37-dcef-494f-ae3c-4af29780f363', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring

-- 0bf34f02: Grade student homework...detailed feedback
('0bf34f02-262d-4505-bbf5-18ff8178d8a3', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring

-- e51df3ff: Provide support to the instructor
('e51df3ff-a4a0-4f73-acbc-1c53fbd1a789', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring

-- ═══════════════════════════════════════════
-- SPRINGBOARD — Cyber Security Career Track Mentor (2 bullets)
-- ═══════════════════════════════════════════

-- 4ebd57b9: Setting and maintaining goals
('4ebd57b9-5168-40ec-8d8e-fb8da44f22d7', 'a5c90a43-2cfc-4418-953e-d1648204ce07'), -- Mentoring

-- bd25a5f7: Targeted responses for students queries
('bd25a5f7-276a-4345-b131-6a95d72d85be', 'a5c90a43-2cfc-4418-953e-d1648204ce07') -- Mentoring

; -- END

-- NOTE: 1 bullet skipped (no strong skill match):
--   2500319d "Helped ensure proper Information Assurance" — too general
