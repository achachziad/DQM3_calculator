//! Shared helpers to read MetalKid's JSON tables.

use crate::CoreError;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::hash::Hash;

/// Parses a JSON array of rows. MetalKid's files start with a UTF-8 BOM,
/// which `serde_json` rejects, so it is stripped first.
///
/// Generic over `T`: works for any row type that serde can build
/// (`DeserializeOwned` = "can be deserialized without borrowing the input").
pub(crate) fn parse_table<T: DeserializeOwned>(
    file: &'static str,
    json: &str,
) -> Result<Vec<T>, CoreError> {
    serde_json::from_str(json.trim_start_matches('\u{feff}'))
        .map_err(|source| CoreError::Parse { file, source })
}

/// Groups items by a key, keeping their original order inside each group.
///
/// `items` is taken by value (`Vec<T>`, moved in): each item is moved into
/// its group, nothing is copied. `key` is any function or closure from
/// `&T` to the key (`impl Fn(&T) -> K`).
pub(crate) fn group_by<K, T>(items: Vec<T>, key: impl Fn(&T) -> K) -> HashMap<K, Vec<T>>
where
    K: Eq + Hash,
{
    let mut groups: HashMap<K, Vec<T>> = HashMap::new();
    for item in items {
        groups.entry(key(&item)).or_default().push(item);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_by_keeps_order_inside_groups() {
        let groups = group_by(vec![(1, "a"), (2, "b"), (1, "c")], |&(k, _)| k);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[&1], vec![(1, "a"), (1, "c")]);
        assert_eq!(groups[&2], vec![(2, "b")]);
    }

    #[test]
    fn group_by_of_nothing_is_empty() {
        let groups = group_by(Vec::<(u32, u32)>::new(), |&(k, _)| k);
        assert!(groups.is_empty());
    }
}
