//! Synthesis recipes (`MonsterSynthesis.json`).

use crate::CoreError;
use crate::ids::MonsterId;
use crate::load::parse_table;
use serde::Deserialize;

/// What goes into a synthesis. An ingredient may be a real monster or a
/// "family + rank" rule (a placeholder `Monster`, see `is_family_rule`).
///
/// An `enum` makes invalid shapes impossible: a recipe has 2 parents *or*
/// 4 grandparents, never 3, never both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parents {
    /// The usual case: parent 1 + parent 2.
    Pair([MonsterId; 2]),
    /// Bosses (Zoma, Estark…): the 4 monsters of the previous generation,
    /// in order 1A, 1B (parents of parent 1), 2A, 2B (parents of parent 2).
    Grandparents([MonsterId; 4]),
}

/// One way to obtain `result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recipe {
    pub result: MonsterId,
    pub parents: Parents,
}

impl Recipe {
    /// All ingredients, whatever the kind of recipe. Returns a slice
    /// borrowed from the recipe's own array (no allocation).
    pub fn ingredients(&self) -> &[MonsterId] {
        match &self.parents {
            Parents::Pair(ids) => ids,
            Parents::Grandparents(ids) => ids,
        }
    }
}

/// One row of `MonsterSynthesis.json`. `AreBothRankSpecific` is ignored:
/// true on a single row, meaning unclear (see docs/DATA.md).
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawRecipe {
    monster_synthesis_id: u32,
    monster_result_id: MonsterId,
    monster_parent1_id: Option<MonsterId>,
    monster_parent2_id: Option<MonsterId>,
    #[serde(rename = "MonsterGrandParent1AId")]
    grandparent_1a: Option<MonsterId>,
    #[serde(rename = "MonsterGrandParent1BId")]
    grandparent_1b: Option<MonsterId>,
    #[serde(rename = "MonsterGrandParent2AId")]
    grandparent_2a: Option<MonsterId>,
    #[serde(rename = "MonsterGrandParent2BId")]
    grandparent_2b: Option<MonsterId>,
}

/// Parses the content of `MonsterSynthesis.json`.
pub(crate) fn parse_recipes(json: &str) -> Result<Vec<Recipe>, CoreError> {
    let rows: Vec<RawRecipe> = parse_table("MonsterSynthesis.json", json)?;
    // `collect` into `Result<Vec<_>, _>`: stops at the first error.
    rows.into_iter().map(Recipe::try_from).collect()
}

/// `TryFrom`: like `From`, but the conversion may fail.
impl TryFrom<RawRecipe> for Recipe {
    type Error = CoreError;

    fn try_from(raw: RawRecipe) -> Result<Self, CoreError> {
        let parents = match (
            raw.monster_parent1_id,
            raw.monster_parent2_id,
            raw.grandparent_1a,
            raw.grandparent_1b,
            raw.grandparent_2a,
            raw.grandparent_2b,
        ) {
            (Some(p1), Some(p2), None, None, None, None) => Parents::Pair([p1, p2]),
            (None, None, Some(a1), Some(b1), Some(a2), Some(b2)) => {
                Parents::Grandparents([a1, b1, a2, b2])
            }
            _ => {
                return Err(CoreError::InvalidRow {
                    file: "MonsterSynthesis.json",
                    row_id: raw.monster_synthesis_id,
                    reason: "expected exactly 2 parents or 4 grandparents",
                });
            }
        };
        Ok(Recipe {
            result: raw.monster_result_id,
            parents,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        id: u32,
        result: u32,
        parents: [Option<u32>; 2],
        grandparents: [Option<u32>; 4],
    ) -> String {
        let n = |v: Option<u32>| v.map_or("null".to_string(), |v| v.to_string());
        format!(
            r#"{{"MonsterSynthesisId": {id}, "MonsterResultId": {result},
                "MonsterParent1Id": {}, "MonsterGrandParent1AId": {}, "MonsterGrandParent1BId": {},
                "MonsterParent2Id": {}, "MonsterGrandParent2AId": {}, "MonsterGrandParent2BId": {},
                "AreBothRankSpecific": false}}"#,
            n(parents[0]),
            n(grandparents[0]),
            n(grandparents[1]),
            n(parents[1]),
            n(grandparents[2]),
            n(grandparents[3]),
        )
    }

    #[test]
    fn two_parent_recipe_is_a_pair() {
        let json = format!("[{}]", row(1, 102, [Some(11), Some(21)], [None; 4]));
        let recipes = parse_recipes(&json).expect("valid fixture");
        assert_eq!(
            recipes,
            vec![Recipe {
                result: MonsterId(102),
                parents: Parents::Pair([MonsterId(11), MonsterId(21)]),
            }]
        );
        assert_eq!(recipes[0].ingredients(), &[MonsterId(11), MonsterId(21)]);
    }

    #[test]
    fn four_grandparent_recipe_keeps_their_order() {
        let json = format!(
            "[{}]",
            row(2, 500, [None; 2], [Some(1), Some(2), Some(3), Some(4)])
        );
        let recipe = parse_recipes(&json).expect("valid fixture")[0];
        assert_eq!(
            recipe.parents,
            Parents::Grandparents([MonsterId(1), MonsterId(2), MonsterId(3), MonsterId(4)])
        );
        assert_eq!(recipe.ingredients().len(), 4);
    }

    #[test]
    fn mixed_or_incomplete_rows_are_refused() {
        let both = row(
            3,
            500,
            [Some(1), Some(2)],
            [Some(1), Some(2), Some(3), Some(4)],
        );
        let one_parent = row(4, 500, [Some(1), None], [None; 4]);
        for bad in [both, one_parent] {
            let error = parse_recipes(&format!("[{bad}]")).unwrap_err();
            assert!(matches!(
                error,
                CoreError::InvalidRow {
                    file: "MonsterSynthesis.json",
                    ..
                }
            ));
        }
    }
}
