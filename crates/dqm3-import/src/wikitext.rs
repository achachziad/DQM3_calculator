//! Pure parsing of wikidragonquest.fr talent pages (no I/O).
//!
//! A talent page may describe several games. Each game has its own
//! `=== Dragon Quest Monsters 3 ===` style heading followed by a table:
//!
//! ```text
//! {| class="standard ccompetence"
//! !Points
//! !Nom
//! !Description
//! |-
//! |<center>3</center>||<center>[[Crève-cœur]]</center>||''Description''
//! |}
//! ```

/// One row of a talent table: the points needed and what it unlocks
/// (a skill or a trait, French name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tier {
    pub points: u32,
    pub name: String,
}

const DQM3_HEADING: &str = "Dragon Quest Monsters 3";

/// If `line` is a heading like `=== Title ===`, returns `(level, title)`.
fn heading(line: &str) -> Option<(usize, &str)> {
    let line = line.trim();
    if !line.starts_with('=') || !line.ends_with('=') {
        return None;
    }
    let level = line.chars().take_while(|&c| c == '=').count();
    let title = line.trim_matches('=').trim();
    Some((level, title))
}

/// Returns the body of the "Dragon Quest Monsters 3" section, if any.
///
/// The returned `&str` borrows from `content` (no copy): its lifetime is
/// tied to `content` thanks to lifetime elision (one input reference ->
/// the output reference lives as long as that input).
pub fn dqm3_section(content: &str) -> Option<&str> {
    section(content, DQM3_HEADING)
}

/// One row of "Liste des talents de Dragon Quest Monsters 3".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEntry {
    /// In-game French name (the link label).
    pub name: String,
    /// Linked page title (may be a redirect).
    pub target: String,
    /// Section of the page holding the talent (`[[Estark (boss)#Talent]]`).
    pub fragment: Option<String>,
}

/// Parses the talent list page: one `Type || [[Target|Name]]` row per talent.
pub fn parse_list(content: &str) -> Vec<ListEntry> {
    table_rows(content)
        .into_iter()
        .filter_map(|cells| {
            let cell = cells.get(1)?;
            let open = cell.find("[[")? + 2;
            let close = open + cell[open..].find("]]")?;
            let link = &cell[open..close];

            let (page, label) = match link.split_once('|') {
                Some((page, label)) => (page, Some(label)),
                None => (link, None),
            };
            let (target, fragment) = match page.split_once('#') {
                Some((target, fragment)) => (target, Some(fragment.trim().to_string())),
                None => (page, None),
            };
            Some(ListEntry {
                name: label.unwrap_or(target).trim().to_string(),
                target: target.trim().to_string(),
                fragment,
            })
        })
        .collect()
}

/// English name from the intro sentence ("…, ''Frizz Afficionado'' en
/// anglais"), used when the page has no `en` interlanguage link.
pub fn english_from_intro(content: &str) -> Option<String> {
    let end = content.find("'' en anglais")?;
    let before = &content[..end];
    let start = before.rfind("''")? + 2;
    let name = clean_cell(&before[start..]);
    (!name.is_empty()).then_some(name)
}

/// Returns the body of the section titled `title`, if any.
///
/// Two input references: elision cannot tell which one the result borrows
/// from, so `'a` says it explicitly: the result lives as long as `content`
/// (`title` may be dropped right after the call).
pub fn section<'a>(content: &'a str, title: &str) -> Option<&'a str> {
    let mut offset = 0;
    let mut start: Option<(usize, usize)> = None; // (byte offset, heading level)

    for line in content.split_inclusive('\n') {
        if let Some((level, heading_title)) = heading(line) {
            match start {
                // A heading of the same or a higher level closes the section.
                Some((begin, own_level)) if level <= own_level => {
                    return Some(&content[begin..offset]);
                }
                None if heading_title == title => start = Some((offset + line.len(), level)),
                _ => {}
            }
        }
        offset += line.len();
    }
    start.map(|(begin, _)| &content[begin..])
}

/// Turns a raw cell into plain text: drops `<center>` tags and resolves
/// links (`[[Target]]` -> "Target", `[[Target|Label]]` -> "Label").
fn clean_cell(cell: &str) -> String {
    let mut text = cell.replace("<center>", "").replace("</center>", "");
    while let (Some(open), Some(close)) = (text.find("[["), text.find("]]")) {
        if close < open {
            break;
        }
        let inner = &text[open + 2..close];
        let shown = inner.rsplit('|').next().unwrap_or(inner).to_string();
        text.replace_range(open..close + 2, &shown);
    }
    text.trim().to_string()
}

