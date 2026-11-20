//! Traits (`Trait.json`) and which monster has which trait
//! (`MonsterTrait.json`).
//!
//! Naming note: `trait` is a Rust keyword, so fields and functions say
//! `trait_id` / `trait_by_id`; the type itself can be `Trait`.

use crate::CoreError;
use crate::ids::{MonsterId, SizeId, TraitId};
use crate::load::parse_table;
use serde::Deserialize;

/// A trait (passive effect).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trait {
    pub id: TraitId,
    pub name_en: String,
    pub description_en: String,
}

/// A trait of a monster, for one size, unlocked at a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonsterTrait {
    pub size: SizeId,
    pub trait_id: TraitId,
    /// 1, 20, 40 or 60 in the data.
    pub unlock_level: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawTrait {
    trait_id: TraitId,
    name: String,
    description: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawMonsterTrait {
    monster_id: MonsterId,
    size_id: SizeId,
    trait_id: TraitId,
    unlock_level: u32,
}

/// Parses `Trait.json`.
pub(crate) fn parse_traits(json: &str) -> Result<Vec<Trait>, CoreError> {
    let rows: Vec<RawTrait> = parse_table("Trait.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| Trait {
            id: r.trait_id,
            name_en: r.name,
            description_en: r.description,
        })
        .collect())
}

/// Parses `MonsterTrait.json` into `(monster, trait)` pairs.
pub(crate) fn parse_monster_traits(
    json: &str,
) -> Result<Vec<(MonsterId, MonsterTrait)>, CoreError> {
    let rows: Vec<RawMonsterTrait> = parse_table("MonsterTrait.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let monster_trait = MonsterTrait {
                size: r.size_id,
                trait_id: r.trait_id,
                unlock_level: r.unlock_level,
            };
            (r.monster_id, monster_trait)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traits_are_parsed() {
        let json = r#"[{"TraitId": 69, "Name": "Absorbent Touch", "Identifier": null,
            "JapaneseName": null, "Description": "Absorbs MP.", "HPBonus": null}]"#;
        assert_eq!(
            parse_traits(json).expect("valid"),
            vec![Trait {
                id: TraitId(69),
                name_en: "Absorbent Touch".into(),
                description_en: "Absorbs MP.".into(),
            }]
        );
    }

    #[test]
    fn monster_traits_keep_size_and_unlock_level() {
        let json = r#"[{"MonsterTraitId": 1, "MonsterId": 101, "SizeId": 2, "TraitId": 28, "UnlockLevel": 40}]"#;
        assert_eq!(
            parse_monster_traits(json).expect("valid"),
            vec![(
                MonsterId(101),
                MonsterTrait {
                    size: SizeId(2),
                    trait_id: TraitId(28),
                    unlock_level: 40
                }
            )]
        );
    }
}
