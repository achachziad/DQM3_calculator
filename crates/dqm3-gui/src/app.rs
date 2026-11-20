//! The application: monster list (left) and monster sheet (right).
//! Display and user input only; every query goes through `dqm3_core`.

use crate::widgets::{self, card, monster_chip, name_fr};
use crate::{assets, theme};
use dqm3_core::{
    CoreError, Database, FamilyId, Monster, MonsterId, MonsterQuery, Parents, RankId, Reward,
    SizeId, SortKey, Stat, TalentId,
};
use eframe::egui::{self, Align, CornerRadius, Image, Layout, RichText, ScrollArea, vec2};

/// Sort choices shown in the list header.
const SORTS: [(SortKey, &str); 9] = [
    (SortKey::Number, "N°"),
    (SortKey::Rank, "Rang"),
    (SortKey::Name, "Nom"),
    (SortKey::Stat(Stat::Hp), "PV"),
    (SortKey::Stat(Stat::Mp), "PM"),
    (SortKey::Stat(Stat::Att), "Attaque"),
    (SortKey::Stat(Stat::Def), "Défense"),
    (SortKey::Stat(Stat::Agi), "Agilité"),
    (SortKey::Stat(Stat::Wis), "Sagesse"),
];

const ROW_HEIGHT: f32 = 46.0;

pub struct App {
    db: Database,
    query: MonsterQuery,
    /// Results of `query`, recomputed only when the query changes.
    results: Vec<MonsterId>,
    results_for: Option<MonsterQuery>,
    selected: Option<MonsterId>,
    /// Previously opened sheets, for the "back" button.
    history: Vec<MonsterId>,
    /// Talents sorted by name, for the filter menu.
    talents: Vec<(TalentId, String)>,
    /// Highest value of each stat (HP, MP, Att, Def, Agi, Wis), to scale bars.
    stat_max: [u32; 6],
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, CoreError> {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx);

        let db = Database::load()?;
        let mut talents: Vec<(TalentId, String)> =
            db.talents().map(|t| (t.id, t.name_en.clone())).collect();
        talents.sort_by(|a, b| a.1.cmp(&b.1));

        let mut stat_max = [1; 6];
        for stats in db.monsters().iter().filter_map(|m| m.max_stats) {
            let values = [
                stats.hp, stats.mp, stats.att, stats.def, stats.agi, stats.wis,
            ];
            for (max, value) in stat_max.iter_mut().zip(values) {
                *max = (*max).max(value);
            }
        }

