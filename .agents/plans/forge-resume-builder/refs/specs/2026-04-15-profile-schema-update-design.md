# User Profile Schema Update: Address, URLs, Clearance Removal

**Date:** 2026-04-15
**Status:** Approved
**Goal:** Replace flat profile fields (location, linkedin, github, website) with a shared addresses table and a profile_urls table. Remove clearance from the UI (already removed from DB in migration 037).

## Current State

`user_profile` table has flat columns: `location` (free-text), `linkedin`, `github`, `website` (individual URL strings). Clearance was migrated to credentials entity in Phase 84 but the UI still shows a clearance input.

## Data Model

### `addresses` table (new, shared entity)

```sql
CREATE TABLE addresses (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,                    -- display name ("Reston, VA", "Remote", "HQ")
  street_1 TEXT,
  street_2 TEXT,
  city TEXT,
  state TEXT,
  zip TEXT,
  country_code TEXT DEFAULT 'US',       -- ISO 3166-1 alpha-2
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;
```

All fields except `name` are nullable. Supports "Remote" (name only), partial addresses (city + state), and full mailing addresses.

Designed as a shared entity. `user_profile` references it via FK. `organization_campuses` will reference it in a future migration (not in this spec's scope).

### `profile_urls` table (new)

```sql
CREATE TABLE profile_urls (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  profile_id TEXT NOT NULL REFERENCES user_profile(id) ON DELETE CASCADE,
  key TEXT NOT NULL,                     -- well-known or free-text
  url TEXT NOT NULL,
  position INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  UNIQUE(profile_id, key)               -- one URL per key per profile
) STRICT;
```

Well-known keys: `linkedin`, `github`, `gitlab`, `indeed`, `blog`, `portfolio`. Validated at the application layer. Custom keys are free-text with only one constraint: can't match a well-known key (case-insensitive).

### `user_profile` table changes

**Add:** `address_id TEXT REFERENCES addresses(id)` -- FK to addresses table.

**Drop columns:** `linkedin`, `github`, `website`, `location`.

`clearance` column was already dropped in migration 037. No DB change needed for clearance -- just remove it from the UI.

### Migration data flow

1. Create `addresses` table
2. Create `profile_urls` table
3. For each profile with a non-null `location`: insert an address row with `name = location`, set `address_id` on profile
4. For each profile: move `linkedin`, `github`, `website` values into `profile_urls` (skip nulls)
5. Drop `linkedin`, `github`, `website`, `location` columns from `user_profile`

## Types

### Core + SDK types

```ts
export interface Address {
  id: string
  name: string              // display name ("Reston, VA")
  street_1: string | null
  street_2: string | null
  city: string | null
  state: string | null
  zip: string | null
  country_code: string      // ISO 3166-1 alpha-2, default "US"
  created_at: string
  updated_at: string
}

export interface ProfileUrl {
  id: string
  profile_id: string
  key: string               // well-known or free-text
  url: string
  position: number
  created_at: string
}

export const WELL_KNOWN_URL_KEYS = ['linkedin', 'github', 'gitlab', 'indeed', 'blog', 'portfolio'] as const
export type WellKnownUrlKey = typeof WELL_KNOWN_URL_KEYS[number]
```

### Updated `UserProfile`

```ts
export interface UserProfile {
  id: string
  name: string
  email: string | null
  phone: string | null
  address_id: string | null
  address: Address | null         // populated on read (JOIN)
  urls: ProfileUrl[]              // populated on read (JOIN)
  salary_minimum: number | null
  salary_target: number | null
  salary_stretch: number | null
  created_at: string
  updated_at: string
}
```

Removed: `location`, `linkedin`, `github`, `website`, `clearance`.

## API

### Profile endpoint updates

**`GET /api/profile`** -- response now includes `address` (joined) and `urls` (joined).

**`PATCH /api/profile`** -- updated payload options:

```ts
{
  // Standard fields (unchanged)
  name?: string
  email?: string | null
  phone?: string | null
  salary_minimum?: number | null
  salary_target?: number | null
  salary_stretch?: number | null

  // Address: link existing or create/update inline
  address_id?: string | null         // link to existing address
  address?: Partial<Address>         // create new or update existing inline

  // URLs: full array replace
  urls?: Array<{ key: string; url: string }>
}
```

When `address` is provided inline:
- If profile already has an `address_id`, update that address row
- If no `address_id`, create a new address row and set the FK

When `urls` is provided:
- Delete all existing `profile_urls` for this profile
- Insert the new array
- Validate: no duplicate keys, custom keys don't collide with well-known keys

### New address endpoints

**`GET /api/addresses`** -- list all addresses (for picker/autocomplete in future org campus work)
**`POST /api/addresses`** -- create address
**`PATCH /api/addresses/:id`** -- update address
**`DELETE /api/addresses/:id`** -- delete (only if no FK references)

## Resume Compiler Impact

The resume compiler currently reads `profile.linkedin`, `profile.github`, `profile.website` for the default header. After migration:

- `linkedin` = `profile.urls.find(u => u.key === 'linkedin')?.url`
- `github` = `profile.urls.find(u => u.key === 'github')?.url`
- `website` = `profile.urls.find(u => u.key === 'blog')?.url ?? profile.urls.find(u => u.key === 'portfolio')?.url`
- `location` = `profile.address?.name`

The per-resume header override mechanism is unchanged.

## WebUI Profile Page

### Layout

**Personal Info:**
- Name, Email, Phone (unchanged)

**Address** (replaces `location` text input):
- Name (display location) -- most prominent field, renders on resumes
- Street 1, Street 2 (side by side)
- City, State, Zip (three-column row)
- Country Code (default "US")

**URLs** (replaces three separate inputs):
- Well-known URLs: fixed list of labeled inputs, only shown if value is set or user adds one
- "+ Add URL" dropdown for well-known keys not yet added
- "+ Add Custom URL" for free-text key + url pairs
- Each row has a remove [x] button

**Salary Expectations** (unchanged):
- Minimum, Target, Stretch

**Removed:**
- Security Clearance field

### Autosave

Same debounced autosave pattern as current page. Address changes save via profile PATCH (inline). URL changes send the full `urls` array.

## Decisions Made

- **Shared `addresses` table** -- not embedded in `user_profile`. Future-proofs for org campus refactor.
- **Full array replace for URLs** -- simpler than individual CRUD for a small list (<10 items). One PATCH replaces all URLs.
- **Custom URL keys are free-text** -- no slugification, just collision check against well-known keys.
- **`address.name` is the display location** -- what renders on resumes. Street/city/state are optional structured data for applications that need full addresses.
- **Clearance: UI removal only** -- DB column already gone (migration 037). Just remove the form field.
- **Org campus → address refactor is out of scope** -- the `addresses` table is designed for it, but the actual campus FK migration is separate work.
