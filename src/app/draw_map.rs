use egui::{Color32, Stroke};
use walkers::{Map, lat_lon};

use crate::app::{App, shapes_plugin::ShapeLinesPlugin, stop_plugin::StopPlugin};

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

        let clicked_cell = std::rc::Rc::new(std::cell::Cell::new(Option::<usize>::None));
        let clicked_cell2 = clicked_cell.clone();
        let highlighted: std::collections::HashSet<usize> = match self.route_result {
            Some((a, b, _)) => [a, b].into(),
            None => std::collections::HashSet::new(),
        };

        let stop_plugin = StopPlugin {
            stops: stops_data,
            pointer,
            clicked_out: clicked_cell2,
            highlighted
        };

        // Get the shape tracking lines from our current search output
        let active_shapes = self.get_active_route_shapes();
        let shapes_plugin = ShapeLinesPlugin {
            paths: active_shapes,
            stroke: Stroke::new(4.0, Color32::from_rgb(0, 122, 255)), // Vibrant Blue Route Path
        };

        // Combine both plugins chain-style inside walkers::Map builder configuration
        let map_widget = Map::new(
            Some(&mut self.tiles),
            &mut self.map_memory,
            live_centre,
        )
        .with_plugin(shapes_plugin) // Renders shapes lines beneath the icons
        .with_plugin(stop_plugin);

        ui.add(map_widget);

        ctx.request_repaint();
    }
}
