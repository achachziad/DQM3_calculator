//! The loaded game data: the entry point of the library.

use crate::CoreError;
use crate::family::{Family, Rank, parse_families, parse_ranks};
use crate::ids::{
    FamilyId, LocationId, MonsterId, RankId, SeasonId, SizeId, SkillId, TalentId, TraitId,
    WeatherId,
};
use crate::index::MonsterIndex;
use crate::load::group_by;
use crate::location::{Recruitment, parse_recruitments};
use crate::monster::{Monster, parse_monsters};
use crate::named::{
    Location, Named, Season, Size, Weather, parse_locations, parse_seasons, parse_sizes,
    parse_weathers,
};
use crate::recipe::{Recipe, parse_recipes};
use crate::search::{MonsterQuery, search};
use crate::synthesis::children_of;
use crate::talent::{
    MonsterTalent, Skill, Talent, TalentEvolution, parse_monster_talents, parse_skills,
    parse_talent_evolutions, parse_talents,
};
use crate::traits::{MonsterTrait, Trait, parse_monster_traits, parse_traits};
use std::collections::HashMap;
use std::hash::Hash;

/// MetalKid's tables, embedded in the executable at compile time by
/// `include_str!` (path relative to this source file). No file to ship or
/// find at runtime; if `data/` is empty (submodule not initialised), the
/// build fails with a clear "file not found".
mod embedded {
    pub const MONSTER: &str = include_str!("../../../data/data/json/raw/Monster.json");
    pub const FAMILY: &str = include_str!("../../../data/data/json/raw/Family.json");
    pub const RANK: &str = include_str!("../../../data/data/json/raw/Rank.json");
    pub const SYNTHESIS: &str = include_str!("../../../data/data/json/raw/MonsterSynthesis.json");
    pub const TALENT: &str = include_str!("../../../data/data/json/raw/Talent.json");
    pub const TALENT_SKILL: &str = include_str!("../../../data/data/json/raw/TalentSkill.json");
    pub const TALENT_TRAIT: &str = include_str!("../../../data/data/json/raw/TalentTrait.json");
    pub const SKILL: &str = include_str!("../../../data/data/json/raw/Skill.json");
    pub const MONSTER_TALENT: &str = include_str!("../../../data/data/json/raw/MonsterTalent.json");
    pub const TALENT_SYNTHESIS: &str =
        include_str!("../../../data/data/json/raw/TalentSynthesis.json");
    pub const TALENT_SYNTHESIS_TALENT: &str =
        include_str!("../../../data/data/json/raw/TalentSynthesisTalent.json");
    pub const TRAIT: &str = include_str!("../../../data/data/json/raw/Trait.json");
    pub const MONSTER_TRAIT: &str = include_str!("../../../data/data/json/raw/MonsterTrait.json");
    pub const SIZE: &str = include_str!("../../../data/data/json/raw/Size.json");
    pub const MONSTER_LOCATION: &str =
        include_str!("../../../data/data/json/raw/MonsterLocation.json");
    pub const LOCATION: &str = include_str!("../../../data/data/json/raw/Location.json");
    pub const SEASON: &str = include_str!("../../../data/data/json/raw/Season.json");
    pub const WEATHER: &str = include_str!("../../../data/data/json/raw/Weather.json");
}

/// The group stored under `key`, or an empty slice if there is none.
/// `&[]` is a valid empty slice that lives forever (`'static`), so it can
/// be returned with any lifetime.
fn group<'a, K: Eq + Hash, T>(map: &'a HashMap<K, Vec<T>>, key: &K) -> &'a [T] {
    map.get(key).map_or(&[], Vec::as_slice)
}

/// Turns `(key, value)` pairs into `key -> [values]`.
fn group_pairs<K: Eq + Hash + Copy, V>(pairs: Vec<(K, V)>) -> HashMap<K, Vec<V>> {
    group_by(pairs, |(key, _)| *key)
        .into_iter()
        .map(|(key, pairs)| (key, pairs.into_iter().map(|(_, value)| value).collect()))
        .collect()
}

