use crate::app::App;

impl App {
    pub fn draw_stop_panel(&self, ui: &mut egui::Ui, idx: usize) {
        let stop = &self.graph.stops[idx];

        ui.vertical(|ui| {
            ui.heading(&stop.stop_name);
            ui.add_space(4.0);

            ui.label(format!("ID: {}", stop.idx));

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

}
