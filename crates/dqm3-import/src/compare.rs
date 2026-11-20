//! Matches French wiki talents with MetalKid talents (by English name),
//! builds the output records and the validation report. Pure logic, no I/O.

use crate::api::{WikiPage, page_url};
use crate::wikitext::{self, ListEntry, Tier};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// A talent as described by MetalKid (the reference data).
#[derive(Debug, Clone)]
pub struct MkTalent {
    pub en: String,
    pub ja: String,
    /// Points of every tier (skills and traits), sorted.
    pub thresholds: Vec<u32>,
}

/// A talent of the French wiki list, with what its page says.
#[derive(Debug, Clone, PartialEq)]
pub struct WikiTalent {
    /// In-game DQM3 name, from the list ("Espoir de Flamme").
    pub fr: String,
    /// Title of the page it leads to ("Apprenti de Flamme"),
    /// `None` if that page does not exist.
    pub fr_page: Option<String>,
    /// From the `en` link, or the intro sentence as a fallback.
    pub en: Option<String>,
    pub tiers: Vec<Tier>,
    pub source_url: String,
}

/// Combines a list entry with its (redirect-resolved) page.
pub fn build_talent(entry: &ListEntry, page: Option<&WikiPage>) -> WikiTalent {
    let Some(page) = page else {
        return WikiTalent {
            fr: entry.name.clone(),
            fr_page: None,
            en: None,
            tiers: Vec::new(),
            source_url: page_url(&entry.target),
        };
    };

    // `[[Page#Section]]`: the talent is described in that section
    // (possibly with its own DQM3 sub-section); otherwise the DQM3 one.
    let body = match &entry.fragment {
        Some(fragment) => wikitext::section(&page.content, fragment)
            .map(|s| wikitext::dqm3_section(s).unwrap_or(s)),
        None => wikitext::dqm3_section(&page.content),
    };
    let en = page
        .en_link
        .clone()
        .or_else(|| wikitext::english_from_intro(&page.content))
        .map(|en| wikitext::clean_title(&en));
    let source_url = match &entry.fragment {
        Some(fragment) => format!("{}#{}", page.url, fragment.replace(' ', "_")),
        None => page.url.clone(),
    };

    WikiTalent {
        fr: entry.name.clone(),
        fr_page: Some(wikitext::clean_title(&page.title)),
        en,
        tiers: body.map(wikitext::parse_tiers).unwrap_or_default(),
        source_url,
    }
}

/// One entry of `data-fr/talents.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrTalent {
    pub en: Option<String>,
    pub fr: String,
    /// Page title when it differs from `fr` (often the DQM4 name):
    /// useful as a search alias.
    pub fr_page: Option<String>,
    pub ja: Option<String>,
    pub thresholds: Vec<u32>,
    /// French names of what each tier unlocks (skills and traits),
    /// in the same order as `thresholds`.
    pub skills_fr: Vec<String>,
    pub source_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdDiff {
    pub en: String,
    pub fr: String,
    pub metalkid: Vec<u32>,
    pub wiki: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unmatched {
    pub fr: String,
    pub en: Option<String>,
    /// `None`: the linked page does not exist on the wiki.
    pub fr_page: Option<String>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Report {
    /// English names of talents whose thresholds match.
    pub ok: Vec<String>,
    pub different: Vec<ThresholdDiff>,
    /// MetalKid talents with no French page.
    pub missing_fr: Vec<String>,
    /// French list entries that match no MetalKid talent.
    pub unmatched_wiki: Vec<Unmatched>,
    /// Corrections from `manual.rs` that were applied.
    pub manual: Vec<String>,
}

/// Name comparison key: case-insensitive, typographic apostrophe = `'`.
fn name_key(name: &str) -> String {
    name.trim().replace('’', "'").to_lowercase()
}

/// Index of MetalKid talents by name key.
///
/// The map stores `&MkTalent` (borrows, not copies): it cannot outlive the
/// `mk` slice. The lifetime is elided here (one input reference), but it
/// is written out explicitly in `find` below, which has two inputs.
fn index(mk: &[MkTalent]) -> HashMap<String, &MkTalent> {
    mk.iter().map(|t| (name_key(&t.en), t)).collect()
}

/// Finds the MetalKid talent of a wiki page, if its English name is known.
fn find<'a>(by_name: &HashMap<String, &'a MkTalent>, page: &WikiTalent) -> Option<&'a MkTalent> {
    let en = page.en.as_deref()?;
    by_name.get(&name_key(en)).copied()
}

fn sorted_tiers(page: &WikiTalent) -> Vec<Tier> {
    let mut tiers = page.tiers.clone();
    tiers.sort_by_key(|t| t.points);
    tiers
}

