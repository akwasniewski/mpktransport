use walkers::{HttpTiles, MapMemory, lat_lon};

use crate::graph::Graph;

#[derive(PartialEq)]
enum Tab { Map }

pub struct App {
    pub(super) graph:Graph,
    pub(super) filter: String,
    pub(super) selected: Option<usize>,
    pub(super) tab: Tab,
    pub(super) tiles: HttpTiles,
    pub(super) map_memory: MapMemory,
}

impl App {
    pub fn new(graph: Graph, ctx: egui::Context) -> Self {
        let centre = graph.centre().unwrap_or((50.06, 19.94));
        let tiles = HttpTiles::new(walkers::sources::OpenStreetMap, ctx);
        let mut map_memory = MapMemory::default();
        map_memory
            .set_zoom(14.0)
            .unwrap_or_else(|_| map_memory.set_zoom(16.0).unwrap());
        map_memory.center_at(lat_lon(centre.0, centre.1));

        Self {
            graph,
            filter: String::new(),
            selected: None,
            tab: Tab::Map,
            tiles,
            map_memory,
        }
    }

    pub(super) fn visible_indices(&self) -> Vec<usize> {
        let q = self.filter.to_lowercase();
        self.graph
            .stops
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                q.is_empty()
                    || s.stop_id.to_lowercase().contains(&q)
                    || s.stop_code.to_lowercase().contains(&q)
                    || s.stop_name.to_lowercase().contains(&q)
                    || s.stop_desc.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {

        // ── Right detail panel ──────────────────────────────────────────────
        egui::Panel::right("detail")
            .resizable(true)
            .default_size(300.0)
            .min_size(240.0)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                ui.heading("Stop detail");
                ui.separator();
                match self.selected {
                    None => {
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("Click a stop to inspect").italics().weak(),
                        );
                    }
                    Some(idx) => {
                        let s = &self.graph.stops[idx];
                        let fields: &[(&str, String)] = &[
                            ("ID",            s.stop_id.clone()),
                            ("Code",          s.stop_code.clone()),
                            ("Name",          s.stop_name.clone()),
                            ("Description",   s.stop_desc.clone()),
                            ("Latitude",      s.stop_lat.map_or("—".into(), |v| format!("{v:.6}"))),
                            ("Longitude",     s.stop_lon.map_or("—".into(), |v| format!("{v:.6}"))),
                            ("Zone",          s.zone_id.clone()),
                            ("URL",           s.stop_url.clone()),
                            ("Location type", s.location_type.map_or("—".into(), |v| v.to_string())),
                            ("Parent",        s.parent_station.clone()),
                            ("Timezone",      s.stop_timezone.clone()),
                            ("Wheelchair",    s.wheelchair_boarding.map_or("—".into(), |v| v.to_string())),
                            ("Platform",      s.platform_code.clone()),
                        ];
                        for (label, val) in fields {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(format!("{label}:")).strong().size(12.5));
                                ui.label(egui::RichText::new(val.as_str()).size(12.5));
                            });
                        }
                    }
                }
            });

        // ── Top toolbar ─────────────────────────────────────────────────────
        egui::Panel::top("toolbar").show_inside(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("🚌  GTFS Viewer");
                ui.separator();
                ui.label(egui::RichText::new(self.graph.source_dir.as_str()).small().weak());
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Map,   "🗺  Map");
                ui.separator();
                ui.label("Filter:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .hint_text("id / code / name / desc …")
                        .desired_width(260.0),
                );
                if ui.small_button("✕").clicked() {
                    self.filter.clear();
                }
                ui.separator();
                let vis = self.visible_indices().len();
                ui.label(
                    egui::RichText::new(format!("{vis} / {} stops", self.graph.stops.len()))
                        .small()
                        .weak(),
                );
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show_inside(ui, |ui| match self.tab {
            Tab::Map   => self.draw_map(ui),
        });

        let _ = frame;
    }
}
