//! Families (`Family.json`) and ranks (`Rank.json`).

use crate::CoreError;
use crate::ids::{FamilyId, RankId};
use crate::load::parse_table;
use serde::Deserialize;

/// A monster family (Slime, Dragon, …, and "???" for bosses).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family {
    pub id: FamilyId,
    /// Stable key: "slime", "dragon", …, "boss" (for "???"). The GUI maps
    /// it to the family symbol (`data/Misc/<identifier>.png`).
    pub identifier: String,
    pub name_en: String,
    /// From `FAMILY_NAMES_FR` (MetalKid has no French family names).
    /// `None` only if the data gains a family unknown to that table.
    pub name_fr: Option<String>,
}

/// French family names as shown in game (checked by the user), by
/// `identifier`.
const FAMILY_NAMES_FR: &[(&str, &str)] = &[
    ("slime", "Gluant"),
    ("dragon", "Dragon"),
    ("nature", "Nature"),
    ("beast", "Bête"),
    ("material", "Matériel"),
    ("demon", "Démon"),
    ("undead", "Mort-vivant"),
    ("boss", "???"),
];

/// A rank: G, F, E, D, C, B, A, S, X, or "Any" (wildcard used by the
/// "family + rank" synthesis rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rank {
    pub id: RankId,
    pub name: String,
}

impl Rank {
    /// True for "Any", the wildcard rank of some synthesis rules.
    pub fn is_wildcard(&self) -> bool {
        self.name == "Any"
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawFamily {
    family_id: FamilyId,
    identifier: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawRank {
    rank_id: RankId,
    name: String,
}

/// Parses the content of `Family.json`.
pub(crate) fn parse_families(json: &str) -> Result<Vec<Family>, CoreError> {
    let rows: Vec<RawFamily> = parse_table("Family.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let name_fr = FAMILY_NAMES_FR
                .iter()
                .find(|(identifier, _)| *identifier == r.identifier)
                .map(|(_, fr)| fr.to_string());
            Family {
                id: r.family_id,
                identifier: r.identifier,
                name_en: r.name,
                name_fr,
            }
        })
        .collect())
}

/// Parses the content of `Rank.json`.
pub(crate) fn parse_ranks(json: &str) -> Result<Vec<Rank>, CoreError> {
    let rows: Vec<RawRank> = parse_table("Rank.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| Rank {
            id: r.rank_id,
            name: r.name,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_are_parsed() {
        let json = r#"[
            {"FamilyId": 1, "Name": "Slime", "Identifier": "slime", "JapaneseName": "スライム系",
             "BonusHP": null, "BonusMP": null, "BonusAtt": null, "BonusDef": null,
             "BonusAgi": null, "BonusWis": null, "Order": 1}
        ]"#;
        assert_eq!(
            parse_families(json).expect("valid fixture"),
            vec![Family {
                id: FamilyId(1),
                identifier: "slime".into(),
                name_en: "Slime".into(),
                name_fr: Some("Gluant".into()),
            }]
        );
    }

    #[test]
    fn ranks_are_parsed_and_any_is_the_wildcard() {
        let json = r#"[
            {"RankId": 1, "Name": "G", "Identifier": "g"},
            {"RankId": 10, "Name": "Any", "Identifier": "any"}
        ]"#;
        let ranks = parse_ranks(json).expect("valid fixture");
        assert_eq!(
            ranks[0],
            Rank {
                id: RankId(1),
                name: "G".into()
            }
        );
        assert!(!ranks[0].is_wildcard());
        assert!(ranks[1].is_wildcard());
    }

    #[test]
    fn invalid_files_name_the_file() {
        assert!(matches!(
            parse_families("{}"),
            Err(CoreError::Parse {
                file: "Family.json",
                ..
            })
        ));
        assert!(matches!(
            parse_ranks("{}"),
            Err(CoreError::Parse {
                file: "Rank.json",
                ..
            })
        ));
    }
}