/// Builds the output records (wiki tiers sorted by points).
pub fn to_records(mk: &[MkTalent], wiki: &[WikiTalent]) -> Vec<FrTalent> {
    let by_name = index(mk);
    wiki.iter()
        .map(|page| {
            let found = find(&by_name, page);
            let tiers = sorted_tiers(page);
            FrTalent {
                // Prefer MetalKid's spelling so the core can join on it.
                en: found.map(|t| t.en.clone()).or_else(|| page.en.clone()),
                fr: page.fr.clone(),
                fr_page: page.fr_page.clone().filter(|p| *p != page.fr),
                ja: found.map(|t| t.ja.clone()),
                thresholds: tiers.iter().map(|t| t.points).collect(),
                skills_fr: tiers.into_iter().map(|t| t.name).collect(),
                source_url: page.source_url.clone(),
            }
        })
        .collect()
}

/// Compares wiki thresholds with MetalKid's.
pub fn compare(mk: &[MkTalent], wiki: &[WikiTalent]) -> Report {
    let by_name = index(mk);
    let mut report = Report::default();
    let mut matched = HashSet::new();

    for page in wiki {
        let Some(talent) = find(&by_name, page) else {
            report.unmatched_wiki.push(Unmatched {
                fr: page.fr.clone(),
                en: page.en.clone(),
                fr_page: page.fr_page.clone(),
            });
            continue;
        };
        matched.insert(name_key(&talent.en));

        let wiki_points: Vec<u32> = sorted_tiers(page).iter().map(|t| t.points).collect();
        if wiki_points == talent.thresholds {
            report.ok.push(talent.en.clone());
        } else {
            report.different.push(ThresholdDiff {
                en: talent.en.clone(),
                fr: page.fr.clone(),
                metalkid: talent.thresholds.clone(),
                wiki: wiki_points,
            });
        }
    }

    report.missing_fr = mk
        .iter()
        .filter(|t| !matched.contains(&name_key(&t.en)))
        .map(|t| t.en.clone())
        .collect();
    report
}

