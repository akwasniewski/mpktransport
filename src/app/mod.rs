pub mod draw_map;
pub mod stop_plugin;
pub mod shapes_plugin;
pub mod draw_stop_panel;
pub mod route_search;

use walkers::{HttpTiles, MapMemory, lat_lon};

use crate::{bar_stops_graph::BarsStops, graph::Graph, journey::Journey};

#[derive(PartialEq)]
pub(super) enum Tab { Map, Routes }

#[derive(Debug, Clone, PartialEq)]
pub enum RoutingAlgorithm {
    Raptor,
    Csa,
}

/// A clickable marker on the map. Keeps bar ids in a separate namespace from
/// stop ids, so `Bar(5)` and `Stop(5)` never collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerId {
    Stop(usize),
    Bar(usize),
}

pub struct App {
    pub(super) graph: Graph,
    pub(super) bars: BarsStops,
    pub(super) filter: String,
    pub(super) selected: Option<MarkerId>,
    pub(super) selected_stop_lines: Vec<String>,
    pub(super) tab: Tab,
    pub(super) tiles: HttpTiles,
    pub(super) map_memory: MapMemory,
    // Route search state
    pub(super) route_from: String,
    pub(super) route_to: String,
    pub(super) route_from_selected: Option<usize>,
    pub(super) route_to_selected: Option<usize>,
    pub(super) route_from_focused: bool,
    pub(super) route_to_focused: bool,
    pub(super) route_result: Option<(usize, usize, Option<Journey>)>,
    pub(super) routing_algorithm: RoutingAlgorithm,
}

impl App {
    pub fn new(graph: Graph, bars: BarsStops, ctx: egui::Context) -> Self {
        let centre = graph.centre().unwrap_or((50.06, 19.94));
        let tiles = HttpTiles::new(walkers::sources::OpenStreetMap, ctx);
        let mut map_memory = MapMemory::default();
        map_memory
            .set_zoom(14.0)
            .unwrap_or_else(|_| map_memory.set_zoom(16.0).unwrap());
        map_memory.center_at(lat_lon(centre.0, centre.1));


    Self {
        graph,
        bars,
        filter: String::new(),
        selected: None,
        selected_stop_lines: Vec::new(),
        tab: Tab::Map,
        tiles,
        map_memory,
        route_from: String::new(),
        route_to: String::new(),
        route_from_selected: None,
        route_to_selected: None,
        route_from_focused: false,
        route_to_focused: false,
        route_result: None,
        routing_algorithm: RoutingAlgorithm::Raptor,
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
                    || s.stop_code.to_lowercase().contains(&q)
                    || s.name.to_lowercase().contains(&q)
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
                ui.selectable_value(&mut self.tab, Tab::Map,    "🗺  Map");
                ui.selectable_value(&mut self.tab, Tab::Routes, "🔀  Routes");
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

    egui::CentralPanel::default().show_inside(ui, |ui| {
    let mut clear_selection = false;

    // ── Right side panel (stop info OR route search) ─────────────────
    let show_panel = self.selected.is_some() || self.tab == Tab::Routes;

    if show_panel {
        let panel_id = egui::Id::new("side_panel");

        let panel_response = egui::Panel::right(panel_id)
            .default_size(300.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.tab {
                        Tab::Routes => {
                            self.draw_route_search(ui);
                        }
                        Tab::Map => {
                            match self.selected {
                                Some(MarkerId::Stop(idx)) => self.draw_stop_panel(ui, idx),
                                Some(MarkerId::Bar(idx)) => self.draw_bar_panel(ui, idx),
                                None => {}
                            }
                        }
                    }
                });
            });

        // Dragged closed — only matters for stop panel
        if self.tab == Tab::Map && panel_response.response.rect.width() <= 100.0 {
            clear_selection = true;
            ui.ctx().memory_mut(|mem| {
                mem.data.remove::<egui::panel::PanelState>(panel_id);
            });
        }
    }

    if clear_selection {
        self.selected = None;
        self.selected_stop_lines.clear();
    }

    // ── Map always underneath ────────────────────────────────────────
    egui::CentralPanel::default().show_inside(ui, |ui| {
        self.draw_map(ui);
    });
});        let _ = frame;
    }
}
