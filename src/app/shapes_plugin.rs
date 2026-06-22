use walkers::{Plugin, Projector, MapMemory, lat_lon};
use egui::{Stroke, Shape, Ui, Response};

pub struct ShapeLinesPlugin {
    pub paths: Vec<Vec<(f64, f64)>>,
    pub stroke: Stroke,
}

impl Plugin for ShapeLinesPlugin {
    fn run(
        self: Box<Self>, 
        ui: &mut Ui, 
        response: &Response, 
        projector: &Projector, 
        _map_memory: &MapMemory
    ) {
        // We use with_clip_rect so that paths don't bleed outside the map's frame
        let painter = ui.painter().with_clip_rect(response.rect);

        for path in &self.paths {
            if path.len() < 2 { continue; }

            // Pinning fix: Most versions of walkers expect you to project the position,
            // which gives a coordinate relative to the widget's screen space.
        let points: Vec<egui::Pos2> = path
            .iter()
            .map(|&(lat, lon)| {
                let relative_vec = projector.project(lat_lon(lat, lon));
                // Convert Vec2 directly to Pos2
                egui::Pos2::new(relative_vec.x, relative_vec.y)
            })
            .collect();

            // Render continuous line segments across coordinate arrays
            painter.add(Shape::line(points, self.stroke));
        }
    }
}
