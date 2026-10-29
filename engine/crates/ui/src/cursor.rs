//! The game's neon outline pointer, used instead of the platform cursor.

use egui::{Color32, Context, Stroke};

/// Draw the current pointer above the game's other UI layers.
pub fn draw(ctx: &Context) {
    let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) else {
        return;
    };
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("openbf_custom_cursor"),
    ));
    // The reference cursor was larger than the native pointer; draw it at
    // roughly half scale so its on-screen footprint is close to a system cursor.
    let scale = 0.5;
    let mut outline = vec![
        pos,
        pos + egui::vec2(0.0, 30.0) * scale,
        pos + egui::vec2(8.0, 22.0) * scale,
        pos + egui::vec2(14.0, 36.0) * scale,
        pos + egui::vec2(21.0, 33.0) * scale,
        pos + egui::vec2(15.0, 19.0) * scale,
        pos + egui::vec2(29.0, 19.0) * scale,
    ];
    outline.push(pos);

    // Layer broad, transparent cyan strokes under the crisp white-blue edge
    // to match the neon cursor reference without obscuring the game beneath.
    for (width, color) in [
        (13.0, Color32::from_rgba_unmultiplied(0, 95, 255, 24)),
        (8.0_f32, Color32::from_rgba_unmultiplied(0, 210, 255, 45)),
        (4.5, Color32::from_rgba_unmultiplied(0, 245, 255, 110)),
        (2.2, Color32::from_rgb(100, 255, 255)),
        (1.0, Color32::WHITE),
    ] {
        p.add(egui::Shape::line(
            outline.clone(),
            Stroke::new(width * scale, color),
        ));
    }
}
