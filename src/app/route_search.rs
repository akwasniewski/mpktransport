use crate::{app::app::App, raptor::{self, Journey, Raptor}};

impl App {
    pub(super) fn draw_route_search(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.heading("🔀 Route Search");
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(12.0);

        // ── FROM ────────────────────────────────────────────────────────
        ui.label(egui::RichText::new("From").strong());
        ui.add_space(4.0);

        let from_edit = ui.add(
            egui::TextEdit::singleline(&mut self.route_from)
                .hint_text("Stop name, code or ID…")
                .desired_width(f32::INFINITY),
        );

        if from_edit.changed() {
            self.route_from_selected = None;
        }
        if from_edit.gained_focus() {
            self.route_from_focused = true;
        }
        if from_edit.lost_focus() && !ui.ctx().input(|i| i.pointer.primary_clicked()) {
            self.route_from_focused = false;
        }

        if self.route_from_focused && !self.route_from.is_empty() && self.route_from_selected.is_none() {
            let q = self.route_from.to_lowercase();
            let suggestions: Vec<(usize, String)> = self
                .graph
                .stops
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.stop_name.to_lowercase().contains(&q)
                        || s.stop_code.to_lowercase().contains(&q)
                        || s.stop_id.to_lowercase().contains(&q)
                })
                .take(6)
                .map(|(i, s)| (i, s.stop_name.clone()))
                .collect();

            if !suggestions.is_empty() {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for (idx, name) in suggestions {
                        if ui.selectable_label(false, &name).clicked() {
                            self.route_from = name;
                            self.route_from_selected = Some(idx);
                            self.route_from_focused = false;
                        }
                    }
                });
            }
        }

        // Confirmed badge
        if let Some(idx) = self.route_from_selected {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("✔  {}", self.graph.stops[idx].stop_name))
                        .small()
                        .color(egui::Color32::from_rgb(80, 180, 80)),
                );
                if ui.small_button("✕").clicked() {
                    self.route_from.clear();
                    self.route_from_selected = None;
                }
            });
        }

        ui.add_space(8.0);

        // ── Swap button ─────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 28.0);
            if ui.button("⇅").on_hover_text("Swap stops").clicked() {
                std::mem::swap(&mut self.route_from, &mut self.route_to);
                std::mem::swap(&mut self.route_from_selected, &mut self.route_to_selected);
            }
        });

        ui.add_space(8.0);

        // ── TO ──────────────────────────────────────────────────────────
        ui.label(egui::RichText::new("To").strong());
        ui.add_space(4.0);

        let to_edit = ui.add(
            egui::TextEdit::singleline(&mut self.route_to)
                .hint_text("Stop name, code or ID…")
                .desired_width(f32::INFINITY),
        );

        if to_edit.changed() {
            self.route_to_selected = None;
        }
        if to_edit.gained_focus() {
            self.route_to_focused = true;
        }
        if to_edit.lost_focus() && !ui.ctx().input(|i| i.pointer.primary_clicked()) {
            self.route_to_focused = false;
        }

        if self.route_to_focused && !self.route_to.is_empty() && self.route_to_selected.is_none() {
            let q = self.route_to.to_lowercase();
            let suggestions: Vec<(usize, String)> = self
                .graph
                .stops
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.stop_name.to_lowercase().contains(&q)
                        || s.stop_code.to_lowercase().contains(&q)
                        || s.stop_id.to_lowercase().contains(&q)
                })
                .take(6)
                .map(|(i, s)| (i, s.stop_name.clone()))
                .collect();

            if !suggestions.is_empty() {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for (idx, name) in suggestions {
                        if ui.selectable_label(false, &name).clicked() {
                            self.route_to = name;
                            self.route_to_selected = Some(idx);
                            self.route_to_focused = false;
                        }
                    }
                });
            }
        }

        // Confirmed badge
        if let Some(idx) = self.route_to_selected {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("✔  {}", self.graph.stops[idx].stop_name))
                        .small()
                        .color(egui::Color32::from_rgb(80, 180, 80)),
                );
                if ui.small_button("✕").clicked() {
                    self.route_to.clear();
                    self.route_to_selected = None;
                }
            });
        }

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(12.0);

        // ── Search button ────────────────────────────────────────────────
        let same = self.route_from_selected.is_some()
            && self.route_from_selected == self.route_to_selected;
        let can_search = self.route_from_selected.is_some()
            && self.route_to_selected.is_some()
            && !same;

        ui.add_enabled_ui(can_search, |ui| {
            if ui
                .add_sized(
                    [ui.available_width(), 32.0],
                    egui::Button::new(egui::RichText::new("🔍  Find Route").strong()),
                )
                .clicked()
                {
                    let from = &self.graph.stops[self.route_from_selected.unwrap()].stop_id;
                    let to = &self.graph.stops[self.route_to_selected.unwrap()].stop_id;

                    let mut raptor = Raptor::new(&self.graph);
                    let journey = raptor.query(from, to, 8 * 3600);
                    self.route_result = Some((self.route_from_selected.unwrap(), self.route_to_selected.unwrap(), journey));
            }
        });

        if !can_search {
            ui.add_space(4.0);
            let hint = if same {
                "Origin and destination must differ."
            } else {
                "Select both a From and a To stop."
            };
            ui.label(egui::RichText::new(hint).small().weak().italics());
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // ── Results ──────────────────────────────────────────────────────
        ui.label(egui::RichText::new("Results").strong());
        ui.add_space(6.0);

        match &self.route_result {
            None => {
                ui.label(egui::RichText::new("No route searched yet.").weak().italics());
            }
            Some((from, to, journey)) => {
                let from_id = &self.graph.stops[*from].stop_id;
                let to_id   = &self.graph.stops[*to].stop_id;
                ui.label(egui::RichText::new(format!("From: {}", from_id)).monospace());
                ui.label(egui::RichText::new(format!("To:   {}", to_id)).monospace());

                match journey {
                    Some(j) => {
                        ui.label(egui::RichText::new("Route found").weak().italics());
                        for (i, leg) in j.legs.iter().enumerate() {
                            ui.label(
                                egui::RichText::new(format!("{:>2}. {} at {}", i + 1, leg.1, leg.0))
                                    .monospace(),
                            );
                        }
                    }
                    None => {
                        ui.label(egui::RichText::new("No route found").weak().italics());
                    }
                }
            }
        }
    }
}


