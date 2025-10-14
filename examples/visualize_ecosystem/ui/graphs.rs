use macroquad::prelude::*;
use crate::ui_common::{draw_panel, PAD, GAP, PANEL_BORDER, SUBPANEL_BG};

// Episode-length time series (grow for entire episode, then reset)
#[derive(Default)]
pub struct Series { pub data: Vec<f32> }
impl Series {
    pub fn push(&mut self, v: f32) { self.data.push(v); }
    pub fn clear(&mut self) { self.data.clear(); }
    pub fn min_max(&self) -> (f32, f32) {
        if self.data.is_empty() { return (0.0, 1.0); }
        let mut min = f32::INFINITY; let mut max = f32::NEG_INFINITY;
        for &v in &self.data { if v < min { min = v; } if v > max { max = v; } }
        if max <= min { (min, min + 1.0) } else { (min, max) }
    }
}

#[derive(Default)]
pub struct Trends {
    pub pop: Series,
    pub species: Series,
    pub best: Series,   // appended per generation/episode end
    pub mean: Series,   // appended per generation/episode end
    pub births: Series,
    pub deaths: Series,
}
impl Trends { pub fn new() -> Self { Self::default() } pub fn reset_episode(&mut self) { self.pop.clear(); self.species.clear(); self.births.clear(); self.deaths.clear(); } }

fn draw_axes(area: Rect) {
    let g = Color::new(1.0, 1.0, 1.0, 0.08);
    for i in 0..=4 { let y = area.y + area.h * (i as f32) / 4.0; draw_line(area.x, y, area.x + area.w, y, 1.0, g); }
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.15));
}

fn draw_line_series(area: Rect, series: &Series, col: Color) {
    let n = series.data.len(); if n < 2 { return; }
    let (mn, mx) = series.min_max(); let range = (mx - mn).max(1e-5);
    // Decimate to panel pixel width (one sample per x column using min/max envelope simplified to polyline)
    let cols = area.w.max(1.0) as usize;
    if n <= cols {
        let mut prev: Option<(f32,f32)> = None;
        for (i,&v) in series.data.iter().enumerate() {
            let t = i as f32 / (n as f32 - 1.0);
            let x = area.x + t * area.w;
            let y = area.y + area.h * (1.0 - (v - mn)/range);
            if let Some((px,py)) = prev { draw_line(px,py,x,y,1.3,col); }
            prev = Some((x,y));
        }
    } else {
        let bucket = n as f32 / cols as f32;
        let mut px = None;
        for ci in 0..cols {
            let start = (ci as f32 * bucket).floor() as usize;
            let end = (((ci+1) as f32 * bucket).ceil() as usize).min(n);
            if start >= end { continue; }
            let mut minv = f32::INFINITY; let mut maxv = f32::NEG_INFINITY;
            for &v in &series.data[start..end] { if v < minv { minv = v; } if v > maxv { maxv = v; } }
            let t = ci as f32 / (cols as f32 - 1.0);
            let x = area.x + t * area.w;
            let y1 = area.y + area.h * (1.0 - (minv - mn)/range);
            let y2 = area.y + area.h * (1.0 - (maxv - mn)/range);
            // vertical range for this column
            draw_line(x, y1, x, y2, 1.0, col);
            if let Some((px_prev, py_prev)) = px { draw_line(px_prev, py_prev, x, (y1+y2)*0.5, 1.0, col); }
            px = Some((x, (y1+y2)*0.5));
        }
    }
}

pub fn draw_graphs_panel(area: Rect, trends: &Trends) {
    let frame = Rect { x: area.x, y: area.y, w: area.w, h: area.h };
    draw_panel(frame, SUBPANEL_BG, PANEL_BORDER, 2.0);
    let inner = Rect { x: frame.x + PAD*0.5, y: frame.y + PAD*0.5, w: frame.w - PAD, h: frame.h - PAD };

    // Split into three rows
    let row_h = (inner.h - GAP*2.0) / 3.0;
    let r = |i: i32| Rect { x: inner.x, y: inner.y + (row_h + GAP) * i as f32, w: inner.w, h: row_h };

    // Helper to draw legend entries
    fn legend_entry(x: f32, y: f32, text: &str, col: Color) -> f32 {
        let box_w = 10.0; let box_h = 6.0;
        draw_rectangle(x, y - box_h + 2.0, box_w, box_h, col);
        draw_text(text, x + box_w + 6.0, y + 2.0, 12.0, LIGHTGRAY);
        y + 14.0
    }

    // Row 1: population and species (per-episode)
    let a1 = r(0);
    draw_axes(a1);
    draw_line_series(a1, &trends.pop, Color::new(0.3, 0.7, 1.0, 0.9));
    draw_line_series(a1, &trends.species, Color::new(0.9, 0.7, 0.3, 0.9));
    draw_text("Population / Species (per-episode)", a1.x + 6.0, a1.y + 14.0, 14.0, LIGHTGRAY);
    // Legend
    let mut _ly = a1.y + 30.0;
    _ly = legend_entry(a1.x + 6.0, _ly, "Population", Color::new(0.3, 0.7, 1.0, 0.9));
    _ly = legend_entry(a1.x + 6.0, _ly, "Species", Color::new(0.9, 0.7, 0.3, 0.9));

    // Row 2: best and mean fitness (cumulative/global)
    let a2 = r(1);
    draw_axes(a2);
    draw_line_series(a2, &trends.best, Color::new(0.6, 1.0, 0.6, 0.9));
    draw_line_series(a2, &trends.mean, Color::new(0.8, 0.8, 0.9, 0.9));
    draw_text("Fitness (best / mean across runs)", a2.x + 6.0, a2.y + 14.0, 14.0, LIGHTGRAY);
    let mut _ly2 = a2.y + 30.0;
    _ly2 = legend_entry(a2.x + 6.0, _ly2, "Best", Color::new(0.6, 1.0, 0.6, 0.9));
    _ly2 = legend_entry(a2.x + 6.0, _ly2, "Mean", Color::new(0.8, 0.8, 0.9, 0.9));

    // Row 3: births and deaths (per-episode)
    let a3 = r(2);
    draw_axes(a3);
    draw_line_series(a3, &trends.births, Color::new(0.6, 0.9, 0.6, 0.9));
    draw_line_series(a3, &trends.deaths, Color::new(0.95, 0.5, 0.5, 0.9));
    draw_text("Births / Deaths (per-episode)", a3.x + 6.0, a3.y + 14.0, 14.0, LIGHTGRAY);
    let mut _ly3 = a3.y + 30.0;
    _ly3 = legend_entry(a3.x + 6.0, _ly3, "Births", Color::new(0.6, 0.9, 0.6, 0.9));
    _ly3 = legend_entry(a3.x + 6.0, _ly3, "Deaths", Color::new(0.95, 0.5, 0.5, 0.9));
}
