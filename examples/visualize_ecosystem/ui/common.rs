use macroquad::prelude::*;

// Re-export types/traits from parent
use crate::Vec2;

// --- Lightweight UI theme & helpers ---
// Centralized colors and spacing used across UI panels
pub const PANEL_BG: Color = Color::new(0.08, 0.08, 0.10, 0.95);
pub const PANEL_BORDER: Color = Color::new(0.25, 0.25, 0.30, 1.0);
pub const SUBPANEL_BG: Color = Color::new(0.05, 0.05, 0.07, 0.95);
pub const ACCENT: Color = Color::new(0.30, 0.70, 1.0, 1.0);

pub const PAD: f32 = 12.0;
pub const GAP: f32 = 6.0;
pub const FONT: f32 = 18.0;

pub fn draw_panel(area: Rect, bg: Color, border: Color, thickness: f32) {
    draw_rectangle(area.x, area.y, area.w, area.h, bg);
    if thickness > 0.0 {
        draw_rectangle_lines(area.x, area.y, area.w, area.h, thickness, border);
    }
}

// Draw a section header with a subtle underline. Returns next y.
pub fn section_title(text: &str, x: f32, y: f32, max_w: f32) -> f32 {
    let title_fs = 18.0;
    draw_text_clamped(text, x, y, title_fs, LIGHTGRAY, max_w);
    let dims = measure_text(text, None, title_fs as u16, 1.0);
    let underline_w = dims.width.min(max_w);
    let uy = y + 6.0;
    // subtle accent underline
    let mut c = ACCENT; c.a = 0.25;
    draw_line(x, uy, x + underline_w, uy, 1.0, c);
    uy + GAP
}

// Draw a horizontal divider line. Returns next y.
pub fn draw_divider(x: f32, y: f32, w: f32) -> f32 {
    draw_line(x, y, x + w, y, 1.0, Color::new(1.0, 1.0, 1.0, 0.10));
    y + GAP
}

// Compute a rectangle inside `area` that preserves the world's aspect ratio (WORLD_W:WORLD_H)
pub fn fit_world_rect(area: Rect) -> Rect {
    let world_w = crate::world::get_world_w();
    let world_h = crate::world::get_world_h();
    let world_ar = world_w / world_h;
    let area_ar = area.w / area.h;
    if area_ar >= world_ar {
        // Fit by height
        let h = area.h;
        let w = world_ar * h;
        let x = area.x + (area.w - w) * 0.5;
        Rect { x, y: area.y, w, h }
    } else {
        // Fit by width
        let w = area.w;
        let h = w / world_ar;
        let y = area.y + (area.h - h) * 0.5;
        Rect { x: area.x, y, w, h }
    }
}

// Pixels per one world unit (uniform scale) for a fitted world rect
pub fn world_scale(fitted: Rect) -> f32 { 
    let world_w = crate::world::get_world_w();
    let world_h = crate::world::get_world_h();
    (fitted.w / world_w).min(fitted.h / world_h) 
}

pub fn world_to_screen(area: Rect, p: Vec2) -> (f32, f32) {
    let world_w = crate::world::get_world_w();
    let world_h = crate::world::get_world_h();
    // Assumes `area` is the fitted world rect from fit_world_rect
    let sx = area.x + (p.x / world_w) * area.w;
    let sy = area.y + (p.y / world_h) * area.h;
    (sx, sy)
}

pub fn screen_to_world(area: Rect, sx: f32, sy: f32) -> Vec2 {
    let world_w = crate::world::get_world_w();
    let world_h = crate::world::get_world_h();
    // Assumes `area` is the fitted world rect from fit_world_rect
    let x = ((sx - area.x) / area.w).clamp(0.0, 1.0) * world_w;
    let y = ((sy - area.y) / area.h).clamp(0.0, 1.0) * world_h;
    Vec2 { x, y }
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = (h % 1.0 + 1.0) % 1.0;
    if s <= 0.0 { return (v, v, v); }
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i as i32 % 6 { 0 => (v, t, p), 1 => (q, v, p), 2 => (p, v, t), 3 => (p, q, v), 4 => (t, p, v), _ => (v, p, q) }
}

#[allow(dead_code)]
pub fn species_color(species_idx: usize) -> Color {
    let hue = ((species_idx as f32) * 0.618_033_988) % 1.0; // golden ratio spacing
    let (r, g, b) = hsv_to_rgb(hue, 0.65, 0.95);
    Color::new(r, g, b, 1.0)
}

pub fn draw_text_clamped(text: &str, x: f32, y: f32, font_size: f32, color: Color, max_width: f32) {
    let dims = measure_text(text, None, font_size as u16, 1.0);
    if dims.width <= max_width {
        draw_text(text, x, y, font_size, color);
        return;
    }
    // Estimate a cut with ellipsis
    let total_chars = text.chars().count().max(1) as f32;
    let avg_w = (dims.width / total_chars).max(1.0);
    let mut take = ((max_width - 10.0) / avg_w).floor().max(0.0) as usize;
    if take == 0 { return; }
    let mut s: String = text.chars().take(take).collect();
    s.push('…');
    let dims2 = measure_text(&s, None, font_size as u16, 1.0);
    if dims2.width > max_width && take > 1 {
        take = take.saturating_sub(2);
        s = text.chars().take(take).collect();
        s.push('…');
    }
    draw_text(&s, x, y, font_size, color);
}

// Draw text wrapped to a given max_width. Returns the new y position after drawing.
pub fn draw_text_wrapped(
    text: &str,
    x: f32,
    mut y: f32,
    font_size: f32,
    color: Color,
    max_width: f32,
    line_gap: f32,
) -> f32 {
    // Simple word-wrapping by measuring candidate lines
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return y;
    }
    let mut line = String::new();
    for (i, w) in words.iter().enumerate() {
        let candidate = if line.is_empty() { (*w).to_string() } else { format!("{} {}", line, w) };
        let dims = measure_text(&candidate, None, font_size as u16, 1.0);
        if dims.width <= max_width {
            line = candidate;
        } else {
            // If a single very-long word overflows, clamp it on one line to avoid infinite loop
            if line.is_empty() {
                draw_text_clamped(w, x, y, font_size, color, max_width);
                y += font_size + line_gap;
            } else {
                draw_text(&line, x, y, font_size, color);
                y += font_size + line_gap;
                line = (*w).to_string();
            }
        }
        // Flush at end
        if i == words.len() - 1 && !line.is_empty() {
            draw_text(&line, x, y, font_size, color);
            y += font_size + line_gap;
        }
    }
    y
}
