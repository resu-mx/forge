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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Patch {
        #[serde(default, deserialize_with = "double_option")]
        note: Option<Option<String>>,
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
