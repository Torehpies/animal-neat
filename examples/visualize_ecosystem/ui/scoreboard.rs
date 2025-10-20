use macroquad::prelude::*;
use crate::ui_common::{draw_panel, section_title, draw_text_clamped, PANEL_BG, PANEL_BORDER, PAD, GAP};
use crate::scoreboard;
use crate::AppState;
use std::fs;
use std::cell::RefCell;

thread_local! {
    static EXPORT_STATE: RefCell<(Option<String>, f32)> = RefCell::new((None, 0.0));
}

pub fn draw_scoreboard(_fullscreen: Rect, state: &AppState, rows: &[scoreboard::ScoreEntry], modal: bool) -> bool {
    // Returns true if user clicked Continue
    let w = screen_width();
    let h = screen_height();
    if modal {
        let mut dim = BLACK; dim.a = 0.6;
        draw_rectangle(0.0, 0.0, w, h, dim);
    }

    // Panel
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

    // Columns header (clickable for sorting)
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
        ("ChsS", 70.0),
    ];
    let base_sum: f32 = cols.iter().map(|(_, w)| *w).sum();
    let scale = if base_sum > max_w { (max_w / base_sum).clamp(0.4, 1.0) } else { 1.0 };
    let cw: Vec<f32> = cols.iter().map(|(_, w)| (*w) * scale).collect();

    static mut SORT_COL: i32 = 3; // default sort: Score
    static mut SORT_ASC: bool = false; // desc for score
    let (mx, my) = mouse_position();
    let mut cx = x;
    for (i, (name, _)) in cols.iter().enumerate() {
        let w = cw[i];
        let header_hover = mx >= cx && mx <= cx + w && my >= y && my <= y + header_fs + 8.0;
        let mut label = (*name).to_string();
        unsafe {
            if SORT_COL == i as i32 { label.push_str(if SORT_ASC { " ↑" } else { " ↓" }); }
        }
        let color = if header_hover { Color::new(0.85, 0.9, 1.0, 1.0) } else { LIGHTGRAY };
        draw_text_clamped(&label, cx, y, header_fs, color, w - 8.0);
        if header_hover && is_mouse_button_pressed(MouseButton::Left) {
            unsafe {
                if SORT_COL == i as i32 { SORT_ASC = !SORT_ASC; }
                else { SORT_COL = i as i32; SORT_ASC = if i == 3 { false } else { true }; }
            }
        }
        cx += w;
    }
    y += header_fs + GAP + 2.0;
    draw_line(x, y, x + max_w, y, 1.0, Color::new(1.0, 1.0, 1.0, 0.1));
    y += 8.0;

    // Rows / scrolling
    let row_h = 20.0;
    let footer_h = 60.0;
    let avail_h = (panel.y + panel.h - PAD - footer_h) - y;
    let max_rows = ((avail_h / row_h).floor() as i32).max(1) as usize;
    let total = rows.len();

    static mut OFFSET: i32 = 0;
    let scroll = mouse_wheel().1;
    let hovered = mx >= panel.x && mx <= panel.x + panel.w && my >= y && my <= panel.y + panel.h - footer_h;
    unsafe {
        if hovered {
            if scroll > 0.0 { OFFSET = (OFFSET - 3).max(0); }
            if scroll < 0.0 { OFFSET = (OFFSET + 3).min((total.saturating_sub(max_rows)) as i32); }
        }
        if is_key_pressed(KeyCode::Up) { OFFSET = (OFFSET - 1).max(0); }
        if is_key_pressed(KeyCode::Down) { OFFSET = (OFFSET + 1).min((total.saturating_sub(max_rows)) as i32); }
        if is_key_pressed(KeyCode::PageUp) { OFFSET = (OFFSET - max_rows as i32).max(0); }
        if is_key_pressed(KeyCode::PageDown) { OFFSET = (OFFSET + max_rows as i32).min((total.saturating_sub(max_rows)) as i32); }
    }

    // Sorted order
    let mut idxs: Vec<usize> = (0..rows.len()).collect();
    let (sort_col, asc) = unsafe { (SORT_COL, SORT_ASC) };
    idxs.sort_by(|&a, &b| {
        let ca = &rows[a];
        let cb = &rows[b];
        let ord = match sort_col {
            0 => a.cmp(&b),
            1 => ca.idx.cmp(&cb.idx),
            2 => ca.species.cmp(&cb.species),
            3 => ca.score.partial_cmp(&cb.score).unwrap_or(std::cmp::Ordering::Equal),
            4 => ca.eaten_plants.cmp(&cb.eaten_plants),
            5 => ca.eaten_meat.cmp(&cb.eaten_meat),
            6 => ca.offspring.cmp(&cb.offspring),
            7 => ca.alive_steps.cmp(&cb.alive_steps),
            8 => ca.idle_penalty_value.partial_cmp(&cb.idle_penalty_value).unwrap_or(std::cmp::Ordering::Equal),
            9 => ca.attack_hits.cmp(&cb.attack_hits),
            10 => ca.kills_caused.cmp(&cb.kills_caused),
            11 => ca.avg_energy_norm.partial_cmp(&cb.avg_energy_norm).unwrap_or(std::cmp::Ordering::Equal),
            12 => ca.herd_value.partial_cmp(&cb.herd_value).unwrap_or(std::cmp::Ordering::Equal),
            13 => ca.approach_value.partial_cmp(&cb.approach_value).unwrap_or(std::cmp::Ordering::Equal),
            14 => ca.chase_value.partial_cmp(&cb.chase_value).unwrap_or(std::cmp::Ordering::Equal),
            15 => ca.chase_same_value.partial_cmp(&cb.chase_same_value).unwrap_or(std::cmp::Ordering::Equal),
            _ => std::cmp::Ordering::Equal,
        };
        if asc { ord } else { ord.reverse() }
    });

    let start = unsafe { OFFSET as usize };
    let end = (start + max_rows).min(total);
    for (vis_idx, &row_idx) in idxs[start..end].iter().enumerate() {
        let rank = start + vis_idx;
        let row = &rows[row_idx];
        let mut cx = x;
        let fs = 16.0;
        let yrow = y + (vis_idx as f32) * row_h;
        let row_hover = mx >= x && mx <= x + max_w && my >= yrow && my <= yrow + row_h;
        if row_hover { draw_rectangle(x - 4.0, yrow - 1.0, max_w + 8.0, row_h + 2.0, Color::new(1.0, 1.0, 1.0, 0.05)); }
        let color = match rank {
            0 => Color::new(1.0, 0.9, 0.5, 1.0),
            1 => Color::new(0.85, 0.9, 1.0, 1.0),
            2 => Color::new(0.9, 0.8, 0.7, 1.0),
            _ => WHITE,
        };
        draw_text_clamped(&format!("{}", rank + 1), cx, yrow, fs, color, cw[0] - 8.0); cx += cw[0];
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
        draw_text_clamped(&format!("{:.2}", row.chase_value), cx, yrow, fs, color, cw[14] - 8.0); cx += cw[14];
        draw_text_clamped(&format!("{:.2}", row.chase_same_value), cx, yrow, fs, color, cw[15] - 8.0);
    }

    // Scrollbar
    let track_x = panel.x + panel.w - 12.0;
    let track_y = y;
    let track_h = avail_h;
    draw_rectangle(track_x, track_y, 8.0, track_h, Color::new(1.0, 1.0, 1.0, 0.08));
    if total > max_rows {
        let thumb_h = (track_h * (max_rows as f32 / total as f32)).clamp(24.0, track_h);
        let max_offset = (total.saturating_sub(max_rows)) as f32;
        let ratio = if max_offset > 0.0 { (unsafe { OFFSET } as f32 / max_offset).clamp(0.0, 1.0) } else { 0.0 };
        let thumb_y = track_y + (track_h - thumb_h) * ratio;
        draw_rectangle(track_x, thumb_y, 8.0, thumb_h, Color::new(1.0, 1.0, 1.0, 0.25));
        if is_mouse_button_pressed(MouseButton::Left) {
            if point_in_rect(mx, my, track_x, track_y, 8.0, track_h) {
                unsafe {
                    if my < thumb_y { OFFSET = (OFFSET - max_rows as i32).max(0); }
                    else if my > thumb_y + thumb_h { OFFSET = (OFFSET + max_rows as i32).min((total.saturating_sub(max_rows)) as i32); }
                }
            }
        }
    }

    // Footer buttons
    let btn_h = 40.0;
    let btn_w = 160.0;
    let gap = 16.0;
    let total_btns = if modal { 3 } else { 2 };
    let total_w = btn_w * (total_btns as f32) + gap * ((total_btns - 1) as f32);
    let bx = panel.x + (panel.w - total_w) * 0.5;
    let by = panel.y + panel.h - PAD - btn_h;

    let close_hover = point_in_rect(mx, my, bx, by, btn_w, btn_h);
    let close_label = if modal { "Close" } else { "Close [T]" };
    draw_button(bx, by, btn_w, btn_h, close_label, close_hover);
    let mut continue_clicked = false;

    // Continue button (modal only)
    let bx2 = bx + btn_w + gap;
    if modal {
        let cont_hover = point_in_rect(mx, my, bx2, by, btn_w, btn_h);
        draw_button(bx2, by, btn_w, btn_h, "Continue [C]", cont_hover);
        if cont_hover && is_mouse_button_pressed(MouseButton::Left) { continue_clicked = true; }
        let hint_fs = 14.0;
        let hint = "Press C to Continue";
        let tw = measure_text(hint, None, hint_fs as u16, 1.0).width;
        draw_text(hint, bx2 + (btn_w - tw) * 0.5, by - 8.0, hint_fs, LIGHTGRAY);
    }

    // Export CSV button
    let bx3 = bx2 + btn_w + gap;
    let exp_hover = point_in_rect(mx, my, bx3, by, btn_w, btn_h);
    draw_button(bx3, by, btn_w, btn_h, "Export CSV", exp_hover);
    if exp_hover && is_mouse_button_pressed(MouseButton::Left) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let path = format!("snapshots/scoreboard_{}.csv", secs);
        let mut out = String::new();
        out.push_str("rank,idx,species,score,plants,meat,offspring,alive_steps,idle_penalty,attack_hits,kills,avg_energy,herd,approach,chase,chase_same\n");
        let mut idxs: Vec<usize> = (0..rows.len()).collect();
        let (sort_col, asc) = unsafe { (SORT_COL, SORT_ASC) };
        idxs.sort_by(|&a, &b| {
            let ca = &rows[a];
            let cb = &rows[b];
            let ord = match sort_col {
                0 => a.cmp(&b),
                1 => ca.idx.cmp(&cb.idx),
                2 => ca.species.cmp(&cb.species),
                3 => ca.score.partial_cmp(&cb.score).unwrap_or(std::cmp::Ordering::Equal),
                4 => ca.eaten_plants.cmp(&cb.eaten_plants),
                5 => ca.eaten_meat.cmp(&cb.eaten_meat),
                6 => ca.offspring.cmp(&cb.offspring),
                7 => ca.alive_steps.cmp(&cb.alive_steps),
                8 => ca.idle_penalty_value.partial_cmp(&cb.idle_penalty_value).unwrap_or(std::cmp::Ordering::Equal),
                9 => ca.attack_hits.cmp(&cb.attack_hits),
                10 => ca.kills_caused.cmp(&cb.kills_caused),
                11 => ca.avg_energy_norm.partial_cmp(&cb.avg_energy_norm).unwrap_or(std::cmp::Ordering::Equal),
                12 => ca.herd_value.partial_cmp(&cb.herd_value).unwrap_or(std::cmp::Ordering::Equal),
                13 => ca.approach_value.partial_cmp(&cb.approach_value).unwrap_or(std::cmp::Ordering::Equal),
                14 => ca.chase_value.partial_cmp(&cb.chase_value).unwrap_or(std::cmp::Ordering::Equal),
                15 => ca.chase_same_value.partial_cmp(&cb.chase_same_value).unwrap_or(std::cmp::Ordering::Equal),
                _ => std::cmp::Ordering::Equal,
            }; if asc { ord } else { ord.reverse() }
        });
        for (rank, &i) in idxs.iter().enumerate() {
            let r = &rows[i];
            out.push_str(&format!("{},{} ,{} ,{:.4},{},{},{},{},-{:.4},{},{},{:.4},{:.4},{:.4},{:.4},{:.4}\n",
                rank+1, r.idx, r.species, r.score, r.eaten_plants, r.eaten_meat, r.offspring, r.alive_steps,
                r.idle_penalty_value, r.attack_hits, r.kills_caused, r.avg_energy_norm, r.herd_value, r.approach_value, r.chase_value, r.chase_same_value));
        }
        if let Some(parent) = std::path::Path::new(&path).parent() { let _ = fs::create_dir_all(parent); }
        match fs::write(&path, out) {
            Ok(_) => EXPORT_STATE.with(|st| { let mut s = st.borrow_mut(); s.0 = Some(format!("Exported to {}", path)); s.1 = 2.5; }),
            Err(e) => EXPORT_STATE.with(|st| { let mut s = st.borrow_mut(); s.0 = Some(format!("Export failed: {}", e)); s.1 = 3.5; }),
        }
    }
    // Render export toast (thread-local state)
    EXPORT_STATE.with(|st| {
        let mut s = st.borrow_mut();
        if s.1 > 0.0 {
            s.1 -= get_frame_time();
            if let Some(msg) = &s.0 {
                let fsz = 16.0; let pad = 10.0;
                let tw = measure_text(msg, None, fsz as u16, 1.0).width + 2.0 * pad;
                let th = fsz + 1.6 * pad;
                let tx = panel.x + panel.w - tw - 18.0;
                let ty = panel.y + panel.h - th - 18.0 - btn_h - 8.0;
                draw_rectangle(tx, ty, tw, th, Color::new(0.0, 0.0, 0.0, 0.7));
                draw_rectangle_lines(tx, ty, tw, th, 1.0, WHITE);
                draw_text(msg, tx + pad, ty + fsz + (th - fsz) * 0.5 - 6.0, fsz, WHITE);
            }
            if s.1 <= 0.0 { s.0 = None; }
        }
    });

    if close_hover && is_mouse_button_pressed(MouseButton::Left) && !modal {
        // No-op; caller handles toggling with [T]
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
