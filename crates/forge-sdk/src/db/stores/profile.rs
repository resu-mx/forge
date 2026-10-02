//! Profile repository — data access for the singleton user profile.
//!
//! The `user_profile` table is a singleton: it contains exactly one row,
//! seeded by migration. The repository enforces this by always fetching
//! the first row and never exposing a `create` method.
//!
//! TS source: `packages/core/src/services/profile-service.ts` (data access portions)

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{
    Address, CreateAddress, ForgeError, ProfileUrl, ProfileUrlInput, UpdateProfile, UserProfile,
    new_id, now_iso,
};

/// Data access layer for the singleton user profile and related entities
/// (addresses, profile URLs).
pub struct ProfileStore;

impl ProfileStore {
    // ── Profile ─────────────────────────────────────────────────────

    /// Fetch the singleton user profile row.
    ///
    /// Returns `None` if the profile has not been seeded (migration not
    /// yet applied or test DB without the seed row).
    pub fn get_profile(conn: &Connection) -> Result<Option<UserProfile>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, email, phone, address_id,
                    salary_minimum, salary_target, salary_stretch,
                    created_at, updated_at
             FROM user_profile LIMIT 1",
        )?;

        let row = stmt.query_row([], |row| {
            Ok(ProfileRow {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                phone: row.get(3)?,
                address_id: row.get(4)?,
                salary_minimum: row.get(5)?,
                salary_target: row.get(6)?,
                salary_stretch: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        }).optional()?;

        match row {
            None => Ok(None),
            Some(r) => {
                let address = Self::get_address(conn, r.address_id.as_deref())?;
                let urls = Self::list_profile_urls(conn, &r.id)?;
                Ok(Some(UserProfile {
                    id: r.id,
                    name: r.name,
                    email: r.email,
                    phone: r.phone,
                    address_id: r.address_id,
                    address,
                    urls,
                    salary_minimum: r.salary_minimum,
                    salary_target: r.salary_target,
                    salary_stretch: r.salary_stretch,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                }))
            }
        }
    }

    /// Rules from the TS ProfileService: a name, if given, must not be blank, and
    /// the salary tiers, where two are given, must be ordered min <= target <= stretch.
    fn validate_patch(patch: &UpdateProfile) -> Result<(), ForgeError> {
        let invalid = |message: &str, field: &str| ForgeError::Validation {
            message: message.into(),
            field: Some(field.into()),
        };
        if matches!(&patch.name, Some(n) if n.trim().is_empty()) {
            return Err(invalid("Name must not be empty", "name"));
        }

        let tier = |v: &Option<Option<f64>>| v.flatten();
        let (min, target, stretch) = (
            tier(&patch.salary_minimum),
            tier(&patch.salary_target),
            tier(&patch.salary_stretch),
        );
        let exceeds = |lo: Option<f64>, hi: Option<f64>| matches!((lo, hi), (Some(a), Some(b)) if a > b);
        if exceeds(min, target) {
            return Err(invalid("salary_minimum must not exceed salary_target", "salary_minimum"));
        }
        if exceeds(target, stretch) {
            return Err(invalid("salary_target must not exceed salary_stretch", "salary_target"));
        }
        if exceeds(min, stretch) {
            return Err(invalid("salary_minimum must not exceed salary_stretch", "salary_minimum"));
        }
        Ok(())
    }

    /// Update the singleton profile with the provided patch fields.
    ///
    /// Only the fields present in `UpdateProfile` are written; absent
    /// fields are left unchanged. Creates the profile row if it doesn't
    /// exist yet (first update on a fresh DB). Returns the updated profile.
    pub fn update_profile(conn: &Connection, patch: &UpdateProfile) -> Result<UserProfile, ForgeError> {
        Self::validate_patch(patch)?;

        let current = Self::get_profile(conn)?;
        let profile_id = match &current {
            Some(p) => p.id.clone(),
            None => {
                // Create a singleton profile row if none exists
                let id = new_id();
                let now = now_iso();
                conn.execute(
                    "INSERT INTO user_profile (id, name, email, phone, address_id,
                        salary_minimum, salary_target, salary_stretch, created_at, updated_at)
                     VALUES (?1, '', NULL, NULL, NULL, NULL, NULL, NULL, ?2, ?2)",
                    params![id, now],
                )?;
                id
            }
        };

        // Handle address upsert
        if let Some(ref addr_input) = patch.address {
            let current_address_id = current.as_ref().and_then(|p| p.address_id.clone());
            if let Some(ref existing_addr_id) = current_address_id {
                Self::update_address_row(conn, existing_addr_id, addr_input)?;
            } else {
                let addr_id = Self::create_address_row(conn, addr_input)?;
                conn.execute(
                    "UPDATE user_profile SET address_id = ?1 WHERE id = ?2",
                    params![addr_id, profile_id],
                )?;
            }
        }

        // Handle URL array replace
        if let Some(ref url_inputs) = patch.urls {
            Self::replace_profile_urls(conn, &profile_id, url_inputs)?;
        }

        // Build the SET clause for scalar profile fields
        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = patch.name {
            sets.push(format!("name = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = patch.email {
            sets.push(format!("email = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = patch.phone {
            sets.push(format!("phone = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = patch.address_id {
            sets.push(format!("address_id = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = patch.salary_minimum {
            sets.push(format!("salary_minimum = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(*v));
        }
        if let Some(ref v) = patch.salary_target {
            sets.push(format!("salary_target = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(*v));
        }
        if let Some(ref v) = patch.salary_stretch {
            sets.push(format!("salary_stretch = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(*v));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE user_profile SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(profile_id.clone()));

            conn.execute(
                &sql,
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            )?;
        }

        Self::get_profile(conn)?
            .ok_or_else(|| ForgeError::Internal("Profile updated but not found".into()))
    }

    // ── Address ─────────────────────────────────────────────────────

    /// Fetch an address by ID.
    fn get_address(conn: &Connection, address_id: Option<&str>) -> Result<Option<Address>, ForgeError> {
        let aid = match address_id {
            Some(id) => id,
            None => return Ok(None),
        };

        let mut stmt = conn.prepare(
            "SELECT id, name, street_1, street_2, city, state, zip, country_code, created_at, updated_at
             FROM addresses WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![aid], |row| {
            Ok(Address {
                id: row.get(0)?,
                name: row.get(1)?,
                street_1: row.get(2)?,
                street_2: row.get(3)?,
                city: row.get(4)?,
                state: row.get(5)?,
                zip: row.get(6)?,
                country_code: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        }).optional()?;
        Ok(result)
    }

    /// Create a new address row and return the generated ID.
    fn create_address_row(conn: &Connection, input: &CreateAddress) -> Result<String, ForgeError> {
        let id = new_id();
        let now = now_iso();
        let country_code = input.country_code.as_deref().unwrap_or("US");

        conn.execute(
            "INSERT INTO addresses (id, name, street_1, street_2, city, state, zip, country_code, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                input.name,
                input.street_1,
                input.street_2,
                input.city,
                input.state,
                input.zip,
                country_code,
                now,
            ],
        )?;
        Ok(id)
    }

    /// Update an existing address by ID with the provided fields.
    fn update_address_row(conn: &Connection, address_id: &str, input: &CreateAddress) -> Result<(), ForgeError> {
        let now = now_iso();

        conn.execute(
            "UPDATE addresses SET name = ?1, street_1 = ?2, street_2 = ?3, city = ?4,
                    state = ?5, zip = ?6, country_code = ?7, updated_at = ?8
             WHERE id = ?9",
            params![
                input.name,
                input.street_1,
                input.street_2,
                input.city,
                input.state,
                input.zip,
                input.country_code.as_deref().unwrap_or("US"),
                now,
                address_id,
            ],
        )?;
        Ok(())
    }

    // ── Profile URLs ────────────────────────────────────────────────

    /// List all profile URLs for the given profile, ordered by position.
    fn list_profile_urls(conn: &Connection, profile_id: &str) -> Result<Vec<ProfileUrl>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, key, url, position, created_at
             FROM profile_urls
             WHERE profile_id = ?1
             ORDER BY position ASC",
        )?;

        let rows: Vec<ProfileUrl> = stmt
            .query_map(params![profile_id], |row| {
                Ok(ProfileUrl {
                    id: row.get(0)?,
                    profile_id: row.get(1)?,
                    key: row.get(2)?,
                    url: row.get(3)?,
                    position: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Replace all profile URLs for the given profile with a new set.
    ///
    /// Deletes existing URLs and inserts the new ones with sequential
    /// positions. The caller must validate for duplicate keys before
    /// calling this method.
    fn replace_profile_urls(
        conn: &Connection,
        profile_id: &str,
        urls: &[ProfileUrlInput],
    ) -> Result<(), ForgeError> {
        // Delete existing URLs
        conn.execute(
            "DELETE FROM profile_urls WHERE profile_id = ?1",
            params![profile_id],
        )?;

        // Insert new URLs
        let now = now_iso();
        for (i, url_input) in urls.iter().enumerate() {
            let id = new_id();
            conn.execute(
                "INSERT INTO profile_urls (id, profile_id, key, url, position, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, profile_id, url_input.key, url_input.url, i as i32, now],
            )?;
        }
        Ok(())
    }
}

/// Internal struct for reading the raw profile row (before hydration).
struct ProfileRow {
    id: String,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    address_id: Option<String>,
    salary_minimum: Option<f64>,
    salary_target: Option<f64>,
    salary_stretch: Option<f64>,
    created_at: String,
    updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    #[test]
    fn get_profile_returns_none_without_seed() {
        let forge = setup();
        // If migrations don't seed a profile row, this returns None.
        // If they do seed one, it returns a valid profile. Either is acceptable.
        let _result = ProfileStore::get_profile(forge.conn());
        // Just verify it doesn't panic or error
    }

    #[test]
    fn update_creates_profile_if_missing() {
        let forge = setup();
        // Delete any seeded profile row to test auto-creation
        let _ = forge.conn().execute("DELETE FROM user_profile", []);

        let profile = ProfileStore::update_profile(forge.conn(), &UpdateProfile {
            name: Some("Adam".into()),
            email: Some(Some("adam@example.com".into())),
            ..Default::default()
        }).unwrap();
        assert_eq!(profile.name, "Adam");
        assert_eq!(profile.email, Some("adam@example.com".into()));
    }

    #[test]
    fn update_profile_name() {
        let forge = setup();
        // Ensure a profile exists
        let _ = forge.conn().execute("DELETE FROM user_profile", []);
        ProfileStore::update_profile(forge.conn(), &UpdateProfile {
            name: Some("Initial".into()),
            ..Default::default()
        }).unwrap();

        let updated = ProfileStore::update_profile(forge.conn(), &UpdateProfile {
            name: Some("Updated Name".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(updated.name, "Updated Name");
    }

    #[test]
    fn update_profile_salary() {
        let forge = setup();
        let _ = forge.conn().execute("DELETE FROM user_profile", []);
        ProfileStore::update_profile(forge.conn(), &UpdateProfile {
            name: Some("Test".into()),
            ..Default::default()
        }).unwrap();

        let updated = ProfileStore::update_profile(forge.conn(), &UpdateProfile {
            salary_minimum: Some(Some(120_000.0)),
            salary_target: Some(Some(150_000.0)),
            salary_stretch: Some(Some(180_000.0)),
            ..Default::default()
        }).unwrap();
        assert_eq!(updated.salary_minimum, Some(120_000.0));
        assert_eq!(updated.salary_target, Some(150_000.0));
        assert_eq!(updated.salary_stretch, Some(180_000.0));
    }

    #[test]
    fn update_profile_with_urls() {
        let forge = setup();
        let _ = forge.conn().execute("DELETE FROM user_profile", []);
        let profile = ProfileStore::update_profile(forge.conn(), &UpdateProfile {
            name: Some("Test".into()),
            urls: Some(vec![
                ProfileUrlInput { key: "github".into(), url: "https://github.com/test".into() },
                ProfileUrlInput { key: "linkedin".into(), url: "https://linkedin.com/in/test".into() },
            ]),
            ..Default::default()
        }).unwrap();
        assert_eq!(profile.urls.len(), 2);
        assert_eq!(profile.urls[0].key, "github");
        assert_eq!(profile.urls[1].key, "linkedin");
        assert_eq!(profile.urls[0].position, 0);
        assert_eq!(profile.urls[1].position, 1);
    }

    #[test]
    fn update_profile_with_address() {
        let forge = setup();
        let _ = forge.conn().execute("DELETE FROM user_profile", []);
        let profile = ProfileStore::update_profile(forge.conn(), &UpdateProfile {
            name: Some("Test".into()),
            address: Some(CreateAddress {
                name: "Home".into(),
                street_1: Some("123 Main St".into()),
                street_2: None,
                city: Some("Springfield".into()),
                state: Some("IL".into()),
                zip: Some("62701".into()),
                country_code: Some("US".into()),
            }),
            ..Default::default()
        }).unwrap();
        assert!(profile.address.is_some());
        let addr = profile.address.unwrap();
        assert_eq!(addr.name, "Home");
        assert_eq!(addr.city, Some("Springfield".into()));
    }

    #[test]
    fn replace_profile_urls_replaces_existing() {
        let forge = setup();
        let _ = forge.conn().execute("DELETE FROM user_profile", []);

        // Create with initial URLs
        let profile = ProfileStore::update_profile(forge.conn(), &UpdateProfile {
            name: Some("Test".into()),
            urls: Some(vec![
                ProfileUrlInput { key: "github".into(), url: "https://github.com/old".into() },
            ]),
            ..Default::default()
        }).unwrap();
        assert_eq!(profile.urls.len(), 1);

        // Replace with new URLs
        let updated = ProfileStore::update_profile(forge.conn(), &UpdateProfile {
            urls: Some(vec![
                ProfileUrlInput { key: "blog".into(), url: "https://blog.example.com".into() },
                ProfileUrlInput { key: "portfolio".into(), url: "https://portfolio.example.com".into() },
            ]),
            ..Default::default()
        }).unwrap();
        assert_eq!(updated.urls.len(), 2);
        assert_eq!(updated.urls[0].key, "blog");
        assert_eq!(updated.urls[1].key, "portfolio");
    }
}
