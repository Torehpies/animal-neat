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
    // Intelligence proxy (behavior shaping signals), appended per episode end
    pub intel_best: Series,
    pub intel_mean: Series,
}
impl Trends { pub fn new() -> Self { Self::default() } pub fn reset_episode(&mut self) { self.pop.clear(); self.species.clear(); self.births.clear(); self.deaths.clear(); } }

// Tabs for the graphs overlay
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum GraphTab {
    Population,
    Fitness,
    BirthsDeaths,
    Intelligence,
}
impl GraphTab { pub fn all() -> &'static [GraphTab] { &[GraphTab::Population, GraphTab::Fitness, GraphTab::BirthsDeaths, GraphTab::Intelligence] }
    pub fn label(&self) -> &'static str {
        match self {
            GraphTab::Population => "Population",
            GraphTab::Fitness => "Fitness",
            GraphTab::BirthsDeaths => "Births/Deaths",
            GraphTab::Intelligence => "Intelligence",
        }
    }
}

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

    // Split into four rows
    let row_h = (inner.h - GAP*3.0) / 4.0;
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

    // Row 4: intelligence proxy (per-episode)
    let a4 = r(3);
    draw_axes(a4);
    draw_line_series(a4, &trends.intel_best, Color::new(0.5, 0.9, 1.0, 0.95));
    draw_line_series(a4, &trends.intel_mean, Color::new(0.7, 0.8, 1.0, 0.9));
    draw_text("Intelligence proxy (best / mean)", a4.x + 6.0, a4.y + 14.0, 14.0, LIGHTGRAY);
    let mut _ly4 = a4.y + 30.0;
    _ly4 = legend_entry(a4.x + 6.0, _ly4, "Best (proxy)", Color::new(0.5, 0.9, 1.0, 0.95));
    _ly4 = legend_entry(a4.x + 6.0, _ly4, "Mean (proxy)", Color::new(0.7, 0.8, 1.0, 0.9));
}

// Draw a full-area population panel (time-series of population & species)
fn draw_population_panel(area: Rect, trends: &Trends) {
    draw_axes(area);
    draw_line_series(area, &trends.pop, Color::new(0.3, 0.7, 1.0, 0.95));
    draw_line_series(area, &trends.species, Color::new(0.9, 0.7, 0.3, 0.95));
    draw_text("Population / Species (per-episode)", area.x + 6.0, area.y + 18.0, 16.0, LIGHTGRAY);
    let mut ly = area.y + 36.0;
    fn legend_entry(x: f32, y: f32, text: &str, col: Color) -> f32 {
        let box_w = 12.0; let box_h = 8.0;
        draw_rectangle(x, y - box_h + 3.0, box_w, box_h, col);
        draw_text(text, x + box_w + 6.0, y + 3.0, 14.0, LIGHTGRAY);
        y + 16.0
    }
    ly = legend_entry(area.x + 6.0, ly, "Population", Color::new(0.3, 0.7, 1.0, 0.95));
    ly = legend_entry(area.x + 6.0, ly, "Species", Color::new(0.9, 0.7, 0.3, 0.95));
}

fn draw_fitness_panel(area: Rect, trends: &Trends) {
    draw_axes(area);
    draw_line_series(area, &trends.best, Color::new(0.6, 1.0, 0.6, 0.95));
    draw_line_series(area, &trends.mean, Color::new(0.8, 0.8, 0.9, 0.95));
    draw_text("Fitness (best / mean across runs)", area.x + 6.0, area.y + 18.0, 16.0, LIGHTGRAY);
    let mut ly = area.y + 36.0;
    fn legend_entry(x: f32, y: f32, text: &str, col: Color) -> f32 {
        let box_w = 12.0; let box_h = 8.0;
        draw_rectangle(x, y - box_h + 3.0, box_w, box_h, col);
        draw_text(text, x + box_w + 6.0, y + 3.0, 14.0, LIGHTGRAY);
        y + 16.0
    }
    ly = legend_entry(area.x + 6.0, ly, "Best", Color::new(0.6, 1.0, 0.6, 0.95));
    ly = legend_entry(area.x + 6.0, ly, "Mean", Color::new(0.8, 0.8, 0.9, 0.95));
}

fn draw_births_deaths_panel(area: Rect, trends: &Trends) {
    draw_axes(area);
    draw_line_series(area, &trends.births, Color::new(0.6, 0.9, 0.6, 0.95));
    draw_line_series(area, &trends.deaths, Color::new(0.95, 0.5, 0.5, 0.95));
    draw_text("Births / Deaths (per-episode)", area.x + 6.0, area.y + 18.0, 16.0, LIGHTGRAY);
    let mut ly = area.y + 36.0;
    fn legend_entry(x: f32, y: f32, text: &str, col: Color) -> f32 {
        let box_w = 12.0; let box_h = 8.0;
        draw_rectangle(x, y - box_h + 3.0, box_w, box_h, col);
        draw_text(text, x + box_w + 6.0, y + 3.0, 14.0, LIGHTGRAY);
        y + 16.0
    }
    ly = legend_entry(area.x + 6.0, ly, "Births", Color::new(0.6, 0.9, 0.6, 0.95));
    ly = legend_entry(area.x + 6.0, ly, "Deaths", Color::new(0.95, 0.5, 0.5, 0.95));
}