        let selected = db.monster_by_number(1).map(|m| m.id);
        Ok(Self {
            db,
            query: MonsterQuery::default(),
            results: Vec::new(),
            results_for: None,
            selected,
            history: Vec::new(),
            talents,
            stat_max,
        })
    }

    fn refresh_results(&mut self) {
        if self.results_for.as_ref() != Some(&self.query) {
            self.results = self
                .db
                .search_monsters(&self.query)
                .iter()
                .map(|m| m.id)
                .collect();
            self.results_for = Some(self.query.clone());
        }
    }

    /// Opens a monster sheet, remembering the current one for "back".
    fn open(&mut self, id: MonsterId) {
        if self.selected != Some(id) {
            if let Some(current) = self.selected {
                self.history.push(current);
            }
            self.selected = Some(id);
        }
    }

    // ---------- Left panel: search, filters, list ----------

    fn list_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Monstres")
                    .size(22.0)
                    .strong()
                    .color(theme::NAVY),
            );
            ui.label(RichText::new(self.results.len().to_string()).color(theme::MUTED));
        });
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.query.text)
                .hint_text("🔍  Rechercher (FR / EN)")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(4.0);
        self.family_filter(ui);
        self.rank_filter(ui);
        self.talent_and_sort(ui);
        ui.add_space(6.0);
        ui.separator();

        let mut clicked = None;
        ScrollArea::vertical()
            .id_salt("monster_list")
            .auto_shrink([false, false])
            .show_rows(ui, ROW_HEIGHT, self.results.len(), |ui, range| {
                for &id in &self.results[range] {
                    if let Some(monster) = self.db.monster(id)
                        && list_row(ui, &self.db, monster, self.selected == Some(id))
                    {
                        clicked = Some(id);
                    }
                }
            });
        if let Some(id) = clicked {
            self.open(id);
        }
    }

    fn family_filter(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            // Tighter buttons, only here, so that the 8 symbols fit on one row.
            ui.spacing_mut().button_padding = vec2(5.0, 4.0);
            ui.spacing_mut().item_spacing.x = 5.0;
            for id in 1..=8 {
                let Some(family) = self.db.family(FamilyId(id)) else {
                    continue;
                };
                let Some(bytes) = assets::icon(&family.identifier) else {
                    continue;
                };
                let selected = self.query.family == Some(family.id);
                let image =
                    Image::from_bytes(format!("bytes://icon/{}.png", family.identifier), bytes)
                        .fit_to_exact_size(vec2(22.0, 22.0));
                let name = family.name_fr.as_deref().unwrap_or(&family.name_en);
                if ui
                    .add(egui::Button::image(image).selected(selected))
                    .on_hover_text(name)
                    .clicked()
                {
                    self.query.family = if selected { None } else { Some(family.id) };
                }
            }
        });
    }

    fn rank_filter(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for id in 1..=9 {
                let Some(rank) = self.db.rank(RankId(id)) else {
                    continue;
                };
                let selected = self.query.rank == Some(rank.id);
                let text = RichText::new(&rank.name)
                    .strong()
                    .color(theme::rank_color(&rank.name));
                if ui
                    .selectable_label(selected, text)
                    .on_hover_text(format!("Rang {}", rank.name))
                    .clicked()
                {
                    self.query.rank = if selected { None } else { Some(rank.id) };
                }
            }
        });
    }

    fn talent_and_sort(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let current = self
                .query
                .talent
                .and_then(|t| self.talents.iter().find(|(id, _)| *id == t))
                .map_or("Tous les talents", |(_, name)| name.as_str());
            egui::ComboBox::from_id_salt("talent_filter")
                .width(170.0)
                .selected_text(current)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.query.talent, None, "Tous les talents");
                    for (id, name) in &self.talents {
                        ui.selectable_value(&mut self.query.talent, Some(*id), name.as_str());
                    }
                });

            let sort_name = SORTS
                .iter()
                .find(|(key, _)| *key == self.query.sort)
                .map_or("N°", |(_, name)| *name);
            egui::ComboBox::from_id_salt("sort")
                .width(80.0)
                .selected_text(sort_name)
                .show_ui(ui, |ui| {
                    for (key, name) in SORTS {
                        ui.selectable_value(&mut self.query.sort, key, name);
                    }
                });
            let arrow = if self.query.descending { "⬇" } else { "⬆" };
            if ui.button(arrow).on_hover_text("Inverser l'ordre").clicked() {
                self.query.descending = !self.query.descending;
            }
            let filtered = self.query.family.is_some()
                || self.query.rank.is_some()
                || self.query.talent.is_some()
                || !self.query.text.is_empty();
            if filtered
                && ui
                    .button("✖")
                    .on_hover_text("Effacer les filtres")
                    .clicked()
            {
                self.query = MonsterQuery {
                    sort: self.query.sort,
                    descending: self.query.descending,
                    ..MonsterQuery::default()
                };
            }
        });
    }

    // ---------- Right panel: monster sheet ----------

    fn sheet_panel(&mut self, ui: &mut egui::Ui) {
        let Some(monster) = self.selected.and_then(|id| self.db.monster(id)) else {
            ui.centered_and_justified(|ui| ui.label("Choisis un monstre dans la liste"));
            return;
        };

        // Actions are collected while drawing (the database is borrowed),
        // then applied once drawing is done.
        let mut open = None;
        let mut back = false;
        let mut talent_filter = None;

        ScrollArea::vertical()
            .id_salt("monster_sheet")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(10.0);
                back = self.header(ui, monster);
                ui.add_space(10.0);
                self.stats_card(ui, monster);
                ui.add_space(8.0);
                talent_filter = self.talents_card(ui, monster);
                ui.add_space(8.0);
                self.traits_card(ui, monster);
                ui.add_space(8.0);
                self.recruitment_card(ui, monster);
                ui.add_space(8.0);
                open = self.recipes_card(ui, monster).or(open);
                ui.add_space(8.0);
                open = self.children_card(ui, monster).or(open);
                ui.add_space(16.0);
            });

        if back {
            self.selected = self.history.pop().or(self.selected);
        }
        if let Some(id) = open {
            self.open(id);
        }
        if let Some(talent) = talent_filter {
            self.query.talent = Some(talent);
        }
    }

    /// Portrait, names, family, rank, number. Returns `true` on "back".
    fn header(&self, ui: &mut egui::Ui, monster: &Monster) -> bool {
        let mut back = false;
        ui.horizontal(|ui| {
            if !self.history.is_empty() && ui.button("⬅").on_hover_text("Retour").clicked() {
                back = true;
            }
            widgets::portrait(ui, monster, 104.0);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(name_fr(monster))
                        .size(28.0)
                        .strong()
                        .color(theme::NAVY),
                );
                if monster.name_fr.is_some() {
                    ui.label(
                        RichText::new(&monster.name_en)
                            .size(15.0)
                            .color(theme::MUTED),
                    );
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    widgets::family_icon(ui, &self.db, monster, 24.0);
                    if let Some(rank) = self.db.rank(monster.rank) {
                        widgets::rank_badge(ui, &rank.name);
                    }
                    if let Some(number) = monster.number {
                        ui.label(RichText::new(format!("n° {number}")).color(theme::MUTED));
                    }
                });
            });
        });
        back
    }

    fn stats_card(&self, ui: &mut egui::Ui, monster: &Monster) {
        let Some(stats) = monster.max_stats else {
            return;
        };
        card(ui, "STATISTIQUES MAX", |ui| {
            let values = [
                stats.hp, stats.mp, stats.att, stats.def, stats.agi, stats.wis,
            ];
            let labels = ["PV", "PM", "Attaque", "Défense", "Agilité", "Sagesse"];
            egui::Grid::new("stats")
                .num_columns(3)
                .spacing(vec2(12.0, 6.0))
                .show(ui, |ui| {
                    for ((label, value), max) in labels.iter().zip(values).zip(self.stat_max) {
                        ui.label(RichText::new(*label).color(theme::MUTED));
                        stat_bar(ui, value as f32 / max as f32);
                        ui.label(RichText::new(value.to_string()).strong());
                        ui.end_row();
                    }
                });
        });
    }

    /// Talent chips; a click filters the list on that talent.
    fn talents_card(&self, ui: &mut egui::Ui, monster: &Monster) -> Option<TalentId> {
        let mut clicked = None;
        card(ui, "TALENTS", |ui| {
            ui.horizontal_wrapped(|ui| {
                for mt in self.db.monster_talents(monster.id) {
                    let Some(talent) = self.db.talent(mt.talent) else {
                        continue;
                    };
                    let text = if mt.fixed {
                        RichText::new(format!("★ {}", talent.name_en)).strong()
                    } else {
                        RichText::new(&talent.name_en)
                    };
                    let mut button = egui::Button::new(text).corner_radius(CornerRadius::same(12));
                    if mt.fixed {
                        button = button.fill(theme::SELECTED);
                    }
                    let tiers = self.talent_tiers_text(mt.talent);
                    let hint = if mt.fixed {
                        "Talent fixe"
                    } else {
                        "Talent aléatoire"
                    };
                    let response = ui.add(button).on_hover_text(format!(
                        "{hint}\n\n{tiers}\n\nClic : monstres ayant ce talent"
                    ));
                    if response.clicked() {
                        clicked = Some(mt.talent);
                    }
                }
            });
        });
        clicked
    }

    /// "3 pts — Heart Breaker" lines for a talent tooltip.
    fn talent_tiers_text(&self, id: TalentId) -> String {
        let Some(talent) = self.db.talent(id) else {
            return String::new();
        };
        talent
            .tiers
            .iter()
            .map(|tier| {
                let name = match tier.reward {
                    Reward::Skill(s) => self.db.skill(s).map(|s| s.name_en.as_str()),
                    Reward::Trait(t) => self.db.trait_by_id(t).map(|t| t.name_en.as_str()),
                };
                format!("{:>3} pts — {}", tier.points, name.unwrap_or("?"))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn traits_card(&self, ui: &mut egui::Ui, monster: &Monster) {
        let traits = self.db.monster_traits(monster.id);
        if traits.is_empty() {
            return;
        }
        card(ui, "TRAITS", |ui| {
            ui.columns(2, |columns| {
                for (column, (size, title)) in columns
                    .iter_mut()
                    .zip([(SizeId(1), "Taille normale"), (SizeId(2), "Grande taille")])
                {
                    column.label(RichText::new(title).strong());
                    let mut rows: Vec<_> = traits.iter().filter(|t| t.size == size).collect();
                    rows.sort_by_key(|t| t.unlock_level);
                    for t in rows {
                        let Some(info) = self.db.trait_by_id(t.trait_id) else {
                            continue;
                        };
                        column.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Niv. {}", t.unlock_level))
                                    .color(theme::MUTED),
                            );
                            ui.label(&info.name_en).on_hover_text(&info.description_en);
                        });
                    }
                }
            });
        });
    }

    fn recruitment_card(&self, ui: &mut egui::Ui, monster: &Monster) {
        card(ui, "OÙ LE RECRUTER", |ui| {
            let places = self.db.recruitment_places(monster.id);
            if places.is_empty() {
                ui.label(
                    RichText::new("Ne se recrute pas à l'état sauvage (synthèse ou œuf).")
                        .color(theme::MUTED),
                );
            }
            for place in places {
                ui.horizontal(|ui| {
                    for season in place.seasons.iter().filter_map(|&s| self.db.season(s)) {
                        widgets::icon(ui, &season.name_en.to_lowercase(), 18.0, &season.name_en);
                    }
                    ui.add_space(4.0);
                    for weather in place.weathers.iter().filter_map(|&w| self.db.weather(w)) {
                        widgets::icon(ui, &weather.name_en.to_lowercase(), 18.0, &weather.name_en);
                    }
                    ui.add_space(6.0);
                    let location = self
                        .db
                        .location(place.location)
                        .map_or("?", |l| l.name_en.as_str());
                    ui.label(location);
                    if place.mini_boss {
                        ui.label(
                            RichText::new("mini-boss")
                                .size(11.0)
                                .color(theme::rank_color("A")),
                        );
                    }
                });
            }
        });
    }

    /// Recipes to obtain the monster. Returns a clicked monster, if any.
    fn recipes_card(&self, ui: &mut egui::Ui, monster: &Monster) -> Option<MonsterId> {
        let mut clicked = None;
        let recipes = self.db.recipes_for(monster.id);
        card(ui, "COMMENT L'OBTENIR", |ui| {
            if recipes.is_empty() {
                ui.label(
                    RichText::new("Aucune recette : se recrute ou s'obtient dans un œuf.")
                        .color(theme::MUTED),
                );
            }
            for recipe in recipes {
                ui.horizontal_wrapped(|ui| {
                    clicked = self.parents_row(ui, &recipe.parents).or(clicked);
                });
                ui.separator();
            }
        });
        clicked
    }

    /// Reverse synthesis. Returns a clicked monster, if any.
    fn children_card(&self, ui: &mut egui::Ui, monster: &Monster) -> Option<MonsterId> {
        let mut clicked = None;
        let children = self.db.children_of(monster.id);
        let title = format!("CE QU'IL PERMET DE CRÉER ({})", children.len());
        card(ui, &title, |ui| {
            if children.is_empty() {
                ui.label(RichText::new("Ne sert dans aucune recette connue.").color(theme::MUTED));
            }
            for recipe in children {
                ui.horizontal_wrapped(|ui| {
                    clicked = self.parents_row(ui, &recipe.parents).or(clicked);
                    ui.label(RichText::new("➡").size(24.0).color(theme::NAVY));
                    if let Some(result) = self.db.monster(recipe.result)
                        && monster_chip(ui, &self.db, result)
                    {
                        clicked = Some(result.id);
                    }
                });
                ui.separator();
            }
        });
        clicked
    }

    /// "A + B", or "(A + B) + (C + D)" for grandparent recipes.
    fn parents_row(&self, ui: &mut egui::Ui, parents: &Parents) -> Option<MonsterId> {
        let mut clicked = None;
        let mut chip = |ui: &mut egui::Ui, id: MonsterId| {
            if let Some(m) = self.db.monster(id)
                && monster_chip(ui, &self.db, m)
            {
                clicked = Some(id);
            }
        };
        let plus = |ui: &mut egui::Ui| {
            ui.label(RichText::new("+").size(22.0).color(theme::MUTED));
        };
        match *parents {
            Parents::Pair([a, b]) => {
                chip(ui, a);
                plus(ui);
                chip(ui, b);
            }
            Parents::Grandparents([a, b, c, d]) => {
                ui.label(RichText::new("(").size(26.0).color(theme::MUTED));
                chip(ui, a);
                plus(ui);
                chip(ui, b);
                ui.label(RichText::new(")").size(26.0).color(theme::MUTED));
                plus(ui);
                ui.label(RichText::new("(").size(26.0).color(theme::MUTED));
                chip(ui, c);
                plus(ui);
                chip(ui, d);
                ui.label(RichText::new(")").size(26.0).color(theme::MUTED));
            }
        }
        clicked
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.refresh_results();
        egui::Panel::left("list_panel")
            .resizable(true)
            .default_size(390.0)
            .min_size(320.0)
            .show(ui, |ui| {
                self.list_panel(ui);
            });
        egui::CentralPanel::default().show(ui, |ui| {
            self.sheet_panel(ui);
            ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
                ui.label(
                    RichText::new("Données : MetalKid / DQM3_Database (MIT)")
                        .size(10.0)
                        .color(theme::MUTED),
                );
            });
        });
    }
}

