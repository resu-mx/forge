# Phase 102: Profile Schema Update — Address, URLs, Clearance Removal

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace flat profile fields (location, linkedin, github, website) with a shared `addresses` table and a `profile_urls` table. Remove clearance from the profile UI.

**Architecture:** New `addresses` table (shared entity for future org campus reuse). New `profile_urls` table (keyed URL list with well-known + custom keys). Migration moves existing data. Profile service handles address inline create/update and URL array replace. Resume compiler reads URLs by key.

**Tech Stack:** SQLite, Bun, Hono, TypeScript, Svelte 5

**Spec:** `.claude/plans/forge-resume-builder/refs/specs/2026-04-15-profile-schema-update-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `packages/core/src/db/migrations/046_profile_addresses_urls.sql` | Create tables, migrate data, drop columns |
| `packages/core/src/types/index.ts` | Add `Address`, `ProfileUrl` types; update `UserProfile`, `UpdateProfile` |
| `packages/sdk/src/types.ts` | Mirror type changes |
| `packages/core/src/services/profile-service.ts` | Handle address + URLs in get/update |
| `packages/core/src/routes/profile.ts` | No changes (patch body is flexible) |
| `packages/sdk/src/resources/profile.ts` | No changes (passes body through) |
| `packages/core/src/services/resume-compiler.ts` | Read URLs from profile.urls instead of flat fields |
| `packages/webui/src/routes/config/profile/+page.svelte` | Replace form with address + URLs sections |
| `packages/core/src/__tests__/profile-schema.test.ts` | Migration + service tests |

---

### Task 1: Write the migration

**Files:**
- Create: `packages/core/src/db/migrations/046_profile_addresses_urls.sql`

- [ ] **Step 1: Write the migration SQL**

```sql
-- 046_profile_addresses_urls.sql
-- Create shared addresses table and profile_urls table.
-- Migrate existing profile location/linkedin/github/website into new tables.
-- Drop old columns from user_profile.

