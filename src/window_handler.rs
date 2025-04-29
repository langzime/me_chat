use crate::config::WindowConfig;
use slint::{ComponentHandle, LogicalPosition, LogicalSize, Weak};

pub trait WindowEvents {
    fn on_close_window(&self, callback: impl Fn() + 'static);
    fn on_minimized_window(&self, callback: impl Fn(bool) + 'static);
    fn on_maximized_window(&self, callback: impl Fn(bool) + 'static);
    fn on_move_window(&self, callback: impl Fn(f32, f32) + 'static);
}

pub struct WindowHandler<T: ComponentHandle + WindowEvents + 'static> {
    window: Weak<T>,
    config: WindowConfig,
}

impl<T: ComponentHandle + WindowEvents + 'static> WindowHandler<T> {
    pub fn new(window: Weak<T>) -> Self {
        Self {
            window,
            config: WindowConfig::default(),
        }
    }

    pub fn init_window(&self) -> anyhow::Result<()> {
        let window = self
            .window
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("Window not found"))?;
        window.window().set_size(LogicalSize::new(
            self.config.default_width,
            self.config.default_height,
        ));
        Ok(())
    }

    pub fn setup_window_events(&self) {
        let window = self.window.clone();
        self.window.upgrade().unwrap().on_close_window(move || {
            if let Some(win) = window.upgrade() {
                win.window().hide().unwrap();
            }
        });

        let window = self.window.clone();
        self.window
            .upgrade()
            .unwrap()
            .on_minimized_window(move |enable| {
                if let Some(win) = window.upgrade() {
                    win.window().set_minimized(enable);
                }
            });

        let window = self.window.clone();
        self.window
            .upgrade()
            .unwrap()
            .on_maximized_window(move |enable| {
                if let Some(win) = window.upgrade() {
                    win.window().set_maximized(enable);
                }
            });

        let window = self.window.clone();
        self.window
            .upgrade()
            .unwrap()
            .on_move_window(move |offset_x: f32, offset_y: f32| {
                if let Some(win) = window.upgrade() {
                    let logical_pos = win
                        .window()
                        .position()
                        .to_logical(win.window().scale_factor());
                    win.window().set_position(LogicalPosition::new(
                        logical_pos.x + offset_x,
                        logical_pos.y + offset_y,
                    ));
                }
            });
    }
}
