use macroquad::prelude::*;
use crate::ui_common::{draw_panel, section_title, draw_text_clamped, PANEL_BG, PANEL_BORDER, SUBPANEL_BG, PAD, GAP};
use crate::{AppState, ScoreEntry};

pub fn draw_scoreboard(_fullscreen: Rect, state: &AppState, rows: &[ScoreEntry], modal: bool) -> bool {
    // Returns true if user clicked Continue
    let w = screen_width();
    let h = screen_height();
    // Optional backdrop for modal
    if modal {
        let mut dim = BLACK; dim.a = 0.6;
        draw_rectangle(0.0, 0.0, w, h, dim);
    }

    // Panel size
    let panel_w = (w * 0.64).clamp(720.0, 1100.0);
    let panel_h = (h * 0.64).clamp(420.0, 720.0);
    let panel_x = (w - panel_w) * 0.5;
    let panel_y = (h - panel_h) * 0.5;
    let panel = Rect { x: panel_x, y: panel_y, w: panel_w, h: panel_h };
    draw_panel(panel, PANEL_BG, PANEL_BORDER, 3.0);

    let mut y = panel.y + PAD;
    let x = panel.x + PAD;
    let max_w = panel.w - PAD * 2.0;

    // Header
    let title = if crate::params::ECO_CONTINUOUS {
        format!("Episode {} results", state.eco_episode_counter)
    } else {
        format!("Generation {} results", state.generation)
    };
    y = section_title(&title, x, y, max_w);

    // Columns header
    let header_fs = 16.0;
    let cols = [
        ("#", 42.0),
        ("Agent", 70.0),
        ("Species", 90.0),
        ("Score", 110.0),
        ("Plants", 80.0),
        ("Meat", 70.0),
        ("Births", 70.0),
        ("Alive", 80.0),
        ("Atk", 60.0),
        ("Kills", 70.0),
        ("AvgE", 80.0),
    ];
    let mut cx = x;
    for (name, cw) in cols.iter() {
        draw_text_clamped(name, cx, y, header_fs, LIGHTGRAY, *cw - 8.0);
        cx += *cw;
    }
    y += header_fs + GAP + 2.0;
    draw_line(x, y, x + max_w, y, 1.0, Color::new(1.0,1.0,1.0,0.1));
    y += 8.0;

    // Scroll area if many rows
    let row_h = 20.0;
    let avail_h = (panel.y + panel.h - PAD - 60.0) - y;
    let max_rows = ((avail_h / row_h).floor() as i32).max(1) as usize;
    let total = rows.len();

    // Simple scroll via mouse wheel when hovered
    let mx = mouse_position().0; let my = mouse_position().1;
    let hovered = mx >= panel.x && mx <= panel.x + panel.w && my >= y && my <= panel.y + panel.h - 60.0;
    static mut OFFSET: i32 = 0;
    let scroll = mouse_wheel().1; // y-axis scroll
    unsafe {
        if hovered {
            if scroll > 0.0 { OFFSET = (OFFSET - 3).max(0); }
            if scroll < 0.0 { OFFSET = (OFFSET + 3).min((total.saturating_sub(max_rows)) as i32); }
        }
        let start = OFFSET as usize;
        let end = (start + max_rows).min(total);
        for (rank, row) in rows.iter().enumerate().take(end).skip(start) {
            let mut cx = x;
            let fs = 16.0;
            let yrow = y + ((rank - start) as f32) * row_h;
            let color = if rank == 0 { Color::new(1.0, 0.9, 0.5, 1.0) } else { WHITE };
            draw_text_clamped(&format!("{}", rank+1), cx, yrow, fs, color, cols[0].1 - 8.0); cx += cols[0].1;
            draw_text_clamped(&format!("{}", row.idx), cx, yrow, fs, color, cols[1].1 - 8.0); cx += cols[1].1;
            draw_text_clamped(&format!("{}", row.species), cx, yrow, fs, color, cols[2].1 - 8.0); cx += cols[2].1;
            draw_text_clamped(&format!("{:.2}", row.score), cx, yrow, fs, color, cols[3].1 - 8.0); cx += cols[3].1;
            draw_text_clamped(&format!("{}", row.eaten_plants), cx, yrow, fs, color, cols[4].1 - 8.0); cx += cols[4].1;
            draw_text_clamped(&format!("{}", row.eaten_meat), cx, yrow, fs, color, cols[5].1 - 8.0); cx += cols[5].1;
            draw_text_clamped(&format!("{}", row.offspring), cx, yrow, fs, color, cols[6].1 - 8.0); cx += cols[6].1;
            draw_text_clamped(&format!("{}", row.alive_steps), cx, yrow, fs, color, cols[7].1 - 8.0); cx += cols[7].1;
            draw_text_clamped(&format!("{}", row.attack_hits), cx, yrow, fs, color, cols[8].1 - 8.0); cx += cols[8].1;
            draw_text_clamped(&format!("{}", row.kills_caused), cx, yrow, fs, color, cols[9].1 - 8.0); cx += cols[9].1;
            draw_text_clamped(&format!("{:.2}", row.avg_energy_norm), cx, yrow, fs, color, cols[10].1 - 8.0); cx += cols[10].1;
        }
    }

    // Footer: buttons
    let btn_h = 40.0;
    let btn_w = 160.0;
    let gap = 16.0;
    let total_w = btn_w * 2.0 + gap;
    let bx = panel.x + (panel.w - total_w) * 0.5;
    let by = panel.y + panel.h - PAD - btn_h;

    // Close toggle button (disabled in modal)
    let close_hover = point_in_rect(mx, my, bx, by, btn_w, btn_h);
    let close_label = if modal { "Close" } else { "Close [T]" };
    draw_button(bx, by, btn_w, btn_h, close_label, close_hover);
    let mut continue_clicked = false;

    // Continue only when modal/pending
    if modal {
        let bx2 = bx + btn_w + gap;
        let cont_hover = point_in_rect(mx, my, bx2, by, btn_w, btn_h);
        draw_button(bx2, by, btn_w, btn_h, "Continue", cont_hover);
        if cont_hover && is_mouse_button_pressed(MouseButton::Left) {
            continue_clicked = true;
        }
    }

    if close_hover && is_mouse_button_pressed(MouseButton::Left) && !modal {
        // Caller owns the toggle flag; here we just rely on [T] key documented on the label.
    }

    continue_clicked
}

fn point_in_rect(mx: f32, my: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

fn draw_button(x: f32, y: f32, w: f32, h: f32, label: &str, hover: bool) {
    let bg = if hover { Color::new(0.3, 0.7, 0.9, 1.0) } else { Color::new(0.2, 0.5, 0.7, 1.0) };
    draw_rectangle(x, y, w, h, bg);
    draw_rectangle_lines(x, y, w, h, 2.0, WHITE);
    let fs = 20.0;
    let tw = measure_text(label, None, fs as u16, 1.0).width;
    draw_text(label, x + (w - tw) * 0.5, y + fs + 6.0, fs, WHITE);
}
