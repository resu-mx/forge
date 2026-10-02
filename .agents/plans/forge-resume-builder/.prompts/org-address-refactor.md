# Org-Address Refactor Session Prompt

Paste this into a new Claude Code session.

---

## Context

Check memory for `project_session_2026_04_19.md` for the most recent session.

## Problem

The org-address refactor (bead `job-hunting-3hr`) is **95% complete** but has 2 untracked files that were never committed, causing **46 test failures** on main:

```
packages/core/src/services/address-service.ts   (93 lines, untracked)
packages/core/src/routes/addresses.ts            (53 lines, untracked)
```

The committed code in `services/index.ts` and `routes/server.ts` already imports from these files, so every test that spins up the app crashes with `Cannot find module './address-service'`.

## What's Already Done

All 4 child beads are closed. The following is committed:
- **Migration 047_org_locations.sql** — renamed org_campuses → org_locations, added address_id FK, migrated data
- **services/index.ts** — imports AddressService, wires into Services
- **routes/server.ts** — mounts addressRoutes
- **SDK types** — Address, OrgLocation with address_id, CreateAddress, UpdateAddress
- **SDK resources/organizations.ts** — address_id in create/update org location interfaces
- **Entity map** — org_locations + addresses entities (Phase 1.2+ storage abstraction)

## What's NOT Committed

1. `packages/core/src/services/address-service.ts` — AddressService CRUD (93 lines)
2. `packages/core/src/routes/addresses.ts` — HTTP routes for /addresses (53 lines)

## Tasks

1. **Read** the 2 untracked files and the committed code that imports them
2. **Verify** the address-service and route implementations are correct and complete
3. **Check** if SDK resources need an AddressResource (or if addresses are accessed through org locations)
4. **Check** if MCP tools need address create/update/delete tools
5. **Run tests** — the 46 failures should resolve once the files are committed
6. **Fix** any remaining test failures (e.g., buildOrgDisplayString, compileResumeIR)
7. **Commit** the files and close the epic bead `job-hunting-3hr`

## Test Failures to Fix (46 fail + 23 errors)

These are the pre-existing failures caused by the missing files:
- `buildOrgDisplayString` (6 failures) — org display string tests
- `compileResumeIR` (11 failures) — resume compiler
- `entity map coverage` (1) — entity map vs DB alignment
- `foreign key consistency` (1) — FK rules
- `runMigrations` (2) — migration count
- Other errors (23) — `Cannot find module './address-service'`

## Dev Commands

```bash
just dev          # Start API + MCP + WebUI
just test         # Run all tests
just test-core    # Core tests only
just migrate      # Run DB migrations
```
