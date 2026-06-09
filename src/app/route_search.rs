use crate::{app::App, raptor::{self, Journey, Raptor}, utils::fmt_time};

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
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("No route searched yet.").weak().italics());
                });
            }
            Some((from, to, journey)) => {
                let from_name = &self.graph.stops[*from].stop_name;
                let to_name = &self.graph.stops[*to].stop_name;

                // --- Header ---
                ui.group(|ui| {
                    egui::Grid::new("route_header_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("From:").weak());
                            ui.label(egui::RichText::new(from_name).strong());
                            ui.end_row();

                            ui.label(egui::RichText::new("To:").weak());
                            ui.label(egui::RichText::new(to_name).strong());
                            ui.end_row();
                        });
                });

                ui.add_space(8.0);

                // --- Journey Segments ---
                match journey {
                    Some(j) => {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Route Found").color(egui::Color32::from_rgb(100, 200, 100)).strong());
                            ui.separator();
                            ui.label(egui::RichText::new(format!("Arriving at {}", fmt_time(j.arrival))).weak());
                        });
                        
                        ui.add_space(5.0);
                        ui.separator();
                        ui.add_space(5.0);

                        egui::ScrollArea::vertical()
                            .id_source("journey_legs_scroll")
                            .show(ui, |ui| {
                                // We iterate through windows of 2 to get (departure_stop, arrival_stop) pairs
                                let mut legs_iter = j.legs.windows(2).enumerate().peekable();
                                
                                while let Some((idx, pair)) = legs_iter.next() {
                                    let start_leg = &pair[0];
                                    let end_leg = &pair[1];
                                    
                                    // Calculate travel duration for this specific segment
                                    let duration_secs = end_leg.time.saturating_sub(start_leg.time);
                                    let duration_mins = duration_secs / 60; 

                                    // --- Render the Sub-journey Segment Card ---
                                    ui.group(|ui| {
                                        ui.set_width(ui.available_width()); // Span full width of side panel
                                        
                                        // Line Badge & Heading info
                                        ui.horizontal(|ui| {
                                            let line_badge = egui::RichText::new(format!(" {} ", start_leg.route_name))
                                                .background_color(ui.visuals().widgets.active.bg_fill)
                                                .color(ui.visuals().widgets.active.text_color())
                                                .strong();
                                            ui.label(line_badge);
                                            ui.label(egui::RichText::new(&start_leg.route_headline).weak().italics());
                                            
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(egui::RichText::new(format!("{} min", duration_mins)).weak().small());
                                            });
                                        });
                                        
                                        ui.add_space(4.0);

                                        // Departure row
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(format!("• {}", fmt_time(start_leg.time))).monospace().weak());
                                            ui.label(&start_leg.stop_name);
                                        });

                                        // Arrival row
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(format!("• {}", fmt_time(end_leg.time))).monospace().weak());
                                            ui.label(&end_leg.stop_name);
                                        });
                                    });

                                    // --- Check and Display Changeover / Transfer Time ---
                                    if let Some((_, next_pair)) = legs_iter.peek() {
                                        let next_start_leg = &next_pair[0];
                                        
                                        // If there's time spent waiting at the current platform, or the line changes
                                        if next_start_leg.time > end_leg.time || end_leg.route_id != next_start_leg.route_id {
                                            let changeover_secs = next_start_leg.time.saturating_sub(end_leg.time);
                                            let changeover_mins = changeover_secs / 60;

                                            ui.add_space(4.0);
                                            ui.horizontal(|ui| {
                                                ui.add_space(15.0); // Indent to separate visually from cards
                                                
                                                // Visual alert color for transfer
                                                let transfer_color = egui::Color32::from_rgb(230, 140, 10); 
                                                
                                                ui.label(egui::RichText::new("🔄").color(transfer_color));
                                                
                                                if end_leg.route_id != next_start_leg.route_id {
                                                    ui.label(egui::RichText::new(format!(
                                                        "Change lines (Wait: {} min)", changeover_mins
                                                    )).color(transfer_color).small());
                                                } else {
                                                    ui.label(egui::RichText::new(format!(
                                                        "Stay on vehicle / Wait {} min", changeover_mins
                                                    )).weak().small());
                                                }
                                            });
                                            ui.add_space(4.0);
                                        }
                                    }
                                }
                            });
                    }
                    None => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("No route found").color(egui::Color32::from_rgb(200, 100, 100)));
                        });
                    }
                }
            }
        }
    }
}


