//! Address repository — CRUD for the `addresses` table.
//!
//! Addresses are shared entities referenced by `user_profile` and
//! `org_locations`.

use rusqlite::{params, Connection, OptionalExtension};

use forge_core::{new_id, now_iso, Address, CreateAddress, ForgeError, Pagination, UpdateAddress};

/// Data-access store for the `addresses` table.
pub struct AddressStore;

impl AddressStore {
    // ── Create ───────────────────────────────────────────────────────

    /// Insert a new address row.
    pub fn create(conn: &Connection, input: &CreateAddress) -> Result<Address, ForgeError> {
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

        Self::get(conn, &id)?
            .ok_or_else(|| ForgeError::Internal("Address created but not found".into()))
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Fetch a single address by ID.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Address>, ForgeError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, street_1, street_2, city, state, zip, country_code,
                    created_at, updated_at
             FROM addresses WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], Self::map_address).optional()?;
        Ok(result)
    }

    /// List addresses with pagination.
    pub fn list(
        conn: &Connection,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<Address>, Pagination), ForgeError> {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM addresses", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, street_1, street_2, city, state, zip, country_code,
                    created_at, updated_at
             FROM addresses
             ORDER BY name ASC
             LIMIT ?1 OFFSET ?2",
        )?;

        let rows: Vec<Address> = stmt
            .query_map(params![limit, offset], Self::map_address)?
            .collect::<Result<_, _>>()?;

        Ok((
            rows,
            Pagination {
                total,
                offset,
                limit,
            },
        ))
    }

    // ── Update ───────────────────────────────────────────────────────

    /// Apply a partial update to an existing address.
    pub fn update(
        conn: &Connection,
        id: &str,
        input: &UpdateAddress,
    ) -> Result<Address, ForgeError> {
        Self::get(conn, id)?.ok_or_else(|| ForgeError::NotFound {
            entity_type: "address".into(),
            id: id.into(),
        })?;

        let mut sets = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = input.name {
            sets.push(format!("name = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.street_1 {
            sets.push(format!("street_1 = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.street_2 {
            sets.push(format!("street_2 = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.city {
            sets.push(format!("city = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.state {
            sets.push(format!("state = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.zip {
            sets.push(format!("zip = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = input.country_code {
            sets.push(format!("country_code = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(v.clone()));
        }

        if !sets.is_empty() {
            let now = now_iso();
            sets.push(format!("updated_at = ?{}", bind_values.len() + 1));
            bind_values.push(Box::new(now));

            let sql = format!(
                "UPDATE addresses SET {} WHERE id = ?{}",
                sets.join(", "),
                bind_values.len() + 1
            );
            bind_values.push(Box::new(id.to_string()));

            conn.execute(
                &sql,
                rusqlite::params_from_iter(bind_values.iter().map(|b| b.as_ref())),
            )?;
        }

        Self::get(conn, id)?
            .ok_or_else(|| ForgeError::Internal("Address updated but not found".into()))
    }

    // ── Delete ───────────────────────────────────────────────────────

    /// Delete an address by ID.
    pub fn delete(conn: &Connection, id: &str) -> Result<(), ForgeError> {
        let deleted = conn.execute("DELETE FROM addresses WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(ForgeError::NotFound {
                entity_type: "address".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    // ── Row mapping ──────────────────────────────────────────────────

    fn map_address(row: &rusqlite::Row) -> rusqlite::Result<Address> {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Forge;

    fn setup() -> Forge {
        Forge::open_memory().unwrap()
    }

    fn sample_input() -> CreateAddress {
        CreateAddress {
            name: "Home".into(),
            street_1: Some("123 Main St".into()),
            street_2: Some("Apt 4".into()),
            city: Some("Springfield".into()),
            state: Some("IL".into()),
            zip: Some("62704".into()),
            country_code: Some("US".into()),
        }
    }

    #[test]
    fn create_and_get() {
        let forge = setup();
        let addr = AddressStore::create(forge.conn(), &sample_input()).unwrap();
        assert_eq!(addr.name, "Home");
        assert_eq!(addr.street_1, Some("123 Main St".into()));
        assert_eq!(addr.country_code, "US");

        let fetched = AddressStore::get(forge.conn(), &addr.id).unwrap().unwrap();
        assert_eq!(fetched.id, addr.id);
        assert_eq!(fetched.name, "Home");
    }

    #[test]
    fn list_with_pagination() {
        let forge = setup();
        AddressStore::create(
            forge.conn(),
            &CreateAddress {
                name: "Office".into(),
                city: Some("Chicago".into()),
                country_code: Some("US".into()),
                ..sample_input()
            },
        )
        .unwrap();
        AddressStore::create(forge.conn(), &sample_input()).unwrap();

        let (rows, pagination) = AddressStore::list(forge.conn(), 0, 50).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(pagination.total, 2);

        // Pagination: limit 1
        let (rows, pagination) = AddressStore::list(forge.conn(), 0, 1).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(pagination.total, 2);
    }

    #[test]
    fn update_address() {
        let forge = setup();
        let created = AddressStore::create(forge.conn(), &sample_input()).unwrap();

        let updated = AddressStore::update(
            forge.conn(),
            &created.id,
            &UpdateAddress {
                name: Some("Work".into()),
                city: Some(Some("Chicago".into())),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.name, "Work");
        assert_eq!(updated.city, Some("Chicago".into()));
        // Unchanged fields preserved
        assert_eq!(updated.street_1, Some("123 Main St".into()));
    }

    #[test]
    fn delete_address() {
        let forge = setup();
        let created = AddressStore::create(forge.conn(), &sample_input()).unwrap();
        AddressStore::delete(forge.conn(), &created.id).unwrap();
        assert!(AddressStore::get(forge.conn(), &created.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn get_not_found() {
        let forge = setup();
        let result = AddressStore::get(forge.conn(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let forge = setup();
        let result = AddressStore::delete(forge.conn(), "nonexistent");
        assert!(matches!(result, Err(ForgeError::NotFound { .. })));
    }
}
