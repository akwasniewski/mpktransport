use walkers::{Map, lat_lon};

use crate::app::{app::App, stop_plugin::StopPlugin};

impl App {
    pub(super) fn draw_map(&mut self, ui: &mut egui::Ui) {
        let live_centre = self
            .map_memory
            .detached()                    // Option<Position> — the panned centre
            .unwrap_or_else(|| {
                let (lat, lon) = self.graph.centre().unwrap_or((50.06, 19.94));
                lat_lon(lat, lon)
            });
        let ctx = ui.ctx().clone();
        let pointer = ctx.input(|i| i.pointer.hover_pos());

        let visible = self.visible_indices();
        let stops_data: Vec<(f64, f64, String, usize, bool)> = visible
            .iter()
            .filter_map(|&i| {
                let s = &self.graph.stops[i];
                Some((s.stop_lat?, s.stop_lon?, s.stop_name.clone(), i, self.selected == Some(i)))
            })
            .collect();


        // Build the plugin — we use a Mutex-free trick: capture `clicked` as a
        // shared cell so the plugin can write back to us after being consumed.
        let clicked_cell = std::rc::Rc::new(std::cell::Cell::new(Option::<usize>::None));
        let clicked_cell2 = clicked_cell.clone();

        let plugin = StopPlugin {
            stops: stops_data,
            pointer,
            clicked_out: clicked_cell2,
        };

        let map_widget = Map::new(
            Some(&mut self.tiles),
            &mut self.map_memory,
            live_centre,          // always matches what the Projector will use
        )
        .with_plugin(plugin);

        ui.add(map_widget);

        // Apply any click that happened inside the plugin
        if let Some(idx) = clicked_cell.get() {
            self.selected = Some(idx);
        }

        ctx.request_repaint();
    }
}
