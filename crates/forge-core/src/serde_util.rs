//! Serde helpers shared by the input types.

use serde::{Deserialize, Deserializer};

/// Deserialize `Option<Option<T>>` so that an absent field and an explicit `null`
/// are different:
///
/// - field absent  -> `None`          (leave the column alone)
/// - `"f": null`   -> `Some(None)`    (set the column to NULL)
/// - `"f": value`  -> `Some(Some(v))` (set the column)
///
/// Plain serde collapses the first two into `None`, which makes it impossible for
/// a PATCH to clear a nullable field. Use together with `#[serde(default)]`.
pub fn double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Deserialize an `Option<i32>` that the wire may carry as a number or as a boolean.
///
/// SQLite has no boolean type, so columns such as `organizations.worked` are stored and
/// returned as `0`/`1`. The web UI's checkbox (and the SDK's `worked?: boolean`) sends a JSON
/// boolean, which the TypeScript server stored without complaint. Accept both; `true` is `1`.
pub fn int_or_bool<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntOrBool {
        Bool(bool),
        Int(i32),
    }
    Ok(
        Option::<IntOrBool>::deserialize(deserializer)?.map(|v| match v {
            IntOrBool::Bool(b) => i32::from(b),
            IntOrBool::Int(n) => n,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Patch {
        #[serde(default, deserialize_with = "double_option")]
        note: Option<Option<String>>,
    }

    #[derive(Debug, Deserialize)]
    struct Flag {
        #[serde(default, deserialize_with = "int_or_bool")]
        worked: Option<i32>,
    }

    #[test]
    fn int_or_bool_takes_numbers_and_booleans() {
        let parse = |j: &str| serde_json::from_str::<Flag>(j).unwrap().worked;
        assert_eq!(parse("{}"), None);
        assert_eq!(parse(r#"{"worked":null}"#), None);
        assert_eq!(parse(r#"{"worked":true}"#), Some(1));
        assert_eq!(parse(r#"{"worked":false}"#), Some(0));
        assert_eq!(parse(r#"{"worked":1}"#), Some(1));
        assert_eq!(parse(r#"{"worked":0}"#), Some(0));
        assert!(serde_json::from_str::<Flag>(r#"{"worked":"yes"}"#).is_err());
    }

    #[test]
    fn absent_null_and_value_are_distinct() {
        let absent: Patch = serde_json::from_str("{}").unwrap();
        let null: Patch = serde_json::from_str(r#"{"note":null}"#).unwrap();
        let value: Patch = serde_json::from_str(r#"{"note":"x"}"#).unwrap();
        assert_eq!(absent.note, None);
        assert_eq!(null.note, Some(None));
        assert_eq!(value.note, Some(Some("x".to_string())));
    }
}
