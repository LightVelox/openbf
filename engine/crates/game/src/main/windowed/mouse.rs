use super::{App, Screen};
use winit::event::WindowEvent;

impl App {
    pub(super) fn in_battle(&self) -> bool {
        self.walk.is_some() && matches!(self.screen, Screen::Battle)
    }

    pub(super) fn release_mouse(&mut self) {
        if let Some(w) = &self.window {
            self.kbm.release(w);
        }
    }

    pub(super) fn mouse_window_event(&mut self, event: &WindowEvent) {
        let battle = self.in_battle();
        let can_grab = battle
            && !self.paused
            && self
                .walk
                .as_ref()
                .and_then(|(walk, _)| walk.sim.unit(walk.player))
                .is_some_and(|u| u.alive);
        self.kbm.window_event(event, battle, can_grab);
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = Some([position.x, position.y]);
            }
            WindowEvent::MouseInput {
                button: winit::event::MouseButton::Left,
                state: winit::event::ElementState::Pressed,
                ..
            } if matches!(self.screen, Screen::Shell) => self.mouse_shell_click = true,
            WindowEvent::Focused(false) => {
                self.kbm.release_keys();
                self.release_mouse();
                self.mouse_position = None;
                self.mouse_shell_click = false;
                if let Some(w) = &self.window {
                    w.set_cursor_visible(true);
                }
            }
            WindowEvent::Focused(true) => {
                if let Some(w) = &self.window {
                    w.set_cursor_visible(!matches!(self.screen, Screen::Shell));
                }
            }
            WindowEvent::CloseRequested => self.release_mouse(),
            _ => {}
        }
    }

    pub(super) fn sync_grab(&mut self) {
        let alive = self
            .walk
            .as_ref()
            .and_then(|(walk, _)| walk.sim.unit(walk.player))
            .is_some_and(|u| u.alive);
        let allowed = self.in_battle() && !self.paused && alive;
        if let Some(w) = &self.window {
            self.kbm.sync_grab(w, allowed);
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.release_mouse();
    }
}
