//! Talents, their tiers and evolutions, skills, and which monster has which
//! talent. Files: `Talent.json`, `TalentSkill.json`, `TalentTrait.json`,
//! `Skill.json`, `MonsterTalent.json`, `TalentSynthesis.json`,
//! `TalentSynthesisTalent.json`.

use crate::CoreError;
use crate::ids::{MonsterId, SkillId, TalentId, TraitId};
use crate::load::{group_by, parse_table};
use serde::Deserialize;

/// What a talent tier unlocks: a skill (spell/ability) or a trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reward {
    Skill(SkillId),
    Trait(TraitId),
}

/// One tier: reached by spending `points` in the talent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TalentTier {
    pub points: u32,
    pub reward: Reward,
}

/// A talent ("skill tree" in game).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Talent {
    pub id: TalentId,
    /// English name. French names come from `data-fr/` (not embedded
    /// while the wiki license is unknown).
    pub name_en: String,
    /// Only obtainable from eggs (3 talents).
    pub egg_only: bool,
    /// Sorted by points.
    pub tiers: Vec<TalentTier>,
}

/// A skill: spell or ability unlocked by a talent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub id: SkillId,
    pub name_en: String,
    pub description_en: String,
}

/// A talent carried by a monster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonsterTalent {
    pub talent: TalentId,
    /// The monster's fixed talent (`IsPrimary`); otherwise a random one.
    /// Source anomalies: Rubble Slime has 2 fixed, Fromage Fray and Ursa
    /// Mega none. Kept as is.
    pub fixed: bool,
}

/// One way to obtain `result` by maxing out talents: from 1 talent (an
/// upgrade, Chilly → Frosty Receptionist) or 2 (a combination).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentEvolution {
    pub result: TalentId,
    pub ingredients: Vec<TalentId>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawTalent {
    talent_id: TalentId,
    name: String,
    is_egg_only: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawTalentSkill {
    talent_id: TalentId,
    skill_id: SkillId,
    points: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawTalentTrait {
    talent_id: TalentId,
    trait_id: TraitId,
    points: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawSkill {
    skill_id: SkillId,
    name: String,
    description: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawMonsterTalent {
    monster_id: MonsterId,
    talent_id: TalentId,
    is_primary: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawTalentSynthesis {
    talent_synthesis_id: u32,
    talent_result_id: TalentId,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawTalentSynthesisTalent {
    talent_synthesis_id: u32,
    talent_id: TalentId,
}

/// Builds talents from `Talent.json`, `TalentSkill.json` and
/// `TalentTrait.json` (tiers merged and sorted by points).
pub(crate) fn parse_talents(
    talents: &str,
    talent_skills: &str,
    talent_traits: &str,
) -> Result<Vec<Talent>, CoreError> {
    let skill_tiers: Vec<RawTalentSkill> = parse_table("TalentSkill.json", talent_skills)?;
    let trait_tiers: Vec<RawTalentTrait> = parse_table("TalentTrait.json", talent_traits)?;

    // All tiers as (talent, tier), skills then traits, grouped by talent.
    let tiers = skill_tiers
        .into_iter()
        .map(|r| {
            (
                r.talent_id,
                TalentTier {
                    points: r.points,
                    reward: Reward::Skill(r.skill_id),
                },
            )
        })
        .chain(trait_tiers.into_iter().map(|r| {
            (
                r.talent_id,
                TalentTier {
                    points: r.points,
                    reward: Reward::Trait(r.trait_id),
                },
            )
        }))
        .collect();
    let mut tiers_by_talent = group_by(tiers, |(talent, _)| *talent);

    let rows: Vec<RawTalent> = parse_table("Talent.json", talents)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            // `remove` takes the group out of the map (moved, not copied).
            let mut tiers: Vec<TalentTier> = tiers_by_talent
                .remove(&r.talent_id)
                .unwrap_or_default()
                .into_iter()
                .map(|(_, tier)| tier)
                .collect();
            tiers.sort_by_key(|t| t.points);
            Talent {
                id: r.talent_id,
                name_en: r.name,
                egg_only: r.is_egg_only,
                tiers,
            }
        })
        .collect())
}

/// Parses `Skill.json`.
pub(crate) fn parse_skills(json: &str) -> Result<Vec<Skill>, CoreError> {
    let rows: Vec<RawSkill> = parse_table("Skill.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| Skill {
            id: r.skill_id,
            name_en: r.name,
            description_en: r.description,
        })
        .collect())
}

/// Parses `MonsterTalent.json` into `(monster, talent)` pairs.
pub(crate) fn parse_monster_talents(
    json: &str,
) -> Result<Vec<(MonsterId, MonsterTalent)>, CoreError> {
    let rows: Vec<RawMonsterTalent> = parse_table("MonsterTalent.json", json)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let talent = MonsterTalent {
                talent: r.talent_id,
                fixed: r.is_primary,
            };
            (r.monster_id, talent)
        })
        .collect())
}

