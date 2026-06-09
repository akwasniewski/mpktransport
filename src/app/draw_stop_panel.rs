use crate::app::App;

impl App {
    pub fn draw_stop_panel(&self, ui: &mut egui::Ui, idx: usize) {
        let stop = &self.graph.stops[idx];

        ui.vertical(|ui| {
            ui.heading(&stop.stop_name);
            ui.add_space(4.0);

            ui.label(format!("ID: {}", stop.stop_id));

            if let (Some(lat), Some(lon)) = (stop.stop_lat, stop.stop_lon) {
                ui.label(format!("Coordinates: {:.6}, {:.6}", lat, lon));
            }

            if !stop.stop_code.is_empty() {
                ui.label(format!("Code: {}", stop.stop_code));
            }

            if !stop.zone_id.is_empty() {
                ui.label(format!("Zone: {}", stop.zone_id));
            }

            if !stop.stop_desc.is_empty() {
                ui.label(&stop.stop_desc);
            }

            ui.separator();
            ui.strong("Available Lines:");

            // Uses the cached data string vector directly - no slow traversal frames!
            if !self.selected_stop_lines.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for line_name in &self.selected_stop_lines {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, format!("[{}]", line_name));
                    }
                });
            } else {
                ui.weak("No scheduled lines service this stop.");
            }
        });
    }

    // Call this helper whenever a stop is selected/clicked on your map
    pub fn select_stop(&mut self, stop_idx: usize) {
        self.selected = Some(stop_idx);
        self.selected_stop_lines.clear();

        let stop = &self.graph.stops[stop_idx];
        if let Some(stop_time_indices) = self.graph.stop_times_by_stop.get(&stop.stop_id) {
            let mut unique_routes = std::collections::HashSet::new();
            
            for &st_idx in stop_time_indices {
                if let Some(st) = self.graph.stop_times.get(st_idx) {
                    if let Some(&trip_idx) = self.graph.trips_by_id.get(&st.trip_id) {
                        if let Some(trip) = self.graph.trips.get(trip_idx) {
                            if let Some(&route_idx) = self.graph.routes_by_id.get(&trip.route_id) {
                                if let Some(route) = self.graph.routes.get(route_idx) {
                                    let name = if !route.route_short_name.is_empty() {
                                        &route.route_short_name
                                    } else {
                                        &route.route_long_name
                                    };
                                    unique_routes.insert(name.clone());
                                }
                            }
                        }
                    }
                }
            }
            self.selected_stop_lines = unique_routes.into_iter().collect();
        }
    }
}
