//! The game's neon outline pointer, used instead of the platform cursor, and
//! the shell's pointer state ([`Pointer`]).

use egui::{Color32, Context, Stroke};

/// The shell pointer: where it is, and whether it moved or clicked since the
/// last frame. Hover moves menu focus only on fresh pointer activity, so a
/// resting cursor cannot undo keyboard or controller navigation.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Pointer {
    pos: Option<[f32; 2]>,
    moved: bool,
    click: bool,
}

impl Pointer {
    /// The cursor is at `pos` (window pixels); a new spot counts as motion.
    pub fn moved_to(&mut self, pos: [f32; 2]) {
        if self.pos != Some(pos) {
            self.pos = Some(pos);
            self.moved = true;
        }
    }

    /// A left click for the next frame.
    pub fn click(&mut self) {
        self.click = true;
    }

    /// The pointer left the window or lost focus.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// This frame's pointer: its position and whether it clicked, only when
    /// it moved or clicked since the last call.
    pub fn take_frame(&mut self) -> Option<([f32; 2], bool)> {
        let fresh = self.moved || self.click;
        let click = std::mem::take(&mut self.click);
        self.moved = false;
        self.pos.filter(|_| fresh).map(|p| (p, click))
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::MenuInput;
    use crate::shell_ps2::{Page, ShellMenus};

    /// One shell frame as the windowed game runs it: a fresh pointer over a
    /// row focuses it (the cursor here rests over row 0), then the step.
    fn frame(menus: &mut ShellMenus, pointer: &mut Pointer, mut input: MenuInput) {
        if let Some((_, click)) = pointer.take_frame() {
            menus.focus_row(0);
            input.confirm |= click;
        }
        menus.step(input, false, 0.1);
    }

    #[test]
    fn a_resting_pointer_does_not_undo_keyboard_navigation() {
        let mut menus = ShellMenus::new(&|_, s| s.to_owned());
        menus.step(MenuInput::default(), true, 1.0);
        let mut pointer = Pointer::default();
        pointer.moved_to([100.0, 200.0]);
        frame(&mut menus, &mut pointer, MenuInput::default());
        assert_eq!(menus.stack().unwrap().focus, 0, "hover focuses row 0");
        let down = MenuInput {
            down: true,
            ..Default::default()
        };
        frame(&mut menus, &mut pointer, down);
        assert_eq!(menus.stack().unwrap().focus, 3, "keyboard moves to row 3");
        // The cursor stays over row 0: idle frames, and a repeat of the same spot.
        frame(&mut menus, &mut pointer, MenuInput::default());
        pointer.moved_to([100.0, 200.0]);
        frame(&mut menus, &mut pointer, MenuInput::default());
        assert_eq!(
            menus.stack().unwrap().focus,
            3,
            "idle pointer reset the focus"
        );
        // Confirm opens the row the keyboard chose.
        let confirm = MenuInput {
            confirm: true,
            ..Default::default()
        };
        frame(&mut menus, &mut pointer, confirm);
        assert_eq!(menus.page, Page::Options);
    }

    #[test]
    fn pointer_motion_and_clicks_are_fresh_once() {
        let mut p = Pointer::default();
        assert_eq!(p.take_frame(), None);
        p.click();
        assert_eq!(p.take_frame(), None, "a click with no position");
        p.moved_to([1.0, 2.0]);
        assert_eq!(p.take_frame(), Some(([1.0, 2.0], false)));
        assert_eq!(p.take_frame(), None);
        p.click();
        assert_eq!(p.take_frame(), Some(([1.0, 2.0], true)));
        assert_eq!(p.take_frame(), None);
        p.clear();
        p.moved_to([1.0, 2.0]);
        assert_eq!(
            p.take_frame(),
            Some(([1.0, 2.0], false)),
            "back after a clear"
        );
    }
}
