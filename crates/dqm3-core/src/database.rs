//! The loaded game data: the entry point of the library.

use crate::CoreError;
use crate::monster::{Monster, parse_monsters};

/// MetalKid's tables, embedded in the executable at compile time by
/// `include_str!` (path relative to this source file). No file to ship or
/// find at runtime; if `data/` is empty (submodule not initialised), the
/// build fails with a clear "file not found".
mod embedded {
    pub const MONSTER: &str = include_str!("../../../data/data/json/raw/Monster.json");
}

/// All the game data, loaded once.
#[derive(Debug)]
pub struct Database {
    monsters: Vec<Monster>,
}

impl Database {
    /// Loads the embedded MetalKid data.
    pub fn load() -> Result<Self, CoreError> {
        Ok(Self {
            monsters: parse_monsters(embedded::MONSTER)?,
        })
    }

    /// All monsters, including the "family + rank" rules.
    ///
    /// Returns a slice `&[Monster]`: a read-only view on the database's
    /// own vector, borrowed for as long as the `Database` lives.
    pub fn monsters(&self) -> &[Monster] {
        &self.monsters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test on the real embedded data (figures from docs/DATA.md).
    #[test]
    fn embedded_data_loads_with_expected_counts() {
        let db = Database::load().expect("embedded data is valid");
        let monsters = db.monsters();
        assert_eq!(monsters.len(), 606);
        assert_eq!(monsters.iter().filter(|m| m.is_family_rule()).count(), 80);
        assert_eq!(monsters.iter().filter(|m| m.name_fr.is_some()).count(), 523);
    }
}
