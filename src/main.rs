use anyhow::Result;
mod window_handler;
mod config;
use window_handler::{WindowHandler, WindowEvents};

slint::slint!{
    import { Main } from "ui/main.slint";
    import { Login } from "ui/login.slint";
    export { Main, Login }
}

impl WindowEvents for Main {
    fn on_close_window(&self, callback: impl Fn() + 'static) { self.on_close_window(callback); }
    fn on_minimized_window(&self, callback: impl Fn(bool) + 'static) { self.on_minimized_window(callback); }
    fn on_maximized_window(&self, callback: impl Fn(bool) + 'static) { self.on_maximized_window(callback); }
    fn on_move_window(&self, callback: impl Fn(f32, f32) + 'static) { self.on_move_window(callback); }
}

impl WindowEvents for Login {
    fn on_close_window(&self, callback: impl Fn() + 'static) { self.on_close_window(callback); }
    fn on_minimized_window(&self, callback: impl Fn(bool) + 'static) { self.on_minimized_window(callback); }
    fn on_maximized_window(&self, callback: impl Fn(bool) + 'static) { self.on_maximized_window(callback); }
    fn on_move_window(&self, callback: impl Fn(f32, f32) + 'static) { self.on_move_window(callback); }
}

fn main() -> Result<()> {
    let app = Login::new()?;
    let window_handler = WindowHandler::new(app.as_weak());
    
    window_handler.init_window()?;
    window_handler.setup_window_events();
    
    // 设置登录按钮点击事件
    let weak_app = app.as_weak();
    app.on_login(move || {
        if let Some(app) = weak_app.upgrade() {
            let main_window = Main::new().unwrap();
            let main_handler = WindowHandler::new(main_window.as_weak());
            main_handler.init_window().unwrap();
            main_handler.setup_window_events();
            main_window.show().unwrap();
            app.window().hide().unwrap();
        }
    });
    
    app.run()?;
    Ok(())
}