/// `id -> item` map, for tables looked up by id.
fn by_id<I: Eq + Hash, T>(items: Vec<T>, id: impl Fn(&T) -> I) -> HashMap<I, T> {
    items.into_iter().map(|item| (id(&item), item)).collect()
}

/// Linear search in a small "id + name" table.
fn find_named<I: PartialEq>(items: &[Named<I>], id: I) -> Option<&Named<I>> {
    items.iter().find(|item| item.id == id)
}

/// All the game data, loaded once.
#[derive(Debug)]
pub struct Database {
    monsters: Vec<Monster>,
    /// Positions in `monsters`, by id / number / name.
    monster_index: MonsterIndex,
    families: Vec<Family>,
    ranks: Vec<Rank>,
    /// Recipes grouped by the monster they produce.
    recipes_by_result: HashMap<MonsterId, Vec<Recipe>>,
    talents: HashMap<TalentId, Talent>,
    skills: HashMap<SkillId, Skill>,
    traits: HashMap<TraitId, Trait>,
    talents_by_monster: HashMap<MonsterId, Vec<MonsterTalent>>,
    /// Reverse of `talents_by_monster`.
    monsters_by_talent: HashMap<TalentId, Vec<MonsterId>>,
    talent_evolutions: Vec<TalentEvolution>,
    traits_by_monster: HashMap<MonsterId, Vec<MonsterTrait>>,
    recruitments_by_monster: HashMap<MonsterId, Vec<Recruitment>>,
    sizes: Vec<Size>,
    locations: Vec<Location>,
    seasons: Vec<Season>,
    weathers: Vec<Weather>,
}

impl Database {
    /// Loads the embedded MetalKid data and builds the indexes.
    /// Fails if a file is invalid or if two monsters share a key.
    pub fn load() -> Result<Self, CoreError> {
        let monsters = parse_monsters(embedded::MONSTER)?;
        let monster_index = MonsterIndex::build(&monsters)?;

        let monster_talents = parse_monster_talents(embedded::MONSTER_TALENT)?;
        let monsters_by_talent = group_pairs(
            monster_talents
                .iter()
                .map(|(monster, mt)| (mt.talent, *monster))
                .collect(),
        );

        Ok(Self {
            monsters,
            monster_index,
            families: parse_families(embedded::FAMILY)?,
            ranks: parse_ranks(embedded::RANK)?,
            recipes_by_result: group_by(parse_recipes(embedded::SYNTHESIS)?, |r| r.result),
            talents: by_id(
                parse_talents(
                    embedded::TALENT,
                    embedded::TALENT_SKILL,
                    embedded::TALENT_TRAIT,
                )?,
                |t| t.id,
            ),
            skills: by_id(parse_skills(embedded::SKILL)?, |s| s.id),
            traits: by_id(parse_traits(embedded::TRAIT)?, |t| t.id),
            talents_by_monster: group_pairs(monster_talents),
            monsters_by_talent,
            talent_evolutions: parse_talent_evolutions(
                embedded::TALENT_SYNTHESIS,
                embedded::TALENT_SYNTHESIS_TALENT,
            )?,
            traits_by_monster: group_pairs(parse_monster_traits(embedded::MONSTER_TRAIT)?),
            recruitments_by_monster: group_pairs(parse_recruitments(embedded::MONSTER_LOCATION)?),
            sizes: parse_sizes(embedded::SIZE)?,
            locations: parse_locations(embedded::LOCATION)?,
            seasons: parse_seasons(embedded::SEASON)?,
            weathers: parse_weathers(embedded::WEATHER)?,
        })
    }

    // ---------- Talents ----------

    /// The talent with this id, if any.
    pub fn talent(&self, id: TalentId) -> Option<&Talent> {
        self.talents.get(&id)
    }

    /// All talents, in no particular order.
    ///
    /// `impl Iterator<Item = &Talent>`: "some iterator over talents"; the
    /// caller can loop, filter or collect without knowing its exact type.
    pub fn talents(&self) -> impl Iterator<Item = &Talent> {
        self.talents.values()
    }

