use crate::{app::{App, RoutingAlgorithm, Time}, csa::Csa, raptor::Raptor, utils::fmt_time};

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
                .hint_text("Stop name or code…")
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
            // stop_id is no longer a field on Stop; search name and code only
            let suggestions: Vec<(usize, String)> = self
                .graph
                .stations
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.name.to_lowercase().contains(&q)
                })
                .take(6)
                .map(|(i, s)| (i, s.name.clone()))
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
                    egui::RichText::new(format!("✔  {}", self.graph.stations[idx].name))
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
                .hint_text("Stop name or code…")
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
                .stations
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.name.to_lowercase().contains(&q)
                })
                .take(6)
                .map(|(i, s)| (i, s.name.clone()))
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
                    egui::RichText::new(format!("✔  {}", self.graph.stations[idx].name))
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

        // ── Time picker ──────────────────────────────────────────────────
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Depart at:").weak());
            time_picker(ui, &mut self.time.hour, &mut self.time.minute);
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(12.0);

        // ── Search button ────────────────────────────────────────────────
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Algorithm:").weak());
            ui.selectable_value(&mut self.routing_algorithm, RoutingAlgorithm::Raptor, "RAPTOR");
            ui.selectable_value(&mut self.routing_algorithm, RoutingAlgorithm::Csa, "CSA");
        });

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
                let from_idx = self.route_from_selected.unwrap();
                let to_idx = self.route_to_selected.unwrap();

                let journey = match self.routing_algorithm {
                    RoutingAlgorithm::Raptor => {
                        let mut raptor = Raptor::new(&self.graph);
                        raptor.query(from_idx, to_idx, self.time.seconds())
                    }
                    RoutingAlgorithm::Csa => {
                        let csa = Csa::new(&self.graph);
                        csa.query(from_idx, to_idx, self.time.seconds())
                    }
                };

                self.route_result = Some((from_idx, to_idx, journey));
            }
        });

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
                let from_name = &self.graph.stations[*from].name;
                let to_name   = &self.graph.stations[*to].name;

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
                                let legs = &j.legs;
                                let n = legs.len();

                                if n >= 2 {
                                    let num_segs = n - 1;

                                    // Same-stop "wait" legs are explicit entries in the data
                                    // (e.g. 08:17→08:17), not just gaps between windows, so we
                                    // classify each segment and merge neighboring Wait segments
                                    // into an adjacent Walk (or Ride) block instead of always
                                    // rendering them as their own boxes.
                                    #[derive(Clone, Copy, PartialEq)]
                                    enum SegKind { Wait, Walk, Ride }

                                    let seg_kind = |i: usize| -> SegKind {
                                        let s = &legs[i];
                                        let e = &legs[i + 1];
                                        if s.stop_name == e.stop_name {
                                            SegKind::Wait
                                        } else if s.is_walk {
                                            SegKind::Walk
                                        } else {
                                            SegKind::Ride
                                        }
                                    };
                                    let seg_mins = |i: usize| -> u32 {
                                        legs[i + 1].time.saturating_sub(legs[i].time) / 60
                                    };

                                    let mut i = 0usize;
                                    while i < num_segs {
                                        match seg_kind(i) {
                                            SegKind::Wait => {
                                                let mins = seg_mins(i);
                                                let next_is_walk = i + 1 < num_segs && seg_kind(i + 1) == SegKind::Walk;

                                                // Zero-length waits are no-ops; waits directly
                                                // adjacent to a walk get absorbed into that
                                                // walk's block instead of standing alone.
                                                if mins == 0 || next_is_walk {
                                                    i += 1;
                                                    continue;
                                                }

                                                let start_leg = &legs[i];
                                                let end_leg = &legs[i + 1];
                                                ui.group(|ui| {
                                                    ui.set_width(ui.available_width());
                                                    ui.horizontal(|ui| {
                                                        let walk_badge = egui::RichText::new("🧍 Wait ")
                                                            .background_color(ui.visuals().widgets.inactive.bg_fill)
                                                            .color(ui.visuals().widgets.active.text_color())
                                                            .strong();
                                                        ui.label(walk_badge);
                                                        ui.label(egui::RichText::new(format!("Transfer inside {}", start_leg.stop_name)).strong());
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.label(egui::RichText::new(format!("{} min", mins)).weak().small());
                                                        });
                                                    });
                                                    ui.add_space(2.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(format!("  {} → {}", fmt_time(start_leg.time), fmt_time(end_leg.time))).monospace().weak().small());
                                                    });
                                                });
                                                i += 1;
                                            }
                                            SegKind::Walk => {
                                                let walk_mins = seg_mins(i);
                                                let start_leg = &legs[i];
                                                let end_leg = &legs[i + 1];

                                                let wait_before = if i > 0 && seg_kind(i - 1) == SegKind::Wait {
                                                    let m = seg_mins(i - 1);
                                                    if m > 0 { Some(m) } else { None }
                                                } else {
                                                    None
                                                };
                                                let wait_after = if i + 1 < num_segs && seg_kind(i + 1) == SegKind::Wait {
                                                    let m = seg_mins(i + 1);
                                                    if m > 0 { Some(m) } else { None }
                                                } else {
                                                    None
                                                };

                                                ui.group(|ui| {
                                                    ui.set_width(ui.available_width());

                                                    if let Some(m) = wait_before {
                                                        ui.horizontal(|ui| {
                                                            let transfer_color = egui::Color32::from_rgb(230, 140, 10);
                                                            ui.label(egui::RichText::new("🔄").color(transfer_color));
                                                            ui.label(egui::RichText::new(format!("Wait {} min, then walk", m)).color(transfer_color).small());
                                                        });
                                                        ui.add_space(4.0);
                                                        ui.separator();
                                                        ui.add_space(4.0);
                                                    }

                                                    ui.horizontal(|ui| {
                                                        let walk_badge = egui::RichText::new(" 🚶 Walk ")
                                                            .background_color(ui.visuals().widgets.inactive.bg_fill)
                                                            .color(ui.visuals().widgets.active.text_color())
                                                            .strong();
                                                        ui.label(walk_badge);
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.label(egui::RichText::new(format!("{} min", walk_mins)).weak().small());
                                                        });
                                                    });
                                                    ui.add_space(4.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(format!("• {}", fmt_time(start_leg.time))).monospace().weak());
                                                        ui.label(&start_leg.stop_name);
                                                    });
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(format!("• {}", fmt_time(end_leg.time))).monospace().weak());
                                                        ui.label(&end_leg.stop_name);
                                                    });

                                                    if let Some(m) = wait_after {
                                                        ui.add_space(4.0);
                                                        ui.separator();
                                                        ui.add_space(2.0);
                                                        ui.horizontal(|ui| {
                                                            let transfer_color = egui::Color32::from_rgb(230, 140, 10);
                                                            ui.label(egui::RichText::new("🔄").color(transfer_color));
                                                            ui.label(egui::RichText::new(format!("Wait {} min", m)).color(transfer_color).small());
                                                        });
                                                    }
                                                });

                                                i += 1;
                                                if wait_after.is_some() {
                                                    i += 1; // skip the wait segment we just absorbed
                                                }
                                            }
                                            SegKind::Ride => {
                                                let ride_mins = seg_mins(i);
                                                let start_leg = &legs[i];
                                                let end_leg = &legs[i + 1];

                                                ui.group(|ui| {
                                                    ui.set_width(ui.available_width());
                                                    ui.horizontal(|ui| {
                                                        let line_badge = egui::RichText::new(format!(" {} ", start_leg.clone().route_name.unwrap_or_else(|| "Unknown route".to_string())))
                                                            .background_color(ui.visuals().widgets.active.bg_fill)
                                                            .color(ui.visuals().widgets.active.text_color())
                                                            .strong();
                                                        ui.label(line_badge);
                                                        ui.label(egui::RichText::new(start_leg.clone().trip_headline.unwrap_or_else(|| "Unknown direction".to_string())).weak().italics());
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.label(egui::RichText::new(format!("{} min", ride_mins)).weak().small());
                                                        });
                                                    });
                                                    ui.add_space(4.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(format!("• {}", fmt_time(start_leg.time))).monospace().weak());
                                                        ui.label(&start_leg.stop_name);
                                                    });
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(format!("• {}", fmt_time(end_leg.time))).monospace().weak());
                                                        ui.label(&end_leg.stop_name);
                                                    });
                                                });

                                                let wait_after = if i + 1 < num_segs && seg_kind(i + 1) == SegKind::Wait {
                                                    let m = seg_mins(i + 1);
                                                    if m > 0 { Some(m) } else { None }
                                                } else {
                                                    None
                                                };

                                                // Riding then waiting stays a separate line: you
                                                // got off the vehicle, then you're standing
                                                // around — a distinct event from the ride itself.
                                                if let Some(m) = wait_after {
                                                    ui.add_space(4.0);
                                                    ui.horizontal(|ui| {
                                                        ui.add_space(15.0);
                                                        let transfer_color = egui::Color32::from_rgb(230, 140, 10);
                                                        ui.label(egui::RichText::new("🔄").color(transfer_color));
                                                        ui.label(egui::RichText::new(format!("Wait {} min for next vehicle", m)).color(transfer_color).small());
                                                    });
                                                    ui.add_space(4.0);
                                                }

                                                i += 1;
                                                if wait_after.is_some() {
                                                    i += 1;
                                                }
                                            }
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

    pub fn get_active_route_shapes(&self) -> Vec<Vec<(f64, f64)>> {
        let mut paths = Vec::new();

        if let Some((_, _, Some(journey))) = &self.route_result {
            for pair in journey.legs.windows(2) {
                let start_leg = &pair[0];
                let end_leg   = &pair[1];

                if let Some(trip_idx) = start_leg.trip_idx {
                    if let Some(trip) = self.graph.trips.get(trip_idx) {
                        // shape_idx replaces shape_id; index into self.graph.shapes
                        if let Some(shape_idx) = trip.shape_idx {
                            if let Some(shape) = self.graph.shapes.get(shape_idx) {
                                let full_shape = &shape.points;

                                let start_coord = self.graph.stops.get(start_leg.stop_idx)
                                    .and_then(|s| Some((s.stop_lat?, s.stop_lon?)));
                                let end_coord = self.graph.stops.get(end_leg.stop_idx)
                                    .and_then(|s| Some((s.stop_lat?, s.stop_lon?)));

                                match (start_coord, end_coord) {
                                    (Some(sc), Some(ec)) => {
                                        if let Some(trimmed) = trim_shape_to_stops(full_shape, sc, ec) {
                                            paths.push(trimmed);
                                        }
                                    }
                                    _ => paths.push(full_shape.clone()),
                                }
                            }
                        }
                    }
                }
            }
        }
        paths
    }
}

