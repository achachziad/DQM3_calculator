//! Monsters (`Monster.json`).

use crate::CoreError;
use crate::ids::{FamilyId, MonsterId, RankId};
use crate::load::parse_table;
use serde::Deserialize;

/// The six stats of a monster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub hp: u32,
    pub mp: u32,
    pub att: u32,
    pub def: u32,
    pub agi: u32,
    pub wis: u32,
}

/// A monster, as the rest of the code sees it.
///
/// Fields are `pub` for reading: the data is owned by the database and
/// handed out as `&Monster` (shared borrow), so callers cannot modify it.
#[derive(Debug, Clone, PartialEq)]
pub struct Monster {
    pub id: MonsterId,
    /// In-game number (1–526). `None` for the 80 "family + rank" rules
    /// such as "Slime Family (G)", which are not real monsters.
    pub number: Option<u32>,
    /// Stable key, e.g. "slime" or "goonache-goodie". The GUI maps it to
    /// the portrait `data/Sprites/<identifier>-thumb.png`.
    pub identifier: String,
    pub name_en: String,
    /// `None` for the 3 expired event monsters.
    pub name_fr: Option<String>,
    pub family: FamilyId,
    pub rank: RankId,
    /// `None` for family rules.
    pub max_stats: Option<Stats>,
}

impl Monster {
    /// True for a synthesis rule ("any monster of family F and rank R").
    pub fn is_family_rule(&self) -> bool {
        self.number.is_none()
    }
}

/// One row of `Monster.json`, exactly as in the file. Kept private: the
/// file format stays an internal detail, converted into `Monster`.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawMonster {
    monster_id: MonsterId,
    family_id: FamilyId,
    rank_id: RankId,
    number: Option<u32>,
    identifier: String,
    name: String,
    french_name: Option<String>,
    #[serde(rename = "MaxHP")]
    max_hp: Option<u32>,
    #[serde(rename = "MaxMP")]
    max_mp: Option<u32>,
    max_att: Option<u32>,
    max_def: Option<u32>,
    max_agi: Option<u32>,
    max_wis: Option<u32>,
}

impl RawMonster {
    /// All six max stats, or `None` if any is missing. Inside a function
    /// returning `Option`, `?` on a `None` returns `None` right away.
    fn max_stats(&self) -> Option<Stats> {
        Some(Stats {
            hp: self.max_hp?,
            mp: self.max_mp?,
            att: self.max_att?,
            def: self.max_def?,
            agi: self.max_agi?,
            wis: self.max_wis?,
        })
    }
}

/// `From` is the standard conversion trait: implementing it also gives
/// `raw.into()` for free. The raw row is taken by value (moved), so its
/// `String`s are moved into the `Monster` without copying.
impl From<RawMonster> for Monster {
    fn from(raw: RawMonster) -> Self {
        let max_stats = raw.max_stats();
        Monster {
            id: raw.monster_id,
            number: raw.number,
            identifier: raw.identifier,
            name_en: raw.name,
            name_fr: raw.french_name,
            family: raw.family_id,
            rank: raw.rank_id,
            max_stats,
        }
    }
}

/// Parses the content of `Monster.json`.
pub(crate) fn parse_monsters(json: &str) -> Result<Vec<Monster>, CoreError> {
    let rows: Vec<RawMonster> = parse_table("Monster.json", json)?;
    Ok(rows.into_iter().map(Monster::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLIME: &str = r#"{
        "MonsterId": 1, "FamilyId": 1, "RankId": 1, "Number": 1,
        "Name": "Slime", "JapaneseName": "スライム", "FrenchName": "Gluant",
        "Identifier": "slime",
        "HP": 10, "MP": 5, "Att": 8, "Def": 6, "Agi": 9, "Wis": 7,
        "MaxHP": 400, "MaxMP": 250, "MaxAtt": 300, "MaxDef": 280, "MaxAgi": 320, "MaxWis": 290,
        "Trivia": null, "EggTypeId": null, "Notes": null
    }"#;

    const FAMILY_RULE: &str = r#"{
        "MonsterId": 87, "FamilyId": 1, "RankId": 1, "Number": null,
        "Name": "Slime Family (G)", "JapaneseName": null, "FrenchName": null,
        "Identifier": "slime-g",
        "HP": null, "MP": null, "Att": null, "Def": null, "Agi": null, "Wis": null,
        "MaxHP": null, "MaxMP": null, "MaxAtt": null, "MaxDef": null, "MaxAgi": null, "MaxWis": null,
        "Trivia": null, "EggTypeId": null, "Notes": null
    }"#;

    fn parse_one(row: &str) -> Monster {
        let mut monsters = parse_monsters(&format!("[{row}]")).expect("valid fixture");
        assert_eq!(monsters.len(), 1);
        monsters.remove(0)
    }

    #[test]
    fn real_monster_has_french_and_english_names_and_stats() {
        let slime = parse_one(SLIME);
        assert_eq!(slime.id, MonsterId(1));
        assert_eq!(slime.number, Some(1));
        assert_eq!(slime.name_en, "Slime");
        assert_eq!(slime.identifier, "slime");
        assert_eq!(slime.name_fr.as_deref(), Some("Gluant"));
        assert_eq!(slime.family, FamilyId(1));
        assert_eq!(slime.rank, RankId(1));
        assert_eq!(
            slime.max_stats,
            Some(Stats {
                hp: 400,
                mp: 250,
                att: 300,
                def: 280,
                agi: 320,
                wis: 290
            })
        );
        assert!(!slime.is_family_rule());
    }

    #[test]
    fn family_rule_has_no_number_and_no_stats() {
        let rule = parse_one(FAMILY_RULE);
        assert!(rule.is_family_rule());
        assert_eq!(rule.max_stats, None);
        assert_eq!(rule.name_fr, None);
    }

    #[test]
    fn utf8_bom_is_accepted() {
        let json = format!("\u{feff}[{SLIME}]");
        assert!(parse_monsters(&json).is_ok());
    }

    #[test]
    fn invalid_file_gives_a_parse_error_naming_the_file() {
        let error = parse_monsters("[{\"MonsterId\": \"oops\"}]").unwrap_err();
        assert!(matches!(
            error,
            CoreError::Parse {
                file: "Monster.json",
                ..
            }
        ));
    }
}
