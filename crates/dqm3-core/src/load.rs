//! Shared helpers to read MetalKid's JSON tables.

use crate::CoreError;
use serde::de::DeserializeOwned;

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
