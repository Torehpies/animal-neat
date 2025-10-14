use macroquad::prelude::*;
use crate::ui_common::{draw_panel, PAD, GAP, PANEL_BORDER, SUBPANEL_BG};

// Simple fixed-capacity ring buffer time series
pub struct Series {
    buf: Vec<f32>,
    head: usize,
    len: usize,
}

impl Series {
    pub fn with_capacity(cap: usize) -> Self { Self { buf: vec![0.0; cap.max(1)], head: 0, len: 0 } }
    pub fn push(&mut self, v: f32) {
        if self.buf.is_empty() { return; }
        self.buf[self.head] = v;
        self.head = (self.head + 1) % self.buf.len();
        self.len = (self.len + 1).min(self.buf.len());
    }
    pub fn iter_recent(&self) -> impl Iterator<Item=f32> + '_ {
        let n = self.len;
        let cap = self.buf.len();
        (0..n).map(move |i| self.buf[(self.head + cap + i - n) % cap])
    }
    pub fn min_max(&self) -> (f32, f32) {
        let mut min = f32::INFINITY; let mut max = f32::NEG_INFINITY;
        for v in self.iter_recent() { min = min.min(v); max = max.max(v); }
        if min == f32::INFINITY { (0.0, 1.0) } else { (min, if max>min { max } else { min+1.0 }) }
    }
}

pub struct Trends {
    pub pop: Series,
    pub species: Series,
    pub best: Series,
    pub mean: Series,
    pub births: Series,
    pub deaths: Series,
}

impl Trends {
    pub fn new(cap: usize) -> Self {
        Self {
            pop: Series::with_capacity(cap),
            species: Series::with_capacity(cap),
            best: Series::with_capacity(cap),
            mean: Series::with_capacity(cap),
            births: Series::with_capacity(cap),
            deaths: Series::with_capacity(cap),
        }
    }
}

fn draw_axes(area: Rect) {
    let g = Color::new(1.0, 1.0, 1.0, 0.08);
    for i in 0..=4 { let y = area.y + area.h * (i as f32) / 4.0; draw_line(area.x, y, area.x + area.w, y, 1.0, g); }
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.15));
}

fn draw_line_series(area: Rect, series: &Series, col: Color) {
    let n = series.len.max(2) as f32;
    let (mn, mx) = series.min_max();
    let range = (mx - mn).max(1e-5);
    let mut prev: Option<(f32, f32)> = None;
    let mut i = 0f32;
    for v in series.iter_recent() {
        let t = i / (n - 1.0);
        let x = area.x + t * area.w;
        let y = area.y + area.h * (1.0 - (v - mn) / range);
        if let Some((px, py)) = prev { draw_line(px, py, x, y, 1.5, col); }
        prev = Some((x, y));
        i += 1.0;
    }
}

pub fn draw_graphs_panel(area: Rect, trends: &Trends) {
    let frame = Rect { x: area.x, y: area.y, w: area.w, h: area.h };
    draw_panel(frame, SUBPANEL_BG, PANEL_BORDER, 2.0);
    let inner = Rect { x: frame.x + PAD*0.5, y: frame.y + PAD*0.5, w: frame.w - PAD, h: frame.h - PAD };

    // Split into three rows
    let row_h = (inner.h - GAP*2.0) / 3.0;
    let r = |i: i32| Rect { x: inner.x, y: inner.y + (row_h + GAP) * i as f32, w: inner.w, h: row_h };

    // Row 1: population and species
    let a1 = r(0);
    draw_axes(a1);
    draw_line_series(a1, &trends.pop, Color::new(0.3, 0.7, 1.0, 0.9));
    draw_line_series(a1, &trends.species, Color::new(0.9, 0.7, 0.3, 0.9));
    draw_text("Pop/Species", a1.x + 6.0, a1.y + 14.0, 14.0, LIGHTGRAY);

    // Row 2: best and mean fitness
    let a2 = r(1);
    draw_axes(a2);
    draw_line_series(a2, &trends.best, Color::new(0.6, 1.0, 0.6, 0.9));
    draw_line_series(a2, &trends.mean, Color::new(0.8, 0.8, 0.9, 0.9));
    draw_text("Best/Mean fitness", a2.x + 6.0, a2.y + 14.0, 14.0, LIGHTGRAY);

    // Row 3: births and deaths
    let a3 = r(2);
    draw_axes(a3);
    draw_line_series(a3, &trends.births, Color::new(0.6, 0.9, 0.6, 0.9));
    draw_line_series(a3, &trends.deaths, Color::new(0.95, 0.5, 0.5, 0.9));
    draw_text("Births/Deaths", a3.x + 6.0, a3.y + 14.0, 14.0, LIGHTGRAY);
}