fn draw_intelligence_panel(area: Rect, trends: &Trends) {
    draw_axes(area);
    draw_line_series(area, &trends.intel_best, Color::new(0.5, 0.9, 1.0, 0.95));
    draw_line_series(area, &trends.intel_mean, Color::new(0.7, 0.8, 1.0, 0.95));
    draw_text("Intelligence proxy (best / mean)", area.x + 6.0, area.y + 18.0, 16.0, LIGHTGRAY);
    let mut ly = area.y + 36.0;
    fn legend_entry(x: f32, y: f32, text: &str, col: Color) -> f32 {
        let box_w = 12.0; let box_h = 8.0;
        draw_rectangle(x, y - box_h + 3.0, box_w, box_h, col);
        draw_text(text, x + box_w + 6.0, y + 3.0, 14.0, LIGHTGRAY);
        y + 16.0
    }
    ly = legend_entry(area.x + 6.0, ly, "Best (proxy)", Color::new(0.5, 0.9, 1.0, 0.95));
    ly = legend_entry(area.x + 6.0, ly, "Mean (proxy)", Color::new(0.7, 0.8, 1.0, 0.95));
}

/// Draw a modal/fullscreen overlay with the graphs panel centered and a dim background.
pub fn draw_graphs_overlay(fullscreen: Rect, trends: &Trends, active_tab: &mut GraphTab) {
    // Dim background
    draw_rectangle(fullscreen.x, fullscreen.y, fullscreen.w, fullscreen.h, Color::new(0.0, 0.0, 0.0, 0.6));
    // Panel size relative to screen
    let panel_w = (fullscreen.w * 0.68).clamp(640.0f32, fullscreen.w - 80.0f32);
    let panel_h = (fullscreen.h * 0.72).clamp(420.0f32, fullscreen.h - 120.0f32);
    let panel_x = fullscreen.x + (fullscreen.w - panel_w) * 0.5;
    let panel_y = fullscreen.y + (fullscreen.h - panel_h) * 0.5;
    let area = Rect { x: panel_x, y: panel_y, w: panel_w, h: panel_h };
    // Panel frame
    draw_panel(area, SUBPANEL_BG, PANEL_BORDER, 2.0);
    let inner = Rect { x: area.x + PAD*0.5, y: area.y + PAD*0.5, w: area.w - PAD, h: area.h - PAD };

    // Draw tab bar
    let tabs = [GraphTab::Population, GraphTab::Fitness, GraphTab::BirthsDeaths, GraphTab::Intelligence];
    let tab_h = 36.0f32;
    let tab_y = inner.y;
    let tab_w = inner.w / tabs.len() as f32;
    let (mx, my) = mouse_position();
    let mouse = Vec2::new(mx, my);
    for (i, t) in tabs.iter().enumerate() {
        let tx = inner.x + i as f32 * tab_w;
        let tr = Rect { x: tx, y: tab_y, w: tab_w, h: tab_h };
        // Background for active/hover
        let is_active = *active_tab == *t;
        let hover = mouse.x >= tr.x && mouse.x <= tr.x + tr.w && mouse.y >= tr.y && mouse.y <= tr.y + tr.h;
        let bg = if is_active { Color::new(0.2, 0.25, 0.3, 0.95) } else if hover { Color::new(0.12, 0.12, 0.12, 0.7) } else { Color::new(0.08, 0.08, 0.08, 0.55) };
        draw_rectangle(tr.x, tr.y, tr.w, tr.h, bg);
        // Label
        let label = t.label();
        let tw = measure_text(label, None, 16u16, 1.0).width;
        draw_text(label, tr.x + (tr.w - tw) * 0.5, tr.y + tr.h * 0.66, 16.0, LIGHTGRAY);
        // Click
        if is_mouse_button_pressed(MouseButton::Left) && hover {
            *active_tab = *t;
        }
    }

    // Content area below tabs
    let content = Rect { x: inner.x, y: inner.y + tab_h + GAP, w: inner.w, h: inner.h - tab_h - GAP };
    // Render selected tab into content
    match active_tab {
        GraphTab::Population => draw_population_panel(content, trends),
        GraphTab::Fitness => draw_fitness_panel(content, trends),
        GraphTab::BirthsDeaths => draw_births_deaths_panel(content, trends),
        GraphTab::Intelligence => draw_intelligence_panel(content, trends),
    }
    // Close hint
    let hint = "[Z] Close";
    let hw = measure_text(hint, None, 16u16, 1.0).width;
    draw_text(hint, fullscreen.x + fullscreen.w - hw - 20.0, fullscreen.y + fullscreen.h - 18.0, 16.0, LIGHTGRAY);
}
