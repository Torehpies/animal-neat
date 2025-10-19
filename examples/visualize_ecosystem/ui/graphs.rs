use macroquad::prelude::*;
use plotters::prelude::{BitMapBackend, DrawingArea, IntoDrawingArea, Polygon, ShapeStyle, Circle, RGBColor};
use plotters::style::{IntoFont};
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
    pub births_herb: Series,
    pub deaths_herb: Series,
    pub births_carn: Series,
    pub deaths_carn: Series,
    // Intelligence proxy (behavior shaping signals), appended per episode end
    pub intel_best: Series,
    pub intel_mean: Series,
}
impl Trends {
    pub fn new() -> Self { Self::default() }
    pub fn reset_episode(&mut self) {
        self.pop.clear();
        self.species.clear();
        self.births.clear();
        self.deaths.clear();
        self.births_herb.clear();
        self.deaths_herb.clear();
        self.births_carn.clear();
        self.deaths_carn.clear();
    }
}

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
fn draw_population_panel_pie(area: Rect, herb_count: usize, carn_count: usize) {
    // Determine square render size for the pie texture
    let render_side = area.w.min(area.h).clamp(160.0, 512.0) as u32;
    let mut rgb = vec![0u8; (render_side * render_side * 3) as usize];
    // Draw pie with plotters into RGB buffer
    {
        let root: DrawingArea<BitMapBackend<'_>, plotters::coord::Shift> = BitMapBackend::with_buffer(&mut rgb, (render_side, render_side)).into_drawing_area();
        let bg = RGBColor(22, 22, 28);
        let _ = root.fill(&bg);
        let total = herb_count + carn_count;
        let cx = (render_side as i32) / 2;
        let cy = (render_side as i32) / 2;
        let radius = ((render_side as f32) * 0.45) as i32;

        // Helper to draw a filled wedge as a polygon
        fn draw_wedge(area: &DrawingArea<BitMapBackend<'_>, plotters::coord::Shift>,
                      cx: i32, cy: i32, r: i32,
                      start: f32, sweep: f32, color: RGBColor) {
            let segments = (sweep.abs() / std::f32::consts::PI * 64.0).ceil().max(8.0) as i32;
            let mut pts: Vec<(i32, i32)> = Vec::with_capacity((segments + 2) as usize);
            pts.push((cx, cy));
            for i in 0..=segments {
                let t = start + sweep * (i as f32 / segments as f32);
                let x = cx as f32 + r as f32 * t.cos();
                let y = cy as f32 + r as f32 * t.sin();
                pts.push((x.round() as i32, y.round() as i32));
            }
            let style = ShapeStyle::from(&color).filled();
            let _ = area.draw(&Polygon::new(pts, style.clone()));
        }

        if total > 0 {
            let herb_angle = 2.0f32 * std::f32::consts::PI * (herb_count as f32) / (total as f32);
            let start = -std::f32::consts::FRAC_PI_2; // start at top
            // Colors roughly match HUD theme
            let herb_col = RGBColor(80, 200, 120);
            let carn_col = RGBColor(220, 90, 90);
            draw_wedge(&root, cx, cy, radius, start, herb_angle, herb_col);
            draw_wedge(&root, cx, cy, radius, start + herb_angle, 2.0 * std::f32::consts::PI - herb_angle, carn_col);
            // Outline circle
            let _ = root.draw(&Circle::new((cx, cy), radius, ShapeStyle::from(&RGBColor(200,200,210)).stroke_width(1)));
        } else {
            // Empty ring to indicate zero population
            let _ = root.draw(&Circle::new((cx, cy), radius, ShapeStyle::from(&RGBColor(120,120,130)).stroke_width(1)));
        }

        // Labels
        let label_style = ("sans-serif", 16).into_font().color(&RGBColor(235,235,245));
        let txt = format!("Herbivores: {}", herb_count);
        let _ = root.draw(&plotters::prelude::Text::new(txt, (10, 24), label_style.clone()));
        let txt2 = format!("Carnivores: {}", carn_count);
        let _ = root.draw(&plotters::prelude::Text::new(txt2, (10, 48), label_style));
    }

    // Convert RGB buffer to RGBA for macroquad
    let mut rgba = vec![0u8; (render_side * render_side * 4) as usize];
    for i in 0..(render_side * render_side) as usize {
        rgba[i * 4] = rgb[i * 3];
        rgba[i * 4 + 1] = rgb[i * 3 + 1];
        rgba[i * 4 + 2] = rgb[i * 3 + 2];
        rgba[i * 4 + 3] = 255;
    }
    let tex = Texture2D::from_rgba8(render_side as u16, render_side as u16, &rgba);

    // Draw texture centered in area
    let scale = (area.w.min(area.h)) / (render_side as f32);
    let draw_w = (render_side as f32) * scale;
    let draw_h = (render_side as f32) * scale;
    let dx = area.x + (area.w - draw_w) * 0.5;
    let dy = area.y + (area.h - draw_h) * 0.5;
    draw_texture_ex(
        &tex,
        dx,
        dy,
        WHITE,
        DrawTextureParams { dest_size: Some(Vec2::new(draw_w, draw_h)), ..Default::default() },
    );
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
    // Helpers
    fn last(series: &Series) -> f32 { series.data.last().copied().unwrap_or(0.0) }
    fn draw_bars(area: Rect, title: &str, labels: &[&str], values: &[f32], colors: &[Color]) {
        // Panel frame and title
        draw_rectangle_lines(area.x, area.y, area.w, area.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.12));
        draw_text(title, area.x + 6.0, area.y + 18.0, 16.0, LIGHTGRAY);
        let inner = Rect { x: area.x + 8.0, y: area.y + 28.0, w: area.w - 16.0, h: area.h - 36.0 };
        // Compute scale
        let max_v = values.iter().cloned().fold(0.0f32, f32::max).max(1.0);
        let n = values.len().max(1) as f32;
        let gap = 10.0f32;
        let bar_w = ((inner.w - gap * (n + 1.0)) / n).max(4.0);
        for (i, v) in values.iter().enumerate() {
            let x = inner.x + gap + i as f32 * (bar_w + gap);
            let h = if max_v > 0.0 { inner.h * (*v / max_v) } else { 0.0 };
            let y = inner.y + inner.h - h;
            draw_rectangle(x, y, bar_w, h, colors.get(i).copied().unwrap_or(WHITE));
            // label and value
            let lbl = labels.get(i).copied().unwrap_or("");
            let tw = measure_text(lbl, None, 12u16, 1.0).width;
            draw_text(lbl, x + (bar_w - tw) * 0.5, inner.y + inner.h + 12.0, 12.0, GRAY);
            let val_txt = format!("{:.0}", *v);
            let vw = measure_text(&val_txt, None, 12u16, 1.0).width;
            draw_text(&val_txt, x + (bar_w - vw) * 0.5, y - 4.0, 12.0, LIGHTGRAY);
        }
    }

    // Precompute values from the latest sample
    let births_all = last(&trends.births).max(0.0);
    let deaths_all = last(&trends.deaths).max(0.0);
    let births_herb = last(&trends.births_herb).max(0.0);
    let births_carn = last(&trends.births_carn).max(0.0);
    let deaths_herb = last(&trends.deaths_herb).max(0.0);
    let deaths_carn = last(&trends.deaths_carn).max(0.0);

    // Layout: two charts on the top row, three charts on the bottom row
    let gap = GAP;
    let top_h = (area.h - gap) * 0.5;
    let bot_h = area.h - gap - top_h;

    // Top row areas
    let top_w = (area.w - gap) * 0.5;
    let top_left = Rect { x: area.x, y: area.y, w: top_w, h: top_h };
    let top_right = Rect { x: area.x + top_w + gap, y: area.y, w: top_w, h: top_h };

    // Bottom row areas (three columns)
    let bot_col_w = (area.w - gap * 2.0) / 3.0;
    let bot_y = area.y + top_h + gap;
    let bot1 = Rect { x: area.x, y: bot_y, w: bot_col_w, h: bot_h };
    let bot2 = Rect { x: area.x + bot_col_w + gap, y: bot_y, w: bot_col_w, h: bot_h };
    let bot3 = Rect { x: area.x + (bot_col_w + gap) * 2.0, y: bot_y, w: bot_col_w, h: bot_h };

    // Draw top: births comparison, deaths comparison
    draw_bars(
        top_left,
        "Births (episode totals)",
        &["All", "Herb", "Carn"],
        &[births_all, births_herb, births_carn],
        &[Color::new(0.55, 0.95, 0.65, 0.95), Color::new(0.50, 0.85, 0.60, 0.95), Color::new(0.45, 0.80, 0.55, 0.95)],
    );
    draw_bars(
        top_right,
        "Deaths (episode)",
        &["All", "Herb", "Carn"],
        &[deaths_all, deaths_herb, deaths_carn],
        &[Color::new(0.95, 0.55, 0.55, 0.95), Color::new(0.95, 0.65, 0.55, 0.95), Color::new(0.95, 0.45, 0.45, 0.95)],
    );

    // Draw bottom: births vs deaths for each category
    draw_bars(
        bot1,
        "All: Births vs Deaths",
        &["Births", "Deaths"],
        &[births_all, deaths_all],
        &[Color::new(0.55, 0.95, 0.65, 0.95), Color::new(0.95, 0.55, 0.55, 0.95)],
    );
    draw_bars(
        bot2,
        "Herb: Births vs Deaths",
        &["Births", "Deaths"],
        &[births_herb, deaths_herb],
        &[Color::new(0.50, 0.85, 0.60, 0.95), Color::new(0.95, 0.65, 0.55, 0.95)],
    );
    draw_bars(
        bot3,
        "Carn: Births vs Deaths",
        &["Births", "Deaths"],
        &[births_carn, deaths_carn],
        &[Color::new(0.45, 0.80, 0.55, 0.95), Color::new(0.95, 0.45, 0.45, 0.95)],
    );
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
pub fn draw_graphs_overlay(fullscreen: Rect, trends: &Trends, active_tab: &mut GraphTab, herb_carn_counts: (usize, usize)) {
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
        GraphTab::Population => {
            draw_population_panel_pie(content, herb_carn_counts.0, herb_carn_counts.1)
        },
        GraphTab::Fitness => draw_fitness_panel(content, trends),
        GraphTab::BirthsDeaths => draw_births_deaths_panel(content, trends),
        GraphTab::Intelligence => draw_intelligence_panel(content, trends),
    }
    // Close hint
    let hint = "[Z] Close";
    let hw = measure_text(hint, None, 16u16, 1.0).width;
    draw_text(hint, fullscreen.x + fullscreen.w - hw - 20.0, fullscreen.y + fullscreen.h - 18.0, 16.0, LIGHTGRAY);
}
