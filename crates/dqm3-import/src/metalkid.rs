//! Reads MetalKid's raw talent tables (the reference data).

use crate::compare::MkTalent;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TalentRow {
    talent_id: u32,
    name: String,
    japanese_name: String,
}

/// Shape shared by `TalentSkill.json` and `TalentTrait.json`.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TierRow {
    talent_id: u32,
    points: u32,
}

/// Builds MetalKid talents from the JSON text of `Talent.json`,
/// `TalentSkill.json` and `TalentTrait.json`.
pub fn parse(talents: &str, skills: &str, traits: &str) -> serde_json::Result<Vec<MkTalent>> {
    let talents: Vec<TalentRow> = from_json(talents)?;
    let mut tiers: Vec<TierRow> = from_json(skills)?;
    tiers.extend(from_json::<TierRow>(traits)?);

    Ok(talents
        .into_iter()
        .map(|t| {
            let mut thresholds: Vec<u32> = tiers
                .iter()
                .filter(|r| r.talent_id == t.talent_id)
                .map(|r| r.points)
                .collect();
            thresholds.sort_unstable();
            MkTalent {
                en: t.name,
                ja: t.japanese_name,
                thresholds,
            }
        })
        .collect())
}

/// MetalKid's files start with a UTF-8 BOM, which `serde_json` rejects.
fn from_json<T: serde::de::DeserializeOwned>(text: &str) -> serde_json::Result<Vec<T>> {
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TALENTS: &str = r#"[
        {"TalentId": 1, "Name": "Agility Booster I", "Identifier": "x", "JapaneseName": "すばやさアップ1", "IsEggOnly": false},
        {"TalentId": 2, "Name": "Assassin", "Identifier": "y", "JapaneseName": "アサシン", "IsEggOnly": false}
    ]"#;
    const SKILLS: &str = r#"[
        {"TalentSkillId": 1, "TalentId": 2, "SkillId": 9, "Points": 10},
        {"TalentSkillId": 2, "TalentId": 2, "SkillId": 8, "Points": 3}
    ]"#;
    const TRAITS: &str = r#"[
        {"TalentTraitId": 1, "TalentId": 1, "TraitId": 7, "Points": 5}
    ]"#;

    #[test]
    fn thresholds_merge_skills_and_traits_sorted() {
        let talents = parse(TALENTS, SKILLS, TRAITS).expect("valid fixture");
        assert_eq!(talents.len(), 2);
        assert_eq!(talents[0].en, "Agility Booster I");
        assert_eq!(talents[0].thresholds, vec![5]);
        assert_eq!(talents[1].ja, "アサシン");
        assert_eq!(talents[1].thresholds, vec![3, 10]);
    }

    #[test]
    fn utf8_bom_is_accepted() {
        let with_bom = format!("\u{feff}{TALENTS}");
        assert!(parse(&with_bom, SKILLS, TRAITS).is_ok());
    }

    #[test]
    fn invalid_json_is_an_error() {
        assert!(parse("not json", SKILLS, TRAITS).is_err());
    }
}
