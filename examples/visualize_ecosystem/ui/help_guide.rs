use macroquad::prelude::*;

use crate::ui_common::{draw_panel, draw_text_clamped, draw_text_wrapped, SUBPANEL_BG, PANEL_BORDER, ACCENT};

/// A single slide in the in-sim help guide.
/// Image is optional; when missing, a placeholder will be drawn.
pub struct GuideSlide {
    pub title: String,
    pub description: String,
    pub image: Option<Texture2D>,
}

impl GuideSlide {
    fn new(title: &str, description: &str, image: Option<Texture2D>) -> Self {
        Self { title: title.to_string(), description: description.to_string(), image }
    }
}

/// Attempt to load a guide image from common locations.
async fn try_load_guide_image(name: &str) -> Option<Texture2D> {
    // Prefer assets/guide/<name> then .vscode/assets/guide/<name>
    let p1 = format!("assets/guide/{}", name);
    let p2 = format!(".vscode/assets/guide/{}", name);
    if let Some(t) = crate::ui_assets::try_load(&p1).await { return Some(t); }
    crate::ui_assets::try_load(&p2).await
}

/// Preload a curated set of slides. It's safe if images are missing.
pub async fn preload_help_slides() -> Vec<GuideSlide> {
    let mut slides = Vec::new();

    slides.push(GuideSlide::new(
        "Welcome",
        "This is the NEAT Ecosystem. On the left you see the simulated world; on the right, the HUD shows key stats. Use the controls to pause, speed up, and view network graphs.",
        try_load_guide_image("overview.png").await,
    ));

    slides.push(GuideSlide::new(
        "Basic Controls",
        "- Pause: toggle the simulation.\n- Fast/Ultra: accelerate evolution for quick experimentation.\n- View Options: open a modal with extra overlays (vision rays, energy bars, etc.).",
        try_load_guide_image("controls.png").await,
    ));

    slides.push(GuideSlide::new(
        "Agents",
        "Herbivores and carnivores roam the world. Click an agent to focus it and see details (energy, health, offspring). Best network panel shows the champion's topology.",
        try_load_guide_image("agents.png").await,
    ));

    slides.push(GuideSlide::new(
        "Graphs",
        "Open the Graphs Panel to monitor population trends, births/deaths per species, and more. Use it to diagnose stability or collapse in your ecosystem.",
        try_load_guide_image("graphs.png").await,
    ));

    slides
}

