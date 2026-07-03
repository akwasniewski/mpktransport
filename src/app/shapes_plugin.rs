use egui::{Response, Shape, Stroke, Ui};
use walkers::{lat_lon, MapMemory, Plugin, Projector};

pub struct ShapeLinesPlugin {
    pub paths: Vec<(Vec<(f64, f64)>, Stroke, bool)>,
}

impl Plugin for ShapeLinesPlugin {
    fn run(
        self: Box<Self>,
        ui: &mut Ui,
        response: &Response,
        projector: &Projector,
        _map_memory: &MapMemory,
    ) {
        let painter = ui.painter().with_clip_rect(response.rect);

        for (path, stroke, dotted) in &self.paths {
            if path.len() < 2 {
                continue;
            }

            let points: Vec<egui::Pos2> = path
                .iter()
                .map(|&(lat, lon)| {
                    let projected = projector.project(lat_lon(lat, lon));
                    egui::pos2(projected.x, projected.y)
                })
                .collect();

            if *dotted {
                draw_dotted_path(&painter, &points, *stroke);
            } else {
                painter.add(Shape::line(points, *stroke));
            }
        }
    }
}

fn draw_dotted_path(painter: &egui::Painter, points: &[egui::Pos2], stroke: Stroke) {
    let radius = (stroke.width / 2.0).max(2.0);
    let spacing = radius * 3.0;

    for segment in points.windows(2) {
        let start = segment[0];
        let end = segment[1];
        let delta = end - start;
        let len = delta.length();
        if len <= f32::EPSILON {
            continue;
        }

        let steps = (len / spacing).ceil() as usize;
        for step in 0..=steps {
            let t = step as f32 / steps.max(1) as f32;
            let point = start + delta * t;
            painter.circle_filled(point, radius, stroke.color);
        }
    }
}
