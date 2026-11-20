//! Reverse synthesis: what a monster can be used to create.

use crate::ids::{MonsterId, RankId};
use crate::monster::Monster;
use crate::recipe::Recipe;

/// Can `monster` fill a recipe slot that asks for `slot`?
///
/// Yes if the slot is that very monster, or if the slot is a
/// "family + rank" rule of the same family and of the same rank or of the
/// wildcard rank ("Any"). A rule itself only fills its own slot.
pub(crate) fn fits(monster: &Monster, slot: &Monster, wildcard_rank: Option<RankId>) -> bool {
    if slot.id == monster.id {
        return true;
    }
    slot.is_family_rule()
        && !monster.is_family_rule()
        && slot.family == monster.family
        && (slot.rank == monster.rank || Some(slot.rank) == wildcard_rank)
}

/// The recipes in which `monster` can be an ingredient, each listed once.
///
/// Two lifetimes: `'a` for the recipes (the returned references point to
/// them), `'b` for the monsters returned by `slot_monster`. They are
/// independent, so the compiler must not assume they are the same.
pub(crate) fn children_of<'a, 'b>(
    monster: &Monster,
    recipes: impl IntoIterator<Item = &'a Recipe>,
    slot_monster: impl Fn(MonsterId) -> Option<&'b Monster>,
    wildcard_rank: Option<RankId>,
) -> Vec<&'a Recipe> {
    recipes
        .into_iter()
        // `any` stops at the first fitting slot: each recipe at most once.
        .filter(|recipe| {
            recipe
                .ingredients()
                .iter()
                .any(|&id| slot_monster(id).is_some_and(|slot| fits(monster, slot, wildcard_rank)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::FamilyId;
    use crate::recipe::Parents;

    const ANY: RankId = RankId(10);

    fn monster(id: u32, number: Option<u32>, family: u32, rank: u32) -> Monster {
        Monster {
            id: MonsterId(id),
            number,
            name_en: format!("m{id}"),
            name_fr: None,
            family: FamilyId(family),
            rank: RankId(rank),
            max_stats: None,
        }
    }

    // Real monsters: Slime (family 1, rank G=1), Dracky (family 2, rank G).
    // Rules: Slime Family (G), Slime Family (Any), Slime Family (F),
    // Dragon Family (G).
    fn slime() -> Monster {
        monster(1, Some(1), 1, 1)
    }
    fn dracky() -> Monster {
        monster(2, Some(50), 2, 1)
    }
    fn slime_g() -> Monster {
        monster(101, None, 1, 1)
    }
    fn slime_any() -> Monster {
        monster(102, None, 1, 10)
    }
    fn slime_f() -> Monster {
        monster(103, None, 1, 2)
    }
    fn dragon_g() -> Monster {
        monster(104, None, 2, 1)
    }

    #[test]
    fn monster_fits_its_own_slot() {
        assert!(fits(&slime(), &slime(), Some(ANY)));
        assert!(!fits(&slime(), &dracky(), Some(ANY)));
    }

    #[test]
    fn monster_fits_rule_of_its_family_and_rank_or_any() {
        assert!(fits(&slime(), &slime_g(), Some(ANY)));
        assert!(fits(&slime(), &slime_any(), Some(ANY)));
    }

    #[test]
    fn monster_does_not_fit_rule_of_other_rank_or_family() {
        assert!(!fits(&slime(), &slime_f(), Some(ANY)));
        assert!(!fits(&slime(), &dragon_g(), Some(ANY)));
    }

    #[test]
    fn a_rule_only_fits_its_own_slot() {
        assert!(fits(&slime_g(), &slime_g(), Some(ANY)));
        assert!(!fits(&slime_g(), &slime_any(), Some(ANY)));
    }

    fn pair(result: u32, a: u32, b: u32) -> Recipe {
        Recipe {
            result: MonsterId(result),
            parents: Parents::Pair([MonsterId(a), MonsterId(b)]),
        }
    }

    #[test]
    fn children_include_direct_and_rule_recipes_once_each() {
        let all = [
            slime(),
            dracky(),
            slime_g(),
            slime_any(),
            slime_f(),
            dragon_g(),
        ];
        let lookup = |id: MonsterId| all.iter().find(|m| m.id == id);
        let recipes = [
            pair(200, 1, 2),     // Slime + Dracky: direct
            pair(201, 101, 104), // Slime Family (G) + Dragon Family (G): rule
            pair(202, 103, 104), // Slime Family (F): no
            pair(203, 102, 101), // fits both slots: listed once
            Recipe {
                result: MonsterId(204),
                parents: Parents::Grandparents([
                    MonsterId(2),
                    MonsterId(1),
                    MonsterId(2),
                    MonsterId(2),
                ]),
            }, // as a grandparent
        ];
        let results: Vec<u32> = children_of(&slime(), &recipes, lookup, Some(ANY))
            .iter()
            .map(|r| r.result.0)
            .collect();
        assert_eq!(results, [200, 201, 203, 204]);
    }
}