-- 1. Create addresses table
CREATE TABLE addresses (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  street_1 TEXT,
  street_2 TEXT,
  city TEXT,
  state TEXT,
  zip TEXT,
  country_code TEXT DEFAULT 'US',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

-- 2. Create profile_urls table
CREATE TABLE profile_urls (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  profile_id TEXT NOT NULL REFERENCES user_profile(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  url TEXT NOT NULL,
  position INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  UNIQUE(profile_id, key)
) STRICT;
CREATE INDEX idx_profile_urls_profile ON profile_urls(profile_id);

-- 3. Add address_id FK to user_profile (before dropping columns)
ALTER TABLE user_profile ADD COLUMN address_id TEXT REFERENCES addresses(id);

-- 4. Migrate existing location → addresses table
INSERT INTO addresses (id, name, created_at, updated_at)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  location,
  strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
  strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
FROM user_profile
WHERE location IS NOT NULL AND location != '';

-- 5. Link profile to newly created address
UPDATE user_profile SET address_id = (
  SELECT a.id FROM addresses a
  WHERE a.name = user_profile.location
  LIMIT 1
)
WHERE location IS NOT NULL AND location != '';

-- 6. Migrate linkedin, github, website → profile_urls
INSERT INTO profile_urls (id, profile_id, key, url, position, created_at)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  id, 'linkedin', linkedin, 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
FROM user_profile WHERE linkedin IS NOT NULL AND linkedin != '';

INSERT INTO profile_urls (id, profile_id, key, url, position, created_at)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  id, 'github', github, 1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
FROM user_profile WHERE github IS NOT NULL AND github != '';

INSERT INTO profile_urls (id, profile_id, key, url, position, created_at)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
  id, 'blog', website, 2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
FROM user_profile WHERE website IS NOT NULL AND website != '';

-- 7. Drop old columns (SQLite requires table rebuild)
CREATE TABLE user_profile_new (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  email TEXT,
  phone TEXT,
  address_id TEXT REFERENCES addresses(id),
  salary_minimum INTEGER,
  salary_target INTEGER,
  salary_stretch INTEGER,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

INSERT INTO user_profile_new (id, name, email, phone, address_id, salary_minimum, salary_target, salary_stretch, created_at, updated_at)
SELECT id, name, email, phone, address_id, salary_minimum, salary_target, salary_stretch, created_at, updated_at
FROM user_profile;

DROP TABLE user_profile;
ALTER TABLE user_profile_new RENAME TO user_profile;
```

- [ ] **Step 2: Run the migration**

Run: `just migrate`
Expected: Migration 046 applies cleanly.

- [ ] **Step 3: Verify data migrated**

```bash
sqlite3 data/forge.db "SELECT name FROM addresses;"
sqlite3 data/forge.db "SELECT key, url FROM profile_urls;"
sqlite3 data/forge.db "SELECT address_id FROM user_profile;"
sqlite3 data/forge.db ".schema user_profile" | grep -c "linkedin\|github\|website\|location"
```

Expected: Address "Reston, VA" exists. Three profile_urls (linkedin, github, blog). address_id is set. Zero old columns remain.

- [ ] **Step 4: Commit**

```bash
git add packages/core/src/db/migrations/046_profile_addresses_urls.sql
git commit -m "feat(db): migration 046 - addresses table, profile_urls table, drop flat profile fields"
```

---

### Task 2: Update core types

**Files:**
- Modify: `packages/core/src/types/index.ts:795-818`

- [ ] **Step 1: Add Address and ProfileUrl types**

After the existing types section, add:

```ts
// ── Addresses ─────────────────────────────────────────────────────────

/** Shared address entity. Referenced by user_profile, and (future) org campuses. */
export interface Address {
  id: string
  name: string
  street_1: string | null
  street_2: string | null
  city: string | null
  state: string | null
  zip: string | null
  country_code: string
  created_at: string
  updated_at: string
}

export interface CreateAddress {
  name: string
  street_1?: string | null
  street_2?: string | null
  city?: string | null
  state?: string | null
  zip?: string | null
  country_code?: string
}

export interface UpdateAddress {
  name?: string
  street_1?: string | null
  street_2?: string | null
  city?: string | null
  state?: string | null
  zip?: string | null
  country_code?: string
}

// ── Profile URLs ──────────────────────────────────────────────────────

export interface ProfileUrl {
  id: string
  profile_id: string
  key: string
  url: string
  position: number
  created_at: string
}

export const WELL_KNOWN_URL_KEYS = ['linkedin', 'github', 'gitlab', 'indeed', 'blog', 'portfolio'] as const
export type WellKnownUrlKey = typeof WELL_KNOWN_URL_KEYS[number]
```

- [ ] **Step 2: Update UserProfile interface**

Replace the existing `UserProfile` interface:

```ts
export interface UserProfile {
  id: string
  name: string
  email: string | null
  phone: string | null
  address_id: string | null
  address: Address | null
  urls: ProfileUrl[]
  salary_minimum: number | null
  salary_target: number | null
  salary_stretch: number | null
  created_at: string
  updated_at: string
}
```

- [ ] **Step 3: Update UpdateProfile type**

```ts
export interface UpdateProfile {
  name?: string | null
  email?: string | null
  phone?: string | null
  address_id?: string | null
  address?: CreateAddress | UpdateAddress
  urls?: Array<{ key: string; url: string }>
  salary_minimum?: number | null
  salary_target?: number | null
  salary_stretch?: number | null
}
```

- [ ] **Step 4: Commit**

```bash
git add packages/core/src/types/index.ts
git commit -m "feat(types): add Address, ProfileUrl types; update UserProfile"
```

---

### Task 3: Update SDK types

**Files:**
- Modify: `packages/sdk/src/types.ts:972-991`

- [ ] **Step 1: Mirror the type changes from Task 2**

Add `Address`, `ProfileUrl`, `WELL_KNOWN_URL_KEYS`, `WellKnownUrlKey` types. Update `UserProfile` to match core. Update `UpdateProfile` to match core.

Use the exact same interfaces as Task 2 — both packages must be in sync.

- [ ] **Step 2: Commit**

```bash
git add packages/sdk/src/types.ts
git commit -m "feat(sdk): mirror Address, ProfileUrl, UserProfile type changes"
```

---

### Task 4: Update ProfileService

**Files:**
- Modify: `packages/core/src/services/profile-service.ts`

- [ ] **Step 1: Update getProfile to JOIN address and urls**

Replace `fetchSingleton` to populate `address` and `urls`:

```ts
private async fetchSingleton(): Promise<UserProfile | null> {
  const listResult = await this.elm.list('user_profile', { limit: 1 })
  if (!listResult.ok) return null
  const row = listResult.value.rows[0] as Record<string, unknown> | undefined
  if (!row) return null

  // Fetch address
  let address: Address | null = null
  if (row.address_id) {
    const addrResult = await this.elm.get('addresses', row.address_id as string)
    if (addrResult.ok) address = addrResult.value as unknown as Address
  }

  // Fetch URLs
  const urlsResult = await this.elm.list('profile_urls', {
    filter: { profile_id: row.id as string },
    sort: { field: 'position', direction: 'asc' },
  })
  const urls: ProfileUrl[] = urlsResult.ok
    ? (urlsResult.value.rows as unknown as ProfileUrl[])
    : []

  return {
    ...(row as unknown as Omit<UserProfile, 'address' | 'urls'>),
    address,
    urls,
  } as UserProfile
}
```

- [ ] **Step 2: Update updateProfile to handle address and urls**

In `updateProfile`, after existing salary validation and before the ELM update call, add handling for `address` and `urls`:

```ts
// Handle inline address create/update
if (patch.address !== undefined) {
  const current = await this.fetchSingleton()
  if (!current) return { ok: false, error: { code: 'NOT_FOUND', message: 'Profile not found' } }

  if (current.address_id) {
    // Update existing address
    await this.elm.update('addresses', current.address_id, patch.address)
  } else {
    // Create new address
    const addrId = crypto.randomUUID()
    const addrData = { id: addrId, ...patch.address, country_code: patch.address.country_code ?? 'US' }
    await this.elm.create('addresses', addrData)
    data.address_id = addrId
  }
}

// Handle URL array replace
if (patch.urls !== undefined) {
  const current = await this.fetchSingleton()
  if (!current) return { ok: false, error: { code: 'NOT_FOUND', message: 'Profile not found' } }

  // Validate: no duplicate keys, custom keys don't collide with well-known
  const keys = patch.urls.map(u => u.key.toLowerCase())
  const uniqueKeys = new Set(keys)
  if (uniqueKeys.size !== keys.length) {
    return { ok: false, error: { code: 'VALIDATION_ERROR', message: 'Duplicate URL keys' } }
  }

  // Delete existing URLs
  for (const existing of current.urls) {
    await this.elm.delete('profile_urls', existing.id)
  }

  // Insert new URLs
  for (let i = 0; i < patch.urls.length; i++) {
    const u = patch.urls[i]
    await this.elm.create('profile_urls', {
      id: crypto.randomUUID(),
      profile_id: current.id,
      key: u.key,
      url: u.url,
      position: i,
    })
  }
}
```

- [ ] **Step 3: Update the allowed fields list**

Replace the `allowed` array — remove `location`, `linkedin`, `github`, `website`, add `address_id`:

```ts
const allowed = [
  'name',
  'email',
  'phone',
  'address_id',
  'salary_minimum',
  'salary_target',
  'salary_stretch',
] as const
```

- [ ] **Step 4: Add Address and ProfileUrl imports**

```ts
import type { UserProfile, UpdateProfile, Address, ProfileUrl, Result } from '../types'
```

- [ ] **Step 5: Register new entities in ELM entity map**

Check `packages/core/src/storage/entity-map.ts` (or wherever the entity map is defined). Add entries for `addresses` and `profile_urls` if not already present.

- [ ] **Step 6: Run existing tests**

Run: `bun test packages/core/`
Expected: Existing profile tests may fail due to schema changes — that's expected, they'll be updated in Task 7.

- [ ] **Step 7: Commit**

```bash
git add packages/core/src/services/profile-service.ts
git commit -m "feat(profile): handle address + urls in get/update"
```

---

### Task 5: Update resume compiler

**Files:**
- Modify: `packages/core/src/services/resume-compiler.ts:310-333`

- [ ] **Step 1: Update the contact fields warning**

Replace line 312:

```ts
// Before:
if (profile && !profile.email && !profile.phone && !profile.linkedin && !profile.github) {

// After:
const hasContactUrl = profile?.urls?.some(u => ['linkedin', 'github'].includes(u.key))
if (profile && !profile.email && !profile.phone && !hasContactUrl) {
```

- [ ] **Step 2: Update the header construction**

Replace lines 328-332 where the header is built:

```ts
// Before:
linkedin: profile?.linkedin ?? null,
github: profile?.github ?? null,
website: profile?.website ?? null,

// After:
linkedin: profile?.urls?.find(u => u.key === 'linkedin')?.url ?? null,
github: profile?.urls?.find(u => u.key === 'github')?.url ?? null,
website: profile?.urls?.find(u => u.key === 'blog')?.url ?? profile?.urls?.find(u => u.key === 'portfolio')?.url ?? null,
```

- [ ] **Step 3: Update location in header**

Find where `profile?.location` is used in the header and replace:

```ts
// Before:
location: profile?.location ?? null,

// After:
location: profile?.address?.name ?? null,
```

- [ ] **Step 4: Commit**

```bash
git add packages/core/src/services/resume-compiler.ts
git commit -m "feat(compiler): read profile URLs and address from new tables"
```

---

### Task 6: Update WebUI profile page

**Files:**
- Modify: `packages/webui/src/routes/config/profile/+page.svelte`

- [ ] **Step 1: Update the form state**

Replace the form initialization:

```ts
let form = $state<Record<string, string | null>>({
  name: '',
  email: null,
  phone: null,
})

// Address fields
let address = $state<{
  name: string
  street_1: string
  street_2: string
  city: string
  state: string
  zip: string
  country_code: string
}>({
  name: '', street_1: '', street_2: '', city: '', state: '', zip: '', country_code: 'US',
})

// URL fields
let urls = $state<Array<{ key: string; url: string }>>([])
let newUrlKey = $state('')
let newCustomKey = $state('')
let newCustomUrl = $state('')

const WELL_KNOWN_URL_KEYS = ['linkedin', 'github', 'gitlab', 'indeed', 'blog', 'portfolio']
const WELL_KNOWN_LABELS: Record<string, string> = {
  linkedin: 'LinkedIn', github: 'GitHub', gitlab: 'GitLab',
  indeed: 'Indeed', blog: 'Blog', portfolio: 'Portfolio',
}
```

- [ ] **Step 2: Update loadProfile**

```ts
async function loadProfile() {
  loading = true
  const result = await forge.profile.get()
  if (result.ok) {
    profile = result.data
    form = {
      name: result.data.name,
      email: result.data.email,
      phone: result.data.phone,
    }
    if (result.data.address) {
      address = {
        name: result.data.address.name ?? '',
        street_1: result.data.address.street_1 ?? '',
        street_2: result.data.address.street_2 ?? '',
        city: result.data.address.city ?? '',
        state: result.data.address.state ?? '',
        zip: result.data.address.zip ?? '',
        country_code: result.data.address.country_code ?? 'US',
      }
    }
    urls = result.data.urls.map(u => ({ key: u.key, url: u.url }))
    salaryMinimum = result.data.salary_minimum ?? null
    salaryTarget = result.data.salary_target ?? null
    salaryStretch = result.data.salary_stretch ?? null
  } else {
    addToast({ message: friendlyError(result.error), type: 'error' })
  }
  loading = false
}
```

- [ ] **Step 3: Update handleSave to include address and urls**

Add address and urls to the patch object:

```ts
async function handleSave() {
  saving = true
  try {
    const patch: Record<string, unknown> = {}

    // Text fields
    for (const [key, value] of Object.entries(form)) {
      if (profile && value !== (profile as Record<string, unknown>)[key]) {
        if (key === 'name') patch[key] = value
        else patch[key] = value === '' ? null : value
      }
    }

    // Address (always send if any field has content)
    const hasAddress = Object.values(address).some(v => v !== '' && v !== 'US')
    if (hasAddress || profile?.address) {
      patch.address = {
        name: address.name || null,
        street_1: address.street_1 || null,
        street_2: address.street_2 || null,
        city: address.city || null,
        state: address.state || null,
        zip: address.zip || null,
        country_code: address.country_code || 'US',
      }
    }

    // URLs (always send full array)
    patch.urls = urls.filter(u => u.url.trim() !== '')

    // Salary
    if (profile && salaryMinimum !== (profile.salary_minimum ?? null)) patch.salary_minimum = salaryMinimum
    if (profile && salaryTarget !== (profile.salary_target ?? null)) patch.salary_target = salaryTarget
    if (profile && salaryStretch !== (profile.salary_stretch ?? null)) patch.salary_stretch = salaryStretch

    if (Object.keys(patch).length === 0) { saving = false; return }

    const result = await forge.profile.update(patch as any)
    if (result.ok) {
      profile = result.data
      addToast({ message: 'Profile saved', type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  } finally {
    saving = false
  }
}
```

- [ ] **Step 4: Replace the form template**

Replace the form grid with three sections — Personal Info, Address, URLs:

```svelte
<form class="profile-form" onsubmit={(e) => { e.preventDefault(); handleSave() }}>
  <!-- Personal Info -->
  <h3 class="section-heading">Personal Information</h3>
  <div class="form-grid">
    <div class="form-field full-width">
      <label for="pf-name">Name <span class="required">*</span></label>
      <input id="pf-name" type="text" bind:value={form.name} required oninput={scheduleAutosave} />
    </div>
    <div class="form-field">
      <label for="pf-email">Email</label>
      <input id="pf-email" type="email" bind:value={form.email} placeholder="adam@example.com" oninput={scheduleAutosave} />
    </div>
    <div class="form-field">
      <label for="pf-phone">Phone</label>
      <input id="pf-phone" type="tel" bind:value={form.phone} placeholder="+1-555-0123" oninput={scheduleAutosave} />
    </div>
  </div>

  <!-- Address -->
  <h3 class="section-heading">Address</h3>
  <div class="form-grid">
    <div class="form-field full-width">
      <label for="pf-addr-name">Display Location</label>
      <input id="pf-addr-name" type="text" bind:value={address.name} placeholder="Reston, VA" oninput={scheduleAutosave} />
      <span class="field-hint">This is what appears on your resumes.</span>
    </div>
    <div class="form-field">
      <label for="pf-addr-street1">Street 1</label>
      <input id="pf-addr-street1" type="text" bind:value={address.street_1} oninput={scheduleAutosave} />
    </div>
    <div class="form-field">
      <label for="pf-addr-street2">Street 2</label>
      <input id="pf-addr-street2" type="text" bind:value={address.street_2} oninput={scheduleAutosave} />
    </div>
    <div class="form-field">
      <label for="pf-addr-city">City</label>
      <input id="pf-addr-city" type="text" bind:value={address.city} oninput={scheduleAutosave} />
    </div>
    <div class="form-field">
      <label for="pf-addr-state">State</label>
      <input id="pf-addr-state" type="text" bind:value={address.state} placeholder="VA" oninput={scheduleAutosave} />
    </div>
    <div class="form-field">
      <label for="pf-addr-zip">Zip</label>
      <input id="pf-addr-zip" type="text" bind:value={address.zip} oninput={scheduleAutosave} />
    </div>
    <div class="form-field">
      <label for="pf-addr-country">Country</label>
      <input id="pf-addr-country" type="text" bind:value={address.country_code} placeholder="US" maxlength="2" oninput={scheduleAutosave} />
    </div>
  </div>

  <!-- URLs -->
  <h3 class="section-heading">URLs</h3>
  <div class="url-list">
    {#each urls as urlEntry, i}
      <div class="url-row">
        <span class="url-label">{WELL_KNOWN_LABELS[urlEntry.key] ?? urlEntry.key}</span>
        <input type="url" bind:value={urlEntry.url} placeholder="https://..." oninput={scheduleAutosave} />
        <button type="button" class="btn btn-ghost btn-sm" onclick={() => { urls = urls.filter((_, j) => j !== i); scheduleAutosave() }}>x</button>
      </div>
    {/each}
  </div>

  <!-- Add well-known URL -->
  {#if WELL_KNOWN_URL_KEYS.filter(k => !urls.some(u => u.key === k)).length > 0}
    <div class="url-add">
      <select bind:value={newUrlKey}>
        <option value="">+ Add URL...</option>
        {#each WELL_KNOWN_URL_KEYS.filter(k => !urls.some(u => u.key === k)) as k}
          <option value={k}>{WELL_KNOWN_LABELS[k]}</option>
        {/each}
      </select>
      {#if newUrlKey}
        <button type="button" class="btn btn-ghost btn-sm" onclick={() => { urls = [...urls, { key: newUrlKey, url: '' }]; newUrlKey = '' }}>Add</button>
      {/if}
    </div>
  {/if}

  <!-- Add custom URL -->
  <div class="url-add-custom">
    <input type="text" bind:value={newCustomKey} placeholder="Label (e.g., Kaggle)" />
    <input type="url" bind:value={newCustomUrl} placeholder="https://..." />
    <button type="button" class="btn btn-ghost btn-sm" disabled={!newCustomKey.trim() || !newCustomUrl.trim() || WELL_KNOWN_URL_KEYS.includes(newCustomKey.toLowerCase())}
      onclick={() => { urls = [...urls, { key: newCustomKey.trim(), url: newCustomUrl.trim() }]; newCustomKey = ''; newCustomUrl = ''; scheduleAutosave() }}>
      + Custom
    </button>
  </div>

  <!-- Salary -->
  <h3 class="section-heading">Salary Expectations</h3>
  <div class="form-grid">
    <div class="form-field">
      <label for="pf-sal-min">Minimum Acceptable ($)</label>
      <input id="pf-sal-min" type="number" bind:value={salaryMinimum} placeholder="120000" step="1000" oninput={scheduleAutosave} />
    </div>
    <div class="form-field">
      <label for="pf-sal-target">Target ($)</label>
      <input id="pf-sal-target" type="number" bind:value={salaryTarget} placeholder="160000" step="1000" oninput={scheduleAutosave} />
    </div>
    <div class="form-field">
      <label for="pf-sal-stretch">Stretch ($)</label>
      <input id="pf-sal-stretch" type="number" bind:value={salaryStretch} placeholder="200000" step="1000" oninput={scheduleAutosave} />
    </div>
  </div>

  <div class="form-actions">
    <button class="btn btn-primary" type="submit" disabled={saving}>
      {saving ? 'Saving...' : 'Save Profile'}
    </button>
  </div>
</form>
```

Note: Security Clearance field is removed entirely.

- [ ] **Step 5: Add CSS for URL rows**

```css
.url-list { display: flex; flex-direction: column; gap: 0.5rem; margin-bottom: 0.75rem; }
.url-row { display: flex; align-items: center; gap: 0.5rem; }
.url-label { min-width: 80px; font-size: var(--text-sm); font-weight: var(--font-medium); color: var(--text-secondary); }
.url-row input { flex: 1; }
.url-add { display: flex; gap: 0.5rem; margin-bottom: 0.5rem; }
.url-add select { font-size: var(--text-sm); }
.url-add-custom { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
.url-add-custom input { flex: 1; }
.field-hint { font-size: var(--text-xs); color: var(--text-muted); margin-top: 0.25rem; }
```

- [ ] **Step 6: Commit**

```bash
git add packages/webui/src/routes/config/profile/+page.svelte
git commit -m "feat(webui): profile page with address, URLs, clearance removed"
```

---

### Task 7: Tests

**Files:**
- Create: `packages/core/src/__tests__/profile-schema.test.ts`

- [ ] **Step 1: Write migration verification test**

```ts
import { describe, test, expect, beforeAll } from 'bun:test'
import { Database } from 'bun:sqlite'
import { runMigrations } from '../db/migrate'

describe('Migration 046: profile addresses + urls', () => {
  let db: Database

  beforeAll(() => {
    db = new Database(':memory:')
    runMigrations(db)
  })

  test('addresses table exists with expected columns', () => {
    const cols = db.prepare("PRAGMA table_info(addresses)").all() as { name: string }[]
    const names = cols.map(c => c.name)
    expect(names).toContain('name')
    expect(names).toContain('street_1')
    expect(names).toContain('city')
    expect(names).toContain('country_code')
  })

  test('profile_urls table exists with expected columns', () => {
    const cols = db.prepare("PRAGMA table_info(profile_urls)").all() as { name: string }[]
    const names = cols.map(c => c.name)
    expect(names).toContain('profile_id')
    expect(names).toContain('key')
    expect(names).toContain('url')
    expect(names).toContain('position')
  })

  test('user_profile has address_id but no linkedin/github/website/location', () => {
    const cols = db.prepare("PRAGMA table_info(user_profile)").all() as { name: string }[]
    const names = cols.map(c => c.name)
    expect(names).toContain('address_id')
    expect(names).not.toContain('linkedin')
    expect(names).not.toContain('github')
    expect(names).not.toContain('website')
    expect(names).not.toContain('location')
  })

  test('profile_urls has unique constraint on (profile_id, key)', () => {
    const profileId = 'test-profile-id-0000-000000000001'
    db.run("INSERT INTO user_profile (id, name) VALUES (?, ?)", [profileId, 'Test'])
    db.run("INSERT INTO profile_urls (id, profile_id, key, url, position) VALUES (?, ?, 'github', 'https://github.com/test', 0)", ['url-1', profileId])
    expect(() => {
      db.run("INSERT INTO profile_urls (id, profile_id, key, url, position) VALUES (?, ?, 'github', 'https://github.com/test2', 1)", ['url-2', profileId])
    }).toThrow()
  })
})
```

- [ ] **Step 2: Run tests**

Run: `bun test packages/core/src/__tests__/profile-schema.test.ts`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/__tests__/profile-schema.test.ts
git commit -m "test(profile): migration 046 verification tests"
```

---

### Task 8: Register entities in ELM

**Files:**
- Modify: `packages/core/src/storage/entity-map.ts` (or equivalent)

- [ ] **Step 1: Add addresses and profile_urls to the entity map**

Add entries for both new tables so the ELM can operate on them:

```ts
addresses: {
  table: 'addresses',
  idField: 'id',
  timestampFields: { created: 'created_at', updated: 'updated_at' },
},
profile_urls: {
  table: 'profile_urls',
  idField: 'id',
  timestampFields: { created: 'created_at' },
},
```

- [ ] **Step 2: Run tests**

Run: `just test`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add packages/core/src/storage/entity-map.ts
git commit -m "feat(storage): register addresses and profile_urls in ELM entity map"
```

---

## Summary

| Task | What | Dependencies |
|------|------|-------------|
| 1 | Migration (tables, data migration, column drops) | None |
| 2 | Core types (Address, ProfileUrl, UserProfile) | None |
| 3 | SDK types (mirror) | Task 2 |
| 4 | ProfileService (get/update with joins) | Tasks 1, 2, 8 |
| 5 | Resume compiler (read from urls/address) | Tasks 2, 4 |
| 6 | WebUI profile page | Tasks 3, 4 |
| 7 | Tests | Tasks 1, 4 |
| 8 | ELM entity map registration | Task 1 |

**Parallel tracks:**
```
Task 1 (migration) + Task 2 (core types)
         ↓                    ↓
Task 8 (ELM) ──→ Task 4 (service) ──→ Task 5 (compiler)
                      ↓
Task 3 (SDK types) → Task 6 (WebUI)
                      ↓
                  Task 7 (tests)
```
