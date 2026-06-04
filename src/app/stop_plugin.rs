use walkers::{lat_lon, MapMemory, Plugin, Projector};

pub(super) struct StopPlugin {
    pub(super) stops: Vec<(f64, f64, String, usize, bool)>,
    pub(super) pointer: Option<egui::Pos2>,
    pub(super) clicked_out: std::rc::Rc<std::cell::Cell<Option<usize>>>,
}

impl Plugin for StopPlugin {
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


