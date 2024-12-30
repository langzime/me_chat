use anyhow::Result;
mod window_handler;
mod config;
use window_handler::WindowHandler;

slint::include_modules!();

fn main() -> Result<()> {
    let app = Main::new()?;
    let window_handler = WindowHandler::new(app.as_weak());
    
    window_handler.init_window()?;
    window_handler.setup_window_events();
    
    app.run()?;
    Ok(())
}