/// Builds evolutions from `TalentSynthesis.json` and
/// `TalentSynthesisTalent.json`. Each must have 1 or 2 ingredients.
pub(crate) fn parse_talent_evolutions(
    syntheses: &str,
    ingredients: &str,
) -> Result<Vec<TalentEvolution>, CoreError> {
    let links: Vec<RawTalentSynthesisTalent> =
        parse_table("TalentSynthesisTalent.json", ingredients)?;
    let mut by_synthesis = group_by(links, |l| l.talent_synthesis_id);

    let rows: Vec<RawTalentSynthesis> = parse_table("TalentSynthesis.json", syntheses)?;
    rows.into_iter()
        .map(|r| {
            let ingredients: Vec<TalentId> = by_synthesis
                .remove(&r.talent_synthesis_id)
                .unwrap_or_default()
                .into_iter()
                .map(|l| l.talent_id)
                .collect();
            if !(1..=2).contains(&ingredients.len()) {
                return Err(CoreError::InvalidRow {
                    file: "TalentSynthesis.json",
                    row_id: r.talent_synthesis_id,
                    reason: "expected 1 or 2 ingredient talents",
                });
            }
            Ok(TalentEvolution {
                result: r.talent_result_id,
                ingredients,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TALENTS: &str = r#"[
        {"TalentId": 1, "Name": "Assassin", "Identifier": "a", "JapaneseName": "x", "IsEggOnly": false},
        {"TalentId": 2, "Name": "Hard Warder", "Identifier": "b", "JapaneseName": "y", "IsEggOnly": true}
    ]"#;
    const TALENT_SKILLS: &str = r#"[
        {"TalentSkillId": 1, "TalentId": 1, "SkillId": 50, "Points": 10},
        {"TalentSkillId": 2, "TalentId": 1, "SkillId": 51, "Points": 3}
    ]"#;
    const TALENT_TRAITS: &str = r#"[
        {"TalentTraitId": 1, "TalentId": 1, "TraitId": 70, "Points": 5}
    ]"#;

    #[test]
    fn tiers_merge_skills_and_traits_sorted_by_points() {
        let talents = parse_talents(TALENTS, TALENT_SKILLS, TALENT_TRAITS).expect("valid");
        assert_eq!(
            talents[0].tiers,
            vec![
                TalentTier {
                    points: 3,
                    reward: Reward::Skill(SkillId(51))
                },
                TalentTier {
                    points: 5,
                    reward: Reward::Trait(TraitId(70))
                },
                TalentTier {
                    points: 10,
                    reward: Reward::Skill(SkillId(50))
                },
            ]
        );
        assert!(!talents[0].egg_only);
    }

    #[test]
    fn talent_without_tiers_and_egg_only_flag() {
        let talents = parse_talents(TALENTS, TALENT_SKILLS, TALENT_TRAITS).expect("valid");
        assert_eq!(talents[1].name_en, "Hard Warder");
        assert!(talents[1].egg_only);
        assert!(talents[1].tiers.is_empty());
    }

    #[test]
    fn skills_are_parsed() {
        let json = r#"[{"SkillId": 44, "SkillTypeId": 3, "SkillTargetTypeId": 3,
            "Name": "Accelerate", "Identifier": "accelerate", "JapaneseName": "x",
            "Description": "Greatly increases agility.", "MPCost": 3}]"#;
        assert_eq!(
            parse_skills(json).expect("valid"),
            vec![Skill {
                id: SkillId(44),
                name_en: "Accelerate".into(),
                description_en: "Greatly increases agility.".into(),
            }]
        );
    }

    #[test]
    fn monster_talents_keep_the_fixed_flag() {
        let json = r#"[
            {"MonsterTalentId": 1, "MonsterId": 101, "TalentId": 23, "IsPrimary": true},
            {"MonsterTalentId": 2, "MonsterId": 101, "TalentId": 24, "IsPrimary": false}
        ]"#;
        assert_eq!(
            parse_monster_talents(json).expect("valid"),
            vec![
                (
                    MonsterId(101),
                    MonsterTalent {
                        talent: TalentId(23),
                        fixed: true
                    }
                ),
                (
                    MonsterId(101),
                    MonsterTalent {
                        talent: TalentId(24),
                        fixed: false
                    }
                ),
            ]
        );
    }

    const SYNTHESES: &str = r#"[
        {"TalentSynthesisId": 1, "TalentResultId": 91},
        {"TalentSynthesisId": 2, "TalentResultId": 92}
    ]"#;

    #[test]
    fn evolutions_have_one_or_two_ingredients() {
        let ingredients = r#"[
            {"TalentSynthesisTalentId": 1, "TalentSynthesisId": 1, "TalentId": 33},
            {"TalentSynthesisTalentId": 2, "TalentSynthesisId": 1, "TalentId": 34},
            {"TalentSynthesisTalentId": 3, "TalentSynthesisId": 2, "TalentId": 91}
        ]"#;
        let evolutions = parse_talent_evolutions(SYNTHESES, ingredients).expect("valid");
        assert_eq!(
            evolutions,
            vec![
                TalentEvolution {
                    result: TalentId(91),
                    ingredients: vec![TalentId(33), TalentId(34)],
                },
                TalentEvolution {
                    result: TalentId(92),
                    ingredients: vec![TalentId(91)],
                },
            ]
        );
    }

    #[test]
    fn evolution_without_ingredient_is_refused() {
        let only_first = r#"[
            {"TalentSynthesisTalentId": 1, "TalentSynthesisId": 1, "TalentId": 33}
        ]"#;
        assert!(matches!(
            parse_talent_evolutions(SYNTHESES, only_first),
            Err(CoreError::InvalidRow {
                file: "TalentSynthesis.json",
                row_id: 2,
                ..
            })
        ));
    }
}
