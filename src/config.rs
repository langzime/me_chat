pub struct WindowConfig {
    pub default_width: f32,
    pub default_height: f32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            default_width: 800.0,
            default_height: 640.0,
        }
    }
}
