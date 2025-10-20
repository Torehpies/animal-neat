use macroquad::prelude::*;

/// Collection of optional textures used by the visualizer.
pub struct Assets {
    pub herb: Option<Texture2D>,
    pub carn: Option<Texture2D>,
    pub plant: Option<Texture2D>,
    pub meat: Option<Texture2D>,
}

impl Default for Assets {
    fn default() -> Self {
        Self { herb: None, carn: None, plant: None, meat: None }
    }
}

/// Try to load a texture at `path`. Returns None on any error. Sets nearest filter for pixel-art sprites.
pub async fn try_load(path: &str) -> Option<Texture2D> {
    match load_texture(path).await {
        Ok(t) => { t.set_filter(FilterMode::Nearest); Some(t) }
        Err(_) => None,
    }
}

/// Attempt to preload the common set of example textures.
/// This tries a couple of likely locations so running from different CWDs works.
pub async fn preload_textures() -> Assets {
    let mut out = Assets::default();

    // Helper to try two paths
    async fn try_two(a: &str, b: &str) -> Option<Texture2D> {
        if let Some(t) = try_load(a).await { return Some(t); }
        try_load(b).await
    }

    out.herb = try_two("assets/sheep.png", ".vscode/assets/sheep.png").await;
    out.carn = try_two("assets/wolf.png", ".vscode/assets/wolf.png").await;
    out.plant = try_two("assets/plant_1.png", ".vscode/assets/plant_1.png").await;
    out.meat = try_two("assets/meat.png", ".vscode/assets/meat.png").await;

    out
}

/// Show a short HUD toast by mutating the central AppState. This keeps the toast API ergonomic
/// and avoids touching internals elsewhere.
#[allow(dead_code)]
pub fn set_hud_toast(state: &mut crate::AppState, msg: impl Into<String>, secs: f32) {
    state.hud_toast = Some((msg.into(), secs));
}

/// Per-frame ticker for the HUD toast; call this once per frame to decrement the remaining time.
pub fn tick_hud_toast(state: &mut crate::AppState) {
    if let Some((_, ref mut t)) = state.hud_toast {
        let dt = get_frame_time();
        *t -= dt;
        if *t <= 0.0 { state.hud_toast = None; }
    }
}
