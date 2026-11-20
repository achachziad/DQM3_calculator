//! Monster search: text (French or English, partial, accent-insensitive),
//! filters and sorting. Pure functions over a monster slice; `Database`
//! exposes them as `search_monsters`.

use crate::ids::{FamilyId, MonsterId, RankId, TalentId};
use crate::monster::{Monster, Stats};
use crate::talent::MonsterTalent;
use std::cmp::Ordering;

/// Search key: lowercase, accents removed, ligatures expanded
/// ("Bottœil NOCTURNE" -> "bottoeil nocturne").
///
/// Written by hand instead of adding a Unicode crate: the names only use
/// French Latin letters (checked in docs/DATA.md).
pub fn fold(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    // `flat_map`: one uppercase char can lowercase to several chars.
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => folded.push('a'),
            'ç' => folded.push('c'),
            'è' | 'é' | 'ê' | 'ë' => folded.push('e'),
            'ì' | 'í' | 'î' | 'ï' => folded.push('i'),
            'ñ' => folded.push('n'),
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => folded.push('o'),
            'ù' | 'ú' | 'û' | 'ü' => folded.push('u'),
            'ý' | 'ÿ' => folded.push('y'),
            'œ' => folded.push_str("oe"),
            'æ' => folded.push_str("ae"),
            other => folded.push(other),
        }
    }
    folded
}

/// The name shown first: French, or English when there is no French name.
fn display_name(monster: &Monster) -> &str {
    monster.name_fr.as_deref().unwrap_or(&monster.name_en)
}

/// Which stat to sort on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    Hp,
    Mp,
    Att,
    Def,
    Agi,
    Wis,
}

impl Stat {
    fn of(self, stats: &Stats) -> u32 {
        match self {
            Stat::Hp => stats.hp,
            Stat::Mp => stats.mp,
            Stat::Att => stats.att,
            Stat::Def => stats.def,
            Stat::Agi => stats.agi,
            Stat::Wis => stats.wis,
        }
    }
}

/// Sort order of the results.
///
/// `#[derive(Default)]` + `#[default]`: `SortKey::default()` is `Number`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// In-game number (family rules, which have none, come last).
    #[default]
    Number,
    /// Rank G → X (`RankId` follows the game order).
    Rank,
    /// French name (English if none), accent-insensitive.
    Name,
    /// A max stat (monsters without stats come last).
    Stat(Stat),
}

/// A monster search. `Default` gives "everything, sorted by number,
/// without the family rules".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MonsterQuery {
    /// Part of the French or English name; empty = no text filter.
    pub text: String,
    pub family: Option<FamilyId>,
    pub rank: Option<RankId>,
    /// Only monsters that can carry this talent (fixed or random).
    pub talent: Option<TalentId>,
    /// Also list the "Slime Family (G)"-like synthesis rules.
    pub include_family_rules: bool,
    pub sort: SortKey,
    pub descending: bool,
}

/// Runs a query.
///
/// Lifetime `'a`: the returned references point into `monsters`, so they
/// are valid as long as that slice is. `talents_of` is any function giving
/// a monster's talents (the database's lookup in practice, a small closure
/// in tests).
pub(crate) fn search<'a, 'b>(
    monsters: &'a [Monster],
    talents_of: impl Fn(MonsterId) -> &'b [MonsterTalent],
    query: &MonsterQuery,
) -> Vec<&'a Monster> {
    let text = fold(query.text.trim());
    let matches_text = |m: &Monster| {
        text.is_empty()
            || fold(&m.name_en).contains(&text)
            || m.name_fr
                .as_deref()
                .is_some_and(|fr| fold(fr).contains(&text))
    };

    let mut results: Vec<&Monster> = monsters
        .iter()
        .filter(|m| query.include_family_rules || !m.is_family_rule())
        .filter(|m| query.family.is_none_or(|f| m.family == f))
        .filter(|m| query.rank.is_none_or(|r| m.rank == r))
        .filter(|m| {
            query
                .talent
                .is_none_or(|t| talents_of(m.id).iter().any(|mt| mt.talent == t))
        })
        .filter(|m| matches_text(m))
        .collect();

    results.sort_by(|a, b| {
        // 1. Monsters without a value for the key always go last.
        has_value(b, query.sort)
            .cmp(&has_value(a, query.sort))
            // 2. The value itself, reversed if descending.
            .then_with(|| {
                let order = compare(a, b, query.sort);
                if query.descending {
                    order.reverse()
                } else {
                    order
                }
            })
            // 3. Stable tie-break: number (none last).
            .then_with(|| compare(a, b, SortKey::Number))
    });
    results
}

/// Does this monster have a value to sort on?
fn has_value(monster: &Monster, key: SortKey) -> bool {
    match key {
        SortKey::Number => monster.number.is_some(),
        SortKey::Stat(_) => monster.max_stats.is_some(),
        SortKey::Rank | SortKey::Name => true,
    }
}