    /// The skill with this id, if any.
    pub fn skill(&self, id: SkillId) -> Option<&Skill> {
        self.skills.get(&id)
    }

    /// The talents of a monster (fixed and random).
    pub fn monster_talents(&self, id: MonsterId) -> &[MonsterTalent] {
        group(&self.talents_by_monster, &id)
    }

    /// The monsters that can have this talent.
    pub fn monsters_with_talent(&self, id: TalentId) -> &[MonsterId] {
        group(&self.monsters_by_talent, &id)
    }

    /// The ways to obtain this talent by evolution. A linear scan is fine:
    /// there are only 67 evolutions.
    pub fn talent_evolutions_into(&self, id: TalentId) -> Vec<&TalentEvolution> {
        self.talent_evolutions
            .iter()
            .filter(|e| e.result == id)
            .collect()
    }

    /// The evolutions this talent takes part in.
    pub fn talent_evolutions_from(&self, id: TalentId) -> Vec<&TalentEvolution> {
        self.talent_evolutions
            .iter()
            .filter(|e| e.ingredients.contains(&id))
            .collect()
    }

    // ---------- Traits ----------

    /// The trait with this id, if any. (`trait` is a Rust keyword.)
    pub fn trait_by_id(&self, id: TraitId) -> Option<&Trait> {
        self.traits.get(&id)
    }

    /// The traits of a monster, for both sizes.
    pub fn monster_traits(&self, id: MonsterId) -> &[MonsterTrait] {
        group(&self.traits_by_monster, &id)
    }

    /// The size with this id (Small / Large), if any.
    pub fn size(&self, id: SizeId) -> Option<&Size> {
        find_named(&self.sizes, id)
    }

    // ---------- Recruitment ----------

    /// Where and when this monster appears in the wild. Empty if it is
    /// only obtained by synthesis or eggs.
    pub fn recruitments(&self, id: MonsterId) -> &[Recruitment] {
        group(&self.recruitments_by_monster, &id)
    }

    pub fn location(&self, id: LocationId) -> Option<&Location> {
        find_named(&self.locations, id)
    }

    pub fn season(&self, id: SeasonId) -> Option<&Season> {
        find_named(&self.seasons, id)
    }

    pub fn weather(&self, id: WeatherId) -> Option<&Weather> {
        find_named(&self.weathers, id)
    }

    // ---------- Monsters ----------

    /// The recipes that produce this monster. Empty for monsters that are
    /// only recruited or hatched (Slime, Metal Slime, egg-only monsters…).
    pub fn recipes_for(&self, id: MonsterId) -> &[Recipe] {
        group(&self.recipes_by_result, &id)
    }

    /// The monster with this id, if any.
    pub fn monster(&self, id: MonsterId) -> Option<&Monster> {
        self.monster_at(self.monster_index.by_id(id))
    }

    /// The monster with this in-game number (1–526), if any.
    pub fn monster_by_number(&self, number: u32) -> Option<&Monster> {
        self.monster_at(self.monster_index.by_number(number))
    }

    /// The monster with this exact French or English name, ignoring case
    /// (accents must match; accent-insensitive search is a separate feature).
    pub fn monster_by_name(&self, name: &str) -> Option<&Monster> {
        self.monster_at(self.monster_index.by_name(name))
    }

    /// Position from an index -> monster. `.get` returns `None` instead of
    /// panicking like `[]` would on a bad position.
    fn monster_at(&self, position: Option<usize>) -> Option<&Monster> {
        position.and_then(|p| self.monsters.get(p))
    }

    /// The family with this id, if any.
    ///
    /// A linear search is fine here: there are only 8 families (a
    /// `HashMap` would cost more than it saves at this size).
    pub fn family(&self, id: FamilyId) -> Option<&Family> {
        self.families.iter().find(|f| f.id == id)
    }

    /// The rank with this id, if any (10 ranks: linear search too).
    pub fn rank(&self, id: RankId) -> Option<&Rank> {
        self.ranks.iter().find(|r| r.id == id)
    }

