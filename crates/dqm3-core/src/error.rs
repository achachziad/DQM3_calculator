//! Typed errors of the core library.

use std::fmt;

/// Everything that can go wrong in the core.
///
/// An `enum`: one variant per kind of failure, each carrying its own data.
/// Callers (GUI, MCP) can `match` on it to react precisely.
#[derive(Debug)]
pub enum CoreError {
    /// A data file is not valid JSON, or does not have the expected shape.
    Parse {
        /// Which data file, e.g. "Monster.json".
        file: &'static str,
        /// The underlying error from `serde_json`.
        source: serde_json::Error,
    },
    /// Two different entries share a key that must be unique (an id, a
    /// number or a name), so a lookup would be ambiguous.
    DuplicateKey {
        /// What kind of key, e.g. "monster name".
        kind: &'static str,
        key: String,
    },
    /// A row is valid JSON but makes no sense (e.g. a recipe with both
    /// parents and grandparents).
    InvalidRow {
        file: &'static str,
        /// The row's own id in that file.
        row_id: u32,
        reason: &'static str,
    },
}

/// `Display` is the trait behind `{}` in `format!`/`println!`:
/// the human-readable message.
impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Parse { file, source } => {
                write!(f, "invalid data file {file}: {source}")
            }
            CoreError::DuplicateKey { kind, key } => {
                write!(f, "duplicate {kind}: {key}")
            }
            CoreError::InvalidRow {
                file,
                row_id,
                reason,
            } => write!(f, "invalid row {row_id} in {file}: {reason}"),
        }
    }
}

/// `std::error::Error` makes `CoreError` a "real" error: it works with `?`
/// in callers returning `Box<dyn Error>`, and exposes its cause.
impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreError::Parse { source, .. } => Some(source),
            CoreError::DuplicateKey { .. } | CoreError::InvalidRow { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn parse_error() -> CoreError {
        let source = serde_json::from_str::<u32>("not json").unwrap_err();
        CoreError::Parse {
            file: "Monster.json",
            source,
        }
    }

    #[test]
    fn parse_error_message_names_the_file() {
        let message = parse_error().to_string();
        assert!(message.contains("Monster.json"), "got: {message}");
    }

    #[test]
    fn parse_error_exposes_its_cause() {
        assert!(parse_error().source().is_some());
    }
}
