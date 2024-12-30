use slint::{LogicalPosition, LogicalSize, Weak, ComponentHandle};
use crate::config::WindowConfig;
use crate::Main;

pub struct WindowHandler {
    window: Weak<Main>,
    config: WindowConfig,
}

impl WindowHandler {
    pub fn new(window: Weak<Main>) -> Self {
        Self {
            window,
            config: WindowConfig::default(),
        }
    }

    pub fn init_window(&self) -> anyhow::Result<()> {
        let window = self.window.upgrade().ok_or_else(|| anyhow::anyhow!("Window not found"))?;
        window.window().set_size(LogicalSize::new(
            self.config.default_width,
            self.config.default_height,
        ));
        Ok(())
    }

    pub fn setup_window_events(&self) {
        let window = self.window.clone();
        self.window.upgrade().unwrap().on_close_window(move || {
            window.upgrade().unwrap().window().hide().unwrap();
        });

        let window = self.window.clone();
        self.window.upgrade().unwrap().on_minimized_window(move |enable| {
            window.upgrade().unwrap().window().set_minimized(enable);
        });

        let window = self.window.clone();
        self.window.upgrade().unwrap().on_maximized_window(move |enable| {
            window.upgrade().unwrap().window().set_maximized(enable);
        });

        let window = self.window.clone();
        self.window.upgrade().unwrap().on_move_window(move |offset_x, offset_y| {
            let main = window.upgrade().unwrap();
            let logical_pos = main.window().position().to_logical(main.window().scale_factor());
            main.window().set_position(LogicalPosition::new(
                logical_pos.x + offset_x,
                logical_pos.y + offset_y,
            ));
        });
    }
} 