/// Drops cell attributes: `align=center | 3` -> ` 3`. The separator is the
/// first `|` outside a `[[link|label]]`; without one, the cell is returned
/// as is.
fn cell_value(cell: &str) -> &str {
    let bytes = cell.as_bytes();
    let mut depth = 0usize; // how many `[[` are open
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'[' if bytes.get(i + 1) == Some(&b'[') => {
                depth += 1;
                i += 1;
            }
            b']' if bytes.get(i + 1) == Some(&b']') => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            // `|` is ASCII, so `i + 1` is a valid UTF-8 boundary.
            b'|' if depth == 0 => return &cell[i + 1..],
            _ => {}
        }
        i += 1;
    }
    cell
}

/// Parses the rows of the first table found in `section`.
///
/// Rows are separated by `|-`; cells start with `|` and can be on one
/// line (`|a||b||c`) or one per line. Header cells (`!`) are ignored, and
/// so is any row whose first cell is not a number.
pub fn parse_tiers(section: &str) -> Vec<Tier> {
    table_rows(section)
        .into_iter()
        .filter_map(|cells| {
            let points = clean_cell(cells.first()?).parse().ok()?;
            let name = clean_cell(cells.get(1)?);
            Some(Tier { points, name })
        })
        .collect()
}

/// Raw cells of each row of the first table in `text` (header rows give
/// an empty list). The `&str` cells borrow from `text`.
fn table_rows(text: &str) -> Vec<Vec<&str>> {
    let Some(table_start) = text.find("{|") else {
        return Vec::new();
    };
    let table = &text[table_start..];
    let table = &table[..table.find("\n|}").unwrap_or(table.len())];

    table
        .split("\n|-")
        .map(|row| {
            row.lines()
                .filter(|l| l.starts_with('|') && !l.starts_with("|-") && !l.starts_with("|}"))
                .flat_map(|l| l[1..].split("||"))
                .map(cell_value)
                .collect()
        })
        .collect()
}