/// One row of the monster list: portrait, names, family, rank, number.
/// Returns `true` when clicked.
fn list_row(ui: &mut egui::Ui, db: &Database, monster: &Monster, selected: bool) -> bool {
    let fill = if selected {
        theme::SELECTED
    } else {
        egui::Color32::TRANSPARENT
    };
    let response = egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                widgets::portrait(ui, monster, 38.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(name_fr(monster)).size(14.0).strong());
                    if monster.name_fr.is_some() {
                        ui.label(
                            RichText::new(&monster.name_en)
                                .size(11.0)
                                .color(theme::MUTED),
                        );
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(number) = monster.number {
                        ui.label(
                            RichText::new(number.to_string())
                                .size(11.0)
                                .color(theme::MUTED),
                        );
                    }
                    if let Some(rank) = db.rank(monster.rank) {
                        widgets::rank_badge(ui, &rank.name);
                    }
                    widgets::family_icon(ui, db, monster, 20.0);
                });
            });
        })
        .response
        .interact(egui::Sense::click());
    response.clicked()
}

/// A thin horizontal bar filled to `fraction` (0..=1).
fn stat_bar(ui: &mut egui::Ui, fraction: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(220.0, 8.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(4), theme::BORDER);
    let mut filled = rect;
    filled.set_width(rect.width() * fraction.clamp(0.0, 1.0));
    painter.rect_filled(filled, CornerRadius::same(4), theme::NAVY);
}