pub fn time_picker(ui: &mut egui::Ui, hour: &mut u32, minute: &mut u32) {
    ui.horizontal(|ui| {
        // Hour Dropdown (24-hour format)
        let hour_res = egui::ComboBox::from_id_source("hour_picker")
            .selected_text(format!("{:02}", hour))
            .width(50.0)
            .show_ui(ui, |ui| {
                for h in 0..24 {
                    ui.selectable_value(hour, h, format!("{:02}", h));
                }
            });

        ui.label(":");

        // Minute Dropdown
        let minute_res = egui::ComboBox::from_id_source("minute_picker")
            .selected_text(format!("{:02}", minute))
            .width(50.0)
            .show_ui(ui, |ui| {
                for m in 0..60 {
                    ui.selectable_value(minute, m, format!("{:02}", m));
                }
            });

        // Combine responses so the caller can check for changes
        hour_res.response.union(minute_res.response)
    });
}

fn trim_shape_to_stops(
    shape: &[(f64, f64)],
    start: (f64, f64),
    end: (f64, f64),
) -> Option<Vec<(f64, f64)>> {
    if shape.is_empty() {
        return None;
    }

    let dist_sq = |p1: (f64, f64), p2: (f64, f64)| {
        (p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2)
    };

    let (closest_start_idx, _) = shape
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            dist_sq(**a, start).total_cmp(&dist_sq(**b, start))
        })?;

    let (closest_end_idx, _) = shape
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            dist_sq(**a, end).total_cmp(&dist_sq(**b, end))
        })?;

    let (from, to) = if closest_start_idx <= closest_end_idx {
        (closest_start_idx, closest_end_idx)
    } else {
        (closest_end_idx, closest_start_idx)
    };

    Some(shape[from..=to].to_vec())
}