/// Removes a disambiguation suffix: "Soigneur (compétence)" -> "Soigneur".
pub fn clean_title(title: &str) -> String {
    match title.rfind(" (") {
        Some(cut) if title.ends_with(')') => title[..cut].to_string(),
        _ => title.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTI_GAME: &str = "\
'''Pot de glu''' est un [[talent]].

== Apparitions ==
=== Dragon Quest Monsters 4 ===
{| class=\"standard ccompetence\"
!Points
!Nom
|-
|2||[[Flamme]]||''??''
|}

===Dragon Quest Monsters 3===
Texte d'introduction.

{| class=\"standard ccompetence\"
!Points
!Nom
!Description
|-
|<center>2</center>||<center>[[Flamme]]</center>||''Roussit un ennemi.''
|-
|5||[[Premiers secours|1ers secours]] || ''Restaure 30 PV.''
|-
|<center>10</center>||<center>PV max. +4</center>||''Augmente les PV max. de 4.''
|}

=== Dragon Quest Monsters: Joker 2 ===
{| class=\"standard ccompetence\"
|-
|99||[[Autre]]||''x''
|}

[[Catégorie:Talent de Dragon Quest Monsters 3]]
";

    fn tier(points: u32, name: &str) -> Tier {
        Tier {
            points,
            name: name.to_string(),
        }
    }

    #[test]
    fn section_is_found_with_compact_heading() {
        let section = dqm3_section(MULTI_GAME).expect("section exists");
        assert!(section.contains("Roussit"));
    }

    #[test]
    fn section_stops_at_next_game_and_ignores_previous_one() {
        let section = dqm3_section(MULTI_GAME).expect("section exists");
        assert!(!section.contains("Joker"), "must stop before the next game");
        assert!(!section.contains("''??''"), "must not include DQM4");
    }

    #[test]
    fn section_heading_with_spaces_is_found() {
        let content = "=== Dragon Quest Monsters 3 ===\nbody\n== Voir aussi ==\nother";
        assert_eq!(dqm3_section(content).map(str::trim), Some("body"));
    }

    #[test]
    fn section_does_not_match_other_dqm_games() {
        let content = "=== Dragon Quest Monsters: Joker 3 ===\nbody";
        assert_eq!(dqm3_section(content), None);
    }

    #[test]
    fn tiers_handle_center_tags_links_traits_and_spaces() {
        let section = dqm3_section(MULTI_GAME).expect("section exists");
        assert_eq!(
            parse_tiers(section),
            vec![
                tier(2, "Flamme"),
                tier(5, "1ers secours"),
                tier(10, "PV max. +4"),
            ]
        );
    }

    #[test]
    fn tiers_support_one_cell_per_line() {
        let section = "{|\n!Points\n|-\n|<center>3</center>\n|[[Crève-cœur]]\n|''desc''\n|}";
        assert_eq!(parse_tiers(section), vec![tier(3, "Crève-cœur")]);
    }

    #[test]
    fn tiers_support_cell_attributes() {
        let section = "{| class=\"standard ccompetence\"\n! Points\n! Nom\n|-\n\
            | align=center | 3\n\
            | align=center | [[Lame de feu (aptitude)|Lame de feu]]\n\
            | ''Inflige des dégâts.''\n|-\n\
            | align=center | 20\n\
            | align=center | PM max. +16\n\
            | ''Augmente les PM max. de 16.''\n|}";
        assert_eq!(
            parse_tiers(section),
            vec![tier(3, "Lame de feu"), tier(20, "PM max. +16")]
        );
    }

    #[test]
    fn pipe_inside_a_link_is_not_an_attribute_separator() {
        assert_eq!(cell_value("[[Soin (sort)|Soin]]"), "[[Soin (sort)|Soin]]");
        assert_eq!(cell_value(" align=center | [[A|B]]"), " [[A|B]]");
        assert_eq!(cell_value("3"), "3");
    }

    #[test]
    fn tiers_are_empty_without_table() {
        assert!(parse_tiers("Pas de tableau ici.").is_empty());
    }

    const LIST: &str = "\
Le jeu compte un total de 241 talents.

{| class=\"standard ccompetence sortable\"
!Type
!Nom
|-
|<center>Compétence</center>||<center>[[Espoir de Flamme]]</center>
|-
|<center>Compétence</center>||<center>[[Soigneur (compétence)|Soigneur]]
|-
|<center>Compétence</center>||<center>[[Estark (boss)#Talent|Estark]]</center>
|-
|<center>Bonus de caractéristique</center>||<center>[[Bonus de PV I]]</center>
|}

== Voir aussi ==
* [[Liste des attributs de Dragon Quest Monsters 3]]
";

    fn entry(name: &str, target: &str, fragment: Option<&str>) -> ListEntry {
        ListEntry {
            name: name.into(),
            target: target.into(),
            fragment: fragment.map(Into::into),
        }
    }

    #[test]
    fn list_rows_give_name_target_and_fragment() {
        assert_eq!(
            parse_list(LIST),
            vec![
                entry("Espoir de Flamme", "Espoir de Flamme", None),
                entry("Soigneur", "Soigneur (compétence)", None),
                entry("Estark", "Estark (boss)", Some("Talent")),
                entry("Bonus de PV I", "Bonus de PV I", None),
            ]
        );
    }

    #[test]
    fn list_ignores_links_outside_the_table() {
        assert!(
            parse_list(LIST)
                .iter()
                .all(|e| !e.target.starts_with("Liste"))
        );
    }

    #[test]
    fn english_name_is_read_from_intro() {
        let intro = "'''Apprenti de Flamme''' (火の心 ''Hi no Kokoro'' en japonais, \
                     ''Frizz Afficionado'' en anglais), également appelé '''Espoir de Flamme'''.";
        assert_eq!(
            english_from_intro(intro).as_deref(),
            Some("Frizz Afficionado")
        );
    }

    #[test]
    fn english_name_is_none_without_mention() {
        assert_eq!(english_from_intro("'''X''' est un [[talent]]."), None);
        assert_eq!(english_from_intro("'''X''' ('''' en anglais)"), None);
    }

    #[test]
    fn any_section_can_be_found_by_title() {
        let boss = "Intro\n== Talent ==\n{|\n|-\n|5||[[Coup]]\n|}\n== Galerie ==\nimg";
        let talent = section(boss, "Talent").expect("section exists");
        assert_eq!(parse_tiers(talent), vec![tier(5, "Coup")]);
        assert!(!talent.contains("img"));
    }

    #[test]
    fn clean_title_removes_disambiguation() {
        assert_eq!(clean_title("Soigneur (compétence)"), "Soigneur");
        assert_eq!(
            clean_title("Protection Étourdissement (talent)"),
            "Protection Étourdissement"
        );
        assert_eq!(clean_title("Bonus de PV I"), "Bonus de PV I");
    }
}
