-- Rebuild skill_domains after migration 044 cascade wipe
-- Domain IDs:
--   ai_ml:        d0000001-0000-4000-8000-000000000005
--   architecture:  da34fc1d-d93f-4cfa-bf46-a82c4c6c853c
--   data_science:  b0a2874e-0bbb-4b6f-b167-08a7be0c9a14
--   devops:        d0000001-0000-4000-8000-000000000004
--   engineering:   c41655ca-19f2-4d22-b41c-cec433244e5b
--   leadership:    d0000001-0000-4000-8000-000000000006
--   operations:    3329fa0f-0fd0-4569-a25f-0fcb85010da5
--   research:      e779c920-37dc-4505-bd62-7c55150f83b7
--   security:      d0000001-0000-4000-8000-000000000003
--   software:      d0000001-0000-4000-8000-000000000002
--   systems:       d0000001-0000-4000-8000-000000000001

INSERT INTO skill_domains (skill_id, domain_id) VALUES

-- ═══════════════════════════════════════════
-- CATEGORY: ai_ml  (23 skills)
-- ═══════════════════════════════════════════
-- Agent Architectures → ai_ml, software
('f7ae5430-079c-440c-a88a-a2d60ad787dc', 'd0000001-0000-4000-8000-000000000005'),
('f7ae5430-079c-440c-a88a-a2d60ad787dc', 'd0000001-0000-4000-8000-000000000002'),
-- Context Engineering → ai_ml
('ed755154-9c4c-48f6-a53d-6836f602638d', 'd0000001-0000-4000-8000-000000000005'),
-- Dataset Cleaning → ai_ml, data_science
('bea892d9-8985-4d7b-92ef-220c68bc6cb0', 'd0000001-0000-4000-8000-000000000005'),
('bea892d9-8985-4d7b-92ef-220c68bc6cb0', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Dataset Collection → ai_ml, data_science
('e3a35eac-bb68-4e82-bda6-c0da1bdcfb58', 'd0000001-0000-4000-8000-000000000005'),
('e3a35eac-bb68-4e82-bda6-c0da1bdcfb58', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Dataset Discovery → ai_ml, data_science
('098d6e36-6669-4ed5-8c89-2c06c9262529', 'd0000001-0000-4000-8000-000000000005'),
('098d6e36-6669-4ed5-8c89-2c06c9262529', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Dataset Refinement → ai_ml, data_science
('b2e67a6a-a82a-4651-ae82-9b60c6dedcaa', 'd0000001-0000-4000-8000-000000000005'),
('b2e67a6a-a82a-4651-ae82-9b60c6dedcaa', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Deep Learning → ai_ml, research
('701d1dc4-31d5-4343-842a-1cf34448ab92', 'd0000001-0000-4000-8000-000000000005'),
('701d1dc4-31d5-4343-842a-1cf34448ab92', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- Deep Reinforcement Learning → ai_ml, research
('524ca9d2-a633-44f4-94cf-04d92ac874c5', 'd0000001-0000-4000-8000-000000000005'),
('524ca9d2-a633-44f4-94cf-04d92ac874c5', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- Engram → ai_ml
('67643187-9896-4578-a425-d88d61df9064', 'd0000001-0000-4000-8000-000000000005'),
-- Fine-tuning → ai_ml
('1e1723a3-4395-4efb-8465-98e6d174f471', 'd0000001-0000-4000-8000-000000000005'),
-- Graph Memory Systems → ai_ml
('9133d61a-35f8-4224-add4-388f9223b796', 'd0000001-0000-4000-8000-000000000005'),
-- Graphiti → ai_ml
('e3a5912a-32be-43a0-8495-95a184b4f2df', 'd0000001-0000-4000-8000-000000000005'),
-- Hyperparameter Tuning → ai_ml
('aef290e8-f974-43af-b84c-6a1df8173111', 'd0000001-0000-4000-8000-000000000005'),
-- LLM Agent Platforms → ai_ml
('e8ebab6c-c9be-433d-b339-bd3caceef94a', 'd0000001-0000-4000-8000-000000000005'),
-- Large Language Models → ai_ml, research
('3081b4f4-bec2-49f7-aa6a-70c851af1edc', 'd0000001-0000-4000-8000-000000000005'),
('3081b4f4-bec2-49f7-aa6a-70c851af1edc', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- MCP Servers → ai_ml, software
('0cdf99dd-9010-4e2c-9654-76abeee4bae0', 'd0000001-0000-4000-8000-000000000005'),
('0cdf99dd-9010-4e2c-9654-76abeee4bae0', 'd0000001-0000-4000-8000-000000000002'),
-- Model Evaluation → ai_ml
('b84537c7-d6d0-4ea7-ba3a-7c2b8232228d', 'd0000001-0000-4000-8000-000000000005'),
-- Model Tuning → ai_ml
('7eb83285-756c-48cd-8b33-50f4a8963bd5', 'd0000001-0000-4000-8000-000000000005'),
-- Multi-Modal → ai_ml
('b831e156-8041-48c7-a5f0-714c95d3ff37', 'd0000001-0000-4000-8000-000000000005'),
-- ONNX → ai_ml
('50441a20-7e75-47ed-a256-a2452bb65ccc', 'd0000001-0000-4000-8000-000000000005'),
-- RAG → ai_ml
('600c6a07-5b86-4012-bb98-c9bb9fa5efc6', 'd0000001-0000-4000-8000-000000000005'),
-- Reinforcement Learning → ai_ml, research
('e8f24fcb-0486-4ea0-a83d-cbdebc807f98', 'd0000001-0000-4000-8000-000000000005'),
('e8f24fcb-0486-4ea0-a83d-cbdebc807f98', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- Retraining → ai_ml
('bef2d9f4-dbe4-4d13-a11d-b06375cc0439', 'd0000001-0000-4000-8000-000000000005'),

-- ═══════════════════════════════════════════
-- CATEGORY: concept  (15 skills)
-- ═══════════════════════════════════════════
-- CI/CD → devops
('c0708d08-4d7d-461e-b2d3-c4de14701644', 'd0000001-0000-4000-8000-000000000004'),
-- Cleared Environments → security, operations
('6ca34f60-771a-4700-a6b3-ba07af4d4a37', 'd0000001-0000-4000-8000-000000000003'),
('6ca34f60-771a-4700-a6b3-ba07af4d4a37', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- Cloud → devops, systems
('34636cb3-8db0-4585-ae02-eb0ed05dbb9c', 'd0000001-0000-4000-8000-000000000004'),
('34636cb3-8db0-4585-ae02-eb0ed05dbb9c', 'd0000001-0000-4000-8000-000000000001'),
-- Data Governance → data_science, leadership
('b17318ff-d827-4044-be71-d25f2b53367a', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('b17318ff-d827-4044-be71-d25f2b53367a', 'd0000001-0000-4000-8000-000000000006'),
-- Data Lineage → data_science, ai_ml
('f7410440-6533-493a-841e-780eebf1c7f9', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('f7410440-6533-493a-841e-780eebf1c7f9', 'd0000001-0000-4000-8000-000000000005'),
-- Distributed Systems → systems, engineering
('39fe8f41-5c0d-47ff-9004-42d4ed21fb02', 'd0000001-0000-4000-8000-000000000001'),
('39fe8f41-5c0d-47ff-9004-42d4ed21fb02', 'c41655ca-19f2-4d22-b41c-cec433244e5b'),
-- Event-driven Architecture → architecture, software
('4af5697e-40a2-4c89-9948-7313c3b33ea8', 'da34fc1d-d93f-4cfa-bf46-a82c4c6c853c'),
('4af5697e-40a2-4c89-9948-7313c3b33ea8', 'd0000001-0000-4000-8000-000000000002'),
-- IaC → devops
('e484bca5-31a0-4718-943c-dcc0fa2413e7', 'd0000001-0000-4000-8000-000000000004'),
-- Microservices → architecture, software
('d5bef3fa-f8ed-4363-a7cb-814386eb0b21', 'da34fc1d-d93f-4cfa-bf46-a82c4c6c853c'),
('d5bef3fa-f8ed-4363-a7cb-814386eb0b21', 'd0000001-0000-4000-8000-000000000002'),
-- Multi-agent Orchestration → ai_ml
('3e84b883-a5bb-4679-a4a1-a0cb2160ea21', 'd0000001-0000-4000-8000-000000000005'),
-- Observability → devops, operations
('41c85575-4781-421f-b97f-253e8003fdac', 'd0000001-0000-4000-8000-000000000004'),
('41c85575-4781-421f-b97f-253e8003fdac', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- Regulated Environments → security, operations
('59576ed0-b2b6-4d85-b148-a270625e05c6', 'd0000001-0000-4000-8000-000000000003'),
('59576ed0-b2b6-4d85-b148-a270625e05c6', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- Responsible AI → ai_ml, leadership
('5537d174-cfc8-4868-a6c0-4c302652ddce', 'd0000001-0000-4000-8000-000000000005'),
('5537d174-cfc8-4868-a6c0-4c302652ddce', 'd0000001-0000-4000-8000-000000000006'),
-- SIEM → security
('62b5c66a-4d97-4d38-a84f-842f63df5fe1', 'd0000001-0000-4000-8000-000000000003'),
-- Solutions Architecture → architecture
('4e9a0784-4709-4cdb-a559-b7c09901c8c9', 'da34fc1d-d93f-4cfa-bf46-a82c4c6c853c'),

-- ═══════════════════════════════════════════
-- CATEGORY: data_systems  (13 skills)
-- ═══════════════════════════════════════════
-- CockroachDB → software, data_science
('691d962a-3a16-446c-aa2b-4c574f894ae4', 'd0000001-0000-4000-8000-000000000002'),
('691d962a-3a16-446c-aa2b-4c574f894ae4', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- DuckPGQ → data_science
('93c62b6f-010f-48e6-9797-ffb574caeff9', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- FalkorDB → ai_ml, data_science
('074baceb-b5a8-4cf9-b542-e812a7224836', 'd0000001-0000-4000-8000-000000000005'),
('074baceb-b5a8-4cf9-b542-e812a7224836', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Graph DBs → data_science, software
('7c79ec5e-4348-4dde-a8bf-d9d5c0c33b29', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('7c79ec5e-4348-4dde-a8bf-d9d5c0c33b29', 'd0000001-0000-4000-8000-000000000002'),
-- Janus Graph → data_science
('34401445-17d0-44c9-a6ba-c5d2f3706ff2', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Kuzu → data_science
('a122d252-e7a9-4049-93aa-f179a5ee1c29', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- MongoDB → software, data_science
('57d7d78f-0bf3-4334-adcf-9e5d55af7a63', 'd0000001-0000-4000-8000-000000000002'),
('57d7d78f-0bf3-4334-adcf-9e5d55af7a63', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Neo4J → data_science, ai_ml
('58cc008d-e368-40dc-904c-ea1f84e82c70', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('58cc008d-e368-40dc-904c-ea1f84e82c70', 'd0000001-0000-4000-8000-000000000005'),
-- NoSQL → software, data_science
('cab69520-ad2c-41a9-b50e-87974bf53b1e', 'd0000001-0000-4000-8000-000000000002'),
('cab69520-ad2c-41a9-b50e-87974bf53b1e', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- OpenMetadata → data_science, devops
('470ede2a-2ff6-470f-805a-8cb108cfd4ed', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('470ede2a-2ff6-470f-805a-8cb108cfd4ed', 'd0000001-0000-4000-8000-000000000004'),
-- PostgreSQL → software, data_science
('9fc04817-9d8f-4c02-b340-b6da9995e80a', 'd0000001-0000-4000-8000-000000000002'),
('9fc04817-9d8f-4c02-b340-b6da9995e80a', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Quine → data_science, security
('b80876a6-e1e1-4616-933f-c82a42cc7583', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('b80876a6-e1e1-4616-933f-c82a42cc7583', 'd0000001-0000-4000-8000-000000000003'),
-- Vector Databases → ai_ml, data_science
('4a4964ec-6754-4882-8f0f-2a3bb39f9bad', 'd0000001-0000-4000-8000-000000000005'),
('4a4964ec-6754-4882-8f0f-2a3bb39f9bad', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),

-- ═══════════════════════════════════════════
-- CATEGORY: framework  (4 skills)
-- ═══════════════════════════════════════════
-- LangChain → ai_ml
('aa770093-8581-40ab-9d55-a4e9c71a3f38', 'd0000001-0000-4000-8000-000000000005'),
-- OpenTelemetry → devops, operations
('91d369b7-fe0e-4b15-bf5f-b81b70347264', 'd0000001-0000-4000-8000-000000000004'),
('91d369b7-fe0e-4b15-bf5f-b81b70347264', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- Spark → data_science, ai_ml
('3dffe270-d4c5-4d32-b9b3-4675d78dee28', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('3dffe270-d4c5-4d32-b9b3-4675d78dee28', 'd0000001-0000-4000-8000-000000000005'),
-- Spring Boot → software
('7ee593de-00c1-4018-9c4f-277d6b31092a', 'd0000001-0000-4000-8000-000000000002'),

-- ═══════════════════════════════════════════
-- CATEGORY: infrastructure  (25 skills)
-- ═══════════════════════════════════════════
-- Air-gapped Deployments → security, systems
('772cd89a-39be-4db2-b40c-0303d10b32cf', 'd0000001-0000-4000-8000-000000000003'),
('772cd89a-39be-4db2-b40c-0303d10b32cf', 'd0000001-0000-4000-8000-000000000001'),
-- Ansible → devops
('7d6c6feb-bd13-4d9f-8831-2cc1109f48cf', 'd0000001-0000-4000-8000-000000000004'),
-- Cloud (AWS, Azure, GCP) → devops, systems
('a83894b9-843d-422e-97a0-a011e3d26528', 'd0000001-0000-4000-8000-000000000004'),
('a83894b9-843d-422e-97a0-a011e3d26528', 'd0000001-0000-4000-8000-000000000001'),
-- Docker → devops
('dec1325e-9b18-4aee-ab0b-7e1be50a35d8', 'd0000001-0000-4000-8000-000000000004'),
-- Elasticsearch → devops, security
('38a56869-0508-44fb-89e7-58755aa5d2c8', 'd0000001-0000-4000-8000-000000000004'),
('38a56869-0508-44fb-89e7-58755aa5d2c8', 'd0000001-0000-4000-8000-000000000003'),
-- Feast → ai_ml, data_science
('04315ce3-9276-468e-a92c-f6ffdefe7a30', 'd0000001-0000-4000-8000-000000000005'),
('04315ce3-9276-468e-a92c-f6ffdefe7a30', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Fluvio → systems, data_science
('9ec67a44-3d0b-409d-bbc4-761a431aeb68', 'd0000001-0000-4000-8000-000000000001'),
('9ec67a44-3d0b-409d-bbc4-761a431aeb68', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- GPU → ai_ml, systems
('b022b854-35f1-4c2c-a1b6-4b629a601b3e', 'd0000001-0000-4000-8000-000000000005'),
('b022b854-35f1-4c2c-a1b6-4b629a601b3e', 'd0000001-0000-4000-8000-000000000001'),
-- GitHub Actions → devops
('3be0dde2-804b-4fb2-8aaa-3f89ed19c51d', 'd0000001-0000-4000-8000-000000000004'),
-- GitLab CI/CD → devops
('419ba7b0-827a-4c62-b33a-d1ae0537d7e4', 'd0000001-0000-4000-8000-000000000004'),
-- HPC → systems, ai_ml
('38489b2c-c2c2-4aa9-8f67-c4a248fc8861', 'd0000001-0000-4000-8000-000000000001'),
('38489b2c-c2c2-4aa9-8f67-c4a248fc8861', 'd0000001-0000-4000-8000-000000000005'),
-- Helm → devops
('63648cb3-a839-44b1-8f1c-999e9f8281bb', 'd0000001-0000-4000-8000-000000000004'),
-- Istio → devops
('833307ae-da01-46ce-b493-7ad2f10b1e68', 'd0000001-0000-4000-8000-000000000004'),
-- KFServing → ai_ml, devops
('54f796f2-5a0a-4ca9-901b-ce1bde25e298', 'd0000001-0000-4000-8000-000000000005'),
('54f796f2-5a0a-4ca9-901b-ce1bde25e298', 'd0000001-0000-4000-8000-000000000004'),
-- Kafka → systems, data_science
('5e6c3452-c003-4705-ad95-70930e3868e5', 'd0000001-0000-4000-8000-000000000001'),
('5e6c3452-c003-4705-ad95-70930e3868e5', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Kubernetes → devops, systems
('54d31ad4-8aff-4417-810c-219cb938644c', 'd0000001-0000-4000-8000-000000000004'),
('54d31ad4-8aff-4417-810c-219cb938644c', 'd0000001-0000-4000-8000-000000000001'),
-- LakeFS → data_science, devops
('a7569724-2aa3-4f3a-bfcc-290619792e0f', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('a7569724-2aa3-4f3a-bfcc-290619792e0f', 'd0000001-0000-4000-8000-000000000004'),
-- Linkerd → devops
('e6c88e4c-41bd-4a46-ba58-024626c787ee', 'd0000001-0000-4000-8000-000000000004'),
-- MLFlow → ai_ml, devops
('395e128b-ece8-4820-b1cf-249a6802e185', 'd0000001-0000-4000-8000-000000000005'),
('395e128b-ece8-4820-b1cf-249a6802e185', 'd0000001-0000-4000-8000-000000000004'),
-- MageAI → ai_ml, data_science
('3a082ba6-2170-4e95-8f70-9028767bc5d2', 'd0000001-0000-4000-8000-000000000005'),
('3a082ba6-2170-4e95-8f70-9028767bc5d2', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- RAG Pipelines → ai_ml
('fff1715a-71dd-4a21-b4d5-db0cb3129959', 'd0000001-0000-4000-8000-000000000005'),
-- Ray Serve → ai_ml, systems
('ae2397bf-93c2-477c-a868-7ddec50746ea', 'd0000001-0000-4000-8000-000000000005'),
('ae2397bf-93c2-477c-a868-7ddec50746ea', 'd0000001-0000-4000-8000-000000000001'),
-- Service Mesh → devops, systems
('0e66328b-a782-467f-bf81-cdbae070a87d', 'd0000001-0000-4000-8000-000000000004'),
('0e66328b-a782-467f-bf81-cdbae070a87d', 'd0000001-0000-4000-8000-000000000001'),
-- Streaming → systems, data_science
('50e7a7ae-e58c-4748-854b-e68f48f5bf78', 'd0000001-0000-4000-8000-000000000001'),
('50e7a7ae-e58c-4748-854b-e68f48f5bf78', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- TPU → ai_ml, systems
('dee2b666-f37e-4004-abe5-6a633f3e248f', 'd0000001-0000-4000-8000-000000000005'),
('dee2b666-f37e-4004-abe5-6a633f3e248f', 'd0000001-0000-4000-8000-000000000001'),

-- ═══════════════════════════════════════════
-- CATEGORY: language  (16 skills)
-- ═══════════════════════════════════════════
-- Bash → software, devops
('8f51e337-6a25-4041-b1b6-7cc37350404b', 'd0000001-0000-4000-8000-000000000002'),
('8f51e337-6a25-4041-b1b6-7cc37350404b', 'd0000001-0000-4000-8000-000000000004'),
-- C → software, systems
('a3f89953-84a0-47a7-aa51-90720d2ca580', 'd0000001-0000-4000-8000-000000000002'),
('a3f89953-84a0-47a7-aa51-90720d2ca580', 'd0000001-0000-4000-8000-000000000001'),
-- C++ → software, systems
('ac6f29c6-7db2-44a7-b7b5-f56e9b0e47bf', 'd0000001-0000-4000-8000-000000000002'),
('ac6f29c6-7db2-44a7-b7b5-f56e9b0e47bf', 'd0000001-0000-4000-8000-000000000001'),
-- English — skip (natural language, not a career domain)
-- Go → software
('b2950a36-d09f-4363-b410-c88fbdeb6f3a', 'd0000001-0000-4000-8000-000000000002'),
-- HCL → devops
('bfa20117-552f-4823-b94c-2b889430ac60', 'd0000001-0000-4000-8000-000000000004'),
-- Japanese — skip (natural language)
-- Java → software
('57b2d8ff-5144-4b0a-b071-42958af476e3', 'd0000001-0000-4000-8000-000000000002'),
-- JavaScript → software
('a950d2d2-d331-4280-bccf-90802975812b', 'd0000001-0000-4000-8000-000000000002'),
-- PowerShell → software, security
('15d6b8e5-2826-44ad-9867-4ad0d2f148ed', 'd0000001-0000-4000-8000-000000000002'),
('15d6b8e5-2826-44ad-9867-4ad0d2f148ed', 'd0000001-0000-4000-8000-000000000003'),
-- Python → software, ai_ml
('820c4f0c-af30-454d-b83c-5fa587d443a1', 'd0000001-0000-4000-8000-000000000002'),
('820c4f0c-af30-454d-b83c-5fa587d443a1', 'd0000001-0000-4000-8000-000000000005'),
-- Rust → software, systems
('33745eac-c880-4964-a76d-fa3daf7ad98b', 'd0000001-0000-4000-8000-000000000002'),
('33745eac-c880-4964-a76d-fa3daf7ad98b', 'd0000001-0000-4000-8000-000000000001'),
-- SQL → software, data_science
('959acf67-065c-4713-94fe-fe76e8e1ce0b', 'd0000001-0000-4000-8000-000000000002'),
('959acf67-065c-4713-94fe-fe76e8e1ce0b', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Terraform → devops
('cf5ea003-687e-4833-ac4f-def3342200c1', 'd0000001-0000-4000-8000-000000000004'),
-- Terragrunt → devops
('21bd8947-eebe-4534-a81f-6221fb4fb9bd', 'd0000001-0000-4000-8000-000000000004'),
-- TypeScript → software
('56265129-28b9-4315-8792-aa8f7ae59520', 'd0000001-0000-4000-8000-000000000002'),

-- ═══════════════════════════════════════════
-- CATEGORY: library  (3 skills)
-- ═══════════════════════════════════════════
-- PyTorch → ai_ml
('0c29480c-072b-4f8f-ad94-9a7d10b3d696', 'd0000001-0000-4000-8000-000000000005'),
-- TensorFlow → ai_ml
('8d57d723-2866-4ff7-b514-6b4ba39d2a1c', 'd0000001-0000-4000-8000-000000000005'),
-- TensorRT → ai_ml
('6e7f058c-10b8-4c8f-a4d3-3b0f369fdcb3', 'd0000001-0000-4000-8000-000000000005'),

-- ═══════════════════════════════════════════
-- CATEGORY: methodology  (25 skills)
-- ═══════════════════════════════════════════
-- Agile → leadership, operations
('984a8498-a3b9-47b5-a0f5-5b1d2d47c1d8', 'd0000001-0000-4000-8000-000000000006'),
('984a8498-a3b9-47b5-a0f5-5b1d2d47c1d8', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- AI Agents → ai_ml
('8ca2467e-0fff-43cb-a2bb-5df8054b6f07', 'd0000001-0000-4000-8000-000000000005'),
-- AI Governance → ai_ml, leadership
('9a2113cc-50bb-4cdd-80e6-c1dbdf22ce86', 'd0000001-0000-4000-8000-000000000005'),
('9a2113cc-50bb-4cdd-80e6-c1dbdf22ce86', 'd0000001-0000-4000-8000-000000000006'),
-- API Design → software, architecture
('ba1a5e1c-df98-4332-a44f-003ee994b1c6', 'd0000001-0000-4000-8000-000000000002'),
('ba1a5e1c-df98-4332-a44f-003ee994b1c6', 'da34fc1d-d93f-4cfa-bf46-a82c4c6c853c'),
-- API Integration → software
('28ac4f05-b491-4e27-a242-55ac5ae816cf', 'd0000001-0000-4000-8000-000000000002'),
-- Cloud Cost Governance → devops, leadership
('291f729d-5276-447d-ab63-451e39c9ae70', 'd0000001-0000-4000-8000-000000000004'),
('291f729d-5276-447d-ab63-451e39c9ae70', 'd0000001-0000-4000-8000-000000000006'),
-- Code Review → software, engineering
('58ec4840-7988-4ede-9e05-94d8ff9b4b31', 'd0000001-0000-4000-8000-000000000002'),
('58ec4840-7988-4ede-9e05-94d8ff9b4b31', 'c41655ca-19f2-4d22-b41c-cec433244e5b'),
-- Continuous Validation → devops, engineering
('d8e38411-8b6c-4783-86c7-15647200340b', 'd0000001-0000-4000-8000-000000000004'),
('d8e38411-8b6c-4783-86c7-15647200340b', 'c41655ca-19f2-4d22-b41c-cec433244e5b'),
-- Data Pipeline Design → data_science, engineering
('c0130354-fa59-4676-9565-96c5936ffd91', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('c0130354-fa59-4676-9565-96c5936ffd91', 'c41655ca-19f2-4d22-b41c-cec433244e5b'),
-- DevOps → devops
('9d352630-f19b-49fa-811c-3d9d29cd46f0', 'd0000001-0000-4000-8000-000000000004'),
-- DevSecOps → devops, security
('a61c09b4-4e2f-4e92-ba10-1b7853243b81', 'd0000001-0000-4000-8000-000000000004'),
('a61c09b4-4e2f-4e92-ba10-1b7853243b81', 'd0000001-0000-4000-8000-000000000003'),
-- Domain-Driven Design → architecture, software
('1b9831f4-3a5e-4bd0-af01-5078c4ce89ab', 'da34fc1d-d93f-4cfa-bf46-a82c4c6c853c'),
('1b9831f4-3a5e-4bd0-af01-5078c4ce89ab', 'd0000001-0000-4000-8000-000000000002'),
-- Experiment Design → research, ai_ml
('190eedcd-0ef5-469e-9e36-e86176fa340a', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
('190eedcd-0ef5-469e-9e36-e86176fa340a', 'd0000001-0000-4000-8000-000000000005'),
-- Exploratory Data Analysis → data_science, research
('18909c3d-5d34-4cdf-9a97-c9e9e7f6ff6e', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('18909c3d-5d34-4cdf-9a97-c9e9e7f6ff6e', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- FedRAMP Compliance → security
('008ef92c-747b-4dbb-a7b4-3464fb6c2e26', 'd0000001-0000-4000-8000-000000000003'),
-- Hypothesis-driven Development → research
('39e7db73-baf1-4cc3-a039-4d687473c539', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- LLMOps → ai_ml, devops
('9b3f77b7-6538-4043-90c3-352f7e5fdd99', 'd0000001-0000-4000-8000-000000000005'),
('9b3f77b7-6538-4043-90c3-352f7e5fdd99', 'd0000001-0000-4000-8000-000000000004'),
-- Machine Learning → ai_ml
('3584f8bd-0b2c-4376-9f52-0b4d140356cc', 'd0000001-0000-4000-8000-000000000005'),
-- MLOps → ai_ml, devops
('332fa61c-d897-4638-9923-59757df0521b', 'd0000001-0000-4000-8000-000000000005'),
('332fa61c-d897-4638-9923-59757df0521b', 'd0000001-0000-4000-8000-000000000004'),
-- Prompt Engineering → ai_ml
('6c3b3a5f-08ad-4a2a-8073-dc69ffd72b40', 'd0000001-0000-4000-8000-000000000005'),
-- Research → research
('07386f87-9bba-401f-a801-1f30b137714e', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- Revenue Management → leadership
('1a13ac67-a8b2-4b1b-8d46-26ae1f29e588', 'd0000001-0000-4000-8000-000000000006'),
-- SAFe → leadership, operations
('1da1e23c-b19a-4ed1-9b71-15681d50a0c0', 'd0000001-0000-4000-8000-000000000006'),
('1da1e23c-b19a-4ed1-9b71-15681d50a0c0', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- SLO/SLI/Error Budget Engineering → devops, operations
('b13ab73f-c139-4602-92b0-7566cb12ace1', 'd0000001-0000-4000-8000-000000000004'),
('b13ab73f-c139-4602-92b0-7566cb12ace1', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- STIG Compliance → security
('4b5de168-b514-4db9-b494-55a868114ee1', 'd0000001-0000-4000-8000-000000000003'),

-- ═══════════════════════════════════════════
-- CATEGORY: other  (14 skills)
-- ═══════════════════════════════════════════
-- InfiniBand → systems
('d8c7f594-07c8-4f2e-980c-073d85379d45', 'd0000001-0000-4000-8000-000000000001'),
-- Literature Review → research
('f6584e5e-f4c6-48ab-b9bd-fdf07dbc79a7', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- Memphis → systems, data_science
('bd3573f9-a6f7-40b0-8d4c-37e14e30e41b', 'd0000001-0000-4000-8000-000000000001'),
('bd3573f9-a6f7-40b0-8d4c-37e14e30e41b', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Military Service → leadership
('d2d6c4fe-a477-4a98-b03a-6baf2c47c7d2', 'd0000001-0000-4000-8000-000000000006'),
-- NCCL → ai_ml, systems
('9078b4dc-3397-4b32-a1ba-ec4040c04b46', 'd0000001-0000-4000-8000-000000000005'),
('9078b4dc-3397-4b32-a1ba-ec4040c04b46', 'd0000001-0000-4000-8000-000000000001'),
-- Penetration Testing → security
('c59aee16-9089-4140-b760-da8a730ead4d', 'd0000001-0000-4000-8000-000000000003'),
-- Pre-Sales Engineering → leadership, architecture
('654b3ed0-6692-41de-84d8-87f66b6ae5ea', 'd0000001-0000-4000-8000-000000000006'),
('654b3ed0-6692-41de-84d8-87f66b6ae5ea', 'da34fc1d-d93f-4cfa-bf46-a82c4c6c853c'),
-- Privacy by Design → security
('9596b652-c68c-4146-8b63-4baedd6ced79', 'd0000001-0000-4000-8000-000000000003'),
-- Privacy Engineering → security
('b365cfac-c17b-4d71-b3f3-644751644158', 'd0000001-0000-4000-8000-000000000003'),
-- Privacy Regulations (GDPR/CCPA) → security, leadership
('4912014b-b858-4fd1-8c46-a4959e51dad9', 'd0000001-0000-4000-8000-000000000003'),
('4912014b-b858-4fd1-8c46-a4959e51dad9', 'd0000001-0000-4000-8000-000000000006'),
-- RoCEv2 → systems
('4cdad735-b29f-41b9-98ce-07ebe353e109', 'd0000001-0000-4000-8000-000000000001'),
-- SOTA models → ai_ml, research
('5245a7d7-322b-49c5-abba-b478bdb97795', 'd0000001-0000-4000-8000-000000000005'),
('5245a7d7-322b-49c5-abba-b478bdb97795', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- Technical Evaluation → engineering, research
('8e6e9343-6fa3-4726-af5f-65ef712b8e46', 'c41655ca-19f2-4d22-b41c-cec433244e5b'),
('8e6e9343-6fa3-4726-af5f-65ef712b8e46', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- Uncertainty Quantification → ai_ml, research
('57e57e13-f0e9-4cdc-99a1-a345d8e1b0e2', 'd0000001-0000-4000-8000-000000000005'),
('57e57e13-f0e9-4cdc-99a1-a345d8e1b0e2', 'e779c920-37dc-4505-bd62-7c55150f83b7'),

-- ═══════════════════════════════════════════
-- CATEGORY: platform  (24 skills)
-- ═══════════════════════════════════════════
-- AppDynamics → devops, operations
('be4ae6b4-7a4b-40f6-abfb-5c582e61d6ba', 'd0000001-0000-4000-8000-000000000004'),
('be4ae6b4-7a4b-40f6-abfb-5c582e61d6ba', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- Aurora → data_science
('a03582ae-4cc3-4d2c-8aad-fe19b3928bae', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- AWS → devops, systems
('791a6433-c3ab-40f7-9606-82c2fbcccc02', 'd0000001-0000-4000-8000-000000000004'),
('791a6433-c3ab-40f7-9606-82c2fbcccc02', 'd0000001-0000-4000-8000-000000000001'),
-- AWS Bedrock → ai_ml
('dd29f083-ce4d-47b4-8779-ac2a9e1e288f', 'd0000001-0000-4000-8000-000000000005'),
-- AWS EKS → devops
('a26473b3-41f6-45aa-a905-1f253bb01a38', 'd0000001-0000-4000-8000-000000000004'),
-- AWS EMR → data_science
('8ab0e958-b8e0-455a-b987-5a6302e394de', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- AWS Lambda → software, devops
('380f5d28-b83e-4ce1-aee2-1af6220ceded', 'd0000001-0000-4000-8000-000000000002'),
('380f5d28-b83e-4ce1-aee2-1af6220ceded', 'd0000001-0000-4000-8000-000000000004'),
-- AWS S3 → devops
('b35abbbd-7fb6-4e36-b784-4bd9ee800ff5', 'd0000001-0000-4000-8000-000000000004'),
-- Azure → devops, systems
('cd4acb94-0bac-4b40-b2b8-83a944f2ca88', 'd0000001-0000-4000-8000-000000000004'),
('cd4acb94-0bac-4b40-b2b8-83a944f2ca88', 'd0000001-0000-4000-8000-000000000001'),
-- Big Bang → devops, security
('5848dbd6-cb86-4f5b-87d4-137ef28db085', 'd0000001-0000-4000-8000-000000000004'),
('5848dbd6-cb86-4f5b-87d4-137ef28db085', 'd0000001-0000-4000-8000-000000000003'),
-- Datadog → devops, operations
('8a4be7d3-31e6-4a1a-8363-42a03cb1953a', 'd0000001-0000-4000-8000-000000000004'),
('8a4be7d3-31e6-4a1a-8363-42a03cb1953a', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- DeepChecks → ai_ml
('75e3f3cc-4c09-479b-92ad-afa9d4135acb', 'd0000001-0000-4000-8000-000000000005'),
-- Dynatrace → devops, operations
('135b903e-fec2-4225-8bac-09b74d63dead', 'd0000001-0000-4000-8000-000000000004'),
('135b903e-fec2-4225-8bac-09b74d63dead', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- Elastic → security, devops
('b6e0ee31-3032-4fdd-af97-60f51b860fed', 'd0000001-0000-4000-8000-000000000003'),
('b6e0ee31-3032-4fdd-af97-60f51b860fed', 'd0000001-0000-4000-8000-000000000004'),
-- GCP → devops, systems
('9bb102b5-adcb-43a9-a24c-1e10499dae68', 'd0000001-0000-4000-8000-000000000004'),
('9bb102b5-adcb-43a9-a24c-1e10499dae68', 'd0000001-0000-4000-8000-000000000001'),
-- Gymnasium → ai_ml, research
('fb42583a-3673-458a-ba33-27c0a6b0bcbb', 'd0000001-0000-4000-8000-000000000005'),
('fb42583a-3673-458a-ba33-27c0a6b0bcbb', 'e779c920-37dc-4505-bd62-7c55150f83b7'),
-- Harness.io → devops
('849c52b1-35d7-49c9-b620-9a0e11e0b411', 'd0000001-0000-4000-8000-000000000004'),
-- Linux → systems
('64c40426-921c-48a9-a404-38b314502781', 'd0000001-0000-4000-8000-000000000001'),
-- OpenShift → devops
('5ba04a20-0bfe-4379-aa7b-b7133ebc3a61', 'd0000001-0000-4000-8000-000000000004'),
-- Platform One → devops, security
('c5ae4f5d-5ae5-422b-acfd-7c87636b4353', 'd0000001-0000-4000-8000-000000000004'),
('c5ae4f5d-5ae5-422b-acfd-7c87636b4353', 'd0000001-0000-4000-8000-000000000003'),
-- SageMaker → ai_ml
('9970d2d0-999b-44ba-bb0b-15338a18da67', 'd0000001-0000-4000-8000-000000000005'),
-- Snowflake → data_science
('d92930cb-547a-433f-82a3-880c767a381b', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
-- Splunk → security, operations
('66a975c2-182e-4286-ad87-f9788a6940d5', 'd0000001-0000-4000-8000-000000000003'),
('66a975c2-182e-4286-ad87-f9788a6940d5', '3329fa0f-0fd0-4569-a25f-0fcb85010da5'),
-- Watsonx → ai_ml
('2b9b6ad0-bc8f-4270-b246-ff5dbb4e415a', 'd0000001-0000-4000-8000-000000000005'),

-- ═══════════════════════════════════════════
-- CATEGORY: protocol  (4 skills)
-- ═══════════════════════════════════════════
-- LDAP → security, systems
('c9e47317-a3e9-4be7-a497-286ae9a2ab23', 'd0000001-0000-4000-8000-000000000003'),
('c9e47317-a3e9-4be7-a497-286ae9a2ab23', 'd0000001-0000-4000-8000-000000000001'),
-- MCP → ai_ml
('8dbdc49b-b285-4cb3-8583-2d19e9e02e9e', 'd0000001-0000-4000-8000-000000000005'),
-- SAML → security
('ae696b32-0439-4bf2-a823-b89581bbbff0', 'd0000001-0000-4000-8000-000000000003'),
-- SSO → security
('1bb6ae88-04f6-4395-b3a0-d6a8a59273be', 'd0000001-0000-4000-8000-000000000003'),

-- ═══════════════════════════════════════════
-- CATEGORY: security  (15 skills) — all → security domain
-- ═══════════════════════════════════════════
('a8fa4200-1d72-41ed-bef5-1c8fbe7e78ab', 'd0000001-0000-4000-8000-000000000003'), -- Cloud Forensics
('d3520fc3-9f8c-447b-a4fa-aeed170e55c5', 'd0000001-0000-4000-8000-000000000003'), -- Cloud Security
('5a889cb1-7904-4c4c-9638-5daae11d8c59', 'd0000001-0000-4000-8000-000000000003'), -- Container Security
('12e2b306-5fde-422b-8ecc-0cbf750b1c25', 'd0000001-0000-4000-8000-000000000003'), -- Cyber Threat Intelligence
('d0a9bf44-fa16-4d95-8f56-9861b0610ed6', 'd0000001-0000-4000-8000-000000000003'), -- Detection Engineering
('50af5b91-0538-4cfe-9917-7dcdf3e8976e', 'd0000001-0000-4000-8000-000000000003'), -- Digital Forensics
('4aaf9d8b-1c00-4c9d-9a2d-ce2ca26b42c6', 'd0000001-0000-4000-8000-000000000003'), -- Incident Response
('3f55f677-2b18-41db-90cb-1fad72d2086c', 'd0000001-0000-4000-8000-000000000003'), -- LLM Security
('763d0ec8-d528-4ace-87c7-72548942777b', 'd0000001-0000-4000-8000-000000000003'), -- MITRE ATT&CK
('058ca1a5-e6ad-46d9-8875-cdc76b0d9dc9', 'd0000001-0000-4000-8000-000000000003'), -- Malware Analysis
('1fc2bd76-6ac9-41ff-a49e-a877549329f7', 'd0000001-0000-4000-8000-000000000003'), -- Red Team Operations
('11873c62-b373-4a8f-8b01-c63b06cf9511', 'd0000001-0000-4000-8000-000000000003'), -- Reverse Engineering
('a022331a-95db-4ff1-828b-9f687106a9ce', 'd0000001-0000-4000-8000-000000000003'), -- Threat Hunting
('c05341ba-ef1f-4cd8-9354-d067bdcc574b', 'd0000001-0000-4000-8000-000000000003'), -- Traffic Analysis
('c1176335-9460-4e10-bfc6-b7104d3c46e7', 'd0000001-0000-4000-8000-000000000003'), -- Vulnerability Assessment

-- ═══════════════════════════════════════════
-- CATEGORY: soft_skill  (9 skills) — all → leadership
-- ═══════════════════════════════════════════
('52280076-d79c-4c54-9e40-80079a2444fb', 'd0000001-0000-4000-8000-000000000006'), -- Consumer Services
('6050b35c-7c3a-4ce7-bc1d-480f8e295aba', 'd0000001-0000-4000-8000-000000000006'), -- Cross-functional Collaboration
('b90ab3da-8361-4baf-a107-15b5adb10ee8', 'd0000001-0000-4000-8000-000000000006'), -- Customer-facing Delivery
('298c0ce0-fdb9-4f03-8032-2de2bc52be7f', 'd0000001-0000-4000-8000-000000000006'), -- Hospitality
('a5c90a43-2cfc-4418-953e-d1648204ce07', 'd0000001-0000-4000-8000-000000000006'), -- Mentoring
('be3ad2d4-9611-40ad-9952-e233f845a9b4', 'd0000001-0000-4000-8000-000000000006'), -- Project Leadership
('a50134a4-a7a2-4dee-abde-8b8872e29ae0', 'd0000001-0000-4000-8000-000000000006'), -- Stakeholder Communication
('8443157d-26e7-448e-8ac6-d380d25ee096', 'd0000001-0000-4000-8000-000000000006'), -- Technical Briefings
-- Technical Writing → leadership, engineering
('c55841ce-2dc4-4b91-aab3-0eaab9b12b12', 'd0000001-0000-4000-8000-000000000006'),
('c55841ce-2dc4-4b91-aab3-0eaab9b12b12', 'c41655ca-19f2-4d22-b41c-cec433244e5b'),

-- ═══════════════════════════════════════════
-- CATEGORY: tool  (5 skills)
-- ═══════════════════════════════════════════
-- Copilot Studio → ai_ml
('04547541-b38a-4960-afb9-e1f23a5a31f3', 'd0000001-0000-4000-8000-000000000005'),
-- Jenkins → devops
('304dd917-ec54-4875-a793-cdcdc97425ab', 'd0000001-0000-4000-8000-000000000004'),
-- Jupyter → data_science, ai_ml
('e6124e76-49aa-407b-be2a-8255bad3bd7a', 'b0a2874e-0bbb-4b6f-b167-08a7be0c9a14'),
('e6124e76-49aa-407b-be2a-8255bad3bd7a', 'd0000001-0000-4000-8000-000000000005'),
-- Packer → devops
('545db0e8-aa6e-4c94-93b2-6c5f0ad48798', 'd0000001-0000-4000-8000-000000000004'),
-- Sematic → ai_ml
('04e73e1c-5707-4cfc-9aa9-2638045c430e', 'd0000001-0000-4000-8000-000000000005')

; -- END
