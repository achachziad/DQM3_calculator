//! Small reusable pieces: images first, names second.

use crate::assets;
use crate::theme;
use dqm3_core::{Database, Monster};
use eframe::egui::{self, Color32, CornerRadius, Image, Response, RichText, Sense, Ui, vec2};

/// A card: rounded block with a title, used for every sheet section.
pub fn card<R>(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(theme::CARD)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(13.0).strong().color(theme::NAVY));
            ui.add_space(6.0);
            add_contents(ui)
        })
        .inner
}

/// A monster portrait, clickable. Family rules have no portrait: their
/// family symbol is shown instead.
pub fn portrait(ui: &mut Ui, monster: &Monster, size: f32) -> Response {
    match assets::sprite(&monster.identifier) {
        Some(bytes) => ui.add(
            Image::from_bytes(format!("bytes://sprite/{}.png", monster.identifier), bytes)
                .fit_to_exact_size(vec2(size, size))
                .corner_radius(CornerRadius::same((size / 6.0) as u8))
                .sense(Sense::click()),
        ),
        None => ui.allocate_response(vec2(size, size), Sense::click()),
    }
}

/// A 24×24-style icon (family, season, weather) with its name on hover.
pub fn icon(ui: &mut Ui, name: &str, size: f32, tooltip: &str) -> Response {
    match assets::icon(name) {
        Some(bytes) => ui
            .add(
                Image::from_bytes(format!("bytes://icon/{name}.png"), bytes)
                    .fit_to_exact_size(vec2(size, size)),
            )
            .on_hover_text(tooltip),
        None => ui.label(tooltip),
    }
}

/// A colored rank badge ("G" … "X", or "★" for the "Any" wildcard).
/// Drawn by hand at a fixed size, so that it never stretches with the row.
pub fn rank_badge(ui: &mut Ui, rank: &str) -> Response {
    let text = if rank == "Any" { "★" } else { rank };
    let (rect, response) = ui.allocate_exact_size(vec2(22.0, 18.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(5), theme::rank_color(rank));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        Color32::WHITE,
    );
    response.on_hover_text(if rank == "Any" {
        "Tout rang".to_string()
    } else {
        format!("Rang {rank}")
    })
}

/// French name, or English when there is none.
pub fn name_fr(monster: &Monster) -> &str {
    monster.name_fr.as_deref().unwrap_or(&monster.name_en)
}

/// The family symbol of a monster.
pub fn family_icon(ui: &mut Ui, db: &Database, monster: &Monster, size: f32) {
    if let Some(family) = db.family(monster.family) {
        let name = family.name_fr.as_deref().unwrap_or(&family.name_en);
        icon(ui, &family.identifier, size, name);
    }
}

/// A recipe ingredient or result: portrait with its name below, or, for a
/// "family + rank" rule, the family symbol with a rank badge.
/// Returns `true` when clicked (to open that monster's sheet).
pub fn monster_chip(ui: &mut Ui, db: &Database, monster: &Monster) -> bool {
    let mut clicked = false;
    ui.vertical(|ui| {
        ui.set_width(72.0);
        if monster.is_family_rule() {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                family_icon(ui, db, monster, 32.0);
                if let Some(rank) = db.rank(monster.rank) {
                    rank_badge(ui, &rank.name);
                }
            });
            let family = db.family(monster.family);
            let family_name = family
                .map(|f| f.name_fr.as_deref().unwrap_or(&f.name_en))
                .unwrap_or("?");
            ui.label(
                RichText::new(format!("Tout {family_name}"))
                    .size(11.0)
                    .color(theme::MUTED),
            );
        } else {
            let response = portrait(ui, monster, 56.0).on_hover_text(format!(
                "{} ({})",
                name_fr(monster),
                monster.name_en
            ));
            clicked = response.clicked();
            ui.label(RichText::new(name_fr(monster)).size(11.0));
        }
    });
    clicked
}