    /// All monsters, including the "family + rank" rules.
    ///
    /// Returns a slice `&[Monster]`: a read-only view on the database's
    /// own vector, borrowed for as long as the `Database` lives.
    pub fn monsters(&self) -> &[Monster] {
        &self.monsters
    }

    // ---------- Queries ----------

    /// Searches monsters: text (French or English, partial, without
    /// accents), filters and sort. See `MonsterQuery`.
    pub fn search_monsters(&self, query: &MonsterQuery) -> Vec<&Monster> {
        search(&self.monsters, |id| self.monster_talents(id), query)
    }

    /// Reverse synthesis: the recipes in which this monster can be used,
    /// directly or through a "family + rank" rule, sorted by the number of
    /// the monster they create. Empty for an unknown id.
    pub fn children_of(&self, id: MonsterId) -> Vec<&Recipe> {
        let Some(monster) = self.monster(id) else {
            return Vec::new();
        };
        let wildcard_rank = self.ranks.iter().find(|r| r.is_wildcard()).map(|r| r.id);
        let all_recipes = self.recipes_by_result.values().flatten();

        let mut children = children_of(monster, all_recipes, |id| self.monster(id), wildcard_rank);
        // `sort_by_key` with a tuple: number first (none last), then id.
        children.sort_by_key(|r| {
            let number = self.monster(r.result).and_then(|m| m.number);
            (number.is_none(), number, r.result)
        });
        children
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

    #[test]
    fn monster_family_and_rank_are_resolved() {
        let db = Database::load().expect("embedded data is valid");
        let slime = db
            .monsters()
            .iter()
            .find(|m| m.number == Some(1))
            .expect("monster #1 exists");
        assert_eq!(
            db.family(slime.family).map(|f| f.name_en.as_str()),
            Some("Slime")
        );
        assert_eq!(db.rank(slime.rank).map(|r| r.name.as_str()), Some("G"));
    }

    #[test]
    fn real_monsters_are_found_by_number_and_names() {
        let db = Database::load().expect("embedded data is valid");
        let by_number = db.monster_by_number(1).expect("monster #1");
        assert_eq!(by_number.name_en, "Slime");
        assert_eq!(db.monster_by_name("Gluant"), Some(by_number));
        assert_eq!(db.monster_by_name("slime"), Some(by_number));
        assert_eq!(db.monster(by_number.id), Some(by_number));
        assert_eq!(
            db.monster_by_name("Méthanosaure")
                .map(|m| m.name_en.as_str()),
            Some("Airbagon")
        );
        assert_eq!(db.monster_by_name("Introuvable"), None);
    }

    fn monster_named<'a>(db: &'a Database, name: &str) -> &'a Monster {
        db.monster_by_name(name)
            .unwrap_or_else(|| panic!("{name} exists"))
    }

    #[test]
    fn real_recipes_are_grouped_by_result() {
        let db = Database::load().expect("embedded data is valid");
        let total: usize = db.recipes_by_result.values().map(Vec::len).sum();
        assert_eq!(total, 714);
        assert_eq!(db.recipes_by_result.len(), 504);
    }

    #[test]
    fn monster_without_recipe_has_an_empty_list() {
        let db = Database::load().expect("embedded data is valid");
        assert!(db.recipes_for(monster_named(&db, "Gluant").id).is_empty());
    }

    #[test]
    fn family_rule_recipe_uses_placeholders() {
        // Wing Slime = Slime Family (G) + Dragon Family (G), among others.
        let db = Database::load().expect("embedded data is valid");
        let wing = monster_named(&db, "Wing Slime");
        let uses_rules = db.recipes_for(wing.id).iter().any(|r| {
            r.ingredients()
                .iter()
                .all(|&id| db.monster(id).is_some_and(Monster::is_family_rule))
        });
        assert!(uses_rules);
    }

    #[test]
    fn boss_recipe_has_four_grandparents() {
        let db = Database::load().expect("embedded data is valid");
        let zoma = monster_named(&db, "Zoma");
        assert!(
            db.recipes_for(zoma.id)
                .iter()
                .any(|r| matches!(r.parents, crate::Parents::Grandparents(_)))
        );
    }

    #[test]
    fn every_recipe_ingredient_is_a_known_monster() {
        let db = Database::load().expect("embedded data is valid");
        for recipe in db.recipes_by_result.values().flatten() {
            assert!(db.monster(recipe.result).is_some());
            for &id in recipe.ingredients() {
                assert!(db.monster(id).is_some(), "unknown ingredient {id}");
            }
        }
    }

    fn talent_named<'a>(db: &'a Database, name: &str) -> &'a Talent {
        db.talents()
            .find(|t| t.name_en == name)
            .unwrap_or_else(|| panic!("talent {name} exists"))
    }

    #[test]
    fn real_talent_tiers_resolve_to_skills() {
        let db = Database::load().expect("embedded data is valid");
        assert_eq!(db.talents().count(), 241);
        let assassin = talent_named(&db, "Assassin");
        assert_eq!(db.talent(assassin.id), Some(assassin));
        let points: Vec<u32> = assassin.tiers.iter().map(|t| t.points).collect();
        assert_eq!(points, [3, 10, 20, 30, 50, 70, 85, 100]);
        let crate::Reward::Skill(first) = assassin.tiers[0].reward else {
            panic!("first Assassin tier is a skill");
        };
        assert_eq!(
            db.skill(first).map(|s| s.name_en.as_str()),
            Some("Heart Breaker")
        );
    }

    #[test]
    fn monster_talents_have_one_fixed_talent() {
        let db = Database::load().expect("embedded data is valid");
        let slime = monster_named(&db, "Gluant");
        let talents = db.monster_talents(slime.id);
        let names: Vec<(&str, bool)> = talents
            .iter()
            .map(|mt| {
                (
                    db.talent(mt.talent).map_or("?", |t| t.name_en.as_str()),
                    mt.fixed,
                )
            })
            .collect();
        assert_eq!(
            names,
            [
                ("Slimer", true),
                ("Agility Booster I", false),
                ("Zap Ward", false)
            ]
        );
    }

    #[test]
    fn source_anomaly_two_fixed_talents_is_accepted() {
        let db = Database::load().expect("embedded data is valid");
        let rubble = monster_named(&db, "Rubble Slime");
        assert_eq!(
            db.monster_talents(rubble.id)
                .iter()
                .filter(|t| t.fixed)
                .count(),
            2
        );
    }

    #[test]
    fn monsters_with_a_talent_are_listed() {
        let db = Database::load().expect("embedded data is valid");
        let doomsayer = talent_named(&db, "Doomsayer");
        let soulspawn = monster_named(&db, "Grouillâme");
        let holders = db.monsters_with_talent(doomsayer.id);
        assert_eq!(holders.len(), 4);
        assert!(holders.contains(&soulspawn.id));
    }

    #[test]
    fn talent_evolutions_work_both_ways() {
        let db = Database::load().expect("embedded data is valid");
        let chilly = talent_named(&db, "Chilly Receptionist");
        let frosty = talent_named(&db, "Frosty Receptionist");
        let into_frosty = db.talent_evolutions_into(frosty.id);
        assert_eq!(into_frosty.len(), 1);
        assert_eq!(into_frosty[0].ingredients, vec![chilly.id]);
        assert!(
            db.talent_evolutions_from(chilly.id)
                .iter()
                .any(|e| e.result == frosty.id)
        );

        let polariser = talent_named(&db, "Polariser");
        assert_eq!(db.talent_evolutions_into(polariser.id).len(), 10);
    }

    #[test]
    fn monster_traits_cover_both_sizes() {
        let db = Database::load().expect("embedded data is valid");
        let slime = monster_named(&db, "Gluant");
        // Same layout for all 526 monsters: Small 1/20/40, Large 1/1/60.
        let mut layout: Vec<(&str, u32)> = db
            .monster_traits(slime.id)
            .iter()
            .filter_map(|t| {
                db.size(t.size)
                    .map(|s| (s.name_en.as_str(), t.unlock_level))
            })
            .collect();
        layout.sort();
        assert_eq!(
            layout,
            [
                ("Large", 1),
                ("Large", 1),
                ("Large", 60),
                ("Small", 1),
                ("Small", 20),
                ("Small", 40)
            ]
        );
    }

    #[test]
    fn recruitment_places_resolve() {
        let db = Database::load().expect("embedded data is valid");
        assert_eq!(db.recruitments_by_monster.len(), 213);

        let slime = monster_named(&db, "Gluant");
        let first = db
            .recruitments(slime.id)
            .first()
            .expect("Slime is found in the wild");
        assert!(db.location(first.location).is_some());
        assert!(db.season(first.season).is_some());
        assert!(db.weather(first.weather).is_some());

        // Egg-only monster: no recruitment, no recipe.
        let barracuda = monster_named(&db, "Barracuda");
        assert!(db.recruitments(barracuda.id).is_empty());
        assert!(db.recipes_for(barracuda.id).is_empty());
    }

    #[test]
    fn every_reference_in_talents_traits_and_places_resolves() {
        let db = Database::load().expect("embedded data is valid");
        for talent in db.talents() {
            for tier in &talent.tiers {
                let known = match tier.reward {
                    crate::Reward::Skill(id) => db.skill(id).is_some(),
                    crate::Reward::Trait(id) => db.trait_by_id(id).is_some(),
                };
                assert!(known, "{} tier {} unknown", talent.name_en, tier.points);
            }
        }
        for monster in db.monsters() {
            for mt in db.monster_talents(monster.id) {
                assert!(db.talent(mt.talent).is_some());
            }
            for t in db.monster_traits(monster.id) {
                assert!(db.trait_by_id(t.trait_id).is_some() && db.size(t.size).is_some());
            }
            for r in db.recruitments(monster.id) {
                assert!(db.location(r.location).is_some());
            }
        }
        for evolution in &db.talent_evolutions {
            assert!(db.talent(evolution.result).is_some());
        }
    }

    #[test]
    fn search_on_real_data() {
        let db = Database::load().expect("embedded data is valid");
        assert_eq!(db.search_monsters(&MonsterQuery::default()).len(), 526);

        let query = MonsterQuery {
            text: "methanosaure".into(),
            ..Default::default()
        };
        let found: Vec<&str> = db
            .search_monsters(&query)
            .iter()
            .map(|m| m.name_en.as_str())
            .collect();
        assert_eq!(found, ["Airbagon"]);

        let doomsayer = talent_named(&db, "Doomsayer");
        let query = MonsterQuery {
            talent: Some(doomsayer.id),
            ..Default::default()
        };
        assert_eq!(db.search_monsters(&query).len(), 4);
    }

    #[test]
    fn reverse_synthesis_on_real_data() {
        let db = Database::load().expect("embedded data is valid");
        let slime = monster_named(&db, "Gluant");
        let wing = monster_named(&db, "Wing Slime");
        let children = db.children_of(slime.id);
        // Wing Slime = Slime Family (G) + Dragon Family (G): Slime fits.
        assert!(children.iter().any(|r| r.result == wing.id));

        let numbers: Vec<Option<u32>> = children
            .iter()
            .map(|r| db.monster(r.result).and_then(|m| m.number))
            .collect();
        assert!(numbers.is_sorted(), "sorted by number of the result");

        assert!(db.children_of(MonsterId(999_999)).is_empty());
    }

    #[test]
    fn every_family_has_a_french_name() {
        let db = Database::load().expect("embedded data is valid");
        let names: Vec<Option<&str>> = (1..=8)
            .map(|id| db.family(FamilyId(id)).and_then(|f| f.name_fr.as_deref()))
            .collect();
        assert_eq!(
            names,
            [
                "Gluant",
                "Dragon",
                "Nature",
                "Bête",
                "Matériel",
                "Démon",
                "Mort-vivant",
                "???"
            ]
            .map(Some)
        );
    }

    #[test]
    fn unknown_ids_give_none() {
        let db = Database::load().expect("embedded data is valid");
        assert_eq!(db.family(FamilyId(999)), None);
        assert_eq!(db.rank(RankId(999)), None);
    }
}
