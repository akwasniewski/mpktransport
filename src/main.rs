use anyhow::{Context, Result};
use eframe::egui;
use serde::Deserialize;
use std::{env, path::Path};
use walkers::{lat_lon, HttpTiles, Map, MapMemory, Plugin, Projector};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct Stop {
    pub stop_id: String,
    #[serde(default)]
    pub stop_code: String,
    #[serde(default)]
    pub stop_name: String,
    #[serde(default)]
    pub stop_desc: String,
    #[serde(default)]
    pub stop_lat: Option<f64>,
    #[serde(default)]
    pub stop_lon: Option<f64>,
    #[serde(default)]
    pub zone_id: String,
    #[serde(default)]
    pub stop_url: String,
    #[serde(default)]
    pub location_type: Option<u8>,
    #[serde(default)]
    pub parent_station: String,
    #[serde(default)]
    pub stop_timezone: String,
    #[serde(default)]
    pub wheelchair_boarding: Option<u8>,
    #[serde(default)]
    pub platform_code: String,
}

#[derive(Debug, Default)]
pub struct Graph {
    pub stops: Vec<Stop>,
    pub source_dir: String,
}

impl Graph {
    pub fn load(dir: &Path) -> Result<Self> {
        let stops = load_stops(&dir.join("stops.txt"))?;
        Ok(Self {
            stops,
            source_dir: dir.to_string_lossy().into_owned(),
        })
    }

    /// Average lat/lon of all stops that have coordinates.
    pub fn centre(&self) -> Option<(f64, f64)> {
        let coords: Vec<(f64, f64)> = self
            .stops
            .iter()
            .filter_map(|s| Some((s.stop_lat?, s.stop_lon?)))
            .collect();
        if coords.is_empty() {
            return None;
        }
        let n = coords.len() as f64;
        Some((
            coords.iter().map(|c| c.0).sum::<f64>() / n,
            coords.iter().map(|c| c.1).sum::<f64>() / n,
        ))
    }
}

// ---------------------------------------------------------------------------
// CSV parsing
// ---------------------------------------------------------------------------

fn load_stops(path: &Path) -> Result<Vec<Stop>> {
    if !path.exists() {
        eprintln!("Warning: {} not found", path.display());
        return Ok(vec![]);
    }
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("Cannot open {}", path.display()))?;

    let mut stops = Vec::new();
    for result in rdr.deserialize::<Stop>() {
        match result {
            Ok(s) => stops.push(s),
            Err(e) => eprintln!("Warning: skipping row – {e}"),
        }
    }
    Ok(stops)
}

// ---------------------------------------------------------------------------
// Tab
// ---------------------------------------------------------------------------

#[derive(PartialEq)]
enum Tab { Table, Map }


// ---------------------------------------------------------------------------
// GUI state
// ---------------------------------------------------------------------------

struct App {
    graph: Graph,
    filter: String,
    selected: Option<usize>,
    tab: Tab,
    tiles: HttpTiles,
    map_memory: MapMemory,
}

impl App {
    fn new(graph: Graph, ctx: egui::Context) -> Self {
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

    fn visible_indices(&self) -> Vec<usize> {
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

// ---------------------------------------------------------------------------
// Table layout constants
// ---------------------------------------------------------------------------

const COL_WIDTHS: [f32; 6]   = [150.0, 80.0, 220.0, 140.0, 80.0, 80.0];
const COL_HEADERS: [&str; 6] = ["stop_id", "code", "name", "desc", "lat", "lon"];
const ROW_HEIGHT: f32        = 22.0;

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

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
                ui.selectable_value(&mut self.tab, Tab::Table, "📋 Table");
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

        // ── Central area ────────────────────────────────────────────────────
        egui::CentralPanel::default().show_inside(ui, |ui| match self.tab {
            Tab::Table => self.draw_table(ui, &ctx),
            Tab::Map   => self.draw_map(ui),
        });

        let _ = frame;
    }
}

// ---------------------------------------------------------------------------
// Table view
// ---------------------------------------------------------------------------

impl App {
    fn draw_table(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            for (i, hdr) in COL_HEADERS.iter().enumerate() {
                ui.add_sized(
                    [COL_WIDTHS[i], 20.0],
                    egui::Label::new(egui::RichText::new(*hdr).strong().size(12.0)),
                );
            }
        });
        ui.separator();

