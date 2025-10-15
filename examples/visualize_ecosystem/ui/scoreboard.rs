use macroquad::prelude::*;
use crate::ui_common::{draw_panel, section_title, draw_text_clamped, PANEL_BG, PANEL_BORDER, PAD, GAP};
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

    // Panel size (enlarged to fit more columns)
    let panel_w = (w * 0.82).clamp(900.0, 1600.0);
    let panel_h = (h * 0.78).clamp(520.0, 900.0);
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
    let header_fs = 15.0;
    let cols = [
        ("#", 36.0),
        ("Idx", 56.0),
        ("Sp", 56.0),
        ("Scr", 100.0),
        ("Plt", 64.0),
        ("Meat", 64.0),
        ("Off", 60.0),
        ("Age", 70.0),
        ("Idle", 84.0),
        ("Atk", 56.0),
        ("K", 48.0),
        ("AvgE", 64.0),
        ("Herd", 70.0),
        ("Appr", 70.0),
        ("Chs", 70.0),
    ];
    // Fit columns within available width by uniform scaling
    let base_sum: f32 = cols.iter().map(|(_, w)| *w).sum();
    let scale = if base_sum > max_w { (max_w / base_sum).clamp(0.4, 1.0) } else { 1.0 };
    let cw: Vec<f32> = cols.iter().map(|(_, w)| (*w) * scale).collect();
    let mut cx = x;
    for (i, (name, _)) in cols.iter().enumerate() {
        let w = cw[i];
        draw_text_clamped(name, cx, y, header_fs, LIGHTGRAY, w - 8.0);
        cx += w;
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
            draw_text_clamped(&format!("{}", rank+1), cx, yrow, fs, color, cw[0] - 8.0); cx += cw[0];
            draw_text_clamped(&format!("{}", row.idx), cx, yrow, fs, color, cw[1] - 8.0); cx += cw[1];
            draw_text_clamped(&format!("{}", row.species), cx, yrow, fs, color, cw[2] - 8.0); cx += cw[2];
            draw_text_clamped(&format!("{:.2}", row.score), cx, yrow, fs, color, cw[3] - 8.0); cx += cw[3];
            draw_text_clamped(&format!("{}", row.eaten_plants), cx, yrow, fs, color, cw[4] - 8.0); cx += cw[4];
            draw_text_clamped(&format!("{}", row.eaten_meat), cx, yrow, fs, color, cw[5] - 8.0); cx += cw[5];
            draw_text_clamped(&format!("{}", row.offspring), cx, yrow, fs, color, cw[6] - 8.0); cx += cw[6];
            draw_text_clamped(&format!("{}", row.alive_steps), cx, yrow, fs, color, cw[7] - 8.0); cx += cw[7];
            draw_text_clamped(&format!("-{:.2}", row.idle_penalty_value), cx, yrow, fs, color, cw[8] - 8.0); cx += cw[8];
            draw_text_clamped(&format!("{}", row.attack_hits), cx, yrow, fs, color, cw[9] - 8.0); cx += cw[9];
            draw_text_clamped(&format!("{}", row.kills_caused), cx, yrow, fs, color, cw[10] - 8.0); cx += cw[10];
            draw_text_clamped(&format!("{:.2}", row.avg_energy_norm), cx, yrow, fs, color, cw[11] - 8.0); cx += cw[11];
            draw_text_clamped(&format!("{:.2}", row.herd_value), cx, yrow, fs, color, cw[12] - 8.0); cx += cw[12];
            draw_text_clamped(&format!("{:.2}", row.approach_value), cx, yrow, fs, color, cw[13] - 8.0); cx += cw[13];
            draw_text_clamped(&format!("{:.2}", row.chase_value), cx, yrow, fs, color, cw[14] - 8.0);
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
        draw_button(bx2, by, btn_w, btn_h, "Continue [C]", cont_hover);
        if cont_hover && is_mouse_button_pressed(MouseButton::Left) {
            continue_clicked = true;
        }
        // Also show a subtle hint near the button area
        let hint_fs = 14.0;
        let hint = "Press C to Continue";
        let tw = measure_text(hint, None, hint_fs as u16, 1.0).width;
        draw_text(hint, bx2 + (btn_w - tw) * 0.5, by - 8.0, hint_fs, LIGHTGRAY);
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