/// Renders the report as Markdown (French, for the user).
pub fn render_markdown(report: &Report) -> String {
    let join = |points: &[u32]| {
        points
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut md = String::from("# Rapport d'import des talents FR\n\n");
    md += "Comparaison des paliers wikidragonquest.fr / MetalKid (`TalentSkill` + `TalentTrait`).\n\n";

    md += &format!("## Correspondances OK ({})\n\n", report.ok.len());
    for en in &report.ok {
        md += &format!("- {en}\n");
    }

    md += &format!("\n## Paliers différents ({})\n\n", report.different.len());
    if !report.different.is_empty() {
        md += "| EN | FR | MetalKid | Wiki FR |\n|---|---|---|---|\n";
        for d in &report.different {
            let wiki = if d.wiki.is_empty() {
                "(aucun palier DQM3 trouvé)".to_string()
            } else {
                join(&d.wiki)
            };
            md += &format!(
                "| {} | {} | {} | {} |\n",
                d.en,
                d.fr,
                join(&d.metalkid),
                wiki
            );
        }
    }

    md += &format!(
        "\n## Talents sans page FR ({})\n\n",
        report.missing_fr.len()
    );
    for en in &report.missing_fr {
        md += &format!("- {en}\n");
    }

    md += &format!(
        "\n## Talents FR sans équivalent EN ({})\n\n",
        report.unmatched_wiki.len()
    );
    for u in &report.unmatched_wiki {
        match (&u.fr_page, &u.en) {
            (None, _) => md += &format!("- {} (page inexistante sur le wiki)\n", u.fr),
            (Some(_), None) => md += &format!("- {} (pas de nom EN sur la page)\n", u.fr),
            (Some(_), Some(en)) => {
                md += &format!("- {} (nom EN « {en} » inconnu de MetalKid)\n", u.fr)
            }
        }
    }

    md += &format!("\n## Corrections manuelles ({})\n\n", report.manual.len());
    for note in &report.manual {
        md += &format!("- {note}\n");
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_report_lists_every_category_with_counts() {
        let report = Report {
            ok: vec!["Assassin".into()],
            different: vec![ThresholdDiff {
                en: "Pot".into(),
                fr: "Pot de glu".into(),
                metalkid: vec![2, 5],
                wiki: vec![2, 6],
            }],
            missing_fr: vec!["Slimer".into()],
            unmatched_wiki: vec![
                Unmatched {
                    fr: "Espoir de Foudre".into(),
                    en: None,
                    fr_page: Some("Apprenti de Foudre".into()),
                },
                Unmatched {
                    fr: "Tourmenteur".into(),
                    en: None,
                    fr_page: None,
                },
                Unmatched {
                    fr: "Inconnu".into(),
                    en: Some("Nobody".into()),
                    fr_page: Some("Inconnu".into()),
                },
            ],
            manual: vec!["Bon génie : palier 150 « Réflexion » ajouté (vérifié en jeu)".into()],
        };
        let md = render_markdown(&report);
        assert!(md.contains("Corrections manuelles (1)"));
        assert!(md.contains("- Bon génie : palier 150"));
        assert!(md.contains("Correspondances OK (1)"));
        assert!(md.contains("Paliers différents (1)"));
        assert!(md.contains("| Pot | Pot de glu | 2, 5 | 2, 6 |"));
        assert!(md.contains("Talents sans page FR (1)"));
        assert!(md.contains("- Slimer"));
        assert!(md.contains("Talents FR sans équivalent EN (3)"));
        assert!(md.contains("- Espoir de Foudre (pas de nom EN sur la page)"));
        assert!(md.contains("- Tourmenteur (page inexistante sur le wiki)"));
        assert!(md.contains("- Inconnu (nom EN « Nobody » inconnu de MetalKid)"));
    }

    fn page(title: &str, content: &str, en_link: Option<&str>) -> WikiPage {
        WikiPage {
            title: title.into(),
            content: content.into(),
            en_link: en_link.map(Into::into),
            url: format!("https://wiki/{}", title.replace(' ', "_")),
        }
    }

    fn list_entry(name: &str, target: &str, fragment: Option<&str>) -> ListEntry {
        ListEntry {
            name: name.into(),
            target: target.into(),
            fragment: fragment.map(Into::into),
        }
    }

    const FLAMME: &str = "'''Apprenti de Flamme''' (''Hi no Kokoro'' en japonais, \
        ''Frizz Afficionado'' en anglais).\n\
        === Dragon Quest Monsters 3 ===\n{|\n|-\n|3||[[Flamme]]||''x''\n|}\n\
        === Dragon Quest Monsters 4 ===\n{|\n|-\n|9||[[Autre]]||''x''\n|}\n";

    #[test]
    fn talent_takes_list_name_and_page_title() {
        let entry = list_entry("Espoir de Flamme", "Espoir de Flamme", None);
        let talent = build_talent(
            &entry,
            Some(&page(
                "Apprenti de Flamme",
                FLAMME,
                Some("Frizz Afficionado"),
            )),
        );
        assert_eq!(talent.fr, "Espoir de Flamme");
        assert_eq!(talent.fr_page.as_deref(), Some("Apprenti de Flamme"));
        assert_eq!(talent.en.as_deref(), Some("Frizz Afficionado"));
        assert_eq!(
            talent.tiers,
            vec![Tier {
                points: 3,
                name: "Flamme".into()
            }]
        );
        assert_eq!(talent.source_url, "https://wiki/Apprenti_de_Flamme");
    }

    #[test]
    fn english_falls_back_to_intro_and_drops_disambiguation() {
        let entry = list_entry("Espoir de Flamme", "Espoir de Flamme", None);
        let from_intro = build_talent(&entry, Some(&page("Apprenti de Flamme", FLAMME, None)));
        assert_eq!(from_intro.en.as_deref(), Some("Frizz Afficionado"));

        let entry = list_entry("Soigneur", "Soigneur (compétence)", None);
        let healer = build_talent(
            &entry,
            Some(&page("Soigneur (compétence)", "", Some("Healer (talent)"))),
        );
        assert_eq!(healer.en.as_deref(), Some("Healer"));
        assert_eq!(healer.fr_page.as_deref(), Some("Soigneur"));
    }

    #[test]
    fn fragment_selects_a_section_of_the_page() {
        let boss = "'''Estark''' (''Estark'' en anglais) est un boss.\n\
            == Talent ==\n{|\n|-\n|10||[[Coup]]\n|}\n== Galerie ==\n";
        let entry = list_entry("Estark", "Estark (boss)", Some("Talent"));
        let talent = build_talent(&entry, Some(&page("Estark (boss)", boss, Some("Estark"))));
        assert_eq!(
            talent.tiers,
            vec![Tier {
                points: 10,
                name: "Coup".into()
            }]
        );
        assert_eq!(talent.source_url, "https://wiki/Estark_(boss)#Talent");
    }

    #[test]
    fn missing_page_gives_empty_talent() {
        let entry = list_entry("Tourmenteur", "Tourmenteur ( )", None);
        let talent = build_talent(&entry, None);
        assert_eq!(talent.fr, "Tourmenteur");
        assert_eq!(talent.fr_page, None);
        assert_eq!(talent.en, None);
        assert!(talent.tiers.is_empty());
    }

    #[test]
    fn record_omits_page_title_equal_to_name() {
        let mut same = wiki("Assassin", Some("Assassin"), &[]);
        same.fr_page = Some("Assassin".into());
        assert_eq!(to_records(&[], &[same])[0].fr_page, None);
    }

    fn mk(en: &str, thresholds: &[u32]) -> MkTalent {
        MkTalent {
            en: en.to_string(),
            ja: format!("ja-{en}"),
            thresholds: thresholds.to_vec(),
        }
    }

    fn wiki(fr: &str, en: Option<&str>, tiers: &[(u32, &str)]) -> WikiTalent {
        WikiTalent {
            fr: fr.to_string(),
            fr_page: Some(fr.to_string()),
            en: en.map(str::to_string),
            tiers: tiers
                .iter()
                .map(|&(points, name)| Tier {
                    points,
                    name: name.to_string(),
                })
                .collect(),
            source_url: format!("https://wiki/{fr}"),
        }
    }

    #[test]
    fn matching_thresholds_are_ok() {
        let report = compare(
            &[mk("Assassin", &[3, 10])],
            &[wiki(
                "Assassin",
                Some("Assassin"),
                &[(3, "Crève-cœur"), (10, "Tempête de sable")],
            )],
        );
        assert_eq!(report.ok, vec!["Assassin"]);
        assert!(report.different.is_empty() && report.missing_fr.is_empty());
        assert!(report.unmatched_wiki.is_empty());
    }

    #[test]
    fn english_match_ignores_case_and_apostrophe_style() {
        let report = compare(
            &[mk("Hunter's Instinct", &[5])],
            &[wiki("Instinct", Some("hunter’s instinct"), &[(5, "X")])],
        );
        assert_eq!(report.ok, vec!["Hunter's Instinct"]);
    }

    #[test]
    fn wiki_row_order_does_not_matter() {
        let report = compare(
            &[mk("A", &[5, 10])],
            &[wiki("a", Some("A"), &[(10, "y"), (5, "x")])],
        );
        assert_eq!(report.ok, vec!["A"]);
    }

    #[test]
    fn different_thresholds_are_reported() {
        let report = compare(
            &[mk("Pot", &[2, 5, 15])],
            &[wiki(
                "Pot de glu",
                Some("Pot"),
                &[(2, "Flamme"), (6, "Soin")],
            )],
        );
        assert_eq!(
            report.different,
            vec![ThresholdDiff {
                en: "Pot".into(),
                fr: "Pot de glu".into(),
                metalkid: vec![2, 5, 15],
                wiki: vec![2, 6],
            }]
        );
        assert!(report.ok.is_empty());
    }

    #[test]
    fn talent_without_french_page_is_reported() {
        let report = compare(
            &[mk("Assassin", &[3]), mk("Slimer", &[1])],
            &[wiki("Assassin", Some("Assassin"), &[(3, "x")])],
        );
        assert_eq!(report.missing_fr, vec!["Slimer"]);
    }

    #[test]
    fn wiki_page_without_or_with_unknown_english_name_is_unmatched() {
        let report = compare(
            &[mk("Assassin", &[3])],
            &[
                wiki("Apprenti de Foudre", None, &[]),
                wiki("Inconnu", Some("Nobody"), &[]),
            ],
        );
        assert_eq!(
            report.unmatched_wiki,
            vec![
                Unmatched {
                    fr: "Apprenti de Foudre".into(),
                    en: None,
                    fr_page: Some("Apprenti de Foudre".into()),
                },
                Unmatched {
                    fr: "Inconnu".into(),
                    en: Some("Nobody".into()),
                    fr_page: Some("Inconnu".into()),
                },
            ]
        );
        assert_eq!(report.missing_fr, vec!["Assassin"]);
    }

    #[test]
    fn records_use_metalkid_english_and_japanese_names_and_sort_tiers() {
        let records = to_records(
            &[mk("Hunter's Instinct", &[5, 10])],
            &[wiki(
                "Instinct",
                Some("hunter’s instinct"),
                &[(10, "B"), (5, "A")],
            )],
        );
        assert_eq!(
            records,
            vec![FrTalent {
                en: Some("Hunter's Instinct".into()),
                fr: "Instinct".into(),
                fr_page: None,
                ja: Some("ja-Hunter's Instinct".into()),
                thresholds: vec![5, 10],
                skills_fr: vec!["A".into(), "B".into()],
                source_url: "https://wiki/Instinct".into(),
            }]
        );
    }

    #[test]
    fn records_keep_unmatched_pages_without_japanese_name() {
        let records = to_records(&[], &[wiki("Inconnu", Some("Nobody"), &[])]);
        assert_eq!(records[0].en.as_deref(), Some("Nobody"));
        assert_eq!(records[0].ja, None);
    }
}
