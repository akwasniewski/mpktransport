use egui::Stroke;
use std::collections::HashSet;
use walkers::{Map, lat_lon};

use crate::app::{map_style, App, shapes_plugin::ShapeLinesPlugin, stop_plugin::StopPlugin};

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
                Some((s.stop_lat?, s.stop_lon?, s.name.clone(), i, self.selected == Some(i)))
            })
            .collect();

        let clicked_cell = std::rc::Rc::new(std::cell::Cell::new(Option::<usize>::None));
        let clicked_cell2 = clicked_cell.clone();
        let (endpoint_stops, transfer_stops) = self.route_highlighted_stops();

        let stop_plugin = StopPlugin {
            stops: stops_data,
            pointer,
            clicked_out: clicked_cell2,
            endpoint_stops,
            transfer_stops,
        };

        // Get the shape tracking lines from our current search output.
        let route_shapes = self.get_active_route_shapes();
        let route_count = route_shapes.len();
        let mut active_paths: Vec<(Vec<(f64, f64)>, Stroke, bool)> = route_shapes
            .into_iter()
            .enumerate()
            .map(|(idx, path)| (path, map_style::route_stroke(idx, route_count), false))
            .collect();
        active_paths.extend(self.get_active_walking_paths());
        let shapes_plugin = ShapeLinesPlugin { paths: active_paths };

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

    fn route_highlighted_stops(&self) -> (HashSet<usize>, HashSet<usize>) {
        let mut endpoint_stops = HashSet::new();
        let mut transfer_stops = HashSet::new();

        let Some((from_station, to_station, journey)) = &self.route_result else {
            return (endpoint_stops, transfer_stops);
        };

        if let Some(journey) = journey {
            if let Some(first_leg) = journey.legs.first() {
                endpoint_stops.insert(first_leg.stop_idx);
            }
            if let Some(last_leg) = journey.legs.last() {
                endpoint_stops.insert(last_leg.stop_idx);
            }

            for pair in journey.legs.windows(2) {
                let previous = &pair[0];
                let next = &pair[1];
                if previous.trip_idx != next.trip_idx {
                    transfer_stops.insert(previous.stop_idx);
                    transfer_stops.insert(next.stop_idx);
                }
            }
        } else {
            if let Some(station) = self.graph.stations.get(*from_station) {
                endpoint_stops.extend(station.stops.iter().copied());
            }
            if let Some(station) = self.graph.stations.get(*to_station) {
                endpoint_stops.extend(station.stops.iter().copied());
            }
        }

        transfer_stops.retain(|stop| !endpoint_stops.contains(stop));
        (endpoint_stops, transfer_stops)
    }

    fn get_active_walking_paths(&self) -> Vec<(Vec<(f64, f64)>, Stroke, bool)> {
        let mut paths = Vec::new();

        if let Some((_, _, Some(journey))) = &self.route_result {
            let walking_pairs: Vec<_> = journey
                .legs
                .windows(2)
                .filter(|pair| pair[0].trip_idx.is_none() && pair[1].trip_idx.is_none())
                .collect();
            let walking_count = walking_pairs.len();

            for (idx, pair) in walking_pairs.into_iter().enumerate() {
                let start = &pair[0];
                let end = &pair[1];

                let start_coord = self
                    .graph
                    .stops
                    .get(start.stop_idx)
                    .and_then(|s| Some((s.stop_lat?, s.stop_lon?)));
                let end_coord = self
                    .graph
                    .stops
                    .get(end.stop_idx)
                    .and_then(|s| Some((s.stop_lat?, s.stop_lon?)));

                if let (Some(start_coord), Some(end_coord)) = (start_coord, end_coord) {
                    paths.push((vec![start_coord, end_coord], map_style::walking_stroke(idx, walking_count), true));
                }
            }
        }

        paths
    }
}
