//! Lookup tables over the monster list, built once at load time.
//!
//! They store *positions* (`usize`) in the database's `Vec<Monster>`, not
//! `&Monster` references: a struct cannot hold references into its own
//! fields (moving the struct would leave them dangling), so the database
//! keeps the `Vec` and these indexes side by side.

use crate::CoreError;
use crate::ids::MonsterId;
use crate::monster::Monster;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::Display;
use std::hash::Hash;

/// Exact-name key: trimmed, lowercase ("  GLUANT " -> "gluant").
/// `to_lowercase` is Unicode-aware: "É" -> "é". Accents are kept (exact
/// lookup); accent-insensitive search comes with the search feature.
pub(crate) fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Inserts `key -> position`, unless another position already holds the
/// key. The same position may insert a key twice (a monster whose French
/// and English names are equal).
///
/// Generic: `K` is any key type usable in a `HashMap` (`Eq + Hash`) that
/// can be shown in the error message (`Display`).
fn insert_unique<K>(
    map: &mut HashMap<K, usize>,
    key: K,
    position: usize,
    kind: &'static str,
) -> Result<(), CoreError>
where
    K: Eq + Hash + Display,
{
    // `entry`: one lookup, then either fill the free slot or inspect it.
    match map.entry(key) {
        Entry::Vacant(slot) => {
            slot.insert(position);
            Ok(())
        }
        Entry::Occupied(slot) if *slot.get() == position => Ok(()),
        Entry::Occupied(slot) => Err(CoreError::DuplicateKey {
            kind,
            key: slot.key().to_string(),
        }),
    }
}

#[derive(Debug, Default)]
pub(crate) struct MonsterIndex {
    by_id: HashMap<MonsterId, usize>,
    by_number: HashMap<u32, usize>,
    by_name: HashMap<String, usize>,
}

impl MonsterIndex {
    /// Builds the indexes, refusing any key shared by two different monsters.
    pub(crate) fn build(monsters: &[Monster]) -> Result<Self, CoreError> {
        let mut index = Self::default();
        // `enumerate` pairs each monster with its position in the slice.
        for (position, monster) in monsters.iter().enumerate() {
            insert_unique(&mut index.by_id, monster.id, position, "monster id")?;
            if let Some(number) = monster.number {
                insert_unique(&mut index.by_number, number, position, "monster number")?;
            }
            let names = std::iter::once(&monster.name_en).chain(&monster.name_fr);
            for name in names {
                insert_unique(&mut index.by_name, name_key(name), position, "monster name")?;
            }
        }
        Ok(index)
    }

    pub(crate) fn by_id(&self, id: MonsterId) -> Option<usize> {
        self.by_id.get(&id).copied()
    }

    pub(crate) fn by_number(&self, number: u32) -> Option<usize> {
        self.by_number.get(&number).copied()
    }

    /// French or English name, case-insensitive.
    pub(crate) fn by_name(&self, name: &str) -> Option<usize> {
        self.by_name.get(&name_key(name)).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{FamilyId, RankId};

    fn monster(id: u32, number: Option<u32>, en: &str, fr: Option<&str>) -> Monster {
        Monster {
            id: MonsterId(id),
            number,
            identifier: format!("m{id}"),
            name_en: en.into(),
            name_fr: fr.map(Into::into),
            family: FamilyId(1),
            rank: RankId(1),
            max_stats: None,
        }
    }

    /// Positions: 0 = Slime, 1 = Airbagon, 2 = Barracuda, 3 = family rule.
    fn sample() -> Vec<Monster> {
        vec![
            monster(10, Some(1), "Slime", Some("Gluant")),
            monster(20, Some(68), "Airbagon", Some("Méthanosaure")),
            monster(30, Some(164), "Barracuda", Some("Barracuda")), // FR = EN
            monster(87, None, "Slime Family (G)", None),
        ]
    }

    fn index() -> MonsterIndex {
        MonsterIndex::build(&sample()).expect("no duplicates")
    }

    #[test]
    fn finds_by_id_and_number() {
        assert_eq!(index().by_id(MonsterId(20)), Some(1));
        assert_eq!(index().by_number(1), Some(0));
    }

    #[test]
    fn finds_by_english_name_ignoring_case() {
        assert_eq!(index().by_name("slime"), Some(0));
        assert_eq!(index().by_name("  SLIME "), Some(0));
    }

    #[test]
    fn finds_by_french_name() {
        assert_eq!(index().by_name("Gluant"), Some(0));
    }

    #[test]
    fn finds_accented_french_name_ignoring_case() {
        assert_eq!(index().by_name("Méthanosaure"), Some(1));
        assert_eq!(index().by_name("MÉTHANOSAURE"), Some(1));
    }

    #[test]
    fn same_french_and_english_name_is_not_a_duplicate() {
        assert_eq!(index().by_name("barracuda"), Some(2));
    }

    #[test]
    fn family_rule_is_found_by_name_but_has_no_number() {
        assert_eq!(index().by_name("Slime Family (G)"), Some(3));
        assert_eq!(index().by_number(0), None);
    }

    #[test]
    fn unknown_name_number_or_id_gives_none() {
        assert_eq!(index().by_name("Introuvable"), None);
        assert_eq!(index().by_number(999), None);
        assert_eq!(index().by_id(MonsterId(999)), None);
    }

    #[test]
    fn name_shared_by_two_monsters_is_refused() {
        let mut monsters = sample();
        monsters.push(monster(40, Some(200), "Other", Some("gluant")));
        let error = MonsterIndex::build(&monsters).unwrap_err();
        assert!(matches!(
            error,
            CoreError::DuplicateKey { kind: "monster name", ref key } if key == "gluant"
        ));
    }

    #[test]
    fn duplicate_number_is_refused() {
        let mut monsters = sample();
        monsters.push(monster(40, Some(1), "Other", None));
        assert!(matches!(
            MonsterIndex::build(&monsters),
            Err(CoreError::DuplicateKey {
                kind: "monster number",
                ..
            })
        ));
    }
}
