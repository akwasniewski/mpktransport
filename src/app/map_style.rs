use egui::{Color32, Stroke};

pub(super) fn route_stroke(idx: usize, total: usize) -> Stroke {
    Stroke::new(
        4.0,
        interpolate_color(
            Color32::from_rgb(0, 105, 255),
            Color32::from_rgb(70, 210, 255),
            gradient_t(idx, total),
        ),
    )
}

pub(super) fn walking_stroke(idx: usize, total: usize) -> Stroke {
    Stroke::new(
        5.0,
        interpolate_color(
            Color32::from_rgb(40, 220, 120),
            Color32::from_rgb(30, 220, 255),
            gradient_t(idx, total),
        ),
    )
}

pub(super) fn endpoint_fill() -> Color32 {
    Color32::from_rgb(255, 100, 0)
}

pub(super) fn transfer_fill() -> Color32 {
    Color32::from_rgb(180, 70, 255)
}

pub(super) fn transfer_ring() -> Color32 {
    Color32::from_rgb(255, 220, 80)
}

pub(super) fn selected_fill() -> Color32 {
    Color32::from_rgb(255, 180, 0)
}

pub(super) fn hovered_fill() -> Color32 {
    Color32::from_rgb(90, 200, 255)
}

pub(super) fn default_stop_fill() -> Color32 {
    Color32::from_rgb(30, 120, 220)
}

pub(super) fn bar_fill() -> Color32 {
    Color32::from_rgb(233, 30, 99)
}

fn gradient_t(idx: usize, total: usize) -> f32 {
    if total <= 1 {
        0.0
    } else {
        idx as f32 / (total - 1) as f32
    }
}

fn interpolate_color(from: Color32, to: Color32, t: f32) -> Color32 {
    let lerp = |a: u8, b: u8| -> u8 {
        (a as f32 + (b as f32 - a as f32) * t).round() as u8
    };

    Color32::from_rgb(
        lerp(from.r(), to.r()),
        lerp(from.g(), to.g()),
        lerp(from.b(), to.b()),
    )
}
