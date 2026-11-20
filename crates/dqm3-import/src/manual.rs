//! Hand-made corrections, applied after the automatic import.
//!
//! Every entry was checked by a human: English names from the MetalKid
//! side, tiers from in-game screenshots. Each applied correction is listed
//! in the report.

use crate::compare::WikiTalent;
use crate::wikitext::Tier;

/// List target (linked page, without `#fragment`) -> MetalKid English name,
/// for entries the automatic matching cannot pair. Keyed by target, not by
/// French name: two different talents are both called "Tourmenteur".
const EN_BY_TARGET: &[(&str, &str)] = &[
    ("Remèdiateur", "Soothsayer"),    // wiki typo: "Soothesayer"
    ("Parangon (talent)", "Paragon"), // page missing
    ("Gros bras", "Musclehead"),      // page missing
    ("Tourmenteur ( )", "Doomsayer"), // page missing; checked in game (Grouillâme)
    ("Sage", "Sage"),                 // no English name on the page
    ("Lordragon", "The Dragonlord"),  // wiki: "Dragonlord"
    ("Psaro l'Exterminateur", "Psaro the Manslayer"), // wiki: "Psaro"
    ("Jasper déchaîné", "Jasper Unbound"), // wiki: "Jasper"
    ("Mortégor", "Mordegon, Lord of Shadows"), // wiki: "Mordegon"
    ("Dragon du temps", "Wyrmhole Dragon"), // wiki: "Dimensional Dragon"
];

/// Tiers read in game, by MetalKid English name. They only fill points the
/// wiki does not have; they never replace wiki data.
const GAME_TIERS: &[(&str, &[(u32, &str)])] = &[
    // Grouillâme, fixed talent "Tourmenteur" (no wiki page).
    (
        "Doomsayer",
        &[
            (3, "Tempête de sable"),
            (10, "Crève-cœur"),
            (20, "Haleine fraîche"),
            (30, "Illusion"),
            (50, "Coup de l'assassin"),
            (70, "Souffle brûlant"),
            (85, "Souffle karstique"),
            (100, "Danse de la mort"),
            (120, "Ultra Mort incertaine"),
            (150, "Kamikaze"),
        ],
    ),
    // Gambastonneur, "Bon génie": the wiki stops at 120.
    ("Protector", &[(150, "Réflexion")]),
];

/// Applies the corrections to one talent and returns what was changed
/// (French, for the report).
///
/// `&mut WikiTalent` is a *mutable borrow*: the function may modify the
/// caller's talent in place, but does not own it. While this borrow is
/// alive, nobody else can read or write that talent: Rust checks it.
pub fn apply(target: &str, talent: &mut WikiTalent) -> Vec<String> {
    let mut notes = Vec::new();

    if let Some(&(_, en)) = EN_BY_TARGET.iter().find(|(t, _)| *t == target)
        && talent.en.as_deref() != Some(en)
    {
        let before = talent.en.as_deref().unwrap_or("aucun");
        notes.push(format!(
            "{} : nom EN « {before} » → « {en} » (correspondance manuelle)",
            talent.fr
        ));
        talent.en = Some(en.to_string());
    }

    let game_tiers = talent
        .en
        .as_deref()
        .and_then(|en| GAME_TIERS.iter().find(|(name, _)| *name == en))
        .map(|&(_, tiers)| tiers)
        .unwrap_or_default();
    for &(points, name) in game_tiers {
        if talent.tiers.iter().all(|t| t.points != points) {
            talent.tiers.push(Tier {
                points,
                name: name.to_string(),
            });
            notes.push(format!(
                "{} : palier {points} « {name} » ajouté (vérifié en jeu)",
                talent.fr
            ));
        }
    }
    talent.tiers.sort_by_key(|t| t.points);
    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn talent(fr: &str, en: Option<&str>, tiers: &[(u32, &str)]) -> WikiTalent {
        WikiTalent {
            fr: fr.into(),
            fr_page: None,
            en: en.map(Into::into),
            tiers: tiers
                .iter()
                .map(|&(points, name)| Tier {
                    points,
                    name: name.into(),
                })
                .collect(),
            source_url: String::new(),
        }
    }

    #[test]
    fn english_name_is_set_by_target_and_reported() {
        let mut t = talent("Lordragon", Some("Dragonlord"), &[]);
        let notes = apply("Lordragon", &mut t);
        assert_eq!(t.en.as_deref(), Some("The Dragonlord"));
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains("Lordragon") && notes[0].contains("The Dragonlord"));
    }

    #[test]
    fn homonyms_are_told_apart_by_target() {
        let mut storm = talent("Tourmenteur", Some("Storm Brewer"), &[]);
        assert!(apply("Tourmenteur", &mut storm).is_empty());
        assert_eq!(storm.en.as_deref(), Some("Storm Brewer"));

        let mut doom = talent("Tourmenteur", None, &[]);
        apply("Tourmenteur ( )", &mut doom);
        assert_eq!(doom.en.as_deref(), Some("Doomsayer"));
    }

    #[test]
    fn game_tiers_fill_a_missing_page() {
        let mut doom = talent("Tourmenteur", None, &[]);
        apply("Tourmenteur ( )", &mut doom);
        assert_eq!(doom.tiers.len(), 10);
        assert_eq!(
            doom.tiers[9],
            Tier {
                points: 150,
                name: "Kamikaze".into()
            }
        );
    }

    #[test]
    fn game_tiers_only_add_missing_points() {
        let mut protector = talent(
            "Bon génie",
            Some("Protector"),
            &[(3, "Attaque-attaque (wiki)"), (120, "Défense +40")],
        );
        let notes = apply("Bon génie", &mut protector);
        let points: Vec<u32> = protector.tiers.iter().map(|t| t.points).collect();
        assert_eq!(points, vec![3, 120, 150]);
        assert_eq!(
            protector.tiers[0].name, "Attaque-attaque (wiki)",
            "wiki data kept"
        );
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains("150") && notes[0].contains("Réflexion"));
    }

    #[test]
    fn untouched_talent_gives_no_note() {
        let mut t = talent("Assassin", Some("Assassin"), &[(3, "Crève-cœur")]);
        let before = t.clone();
        assert!(apply("Assassin", &mut t).is_empty());
        assert_eq!(t, before);
    }
}