/// Draw the fullscreen help overlay with next/prev navigation and close.
/// Modifies `show` and `current_idx` based on input/clicks.
pub fn draw_help_overlay(
    fullscreen: Rect,
    slides: &mut [GuideSlide],
    current_idx: &mut usize,
    show: &mut bool,
) {
    if !*show { return; }

    // Background dim
    draw_rectangle(fullscreen.x, fullscreen.y, fullscreen.w, fullscreen.h, Color::new(0.0, 0.0, 0.0, 0.6));

    // Modal frame
    let modal_w = (screen_width() * 0.72).clamp(720.0, 1200.0);
    let modal_h = (screen_height() * 0.72).clamp(420.0, 820.0);
    let mx = (fullscreen.w - modal_w) * 0.5;
    let my = (fullscreen.h - modal_h) * 0.5;
    let frame = Rect { x: mx - 6.0, y: my - 6.0, w: modal_w + 12.0, h: modal_h + 12.0 };
    draw_panel(frame, SUBPANEL_BG, PANEL_BORDER, 2.0);

    // Header with title and progress
    let total = slides.len().max(1);
    *current_idx = (*current_idx).min(total - 1);
    let title = if slides.is_empty() { "Help".to_string() } else { format!("{}  ({}/{})", slides[*current_idx].title, *current_idx + 1, total) };
    draw_text_clamped(&title, mx + 16.0, my + 16.0, 24.0, LIGHTGRAY, modal_w - 32.0);

    // Body layout: image on left, text on right (responsive)
    let pad = 16.0;
    let content_x = mx + pad;
    let content_y = my + 52.0;
    let content_w = modal_w - pad * 2.0;
    let content_h = modal_h - 52.0 - 64.0; // minus header and footer

    let split = if content_w > 700.0 { 0.75 } else { 0.0 }; // stack on narrow screens
    let img_area = if split > 0.0 {
        Rect { x: content_x, y: content_y, w: content_w * split - pad * 0.5, h: content_h }
    } else {
        Rect { x: content_x, y: content_y, w: content_w, h: content_h * 0.55 }
    };
    let text_area = if split > 0.0 {
        Rect { x: content_x + content_w * split + pad * 0.5, y: content_y, w: content_w * (1.0 - split) - pad * 0.5, h: content_h }
    } else {
        Rect { x: content_x, y: content_y + img_area.h + pad, w: content_w, h: content_h - img_area.h - pad }
    };

    // Draw image or placeholder
    if let Some(slide) = slides.get(*current_idx) {
        if let Some(tex) = slide.image.as_ref() {
            // Fit image preserving aspect
            let iw = tex.width();
            let ih = tex.height();
            let ar = iw / ih;
            let target_w = img_area.w.min(img_area.h * ar);
            let target_h = target_w / ar;
            let ix = img_area.x + (img_area.w - target_w) * 0.5;
            let iy = img_area.y + (img_area.h - target_h) * 0.5;
            draw_texture_ex(tex, ix, iy, WHITE, DrawTextureParams { dest_size: Some(Vec2::new(target_w, target_h)), ..Default::default() });
        } else {
            // Placeholder
            let mut c = ACCENT; c.a = 0.15;
            draw_rectangle(img_area.x, img_area.y, img_area.w, img_area.h, Color::new(0.1, 0.1, 0.12, 1.0));
            draw_rectangle_lines(img_area.x, img_area.y, img_area.w, img_area.h, 1.0, c);
            draw_text_clamped("(No image)", img_area.x + 12.0, img_area.y + 22.0, 18.0, GRAY, img_area.w - 24.0);
        }

        // Description text
    let desc_fs = 18.0;
    let y = text_area.y;
    let _ = draw_text_wrapped(&slide.description, text_area.x, y, desc_fs, LIGHTGRAY, text_area.w, 6.0);
        let hint = "Tip: Use Left/Right arrows or click the chevrons. Press Esc to close.";
        draw_text_clamped(hint, text_area.x, text_area.y + text_area.h - 4.0, 14.0, GRAY, text_area.w);
    }

    // Footer with navigation and close
    let btn_h = 40.0;
    let footer_y = my + modal_h - btn_h - 12.0;

    // Prev button
    let prev_w = 120.0;
    let prev_x = mx + 16.0;
    let (mxp, myp) = mouse_position();
    let prev_hover = mxp >= prev_x && mxp <= prev_x + prev_w && myp >= footer_y && myp <= footer_y + btn_h;
    draw_rectangle(prev_x, footer_y, prev_w, btn_h, if prev_hover { Color::new(0.18, 0.18, 0.18, 1.0) } else { Color::new(0.12, 0.12, 0.12, 0.9) });
    draw_rectangle_lines(prev_x, footer_y, prev_w, btn_h, 1.0, Color::new(0.6, 0.6, 0.6, 0.8));
    draw_text("< Prev", prev_x + 18.0, footer_y + btn_h * 0.68, 20.0, WHITE);

    // Next button
    let next_w = 120.0;
    let next_x = mx + modal_w - next_w - 16.0 - 120.0 - 12.0; // leave space for Close
    let next_hover = mxp >= next_x && mxp <= next_x + next_w && myp >= footer_y && myp <= footer_y + btn_h;
    draw_rectangle(next_x, footer_y, next_w, btn_h, if next_hover { Color::new(0.18, 0.18, 0.18, 1.0) } else { Color::new(0.12, 0.12, 0.12, 0.9) });
    draw_rectangle_lines(next_x, footer_y, next_w, btn_h, 1.0, Color::new(0.6, 0.6, 0.6, 0.8));
    draw_text("Next >", next_x + 22.0, footer_y + btn_h * 0.68, 20.0, WHITE);

    // Close button
    let close_w = 120.0;
    let close_x = mx + modal_w - close_w - 16.0;
    let close_hover = mxp >= close_x && mxp <= close_x + close_w && myp >= footer_y && myp <= footer_y + btn_h;
    draw_rectangle(close_x, footer_y, close_w, btn_h, if close_hover { Color::new(0.18, 0.18, 0.18, 1.0) } else { Color::new(0.12, 0.12, 0.12, 0.9) });
    draw_rectangle_lines(close_x, footer_y, close_w, btn_h, 1.0, Color::new(0.6, 0.6, 0.6, 0.8));
    draw_text("Close", close_x + 34.0, footer_y + btn_h * 0.68, 20.0, WHITE);

    // Handle clicks
    if is_mouse_button_pressed(MouseButton::Left) {
        if prev_hover {
            if *current_idx == 0 { *current_idx = total - 1; } else { *current_idx -= 1; }
        } else if next_hover {
            *current_idx = (*current_idx + 1) % total;
        } else if close_hover {
            *show = false;
        }
    }

    // Keyboard
    if is_key_pressed(KeyCode::Left) { if *current_idx == 0 { *current_idx = total - 1; } else { *current_idx -= 1; } }
    if is_key_pressed(KeyCode::Right) { *current_idx = (*current_idx + 1) % total; }
    if is_key_pressed(KeyCode::Escape) { *show = false; }
}
