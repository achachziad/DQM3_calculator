//! One-shot, manual importer of French talent names from wikidragonquest.fr.
//!
//! Run by hand only: `cargo run -p dqm3-import`. Never called by the app.
//! Outputs (git-ignored, wiki license unknown):
//! - `data-fr/talents.json`: French talents with tiers;
//! - `data-fr/report.md`: comparison with MetalKid's thresholds;
//! - `data-fr/cache/`: raw API responses (delete it to re-download).

mod api;
mod compare;
mod manual;
mod metalkid;
mod wikitext;

use std::fs;
use std::path::{Path, PathBuf};

const LIST_PAGE: &str = "Liste des talents de Dragon Quest Monsters 3";

/// `main` may return a `Result`: on `Err`, Rust prints the error and exits
/// with a non-zero code. `Box<dyn Error>` accepts any error type, which is
/// fine for a small tool (the core library will use typed errors).
fn main() -> Result<(), api::Error> {
    let root = workspace_root();
    let raw = root.join("data/data/json/raw");
    let out = root.join("data-fr");

    println!("Reading MetalKid talents from {}", raw.display());
    let mk = metalkid::parse(
        &fs::read_to_string(raw.join("Talent.json"))?,
        &fs::read_to_string(raw.join("TalentSkill.json"))?,
        &fs::read_to_string(raw.join("TalentTrait.json"))?,
    )?;

    let mut client = api::Client::new(out.join("cache"))?;

    println!("Fetching \"{LIST_PAGE}\"");
    let list_title = [LIST_PAGE.to_string()];
    let list_set = client.fetch_pages(&list_title)?;
    let list = list_set
        .resolve(LIST_PAGE)
        .ok_or("talent list page not found on the wiki")?;
    let entries = wikitext::parse_list(&list.content);
    println!("{} talents in the list", entries.len());

    // Each linked page once, even if several entries point to it.
    let mut targets: Vec<String> = entries.iter().map(|e| e.target.clone()).collect();
    targets.sort();
    targets.dedup();
    println!(
        "Fetching {} linked pages (redirects followed)",
        targets.len()
    );
    let pages = client.fetch_pages(&targets)?;

    let mut manual_notes = Vec::new();
    let wiki: Vec<compare::WikiTalent> = entries
        .iter()
        .map(|e| {
            let mut talent = compare::build_talent(e, pages.resolve(&e.target));
            // `&mut talent`: lend it mutably to `apply`, then get it back.
            manual_notes.extend(manual::apply(&e.target, &mut talent));
            talent
        })
        .collect();

    let records = compare::to_records(&mk, &wiki);
    fs::write(
        out.join("talents.json"),
        serde_json::to_string_pretty(&records)?,
    )?;

    let mut report = compare::compare(&mk, &wiki);
    report.manual = manual_notes;
    fs::write(out.join("report.md"), compare::render_markdown(&report))?;

    println!(
        "Done: {} FR talents, {} OK, {} different, {} MetalKid talents without FR match, {} unmatched FR talents.",
        wiki.len(),
        report.ok.len(),
        report.different.len(),
        report.missing_fr.len(),
        report.unmatched_wiki.len()
    );
    println!("See {}", out.join("report.md").display());
    Ok(())
}

/// `crates/dqm3-import` -> workspace root (resolved at compile time).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