/// Ascending ordering of two monsters on `key` alone.
fn compare(a: &Monster, b: &Monster, key: SortKey) -> Ordering {
    match key {
        // `Option` is ordered `None < Some`; `has_value` already put the
        // `None`s last, so here this only compares real values.
        SortKey::Number => a.number.cmp(&b.number),
        SortKey::Rank => a.rank.cmp(&b.rank),
        SortKey::Name => fold(display_name(a)).cmp(&fold(display_name(b))),
        SortKey::Stat(stat) => {
            let value = |m: &Monster| m.max_stats.as_ref().map(|s| stat.of(s));
            value(a).cmp(&value(b))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_removes_case_accents_and_ligatures() {
        assert_eq!(fold("Méthanosaure"), "methanosaure");
        assert_eq!(fold("MÉTHANOSAURE"), "methanosaure");
        assert_eq!(fold("Bottœil nocturne"), "bottoeil nocturne");
        assert_eq!(fold("Crâne Gelé Façon Noël"), "crane gele facon noel");
        assert_eq!(fold("Œuf"), "oeuf");
        assert_eq!(fold("Slime"), "slime");
    }

    fn monster(
        id: u32,
        number: Option<u32>,
        en: &str,
        fr: Option<&str>,
        family: u32,
        rank: u32,
        hp: u32,
    ) -> Monster {
        Monster {
            id: MonsterId(id),
            number,
            identifier: format!("m{id}"),
            name_en: en.into(),
            name_fr: fr.map(Into::into),
            family: FamilyId(family),
            rank: RankId(rank),
            max_stats: number.map(|_| Stats {
                hp,
                mp: 0,
                att: 0,
                def: 0,
                agi: 0,
                wis: 0,
            }),
        }
    }

    /// Slime (#1, Slime family, G), Airbagon (#68, Nature, F),
    /// Healslime (#8, Slime, F), and a family rule.
    fn sample() -> Vec<Monster> {
        vec![
            monster(1, Some(1), "Slime", Some("Gluant"), 1, 1, 400),
            monster(2, Some(68), "Airbagon", Some("Méthanosaure"), 3, 2, 900),
            monster(3, Some(8), "Healslime", Some("Médigluant"), 1, 2, 500),
            monster(4, None, "Slime Family (G)", None, 1, 1, 0),
        ]
    }

    fn no_talents(_: MonsterId) -> &'static [MonsterTalent] {
        &[]
    }

    fn names(results: &[&Monster]) -> Vec<String> {
        results.iter().map(|m| m.name_en.clone()).collect()
    }

    fn run(query: MonsterQuery) -> Vec<String> {
        let monsters = sample();
        names(&search(&monsters, no_talents, &query))
    }

    #[test]
    fn default_query_lists_real_monsters_by_number() {
        assert_eq!(
            run(MonsterQuery::default()),
            ["Slime", "Healslime", "Airbagon"]
        );
    }

    #[test]
    fn family_rules_can_be_included() {
        let query = MonsterQuery {
            include_family_rules: true,
            ..Default::default()
        };
        assert_eq!(
            run(query).last().map(String::as_str),
            Some("Slime Family (G)")
        );
    }

    #[test]
    fn text_matches_french_name_without_accents() {
        let query = MonsterQuery {
            text: "methano".into(),
            ..Default::default()
        };
        assert_eq!(run(query), ["Airbagon"]);
    }

    #[test]
    fn text_matches_english_name_and_parts_of_names() {
        let query = MonsterQuery {
            text: "SLIME".into(),
            ..Default::default()
        };
        assert_eq!(run(query), ["Slime", "Healslime"]);

        let query = MonsterQuery {
            text: "gluant".into(),
            ..Default::default()
        };
        assert_eq!(run(query), ["Slime", "Healslime"], "Gluant and Médigluant");
    }

    #[test]
    fn unknown_text_finds_nothing() {
        let query = MonsterQuery {
            text: "introuvable".into(),
            ..Default::default()
        };
        assert!(run(query).is_empty());
    }

    #[test]
    fn family_and_rank_filters_combine() {
        let query = MonsterQuery {
            family: Some(FamilyId(1)),
            rank: Some(RankId(2)),
            ..Default::default()
        };
        assert_eq!(run(query), ["Healslime"]);
    }

    #[test]
    fn talent_filter_keeps_monsters_that_can_carry_it() {
        let healer = [MonsterTalent {
            talent: TalentId(9),
            fixed: false,
        }];
        let talents_of =
            |id: MonsterId| -> &[MonsterTalent] { if id == MonsterId(3) { &healer } else { &[] } };
        let monsters = sample();
        let query = MonsterQuery {
            talent: Some(TalentId(9)),
            ..Default::default()
        };
        assert_eq!(names(&search(&monsters, talents_of, &query)), ["Healslime"]);
    }

    #[test]
    fn sort_by_rank_then_number_and_descending_stats() {
        let query = MonsterQuery {
            sort: SortKey::Rank,
            ..Default::default()
        };
        assert_eq!(run(query), ["Slime", "Healslime", "Airbagon"]);

        let query = MonsterQuery {
            sort: SortKey::Stat(Stat::Hp),
            descending: true,
            ..Default::default()
        };
        assert_eq!(run(query), ["Airbagon", "Healslime", "Slime"]);
    }

    #[test]
    fn sort_by_name_uses_french_without_accents() {
        // Gluant < Médigluant < Méthanosaure
        let query = MonsterQuery {
            sort: SortKey::Name,
            ..Default::default()
        };
        assert_eq!(run(query), ["Slime", "Healslime", "Airbagon"]);
    }
}
