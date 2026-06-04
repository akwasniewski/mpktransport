use walkers::{HttpTiles, MapMemory, lat_lon};

use crate::graph::Graph;

#[derive(PartialEq)]
enum Tab { Map }

pub struct App {
    pub(super) graph:Graph,
    pub(super) filter: String,
    pub(super) selected: Option<usize>,
    pub(super) selected_stop_lines: Vec<String>,
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
            selected_stop_lines: Vec::new(),
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
        let ctx = ui.ctx().clone();

        // ── Top toolbar ─────────────────────────────────────────────────────
        egui::Panel::top("toolbar").show_inside(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("🚊 MPKTransport");
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
        Tab::Map => {
            let mut clear_selection = false;

            // 1. Right Side Panel
            if let Some(idx) = self.selected {
                // We use an explicit ID so we can clear its state later
                let panel_id = egui::Id::new("stop_detail_panel");

                let panel_response = egui::Panel::right(panel_id)
                    .default_size(300.0)
                    .resizable(true)
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            self.draw_stop_panel(ui, idx);
                        });
                    });

                // Check if user dragged it closed
                if panel_response.response.rect.width() <= 100.0 {
                    clear_selection = true;
                    
                    // FIX: Force egui to delete the stored width state for this panel
                    ui.ctx().memory_mut(|mem| {
                        // This clears the persisted size data for this specific panel ID
                        mem.data.remove::<egui::panel::PanelState>(panel_id);
                    });
                }
            }

                // Deferred state update out of layout loop
                if clear_selection {
                    self.selected = None;
                    self.selected_stop_lines.clear();
                }

                // 2. Central Panel Map view
                egui::CentralPanel::default().show_inside(ui, |ui| match self.tab {
                    Tab::Map => {
                        self.draw_map(ui);
                    }
                });
            }
        });

        let _ = frame;
    }
}
