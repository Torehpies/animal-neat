use macroquad::prelude::*;

// Re-export types/traits from parent
use crate::params::*;
use crate::Vec2;
use crate::Episode;

pub fn world_to_screen(area: Rect, p: Vec2) -> (f32, f32) {
    let sx = area.x + (p.x / WORLD_W) * area.w;
    let sy = area.y + (p.y / WORLD_H) * area.h;
    (sx, sy)
}

pub fn screen_to_world(area: Rect, sx: f32, sy: f32) -> Vec2 {
    let x = ((sx - area.x) / area.w).clamp(0.0, 1.0) * WORLD_W;
    let y = ((sy - area.y) / area.h).clamp(0.0, 1.0) * WORLD_H;
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
