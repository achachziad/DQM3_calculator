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
}

/// `Display` is the trait behind `{}` in `format!`/`println!`:
/// the human-readable message.
impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Parse { file, source } => {
                write!(f, "invalid data file {file}: {source}")
            }
        }
    }
}

/// `std::error::Error` makes `CoreError` a "real" error: it works with `?`
/// in callers returning `Box<dyn Error>`, and exposes its cause.
impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreError::Parse { source, .. } => Some(source),
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