        let visible = self.visible_indices();
        let sel_color = ctx.global_style().visuals.selection.bg_fill.linear_multiply(0.40);
        let hov_color = ctx.global_style().visuals.widgets.hovered.bg_fill.linear_multiply(0.30);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, ROW_HEIGHT, visible.len(), |ui, range| {
                for row_i in range {
                    let idx = visible[row_i];
                    let s = &self.graph.stops[idx];
                    let is_selected = self.selected == Some(idx);

                    let cells: [String; 6] = [
                        s.stop_id.clone(),
                        s.stop_code.clone(),
                        s.stop_name.clone(),
                        s.stop_desc.clone(),
                        s.stop_lat.map_or("—".into(), |v| format!("{v:.5}")),
                        s.stop_lon.map_or("—".into(), |v| format!("{v:.5}")),
                    ];

                    let row_resp = ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        for (ci, text) in cells.iter().enumerate() {
                            ui.add_sized(
                                [COL_WIDTHS[ci], ROW_HEIGHT - 2.0],
                                egui::Label::new(egui::RichText::new(text.as_str()).size(12.0))
                                    .truncate(),
                            );
                        }
                    }).response;

                    let rect = row_resp.rect;
                    let hovered = ctx.input(|i| i.pointer.hover_pos().map_or(false, |p| rect.contains(p)));

                    if is_selected {
                        ui.painter().rect_filled(rect, 2.0, sel_color);
                    } else if hovered {
                        ui.painter().rect_filled(rect, 2.0, hov_color);
                    }

                    if hovered && ctx.input(|i| i.pointer.primary_clicked()) {
                        self.selected = Some(idx);
                    }
                }
            });
    }
}

// ---------------------------------------------------------------------------
// Map view
// ---------------------------------------------------------------------------

impl App {
    fn draw_map(&mut self, ui: &mut egui::Ui) {
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

        let plugin = StopPluginV2 {
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

// ---------------------------------------------------------------------------
// Plugin V2 — uses Rc<Cell<>> instead of unsafe for click-back
// ---------------------------------------------------------------------------

struct StopPluginV2 {
    stops: Vec<(f64, f64, String, usize, bool)>,
    pointer: Option<egui::Pos2>,
    clicked_out: std::rc::Rc<std::cell::Cell<Option<usize>>>,
}

impl Plugin for StopPluginV2 {
    fn run(
        self: Box<Self>,
        ui: &mut egui::Ui,
        response: &egui::Response,
        projector: &Projector,
        _map_memory: &MapMemory,
    ) {
        let painter = ui.painter();
        let primary_clicked = ui.ctx().input(|i| i.pointer.primary_clicked());

        let to_screen = |lat: f64, lon: f64| -> egui::Pos2 {
            let v = projector.project(lat_lon(lat, lon));
            egui::pos2(v.x, v.y)
        };

        for (lat, lon, name, idx, is_sel) in &self.stops {
            let screen_pt = to_screen(*lat, *lon);

            if !response.rect.expand(12.0).contains(screen_pt) {
                continue;
            }

            let radius = if *is_sel { 8.0_f32 } else { 5.5_f32 };
            let dist   = self.pointer.map_or(f32::MAX, |p| p.distance(screen_pt));
            let hovered = dist < radius + 5.0;

            let fill = if *is_sel {
                egui::Color32::from_rgb(255, 180, 0)
            } else if hovered {
                egui::Color32::from_rgb(90, 200, 255)
            } else {
                egui::Color32::from_rgb(30, 120, 220)
            };

            painter.circle(screen_pt, radius, fill, egui::Stroke::new(1.5, egui::Color32::WHITE));

            if hovered {
                // Draw tooltip above the circle
                let font   = egui::FontId::proportional(13.0);
                let galley = painter.layout_no_wrap(name.clone(), font, egui::Color32::WHITE);
                let pad    = egui::vec2(8.0, 4.0);
                let tip_size = galley.size() + pad * 2.0;
                let tip_min = screen_pt + egui::vec2(-tip_size.x / 2.0, -radius - tip_size.y - 4.0);
                let bg_rect = egui::Rect::from_min_size(tip_min, tip_size);

                painter.rect_filled(
                    bg_rect,
                    4.0,
                    egui::Color32::from_rgba_unmultiplied(20, 20, 20, 215),
                );
                painter.galley(
                    bg_rect.min + pad,
                    galley,
                    egui::Color32::WHITE,
                );

                if primary_clicked {
                    self.clicked_out.set(Some(*idx));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path/to/gtfs_directory>", args[0]);
        std::process::exit(1);
    }

    let dir = Path::new(&args[1]);
    if !dir.is_dir() {
        anyhow::bail!("'{}' is not a directory", dir.display());
    }

    println!("Loading GTFS from: {}", dir.display());
    let graph = Graph::load(dir).context("Failed to load GTFS")?;
    println!("  stops loaded: {}", graph.stops.len());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GTFS Viewer")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "GTFS Viewer",
        options,
        Box::new(|cc| Ok(Box::new(App::new(graph, cc.egui_ctx.clone())))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
